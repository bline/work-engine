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

## Method: discovering the operation count, not presuming it

Sol's test for whether two transitions belong in one operation: do they
share the same authority, atomicity boundary, idempotency identity,
failure/recovery consequence, and independently meaningful durable state? If
not, they are probably not one operation. Applied to the four candidate
transitions above:

| Transition | Authority | Atomicity boundary | Idempotency identity | Durable state |
| --- | --- | --- | --- | --- |
| Nominate impact | evidence-producer class (observer/analyzer) | one nomination against one target revision | content-derived (see below) | nomination record |
| Open refresh episode | authorized domain-owner class | one episode against one subject revision | caller-supplied episode identity | episode record, opened |
| Publish refresh judgment | authorized domain-owner class (not necessarily the same actor who opened it) | closes exactly one open episode, once | episode identity + fencing generation | episode record, terminal |
| Successor claim revision | — | — | — | **not a new operation** — reuses the existing `publish_revision` operation |

All three columns of authority/atomicity/idempotency differ between
nomination and episode-opening, and episode-opening's authority and fencing
requirements differ again from judgment-publication's CAS-guarded close.
That is three operations, not one, not four — the fourth transition already
has an owner. This matches Sol's own speculative sketch
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
- `LINEAGE_RELATIONSHIPS` already contains `"refresh"` alongside `correction`/`supersession`/`composition`/`derivation`/`identity_fork`/`retraction` (`contract.mjs:9-12`) — the original substrate already anticipated linking a refresh's predecessor and successor revisions via the existing `publish_lineage` operation. This surface reuses that rather than inventing a new relationship field.

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

**Identity and idempotency.** `nomination_id` is a content digest over
`{target_claim_revision, source_event, evidence_references, producer}` —
deliberately *not* including `operation_id` or a timestamp. This is a
divergence from the existing six operations worth stating explicitly: where
`create_claim` rejects a second attempt at the same identity as an error,
nomination must not — `semantic-model.md`'s own text requires "duplicate and
late delivery preserve one semantic nomination consequence and retain
delivery history." So a `nominate_impact` call whose computed `nomination_id`
already exists is not a conflict; it is a content-idempotent no-op that
returns the existing nomination's identity (as `idempotent: true`),
regardless of whether the calling `operation_id` differs from the original
submission's. `operation_id`-based replay-guard (the existing mechanism)
still applies underneath this for exact-retry detection; content-based
collapse is an additional, stronger dedup this record type specifically
needs.

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
    domain_profile: <the owning domain's own profile identity — may differ from the claim's storage profile; this document does not decide whether these must be equal, see Open questions>,
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

**Identity and fencing.** `episode_id` is `refresh-episode-v1@digest({episode_identity, subject_revision, reopened_by, domain_profile, evidence_cutoff})`, following the `production-path-establishment` precedent (caller-supplied identity string plus a content-derived record id, `production-path-contract.mjs:141-152`). Opening also mints an initial **writer-generation fence**, reusing the fencing-token pattern this session already established as Work Engine's standard answer to "at most one writer may currently act" (`workspace-coordination`'s own fencing tokens, referenced throughout the routing-seam and organizational-envelope reconciliations) — not a newly invented mechanism. `publish_refresh_judgment` (below) must present this generation as its `expected_state`, so two investigations racing to close the same episode cannot both succeed.

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
terminal disposition, optionally referencing a successor claim revision
already published through the *existing* `publish_revision` operation. Per
`semantic-model.md`: "Refresh judgment belongs to the authorized domain
owner," and "`retained_unchanged` and `changed` each produce exactly one new
claim revision against the evidence world actually examined... Inapplicable,
insufficient, contested, and deferred outcomes produce no claim revision."

**Request:**

