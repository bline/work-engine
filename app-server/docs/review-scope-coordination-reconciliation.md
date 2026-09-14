# Reconciliation: Review Applicability and Mutation-Scope Coordination

## Status

Reconciliation of `ideas/review-scope-coordination.md` against current
app-server implementation and current prospective architecture. Wave 1, item
2 of the sequel reconciliation queue.

Per instruction: the idea was not improved or modernized. Every clause was
checked against implemented code or an existing prospective-architecture
document. What is already implemented or already owned elsewhere is retired
below. One clause has no current owner and is placed, not rewritten.

## Idea summary (unchanged)

`ideas/review-scope-coordination.md` asks how independent review stays
useful while repository state keeps evolving. It frames the problem as
coordination between review applicability and mutable scope, lists a
required-consequence set (exact reviewed subject; the scope whose mutation
threatens applicability; mechanical and semantic overlap detection; whether
work may continue/wait/need a new subject/need adjudication; whether a
result still applies, composes, needs refresh, or is superseded), and a
coordination-concepts list (durable review obligations, protected mutation
scopes, overlap detection, reservations/claims, explicit supersession,
applicability judgments, bounded conflict dispositions) — explicitly framed
as candidate mechanisms, not a mandatory procedure.

## What is already implemented

### `app-server/src/services/claim-evidence` (retires the evidence/lineage half of "Required consequence" and "Coordination concepts")

This is not a design document only — it is a real, substantial
implementation (15 files, ~2400 lines) already in `app-server/src/services/`.
Most directly relevant to this idea:

- `review-finding-bridge.mjs` already implements a domain profile named
  `revision-bound-review-finding-v1` — the exact profile the formed
  `proposals/adaptive-specialized-review/revision-bound-review-artifacts`
  proposal describes (see below), with `publishFindings`, `recordReliance`,
  and `project` operations;
- `identity.mjs` gives every review finding a stable claim identity bound to
  its exact evidence world (`stableClaimId`, `digest`);
- `recordReliance` records reliance against an exact finding revision,
  consumer, consumer revision, and decision scope, with `active`/`retired`/
  `superseded` state. This is downstream-reliance *bookkeeping* — "decision X
  used finding/claim Y" — not a protective reservation over a mutable scope.
  It is a distinct object from "review protection" ("mutation to scope S
  would invalidate work currently being reviewed"): a reliance record only
  exists after a consumer has already used a finding, so it cannot detect a
  review in progress with no finding published yet, and an active reliance
  record on an unrelated finding could be mistaken for a block on an
  unrelated, legitimate mutation if treated as the protective mechanism.
  `claim-evidence` separately owns durable refresh/selective-reopening
  obligation records (a *post-impact* obligation: an impact was nominated,
  and a domain workflow now owes a decision about it) — this is distinct
  from `recordReliance` and is also distinct from a *pre-mutation* active-
  review protection obligation, which neither mechanism owns;
- `reviewer-projection.mjs` projects only relevant, provenance-bearing claim
  revisions into a reviewer's context, with explicit freshness/completeness/
  exclusion/failure fields.

`app-server/docs/claim-evidence-service.md` (the authorized design target
this implementation is converging on) explicitly names "revision-bound
review findings" as one of its two initial proving domain profiles
(`claim-evidence-service.md:121-128`): "Their domain owners retain
materiality, severity, review episode, synthesis, support, outcome, and
decision semantics. The shared service owns only the common evidence and
lineage boundary." Its "Preserved semantics from the evidence-lineage
proposals" section already lists refresh, correction, supersession,
composition, derivation, identity fork, and retraction as distinct
non-destructive relations — this is "explicit supersession" from the idea's
coordination-concepts list, and the *relation vocabulary* half of
"applicability judgments" (naming what kind of relation exists between two
claim revisions once both exist).

This matches the ownership split already present in the claim-maintenance
design: shared `claim-evidence` owns record identity and lineage, while
domain workflows retain applicability, reliance decisions, selective
reopening, and semantic completion. `claim-evidence` is not, and does not
claim to be, the owner of *whether a mutation may proceed while a review
relationship is active* — see the residue below.

Disposition: retire "exact immutable subject being reviewed" and "explicit
supersession" as already owned and substantially already built by
`claim-evidence`. Retire the relation-vocabulary half of "applicability
judgments" on the same basis, but not the protective/prospective half (see
residue). Partially retire "durable review obligations / reservations or
claims": the post-impact obligation (refresh/selective reopening, plus
reliance bookkeeping via `recordReliance`) is already owned by
`claim-evidence`; the pre-mutation active-review protection obligation is
not owned by anything found here and is folded into the residue below.

