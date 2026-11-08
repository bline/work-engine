# Knowledge Oracle: an agent-facing question-resolution adapter

## Status

Exploratory architectural proposal, revised on 2026-11-08 from a first draft
the operator supplied in session (the first draft is not in the repository).
The revision follows a review of that draft against the Rust code at
`97f6486f` and the operator's direction to narrow the oracle to a
question-resolution adapter.

Revision 2, the same day, applies three refinements the operator requested on
accepting the direction for continued exploration:

- routing-hint revalidation (section 8.3);
- separate measurement of receiving knowledge versus recognizing and asking
  for it (section 12);
- a durable referral path for content challenges (section 8.5).

It also restates the end-to-end vision (section 2). The operator's acceptance
is of the direction for exploration only. It is not approval of the
architecture or of implementation.

This document does not accept a design, amend `DESIGN.md`, authorize
implementation, publish claims, change any claim, register, anchor or
lifecycle contract, or assign a permanent owner. Its first proposed step is a
feasibility experiment that itself needs separate approval.

```yaml
idea_status:
  architectural_supersession: none
  architectural_supersession_note: "Compared on 2026-11-08 against design/architecture/evidence-and-claims.md, proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/ (proposal.md, semantic-model.md, repository-integration.md role-facing retrieval, operation-contract-surface.md status), design/architecture/substrates/evidence-anchor.md, metrics/register/README.md, and the status sections of design/ideas/pending/active-evidence-acquisition-during-execution.md and evidence-basis-topology-for-bounded-inference.md. The first draft's invalidation, dependency and reopening content was found covered by claim-maintenance and evidence-anchor and is removed here. The remaining core (question routing, routing hints and their revalidation, answer projection, provisional composition, content referral records) is not stated in any of the compared documents."
  residue: present
  residue_note: "Ownership decisions OD-1 to OD-8 in the open-question ledger are RESIDUE-kind and OPEN."
  backlog: present
  backlog_note: "Implementation questions B-1 to B-6 in the open-question ledger are BACKLOG-kind and OPEN."
  audit_scope:
    - open-question-ledger
  audit_scope_completeness: complete
  status_as_of: 2026-11-08
idea_provenance:
  origin: synthesized_from_raw_material
  captured_on: 2026-11-08
```

## 1. Summary

An agent doing work often needs a fact that earlier work already established.
Today it either rediscovers the fact or finds a claim by guessing an exact
substring. The adapter proposed here takes a natural-language question plus
task context and returns:

- the exact claim revisions and evidence that bear on it, with each claim's
  standing passed through from the owner that reports it;
- research-register findings, labelled as register findings rather than claims,
  where no claim exists;
- optionally, a composed answer, always labelled provisional;
- or a truthful "no route", "ambiguous" or "unavailable" result, plus an
  optional investigation request the requester may take to its supervisor.

The adapter is a read-side consumer. It does not own claims, reliance,
applicability, change nomination, refresh, reopening, anchors or register
records, and it keeps no answer cache whose staleness it would have to manage.
The only durable state it proposes to own is:

- a store of non-authoritative routing hints, and the challenges raised against
  them;
- a store of referral records: content challenges routed to their owners, with
  acknowledgement and disposition.

Whether this is worth building is not established. The two research-register
questions it depends on, Q17 and Q18, have no linked findings in the register's
Stage 0 (8 of the 259 inventoried studies). Section 12 therefore puts a small
feasibility experiment, with a comparison against unassisted agents, ahead of
any new implementation.

## 2. End-to-end vision and the adapter's part

The vision from the first draft is kept. Knowledge one agent needs and obtains
should be reusable by later agents without rediscovery, and outdated
conclusions should not be reused as if current:

```text
agent asks a question
      |
      v
adapter resolves it  ............................. this proposal
      |                    |
   found:               insufficient:
   projections          investigation request record
   to the agent             |
                            v
              requester's supervisor decides, under its existing
              contract, authority and budget
                            |
                            v
              investigation through existing orchestration,
              model selection and review
                            |
                            v
              qualified result enters an existing system:
                claims, through the trusted producer, or
                the research register, through its owner
                            |
                            v
later agents ask; the adapter resolves to the new claim or finding
```

The adapter implements only the question-resolution part of this flow:

- routing and its hints;
- retrieval and projection;
- provisional composition;
- the investigation request record;
- content referral records.

Every other step belongs to an existing owner:

- deciding to investigate, running the investigation, and qualifying,
  publishing or registering its result;
- maintaining claims and their reliance over time (claim-maintenance, once
  implemented).

The flow is only complete if those owners do their parts. The adapter does not
take any of them over.

## 3. Owners this proposal defers to

These owners are treated as authoritative for their scope. Their actual status
is recorded because several are design, not code.

| Owner | Scope the adapter defers to | Status at 2026-11-08 |
|---|---|---|
| `claim-evidence` crate | claim identity, revisions, publication, exact reliance | Rust implementation of a subset (section 4) |
| [`claim-maintenance-and-reliance-propagation`](../../../proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/proposal.md) | change nomination (`may_affect`), refresh episodes and their dispositions, canonical-support selection, reliance with applicability and accepted limitations, selective-reopening obligations, delivery, recovery across context replacement | meaning approved 2026-09-15; placement revised 2026-10-10 (`decision.json`, `revise_placement`); operation surface for nomination and refresh passed review, with profile refresh policies the next prerequisite; `implementation_authorized: false`. Designed against the app-server JavaScript claims service. No part is implemented in the Rust crate, and Rust claims migration is "separately scoped work" (`docs/rust-development.md`). |
| [`evidence-anchor`](../../architecture/substrates/evidence-anchor.md) substrate | anchor observation, the five-state comparator, nomination candidates; extensible anchor kinds including `ServiceStateAnchor` and `ImplementationRevisionAnchor` | `design: proposed`; core `authorization: exploration_only`; adding anchor kinds against evidence of need authorized 2026-09-14; no implementation |
| [Research register](../../../metrics/register/README.md) | studies, findings, events, judgments, decisions, per-(finding, decision revision) applicability with owner authorization | Stage 0 in repository, Python (`metrics/scripts/register.py`), 20 questions, 12 findings; `materialized.jsonl` empty |
| `repo-search` evidence contract | backend-neutral evidence retrieval presented to roles ([repository-integration.md](../../../proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/repository-integration.md), role-facing retrieval) | skill-level contract |
| Decision owners and domain workflows | applicability, reliance decisions, reopening | per their own contracts |