```text
{
  schema_version: 1,
  operation_id: <idempotency key>,
  action: "publish_refresh_judgment",
  profile: <same as the episode's subject claim>,
  expected_state: <the episode's current writer-generation fence>,
  payload: {
    episode_id: <must be an open episode>,
    disposition: one of [retained_unchanged, changed, inapplicable,
                          insufficient, contested, deferred, superseded],
    rationale: <nonempty text>,
    evidence_references: [ <Reference>, ... ],
    changed_because_of: <source_event identity, required only when disposition == "changed", per semantic-model.md's own rule that causality is never manufactured mechanically>,
    successor_revision: <exact revision id, required only for retained_unchanged/changed, else null — MUST already exist in store.revisions, published by a separate prior publish_revision call>,
    decision_scope: <matches authority.decision_scope>,
  }
}
```

**Who may request/admit it.** The same `open_refresh_episode`-class
authorized domain-owner permission (a distinct `publish_refresh_judgment`
permission, grantable to the same or a different specific actor than the one
who opened the episode — this document deliberately does not require
same-actor continuity between opening and judgment, matching the "same
actor ≠ same authority ≠ same act" distinction this session's routing-seam
correction already established elsewhere).

**Identity and CAS.** `expected_state` must equal the episode's current
writer-generation fence (minted at open, or advanced by any prior rejected
attempt — mirroring `publish_revision`'s own `expected_state`-against-heads
check). A mismatch is a conflicting-predecessor-shaped rejection, identical
in kind to `publish_revision`'s existing behavior. `operation_id` +
payload-digest idempotency applies exactly as it does for every existing
operation — a byte-identical retry is a no-op returning the prior result.

**Successor-revision sequencing (reuses existing operations, does not
reinvent them).** This operation does not create a claim revision itself.
The calling workflow:

1. Calls the existing `publish_revision` operation to publish the successor
   revision (ordinary revision publication, nothing new).
2. Calls the existing `publish_lineage` operation with
   `relationship: "refresh"` — already present in `LINEAGE_RELATIONSHIPS`,
   unused until now — linking the subject revision (source) to the new
   successor revision (target).
3. Calls `publish_refresh_judgment` referencing that successor revision's id
   in `payload.successor_revision`.

`publish_refresh_judgment` validates that the referenced `successor_revision`
exists, belongs to the same claim as the episode's subject, and that a
matching `refresh` lineage edge exists from subject to successor — it does
not publish either the revision or the lineage edge itself. This keeps the
"existing six operations remain the substrate this surface extends, not
replaces" boundary structural rather than aspirational: the new surface has
no code path that can create a revision or a lineage edge outside the
already-existing operations.

**Result and state.** The episode record becomes terminal
(`lifecycle_state: "closed"`), carrying its disposition, rationale, evidence
references, and (when applicable) the successor-revision reference. Every
triggering nomination's disposition (a read-time projection, per Operation
1) now resolves through this episode.

**Failure states.** `episode_id` not found or already closed under a
different generation → reject. `disposition == "changed"` without
`changed_because_of` → reject (never manufacture causality mechanically).
`successor_revision` present for a disposition other than
`retained_unchanged`/`changed` → reject. `successor_revision` absent for
`retained_unchanged`/`changed` → reject. Referenced `successor_revision` not
found, wrong claim, or missing the paired `refresh` lineage edge → reject.

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
- The exact intermediate episode states `semantic-model.md` names
  (`active`/`awaiting_evidence`/`awaiting_authority`, between `opened` and a
  terminal disposition, `semantic-model.md:44-49`). Nothing in the source
  text assigns these their own identity, authority, or transition operation
  distinct from open/close — treated here as descriptive/observable
  projection states, not additional operations, until evidence says
  otherwise.
- Any schema for `store.nominations` / `store.refresh_episodes` beyond the
  field lists implied above, or the corresponding `validateStore` extension
  — that is implementation detail belonging to the next stage, not this
  contract-formation stage.
- New `PERMISSIONS` values' exact string spelling (`nominate_impact` /
  `open_refresh_episode` / `publish_refresh_judgment` are working names,
  consistent with the existing `snake_case` action vocabulary, not
  necessarily final).

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
  surface extends, not replaces — structurally enforced above (successor
  revisions and their lineage edges are published only through the existing
  operations; this surface only ever references them by id, never creates
  them by another path).

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