### `workspace-coordination` (retires generic protected-resource *enforcement* only)

Already established in the item-1 reconciliation
(`control-plane-and-client-protocol-reconciliation.md`): typed resource
leases (`directory`, `git-ref`, `git-index`, `port`, `index`,
`review-budget`, `database`), fencing tokens, expiry, and CAS-protected
`admitMutation`. `review-budget` is already one of its resource types,
confirming this mechanism is already understood to reach into review-adjacent
concerns.

This retires the *generic enforcement mechanism* — lease, fence, expiry,
`admitMutation` — as already implemented and already inventoried, not a
hypothesis. It does not retire "protected mutation scopes" as a whole. The
idea's own coordination-concepts list is asking two separable questions:

```text
review domain
    owns/declares (not found anywhere):
        exact review subject
        scope whose change threatens that review

workspace-coordination
    owns (already implemented):
        exclusive/fenced mutation enforcement
        once told which resource to protect
```

Nothing inspected in this reconciliation declares, on behalf of an active
review, which scope `workspace-coordination` should be asked to protect.
`workspace-coordination`'s existing `RESOURCE_TYPES` (including
`review-budget`) are declared and leased by whichever caller already knows
its own resource key; no caller inspected here derives that key from "a
review is currently relying on this scope." This half of "protected mutation
scopes" is folded into the residue below.

### `proposals/adaptive-specialized-review/revision-bound-review-artifacts` (formed proposal; retires only the review-result half of "bounded conflict dispositions")

This proposal (`disposition: approve_proposal_meaning`,
`implementation_authorized: false` — approved in meaning, not yet built as a
general contract) already defines the review-artifact profile the idea asks
for: episode identity, per-finding disposition vocabulary (retained,
deferred, inapplicable, omitted), truthful non-success outcomes, and
"review-specific applicability, partial-applicability, refresh, composition,
supersession, correction, and invalidation consequences over shared
lineage." Its `relationships.md` already names its dependency on
claim-centered evidence lineage — the proposal family
`claim-evidence-service.md` explicitly carries forward
(`claim-evidence-service.md:6-7`: "carr[ies] forward the evidence-lineage
semantics developed under `proposals/evidence-lineage`"). The
`review-finding-bridge.mjs` implementation above is, in substance, an
in-progress realization of this proposal's placement hypothesis.

This proposal's disposition vocabulary (retained, deferred, inapplicable,
omitted) answers *what happened to this finding*. The idea's "Required
consequence" vocabulary (continue, wait, needs a new review subject, needs
explicit adjudication) answers a different question: *what do we do about a
planned mutation while this review relationship exists*. No document
inspected in this reconciliation demonstrates a direct mapping between the
two vocabularies, and one is not assumed here.

Disposition: retire only the review-result half of "bounded conflict
dispositions" — already owned in design (this proposal) and partially owned
in code (`review-finding-bridge.mjs`'s `revisionPayload` disposition
fields). The mutation-coordination disposition vocabulary (continue / wait /
new subject / adjudication) is not retired by this proposal and remains
part of the residue below.

## What the idea's own evidence already confirms is unresolved

