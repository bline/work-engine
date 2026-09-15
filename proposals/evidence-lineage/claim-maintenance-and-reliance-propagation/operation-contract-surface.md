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

**Companion principle (Sol's, contributed on the fourth review, after the
claim-head admission gap was found):** reusing a mechanism does not inherit
the contract that previously governed its use. `makeRevision` gives you a
revision; it does not, by itself, give you permission to publish that
revision from that predecessor under that claim's branching policy. Every
internal reuse in this document (`makeRevision`, lineage-edge construction,
`revisionHeads`-style CAS) is checked against this directly — see Operation
3's "Claim-head admission, not just episode CAS."

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
corrected in the third draft.

**2026-09-15, third draft reviewed by Sol — still not passed, three
source-level contradictions and one implementation-boundary ambiguity
found.** (1) Folding causal attribution into `trigger_resolutions`
(`resolution: "resolved_changed"`) repeated the exact failure mode again at
a finer grain: `semantic-model.md` keeps "this nomination's candidate
impact was resolved, and the claim changed" (`resolved_changed`, a
per-nomination resolution) strictly separate from "this source event's
causal effect was actually adjudicated" (`changed_because_of`, keyed on
source events) — a nomination can resolve `resolved_changed` without its own
source event being the one adjudicated causal. Restored a distinct causal-
attribution field, not derived from `trigger_resolutions`. (2) The "two-state
first vertical" framing had drifted from *lowering* the approved semantic
model into *silently narrowing* it: `semantic-model.md` names "current writer
generation" and the three intermediate lifecycle states as real, approved
semantics, not merely-possible extensions — this design cannot decide they
are unneeded in general, only that this bounded slice does not implement
them yet. Reframed throughout: "not represented by this first production
vertical; still part of the approved broader semantic model," never
"resolved" or "not needed." (3) `episode_disposition: superseded` had no
required successor reference, though `semantic-model.md` states "supersession
identifies the exact successor episode, judgment, or revision" — added a
required `superseded_by` field. (4) Defined only the logically-forced
episode↔trigger consistency invariants (a `changed` episode requires ≥1
causal source event; a `retained_unchanged` episode prohibits any
`resolved_changed` trigger; every triggering nomination must be exhaustively
represented) rather than leaving the door open to structurally-admissible
contradictions, without inventing the full matrix beyond what the approved
semantics force. Also made explicit, per Sol's request, that closing an
episode publishes a new immutable episode-state revision succeeding the
opened state, never a destructive overwrite of it — matching
`semantic-model.md`'s own "immutable lifecycle transitions" requirement
directly, the same way claim revisions already chain via
`predecessor_revision`. All corrected in the fourth draft.

**2026-09-15, fourth draft reviewed by Sol — "Architecture PASS. Bounded
three-operation decomposition PASS. Contract surface NOT YET
implementation-ready" — three invariants and one identity clarification
remained, none requiring further redesign.** (1) The third round's
`retained_unchanged`-prohibits-`resolved_changed` rule was too narrow — it
still admitted `inapplicable`/`contested`/`deferred` episodes with a
`resolved_changed` trigger, equally impossible since those dispositions
produce no claim revision at all. Strengthened to the tight biconditional
`resolved_changed ⇒ episode_disposition == "changed"`. (2) Internally
reusing `makeRevision` fixed the atomicity problem but does not by itself
inherit `publish_revision`'s own admission invariant that the target claim's
head hasn't moved — Sol's naming: "reusing a mechanism does not inherit the
contract that previously governed its use." Added an explicit claim-head
admission check before minting any successor revision, and scoped this
first vertical's successor-revision path to non-branching domain profiles
only, stated as a scope choice rather than left ambiguous. (3) A
`retained_unchanged` judgment's successor revision was accepted at face
value with no check that its proposition actually matches its predecessor's
— `semantic-model.md` and the proposal's own invariants require preserving
proposition identity, not merely producing *a* revision. Required the
claim's own domain profile to identify proposition-identity fields and
enforced the comparison, without inventing the comparator itself. (4)
`superseded_by.kind == "judgment"` pointed at a category with no defined
concrete identity. Resolved: it names the terminal `episode_state_id` of
whichever episode is the successor judgment — no new identity concept
introduced. All four addressed in the fifth draft.

