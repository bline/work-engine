# O7 owners implementation plan (planning only)

Author: Claude Opus 5.5 (`claude-opus-5-5`), read-only planning worker, 2026-11-18. This document is a plan and
grants no implementation authority. No builds, tests, git writes, chatboard posts or subagents were used.

**Tree.** `/home/bline/code/work-engine`, branch `rescue-detached-20260911`. HEAD is **`f772612f`, not the
packet's `432657cc`**. `432657cc` is an ancestor of HEAD, and `git diff --stat 432657cc HEAD` over
`work-engine-organization`, `-organization-authority`, `-review-preparation`, `-review-delivery`,
`slice-campaign`, `review-episode-core` and `work-engine-review-hosted` is empty. Every citation below therefore
holds at both commits. Paths are relative to `rust/crates/` unless stated otherwise.

**Notation.** **V** = I read the line in this session. **U** = unverified (inferred or not read). **P** = proposal.

---

## 0. Corrections to the D1 proposal that change the plan

I checked the D1 citations I relied on, and nearly all of them hold: prepare.rs blockers, delivery `discharge`,
the `OperationRequest` shape, `records.rs`, slice-campaign `contract.rs:368-382` and `episode_digest`, and the
episode `Identity`. These four points differ from the proposal or add to it:

1. **The consumption check is in review-preparation, not review-delivery.** The episode identity and the
   evidence admissions are checked in `work-engine-review-preparation/src/fixture.rs::consume_fixture_result`
   (`:559-784`, V). That function restates the episode (`:573-629`), matches admissions by `(boundary,
   admission_consumer)` (`:661-686`), checks the references and the `established` status (`:687-752`), then calls
   `RecordResult` (`:766-777`). review-delivery reaches it only through `binding.consume(...)`
   (`work-engine-review-delivery/src/native.rs:1641`, V). Items 2 and 3 therefore change `fixture.rs`. The D8
   dependency edge would come from **review-preparation** (and so also from delivery and hosted, which depend
   on it), not only from review-delivery.
2. **Item 3's "private code" is `slice-campaign/src/contract.rs:374-382`, inside
   `validate_selected_claim` (`:320`)** (V): `builder_projection` → `slice-builder:<identity.key()>`,
   `campaign_terminalization` → `slice-campaign:<identity.key()>`, anything else is refused. The episode-id rule
   sits next to it (`:368-373`), and the codec it uses is already public: `pub fn episode_digest`
   (`contract.rs:215-220`, which is `review_episode_core::codec::digest`).
3. **The hosted crate does not become consumable when the four items land.** `work-engine-review-hosted`
   writes `standing: "owner_pending"`, `org_admission: "none_requested"` and `consumable: false` unconditionally
   (`work-engine-review-hosted/src/check.rs:854-858`, V; `CLAIMS.md:85-86`, V). It depends on review-delivery
   and review-preparation but runs neither S-A admission nor the consumer, and it carries preparation blockers
   verbatim. Implementing items 1-4 changes nothing in that crate. Its standing moves only after D7 (hosted
   episode identity), a hosted admission route, O6, K-c and production authorization. The packet's sentence
   "results stay `owner_pending` because the four items have no owner implementation" is true but incomplete.
4. **Item 1 cannot leave every test world untouched.** In both the frozen requirements and the R2-S assumption
   world, `scope.digest` is the stand-in `5212141d…` (= `subject.patch_identity`; V, by parsing
   `work-engine-organization/tests/fixtures/frozen/requirements.json` and
   `work-engine-review-delivery/tests/fixtures/assumption-world/requirements.json`). Under D1(A), any world that
   reaches S-A admission needs a regenerated `scope.digest`. The frozen 1A file can stay byte-identical, so the
   1A goldens are unchanged, but frozen-world admissions then become negatives.

---

## 1. Item 1: `scope.digest` is the exact-subject digest (S-A; D1 = (A))

**Owner.** Organizational authority (`work-engine-organization-authority`, S-A) implements the check.
`work-engine-organization` (contract 3.1) owns the field's meaning. The subject codec belongs to
`review-episode-core`.