The adapter adds nothing to any of these scopes. Where an owner's capability is
not implemented, the adapter reports the gap. It does not substitute its own
mechanism.

## 4. Current Rust baseline

"Implemented" below means present in Rust source at `97f6486f`. Absence entries
name the search domain they rest on.

| Capability | Rust today | Basis |
|---|---|---|
| Claim identity and revisions (`repository-fact-v1`) | Implemented. Claim and revision ids are content digests. Revisions are written append-only into one JSON state row replaced by compare-and-swap; immutability comes from content addressing and the write paths, not from a database constraint. | `claim-evidence/src/findings.rs:46-78`, `application.rs:795-816` |
| Successor revision | Implemented. A second successor of one predecessor is refused ("conflicting predecessor"). | `application.rs:758-764` |
| Withdrawal or retraction | Not implemented. `tombstone` is type-checked and never interpreted; `lineage` must be empty. | grep of `claim-evidence/src` and `work-engine-claims-host/src` for `withdraw\|retract\|contest\|supersed\|tombstone\|retire\|revok`; `findings.rs:174-177`, `store.rs:611-614` |
| Contested state, branching | Not implemented | same search |
| Exact reliance | Implemented: consumer, consumer revision, decision scope, exact claim revision, state `active`. No applicability, basis or limitation field; no retire; reliances on a revision are embedded in `resolve` and `discover` output, with no separate reverse query. | `store.rs:760-782`, `repository.rs:300-323` |
| Claim readers | `resolve` (exact id, revision, digest) and `discover` (literal substring over the canonical JSON of subject and proposition, current revisions only, 1–100 per page) | `repository.rs:44-59`, `115-170` |
| Topic, question, decision-class or semantic search | Not implemented | grep of both crates' `src`, `schemas`, `migrations` and README for `proposal\|research\|subject_key\|topic\|question\|fts`: 0 hits |
| Research claim profile | Not implemented. Profiles are `repository-fact-v1`, `revision-bound-review-finding-v1`, `production-path-v1`. | `authority.rs:10-13` |
| Live claims | Live store at revision 15: 6 claims, 10 revisions, 4 reliances, all `repository-fact-v1`, all about the claims and Codebase Memory implementation itself. No research findings. | read-only read of the live store, 2026-11-08 |
| Read tools for agents | `work-engine-server` claims read profile serves `discover_repository_facts`, `resolve_repository_fact`, and with evidence enabled `read_repository_evidence` and `check_repository_evidence_currency`. Tool callers cannot initialize roots, publish, record reliance, choose authority or change scope. | `work-engine-server/src/claims.rs:270-297`; `docs/repository-claims-operator-guide.md:86-92` |
| Captured evidence and currency | `repository-evidence-cbm` captures whole-file anchors (SHA-256) and Codebase Memory observations. Currency is byte comparison: `bytes_match`, `bytes_changed`, `unavailable`; applicability `not_assessed`. | `repository-evidence-cbm/src/lib.rs:142-153`, `525-543` |
| Reassessment | Manual `reassess` command. Produces a worklist (`blocked`, `review_nominated`, `uncertain`, `no_nomination_observed`) and writes nothing. Accepts only Codebase Memory capture evidence. | `work-engine-claims-host/src/reassessment.rs:1-10`, `637-665`, `933-957` |
| Change nomination, refresh episodes, obligations | Not implemented in Rust | same search as withdrawal |
| Environment identity anchors (model, harness, runtime, pricing) | Not implemented. Model identity is recorded in several crates' call records; harness identity appears as program digests; nothing binds either to a claim. | grep of `rust/crates/**/*.rs` for `model_version\|harness_version\|runtime_version\|harness_revision\|runtime_revision`: 0 hits |
| Agent consumer and handoff | `repository-claims-handoff`: a `CLAIMS_HANDOFF_V1` line of at most 4096 bytes carrying 1–4 exact refs; selection is a free-text, unauthenticated `selected_by`; consumer tools `claims_discover`, `claims_resolve`, `claims_evidence`, `claims_currency`. The Pi consumer is experimental scaffolding with no production embedder, and the lifecycle owner does not yet admit its profile. | `repository-claims-handoff/src/lib.rs:29-35`, `consumer.rs:24-28`; `docs/repository-claims-operator-guide.md:239-245` |
| Bounded, source-cited model output | `lifecycle-semantic`: a host-built `SourceBasis` of `SourceMaterial` items (owner, revision, digest, kind, trust, availability); model statements must cite sources that resolve in the basis; `revalidate_dependencies` reports `Changed`, `Missing` and `Unavailable` separately. Specialized to continuation plans; model roles are the closed set `Compile` and `Verify`. | `lifecycle-semantic/src/material.rs:1-108`, `399-471`; `continuation.rs:379-393`, `425-522`; `direct_control.rs:20-30`, `150-170` |
| Model calls | `bounded-inference`: one-shot, tool-free OpenAI Responses calls with byte, event and deadline bounds and a durable attempt journal. No reasoning-effort parameter, no provider-enforced JSON schema, no other provider. | `bounded-inference/src/request.rs:7-119`, `journal.rs:15-40`, `direct.rs:234` |

The first draft's examples need these facts. Its section 15 answer about Pi
acceptance comes from register finding K7, a single observation. That finding
is not a claim, so no live claim reader can return it today.

## 5. What the adapter is, and is not

The adapter does six things:

1. **Deterministic routing** where an identity is supplied, or where a routing
   hint matches on structured context and its routing basis revalidates
   (section 8).
2. **Bounded semantic interpretation** where it is not: one bounded model call
   that proposes a route among candidates the host supplied, or abstains.
3. **Retrieval** of exact claim revisions, evidence and currency through the
   existing Rust readers, plus register findings if OD-4 permits.
4. **Answer projection**: a compact, attributed rendering of what was retrieved
   (section 9).
5. **Provisional composition** on request: model-written text that cites only
   retrieved material and is always labelled provisional.
6. **Investigation request preparation** when retrieval is insufficient and the
   requester's contract permits asking for one (section 11).

It also records two kinds of challenge, without adjudicating either: challenges
to its own routing (section 8.4), and content challenges it refers to the
owner of the challenged claim or finding (section 8.5).

The adapter does not:

- create, revise, retire, contest or withdraw claims, revisions or reliance;
- record or imply applicability, which belongs to decision owners;
- derive or cache claim standing, nominate change, open refresh, or reopen
  consumers;
- declare or observe anchors, including environment anchors;
- write register records;
- launch, budget or deduplicate investigations;
- present a routing hint, a lexical match, a composed answer or an unadjudicated
  referral as established knowledge.

