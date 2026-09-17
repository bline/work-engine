# Idea: Candidate Trajectory — Durable Foundation

**Status:** Architecture idea; not accepted, prioritized, or authorized for
implementation.

**Scope:** The relationship between successive candidate checkpoints inside one
slice/attempt lifecycle (first candidate, remediation candidates, and any
later replacement candidates).

**Authority:** Exploratory only. This document does not amend the migration
roadmap, admit an implementation, or authorize spending. The user or an
explicitly authorized portfolio owner retains decision authority.

```yaml
idea_status:
  architectural_supersession: not_applicable
  architectural_supersession_note: "Checked 2026-09-17, per review: no plausible canonical owner exists to compare against, so this needs no additional semantic-coverage check to earn none rather than unknown -- this is App Server slice-campaign implementation detail, an unaccepted proposal for a checkpoint-diffing primitive internal to that service, not the kind of content any of the 13 dimensions, 5 mechanisms, or 2 substrates tracks."
  residue: none
  backlog: none
  backlog_note: "Corrected 2026-09-17, per re-audit: the 2026-09-17 correction above was itself wrong, and is retracted. BACKLOG requires the surrounding architecture to already be settled, with only construction/choice/checking remaining (SS2's own definition: 'its architecture is not in question'). This document's core primitive is not that -- its whole premise ('not accepted, prioritized, or authorized for implementation') means adoption itself, not merely construction, is undecided; there is no settled architecture this primitive extends the way, e.g., executable-generation-maintenance-rollover.md's SS52 extends already-confirmed executable-generation-lifecycle architecture. A concretely-specified, buildable design is not the same claim as operative backlog -- conflating 'this proposal is detailed enough to build' with 'this is confirmed unbuilt work' was exactly the error Predicate D was checking for, and this document is itself an instance of it, not an escape from it. The inline KIND/BACKLOG-OPEN tag previously placed in the Summary is removed accordingly; this is an unaccepted proposal, which is the corpus's baseline condition for an idea document, not itself a status this grammar's backlog axis tracks."
  audit_scope:
    - keyword-scan: full_document
    - close-read: "SS1-7"
  audit_scope_completeness: complete
  status_as_of: 2026-09-16
```

```yaml
idea_provenance:
  origin: direct_capture
```

## Summary

App Server already derives a deterministic physical profile for every
candidate checkpoint it binds, but it keeps only the current profile. The
predecessor profile and any relationship between successive candidates are
discarded the moment a replacement candidate is bound. This idea proposes a
narrow, purely mechanical primitive — **Candidate Trajectory** — that makes the
predecessor/successor relationship between candidate checkpoints, a
deterministic profile-to-profile structural delta, and (where the fact
actually requires it) a deterministic candidate-to-candidate delta, a durable,
recoverable App Server fact instead of something every future consumer has to
reconstruct from Git or from model conversation state.

An earlier pass at this document claimed the whole delta could be computed
from two physical profiles alone. That overclaimed what a profile-to-profile
comparison can establish; section 3 below corrects it by splitting the delta
into two distinct deterministic products with different evidence sources.

This is deliberately the smallest useful slice. It changes no reviewer
behavior, no builder behavior, and introduces no scoring or policy. Two later
ideas — [remediation delta for native review](candidate-trajectory-remediation-delta-for-native-review.md)
and [builder-side trajectory consumption](candidate-trajectory-builder-side-consumption.md)
— are consumers of this primitive and are intentionally kept separate because
they change what a model reads and therefore carry evaluation obligations this
primitive does not.

## 1. What already exists (do not rebuild it)

`review-subject` is already the App Server service boundary for candidate
checkpoints and physical profiles — this is not a proposal, it is current,
documented behavior:

- `createReviewSubjectService` exposes `createCandidate`, `createPhysicalProfile`,
  and `validatePhysicalProfile` as mediated operations
  (`app-server/src/services/review-subject/service.mjs:1-40`).