A prior reconciliation pass (`app-server/docs/root-ideas-reconciliation.md:276-284`)
already verified directly against source, not inferred, that claim-evidence
deliberately declines to resolve applicability:
`review-finding-bridge.mjs:96` states in a live template string,
`"Exact revision for review finding ${findingId}; applicability and
reliance remain consumer decisions."` That pass also found the only
overlap-detection logic anywhere in the codebase: a narrow, single-purpose
exact-path-membership check in
`slice-campaign/completion-publication.mjs:129`
(`operational_paths.some(entry => manifest.some(...))`), scoped to one
completion-offer request's own fields — not a general coordinator, not
reachable from outside slice-campaign, and not aware of claim-evidence
reliance records at all. Re-verified directly against source in this pass;
unchanged.

`app-server/ideas/pending/evidence-anchor-observation-and-impact-nomination.md`
was checked as a candidate owner of overlap detection. It explicitly scopes
itself to observing whether an *already-declared* anchor's cited evidence
still matches (`"a deterministic boundary that observes whether a declared
dependency still matches its cited evidence"`) — reactive verification of a
named dependency, not discovery of which claims a newly planned mutation
would touch. It does not claim ownership of prospective overlap discovery.

`claim-evidence-service.md`'s "Impact, refresh, and reliance propagation"
pipeline (`claim-evidence-service.md:457-471`) is explicitly reactive: it
starts from a `repository or contract event` — something that already
happened — and states a preference for "on-demand refresh when a real
consumer needs a decision," not continuous or prospective evaluation. It
also states as an explicit non-goal (`claim-evidence-service.md:74-84`, "The
service does not acquire authority to... reopen downstream work merely
because an impact was nominated").

## The smallest remaining semantic consequence still lacking an owner

> Before changing something under active review, somebody needs to
> establish whether the review's evidence world must remain stable and what
> consequence follows if it won't.

**Prospective review-scope coordination before mutation admission** —
determining whether a planned mutation threatens an active review subject or
an applicable downstream review consequence, and obtaining the appropriate
workflow-owned disposition before generic mutation machinery admits the
change — is not owned by any current implementation or prospective-
architecture document inspected in this reconciliation.

The defining gate input is **active review applicability/protection state**,
not reliance alone. Exact reliance (`recordReliance`) is one possible
relevant input — a decision that already used a finding is one reason a
scope might need protecting — but it is neither necessary (a review in
progress with no finding published yet has no reliance record, yet still
needs protecting from the false negative described above) nor sufficient (an
old, unrelated active reliance record must not itself block an unrelated,
legitimate mutation — the false positive described above) as the gate:

```text
False positive
  old decision still has an active reliance record
      -> new unrelated/legitimate mutation overlaps
      -> mutation accidentally treated as blocked

False negative
  review is currently in progress
      -> no finding published yet
      -> no downstream recordReliance exists
      -> mutation changes the subject under the reviewer
```

The cleaner surviving seam, decomposed rather than collapsed into one new
service:

```text
Review owner / review workflow
    |
    | exact subject
    | active applicability/protection state
    | scope whose mutation may matter
    v
prospective overlap observation
    |
    | mechanical where mechanically decidable
    | otherwise attributed semantic assessment
    v
coordination consequence
    |
    +-- continue
    +-- wait
    +-- bind new review subject
    +-- require adjudication
    |
    v
workspace-coordination
    enforces whatever mutation admission
    consequence was authoritatively established
```

This preserves the existing architecture. It does not make
`workspace-coordination` understand review semantics — it still only
enforces a lease/fence outcome once told what to protect. It does not make
`claim-evidence` acquire mutation authority — its non-goals already exclude
that. And it does not turn a historical reliance edge into a lock.

Concretely, three things compose here and none of them currently reference
each other:

```text
workspace-coordination.admitMutation
    decides: may this fencing-token holder mutate this resource
    inputs: lease state, fencing generation
    blind to: any review's applicability/protection state

claim-evidence (recordReliance, impact-nomination pipeline)
    decides: does a changed subject affect an existing claim, after the fact
    inputs: repository/contract events, exact reliance records
    triggered by: an event that already happened
    blind to: a mutation that has not yet been admitted; a review in
    progress with no finding yet

review workflow (no current owner identified)
    would decide: which scope an active review currently needs protected,
    and what disposition (continue/wait/new subject/adjudication) applies
    to a planned mutation against that scope
    inputs: none currently defined
```

`admitMutation` has no parameter or check against review-applicability
state; claim-evidence's propagation pipeline has no mutation-admission hook
and is explicitly on-demand/reactive rather than a precondition; and no
review-domain owner was found declaring a protected scope in the first
place. `production-path-service.mjs`'s `mutationAuthorized` capability-
envelope flag was checked as a candidate owner and ruled out — it verifies
that an *observation transport* did not require mutation authority
(protecting the integrity of a passive observation), which is orthogonal to
this seam.

This is the one clause placed rather than retired. It is not rewritten or
designed here, per the standing constraint. A future owner should decide
whether this becomes a new `workspace-coordination` admission input, a
`claim-evidence`-adjacent pre-mutation query operation, a review-workflow
responsibility that declares protected scope and calls `workspace-
coordination` directly, or a distinct coordination service — that decision
is out of scope for this reconciliation.

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| Exact immutable subject being reviewed | Retired — `claim-evidence` identity/evidence-world binding, already implemented. |
| Scope whose mutation may invalidate applicability (reactive, after a decision relied on it) | Retired — `claim-evidence`'s impact/refresh/reliance pipeline, already designed and partially implemented. |
| Scope whose mutation may invalidate applicability (prospective — declaring what an *active* review needs protected, before any finding/reliance exists) | **Not retired.** Folded into the surviving seam below. |
| Mechanical/semantic overlap detection (prospective, pre-mutation) | **Not retired.** Folded into the surviving seam below. |
| Whether work may continue/wait/need new subject/need adjudication | **Not retired.** This is the coordination-consequence vocabulary of the surviving seam; no direct mapping to review-artifact dispositions was demonstrated. |
| Whether a result applies/composes/needs refresh/is superseded | Retired (relation vocabulary) — `claim-evidence-service.md`'s preserved evidence-lineage relations (refresh, correction, supersession, composition, derivation). The protective/prospective half is not retired — see above. |
| Durable review obligations / reservations or claims | **Partially retired.** `claim-evidence` already owns durable refresh/selective-reopening obligation records and downstream reliance bookkeeping (post-impact obligations). A pre-mutation active-review protection/reservation declaring which mutable scope must remain stable (a distinct, pre-impact obligation) is not owned and is folded into the surviving seam. |
| Protected mutation scopes | Partially retired — generic lease/fence/`admitMutation` enforcement is retired to `workspace-coordination`. Review-specific declaration of *which* scope to protect is not retired. |
| Explicit supersession | Retired — `claim-evidence-service.md`'s preserved relations. |
| Bounded conflict dispositions | Partially retired — the review-result half (retained/deferred/inapplicable/omitted) is owned by `revision-bound-review-artifacts` (formed, placement-uncertain) and partially built in `review-finding-bridge.mjs`. The mutation-coordination half (continue/wait/new subject/adjudication) is not retired. |
| Boundary from scheduling / Boundary from review artifacts | Retired — consistent with `control-plane-and-client-protocol-reconciliation.md`'s findings on role-scheduler and with `revision-bound-review-artifacts`'s own boundary section. |

**Smallest remaining semantic consequence still lacking an owner:**
prospective review-scope coordination before mutation admission (see above)
— this single seam is what the several "Not retired" rows above collapse
into; they are not five separate open items.

## Recommended status change to the idea file

Update `ideas/review-scope-coordination.md`'s Status section to note that
this reconciliation exists and that most clauses are retired or partially
retired, pointing to this document and to `claim-evidence-service.md`,
`review-finding-bridge.mjs`, `workspace-coordination`, and
`revision-bound-review-artifacts`. The remaining open clause (prospective
review-scope coordination before mutation admission) should be the only
thing left active in that idea file going forward.