### Invariants

- **I-1.** The adapter writes no claim, revision, reliance, applicability,
  nomination, refresh, obligation, anchor or register record.
- **I-2.** Claim standing in a response is copied from the owner that reports
  it, with that owner named. The adapter never computes standing.
- **I-3.** Only exact claim revisions retrieved through a trusted reader are
  shown as claims. Register findings are shown as register findings. Composed
  text is shown as provisional.
- **I-4.** A routing hint is never shown or stored as an applicability
  judgment. Every response names the hint it used and the hint's producer.
- **I-5.** "No route" is reported relative to the sources consulted and their
  declared coverage. It is never reported as "no evidence exists".
- **I-6.** Mechanical validation is never reported as semantic correctness.
- **I-7.** A challenge can automatically reduce a hint's use, never increase it.
- **I-8.** A routing hint is used as a route only when its recorded routing
  basis revalidates as unchanged at the time of use.
- **I-9.** A challenge is confirmed to its sender only after it has been
  durably recorded. A recorded referral is never deleted, and is closed only by
  a disposition record that cites an owner's act.

## 6. Interface

### `oracle.ask`

Request:

- natural-language question;
- requesting task or objective identity and role kind;
- optional known identities: register question id, claim id, decision id;
- structured context: repository id, decision scope, and any environment
  identities the requester can state as structured values;
- whether a provisional composition is wanted;
- whether an investigation request may be prepared, and the requester's
  intended use of the answer.

Response:

- response id;
- **resolution state**, one of:
  - `claims_found`: exact claim revisions were routed to and retrieved;
  - `register_findings_only`: register findings were routed to, with no claim;
  - `ambiguous`: several candidate routes; candidates and the distinguishing
    question are returned, and none is chosen;
  - `no_route`: nothing found within the coverage reported;
  - `source_unavailable`: a reader failed, was truncated or was unreachable;
  - `refused`: outside the read scope or the requester's authority;
- the route: which identity, hint or interpretation was used, its producer, the
  hint's revalidation outcome, and the candidates rejected;
- the answer projection (section 9);
- optional provisional composition (section 9.3);
- optional investigation request (section 11);
- coverage: sources consulted, their revisions or digests, exclusions,
  truncation.

These are resolution states. They say whether and how the question reached
evidence. They are not claim standing, and they deliberately do not repeat the
first draft's "current / reassessment required / contested / superseded" table:
claim-maintenance owns those dispositions (section 10).

### `oracle.check_refs`

A deterministic check of exact refs a requester or contract already holds:
resolve, verify the digest, report whether the revision is still the current
revision and which successor exists, and report evidence currency. It needs no
question and no model. It is the realization proposed for the first draft's
"contract-required preflight": where a contract designates the claims a decision
relies on, checking them is fully determined (`DESIGN.md` section 9). Where no
reliance is declared, no preflight is proposed.

### `oracle.challenge_route`

Records a challenge to the adapter's routing (section 8.4).

### `oracle.refer_content` and `oracle.referrals`

`refer_content` records a content challenge and refers it to the owner of the
challenged claim or finding. `referrals` lists referrals by owner, state and
age. Both are covered in section 8.5. Acknowledgement and disposition are not
agent tools. They are recorded through an owner-side command (section 8.5).

## 7. Resolution path

```text
question + structured context
        |
        v
L0 deterministic routing
   supplied register question / claim / decision id
   routing hint: structured discriminators match AND routing basis revalidates
   substring discover, as candidate generation only
        |
   one route ----------------------------+
   several or none, or basis changed     |
        |                                |
        v                                |
L1 bounded interpretation (optional)     |
   host-built candidate basis            |
   one model call -> route or abstain    |
   mechanical validation                 |
        |                                |
        +---------------> retrieval via existing readers
                                |
                                v
                      answer projection
                      (+ provisional composition on request)
                      (+ investigation request when permitted)
```

**L0** uses identities the requester supplies, register question ids (the only
question identities that exist today), and active routing hints that pass
section 8.3 revalidation. Substring `discover` may generate candidates, but a
lexical match never counts as the same question; the first draft's own caution
applies directly, because the existing reader is a literal substring filter.

**L1** runs only when L0 yields no route, several routes, or a hint whose basis
changed, and only if the deployment enables it. The host builds a bounded
candidate basis from the readers: candidate register questions, claim subjects
and propositions with exact revisions, and register finding texts. One model
call proposes a route to one or more candidates, or abstains, with a short
statement of what distinguishes them. L1 has no authority to establish, revise
or apply anything. Its output is a route proposal, validated mechanically
(section 9.4) and shown as such.

## 8. Routing hints

### 8.1 What a hint is

A routing hint records that a question form, in a stated structured context,
was routed to a target. It speeds up the next routing decision. It does not say
the target answers the question, applies to the asker, or remains true.

Proposed fields (exact schema is B-1):

- hint id (content digest);
- question form: normalized text and, where known, register question id;
- structured discriminators: repository id, decision scope, declared
  environment identities, role kind;
- target: a register question id, register finding id, or claim id. The
  projection always resolves a claim's current revision fresh;
- **routing basis**, recorded when the hint is produced or re-validated:
  - for a claim target, the revision id and digest the routing was decided
    against;
  - for a register target, the digest of the finding record or question text;
  - the candidate-generation inputs (the deterministic queries, register
    question membership and scope used to build the candidate set) and the
    digest of the resulting candidate set: candidate ids with their revisions
    or record digests;
  - the router version: the deterministic rule version, or for L1 the policy
    revision and model identity;
- producer: deterministic rule, L1 interpretation (with its journal reference),
  requester confirmation, or curator;
- state: `active`, `suspended`, `retired`;
- predecessor hint, when this hint re-validates an earlier one;
- challenges raised against it.

### 8.2 Authority

Hints are adapter-local and non-authoritative. They are not applicability
records. Applicability stays with decision owners: in the register as owner-
authorized applicability records, and in claim-maintenance, once implemented,
as the applicability assessment a reliance records. A hint is never written
into either. A response always shows the target's standing from its owner
beside the hint, so a hint cannot hide standing.

### 8.3 Revalidation at use

A routing decision was made against particular material. When that material
changes, the decision has to be made again, not silently trusted. Every time
an active hint matches a request, the adapter deterministically revalidates its
routing basis before using it. This uses reader calls only, no model call:

1. Resolve the target. For a claim, read its current revision; for a register
   target, recompute the record or question-text digest.