- The legacy backend adapter verifies the exact SHA-256 of both the
  checkpoint-owner validator and the `code-change-profile` analyzer before
  invoking either, and validates the returned envelope against the requested
  operation and those digests
  (`app-server/src/services/review-subject/legacy-backend-adapter.mjs:57-106`,
  `app-server/src/services/review-subject/contract.mjs:48-72`).
- `docs/review-subject-service.md` documents this as a finished ownership
  boundary: "the `code-change-profile` backend owns only its derived subject
  and profile identities, analyzer/provenance identity, physical observations,
  coverage, limitations, and measurement-state vocabulary," delegating
  Git/checkpoint truth to `slice-checkpoint`.
- `docs/skills-migration-plan.md` (S7) records this as an objective already
  landed: "Port `slice-checkpoint` and `code-change-profile` as host or server
  capabilities that produce an immutable attributed review subject and a
  recomputable deterministic physical profile" (`app-server/docs/skills-migration-plan.md:1059-1076`).

So the "promote Code Change Profile to an explicit App Server service
boundary" step that motivated this exploration is **already done**. Any
implementer picking this up should not re-propose that migration; the gap is
elsewhere.

## 2. The actual gap: the relationship between candidates is not retained

`createSliceCampaignService`'s campaign state holds exactly one candidate and
exactly one physical profile at a time:

```js
candidateRequestDigest: null, candidate: null, physicalProfile: null,
```

(`app-server/src/services/slice-campaign/service.mjs:99`)

`bindCandidate` confirms this is a true overwrite, not an append, on the
remediation/replacement path:

```js
const candidateState = state.candidate && !replacement ? state : publish({
  ...state, candidateRequestDigest: requestDigest, candidate, physicalProfile: null,
  ...(replacement ? {nativeReview: replacementNativeEnvelope(state)} : {}),
}, state.revision);
const physicalProfile = await reviewSubject.createPhysicalProfile({ subject: {
  schema_version: 2,
  construction_method: "slice_checkpoint_candidate_receipt",
  evidence_cutoff: candidate.created_at,
  checkpoint: candidate,
} });
return publish({ ...candidateState, physicalProfile }, candidateState.revision);
```

(`app-server/src/services/slice-campaign/service.mjs:255-278`)

When a remediation candidate C2 replaces C1, C1's physical profile is gone
from campaign state. Nothing durable records that C2 followed C1, or what
changed between their profiles. Any later consumer that wants "what did this
repair actually change" has to reconstruct it from Git directly, or (worse)
from a model's own recollection of its prior turns. This is exactly the kind
of inference Work Engine otherwise goes out of its way to avoid.

## 3. Proposed primitive: Candidate Trajectory

An append-only, host-owned record of the candidate sequence within one
slice/attempt, with three separately identified layers rather than one
overloaded identity, and two distinct deterministic delta products rather than
one.

### 3.1 Identity: keep candidate, profile, and trajectory separately versioned

A candidate's identity must not depend on which analyzer version happened to
produce its profile, and a trajectory edge's identity must not depend on
recomputing a profile with a later analyzer. Otherwise recomputing C2's
profile with a revised analyzer could make it look like a different candidate
in the trajectory, or silently orphan the edge that pointed at the old profile
digest. Three separate identities:

```text
candidate identity
  run, slice, attempt, plan revision, candidate ordinal,
  checkpoint commit/tree

profile observation
  candidate identity
  profile digest
  analyzer identity/version

trajectory derivation
  predecessor candidate identity
  successor candidate identity
  predecessor profile digest
  successor profile digest
  trajectory deriver version
  trajectory digest
```

