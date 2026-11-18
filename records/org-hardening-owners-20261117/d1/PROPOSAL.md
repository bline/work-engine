# D1: owners for the O7 mapping rules and the organization anchor (read-only design packet)

Author: Claude Opus 5.5 (`claude-opus-5-5`), read-only design worker, 2026-11-17. Parent/collector: Pi root
`01a1e56e-be12-7424-977b-c7c7983887d0`. Source: `/home/bline/code/work-engine` working tree at HEAD `0fe9dc74`
(branch `rescue-detached-20260911`). No repository edits, builds, network, or live providers. This is a proposal.
It grants no implementation authority.

Notation: **F** = fact, cited file:line, read in this session (n = 1 reading per site). **P** = proposal.
**NE** = `not established`. Input classes: **host** = host-derived, **digest** = digest-bound preimage,
**label** = labelled assertion.

## 0. Summary

| Item | Honest status | Recommended owner | Size of next slice |
|---|---|---|---|
| 1 `scope-digest-to-subject` | **Partly ill-posed.** `scope.digest` has no defined preimage | Organizational-requirements schema owner defines the preimage, using the episode owner's subject codec. S-A gate checks equality | S |
| 2 `problem-scope-to-episode-identity` | **Ill-posed as a mapping.** Must be a membership predicate. `planLineage` has no owner outside the org crates | `slice-campaign` (mints the identity and the episode id) | M (plus a lineage decision) |
| 3 `consumption-evidence-mapping` | **Owner exists. Rule exists in private code** | `slice-campaign` (consumer derivation) + `claim-evidence` (boundary vocabulary) | S (after 2) |
| 4 organization anchor | **No record exists. Owner exists by O1.** "Organization id" has two candidate meanings | Organizational authority (S-A crate): new record kind, the admitted organization (envelope) | M (fixture), L (production) |

Step 3 (hosted read-only review that produces and validates a result without consuming it) needs **none** of
the four (§6). Production consumption needs all four, plus O6 storage, the K-c issuer and production
authorization.

## 1. Common facts

- F: preparation emits the three mapping blockers unconditionally. They have these reasons:
  `scope-digest-to-subject` "no owner states the rule that relates scope.digest to the review subject"
  (`work-engine-review-preparation/src/prepare.rs:529-535`); `problem-scope-to-episode-identity` "no owner states
  the correspondence between problem-scope fields and episode identity fields" (`:536-542`);
  `consumption-evidence-mapping:<cond>` "matching strings are not a mapping" (`:556-567`).
- F: the host discharge function maps all three to `Discharge::AssumedDischarge`
  (`work-engine-review-delivery/src/gate.rs:103-105`). The label is `fixture_assumed_discharge_not_admission`
  (`:61`). The fixture stand-in is `FixtureDeclaration` (`prepare/src/fixture.rs:87-98`), labelled
  `synthetic_test_assumption_not_evidence` (`:41`).
- F: S-B is accepted at fixture standing, with `carried_open` listing "organization id caller-asserted,
  unanchored" and "assumed mapping kinds (O7)" (`records/org-r2-core-20261117/sb/SB-ACCEPTANCE.json:54-62`).
- F: the campaign path already has a single owner of review identity. `slice-campaign::CampaignIdentity
  {run_id, slice_number, attempt_id, plan_version}` (`slice-campaign/src/contract.rs:15-22`); the episode id is
  `episode_digest({identity, obligationId})[..32]` (`contract.rs:368-373`); the candidate subject is
  `{commit, tree, patchIdentity}` (`contract.rs:92-98`, `:246-249`).

## 2. Item 1: `scope-digest-to-subject`

**Question.** Which exact review subject does an obligation's `scope.digest` identify, and how is that
checked?

**Facts.**
- F: `Obligation.scope_digest: Digest` is parsed from `obligations[].scope.digest`
  (`work-engine-organization/src/schema.rs:211-215`, `:915-917`). The obligation subject is the literal
  `campaign_candidate` (`:220`).
- F: no frozen input defines the preimage. "scope.digest is NOT established by any frozen file:
  selection.subject.patchIdentity is used as a stand-in" (`work-engine-organization/tests/fixtures/frozen/provenance.json:23`).
- F: preparation carries `scope_digest` as identity only (`prepare.rs:82-84`). S-A binds the subject but "no
  check in this crate relates it to the problem scope (that mapping rule has no owner, O7)"
  (`work-engine-organization-authority/src/gate.rs:216-223`).