2. Re-run the recorded candidate-generation inputs in the same scope, and
   recompute the candidate-set digest.
3. Compare the router version with the current one.

The outcomes follow `lifecycle-semantic`'s `revalidate_dependencies`
distinction between changed, missing and unavailable:

| Outcome | Condition | Adapter behaviour |
|---|---|---|
| `unchanged` | target revision or digest, candidate set and router version all match | hint used as an L0 route |
| `target_revised` | a later claim revision exists, or the register record or question text changed | hint not used |
| `candidates_changed` | the candidate set gained or lost members, or a candidate's revision changed | hint not used |
| `router_changed` | the deterministic rule or L1 policy version differs | hint not used |
| `target_missing` | the target no longer resolves | hint suspended with that reason |
| `unverifiable` | a reader failed or truncated | hint not used for this request; nothing recorded against it |

When a hint is not used, the request falls back deterministically:

- **L1 enabled.** The question is re-interpreted against a fresh candidate
  basis. If L1 chooses the same target, a new hint version is recorded with the
  new routing basis, producer "L1 revalidation", and a link to its predecessor.
  If L1 chooses differently or abstains, the old hint is suspended with reason
  "re-route disagreed", for the hint maintainer to review.
- **L1 disabled.** The response is `ambiguous`. The fresh candidates are
  returned, and the old hint's target is flagged "routing basis changed:
  <outcome>". Nothing is re-bound automatically.
- **Deterministic-rule hints** are re-derived by re-running the rule. That is
  fully determined, so a matching result is re-recorded without judgment.
- **Curator hints** need re-confirmation, by L1 or the curator, before reuse.

**This is not a claim-standing judgment.** The revalidation reads facts the
owners publish, such as "a successor revision exists" or "this record's digest
changed", only to decide whether the routing decision must be made again. It
draws no conclusion about whether the claim or finding is true, current or
applicable. The projection still shows the owner's standing unchanged. A
successor revision invalidates a route, not a claim.

### 8.4 How incorrect hints are detected and challenged

Revalidation catches routes whose basis changed. It does not catch a route
that was wrong from the start, or one made wrong by context the discriminators
do not capture. A hint can be wrong in three ways: the wrong target, a missing
target, or a target that is right in another context but not this one.
Detection therefore uses several independent routes, because no single route
is complete:

1. **Visible routing.** Every response names the hint, its producer, its
   revalidation outcome and the rejected candidates. The asker sees the route,
   not just an answer.
2. **Mechanical refusal at use.** A hint is skipped when its target does not
   resolve, is outside the requester's decision scope, any structured
   discriminator differs from the request, or revalidation does not return
   `unchanged`. This catches context mismatch only where context is structured.
   Prose-only context differences are not caught here, which is why routes 3–5
   exist.
3. **Asker challenge.** `oracle.challenge_route(response_id, kind, statement)`,
   where kind is `wrong_target`, `missing_target` or `context_mismatch`. The
   challenge is recorded against the hint, and the hint is suspended for the
   challenged context. Askers cannot create or retarget hints by challenging,
   so one agent's misunderstanding cannot become routing for others.
4. **Independent sampled audit.** A sample of hinted resolutions is re-routed by
   a party other than the hint's producer (a different model or a curator),
   blind to the hint. Disagreement suspends the hint and records the result.
   The sampling rate is a knob for the hint maintainer (OD-2); this proposal
   sets no number.
5. **Downstream evidence.** A review finding or task outcome that identifies an
   adapter response as misrouted is recorded as a challenge by the supervisor
   or reviewer who observed it.

Suspension is automatic and only reduces reuse (I-7). Re-enabling a suspended
hint, or retiring one, is the hint maintainer's act (OD-2), recorded with its
basis. The measured false-route rate combines audit and challenge results;
challenges alone undercount, because askers do not always notice a wrong route.

### 8.5 Content referrals before claim-maintenance exists

Sometimes a challenge is about the content rather than the route: an agent
says the claim or finding itself is wrong, outdated, or does not hold where it
was used. The adapter does not adjudicate that, and it must not discard it.
Until Rust claim-maintenance exists, there is no owner-side record to receive
such a challenge. The adapter therefore keeps a durable referral record that
routes the challenge to the owner and tracks acknowledgement, without becoming
a claims authority.

**Record.** Append-only and content-addressed, in the adapter's state directory
beside the hint store. It holds:

- referral id;
- the challenged target: a claim id plus the exact revision and digest the
  challenger saw, or a register finding id plus its record digest;
- the response id that prompted it, if any;
- the challenger's asserted identity and requesting task;
- the statement, and any evidence references the challenger supplies;
- the addressed owner, derived deterministically from the target. For a claim,
  that is the producer and authority reference recorded on the revision. For a
  register finding, it is the register owner as the register names it.

**States.** Each transition is a new record, never an edit:

```text
referred -> acknowledged -> dispositioned
                      \-> superseded_by_nomination   (once claim-maintenance exists)
```

- `referred` is written before the adapter confirms anything to the challenger.
  If the write fails, the challenger receives "not recorded", never a success.
  An unacknowledged referral stays `referred` indefinitely. There is no expiry
  or deletion state.
- `acknowledged` records that the owner, or a delegate acting for the owner,
  has received the referral. As in claim-maintenance's semantic model,
  acknowledgement is not disposition.
- `dispositioned` cites an artifact the owner produced in the owner's own
  system:
  - a successor claim revision published by the trusted producer;
  - a `reassess` worklist digest;
  - a register event id added by the register owner;
  - or an owner statement of no change, with its basis.

  The adapter checks mechanically that the cited artifact exists (the revision
  resolves, the event id is in the register, the digest matches). It does not
  judge whether the disposition is adequate.
- `superseded_by_nomination` records that the owner turned the referral into a
  claim-maintenance nomination, once that exists. The nomination is the
  owner's act.

**Surfacing.** Referrals are surfaced in three ways:

- the challenger receives the referral id immediately, and can cite it in its
  own receipt or handoff;
- `oracle.referrals` lists open referrals by owner and age;
- claim and register projections show the open referral count and ids,
  labelled "unadjudicated referrals; not owner judgments; standing unchanged".

Optionally, a notice can be posted to the coordination board as transport.
Posting is not acknowledgement. Whether the adapter posts these notices is part
of OD-3.

**Owner side.** Acknowledgement and disposition are recorded through an
owner-side command, not an agent tool. The command records the identity that
ran it. Who may run it for each owner, and how that identity is authenticated,
is OD-3.