This mirrors an existing Work Engine pattern: the historical event (the
candidate) is immutable and permanent, while derived observations about it
(the profile, and now the trajectory edge) may be revisioned without rewriting
or losing the event they describe (`skills/code-change-profile/references/profile-contract.md:37-39`:
"Recomputing with a revised analyzer creates a new profile identity; it does
not rewrite the subject or earlier profile.").

```text
C1  predecessor=null
C2  predecessor=C1, predecessor_profile=P1, successor_profile=P2
C3  predecessor=C2, predecessor_profile=P2, successor_profile=P3
```

This three-layer split also independently converges with
[revisioned-research-and-execution-architecture.md](revisioned-research-and-execution-architecture.md)'s
reproducibility requirements, arrived at from a completely different direction
(experimental reproducibility across research branches, not candidate
remediation): "Derived comparison artifacts such as diffs are projections, not
subject identity... bind [their] endpoint identities and a canonical
projection format, or record the generator and every option or configuration
input that can change [their] bytes" (section 23). That document's rule —
comparisons are valid only when subject identity and projection identity are
separately attributable — is the same rule this section applies to candidates
and their derived deltas. Two independent designs reaching the same
separation is evidence the separation is a real requirement here, not
incidental caution.

The same document's section 27 ("Coordinate Adequacy Is Claim-Relative") also
describes the right posture for this primitive's own outputs: "the same
coordinate may be adequate for forensic inspection and inadequate for role
recovery." A Candidate Trajectory delta establishes physical facts
unconditionally, but whether those facts are *sufficient* — for a reviewer's
remediation judgment, a capability-evidence claim, or a research comparison —
depends on the consuming claim, not on the delta itself. Section 3.4 below and
the two consumer documents each specify their own sufficiency question rather
than assuming the delta is generically "enough."

### 3.2 Two distinct delta products, not one

The physical profile is computed **baseline → candidate**, always. Two
profiles P1 and P2 therefore tell you what each candidate's surface looks like
relative to the same baseline — they do not by themselves tell you what
changed **between** C1 and C2. Concretely: if `foo.py` and symbol `f` both
appear as changed in P1 and in P2, that only establishes that `foo.py`/`f` are
part of both candidates' surfaces. C2 could contain the exact same bytes for
`foo.py` as C1 (nothing changed during remediation) or a further edit
(something changed again) — a profile-to-profile comparison cannot
distinguish these two cases. Establishing "modified again" is therefore a
genuinely different, and in one case more expensive, deterministic product:

```text
Profile delta            D(P1, P2)
  computed from the two already-validated physical profiles alone;
  no new Git inspection, no new analyzer invocation.

  can establish:
    paths entering/leaving the candidate surface (relative to baseline)
    modules entering/leaving the surface
    category/count differences
    aggregate line/hunk shape differences
    attribution differences where represented
    coverage/limitations differences

Candidate delta          D(C1, C2)
  requires evidence beyond the two profiles for path- and symbol-level
  "modified again" facts specifically.

  can establish:
    paths actually modified again between C1 and C2
    exact C1 -> C2 patch
    symbols actually modified again (where supported)
    additions/reversions introduced during remediation
```

**Path-level "modified again" is cheaper than a full Git diff.** Each
candidate's own attributed manifest already carries a per-path
`content_digest` — `slice-checkpoint`'s `create_candidate` computes a SHA-256
blob digest for every included path when the candidate is created
(`skills/slice-checkpoint/scripts/checkpoint.py:312-320`), and the
`code-change-profile` validator requires every manifest entry to carry it
(`skills/code-change-profile/scripts/code_change_profile.py:504-511`). So
`paths_retained_and_modified` can be established by comparing C1's and C2's
own manifests path-by-path — a path present in both with a differing
`content_digest` was genuinely modified again — with **no Git subprocess
call at all**, since both manifests are already materialized JSON on the
candidate objects App Server already holds.

**Symbol-level "modified again" still needs a real C1→C2 diff.** There is no
per-symbol content identity anywhere in the current contract — only a
per-path content digest. Determining whether symbol `f` specifically changed
again (as opposed to some unrelated part of the same file) requires rerunning
the profiler's existing AST symbol-diff routine against the two candidate
trees directly, exactly the way `createReviewBoundary` already derives an
exact patch between two trees
(`app-server/src/services/slice-campaign/native-review-host.mjs:125-198`) —
same trusted mechanism, pointed at C1's tree and C2's tree instead of
baseline and candidate. This is the one place this primitive does add a new
(still fully deterministic, still Git-bound-only) derivation step beyond what
`review-subject` already computes; it reuses that mechanism rather than
inventing a second one.

### 3.3 Delta contract

```text
paths_added / paths_removed                    -- from profile delta
paths_retained_and_modified                    -- from manifest content_digest comparison
modules_added / modules_removed / modules_retained   -- from profile delta
symbols_added / symbols_removed                -- from profile delta (baseline-relative, where supported)
symbols_modified_again                         -- from candidate delta (C1->C2 tree diff, where supported)
file_category_delta / line_delta / hunk_delta  -- from profile delta
task_owned_surface_delta                       -- from profile delta
generated_dependency_delta / validation_dependency_delta  -- from profile delta
coverage
limitations
```

`coverage` and `limitations` are not an afterthought. `skills/code-change-profile/references/profile-contract.md:27-39`
defines a measurement-state vocabulary (`observed` / `unknown` / `unsupported`
/ `failed` / `not_applicable`) specifically so an absent measurement is never
silently treated as zero. Both delta products must inherit that discipline
field-by-field: if a symbol count on either side is `unsupported`, or a
candidate delta could not be computed (e.g. the C1 tree is no longer
reachable), the corresponding field must say so rather than compute a numeric
difference against an absent value or silently fall back to the cheaper
profile-delta approximation without saying so.

Both products remain fully deterministic — `deterministic(P_{n-1}, P_n)` for
the profile delta, `deterministic(C_{n-1}, C_n)` (via manifest comparison and,
where needed, tree diff) for the candidate delta. Neither introduces
inference, and neither introduces new concepts like "bad churn."

### 3.4 What this primitive explicitly does not do

- It does not decide whether a delta is good or bad ("remediation is
  diverging" is a policy/inference conclusion, not a trajectory fact).
- It does not run against a mutable worktree. Every observation is bound to an
  immutable candidate checkpoint, exactly like the existing physical profiler.
- It does not change reviewer or builder behavior. Nothing reads it yet.
- It does not introduce a refactor score, weighting, or cross-slice
  aggregation. That is longitudinal analysis and belongs to a separate
  consumer (see [relationship to Refactor Pressure](candidate-trajectory-upstream-amendments.md)).

## 4. Storage and recovery

The trajectory record should be durable and recoverable the same way campaign
state already is (`sqlite-store.mjs`), keyed so that a restart can
deterministically recover the full candidate chain for an in-progress
slice/attempt without model inference or transcript reconstruction.

`bindCandidate` already has three sequential, independently interruptible
steps — publish the candidate with `physicalProfile: null`, compute the
profile, publish again with the profile attached
(`app-server/src/services/slice-campaign/service.mjs:255-278`) — and this
primitive adds at least one more (append the trajectory edge, once a
predecessor exists). That multi-step sequence is exactly the kind of failure
boundary Candidate Trajectory ought to close, not add a new way to open. The
crash-recovery obligations in section 5 below should drive whether this lives
as a new table in the existing slice-campaign SQLite store (so candidate,
profile, and trajectory append can share one transaction) or as a sibling
store analogous to `review-subject`'s own boundary (which then needs its own
cross-store reconciliation story) — this document takes no position beyond
requiring that a restart at any point in the sequence produce exactly one
recoverable interpretation.

**This storage decision is not this document's to make independently.**
[`service-plane-reconciliation.md`](../docs/service-plane-reconciliation.md)
found a `COUPLED_DECISION` bearing directly on it: whether `slice-campaign`'s
campaign-state persistence should separate from campaign orchestration the
way `review-episode` already separates state from its consumer. That
decision determines what "the existing slice-campaign SQLite store" even
means going forward — if campaign-state persistence is factored out as its
own service, Candidate Trajectory should bind to whatever that service
becomes rather than to `slice-campaign`'s current storage directly. Building
a bespoke predecessor store here before that decision is made risks solving
the same problem twice, incompatibly. This document's position remains: no
storage choice, pending that decision.

## 5. Acceptance shape

A first implementation should be judged entirely by deterministic tests, not
by any reviewer- or builder-facing behavior change:

- first candidate produces a trajectory record with `predecessor: null`;
- a replacement candidate produces a record bound to its immediate
  predecessor's profile digest, with a computed delta;
- idempotent replay of the same `bindCandidate` request does not duplicate a
  trajectory record or recompute a different delta;
- a host restart recovers the full candidate chain from durable storage alone;
- a physical-profile failure on either side of a delta is recorded as a
  failed/unsupported delta, never a fabricated zero;
- a predecessor-digest mismatch (the bound predecessor profile does not match
  what the store has on record) fails closed rather than silently rebinding;
- unsupported symbol coverage on either input profile propagates into
  `coverage`/`limitations` on the delta rather than being dropped;
- the trajectory record's own digest is verifiable the same way profile and
  checkpoint digests already are;
- a candidate delta correctly distinguishes a retained-but-byte-identical path
  from a retained-and-modified path using manifest `content_digest` alone (no
  tree diff needed for that specific fact);
- a symbol-level candidate delta that cannot be computed (e.g. the analyzer
  only supports Python and the retained-modified path is not Python) is
  recorded as `unsupported`, never silently omitted or approximated from the
  path-level fact.

Explicit crash-window tests, one per step in the sequence section 4
describes, each restarted mid-step:

- candidate persisted, crash before the physical profile is computed — restart
  must either resume profile computation for the same candidate identity or
  deterministically detect and report the incomplete state; it must never
  silently create a second candidate for the same request;
- profile computed, crash before the trajectory edge is appended — restart
  must append exactly one edge, not zero and not a duplicate;
- trajectory edge appended, crash before campaign state advances past it —
  restart must recover a campaign state consistent with the already-appended
  edge, not one that tries to append it again.

In every case, after restart there must be exactly one recoverable
interpretation of what happened, with no duplicated trajectory edge and no
orphaned candidate or profile left unreferenced by any edge.

## 6. Why this does not need a pilot

Everything in this document is a deterministic transformation of data App
Server already produces and validates (two physical profiles, two already-
materialized candidate manifests, and — for symbol-level facts only — one
additional Git tree diff computed the same trusted way `createReviewBoundary`
already computes one). There is no model in this loop, no claim about behavior
change, and no judgment being reduced or replaced yet — this idea only makes
an existing, transient relationship durable. It should be accepted or rejected on ordinary
engineering grounds (contract clarity, storage cost, test coverage), not on
empirical/behavioral evidence. The pilot question becomes relevant only once a
consumer starts handing trajectory facts to a model — see the two consumer
ideas below.

## 7. Relationships

| Direction | Relationship |
| --- | --- |
| `review-subject` service | Owns the physical profile this idea consumes as an immutable input. This idea adds no new physical-profile semantics. |
| [Remediation delta for native review](candidate-trajectory-remediation-delta-for-native-review.md) | First consumer; depends on this primitive existing. |
| [Builder-side trajectory consumption](candidate-trajectory-builder-side-consumption.md) | Second consumer; depends on this primitive existing. |
| [Deterministic Refactor Pressure](deterministic-refactor-pressure-from-work-engine-evidence.md) | Longitudinal consumer across many slices; see the
[proposed amendments](candidate-trajectory-upstream-amendments.md) for how that document's rework/recurrence evidence should be revised to source from this primitive instead of ad hoc checkpoint comparison. |

This document does not own the trajectory storage schema's exact shape, the
`review-subject` contract, campaign-state schema versioning, or any consumer's
behavior. It only proposes that the candidate-to-candidate relationship become
a first-class, durable, recoverable App Server fact.