- F: the subject *shape* is owned by the episode owner. Preparation validates it with
  `review_episode_core::implementation_review_v1::validate_subject` (`prepare.rs:11`, `:386-387`).
  `slice-campaign` compares subjects by a campaign digest (`contract.rs:246-248`).
- NE: the text of "contract 3.1" (cited at `schema.rs:1`, `:217`). A grep of `design/`, `docs/`, `planning/` and
  `records/` `*.md` for `OrganizationalRequirements v0` / `scope.digest` returned 0 files. The contract
  probably lives outside the repository. I did not read it.

**Candidate owners.** (a) The requirements schema owner (`work-engine-organization`, contract 3.1) owns the
field but defines no preimage. (b) `review-episode-core` owns the subject shape. (c) `slice-campaign` owns
the candidate subject value.

**Ill-posedness.** "scope.digest identifies the supplied exact subject" is only checkable once the preimage
is defined. Two readings are possible, and they have different consequences:
- (A) exact subject: `scope.digest = H(subject)`. Each new candidate (for example each remediation round)
  needs a new requirements revision, so a new candidate digest and a new organization admission.
- (B) scope: the digest names a slice-level scope, and a separate predicate "subject ∈ scope" is needed. Its
  natural owner would be `slice-campaign` (it verifies candidate snapshots, `subject_binding.rs:276`). It is
  larger and needs a scope preimage that nothing defines either.

**Recommendation (P): (A).** The schema owner defines `scope.digest := sha256(canonical(subject))`, using the
episode owner's codec over `Subject::to_js_value()` (`prepare.rs:31-37`), so one codec governs the subject. It
is a fully determined check, so it belongs in deterministic machinery (system-design §9). The accepted 1A
subtree is unchanged: the field already exists, and only its meaning is written down.

**What the owner publishes.** (P) Contract text: the preimage definition. A pure function
`subject_scope_digest(subject) -> Digest`, placed with the subject codec (review-episode-core) or in S-A.
Inputs: `subject` (**host**: from the binding, `native.rs:1400`); `scope.digest` (**digest**: inside the
requirements preimage the host already checks against the candidate, `native.rs:1395-1399`).

**How the gate consumes it.** (P) A new S-A check (for example `CheckId::S1`):
`subject_scope_digest(request.subject) == obligation.scope_digest`, evaluated with O1–O9 and never
short-circuited. In review-delivery, `discharge(ScopeDigestToSubject) = Admitted(&[S1])`. The preparation
blocker stays unconditional, and only the discharge changes, which keeps the S-B design (prepare.rs:7-8).

**Next slice.** S. Contract text (owner sign-off), S-A check plus negative tests (subject mismatch refuses;
the frozen stand-in world becomes a negative unless regenerated), and one discharge line. Unblocks: the first
of three kinds moves from assumed to admitted, at fixture standing.

## 3. Item 2: `problem-scope-to-episode-identity`

**Question.** When does a review episode belong to an organization's problem scope?

**Facts.**
- F: `ProblemScope {run_id, plan_lineage, slice_number}` (`schema.rs:191-198`). Episode
  `Identity {run_id, slice_number, attempt_id, plan_version, review_obligation_id, review_episode_id}`
  (`review-episode-core/src/identity.rs:27-34`).
- F: "planLineage: identity.planVersion (r2 renamed planVersion to planLineage); the value mapping is a
  reading, not a stated rule" (`frozen/provenance.json:22`).
- F: `plan_lineage|planLineage` occurs in 22 files, all in `work-engine-organization`, `-organization-authority`,
  `-review-preparation`, `-review-delivery` and the P1-F `-authority-store` adapter. That is a repository-wide
  grep excluding `target/`, `.claude/`, `records/` and `metrics/`. No planning or campaign owner uses the term
  within that domain.
- F: `slice-campaign` mints `CampaignIdentity` (`contract.rs:15-50`) and derives the episode id from
  `{identity, obligationId}` (`contract.rs:368`).
- F: on the hosted path, `work-engine-server/src` and `work-engine-manual-host/src` have 0 hits for
  `run_id|plan_version|slice_number|CampaignIdentity`. The only `review_episode` use there is result-shape
  parsing (`pi_native.rs:732-734`). This is a name-based search of two crates, so **within that subset**
  the hosted path carries no episode identity. Other names: NE.

**Ill-posedness.** As stated ("problem scope maps to episode identity") it is not a function.
`attempt_id` and `review_episode_id` have no counterpart in the problem scope, and a lineage covers many
versions. The well-posed form is a **predicate**:

`episode_within_scope(identity, obligation_id, scope)` ⇔ `identity.run_id = scope.run_id` ∧
`identity.slice_number = scope.slice_number` ∧ `member(identity.plan_version, scope.plan_lineage)` ∧
`identity.review_obligation_id = obligation_id` ∧ `identity.review_episode_id = episode_id(identity, obligation_id)`.

Every term except `member` is already owned by `slice-campaign`. `member` has **no owner** (see the facts
above).

**Recommendation (P).** Owner: `slice-campaign` on the campaign path. It holds every fact the predicate needs
and is accountable for changing episode identity. For `member`, pick one of two:
- (i) undo the r2 rename semantically, so `planLineage` is compared by equality with `planVersion`. A new
  plan version then needs a new requirements revision, which matches item 1(A).
- (ii) name a planning owner for lineage membership. No such owner was found.

I recommend (i): it needs no new owner and gives deterministic equality.

**What the owner publishes.** (P) `slice_campaign::episode_within_scope(&EpisodeIdentity, obligation_id,
&ProblemScopeRef) -> Result<(), Mismatch>`, or a `ProblemScopeRef`-shaped projection
`CampaignIdentity::problem_scope()` plus the existing episode-id derivation made public. Inputs: the episode
identity (**host**: read from the episode state the consumer already holds, not supplied by the caller;
`review-episode-core` `Authority` carries `identity`, `identity.rs:138-145`); the problem scope (**digest**:
from the requirements preimage); the obligation id (**host**: from the binding).

**How the gate consumes it.** (P) This applies at the **consumption** boundary only, because the episode
exists only there. In review-delivery's consumption regate, evaluate the predicate against the episode
state's identity before the `RecordResult` transition. `discharge(ProblemScopeToEpisodeIdentity)` becomes a
host-check outcome rather than an S-A `CheckId`, since S-A has no episode input. A dependency edge
review-delivery → slice-campaign must be accepted, or the predicate must be placed in a small shared crate
that slice-campaign owns. NE which of the two the dependency graph tolerates; I did not read `Cargo.toml`
graphs.

**Next slice.** M: the lineage decision, the published predicate with tests in slice-campaign, consumption-gate
wiring, and regenerated fixtures. Unblocks: the episode-binding kind on the campaign path. On the hosted path
it stays unowned until hosted review jobs carry a campaign identity (decision D7).

## 4. Item 3: `consumption-evidence-mapping:<condition>`

**Question.** Which evidence admission (boundary, consumer) satisfies a condition's (boundary, consumer
participant)?

**Facts.**
- F: candidate condition: `boundary: Id`, `consumer`: "the participant ID the condition's `consumerClass` maps
  to" (`work-engine-organization/src/derive.rs:64-79`). Frozen values: `builder_projection`/`vantage-builder`
  and `campaign_terminalization`/`component:campaign`, from classes `builder`/`campaign`
  (`tests/fixtures/golden/frozen-{candidate,requirements}.canonical.json`, `-o` grep).
- F: the episode evidence admission carries `boundary` and `consumer` as non-empty text with no vocabulary
  (`review-episode-core/src/state.rs:286-319`).
- F: `claim-evidence` owns the boundary vocabulary. `production-path-claim-v1` has
  `consumptionBoundary: enum [builder_projection, campaign_terminalization]` and `consumer: string`
  (`claim-evidence/schemas/production-path-claim-v1.schema.json:1`). It is the same vocabulary as the
  organization's.
- F: **`slice-campaign` already implements the rule**, privately: `builder_projection` →
  `slice-builder:<identity.key()>`, `campaign_terminalization` → `slice-campaign:<identity.key()>`, and any
  other value is refused (`contract.rs:374-382`, inside `validate_selected_claim`, `:320`).

**Recommendation (P).** Owners: `slice-campaign` for the consumer derivation, with `claim-evidence` owning the
boundary enum. This is not a new rule. The rule exists and needs one addition, class correspondence:
`builder_projection` ↔ consumer class `builder`, `campaign_terminalization` ↔ `campaign`. Without it a
condition whose consumer is the builder could be satisfied by a campaign-terminalization admission.

**What the owner publishes.** (P) `slice_campaign::expected_admission(boundary, consumer_class, &CampaignIdentity)
-> Result<(Boundary, ConsumerString), Unsupported>`, extracted from `contract.rs:374-382` with the class check
added. Inputs: boundary and consumer class (**digest**: from the requirements and candidate preimages); the
identity (**host**: the episode state, as in item 2); the admission's boundary and consumer (**host**: read
from the episode's `evidenceAdmissions`, not supplied).