**Why this is not a claims authority.** A referral asserts only "this
challenger said this about this exact target". It is never a claim, register
event, nomination, applicability record or standing, and the adapter never
changes the target. Turning a referral into a successor revision, a register
event or a nomination is the owner's act, in the owner's system. When
claim-maintenance lands in Rust, open referrals are available to its owner as
external observations, a nomination source its semantic model already lists.
The adapter does not create the nominations.

## 9. Answer projections

### 9.1 Claim projection (qualified)

This is a pure projection of exact claim revisions:

- claim id, revision id and digest, profile, producer and authority reference,
  proposition, limitations and reopening conditions as recorded;
- standing as its owner reports it today: whether this is the current revision
  and which successor exists, reliances on the revision, evidence currency per
  anchor, and any reassessment worklist result the requester supplies;
- open content referrals against the revision, labelled as in section 8.5;
- once claim-maintenance is implemented, its nomination and episode
  dispositions, copied with their identities, never derived.

"Qualified" here means only that a trusted producer published the revision
under its authority. The projection adds no statement of its own. Where the
owner cannot report something, the projection says so. For example, it says
"change-driven refresh not implemented; no nomination state available" rather
than implying the claim is current. Environment applicability is reported as
`not assessed` unless both the claim and the request carry the same structured
environment identity. Until evidence-anchor kinds exist, that never happens.

### 9.2 Register projection

Register findings are shown with their study, lifecycle, latest judgment per
axis, standing derived from events, applicability records with their
authorization status, and open referrals, as `register.py build` and the
referral store report them. They are labelled register findings, not claims.
Whether the Rust adapter may read the register's generated view is OD-4.

### 9.3 Provisional composition

On request, one bounded model call composes a short answer from the projected
material only. It is labelled `provisional`, names its producer and journal
reference, lists the exact sources each sentence cites, and is never stored as
a qualified answer or reused as established knowledge. The first draft's
section 15 answer is this kind of output. It synthesizes a proposition that no
single source asserts, and
[`evidence-and-claims.md`](../../architecture/evidence-and-claims.md)
invariant 7 forbids materializing such a conclusion as a claim. A composition
becomes established only if a producer publishes it as a claim through its own
authority, outside the adapter.

The adapter keeps no answer cache. Projections are recomputed from the readers
on each request, which is deterministic. Whether composed answers should be
cached as provisional renderings is a later decision, gated on measured
composition cost (B-5).

### 9.4 Mechanical validation versus semantic correctness

| Check | Kind | Establishes | Does not establish |
|---|---|---|---|
| Cited claim revision resolves and its digest verifies | mechanical | the cited record is the one named | that it answers the question |
| Every composed or L1 statement cites only sources in the host-built basis (the `lifecycle-semantic` `check_statement` pattern) | mechanical | no citation points outside retrieved material | that the statement follows from what it cites |
| Basis sources are current, with trust and availability as loaded (`SourceMaterial`, `revalidate_dependencies`) | mechanical | the cited bytes and revisions are the ones admitted, and whether they changed | that the claim is still true or applies |
| Hint routing-basis revalidation (section 8.3) | mechanical | whether the material the route was decided against changed | that the route is right, or that the target is true |
| Current revision and successor | mechanical | whether a later revision was published | whether the later revision applies to the asker |
| Hint discriminators match | mechanical | structured context fields agree | that two questions mean the same thing |
| Referral disposition artifact exists | mechanical | the cited owner artifact resolves | that the disposition is adequate |
| L1 route proposal | model judgment | a candidate route | correctness; requires audit (section 8.4) |
| Provisional composition | model judgment | a readable summary with exact citations | entailment; never qualified |
| An independent verify call (the `lifecycle-semantic` `Verify` role pattern) | second model judgment | reduces correlated error when independent | proof; output stays provisional |

`check_statement` (`continuation.rs:379-393`) checks only that text is
non-empty and that every cited source resolves in the basis. That is the
property the adapter can claim mechanically, and no more.

## 10. Change, staleness and context replacement

The adapter keeps no per-answer currency state and computes no claim standing,
so it adds no invalidation mechanism. Each concern of the first draft's
sections 7–9 has an owner:

| First-draft concern | Owner | What the adapter does today |
|---|---|---|
| Source change (7.1) | evidence-anchor observation → claim-maintenance nomination; today the `reassess` worklist | reports evidence currency and any supplied worklist result |
| Environment change (7.2) | evidence-anchor kinds, declared by the claim (or applicability record) | reports environment applicability `not assessed` |
| Evidentiary challenge (7.3) | claim-maintenance nomination from external observation or review; register `challenge` events | shows register events; records content referrals (section 8.5) |
| Revision or withdrawal (7.4) | claim-evidence (successor implemented; withdrawal not implemented) and claim-maintenance supersession | reports current revision and successor |
| Consumer applicability (7.5) | decision owners; register applicability records; claim-maintenance reliance | shows owner-authorized records; never infers applicability |
| Dependents and reopening (9) | claim-maintenance reliance, reverse reliance, obligations | shows reliances the reader embeds |
| Context replacement (O4) | claim-maintenance invariant: context replacement cannot erase or manufacture a durable semantic consequence; `CLAIMS_HANDOFF_V1` carriage, experimental | returns response and referral ids and exact refs a successor can re-check with `oracle.check_refs` |

Routing hints are the one place the adapter tracks validity. A hint's validity
concerns routing, not truth, and sections 8.3 and 8.4 handle it. A hint records
the revision it was routed against only as routing basis. A change to that
revision forces re-routing. It does not tell the adapter anything about the
claim's standing.

## 11. Governed investigation requests

When retrieval is insufficient and the request permits it, the adapter returns
an investigation request record. The record holds the question, the routing
outcome, what was found with its standing, known gaps, the requester's intended
use, the evidence standard requested, and a suggested scope. The adapter does
not launch, admit or budget the investigation, and does not store the request.

The requester decides whether to proceed with its supervisor, under the
requester's own contract and budget, using existing orchestration. Results
become claims only through a trusted producer, and register findings only
through the register owner. The adapter answers later questions from those
results like any other source. This is the section 2 flow.

The first draft proposed that the oracle deduplicate equivalent questions. That
needs an owner of inquiry records, and question equivalence would be routing-
hint quality, not established fact. Deduplication is therefore left to OD-6.
The candidate execution-time inquiry loop in
[`active-evidence-acquisition-during-execution.md`](active-evidence-acquisition-during-execution.md)
is a possible owner.

## 12. Feasibility experiment before implementation