**Current tree (V).**
- `work-engine-organization/src/schema.rs:211-215` `Obligation { id, result_contract, scope_digest: Digest }`;
  parsed with `Digest::from_hex` at `:915-917`. `Digest` is a 32-byte value; hex in, `to_hex` out
  (`digest.rs:59,76,99`).
- `work-engine-organization-authority/src/gate.rs:215-223` `SubjectRef` (its doc says no check relates it to
  the scope, O7). `:31-52` `CheckId` and `CheckId::ALL` (nine ids). `:448-582` `evaluate()` inserts every
  check, and `:568-576` requires that every `CheckId::ALL` entry was evaluated.
- `work-engine-review-delivery/src/gate.rs:94-107` `discharge()`; `:103-105` maps the three mapping kinds to
  `AssumedDischarge`.
- `work-engine-review-preparation/src/prepare.rs:529-535` emits the blocker unconditionally (unchanged, per
  S-B design `prepare.rs:7-8`).
- The digest the episode already uses for its subject is `review_episode_core::codec::digest(&JsValue)`
  (`review-episode-core/src/codec.rs:149-152`: sha256 of canonical JSON, lowercase hex). The fixture
  consumer already asserts `digest(declaration.subject.to_js_value()) == state.subject.sha256`
  (`fixture.rs:632-642`). So D1(A) can reuse a digest the episode owner already defines.

**Changes.**
1. Contract text (owner sign-off): `scope.digest := review_episode_core::codec::digest({commit, tree,
   patchIdentity})`, lowercase hex. Contract 3.1 is **not in the repository** (V: a grep of `design/`, `docs/`,
   `planning/` and `records/` finds `scope.digest` only in records of the step-3 run; `gap-decisions-r1.md`,
   cited at `schema.rs:1`, has no file in the tree). The definition must be recorded somewhere: see operator
   decision O-1.
2. A pure function `subject_scope_digest(&SubjectRef) -> Digest`. Placement options: (a) in S-A, with a new
   dependency S-A → `review-episode-core` (light: sha2 and ryu-js only; acyclic, V from `Cargo.toml`); (b) in
   `review-episode-core`, returning hex, with S-A converting. (a) keeps the check and its function together.
   A second codec in `work-engine-organization` is not recommended: two canonicalizers for one subject could
   drift.
