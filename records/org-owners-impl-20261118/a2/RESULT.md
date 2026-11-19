# A2 RESULT: item 4, OrganizationAdmission v1 at fixture standing (S-A)

Status: **implemented, gate green, all 10 mutants killed.** Uncommitted in the worktree; nothing staged.
Builder: Claude Opus 5.5 (`claude-opus-5-5`), worker role. Usage/cost: unknown to me (parent observes budget).
Context-pressure route: not established (no listener binding supplied; packet forbids listeners). Recorded as a gap.

## Entry check

- Worktree `/home/bline/cbm-work/wt-org-owners-a2-20261118`, branch `org-owners-a2-20261118`, HEAD
  `f772612ff8cfc610a5121c6763a71e48e293e446`; `git status --short` empty at entry. HEAD unchanged at exit.
- Read: packet; manual-orchestration `SKILL.md`, `references/worker.md`, `references/call-cost.md`;
  worktree `AGENTS.md`, `docs/contracts/core.md`; PLAN §0, §4, §5, §8 (and §1 lines naming A1's check);
  D1 PROPOSAL §5 and the D4/D5 rows; S1 `mutants/mutate.py`, `gen_plan.py`, `root-gate-plan.json`.
- Plan §8 items relied on: only the code citations in §4, all re-verified by reading the code (they hold at
  `f772612f`, line numbers drift slightly). No §8 "U" item was relied on.

## Design note

**Route.** A new S-A record `OrganizationAdmission` and a new S-A check `CheckId::OA`.

- `records.rs`: `OrganizationAdmission { organization_id, revision, predecessor: Option<IssuanceId>,
  problem_scope: ProblemScopeRef, requirements_digest: Digest, candidate_content_digest: Digest }`,
  `Record::OrganizationAdmission`, `kind() = "organization_admission"`, a `body_digest` encoding, read
  accessors (`organization_id`, `revision`, `predecessor`) and the fixture-only
  `Record::fixture_organization_admission(...)`. `VantageActorBinding.organization_id: String` became
  `organization: IssuanceId` (the admission's issuance); its encoding and `fixture_vantage_binding` changed
  accordingly (first argument is now an `IssuanceId`).
- `gate.rs` (S-A): `OperationRequest` lost `organization_id` (field, `new` parameter, accessor). New
  `oa(...)`: the unique **committed** `OrganizationAdmission` whose candidate content digest, requirements
  digest and problem scope (`ProblemScopeRef::of` the derived candidate) equal the request's. Zero →
  `ReasonCode::OrganizationUnadmitted`, two or more → `ReasonCode::OrganizationHeadsAmbiguous` (subject:
  candidate digest hex). Without a derived candidate OA is `CandidateUnavailable` (like O4/O5/O8). On success
  the admission is pinned in the new `AuthorityPins.organization_admissions` (included in `all()`, so the point
  check gives `Revoked`/`Stale` exactly as for the other pins), and its `IssuanceId` is what O8 and R3 pass to
  `vantage_bindings` (now `Option<IssuanceId>`; `None` yields no bindings). `EffectAdmission::organization()`
  returns the new public value `AdmittedOrganization { admission: IssuanceId, organization_id: String }`.
  O3's doc now says O3 alone is self-consistency and OA relates the request to what the owner committed.
  `CheckId::ALL` is 10 long (OA after O3).
- Delivery: `FixtureAuthority.organization_id` and `CallerAsserted.organization_id` removed;
  `CALLER_ASSERTED_LABEL` now labels `actor_identity` only (doc updated). `AdmissionSummary::organization()`
  carries the S-A-derived `AdmittedOrganization` (unlabelled). `construct_request` no longer passes an org.
  `discharge`: Separation `[OA, O8, R3]`, ComponentBinding `[OA, O4, R3]`, ObserverIdentity `[OA, R2, R3]`
  (each names R3 or O8, which now depend on the derived organization).
- Supersession needs no new machinery (`FixtureSource::supersede`, `IssuanceStatus::Superseded`). Because
  bindings pin the admission's issuance, a successor admission needs re-issued bindings (tested, I7).
- Production: no production constructor exists for any record, so production states hold no admission; the
  production policy still refuses (O4/O5/O8/R3 unchanged). OA itself is a record check and is not
  domain-specific (it passes in the existing production test because that test's state holds fixture records,
  exactly as O4's authority half already does).

**Public API added/changed** (S-A): `CheckId::OA`; `ReasonCode::{OrganizationUnadmitted,
OrganizationHeadsAmbiguous}`; `records::OrganizationAdmission` (+ accessors), `Record::OrganizationAdmission`,
`Record::fixture_organization_admission`; `AdmittedOrganization` (re-exported); `EffectAdmission::organization()`;
`AuthorityPins.organization_admissions`. Removed: `OperationRequest::organization_id()`, the `new` parameter.
Changed: `Record::fixture_vantage_binding(organization: IssuanceId, ...)`. Delivery: `AdmissionSummary::organization()`
added; `CallerAsserted.organization_id`, `FixtureAuthority.organization_id` removed.
**No dependency edge added** (S-A already depends on `work-engine-organization`, which owns `Digest`).
`Cargo.lock` unchanged.

**Rejected alternatives.**
- OA inside O3: rejected (plan recommendation) because a distinct check gives a distinct reason and mutant
  target, and keeps O3's meaning stable.
- Naming the check `O1`/`O2`/`O6`/`O7`: rejected because those Gate O numbers belong to the external Stage 0
  rev6 text, which is not in the repository; `OA` cannot collide with it or with A1's `S1`.
- Special-casing R3/O8 to report `OrganizationUnadmitted` when OA fails: rejected; the existing lookups already
  refuse (`SeparationUnresolved`, `VantageUnbound`) and the exact sets in tests pin that cascade.
- Gate-side validation of the predecessor chain (revision+1, predecessor superseded by this issuance): not
  implemented (see Unfinished); not in the plan's outcomes or negatives.

## Owner definitions

Recorded as the doc comment of `pub struct OrganizationAdmission` in
`rust/crates/work-engine-organization-authority/src/records.rs` (this section is the records entry under O-1).
Exact text:

```text
/// `OrganizationAdmission v1 { organizationId, revision, predecessor?: IssuanceId, problemScope,
/// requirementsDigest, candidateContentDigest, issuer: deployment_owner }`.
///
/// Owner: organizational authority (S-A). Operator decisions O1 and D4/D5 (2026-11-17); definition
/// recorded here under O-1 (2026-11-18). It adds one kind to the S-A record set (Stage 0 rev6 §18
/// with Amendment 6.1).
///
/// - `organizationId` names the admitted organization: one problem's organization (the execution
///   envelope identity), stable across revisions. It is not a tenant or an installation.
/// - A committed issuance admits exactly one candidate: the one whose content digest is
///   `candidateContentDigest`, derived from the requirements whose digest is `requirementsDigest`,
///   for `problemScope`. Like every record it confers nothing by its content; it counts only while
///   an observation reports its issuance committed, and it is revocable (6.1 F5).
/// - Revision 1 has no predecessor (create-only). A successor keeps `organizationId`, has the next
///   revision, names its predecessor's issuance and is issued by superseding that predecessor, so
///   one organization has at most one committed head (no branching).
/// - Vantage-actor bindings name the admission they belong to by its issuance
///   ([`VantageActorBinding`] `organization`). Revoking or superseding an admission therefore
///   leaves every binding that pins it without an admitted organization.
/// - The gate takes the organization of a request from the unique committed admission whose
///   candidate digest, requirements digest and problem scope equal the request's; no caller
///   supplies it. None refuses `OrganizationUnadmitted`, more than one refuses
///   `OrganizationHeadsAmbiguous` (check `OA`).
/// - Issuer: the deployment owner. Fixture standing only: the constructor exists only with the
///   `fixture` feature. Production issuance needs observation-source storage (O6), an independent
///   issuer (K-c) and production authorization.
```

Companion doc text (not owner definitions, but the same rule where it is enforced): `CheckId::OA` and
`CheckId::O3` docs in S-A `gate.rs`; `VantageActorBinding` doc (`organization` is the admission's issuance).

## Files (all under `rust/crates/`)

| File | Reason |
|---|---|
| `work-engine-organization-authority/src/records.rs` | new record + owner definition; binding field → `IssuanceId`; encoding; fixture constructors |
| `work-engine-organization-authority/src/gate.rs` | `CheckId::OA`, `ALL`, reason codes, `OperationRequest` without org, pins, `AdmittedOrganization`, `oa()`, O8/R3/`vantage_bindings` take the derived admission |
| `work-engine-organization-authority/src/lib.rs` | module docs; re-export `AdmittedOrganization`; `compile_fail,E0061` doctest for `OperationRequest::new` with an org |
| `work-engine-organization-authority/Cargo.toml` | description string lists OA (no dependency change) |
| `work-engine-organization-authority/tests/admission.rs` | world issues an admission first and binds to it (`Terms`, `admission_record`); 3 existing tests admit their own request's terms; 2 expected sets extended; 6 new OA/state tests; positive asserts `organization()` and the pin |
| `work-engine-review-delivery/src/gate.rs` | discharge names OA; `CallerAsserted` without org; summary carries `organization()`; docs |
| `work-engine-review-delivery/src/native.rs` | `FixtureAuthority` without org; `construct_request`; docs |
| `work-engine-review-delivery/tests/common/mod.rs` | world issues the admission first (`organization_admission`), `Ids.organization`, bindings rebound |
| `work-engine-review-delivery/tests/gate.rs` | discharge table; superseding binding names the admission |
| `work-engine-review-delivery/tests/native_vertical.rs` | summary asserts the derived org; binding rebinds; new unadmitted-before-spawn control |
| `work-engine-review-preparation/tests/common/mod.rs` | world issues the admission of the request's candidate first; bindings rebound |

Diff stat vs `f772612f`: 11 files changed, 614 insertions(+), 140 deletions(-). `work-engine-review-hosted`
untouched (O-5).

## Runs (custody)

- `runs/gate-1/summary.json`: 14/15. The failing step was an **extra** step I added,
  `cargo clippy -p work-engine-review-hosted --all-targets -D warnings`; it failed on three pre-existing
  `clippy::type_complexity` lints in `work-engine-review-hosted/tests/check.rs` (untouched file). Replaced by
  `cargo check -p work-engine-review-hosted --all-targets` in gate-2 (compile proof only).
- `runs/gate-2/summary.json` (sha256 `1dfa9639ac65754665e6f59e5b3d1478162d00fc47c9ab166e678d7de4315288`):
  **15/15 passed** — toolchain; fmt (5 crates); clippy fixture / default / test-support feature sets and
  preparation-fixture lib alone (`-D warnings`); `test-four-plus-sa` 311 passed; `test-preparation-fixture` 42;
  `test-default-features` 92; hosted `cargo check --all-targets`; S-A `--doc` default features 8 (includes the new
  compile_fail); authority-store `--test store` 37 (not touched; kept as S1 did); `native-1` 33; hashes; git status.
  Plan: `gate-plan.json` (S1 root-gate plan, cwd this worktree's `rust`, own `CARGO_TARGET_DIR`, same `WE_PI_SDK_*`).
- `mutants/runs-1/summary.json` (sha256 `b550b7eba16ac6031dbd2bfe56c0e93bd8b8cf57699dbf90c23a4864ce8f1336`):
  10 mutants, each apply → mapped test(s) fail with `test result: FAILED` (no compile-error kills) → exact restore.
  Pre-hashes = post-hashes = gate-2 hashes for the two mutated files. Scripts: `mutants/mutate.py`
  (adapted from S1), `mutants/gen_plan.py`, `mutants/mutant-plan-1.json`.

## Matrix summary

See `MATRIX.md`. 11 invariants; 10 hand mutants, all killed: first-of-two-heads, skip-pin, ignore-scope,
ignore-candidate, ignore-requirements, bindings-by-vantage-only (O8 and R3 share `vantage_bindings`),
point-check-ignores-organization-pins, revoked-admission-counts, OA-dropped-from-ALL, Separation-discharge-drops-OA.
11 negative controls (revoked before; other candidate / requirements / scope; two heads; other organization's
bindings; revoked after → `Revoked`; superseded after → `Stale`; successor without rebound bindings; delivery
unadmitted before spawn; no-candidate OA unavailable), each with a positive control.

## Deviations from the plan

1. Check name `OA` (plan left the id open). Discharge also adds OA to **ComponentBinding** (plan: "at least"
   Separation and ObserverIdentity), because ComponentBinding names R3, which now selects by the derived org.
2. Test worlds: the plan said "issue one admission in each world". The authority tests whose request is not
   the default one (`g1_o3_…`, both `g1_o5`/`g2_o5`, `g1_o4_decision_scope_boundaries_…`) now admit their own
   request's terms, so their isolation (only O3 / only O5 / only O4 refuse) is preserved. Two expected sets grew
   by the honest cascade: `g2_o3_no_derived_candidate…` gains OA `CandidateUnavailable` and R3 `VantageUnbound`.
3. Plan Change 5 mentions the `AdmissionSummary` gaining the org's `IssuanceId`/`organization_id`: implemented as
   one `AdmittedOrganization` value shared by S-A and delivery (one spelling for the concept).
4. The `compile_fail` doctest the plan called optional was added (cheap; E0061 arity).
5. Plan says 12 `fixture_vantage_binding` call sites; at `f772612f` there are 11 outside `src/` (3 + 2 + 1 + 2 + 3),
   all updated; the new tests add 4. No production caller of `OperationRequest::new` outside review-delivery,
   as the plan said.

## Unfinished

- Gate-side predecessor-chain validation is not implemented: OA does not check that a revision >1 names a
  predecessor superseded by it, that `organizationId` is unchanged, or that revision = predecessor + 1. The
  record carries the fields and the owner definition states the rule; the fixture issuer is trusted to follow it.
  Uniqueness of the committed head is enforced (I2).
- Not rebased on A1 (A1 merges first). Expected conflicts are mechanical: S-A `CheckId` enum/`ALL`
  (`[CheckId; 10]` vs A1's `S1`: becomes 11), `evaluate` insert sites, delivery `discharge` lines, delivery
  `tests/common/mod.rs` (A1 regenerates the assumption world; `organization_admission()` derives its digests from
  `operation_request()`, so it follows a regenerated world automatically).
- No independent review yet (review gate required by the parent).

## Explicitly unclaimed

- Fixture standing only. No production record constructor, storage (O6), K-c issuer or production admission
  authorization exists; the production policy still refuses every admission.
- `work-engine-review-hosted` stays `owner_pending` / `none_requested` / `consumable: false` (untouched; O-5).
- Items 1–3 (scope digest, episode-within-scope, consumption-evidence mapping) untouched; the three mapping
  kinds remain `AssumedDischarge`.
- `work-engine-authority-store`'s `candidate_digest` "organization identity" untouched (N8).
- `actor_identity` remains caller-asserted (`CALLER_ASSERTED_LABEL`).

## Known gap: admission chain (added by remediation-1, review P2-1)

At fixture standing the gate does **not** check an `OrganizationAdmission`'s predecessor chain
(revision = predecessor + 1, same `organizationId`, predecessor superseded by this issuance) or
per-organization uniqueness of the committed head. The owner definition's "at most one committed head
(no branching)" is an obligation on the issuer, not a gate guarantee. `OA` refuses only when more than
one committed admission matches the same candidate terms (candidate digest, requirements digest, problem
scope), and those may belong to different organizations. A revision-2 admission issued without superseding
its predecessor, an unrelated or non-admission `predecessor`, another `organizationId`, a non-consecutive
revision, or a second revision-1 admission for an existing `organizationId` are all admitted exactly like
a valid one. An issuer outside fixture standing requires that check first, alongside O6 and K-c. The
`OrganizationAdmission` doc comment in `records.rs` now says the same.

Owner question (settle before enforcing): admissions are keyed by problem scope, which includes the slice
number, so one head per organization would block concurrent slices of one organization. Whether the
head is per organization, per problem scope, or per candidate terms is not decided; only the chain check
(revision +1, same organization, predecessor superseded by this issuance) is uncontroversial.