**Purpose.** Measure whether exact claims and answer projections help agents
answer real information needs at lower total cost, without more false confident
reuse, compared with agents that have no assistance. This tests register
questions Q18 ("Can models consume exact claims across context replacement
correctly, and does reliance improve outcomes?") and Q17 ("When does a change in
evidence or configuration change a conclusion's applicability, and how is
reconsideration nominated?"). The experiment uses existing readers and
experiment instrumentation only. It adds no adapter code and publishes no live
claims.

Sizing, cost forecast, roles and stop conditions belong to the S0 approval
packet, not to this document.

### 12.1 Corpus

- The live claims store, read-only (6 code-fact claims).
- The register's generated Stage 0 view (12 findings, 6 events, 13
  applicability records).
- Optionally, a disposable claims root seeded with selected register findings,
  as the 2026-11-07 register pass 2 did for K2, K5 and K8. Seeding a disposable
  root is a publication into that root and needs its own approval.

### 12.2 Two kinds of item

- **Explicit items** state the question. They measure what an agent does with
  an information need it has been given. Recognition is not tested. Categories:
  1. answerable from an exact claim;
  2. paraphrase of 1;
  3. ambiguous or underspecified;
  4. answered only for another environment;
  5. bears on a corrected or challenged finding;
  6. stale by construction: a controlled change applied in a scratch checkout
     to an anchored source (Q17);
  7. not established: no evidence exists, and the correct answer says so;
  8. lexical decoy: a substring hit with a different meaning.
- **Embedded items** are short tasks, such as preparing a decision note or
  choosing between two options, whose correct completion depends on a corpus
  fact the task does not mention. They are the only items that test whether an
  agent recognizes an information need.

Each item has an answer key prepared and reviewed before launch, independently
of whoever prepares arm C's material.

### 12.3 Arms

The model, effort, task prompt and tool budget are the same in every arm. Only
the assistance differs.

- **A, unassisted.** Ordinary repository access (files, search, Codebase Memory
  as normally configured). No claims tools, no register view.
- **B, existing readers on request.** A's access plus the `work-engine-server`
  claims read tools and read access to the register's generated view. Nothing
  is pushed; the agent must decide to look.
- **C, curated knowledge delivered.** B's access plus, for each item, an answer
  projection in the section 9 format, prepared before the run by a curator
  using only the readers and attached to the task unasked. C measures the value
  of receiving curated knowledge **under curator routing**. Because curator
  routing is near-perfect, this is an upper bound.

### 12.4 Receiving knowledge versus recognizing and asking for it

The two benefits are reported separately:

- **Receiving curated knowledge**: C against A, and C against B, on both item
  types. This is the gain when the knowledge arrives without being asked for,
  as with a contract-designated preflight.
- **Recognizing a need and asking**: arm B on embedded items, from the
  transcripts. Each run is scored on four steps:
  - (r1) the need was recognized before the agent acted on the fact: it looked
    for it at all, in any source;
  - (r2) the agent asked a knowledge source (a claims tool or the register
    view) rather than only searching the repository;
  - (r3) the retrieval returned the keyed target;
  - (r4) the agent used it correctly.

  Arm A's transcripts give the unassisted baseline for r1. The gap between B
  and C on embedded items is attributed to losses at r1 through r4.

These measures come from the planned runs' transcripts, with no extra runs.
The embedded items are the only added cost, and they are the only way to
measure recognition at all. The packet offsets them by reducing the number of
explicit items.

**Not in S0: asking an adapter-shaped tool.** A "pull" variant of C, where
projections are returned only when the agent asks, would need a fixture tool
and an extra arm. It would only explain the difference between B's raw reads
and C's projections. It is deferred until S0 shows a B-versus-C gap large
enough to be worth explaining.

### 12.5 Routing benchmark (offline, no agents)

The same items are run through three routers: substring `discover` alone; a
bounded model call choosing among host-supplied candidates (a script, not
product code); and an abstain-always baseline. Measured per router: correct
route, false route, missed route, correct abstention, and cost per item. The
benchmark estimates the routing error that arm C deliberately leaves out. The
combined effect of routing error and projection value is not measured by this
design, and the results must say so.

### 12.6 Context-replacement extension (optional, priced separately)

On a subset of items:

1. Phase 1: the agent establishes an answer.
2. The context is replaced. The successor receives a prose summary (A), exact
   refs (B), or exact refs plus response ids (C).
3. Between the phases, a controlled change is applied to half the subset. The
   successor is not told which half.
4. The successor answers a follow-up question.

This tests the context-replacement part of Q18. It runs only if the core
experiment meets its proceed rule, because it roughly doubles the cost per item
it covers.

### 12.7 Acceptance measures

- **False confident reuse.** The arm states or acts on an answer as current,
  supported or applicable when the key says it is stale, corrected,
  challenged, contested, for another environment, or not established. This is
  the primary safety measure.
- **Total inference cost per correctly resolved information need.** It counts
  every model call in the arm. For arm C it includes the cost of preparing the
  projections; for the benchmark, the routing calls. Curator and key-preparation
  cost are reported separately. Cost comes from the usage ledger
  (`metrics/scripts/usage_ledger.py`). A cost that cannot be measured is
  reported as unavailable, never as zero.
- Correct resolution rate, and correct "not established" or abstention.
- The recognition steps r1–r4 (section 12.4).
- Missed reuse: the arm rediscovered something the corpus already held.
- Provenance accuracy: cited revisions exist and support the statement, judged
  by a scorer.
- Latency, and context tokens added by the assistance.

Scoring is blind to arm where transcripts allow. Assignment, exposure and
phase control can use `experiment-control`, which carries arms, phases and
exposure but no outcome scoring.

### 12.8 Decision rule

The decision rule's form is fixed here. The thresholds are frozen by the owner
before launch:

- **Proceed to stage S1** only if arm B or C beats arm A on cost per correct
  resolution, and its false confident reuse does not exceed arm A's by more
  than the frozen margin.
- **If C beats A but B does not, because r1 or r2 is low**, the value lies in
  delivery rather than agent-initiated asking. S1 then centres on
  `oracle.check_refs` and contract-designated delivery.
- **If B's r1 and r2 are high but r3 is low**, routing is the bottleneck, which
  supports stage S2 if the routing benchmark also passes.
- **If B is about as good as C**, the projection format adds little over raw
  reads, and S1 drops composition.
- **Proceed to stage S2** only if the routing benchmark's false-route rate for
  the model router is under the frozen ceiling and it beats substring
  `discover` on correct routes.