3. A new `CheckId::S1` ("the subject's scope digest equals every obligation the vantage serves") in
   `gate.rs:31-52` and `ALL`. It is evaluated in `evaluate()` next to `r1_r2` (which already resolves the
   vantage's obligations), is never short-circuited, and has a new `ReasonCode::ScopeDigestMismatch`. When no
   candidate exists, `S1` receives `CandidateUnavailable` like O4/O5/O8 (`gate.rs:497-508`).
   `admission_state`'s re-evaluation (`:1275-1340`) covers it automatically.
4. `work-engine-review-delivery/src/gate.rs:103`: `ScopeDigestToSubject => Discharge::Admitted(&[CheckId::S1])`.
   Update the module docs (`:17-22`) that say all three mapping kinds are assumed.
5. Test world: regenerate `work-engine-review-delivery/tests/fixtures/assumption-world/requirements.json`
   (`scope.digest` = digest of the test subject; expected `18108e56…`, the frozen episode authority's
   `initialSubject.sha256`. **U**: computed by inference from the merged consumer test, not recomputed). Update
   `DERIVATION.md` there. Any candidate content digest the tests pin changes with it. The S-A test helper
   (`work-engine-organization-authority/tests/admission.rs:103`) needs a requirements world whose scope digest
   matches its subject.

**Acceptance evidence.**
- Positive: an admission over the regenerated world has `S1` passed, and the delivery discharge for
  `scope-digest-to-subject:*` moves from `AssumedDischarges` to `AdmittedDischarges` with `[S1]` (delivery
  `tests/gate.rs` and `tests/native_vertical.rs`).
- Negatives: a subject differing in each of `commit`, `tree` and `patchIdentity` gives `S1` refused, and no
  other check changes. The frozen-stand-in world refuses `S1` (a named negative control). A pin revoked after
  admission still gives `Revoked`, regression only. A multi-obligation vantage where one obligation mismatches
  refuses (this mirrors S1's multi-obligation control).
- Hand mutants, in the S1 `mutants/` format (`records/org-hardening-owners-20261117/s1/mutants/gen_plan.py`):
  `S1` always passes; `S1` compares only `patchIdentity`; `S1` checks only the first served obligation;
  `discharge(ScopeDigestToSubject)` reverts to `AssumedDischarge`; `S1` dropped from `ALL`. This last one should
  panic at `gate.rs:572`, so it needs a test that asserts the outcome list. Each must be killed by a named test.
- Clippy (`-D warnings`) and fmt across the three feature sets in the S1 root-gate plan
  (`s1/root-gate-plan.json`: fixture, default, test-support).

**Risks and open questions.**
- R1.1: Under (A), each remediation round needs a new requirements revision, so a new candidate digest and a new
  admission. D1 accepted this. It interacts with item 4: one `OrganizationAdmission` revision per candidate.
- R1.2: The function's placement (S-A vs review-episode-core) is a small owner choice; it is not an operator
  decision unless a new dependency edge needs approval.
- R1.3: Hosted r1/r2 requirements (`work-engine-review-hosted/fixtures/hosted-route-qualification-r{1,2}/`) also
  use stand-in digests (U: not parsed). They are unaffected, because hosted never runs S-A.

**Size.** S (D1 says S; confirmed).

---

## 2. Item 2: episode-within-scope predicate (slice-campaign; D3 = planLineage equality)

**Owner.** `slice-campaign` (D2). It mints `CampaignIdentity` (`slice-campaign/src/contract.rs:15-50`, V)
and the episode id (`:368-373`, V).

**Current tree (V).**
- `work-engine-organization/src/schema.rs:191-198` `ProblemScope { run_id, plan_lineage, slice_number }`.
- `review-episode-core/src/identity.rs:27-34` `Identity { run_id, slice_number, attempt_id, plan_version,
  review_obligation_id, review_episode_id }` (`JsString` fields; `to_string_checked`).
- `slice-campaign/src/contract.rs:15-22` `CampaignIdentity`; `:46-51` `key()`; `:215-220` `pub fn
  episode_digest`; `:368-373` the id rule `episode_digest({identity, obligationId})[..32]` (private, inline).
- Consumer: `work-engine-review-preparation/src/fixture.rs:359-380` checks the declared problem scope against
  the candidate; `:573-629` checks the declared episode against `state.identity()`. **Nothing relates the
  two**, and that missing relation is item 2.
- Frozen world: `planLineage = planVersion = "timestamp-projection-v1"`, `runId` and `sliceNumber` equal (V,
  parsed). The equality predicate (i) passes on the existing world without regeneration. Whether
  `reviewEpisodeId ad20341a…` equals `episode_digest(...)[..32]` is **U** (not recomputed). If it is not, the
  frozen episode authority is a negative, and the test world needs a derived episode.

**Changes.**
1. Publish in slice-campaign (or a campaign-owned crate, D8):
   `episode_id(identity: &CampaignIdentity, obligation_id: &str) -> Result<String>` extracted from
   `contract.rs:368`, and
   `episode_within_scope(episode: &review_episode_core::identity::Identity, obligation_id: &str, scope:
   &ProblemScope-shaped ref) -> Result<(), ScopeMismatch>` with terms run, slice, `plan_version ==
   plan_lineage` (D3(i)), `review_obligation_id == obligation_id`, and `review_episode_id == episode_id(...)`.
   `validate_selected_claim` must call the extracted `episode_id`, so one rule exists, not two copies.
2. Consumer: in `consume_fixture_result`, after the declaration checks (`fixture.rs:629`) and before the
   admissions, evaluate the predicate over `state.identity()` (host), `candidate.problem_scope()`
   (digest-bound), and `declaration.obligation_id` (already validated against the candidate by `Selection`).
   Add a new `ConsumeRefusal::EpisodeOutsideScope { term }` (`fixture.rs:137`, `#[non_exhaustive]`).
3. Delivery discharge: `ProblemScopeToEpisodeIdentity` cannot be an S-A `CheckId`, because S-A has no episode
   input. This needs a new `Discharge` variant (for example `HostChecked`) that `assess`
   (`work-engine-review-delivery/src/gate.rs:319-386`) resolves from the binding, which already holds the
   candidate, the declaration and the episode inputs (`binding.rs:131-140`, V). That changes the documented
   S-B invariant that admitted kinds are "decided by those ids alone" (`gate.rs:17-22`). It is a design point
   for the slice's reviewer (open question Q2.2).

**Acceptance evidence.**
- Positive: the frozen/assumption world passes, and the kind is reported as host-checked (not assumed) at both
  boundaries.
- Negatives, one per term, each refusing before `RecordResult` with the state untouched: run id differs; slice
  differs; `plan_version ≠ plan_lineage` (D3 control); obligation differs; `review_episode_id` not derived
  from the identity (for example the attempt id changed, the id kept). Also a declaration that restates a
  consistent but wrong episode, which shows the predicate reads the state rather than the declaration.
- slice-campaign: unit tests for `episode_id` and the predicate, plus the existing selection-validation tests
  unchanged (`slice-campaign/tests/admission.rs`, `qualified_initial.rs`; U as to which cover `:368`).
- Hand mutants: drop each predicate term (five mutants); compare the episode id against the declaration instead
  of the state; `HostChecked` resolves to passed without calling the predicate.

**Risks and open questions.**
- Q2.1 (D8): dependency placement. slice-campaign pulls `rusqlite` (bundled C), `rustix`,
  `workspace-publication`, `review-episode-store`, `review-execution-custody` and `claim-evidence`
  (`slice-campaign/Cargo.toml`, V). An edge from review-preparation would add these to preparation, delivery and
  hosted builds. The edge would be acyclic: none of those crates depends on the org, preparation or delivery
  crates (V). The alternative is a small campaign-owned crate (for example `slice-campaign-identity`:
  `CampaignIdentity`, `episode_id`, the predicate and `expected_admission`, depending only on
  `review-episode-core` and `serde_json`), re-exported by slice-campaign. **D8 must be decided before slice B
  starts.**
- Q2.2: whether the host-checked discharge is acceptable as a new `Discharge` variant evaluated in `assess`,
  or whether the consumer alone decides it and the gate keeps it assumed with a pointer. Recommendation: a new
  variant, so the label truthfully stops saying "assumed".
- Q2.3: whether `DeclaredProblemScope` and `DeclaredEpisode` (`fixture.rs:76-95`) stay as restated assumptions
  or are removed now that the predicate reads host values. Recommendation: keep them in this slice (smaller
  diff) and record removal as follow-up.
- R2.4: remediation semantics. Whether a remediation round reuses an episode (attempt id or plan version) is
  **U**. Under D3(i), a new plan version needs a new requirements revision; the proposal already flags this.
- R2.5: the hosted path carries no `CampaignIdentity` (D7 deferred). Nothing changes there.

**Size.** M (D1 says M; confirmed, larger if D8 chooses a new crate).

---

## 3. Item 3: consumption-evidence mapping (slice-campaign + claim-evidence vocabulary)

**Owner.** `slice-campaign` for the consumer derivation (D2). `claim-evidence` owns the boundary enum
(`claim-evidence/schemas/production-path-claim-v1.schema.json`; D1 cites `:1`, **U**: not re-read).

**Where the private rule is (V).** `slice-campaign/src/contract.rs:374-382`, inside `validate_selected_claim`
(`:320-451`), called from `validate_selection` at `:282`. Selection also requires the selected obligation's
boundaries to be exactly `{builder_projection, campaign_terminalization}` (`:287-298`).

**Current tree (V).**
- `work-engine-organization/src/derive.rs:64-79` `CandidateCondition { consumer: participant id,
  boundary, … }`; `schema.rs:264` `consumer_class: Id`.
- Episode admission shape: `review-episode-core/src/state.rs:286-319` `validate_admission` (boundary and consumer
  are non-empty text only).
- Fixture stand-in: `ConditionBinding.admission_consumer` (`fixture.rs:111-120`), used in `fixture_coverage`
  (`:416-425`, `:458`) and in matching (`:675`).

**Changes.**
1. Publish `expected_admission(boundary, consumer_class, &CampaignIdentity) -> Result<(String, String),
   Unsupported>`, extracted from `contract.rs:374-382`, with the class correspondence added
   (`builder_projection` ↔ class `builder`, `campaign_terminalization` ↔ class `campaign`; anything else is
   refused). `validate_selected_claim` calls it. The class input there is **U**: the selection claim carries no
   `consumerClass`, so the existing call may need a boundary-only variant, or must derive the class from the
   boundary. The builder must resolve this without forking the rule.
2. Consumer: in `consume_fixture_result`, derive each covered condition's expected `(boundary, consumer)` from
   `expected_admission(condition.boundary, condition.consumer_class, identity-from-state)`, and match on that
   instead of `binding.admission_consumer`. The candidate condition exposes `consumer` (a participant id), not
   `consumer_class`. **Resolved (V):** derive maps class to participant one-to-one: `builder` → `vantage-builder`,
   `campaign` → `component:campaign`, and an unknown class is unresolved (`work-engine-organization/src/derive.rs:29-30`,
   `:453-459`, `:461-479`). The class is therefore recoverable from the candidate with no shape change. To avoid
   a second spelling of that map, `work-engine-organization` should publish its inverse (for example
   `consumer_class_of(participant) -> Option<&'static str>`, next to `consumer_of`). That is a small public
   addition, not a 1A shape change, but it needs organization-crate owner agreement.
3. `ConditionBinding.admission_consumer`: keep it as a restated assumption that must equal the derived value
   (refuse on conflict), or remove it. Recommendation: keep and check in this slice.
4. Delivery discharge: `ConsumptionEvidenceMapping` uses the same `HostChecked` variant as item 2. At
   `BeforeSpawn`, only derivability can be checked, because admissions arrive with the result. At
   `BeforeConsumption`, the match is checked.

**Acceptance evidence.**
- Positive: both frozen conditions (`cond-builder-projection` → `slice-builder:<key>`, `cond-campaign-terminalization` →
  `slice-campaign:<key>`) are admitted by derivation.
- Negatives: a builder-class condition satisfied by a `campaign_terminalization` admission is refused (the
  class-crossing control); an admission consumer for another identity key is refused (for example a different
  attempt id); an unsupported boundary is refused; a declaration `admission_consumer` that differs from the
  derived value is refused; a status other than `established` is still refused (regression).
- slice-campaign: the selection tests that exercise `"claim consumer differs"` / `"claim boundary unsupported"`
  stay green, which shows the extraction preserved behaviour.
- Hand mutants: swap the two boundary arms; drop the class check; use the declaration's consumer instead of the
  derived one; derive from the declared episode rather than `state.identity()`.

**Risks and open questions.**
- Q3.1: D1 sized this S "after 2". It shares item 2's identity read, and D1 already plans one slice for both.
- Q3.2 (mostly resolved): the candidate does **not** need a `consumer_class` field. The class is recoverable
  through derive's one-to-one map (change 2 above). What remains is a small public inverse function in
  `work-engine-organization`, so the 1A candidate shape and goldens stay unchanged.
- R3.3: claim-evidence's enum is used only by reference. If the slice wants a shared Rust enum instead of
  strings, that adds a claim-evidence change and its owner. Recommendation: not in this slice.

**Size.** S on top of item 2 (D1 says S; confirmed now that Q3.2 needs no candidate change).

---

## 4. Item 4: `OrganizationAdmission` record at fixture standing (S-A; D4/D5)

**Owner.** Organizational authority (S-A), per O1 and D5.

**Current tree (V).**
- `work-engine-organization-authority/src/gate.rs:226-264` `OperationRequest { organization_id, requirements,
  candidate_digest, vantage, subject }`, with `new(...)` at `:238-251` and `organization_id()` at `:253`.
- Consumers of the org id: `evaluate` passes it to `o8(...)` (`:524-535`); `r3` calls
  `vantage_bindings(state, &request.organization_id, vantage)` (`:1169`); the filter is at `:651-668`.
- `records.rs:96-104` `VantageActorBinding { organization_id: String, … }`; `:162-170` `Record` (six
  variants); `:172-240` `kind()` and `body_digest()`; `:242-389` fixture constructors
  (`fixture_vantage_binding` at `:293-309`).
- `observation.rs`: `IssuanceStatus { Committed, Revoked, Superseded { by } }` already exists. The fixture
  source has `issue`, `revoke` and `supersede` (`fixture.rs:35,57,70`). Supersession therefore needs no new
  machinery.
- `gate.rs:301-320` `AuthorityPins`, six sets. The point check (`:1275-1340`) turns any pin that is revoked or
  superseded into `Revoked` or `Stale`.
- Delivery: `gate.rs:63-71` `CALLER_ASSERTED_LABEL`; `:204-208` `CallerAsserted { organization_id,
  actor_identity }`; `:367` copies `request.organization_id()`. `native.rs:1366-1394` `FixtureAuthority {
  organization_id, … }` (docs from `:1366`, struct at `:1387`); `:1409-1432` `construct_request` passes it to `OperationRequest::new`.
- `OperationRequest::new` call sites (V, repository-wide grep): `organization-authority/tests/admission.rs:103,
  :1416`; `review-delivery/src/native.rs:1420`; `review-delivery/tests/common/mod.rs:572`;
  `review-preparation/tests/common/mod.rs:354`. No production caller exists outside review-delivery.
  `fixture_vantage_binding` has 12 call sites, and there are 22 `organization_id|ORG` hits in the delivery,
  preparation and authority tests (V, grep counts).

**Changes.**
1. `records.rs`: a new `OrganizationAdmission { organization_id (stable envelope identity), revision: u64,
   predecessor: Option<IssuanceId>, problem_scope: ProblemScopeRef, requirements_digest, candidate_content_digest }`,
   plus `Record::OrganizationAdmission`, `kind() = "organization_admission"`, a `body_digest` encoding and
   `Record::fixture_organization_admission(...)`. Issuer: `deployment_owner` (like the others).
2. `VantageActorBinding.organization_id: String` → `organization: IssuanceId` (the admission), with its
   encoding and fixture constructor updated.
3. `OperationRequest` loses `organization_id` (D5): update `new`, remove the accessor, update the 5 call sites.
4. `evaluate`: resolve the **unique** committed `OrganizationAdmission` whose `(candidate_content_digest,
   requirements_digest, problem_scope)` equal the request's (candidate digest from the request, requirements
   digest from `request.requirements_digest()`, scope from the derived candidate). Zero matches refuses with
   `organization_unadmitted`; two or more refuses with `organization_heads_ambiguous`. Pin the result (a new
   `AuthorityPins.organization_admissions` set, added to `all()`), then pass its `IssuanceId` to `o8` and `r3`.
   Where the outcome lives is open: a new `CheckId` (recommended, because it gives a distinct reason and
   mutant target) or inside O3. O3's doc (`gate.rs:32-35`) must be updated either way, because O3 is no longer
   "self-consistency only". If a new id is added, `discharge` decides which kinds require it (at least
   `Separation` and `ObserverIdentity`, which already depend on O8/R3).
5. Delivery: remove `FixtureAuthority.organization_id` and `CallerAsserted.organization_id`.
   `CALLER_ASSERTED_LABEL` stays for `actor_identity` only. Update the module docs (`native.rs:1366-1386`,
   `gate.rs:63-71`). The `AdmissionSummary` gains the admitted organization's `IssuanceId`/`organization_id`
   (host-derived, unlabelled).
6. Fixture worlds: issue one `OrganizationAdmission` in each authority test world (delivery
   `tests/common/mod.rs:~278-520`, preparation `tests/common/mod.rs`, authority `tests/admission.rs`) and
   rebind vantage bindings to it.

**Acceptance evidence.**
- Positive: the existing admission tests pass with the org derived, and the summary carries the derived org with
  no caller-asserted org.
- Negatives (named in D1): unadmitted → refused; admission revoked before admission → refused; revoked after
  admission → the point check gives `Revoked`; superseded after → `Stale`; two committed heads → refused; a
  vantage binding pinned to another organization's admission → O8/R3 refuse; an admission for another candidate
  digest, requirements digest or scope → refused (three controls).
- Compile-level: a `compile_fail` doctest showing `OperationRequest::new` no longer accepts an org id. This is
  optional, because arity already enforces it.
- Hand mutants: accept the first of two heads; skip the pin; ignore `problem_scope` in matching; O8 and R3
  select by `vantage` only; the point check ignores the new pin set.
- The three-feature clippy and fmt gate, as in S1.

**Risks and open questions.**
- Q4.1: The record schema amends the S-A record set ("Stage 0 rev6 §18 with Amendment 6.1", `records.rs:1`).
  That text is **not in the repository** (V: no `.md` contains `VantageActorBinding v1`). An owner sign-off on
  `OrganizationAdmission v1` is required, and the place to record it must be chosen (operator decision O-1).
- Q4.2: under item 1(A) and D3(i), each candidate needs an admission revision. Does a remediation round
  supersede (successor chain) or issue a new organization? D1 says successor chain, stable `organization_id`.
  The builder should implement supersession as the only branch-free path.
- R4.3: production (O6 storage, K-c issuer, production authorization) is out of scope. The production policy
  still admits nothing.
- R4.4: the P1-F `work-engine-authority-store` `candidate_digest` "organization identity"
  (`work-engine-authority-store/src/store.rs:553-568`, D1 cite, **U**) is untouched, and stays out of the
  issuer role (N8).

**Size.** M at fixture standing (D1 says M; confirmed by blast radius: 2 production crates plus 3 test suites).

---

## 5. Slice plan

Three slices. The proposal's order ("1 and 4 in parallel, then 2+3") is a preference. These are the actual
dependencies:

| Slice | Items | Crates written | Hard dependencies | Shared files with other slices |
|---|---|---|---|---|
| **A1** | 1 | org-authority (`gate.rs`, maybe `Cargo.toml`), review-delivery (`gate.rs`, tests, assumption-world fixture) | O-1 (where contract text lives) | org-authority `gate.rs` (`CheckId`, `ALL`, `evaluate`) with A2; delivery `gate.rs::discharge` with A2 and B; delivery `tests/common/mod.rs` with A2 and B |
| **A2** | 4 (fixture) | org-authority (`records.rs`, `gate.rs`, tests), review-delivery (`gate.rs`, `native.rs`, tests), review-preparation (tests only) | O-1 (record schema sign-off) | as above |
| **B** | 2 + 3 | slice-campaign (or a new campaign-owned crate), review-preparation (`fixture.rs`, `Cargo.toml`, tests), review-delivery (`gate.rs` `Discharge`/`assess`, tests), work-engine-organization (one public inverse fn, Q3.2) | **D8 placement decided** (O-2) | delivery `gate.rs`, delivery `tests/common/mod.rs` |

- **No semantic dependency exists between A1, A2 and B.** All three can run concurrently in separate
  worktrees. Concurrent work costs merge conflicts in the shared files listed above. They are small and
  mechanical (enum variants, match arms, test-world builders), but A1's regenerated assumption world changes
  candidate digests that A2's and B's test worlds read.
- Recommended schedule: **A1 ∥ A2 now**, merging A1 first: it is smaller, and it fixes the test world the
  others rebase onto. **Start B in parallel once O-2 (D8) is decided.** If the budget favours fewer rebases over
  wall-clock time, start B after A1 merges. That is a conflict-cost reason, not a dependency.
- Each slice uses the S1 shape: builder → Fable review → remediation → Fable recheck, plus root coordination
  and a root gate (fmt, three clippy feature sets, tests, hand mutants) on a clean export.

## 6. Cost estimates (API-equivalent USD, Claude subscription)

Calibration (S1, one medium slice over review-delivery and review-preparation): builder Opus high $8.98 (hit the
cap before reporting), review Opus xhigh $3.27, remediation-1 Sonnet $1.77, Fable recheck $4.46, remediation-2
Sonnet $1.23, Fable recheck-2 $2.91, root about $4, **total about $26.6**. Adjustments: all reviews are now Fable
high/xhigh (operator, 2026-11-17), so the first review is costed at Fable recheck rates ($3-5), not $3.27. The S1
builder was underfunded: an M builder needs about $11-13 to finish and report.

| Slice | Builder | First review (Fable) | Remediation(s) | Recheck(s) (Fable) | Root | **Low** | **Expected** | **High** | **Recommended cap** |
|---|---|---|---|---|---|---|---|---|---|
| A1 (S) | $4-6 | $2.5-3.5 | $1-1.5 | $2-3 | $2-3 | **$9** | **$13** | **$19** | **$16** |
| A2 (M) | $9-13 | $3.5-5 | $1.5-3 | $3-7 (1-2 rounds) | $3-4 | **$20** | **$27** | **$34** | **$30** |
| B (M, cross-owner) | $10-14 | $4-5 | $2-3 | $3-7 (1-2 rounds) | $4-5 | **$23** | **$31** | **$42** | **$35** |
| **Total** | | | | | | **$52** | **$71** | **$95** | **$81** |

Reasoning:
- **A1**: one check, one function, one discharge line, one fixture regeneration, about 5 negatives and 5
  mutants. Roughly half of S1. One remediation round expected. The high case adds a second round triggered by
  test-world fallout.
- **A2**: comparable to S1 in scope (a new record kind, a removed request field, 5 constructor sites, 12 binding
  sites, about 10 negatives). S1's actual $26.6 with two remediation rounds is the expected case. The high case
  has a second Fable round at the $4.46 rate and a larger root gate.
- **B**: three crates and two owners (slice-campaign and the org-side consumer), a new `Discharge` variant
  touching an S-B invariant, and the open Q3.2. That is costlier than S1. The high case covers a new crate under D8 and
  a second Fable round. Q3.2 no longer threatens a candidate-shape change.
- **Caps**: set at about expected + 15-20%, so one extra remediation and recheck fits without fitting an
  uncontrolled overrun. Each builder should have a **sub-cap that leaves room for its report**: A1 $7, A2 $13,
  B $14. S1's builder stopped at $8.98 without a report, which wasted part of that spend. The cap-stop watcher
  must confirm the stop (operator, 2026-11-17).
- Not included: a second planning or design pass, live/provider runs (none needed), and any production (L)
  work for item 4.

## 7. Operator decisions needed before implementation

1. **O-1: where owner text is recorded.** Contract 3.1 (item 1's `scope.digest` definition) and the S-A record
   contract (rev6 §18 / 6.1, item 4's `OrganizationAdmission v1`) are not in the repository. Either authorize
   recording the definitions as crate doc comments plus a records entry (recommended), or name the external
   document and owner who must amend it. A1 and A2 both need this.
2. **O-2: D8 placement (blocks B).** Choose between (a) review-preparation → slice-campaign directly, behind
   preparation's `fixture` feature, which adds rusqlite, rustix and the rest to preparation, delivery and hosted
   builds; (b) a small campaign-owned crate holding `CampaignIdentity`, `episode_id`, `episode_within_scope` and
   `expected_admission`, re-exported by slice-campaign (recommended: light, one rule, acyclic); (c) another
   placement the slice-campaign owner names. The owner of slice-campaign must sign off on extracting its private
   rule.
3. **O-3: budget and caps.** Authorize the per-slice caps (A1 $16, A2 $30, B $35; total $81) and the builder
   sub-caps, or a smaller first tranche (A1 + A2 = $46).
4. **O-4: schedule.** A1 ∥ A2 now, with B in parallel once O-2 is decided (fastest; more rebases). The
   alternative is B after A1 merges (fewer rebases).
5. **O-5: expectation on hosted standing.** Acknowledge that completing items 1-4 leaves
   `work-engine-review-hosted` at `owner_pending` / `none_requested` (§0.3). Moving it needs D7 and a hosted
   admission route, which is a separate decision.
6. Recorded for the slice reviewers, not for the operator: Q2.2 (a new `HostChecked` discharge variant that
   amends the S-B "ids alone" invariant), Q3.2 (the public class inverse in `work-engine-organization`; no 1A shape
   change), and item 4's `CheckId` placement.

## 8. Limits

- Read at the cited lines only, n = 1 per site. Nothing was compiled or run. Test counts are `#[test]` greps.
- U items: the claim-evidence schema line; `ad20341a…` = `episode_digest(...)[..32]`; `18108e56…` = the
  digest of the test subject; which slice-campaign
  tests cover `contract.rs:368-382`; the hosted r1/r2 `scope.digest` values; remediation-round episode
  semantics; `authority-store` lines.
- Context-pressure delivery for this session was not established (no listener binding was supplied). The read
  volume stayed small.