**How the gate consumes it.** (P) In the consumption regate, for each selected condition, require exactly the
admissions whose `(boundary, consumer)` equal `expected_admission(...)` and whose status is `established`.
This replaces the fixture's `ConditionBinding.admission_consumer` assumption (`fixture.rs:72-85`). The
discharge is a host-check outcome.

**Next slice.** S once item 2's identity input exists (they share the episode-identity read). The two can be
built in one M slice. Unblocks: the third kind on the campaign path.

## 5. Item 4: organization anchor

**Question.** Which record makes an organization id real (issued, related to a candidate/problem scope and
to principals, revocable), so the gate derives it rather than accepting it?

**Facts.**
- F: `OperationRequest.organization_id` is a free string (`organization-authority/src/gate.rs:226-255`). The host
  passes the caller's value through, labelled `caller_asserted_unanchored` (`review-delivery/src/gate.rs:67-71`;
  `native.rs:1359-1361`, `:1401-1402`).
- F: its only consumer is vantage-binding selection for O8 and R3: `vantage_bindings(state, organization,
  vantage)` filters `VantageActorBinding.organization_id` (`organization-authority/src/gate.rs:524-533`,
  `:650-668`, `:1169`). The record has `organization_id: String` (`records.rs:97-104`).
- F: the other S-A records are keyed by **problem scope**, not organization: `ComponentBinding`, `DecisionScope`,
  `AuthorityGrant` (`records.rs:106-138`). No record relates an organization id to a problem scope or candidate.
  Over the six `Record` variants (`records.rs:163-170`), only `VantageActorBinding` mentions an organization.
- F: vantage ids are per-obligation (`vantage:<obligationId>`, `derive.rs:372`), so `(organization, vantage)`
  only disambiguates if the organization identifies one candidate lineage.
- F: the issuer of S-A records is `deployment_owner` (`records.rs:70`, `:153`). Production records have no
  constructor (`records.rs:1-5`).
- F: doctrine places admission with organizational authority and names the result a durable revision: "organizational
  authority admits the surviving candidate → ExecutionEnvelope records the admitted selection as a new revision"
  (`design/architecture/organizational-compilation.md:414-416`, `:423`, accepted 2026-09-15, design-only).
  The pending idea `design/ideas/pending/execution-envelope-live-effect-admission.md:384-416` defines
  `envelope_identity` as "stable across revisions; names 'this problem's organization'", plus `revision_id`,
  `predecessor_revision`, `organizational_subject_ref` and `producer`. It has no implementation (`:375`).
- F: the P1-F `authority-store` checks an installed `candidate_digest` as "organization identity"
  (`work-engine-authority-store/src/store.rs:553-568`). Reconciliation N8 keeps P1-F out of the issuer role
  (`RECONCILIATION.md` §1).
- F: O1 (approved) makes organizational authority the single owner of standing, records and Gate O
  (`s2/ROOT-S2-ADDENDUM.md` O1).

**Ill-posedness.** "Organization id" conflates two things. (a) The **admitted organization**: one problem's
organization, revisioned (the envelope). (b) A **tenant or installation** (whose deployment). For (b) the
deployment owner already is the single issuer, and no multi-tenant requirement was found (NE: not searched
beyond these records). Only (a) gives `(organization, vantage)` a defined meaning.

**Recommendation (P).** Owner: **organizational authority (S-A crate)**, as O1 already decided. Record: a new
kind, the admitted organization. **No existing record serves; a new record must be created.**

```text
OrganizationAdmission v1 { organizationId (= envelope identity, stable), revision, predecessor?: IssuanceId,
  problemScope: ProblemScopeRef, requirementsDigest, candidateContentDigest, issuer: deployment_owner }
```

- Issue: organizational authority creates one when it admits a candidate. The first revision is
  create-only, and a successor names its predecessor (no branching, per the idea doc §3 table).
- Bind: `VantageActorBinding.organization` becomes the admission's `IssuanceId`, not a string. Component
  bindings, grants and decision scopes keep their problem-scope key, and the admission's `problemScope`
  relates the two key families.
- Revoke or supersede: 6.1 F5 already makes every kind revocable. A revoked or superseded admission makes
  every binding that pins it resolve to `revoked`/`stale`, which the existing state table handles
  (PROPOSAL §4.4).