- **Stop** if neither B nor C beats A. The negative result is recorded through
  the register owner as a study under Q17 and Q18.

**Limits.** Small n makes this a feasibility result, not a significance test.
It covers one model and one host. Arm C's curator routing overstates the value
of real routing.

## 13. Integration and proposed ownership

Proposed, not decided:

- **Crate.** A new library crate in the root Rust workspace, provisionally
  `question-resolution` (the name is not decided). It would own only:
  - the versioned `oracle.*` request and response contract;
  - the routing-hint store and its challenge records;
  - the referral store and its state records;
  - the projection format;
  - the investigation request record format.
- **Serving.** Read-only tools added to `work-engine-server`'s existing claims
  read profile, so they inherit its startup binding and its rule that tool
  callers cannot publish, record reliance or choose authority. The adapter's
  own writes (hints, challenges, referrals) go only to its state directory.
  Pi coding agents would reach the tools through the `repository-claims-handoff`
  consumer once that profile is admitted. Claude Code and Codex sessions would
  reach them through the claims stdio entry. The owner-side referral command is
  a separate operator command, not a tool.
- **Reads.**
  - `claim-evidence` `RepositoryReader` (`resolve`, `discover`);
  - the existing evidence and currency tools;
  - the register's generated view, if OD-4 permits.
- **Model calls.** Through `bounded-inference` (OpenAI Responses only), behind a
  port. Either `lifecycle-semantic`'s `DirectInferencePort` gains a role, which
  changes that crate's closed `Compile`/`Verify` set and is its owner's
  decision, or a sibling port is built over `bounded-inference`'s request and
  journal layer. This choice is OD-7. A reasoning-effort setting does not exist
  today (B-4).
- **Source-basis and revalidation types.** Reuse `lifecycle-semantic`'s
  `SourceMaterial`, `SourceReference`, citation check and
  `revalidate_dependencies`. Depending directly on `lifecycle-semantic` would
  couple a read-side adapter to lifecycle internals, which
  `docs/rust-development.md` advises against. The alternative it describes is
  extracting a shared crate once a second consumer justifies one. The
  extraction would be decided by `lifecycle-semantic`'s owner (OD-7, B-2).
- **Persistence.** The hint and referral stores live in the adapter's own state
  directory, not the claims root. The claims root is `claim-evidence`'s
  single-repository store, and keeping these records there would make them look
  like claims.

What the adapter would own is limited to section 5. It holds no maintenance
state, so no second claim-maintenance authority arises: if claim-maintenance is
implemented in Rust, the adapter's only change is to copy its dispositions into
claim projections and let owners convert referrals into nominations (stage S4).

## 14. Stages after the experiment

Each stage needs its own authorization, and each is conditional on the
experiment's decision rule.

- **S0. Feasibility experiment** (section 12). No product code.
- **S1. Deterministic adapter.**
  - routing on supplied identities and register question ids;
  - substring candidate listing;
  - claim and register projections;
  - `oracle.check_refs`;
  - the referral store with its owner-side command.

  No model calls; hints only from deterministic rules or a curator, with
  section 8.3 revalidation. Acceptance uses the section 12 measures re-run
  against the implementation.
- **S2. Bounded interpretation.** L1 routing, L1 revalidation fallback, the
  section 8.4 challenge routes, and provisional composition. Acceptance is the
  benchmark re-run on the implementation, against the frozen false-route
  ceiling.
- **S3. Investigation requests.** Stateless request records (section 11).
- **S4. Claim-maintenance pass-through.** Only if and when claim-maintenance
  lands in Rust. The adapter copies its dispositions, and owners may convert
  open referrals into nominations. The adapter replaces none of it.
- **S5. Lifecycle carriage.** Response and referral ids and exact refs in
  handoffs. Depends on the lifecycle owner admitting the Pi claims profile.

## 15. Risks

- **False semantic equivalence through hints.** Mitigated by structured
  discriminators, routing-basis revalidation, visible routes, sampled audit and
  challenge-driven suspension. The residual risk is prose-only context
  differences.
- **Lexical decoys from substring discovery.** It is candidate generation only.
  The experiment includes decoy items.
- **Provisional text treated as established.** Mitigated by labelling, source
  lists, and composition only on request. The consumer's contract still decides
  what it may act on.
- **Hints drifting into de facto applicability.** Mitigated by I-4 and owner
  standing shown beside every hint.
- **Referrals read as contestation.** Many open referrals could make a claim
  look disputed. Mitigated by labelling them unadjudicated and showing counts
  and ids, never standing. Owners can disposition referrals in bulk.
- **Unacknowledged referrals accumulating.** Visible through `oracle.referrals`
  ages. No automatic closure. Who watches the queue is part of OD-3.
- **Adapter drifting into maintenance.** Mitigated by no answer cache, no
  computed standing, and I-1 and I-2.
- **Register and claims diverging.** Both are shown, labelled. The adapter does
  not reconcile them.
- **Cost exceeding savings.** Measured before building (section 12). A negative
  result stops the work.

## 16. Success measures

These are kept from the first draft. The primary measure is total inference and
execution cost per correctly resolved, applicable information need, reported
together with false confident reuse. Supporting measures:

- share resolved at L0, at L1, and needing investigation;
- false-route rate from audits and challenges;
- hint revalidation outcomes and the L1 calls they trigger;
- referral acknowledgement and disposition latency;
- missed reuse;
- provenance errors;
- latency;
- context tokens added;
- downstream rework attributed to adapter responses.

A class of tasks where the adapter costs more than it saves should not be
required to use it.

## 17. Open-question ledger

### Ownership decisions

- **OD-1. Adapter owner and host.** [KIND: RESIDUE] [OPEN] Should it be a new
  crate served through `work-engine-server`'s claims read profile (proposed), a
  module inside `work-engine-server`, or part of the Pi host only? And who owns
  the `oracle.*` contract?
- **OD-2. Hint maintainer.** [KIND: RESIDUE] [OPEN] Who may re-enable or retire
  suspended hints, confirm curator hints, and set the audit sampling rate?
  Where do the hint and referral stores persist, and under which durability
  contract?
- **OD-3. Referral acknowledgement.** [KIND: RESIDUE] [OPEN] Several questions
  about acknowledging and disposing of referrals remain open:
  - Who may acknowledge and disposition referrals for the claims producer or
    curator, and for the register owner?
  - How is that identity authenticated? The claims host's trusted startup is
    one candidate, but using it would put adapter records under the claims
    host.
  - Who watches the open-referral queue?
  - Should referral notices also be posted to the coordination board?

  The referral path itself (section 8.5) is proposed. Only these points are
  open.