**2026-09-15, fifth draft reviewed by Sol — "Architecture: PASS.
Three-operation bounded first vertical: PASS. Operation semantics: PASS,
except remove the unsupported `changed ⇒ resolved_changed` implication.
Implementation readiness: BLOCKED only on the profile-owned refresh-policy
interface and the exact sole-head non-branching check."** Three of the
fourth round's four fixes were confirmed closed outright (claim-head
admission, proposition preservation, supersession identity). Two items
remained, plus one self-correction from Sol on the round-4 review itself.
(1) Sol found their own fourth-round rule wrong: "`changed` requires ≥1
`resolved_changed` trigger" is not supported by the approved semantics — the
authorized domain owner may adjudicate causality against any source event,
including one discovered mid-investigation that was never among the
episode's own triggering nominations, so a `changed` episode's triggers may
all resolve non-causally (e.g. all `inapplicable`) while `causal_attribution`
names a distinct causal event. Requiring a `resolved_changed` trigger would
partially re-collapse `trigger_resolutions` into `causal_attribution` — the
exact distinction the third round existed to establish. Removed; the forced
rule is one-directional only (`resolved_changed ⇒ changed`), not a
biconditional. (2) The proposition-preservation requirement named a
domain-profile-owned comparator without defining how the claim-evidence
service actually asks a profile for it — "a requirement, not yet a
contract." Added a defined profile-level refresh-policy extension point (see
"Profile-owned refresh policy" below) that this vertical's admitted profiles
populate, without deciding what any specific profile's proposition-identity
fields are. (3) The non-branching claim-head check accepted
`subject_revision ∈ current_heads(claim)` when the forced invariant for a
non-branching profile is stricter: `current_heads(claim) == {subject_revision}`
— membership alone would wrongly admit a successor when the store already
holds multiple heads for reasons unrelated to this episode. Tightened
accordingly. All three addressed below.

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
below). **This conclusion depends on a scope choice made explicit in
Operation 3, corrected on third review to state accurately:** this document
implements only a bounded first production vertical —
`nominate_impact`/`open_refresh_episode`/`publish_refresh_judgment` — that
does not exercise writer-generation replacement or the three approved
intermediate episode lifecycle states. That is a statement about what this
slice implements, not a claim that writer-generation or the intermediate
states are unneeded in general: `semantic-model.md` names "current writer
generation" and the intermediate lifecycle states as real, approved
semantics, and this design does not amend or narrow that approval. A future
vertical implementing the full lifecycle may require a fourth operation
admitting writer takeover, exactly as `review-episode`'s own
`replace_writer` does. Three operations is this bounded slice's own count,
not a claim about the eventual full surface. This still matches Sol's own
speculative sketch
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
- `review-episode` (`app-server/src/services/review-episode/{contract,service}.mjs`) is domain-locality-correct precedent for writer-generation fencing generally — a closer analogy than `workspace-coordination` for any domain-local concurrency problem in this family. **Second review finding:** its writer-generation mechanism only means something because `replace_writer` is itself a real, separately-admitted transition; this bounded first vertical does not implement an equivalent transition, so it does not implement the mechanism *in this slice* — see Operation 3's "Concurrency" below. **Third-review correction:** this is a statement about what this slice implements, not that writer-generation is unneeded — `semantic-model.md` names "current writer generation" as real, approved semantics for the full episode lifecycle. `review-episode`'s pattern is the answer once a future vertical implements writer takeover, not a discarded alternative.

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

