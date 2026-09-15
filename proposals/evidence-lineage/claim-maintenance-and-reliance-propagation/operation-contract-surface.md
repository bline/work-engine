# Operation-Contract Surface: Impact Nomination and Refresh-Episode Admission

## Status

Design document, not implementation. This is the "closed operation contracts"
stage of the lowering Sol named when scoping this bounded unit of work:

```text
already-approved semantic model
            ↓
production transition boundaries
            ↓
closed operation contracts        <- this document
            ↓
implementation
            ↓
adversarial acceptance evidence (acceptance.md)
```

It does not authorize implementation. `implementation_authorized` remains
`false` on both
[`claim-maintenance-and-reliance-propagation/decision.json`](decision.json)
and
[`production-claim-evidence-interface/decision.json`](../production-claim-evidence-interface/decision.json)
until this design is reviewed and a separate implementation authorization is
recorded.

**Scope.** This covers only impact nomination and refresh-episode admission —
the segment of the semantic chain
[`semantic-model.md`](semantic-model.md) states as:

```text
source event -> candidate impact nomination -> refresh episode
    opened/admitted -> semantic investigation/judgment
    -> optional successor claim revision
```

It deliberately does not cover reliance, selective-reopening obligations, or
delivery/recovery (`semantic-model.md`'s own "Selective-reopening
obligations" and "Publication, delivery, and recovery" sections) — those
remain owned by the domain workflow and a general control-plane delivery
capability respectively, unaffected by this surface, and are their own later,
separately-bounded unit of work.

**Governing authority principle (the operator's own, contributed during
review of the first draft; wording tightened by Sol on the second
review).** Semantic judgment should be assigned to the bounded role with the
strongest relevant evidence and context; authority to make that judgment
consequential exists only through the applicable explicit grant or owning
contract. Evidence position alone does not create authority — Work Engine
explicitly rejects "most informed ⇒ authoritative." Coordination authority
does not imply domain authority, and an upstream role must not appropriate a
downstream role's judgment merely because it can observe or control the
workflow. This governs the concurrency design in Operation 3 below directly:
a kernel-level, cross-domain coordination service is the wrong owner for a
domain-local judgment's concurrency control, precisely because coordination
authority over *when* something may run is not the same as domain authority
over *whether a judgment is correct*.

## Revision history

**2026-09-15, first draft reviewed by Sol — not passed.** Three material
contract issues and two overclaims were found: (1) the original design had
`publish_refresh_judgment` reference an already-published revision rather
than causally produce it, inverting the semantic model's own stated
causality and creating a crash window with an authoritative revision and no
admitted judgment explaining it; (2) content-derived nomination identity
silently decided an unresolved semantic question (whether independently
nominated duplicates should collapse); (3) the writer-generation fence
allowed a different actor to close an episode without any explicit
takeover/admission transition, and cited `workspace-coordination` — a
kernel/coordination-plane service — as precedent for a domain-local
concern; (4) the design left three named intermediate episode lifecycle
states real but unowned instead of making an explicit claim about them; (5)
the claim that an unused `LINEAGE_RELATIONSHIPS` value proved the substrate
anticipated "exactly this composition" overstated what that one fact
supports. All five were corrected in the second draft.

**2026-09-15, second draft reviewed by Sol — still not passed, two material
issues remained.** (1) The second draft borrowed `review-episode`'s
generation/predecessorRevision *validation shape* without also borrowing the
admitted transition that gives it meaning — `review-episode`'s check only
makes sense because `replace_writer` is itself a real, separately-admitted
state transition; the second draft had "the token but not the transition."
Applying the contracts-not-procedures test directly — what invariant becomes
false if refresh episodes do *not* have writer generations in this
two-state (`opened` → terminal) first vertical? — the answer is none: plain
CAS on the episode's own revision, exactly `publish_revision`'s own existing
pattern, already prevents two concurrent judgments from both succeeding.
Removed the writer-generation abstraction entirely; see Operation 3's
"Concurrency" below. (2) `publish_refresh_judgment` derived every triggering
nomination's disposition from the episode's own single terminal disposition
— but the approved proposal requires every triggering nomination to be
"resolved or visibly preserved" individually (`semantic-model.md`), and one
episode can truthfully resolve some triggers as causal and others as
irrelevant. Added per-trigger resolution to Operation 3, distinct from the
episode's own disposition, and removed the singular `changed_because_of`
field in favor of causality attribution living at the per-trigger level
(neither forcing it into an array nor presuming it singular). Both
corrected below; the sections below reflect this third-round design.

## Method: discovering the operation count, not presuming it

Sol's test for whether two transitions belong in one operation: do they
share the same authority, atomicity boundary, idempotency identity,
failure/recovery consequence, and independently meaningful durable state? If
not, they are probably not one operation. Applied to the four candidate
transitions above:

| Transition | Authority | Atomicity boundary | Idempotency identity | Durable state |
| --- | --- | --- | --- | --- |
| Nominate impact | evidence-producer class (observer/analyzer) | one nomination against one target revision | caller-asserted nomination identity + standard operation-id/payload-digest retry (see below) | nomination record |
| Open refresh episode | authorized domain-owner class | one episode against one subject revision | caller-supplied episode identity | episode record, opened |
| Publish refresh judgment + resulting revision, when required | authorized domain-owner class (structurally provable via plain CAS even when a different actor closes than opened it — see below) | **one** protected consequence: terminal judgment, per-trigger resolutions, resulting revision when the disposition requires one, and its `refresh` lineage edge, published atomically together | episode identity + CAS on the episode's own revision — no separate writer abstraction (see below) | episode record, terminal; revision; lineage edge — one operation receipt |

All three columns of authority/atomicity/idempotency differ between
nomination and episode-opening, and episode-opening's authority and CAS
requirements differ again from judgment-publication's guarded close. That is
three operations, not one, not four — the causal production of a successor
revision belongs *inside* the judgment operation's own atomicity boundary,
not to a separately-sequenced call to an existing operation (see Operation 3
below). **This conclusion depends on a choice made explicit in Operation 3:**
this two-state (`opened` → terminal) first vertical needs no writer-ownership
abstraction, only plain revision CAS — had a genuinely multi-transition
lifecycle been in scope (the deferred intermediate states), a fourth
operation admitting writer takeover might have been required, per Sol's own
second review. Adopting the plain-CAS design is what keeps this at three
operations. This still matches Sol's own speculative sketch
(`nominate_impact`/`open_refresh_episode`/`publish_refresh_judgment`), but
arrived at by the test rather than presumed from it.

## House style this surface must match

Read directly from `app-server/src/services/claim-evidence/{contract,service,validation,production-path-contract}.mjs`
before drafting anything below, so the new operations extend rather than
invent conventions:

- An operation request is `{schema_version, operation_id, action, profile, expected_state, payload}`; `action` is a member of `PERMISSIONS`; `profile` must match the authority's granted profile.
- Idempotency is a caller-supplied `operation_id`; a retry with the same `operation_id` and identical payload digest returns the prior result marked `idempotent: true` without re-mutating; a same-`operation_id`-different-payload retry is a hard conflict (`operationPayloadDigest`, `service.mjs:56-61`).
- Optimistic concurrency is `expected_state`, checked against the current head/generation before mutation (`service.mjs:83`, revision heads; production-path succession's `predecessorRevision`/`successorRevision` pairing).
- Every produced record's own `id` is a content digest of the record minus its own `id` field (`revisionId`, `production-path-establishment-v1@digest(...)`, `lineage@digest(edge)`), never a random or caller-chosen value.
- `authority` is validated separately from the operation payload (`validateAuthority`) — profile match, permission membership, decision-scope match, a verified `authority_reference`.
- Records are exact-field validated (`exactFields`) — no optional/undeclared fields.
- `LINEAGE_RELATIONSHIPS` already contains `"refresh"` alongside `correction`/`supersession`/`composition`/`derivation`/`identity_fork`/`retraction` (`contract.mjs:9-12`). **Corrected 2026-09-15** (Sol's review): the first draft claimed this was "independent evidence the original substrate had already anticipated exactly this composition" — that overstates what one unused enum value supports. It is evidence that refresh lineage was anticipated by the substrate, and that this design may extend rather than contradict the existing relationship vocabulary. It says nothing about the three operations, their authority split, their concurrency mechanism, or their atomicity boundaries — those are established below on their own evidence, not borrowed from this one fact.
- `review-episode` (`app-server/src/services/review-episode/{contract,service}.mjs`) is domain-locality-correct precedent for writer-generation fencing generally — a closer analogy than `workspace-coordination` for any domain-local concurrency problem in this family. **Second review finding:** its writer-generation mechanism only means something because `replace_writer` is itself a real, separately-admitted transition; this surface's own two-state vertical has no equivalent transition to admit a takeover through, so it does not borrow the mechanism after all — see Operation 3's "Concurrency" below. `review-episode`'s pattern remains the right answer *if* a future, richer refresh-episode vertical needs genuine multi-actor writer ownership.

## Operation 1: `nominate_impact`

**What it does.** Publishes one immutable candidate-impact assertion against
one exact target claim revision, from one identified source event. Per
`semantic-model.md`: "Nomination asserts candidate impact only. It cannot
make the claim false, stale, inapplicable, insufficient, reopened,
refreshed, or causally changed."

**Request:**

```text
{
  schema_version: 1,
  operation_id: <caller-supplied idempotency key>,
  action: "nominate_impact",
  profile: <target claim's profile>,
  expected_state: null,   // nominations are append-only; never supersede one another
  payload: {
    nomination_identity: <caller-asserted stable identity for THIS semantic nomination>,
    target_claim_revision: <exact revision id — must exist in store.revisions>,
    source_event: {
      kind: <one of: repository_change, contract_change, proposal_transition,
             external_observation, review_result, runtime_event>,
      identity: <exact source event id>,
      revision: <exact source revision/digest>,
    },
    evidence_references: [ <Reference — existing validateReference shape> ],
    producer: { <existing provenance shape: producer, model/kind, version, inferenceId-or-equivalent> },
    decision_scope: <matches authority.decision_scope, same convention as every existing payload>,
  }
}
```

**Who may request it.** A new, deliberately weak `nominate_impact`
permission — an evidence-producer authority class (an `EvidenceAnchorObserver`
implementation, a code-change analyzer, or any other named source-event
observer). Per `evidence-anchor-observation-and-impact-nomination.md`'s own
closed boundary: "evidence-anchor observers supply observations/nominations;
they do not become the claim-state owner" — this permission must not be
grantable alongside `open_refresh_episode` or `publish_refresh_judgment`
under the same authority grant, preserving that separation structurally, not
just by convention.

**Who may admit it.** Nobody separately "admits" a nomination — publication
is itself the admission. There is no domain-owner gate at this step; that is
what distinguishes nomination from episode-opening.

**Identity, idempotency, and duplicate nominations — kept as three separate
questions, per Sol's correction.** The first draft used a content digest as
`nomination_id` and treated content equality as proof of semantic sameness,
collapsing three things that are not automatically the same mechanism:

```text
Is this the same semantic nomination?
Is this the same attempted publication (retry)?
Is this a delivery-attempt identity distinct from either?
```

Corrected: `nomination_identity` is caller-asserted, exactly like
`episode_identity` below — the producer states which semantic nomination
this is, rather than having it derived from content. `nomination_id` is then
`impact-nomination-v1@digest(withoutId)`, a content-derived *record pointer*
for stable reference, not a dedup key. Idempotency for retry-safety remains
the *existing*, unmodified mechanism: `operation_id` + payload digest,
exactly as every other operation already uses it (`service.mjs:56-61`) — a
byte-identical retry under the same `operation_id` is a no-op; a different
`operation_id` asserting the *same* `nomination_identity` with *different*
content is a conflict (mirroring `create_claim`'s existing "identity already
exists" rejection), not a silent collapse.

**Left explicitly open, not decided here:** whether two independently
produced nominations — different producers, different `nomination_identity`
values, but the same `{target_claim_revision, source_event}` pair — should
collapse into one semantic nomination or remain two visible, distinct
nominations resolved separately. `semantic-model.md` does not establish this
either way, and inventing an answer would be exactly the kind of unsupported
connective tissue this design should not add. Both are structurally
representable under caller-asserted identity (distinct identities always
remain distinct records); resolving *whether they should* logically collapse
is future domain-policy work, not a decision this contract makes for its
callers.

**Result and state.** A new record in a new `nominations` collection (store
shape addition, alongside `claims`/`revisions`/`lineage`/`reliances`), with a
derived `disposition` — starting `pending`. Per `semantic-model.md`:
"Current disposition is derived from immutable successors rather than
destructive field mutation" — so the nomination record itself is never
mutated after creation; its displayed disposition is a read-time projection
over whichever refresh episode later resolves it — specifically, the
nomination's own entry in that episode's `trigger_resolutions` (Operation 3),
**not** the episode's own overall disposition. `semantic-model.md` uses
distinct vocabularies for the two: nomination dispositions are "pending,
investigating, resolved unchanged, resolved changed, inapplicable,
insufficient evidence, contested, deferred, and superseded"; episode-level
terminal outcomes are "retained_unchanged / changed / inapplicable /
insufficient / contested / deferred / superseded" — visibly different
spellings (`resolved_changed` vs. `changed`, "insufficient evidence" vs.
"insufficient") for what the source text treats as two separate fields, not
one reused enum. This design keeps them separate throughout, per Sol's
second-review correction (see Operation 3).

**Failure states.** `target_claim_revision` not found in `store.revisions` →
reject (dangling target, same shape as lineage's existing dangling-target
check). Authority profile mismatch, decision-scope mismatch, unverified
authority reference → reject, identical to every existing operation's
authority checks.

## Operation 2: `open_refresh_episode`

**What it does.** Admits one refresh episode against one exact subject
revision, triggered by one or more prior nominations. Per
`semantic-model.md`: "Opening records `reopened_by` only; it does not prove
investigation or a terminal result."

**Request:**

```text
{
  schema_version: 1,
  operation_id: <idempotency key>,
  action: "open_refresh_episode",
  profile: <subject claim's profile>,
  expected_state: null,   // opening creates a new episode; not a CAS update to an existing one
  payload: {
    episode_identity: <caller-supplied stable episode identity>,
    subject_revision: <exact revision id being investigated>,
    reopened_by: [ <nomination_id>, ... ],   // one or more triggering nominations; each must exist
    domain_profile: <the owning domain's own profile identity — may differ from the claim's storage profile; this document does not decide whether these must be equal, see "What this design deliberately does not decide">,
    evidence_cutoff: <string>,
  }
}
```

**Who may request/admit it.** A new `open_refresh_episode` permission — the
*authorized domain owner* class, structurally distinct from
`nominate_impact`'s producer class. This is the first point in the chain
where "who may request" and "who may admit" collapse into one actor, because
episode-opening is itself the admission act (there is no separate approval
step before an authorized owner may open an episode against evidence
already nominated).

**Identity.** `episode_id` is `refresh-episode-v1@digest({episode_identity, subject_revision, reopened_by, domain_profile, evidence_cutoff})`, following the `production-path-establishment` precedent (caller-supplied identity string plus a content-derived record id, `production-path-contract.mjs:141-152`). The episode record's own content digest (its `episode_revision`, computed the same way every other record's identity is — content minus the id field) is what `publish_refresh_judgment` will present as `expected_state`; no separate writer/generation object is minted here. **Second-review correction (2026-09-15):** an earlier draft minted a `review-episode`-style writer-generation authority at this step. Sol's review found that mechanism only means something because `review-episode` has a real, separately-admitted `replace_writer` transition giving generation-advancement meaning; this surface's two-state vertical (`opened` → terminal, no intermediate writable states per the deferral below) has no such transition to admit, so borrowing the token without the transition would have been "the token but not the transition." See Operation 3's "Concurrency" for the plain-CAS mechanism used instead.

**Result and state.** A new record in a new `refresh_episodes` collection,
`lifecycle_state: "opened"`, no disposition, no candidate/verification
content — matching `semantic-shadow-episode`'s own precedent of an
`exactFields`-validated record whose non-terminal fields are explicitly
`null` until closed (`semantic-shadow-contract.mjs:161-205`'s
`validateSemanticShadowEpisode` is a close structural precedent for this
record's own eventual validator, though it validates a different object).

**Failure states.** Any `reopened_by` nomination id not found → reject
(dangling trigger). `subject_revision` not found in `store.revisions` →
reject. Authority not of the domain-owner permission class → reject.

## Operation 3: `publish_refresh_judgment`

**What it does.** Closes exactly one open episode with an attributed
terminal episode disposition *and* an individually attributed resolution for
every nomination that triggered it, and, when the episode disposition
requires one, **causally produces** the resulting successor claim revision
and its `refresh` lineage edge — all as part of the same protected
consequence. Per `semantic-model.md`: "Refresh judgment belongs to the
authorized domain owner," "`retained_unchanged` and `changed` each produce
exactly one new claim revision against the evidence world actually
examined... Inapplicable, insufficient, contested, and deferred outcomes
produce no claim revision," and "Every outcome resolves or visibly preserves
each triggering nomination" — the last of these is what makes per-trigger
resolution a requirement, not an enhancement.

**Corrected 2026-09-15 (Sol's review, the most serious finding on the first
draft).** The first draft had this operation *reference* a revision already
published through a separately-sequenced call to the existing
`publish_revision` operation. That inverts the semantic model's own stated
causality:

```text
first draft:  claim revision exists authoritatively -> later judgment says it produced it
approved semantic model:  authorized refresh judgment -> produces -> claim revision
```

and leaves a real crash window: a process that publishes the revision and
crashes before publishing the judgment leaves an authoritative revision with
no admitted judgment establishing why it exists — exactly the kind of
failure-on-either-side-of-a-publication-boundary case `acceptance.md`'s own
evidence bar requires covering. Sol's guidance, applied directly: distinguish
*reuse of implementation mechanics* from *composition of public operations*.
The fix is not to duplicate `publish_revision`'s logic in a new place; it is
to have `publish_refresh_judgment` internally call the *same* shared
revision-construction helper `service.mjs` already factors out
(`makeRevision(claimId, predecessor, payload, authority)`, used today by both
`create_claim` and `publish_revision`, `service.mjs:39-47`) and the same
lineage-edge construction the `publish_lineage` action branch already uses,
**inside its own operation branch**, so the revision, its `refresh` lineage
edge, and the episode's terminal state are appended to the store together in
one `applyOperation` call — one operation receipt, one atomic mutation,
impossible to leave half-done. The operation count stays three, as the
Method table above already found; what changed is where the atomicity
boundary sits.

**Request:**

```text
{
  schema_version: 1,
  operation_id: <idempotency key>,
  action: "publish_refresh_judgment",
  profile: <same as the episode's subject claim>,
  expected_state: <the episode's own exact episode_revision digest — see Concurrency>,
  payload: {
    episode_id: <must be an open episode>,
    episode_disposition: one of [retained_unchanged, changed, inapplicable,
                          insufficient, contested, deferred, superseded],
    rationale: <nonempty text>,
    evidence_references: [ <Reference>, ... ],
    trigger_resolutions: [
      {
        nomination_id: <must be exactly one of the episode's own reopened_by entries>,
        resolution: one of [resolved_unchanged, resolved_changed, inapplicable,
                             insufficient_evidence, contested, deferred, superseded],
      },
      ...   // exactly one entry per reopened_by nomination_id — none omitted, none extra, none duplicated
    ],
    successor_revision_payload: <exact same shape publish_revision's own payload.revision already requires, present only for retained_unchanged/changed, else null — this operation constructs and publishes the revision itself, using the existing makeRevision helper; the caller supplies the revision's content, not a pre-existing id>,
    decision_scope: <matches authority.decision_scope>,
  }
}
```

**Corrected 2026-09-15 (Sol's second review).** The prior draft had one
top-level `disposition` field applied uniformly to the episode, and a
singular `changed_because_of: <source_event>`. Sol's counterexample: one
episode triggered by a repository-change nomination and a documentation-change
nomination against the same claim may truthfully find the repository change
causal and the documentation change irrelevant — episode disposition
`changed`, but nomination A resolves `resolved_changed` while nomination B
resolves `inapplicable`. Deriving both triggers' dispositions from the one
episode disposition would be wrong, and the approved proposal's own
requirement that every triggering nomination be "resolved or visibly
preserved" only makes sense if trigger-level resolution is its own fact.
Fixed: `trigger_resolutions` is now a required, exhaustive, per-nomination
field, using its *own* vocabulary (`resolved_unchanged`/`resolved_changed`/
`inapplicable`/`insufficient_evidence`/`contested`/`deferred`/`superseded`) —
visibly distinct spellings from `episode_disposition`'s own vocabulary
(`retained_unchanged`/`changed`/`inapplicable`/`insufficient`/`contested`/
`deferred`/`superseded`), matching `semantic-model.md`'s own differing
spellings for the two concepts rather than treating them as one reused enum.
The singular `changed_because_of` field is removed entirely: causal
attribution is now which `trigger_resolutions` entries carry
`resolution: "resolved_changed"` — a naturally-plural, per-trigger fact, not
a top-level field forced into either a singular or array shape by this
document's own assumption. If exactly one trigger resolves `resolved_changed`,
causality reads as singular; if several do, it reads as plural — the schema
does not presuppose either.

**Who may request/admit it.** The same `publish_refresh_judgment` domain-
owner permission class as `open_refresh_episode` — still not required to be
the identical specific actor who opened the episode (the "same actor ≠ same
authority ≠ same act" distinction this session's routing-seam correction
already established). Domain authority answers *who may judge*; see
Concurrency below for the separate question of *whether this judgment still
applies to the current episode state*.

**Concurrency (second-review correction, replaces a rejected
writer-generation design).** An earlier draft minted a `review-episode`-style
writer-generation authority at `open_refresh_episode` and required
`publish_refresh_judgment` to present a successor generation to close it.
Sol's review applied the contracts-not-procedures test directly: *what
invariant becomes false if refresh episodes do not have writer generations
in this first vertical?* None — this vertical models exactly two durable
states (`opened`, terminal; the intermediate states are deferred below), so
there is no ongoing multi-transition lifecycle for a writer to hold across.
`review-episode`'s generation mechanism means something only because
`replace_writer` is itself a real, separately-admitted transition; without
an equivalent transition here, presenting "generation 2" would have proven
nothing beyond the closer's own say-so — self-authorizing, not admitted.

The corrected design uses plain optimistic concurrency, identical in kind to
`publish_revision`'s own existing `expected_state`-against-heads check:
`expected_state` must equal the episode's exact `episode_revision` digest as
it stood at open. If two judges race to close the same episode, the second's
`expected_state` no longer matches (the first's close already advanced the
record), and it is rejected as a plain CAS conflict — the same mechanism
`publish_revision` already uses for every claim revision, not a new concept.
Domain authority (the `publish_refresh_judgment` permission) answers who may
judge; CAS answers whether this judgment still targets current episode
state. No writer-ownership abstraction sits between them.

**Result and state, as one atomic consequence.** In a single
`applyOperation` call: the episode record becomes terminal
(`lifecycle_state: "closed"`, `episode_disposition`, `trigger_resolutions`,
rationale, evidence references — its `episode_revision` digest advancing,
which is what makes the CAS above meaningful for any subsequent attempt);
when `episode_disposition` is `retained_unchanged`/`changed`, a new revision
is constructed via the existing `makeRevision` helper and appended to
`store.revisions`, and a `refresh` lineage edge is appended to
`store.lineage` linking the episode's subject revision to that new
successor — all appends, or none, under one operation receipt. Every
triggering nomination's disposition now resolves through its own entry in
this episode's `trigger_resolutions` (a read-time projection, per Operation
1) — never through the episode's own overall disposition directly.

**Canonical support is a separate act, not implied by this operation.**
Per `semantic-model.md`'s own "Branching, conflict, and canonical support"
section: "The domain's authorized maintenance owner records an explicit
canonical-support selection... That selection is immutable, attributed,
authority-bound, and replaceable by a successor." Publishing a successor
revision here does **not** by itself make that revision the domain's
canonical support — a resulting revision existing and a resulting revision
*governing* are kept distinct. **Tightened per Sol's second review:** under
a non-branching domain profile, canonical support may be mechanically
derived from the sole admissible head *only if the domain profile explicitly
establishes that rule* — not merely "in practice," which would let revision
publication quietly become canonical-support selection through convention.
Under a branching-permitted profile, an explicit canonical-support-selection
mechanism is required before the produced revision governs, and this
document does not design that mechanism — it is named here as a boundary
this surface must not silently cross, not solved.

**Failure states.** `episode_id` not found or already closed → reject.
`expected_state` not matching the episode's current `episode_revision` →
reject (plain CAS conflict, identical in kind to `publish_revision`'s
existing conflicting-predecessor rejection). `trigger_resolutions` missing
an entry for any of the episode's own `reopened_by` nominations, containing
an entry for a nomination not in `reopened_by`, or containing a duplicate
nomination_id → reject (exhaustiveness is required, not advisory).
`episode_disposition == "changed"` with no `trigger_resolutions` entry
resolving `resolved_changed` → reject (never manufacture causality
mechanically — at least one adjudicated trigger must support it).
`successor_revision_payload` present for an `episode_disposition` other than
`retained_unchanged`/`changed` → reject. `successor_revision_payload` absent
for `retained_unchanged`/`changed` → reject. Any failure in constructing the
successor revision or lineage edge aborts the entire operation — no partial
state is ever visible, matching `applyOperation`'s existing all-or-nothing
behavior for every other action. Left open, not decided here: the full cross-
consistency matrix between `episode_disposition` and the *set* of
`trigger_resolutions` values beyond the one rule above (for example, whether
an episode may resolve `retained_unchanged` while some individual trigger
resolves `resolved_changed`) — `semantic-model.md` does not specify this
matrix, and inventing one would be unsupported connective tissue.

## What this design deliberately does not decide

- Whether `domain_profile` (the episode's own judging-domain identity) must
  equal the subject claim's storage `profile`, or may legitimately differ.
  `semantic-model.md` lists them as separate fields without stating the
  relationship; inventing an equality constraint here would be exactly the
  kind of unsupported connective tissue this session's discipline exists to
  avoid.
- Whether opening an episode against an already-retracted subject revision
  should be rejected outright or permitted with a visible flag —
  `validateStore` does not currently track a "retracted" status on
  revisions distinctly from ordinary lineage `retraction` edges, and this
  document does not propose adding one.
- Any schema for `store.nominations` / `store.refresh_episodes` beyond the
  field lists implied above, or the corresponding `validateStore` extension
  — that is implementation detail belonging to the next stage, not this
  contract-formation stage.
- New `PERMISSIONS` values' exact string spelling (`nominate_impact` /
  `open_refresh_episode` / `publish_refresh_judgment` are working names,
  consistent with the existing `snake_case` action vocabulary, not
  necessarily final).
- The exact canonical-support-selection mechanism required under a
  branching-permitted domain profile (Operation 3's "Canonical support is a
  separate act") — named as a real, required boundary, not designed here.
- The full cross-consistency matrix between `episode_disposition` and the
  set of `trigger_resolutions` values, beyond the one rule this design
  states (a `changed` episode requires at least one `resolved_changed`
  trigger) — see Operation 3's failure states.

## Intermediate episode lifecycle states — resolved for this first vertical

**Corrected 2026-09-15** (Sol's review): the first draft left
`semantic-model.md`'s named intermediate states (`active`, `awaiting_evidence`,
`awaiting_authority`, between `opened` and a terminal disposition,
`semantic-model.md:44-49`) as "descriptive, not additional operations, until
evidence says otherwise" — Sol correctly pointed out this is not a resolved
position, since the source text calls them episode lifecycle states, not
commentary, and if they are durable/queryable something must establish them.
This design makes the explicit choice Sol asked for, rather than leaving the
question open by omission:

**Chosen: (C) deferred from this first production vertical.** This surface
models exactly two durable episode states — `opened` (Operation 2) and
terminal (Operation 3) — and does not attempt to give `active`/
`awaiting_evidence`/`awaiting_authority` their own authority, transition
operation, or durable representation. A domain workflow may track its own
in-progress investigation status in its own state, outside this shared
substrate, without this surface needing to represent it. If a later vertical
finds real consumers need to query or rely on those intermediate states as
durable facts (option A: authoritative transitions with a named publisher,
or option B: deterministic projections derived from other owned facts), that
is new, separately-bounded work extending this surface — not a gap silently
left in it. The surface described in this document is closed and complete
for its own stated scope (nomination through terminal judgment), not for
`semantic-model.md`'s full episode lifecycle vocabulary.

**This choice is what makes Operation 3's plain-CAS concurrency design
correct, not merely convenient.** A two-state lifecycle (`opened` → terminal,
no durable states in between) has no window in which a writer might need to
be replaced mid-investigation — the only concurrency hazard is two closers
racing to publish the *same* terminal transition, which plain revision CAS
already resolves. If a future vertical promotes any of the deferred states to
durable, authoritative transitions (option A above), that reopens the
writer-ownership question this document currently answers "not needed" —
it would need its own re-derivation, not an assumption that plain CAS still
suffices.

## Boundaries this design must not cross (carried forward, not reopened)

- `may_affect`/nomination remains candidate impact only, never semantic
  invalidation — enforced structurally: `nominate_impact`'s payload has no
  disposition field to assert one.
- Opening a refresh episode proves nothing about investigation or outcome —
  enforced structurally: `open_refresh_episode`'s payload has no
  disposition/outcome field.
- Refresh judgment belongs to the authorized domain owner, a distinct
  authority class from the nomination producer — enforced via separate
  `PERMISSIONS` entries that must not be co-granted under evidence-producer
  authority.
- A changed claim does not automatically reopen downstream work, and exact
  reliance does not silently follow a successor revision — this surface
  never touches `record_reliance`/`retire_reliance`; reopening remains
  future, separately-owned work.
- Delivery acknowledgment never equals semantic completion — no delivery
  mechanism is proposed here at all.
- The existing six claim-evidence operations remain the substrate this
  surface extends, not replaces. **Revised 2026-09-15:** this is now
  enforced by *sharing implementation mechanics* (the same `makeRevision`
  helper and the same lineage-edge construction `publish_revision`/
  `publish_lineage` already use) rather than by *composing them as separate
  public operation calls* — the latter was the atomicity flaw Sol's review
  found in Operation 3. The revision schema, lineage relationship vocabulary,
  and validation rules are unchanged and unduplicated; only where the
  mutation is committed moved, from two sequenced public calls to one.

## Relationship to other open work

- `evidence-anchor-observation-and-impact-nomination.md`'s production
  anchor registry and durable `ImpactNomination` records remain blocked on
  this surface landing, per its own §6 — this design is the landing this
  surface refers to.
- Items 5 (readiness contract) and 6 (comparison-contract mechanism) both
  treat claim-evidence freshness as an input; this surface is what makes
  that freshness durable and queryable, once implemented.
- This document does not authorize implementation. The next bounded step,
  if this design holds up under review, is a separate implementation-
  authorization decision, followed by the actual `.mjs` changes and
  `acceptance.md`'s full adversarial evidence matrix — unaffected and
  unshortened by anything here.
