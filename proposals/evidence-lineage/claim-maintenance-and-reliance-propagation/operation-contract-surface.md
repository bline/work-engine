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
review of the first draft):** semantic authority resides with the role whose
bounded position gives it the strongest relevant evidence and context for
that decision. Coordination authority does not imply domain authority, and
an upstream role must not appropriate a downstream role's judgment merely
because it can observe or control the workflow. This governs the fencing
design in Operation 3 below directly: a kernel-level, cross-domain
coordination service is the wrong owner for a domain-local judgment's
concurrency control, precisely because coordination authority over *when*
something may run is not the same as domain authority over *whether a
judgment is correct*.

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
supports. All five are corrected below; the sections below reflect the
corrected design, not the original.

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
| Publish refresh judgment + resulting revision, when required | authorized domain-owner class (structurally provable even when a different actor closes than opened it — see below) | **one** protected consequence: terminal judgment, resulting revision when the disposition requires one, and its `refresh` lineage edge, published atomically together | episode identity + fencing generation | episode record, terminal; revision; lineage edge — one operation receipt |

All three columns of authority/atomicity/idempotency differ between
nomination and episode-opening, and episode-opening's authority and fencing
requirements differ again from judgment-publication's fenced close. That is
three operations, not one, not four — the causal production of a successor
revision belongs *inside* the judgment operation's own atomicity boundary,
not to a separately-sequenced call to an existing operation (see Operation 3
below; this was the first draft's most serious error, corrected here). This
still matches Sol's own speculative sketch
(`nominate_impact`/`open_refresh_episode`/`publish_refresh_judgment`), but
arrived at by the test rather than presumed from it — the issue Sol's review
found was the atomicity boundary, not the operation count.

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
- `LINEAGE_RELATIONSHIPS` already contains `"refresh"` alongside `correction`/`supersession`/`composition`/`derivation`/`identity_fork`/`retraction` (`contract.mjs:9-12`). **Corrected 2026-09-15** (Sol's review): the first draft claimed this was "independent evidence the original substrate had already anticipated exactly this composition" — that overstates what one unused enum value supports. It is evidence that refresh lineage was anticipated by the substrate, and that this design may extend rather than contradict the existing relationship vocabulary. It says nothing about the three operations, their authority split, the fencing mechanism, or their atomicity boundaries — those are established below on their own evidence, not borrowed from this one fact.
- `review-episode` (`app-server/src/services/review-episode/{contract,service}.mjs`) is a closer precedent than `workspace-coordination` for this surface's own writer-generation fencing — see Operation 3's "Fencing and takeover" below for why domain-locality, not mechanism resemblance, decides which precedent applies.

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
over whichever refresh episode(s) later resolve it, not a field this
operation or any other writes back onto the nomination.

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

**Identity and fencing.** `episode_id` is `refresh-episode-v1@digest({episode_identity, subject_revision, reopened_by, domain_profile, evidence_cutoff})`, following the `production-path-establishment` precedent (caller-supplied identity string plus a content-derived record id, `production-path-contract.mjs:141-152`). Opening mints an initial writer authority at `generation: 1`. **Corrected 2026-09-15** (Sol's review): the first draft cited `workspace-coordination`'s fencing as precedent — that is a kernel-level, cross-domain coordination service, and per the governing authority principle above, coordination authority over *when* something may run does not extend to authority over a domain-local judgment's own concurrency control. The closer, correct precedent is `review-episode`'s own service-local writer-generation mechanism (`app-server/src/services/review-episode/{contract,service}.mjs`) — a domain-specific service fencing its own state, not a shared kernel resource. See Operation 3's "Fencing and takeover" for the exact mechanism this surface reuses from it.

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
terminal disposition and, when the disposition requires one, **causally
produces** the resulting successor claim revision and its `refresh` lineage
edge as part of the same protected consequence. Per `semantic-model.md`:
"Refresh judgment belongs to the authorized domain owner," and
"`retained_unchanged` and `changed` each produce exactly one new claim
revision against the evidence world actually examined... Inapplicable,
insufficient, contested, and deferred outcomes produce no claim revision."

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
  expected_state: <the episode's writer authority — see Fencing and takeover>,
  payload: {
    episode_id: <must be an open episode>,
    disposition: one of [retained_unchanged, changed, inapplicable,
                          insufficient, contested, deferred, superseded],
    rationale: <nonempty text>,
    evidence_references: [ <Reference>, ... ],
    changed_because_of: <source_event identity, required only when disposition == "changed", per semantic-model.md's own rule that causality is never manufactured mechanically>,
    successor_revision_payload: <exact same shape publish_revision's own payload.revision already requires, present only for retained_unchanged/changed, else null — this operation constructs and publishes the revision itself, using the existing makeRevision helper; the caller supplies the revision's content, not a pre-existing id>,
    decision_scope: <matches authority.decision_scope>,
  }
}
```

**Who may request/admit it.** The same `publish_refresh_judgment` domain-
owner permission class as `open_refresh_episode` — still not required to be
the identical specific actor who opened the episode (the "same actor ≠ same
authority ≠ same act" distinction this session's routing-seam correction
already established), but now with a complete ownership story instead of an
informal handoff — see below.

**Fencing and takeover.** Reuses `review-episode`'s own precedent exactly,
not `workspace-coordination`'s, per the governing authority principle above
(a domain-local judgment's concurrency control belongs to a domain-local
mechanism, not a kernel coordination service). `open_refresh_episode` mints
a writer authority at `generation: 1`. `publish_refresh_judgment`'s
`expected_state` must be a writer authority at `generation: 2` whose
`predecessorRevision` equals the exact digest of the episode's
just-opened state — structurally identical to `review-episode`'s own
`replace_writer` check (`authority.writer.generation !== current.writer.generation + 1 || authority.predecessorRevision !== current.revision`,
`review-episode/service.mjs:84-86`). This is the "complete ownership story"
Sol's review required: whoever closes the episode, same actor or different,
must present cryptographic proof they read the exact opened-episode state
before acting — not merely hold a copy of a token passed between actors.
Holding the correct generation-2 authority is what makes the close
admissible; it is a separate fact from the domain-owner permission that
makes the *judgment itself* authorized — the fence proves freshness and
exclusivity of the write, the permission proves authority to judge, and this
design keeps them as two checks, not one.

**Result and state, as one atomic consequence.** In a single
`applyOperation` call: the episode record becomes terminal
(`lifecycle_state: "closed"`, generation 2, disposition, rationale, evidence
references); when the disposition is `retained_unchanged`/`changed`, a new
revision is constructed via the existing `makeRevision` helper and appended
to `store.revisions`, and a `refresh` lineage edge is appended to
`store.lineage` linking the episode's subject revision to that new
successor — all three appends, or none, under one operation receipt. Every
triggering nomination's disposition (a read-time projection, per Operation
1) now resolves through this episode.

**Canonical support is a separate act, not implied by this operation.**
Per `semantic-model.md`'s own "Branching, conflict, and canonical support"
section: "The domain's authorized maintenance owner records an explicit
canonical-support selection... That selection is immutable, attributed,
authority-bound, and replaceable by a successor." Publishing a successor
revision here does **not** by itself make that revision the domain's
canonical support when the domain profile permits branching — a resulting
revision existing and a resulting revision *governing* are kept distinct.
Under a non-branching domain profile there is no competing candidate to
select among, so the two may coincide in practice without a separate record;
under a branching-permitted profile, an explicit canonical-support-selection
mechanism is required before the produced revision governs, and this
document does not design that mechanism — it is named here as a boundary
this surface must not silently cross, not solved.

**Failure states.** `episode_id` not found, already closed, or presented
generation/predecessor mismatch → reject (fence violation). `disposition ==
"changed"` without `changed_because_of` → reject (never manufacture
causality mechanically). `successor_revision_payload` present for a
disposition other than `retained_unchanged`/`changed` → reject.
`successor_revision_payload` absent for `retained_unchanged`/`changed` →
reject. Any failure in constructing the successor revision or lineage edge
aborts the entire operation — no partial state is ever visible, matching
`applyOperation`'s existing all-or-nothing behavior for every other action.

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