**Identity, and episode state as a revision chain, not a mutable row.**
`episode_logical_id` is `refresh-episode-v1@digest({episode_identity, subject_revision, reopened_by, domain_profile, evidence_cutoff})`, following the `production-path-establishment` precedent (caller-supplied identity string plus a content-derived record id, `production-path-contract.mjs:141-152`) — this is the stable identity of the episode across its whole lifecycle, not one state snapshot. Opening publishes the *first* immutable episode-state record: `{episode_logical_id, predecessor_state: null, lifecycle_state: "opened", ...payload fields..., authority_ref, operation_id}`, with its own content-derived `episode_state_id` — exactly the same predecessor-chained-revision pattern `store.revisions` already uses for claims (`service.mjs`'s `makeRevision`/`revisionHeads`), not a single row that later gets overwritten. **Third-review correction (2026-09-15):** `semantic-model.md` requires "immutable lifecycle transitions" for refresh episodes explicitly — an earlier draft described the episode as one record whose digest changes on close, which reads as an overwrite even though CAS would have prevented races. Modeled as a proper chain instead: the opened state remains individually addressable forever; closing (Operation 3) publishes a *second*, successor state record referencing the first via `predecessor_state`, mirroring exactly how a claim's successor revision references its predecessor. **Second-review correction, still applicable:** no separate writer/generation authority object is minted here — see Operation 3's "Concurrency" for why plain revision-CAS (checking the episode's current state head, exactly as `revisionHeads` does for claims) is sufficient for this bounded vertical's two states, without claiming writer-generation is unneeded for the full approved lifecycle.