**How the gate derives it.** (P) `OperationRequest` loses `organization_id`. In one observation the gate
resolves the **unique** committed, unrevoked, head `OrganizationAdmission` whose `(candidateContentDigest,
requirementsDigest, problemScope)` equals the request's. Those inputs are already host-derived or
digest-bound: candidate digest from the binding, requirements preimage checked (`native.rs:1394-1404`).
Zero matches refuses with `organization_unadmitted`. More than one means two heads, an invariant failure, and
refuses. The result is pinned. O8 and R3 then select bindings by that `IssuanceId`. As a side effect O3 stops
being self-consistency only (`gate.rs:236-237`): the candidate is admitted because a record says so.

**Next slice.** M at fixture standing: a new record kind with a fixture constructor, gate derivation, removal
of the request field, and negatives (unadmitted, revoked, superseded, two heads, binding pinned to another
organization's admission). It is all in S-A and review-delivery, and touches neither the 1A subtree nor
preparation. **Production is L.** It needs O6 (observation-source storage), the K-c independent issuer
(RECONCILIATION §4) and production-admission authorization. Unblocks: removes `caller_asserted_unanchored`
from O8/R3 at fixture standing and gives the production path a defined record to issue.

## 6. Dependencies, order and what step 3 needs

```text
1 (scope digest)   ── independent ───────────────────────────────┐
2 (episode pred.)  ── needs D3 (lineage) ──> 3 (consumption map) ──┼─> production consumption
4 (org anchor)     ── fixture: independent; production: O6 + K-c ┘    (+ O6, K-c, prod authorization)
```

- Order (P): 4-fixture and 1 in parallel (both S-A-local). Then 2+3 as one slice in slice-campaign plus the
  review-delivery consumption gate.
- Interaction: 1(A) and 2(i) both make requirements per-candidate and per-plan-version. Decide them together.
  Under (A)/(i), 4's admission is per candidate, and the envelope's successor chain carries remediation rounds.
- **Step 3 (hosted read-only review: produce + validate implementation-review/v1, no consumption)** needs none of
  1–4. RECONCILIATION §3 already scopes it as produced and validated but "not consumed with production standing".
  The hosted path has no episode identity (§3 facts), so 2 and 3 have nothing to bind there. 1 and 4 would only
  matter if step 3 also runs fixture org admission on the hosted path. In that case 4-fixture is cheap and
  removes one label, but it is optional.
- **Production consumption** needs 1, 2, 3, 4-production, plus O6, K-c, D7 if consumption happens on the hosted
  path, and production authorization, which is currently withheld.

## 7. Operator decisions

| # | Question | Recommendation |
|---|---|---|
| D1 | `scope.digest` meaning: (A) exact-subject digest under the episode subject codec, (B) slice scope + campaign membership predicate, or (C) drop it from v0 and bind the subject only at admission | **(A)**. Consequence: one requirements revision per candidate |
| D2 | `slice-campaign` owns the episode-scope predicate (item 2) and the consumption mapping (item 3) on the campaign path, published as pure functions | **Yes** |
| D3 | `planLineage`: (i) compare by equality with `planVersion` (undo the r2 rename's semantic gap), or (ii) name a planning owner for lineage membership | **(i)**. Flag: as worded, item 2 is ill-posed; (ii) has no existing owner |
| D4 | "Organization id" means the admitted organization (envelope identity, stable across revisions), not a tenant | **Yes** |
| D5 | Create the new S-A record kind `OrganizationAdmission`, have the gate derive the organization from it, and remove `organization_id` from `OperationRequest`; fixture first, production after O6 + K-c | **Yes**. Flag: no existing record; a new one must be created |
| D6 | Step 3 proceeds without O7 owners or the anchor, scoped to produce + validate with no consumption | **Yes** |
| D7 | Hosted-path episode identity: hosted review jobs dispatched by slice-campaign carry `CampaignIdentity`, or a new hosted owner | **Defer** until hosted production consumption is wanted |
| D8 | Accept a dependency from review-delivery's consumption gate on slice-campaign's published predicates, or have them placed in a small campaign-owned crate | Owner's call. The dependency graph was not read |

## 8. Limits of this reading

- Read at cited lines only, n = 1 per site. No builds or tests. Coverage hooks reported `unresolved_calls` for
  several files, so the graph was not relied on and all citations are direct reads.
- NE: contract 3.1 text; `slice-campaign` remediation semantics (whether a remediation round reuses an
  episode); lifecycle-store subject registration ↔ review subject (RECONCILIATION §2.1 calls this the O7 rule on
  the hosted path; I did not verify its fields); the `Cargo.toml` dependency graph; whether any non-test caller
  constructs `OperationRequest` outside review-delivery.
- Process: context-pressure delivery for this session was not established (no listener binding was supplied).
  The read volume stayed small.