- **OD-4. Register as a second source.** [KIND: RESIDUE] [OPEN] May the Rust
  adapter read the Python register's generated `current-knowledge.json`, pinned
  by digest? Who owns that read contract? The alternative is to wait for
  register findings to be materialized as claims, which is deferred.
- **OD-5. Decision-class identities.** [KIND: RESIDUE] [OPEN] The first draft
  routed on "known decision classes". Candidates exist in three places: register
  `decisions.json`, claims `decision_scope`, and `decision-readiness`
  assessments. None is designated as the routing identity.
- **OD-6. Investigation request deduplication and records.** [KIND: RESIDUE]
  [OPEN] Who owns inquiry records and decides that two requests are the same
  investigation? The candidate is the inquiry loop in
  `active-evidence-acquisition-during-execution.md`, which is exploratory.
- **OD-7. Inference seam and shared validation types.** [KIND: RESIDUE] [OPEN]
  Should `lifecycle-semantic`'s closed role set gain a routing role, or should a
  sibling port be built over `bounded-inference`? Should source-basis and
  revalidation types be extracted into a shared crate? Each changes an existing
  owner's contract.
- **OD-8. Environment anchors.** [KIND: RESIDUE] [OPEN] Environment
  applicability needs evidence-anchor kinds whose core is `exploration_only`,
  and claim-maintenance has no Rust implementation plan. Until an owner moves
  either forward, the adapter reports environment applicability as not
  assessed.

### Implementation questions

- **B-1. Hint record schema.** [KIND: BACKLOG] [OPEN] Exact fields, normalization
  of question forms, routing-basis digest construction, and candidate-generation
  input encoding.
- **B-2. Type reuse.** [KIND: BACKLOG] [OPEN] Mapping claim revisions and
  register findings onto `SourceMaterial` kinds and trust levels without
  widening `lifecycle-semantic`'s meaning.
- **B-3. Projection bounds.** [KIND: BACKLOG] [OPEN] Byte and page limits
  consistent with the claims read profile's digest-bound pages.
- **B-4. Effort and output shape.** [KIND: BACKLOG] [OPEN] `bounded-inference`
  has no reasoning-effort parameter and no provider-enforced schema. Decide
  whether L1 needs either.
- **B-5. Composition caching.** [KIND: BACKLOG] [OPEN] Whether to cache
  provisional compositions, decided only on measured cost.
- **B-6. Referral record and owner command.** [KIND: BACKLOG] [OPEN] Record
  schema, state-record chaining, the mechanical checks for each disposition
  artifact type, and the command's interface.

## 18. Changes from the first draft

Section numbers in parentheses refer to the first draft.

**Revision 1:**

- **Narrowed.** The oracle is a question-resolution adapter. The first draft's
  answer cache (10.2), answer invalidation states (8), dependency and reliance
  model (9), invalidation monitor, and environmental dependency mechanism (7.2)
  are removed. Those concerns are assigned to claim-maintenance, evidence-anchor,
  decision owners and the register (section 10).
- **Option (a).** Only exact claim revisions are presented as qualified.
  Composed answers are always provisional. Question mappings are
  non-authoritative routing hints, with several detection and challenge routes.
- **Rust baseline corrected** (section 4). The first draft implied, or did not
  rule out, live research claims, withdrawal, contested states, environment
  anchors, question and decision-class indexes, and applicability on reliance.
  None exists in Rust.
- **Investigation (L2).** The adapter prepares a request record. It no longer
  launches, budgets or deduplicates investigations.
- **Preflight (12).** It becomes the deterministic `oracle.check_refs` over
  declared refs, not a natural-language query.
- **Reuse named.** `lifecycle-semantic` and `bounded-inference` are named as
  reuse targets, with the line drawn between mechanical source validation and
  semantic correctness.
- **Experiment first.** A Q17/Q18 feasibility experiment with an unassisted arm
  comes before any implementation.
- **Model-neutral (13).** The first draft named a specific model for L1. Model
  choice now follows `docs/model-selection.md` at the time a stage is
  authorized.
- **Ownership.** A proposed home and the unresolved ownership decisions are
  stated.

**Revision 2:**

- **End-to-end vision restored** (section 2). The adapter is the
  question-resolution part of a flow whose other steps belong to existing
  owners.
- **Hint revalidation** (section 8.3). A hint records its routing basis: target
  revision or digest, candidate set and router version. It is used only when
  that basis revalidates unchanged (I-8). Otherwise the request falls back
  deterministically to L1 re-interpretation or `ambiguous`. The check makes no
  standing judgment.
- **Receiving versus asking** (section 12.4). The experiment separates the
  benefit of receiving curated knowledge (C against A and B) from recognizing
  and asking for it (arm B on embedded items, steps r1–r4). The added embedded
  items are offset by fewer explicit items. A pull variant is deferred, and the
  context-replacement extension is priced separately.
- **Content referrals** (section 8.5). Durable, append-only referral records
  route content challenges to the owner the target names. Referrals move
  through `referred`, `acknowledged` and `dispositioned` (citing an owner
  artifact), with no expiry. They are never discarded and carry no claims
  authority (I-9).

## 19. Sources

- `DESIGN.md` sections 1.5, 9, 12, 13 and 14.
- `design/architecture/evidence-and-claims.md` (invariant 7 at line 330;
  maintenance status at line 387).
- `proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/`:
  `decision.json`, `proposal.md`, `semantic-model.md` (nomination sources;
  acknowledgement is not semantic completion), `repository-integration.md`,
  and the `operation-contract-surface.md` status.
- `design/architecture/substrates/evidence-anchor.md` section 2 and "Source
  and Status" (lines 241-264).
- `metrics/register/README.md`; register view built 2026-11-08 by
  `register.py build` (validate: 8 studies, 12 findings, 6 events, 38
  judgments, 11 decisions, 13 applicability records).
- The 259-study inventory: `~/cbm-work/evidence-map-20261106/evidence-map.json`
  (off-repo; 259 entries under `studies`, counted 2026-11-08), also cited at
  `docs/model-selection.md:126`.
- `docs/repository-claims-operator-guide.md`; `docs/rust-development.md`.
- The Rust citations in section 4 come from three source verification reads at
  `97f6486f` on 2026-11-08. The `lifecycle-semantic` and `bounded-inference`
  passages cited in sections 4, 9.4 and 13 were re-read directly for this
  revision. The live store counts come from a direct read-only read of its
  state row.