**Result and state.** A new record in a new `refresh_episode_states`
collection (the first in its episode's own state chain, per the Identity
section above): `lifecycle_state: "opened"`, `predecessor_state: null`, no
disposition, no candidate/verification content — matching
`semantic-shadow-episode`'s own precedent of an `exactFields`-validated
record whose non-terminal fields are explicitly `null` until closed
(`semantic-shadow-contract.mjs:161-205`'s `validateSemanticShadowEpisode` is
a close structural precedent for this record's own eventual validator,
though it validates a different object). This record is never mutated; the
episode's "current state" is whichever state in its chain has no successor,
exactly as `revisionHeads` finds a claim's current revision.

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
  expected_state: <the episode's current state head's exact episode_state_id — see Concurrency>,
  payload: {
    episode_logical_id: <the episode being closed>,
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
    causal_attribution: [ <source_event identity>, ... ],   // required non-empty only when episode_disposition == "changed"; else must be empty. Records exactly which source events' causal effect was affirmatively adjudicated -- NOT derived from trigger_resolutions, see below.
    superseded_by: { kind: one of [episode, judgment, revision], id: <exact successor identity> } | null,   // required exactly when episode_disposition == "superseded", else null
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
The singular `changed_because_of` field was removed entirely on that round,
in favor of reading causal attribution off `trigger_resolutions` directly.

**Corrected again, third review (2026-09-15) — restored, in a different
shape.** Sol found that this repeated the exact failure mode one level
finer: `semantic-model.md` keeps a nomination's own resolution
(`resolved_changed` — "this candidate impact has been resolved, and the
claim changed") strictly separate from causal attribution ("a changed
judgment records `changed_because_of` only for source events whose causal
effect the authorized domain owner actually adjudicated"). A nomination can
truthfully resolve `resolved_changed` — the investigation concluded the
claim changed — without that nomination's own source event being the one
adjudicated causal; multiple source events can feed one investigation with
only some found causal. Deriving causal attribution from
`trigger_resolutions` would silently conflate "this trigger's own outcome"
with "this source event was the adjudicated cause," which are not the same
claim. Restored as `causal_attribution`: an explicit set of source-event
identities affirmatively adjudicated as causal, required non-empty exactly
when `episode_disposition == "changed"`, and **not derived from**
`trigger_resolutions` — the two are independently populated and independently
validated. `semantic-model.md` itself refers to "source events" in the
plural without dictating a schema shape, so this is deliberately a set, not
a reflexive re-singularization of the original field — if exactly one event
is adjudicated causal the set has one member; nothing about the shape
presumes that in advance.

**Who may request/admit it.** The same `publish_refresh_judgment` domain-
owner permission class as `open_refresh_episode` — still not required to be
the identical specific actor who opened the episode (the "same actor ≠ same
authority ≠ same act" distinction this session's routing-seam correction
already established). Domain authority answers *who may judge*; see
Concurrency below for the separate question of *whether this judgment still
applies to the current episode state*.

**Concurrency (second-review correction, sharpened on third review — a
bounded-slice choice, not a claim about the approved semantic model).** An
earlier draft minted a `review-episode`-style writer-generation authority at
`open_refresh_episode` and required `publish_refresh_judgment` to present a
successor generation to close it. Sol's second review applied the
contracts-not-procedures test directly: *what invariant becomes false if
this bounded slice's two states (`opened`, terminal) do not have writer
generations?* None — there is no ongoing multi-transition lifecycle within
*this slice* for a writer to hold across. `review-episode`'s generation
mechanism means something only because `replace_writer` is itself a real,
separately-admitted transition; without an equivalent transition here,
presenting "generation 2" would have proven nothing beyond the closer's own
say-so. **Third-review correction:** this is not a finding that
writer-generation is unneeded — `semantic-model.md` names "current writer
generation" as real, approved semantics for the full episode lifecycle this
document does not fully implement. It is a finding that *this bounded
vertical's own two states* do not need it.

The corrected design uses plain optimistic concurrency, identical in kind to
`publish_revision`'s own existing `expected_state`-against-heads check:
`expected_state` must equal the episode's current state-chain head — its
exact `episode_state_id`, per Operation 2's Identity section — as it stood
after opening. If two judges race to close the same episode, the second's
`expected_state` no longer matches (the first's close already published a
new head), and it is rejected as a plain CAS conflict — the same mechanism
`publish_revision` already uses for every claim revision, not a new concept.
Domain authority (the `publish_refresh_judgment` permission) answers who may
judge; CAS answers whether this judgment still targets the current episode
state. No writer-ownership abstraction sits between them, for this slice.

**Result and state, as one atomic consequence, published as a successor
state — never an overwrite.** In a single `applyOperation` call: a *new*
episode-state record is appended (never mutating the opened state),
`predecessor_state` pointing at the opened state's `episode_state_id`,
carrying `lifecycle_state: "closed"`, `episode_disposition`,
`trigger_resolutions`, `causal_attribution`, `superseded_by`, rationale, and
evidence references; when `episode_disposition` is
`retained_unchanged`/`changed`, this operation first checks claim-head
admission (`subject_revision` still current, or rejects — see "Claim-head
admission" above) and, for `retained_unchanged`, proposition preservation
(see "Retained-unchanged must preserve the proposition" above), then
constructs the new revision via the existing `makeRevision` helper and
appends it to `store.revisions`, and appends a `refresh` lineage edge to
`store.lineage` linking the episode's subject revision to that new
successor — all appends, or none, under one operation receipt. The opened
state remains individually addressable
afterward, satisfying `semantic-model.md`'s "immutable lifecycle
transitions" directly. Every triggering nomination's disposition now
resolves through its own entry in this closing state's `trigger_resolutions`
(a read-time projection, per Operation 1) — never through the episode's own
overall disposition directly, and causal attribution resolves through
`causal_attribution` — never derived from `trigger_resolutions`.

**Claim-head admission, not just episode CAS (fourth-review correction —
the most serious finding this round).** Internally reusing `makeRevision`
fixed round 1's atomicity problem, but reuse of a construction mechanic is
not the same as inheriting the admission invariants the *public*
`publish_revision` operation enforces around it — Sol's own naming for this:
"reusing a mechanism does not inherit the contract that previously governed
its use." The episode's own CAS (`expected_state` matching the episode's
current state-chain head) establishes only that nobody else changed *this
episode*. It says nothing about whether `subject_revision` is still an
admissible predecessor for the claim — the claim can advance independently
while an episode is open (another authorized writer publishes a new head
between this episode's opening and its closing). `semantic-model.md`'s own
"Branching, conflict, and canonical support" section is explicit that
competing revisions or refresh judgments are permitted "only when the domain
profile permits branching," and that "no wall-clock ordering... silently
selects canonical support" — so this operation must not mint a successor
revision from a `subject_revision` that is no longer a current head without
consulting that policy. Before constructing any successor revision, this
operation checks whether `subject_revision` remains an admissible
predecessor for the claim, exactly as `publish_revision`'s own
`expected_state`-against-`revisionHeads` check already does for its own
callers:

```text
non-branching domain profile:
    current_heads(claim) == { subject_revision }
        -> proceed
    anything else (subject_revision missing from heads, or heads contains
    more than just subject_revision)
        -> reject: publishing this successor is no longer admissible

branching-permitted domain profile:
    out of scope for this vertical's successor-revision path — see below
```

**Fifth-review tightening:** the fourth round's rule accepted
`subject_revision ∈ current_heads(claim)` (mere membership). Sol's review
found that too loose for a non-branching profile: if the store already holds
multiple heads for the claim — for any reason, not necessarily one this
episode caused — "`subject_revision` is still *a* head" wrongly permits a
successor when the actual forced invariant for a non-branching profile is
that `subject_revision` is the *only* head. Tightened to set equality:
`current_heads(claim) == {subject_revision}`, failing closed on anything
else, per `semantic-model.md`'s own requirement that branches and conflicts
remain visible and governed rather than silently ordered away.

**Scope choice for this first vertical, stated explicitly rather than left
ambiguous:** rather than design branch representation now, this vertical
admits refresh-judgment successor-revision publication only for domain
profiles whose policy is non-branching; a domain profile that permits
branching is out of scope for `publish_refresh_judgment`'s successor-revision
path until branch representation is designed — consistent with this
document's own Route-B discipline (a genuine, explicitly-scoped subset of
approved semantics, not a claim that branching-permitted refresh doesn't
exist).

**Profile-owned refresh policy: the interface, not the policy itself
(fifth-review correction).** The fourth round said "the domain profile must
identify proposition-identity fields" without saying how the claim-evidence
service actually asks a profile that question — Sol: "a requirement, not yet
a contract." This document does not decide what any specific profile's
proposition-identity fields are (that would steal domain meaning this
proposal's own ownership boundary gives to domain workflows), but it does
define the extension point every profile admitted into this vertical must
populate, mirroring exactly how `validateProfilePayload` already dispatches
validation by `profile` name (`validation.mjs:47-61`):

```text
refresh_policy(profile) -> {
  branching_permitted: boolean,
  proposition_equivalent(predecessor_payload, successor_payload) -> boolean,
}
```

`publish_refresh_judgment` calls `refresh_policy(profile).branching_permitted`
to select the non-branching-only head-equality check above, and — for
`retained_unchanged` — calls
`refresh_policy(profile).proposition_equivalent(predecessor.profile_payload,
successor_payload.profile_payload)`, rejecting the operation if it returns
`false`. This vertical admits only domain profiles that supply a
`refresh_policy` with `branching_permitted: false`; a profile without one, or
with `branching_permitted: true`, is out of scope for this vertical's
successor-revision path (same scope choice as the claim-head rule above).
Which existing profiles (`proposal-research-v1`,
`revision-bound-review-finding-v1`, `production-path-v1`) get a
`refresh_policy` definition, and what each one's `proposition_equivalent`
concretely compares, is domain-profile-owned work this document does not
perform — it defines the socket, not what plugs into it for any given
profile.

**Retained-unchanged must preserve the proposition, not merely produce a
revision.** `semantic-model.md`: "An unchanged refresh preserves the
proposition while updating its evidence support." `proposal.md`'s own
invariants: "Unchanged refresh produces a new evidence-bound revision
without pretending the proposition changed." A generic
`successor_revision_payload` accepted at face value does not prove this;
`refresh_policy(profile).proposition_equivalent`, above, is what this
operation calls to enforce it before a `retained_unchanged` successor
publishes, not merely trusted.

**Superseded identity: `kind: judgment` names the terminal episode state
itself.** `superseded_by.kind == "judgment"` refers to exactly the terminal
state this same Operation 3 produces for some other episode — its own
`episode_state_id` *is* the durable identity of that refresh judgment, since
the terminal state is defined as the attributed judgment record (disposition,
trigger resolutions, causal attribution, and any successor revision, all
bound together in one immutable record). No separate `refresh_judgment_id`
is introduced: doing so would create a second identity for the same fact
without evidence one is needed, and this session's own standing discipline
is to avoid inventing identity machinery the approved semantics do not
require. `superseded_by.kind == "episode"` names an `episode_logical_id`;
`superseded_by.kind == "revision"` names a claim revision id, exactly as
`store.revisions` already identifies them. This resolves the concrete
locally-addressable identity for supersession without attempting to solve
the broader, still-open durable-semantic-judgment-identity seam named
elsewhere in the capstone (`work-engine-planned-architecture.md` §13 item
4) — this proposal already locally owns refresh-judgment authority and
publication, so a locally-scoped answer is sufficient here.

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

**Failure states, including the minimum episode↔trigger invariants the
approved semantics force (third-review correction — not the full matrix,
but not silent about the part that is forced either).** `episode_id`/
`episode_logical_id` not found or already closed → reject. `expected_state`
not matching the episode's current state-chain head → reject (plain CAS
conflict, identical in kind to `publish_revision`'s existing
conflicting-predecessor rejection). `trigger_resolutions` missing an entry
for any of the episode's own `reopened_by` nominations, containing an entry
for a nomination not in `reopened_by`, or containing a duplicate
nomination_id → reject (exhaustiveness is required, not advisory).
`episode_disposition == "changed"` with no `causal_attribution` entries, or
with `causal_attribution` present for any other disposition → reject (never
manufacture causality mechanically — at least one affirmatively adjudicated
source event must support a `changed` outcome, and none may be claimed
otherwise). **Fourth-review strengthening, corrected on fifth review:** any
`trigger_resolutions` entry resolving `resolved_changed` when
`episode_disposition` is anything other than `changed` → reject. This
replaces (and subsumes) the third round's narrower
`retained_unchanged`-only prohibition — Sol's fourth review found that rule
alone still admitted `inapplicable`/`contested`/`deferred` episodes with a
`resolved_changed` trigger, which is equally impossible: `semantic-model.md`
treats `inapplicable`/`insufficient`/`contested`/`deferred` as producing no
claim revision at all, so no trigger can truthfully resolve "the claim
changed" under any of them. **The forced rule is one-directional only:**
`resolved_changed ⇒ episode_disposition == "changed"`. The fourth round
additionally claimed the converse — `changed ⇒ at least one trigger resolves
resolved_changed` — as if the two together formed a biconditional. Sol's
fifth review found this unsupported and removed it: the approved semantics
let the authorized domain owner adjudicate causality against *any* source
event, including one discovered during investigation that was never among
the episode's own triggering nominations — none of the original triggers
need resolve `resolved_changed` for the episode to truthfully resolve
`changed`, as long as `causal_attribution` names the actual causal event(s).
Requiring a `resolved_changed` trigger would have partially re-collapsed
`trigger_resolutions` and `causal_attribution` into each other, the exact
distinction the third round existed to establish. The minimum forced rules
are therefore: `resolved_changed ⇒ changed`; `changed ⇒ causal_attribution`
non-empty; every triggering nomination individually resolved — not a
biconditional, and a nice-looking symmetry is not itself evidence.
`episode_disposition == "superseded"`
without a `superseded_by` reference, or `superseded_by` present for any other
disposition → reject; `superseded_by.id` must resolve to an actual episode,
judgment, or revision, per `semantic-model.md`'s "supersession identifies
the exact successor episode, judgment, or revision" — see "Superseded
identity" below for what `kind: judgment` concretely names.
`successor_revision_payload` present for an `episode_disposition` other than
`retained_unchanged`/`changed` → reject. `successor_revision_payload` absent
for `retained_unchanged`/`changed` → reject. **Fourth-review addition:**
when `episode_disposition == "retained_unchanged"`, the constructed
successor revision's proposition-identity fields (as the claim's own domain
profile defines proposition identity, distinct from its evidence-bound
fields) must match the predecessor's — see "Retained-unchanged must
preserve the proposition" below; a generic `successor_revision_payload` is
not by itself proof of semantic unchangedness. **Fourth-review addition:**
before minting any successor revision, `subject_revision` must still be an
admissible predecessor for the claim under its domain profile's branching
policy — see "Claim-head admission, not just episode CAS" below; the
episode's own CAS answers only whether the episode itself is unchanged, not
whether the claim underneath it is. Any failure in constructing the
successor revision or lineage edge aborts the entire operation — no partial
state is ever visible, matching `applyOperation`'s existing all-or-nothing
behavior for every other action.

Beyond these logically-forced rules, the full cross-consistency matrix
between `episode_disposition` and the complete *set* of `trigger_resolutions`
values remains open — `semantic-model.md` does not specify it, and inventing
the rest would be unsupported connective tissue. The rules above are not
optional hardening; they are the minimum the approved semantics already
require, and an implementation that admits them as durable state without
enforcing at least these would be admitting contradictory state, not merely
an incomplete one.

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
  set of `trigger_resolutions` values, beyond the logically-forced rules
  Operation 3's failure states already state (`resolved_changed` implies and
  is implied by a `changed` episode with non-empty causal attribution; every
  triggering nomination must be exhaustively represented).
- How a branching-permitted domain profile represents a competing successor
  revision (Operation 3's "Claim-head admission, not just episode CAS") —
  this vertical's `publish_refresh_judgment` successor-revision path admits
  only profiles whose `refresh_policy.branching_permitted` is `false`;
  branch representation is named as required future work, not designed
  here.
- What each admitted profile's `refresh_policy.proposition_equivalent`
  concretely compares field-by-field (Operation 3's "Profile-owned refresh
  policy" and "Retained-unchanged must preserve the proposition") — this
  document defines the extension point every profile must populate, not the
  domain-owned comparison logic behind it for any given profile.
- Which of the three existing profiles (`proposal-research-v1`,
  `revision-bound-review-finding-v1`, `production-path-v1`) receive a
  `refresh_policy` definition first, or whether all three need one before
  this vertical can accept nominations/episodes against every profile —
  domain-profile-owned formation work, not this contract's to schedule.

## Intermediate episode lifecycle states — not represented by this first production vertical

**Corrected twice.** First (second review): the first draft left
`semantic-model.md`'s named intermediate states (`active`, `awaiting_evidence`,
`awaiting_authority`, between `opened` and a terminal disposition,
`semantic-model.md:44-49`) as "descriptive, not additional operations, until
evidence says otherwise" — Sol pointed out this is not a resolved position,
since the source text calls them episode lifecycle states, not commentary.
Second (third review): the fix that followed — "(C) deferred... not needed"
— went too far the other way. `semantic-model.md` states "current writer
generation, and immutable lifecycle transitions" as part of the *already
approved* refresh-episode design, alongside the named intermediate states.
This document cannot decide they are unneeded; that would silently narrow
proposal meaning the user already approved, not merely lower it into an
implementation. Sol's framing, adopted directly: this is **Route B — a
bounded first production vertical**, not **Route A — the full operation-
contract surface**. Route A would need to design writer replacement and the
intermediate states' own operations before the surface could be called
closed; Route B implements a genuine subset and says so plainly.

**Stated correctly: not represented by this first production vertical; still
part of the approved broader semantic model.** This surface models exactly
two durable episode states — `opened` (Operation 2) and terminal
(Operation 3) — and does not implement `active`/`awaiting_evidence`/
`awaiting_authority` or writer-generation replacement in this slice. That is
a scope boundary of this implementation vertical, not a finding about what
the approved semantics require. A domain workflow may track its own
in-progress investigation status outside this shared substrate for now. If a
later vertical implements the full lifecycle (option A: authoritative
transitions with a named publisher and admitted writer-takeover transition,
or option B: deterministic projections derived from other owned facts), it
extends this surface into the semantics already approved — it does not
introduce new semantics this document declined to own.

**This scope choice is what makes Operation 3's plain-CAS concurrency design
correct for this slice — not evidence that writer ownership is unneeded in
general.** Within the two states this slice actually implements, there is no
window in which a writer might need to be replaced mid-investigation — the
only concurrency hazard is two closers racing to publish the *same* terminal
transition, which plain revision CAS already resolves. The moment a future
vertical implements any of the deferred states as durable, authoritative
transitions, the writer-ownership question reopens on its own terms — this
document's "not needed here" claim would not transfer to that vertical
without its own re-derivation.

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
- A nomination's own resolution is never the same fact as causal attribution
  — `resolved_changed` records that a triggering nomination's candidate
  impact was resolved and the claim changed; `causal_attribution` separately
  records which source events were affirmatively adjudicated causal. Neither
  is derived from the other.
- Supersession always identifies its exact successor — `episode_disposition
  == "superseded"` requires a `superseded_by` reference to the exact
  successor episode, judgment, or revision; "something else won" with no
  named successor is not a representable state.
- Closing an episode publishes a successor episode-state revision; it does
  not destructively rewrite the opened state — the opened state remains
  individually addressable after closing, matching `semantic-model.md`'s
  "immutable lifecycle transitions" requirement.
- Writer-generation replacement and the three named intermediate episode
  lifecycle states are real, approved semantics this document does not
  implement — not semantics this document has found unnecessary. A future
  vertical implementing them extends this surface into already-approved
  territory; it does not introduce new territory.

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
