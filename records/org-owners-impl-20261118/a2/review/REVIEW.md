# A2 review: OrganizationAdmission at fixture standing

Status: complete.

Reviewer: fresh-entry adversarial code reviewer (Claude Opus 5.5, `claude-opus-5-5`), read-only.
Subject: worktree `/home/bline/cbm-work/wt-org-owners-a2-20261118`, uncommitted diff against `f772612f`.
Authority read: `AGENTS.md`, `docs/contracts/core.md`, PLAN §0, §4, §5 and §8, D1 PROPOSAL (the D4/D5 rows
and the item-4 section, lines 215-260 and 288-289), and the builder's RESULT.md and MATRIX.md.

## Verdict

**accept_with_changes**: one P2 finding (owner-definition truthfulness about the predecessor chain) and two
optional P3s. No P1. The core claim holds. The organization comes only from the unique committed
`OrganizationAdmission` that matches the candidate digest, the requirements digest and the derived problem scope.
No caller-supplied organization reaches admission, binding selection or delivery.

## Custody checks (verified)

- `git diff f772612f | sha256sum` gives `0174eabe…`, equal to `a2-1.patch`. Nothing is staged
  (`git diff --cached` is empty). There are no untracked files, and HEAD is still `f772612f`.
- Current S-A `gate.rs` has sha256 `759a1de6…` and delivery `gate.rs` has `f2bb9696…`. These equal the
  `original_sha256` in every `mutants/*/mutation.json`, so the mutants were applied to the reviewed bytes. Their
  git hash-objects (`3958e386…`, `7d52f4cc…`) and the other changed files' hashes equal `runs/gate-2/hashes.stdout`
  and `root-gate-1/hashes.stdout`.
- `runs/gate-2/summary.json` is `1dfa9639…` and `mutants/runs-1/summary.json` is `b550b7eb…`, as RESULT states.
  Gate-2 passed 15/15, with 311 passed in `test-four-plus-sa`. The root's `root-gate-1` ran on the clean export
  `wt-a2-export-1` with its own target and also passed 15/15 (native-1 33 passed; S-A doc 8 passed).
- Mutants: all 12 test steps exit 101 with `test result: FAILED. 0 passed; 1 failed`, and the named test
  panicked each time. There are no `error[` compile kills. The summary's overall `"status": "failed"` is expected,
  because the test steps are meant to fail. Pre-hashes equal post-hashes.

## Findings

### P2-1: the predecessor chain is an unchecked issuer obligation, but the owner definition reads as a gate guarantee

- Where: `rust/crates/work-engine-organization-authority/src/records.rs:110-112` (owner definition, chain
  bullet) and `src/gate.rs:685-722` (`oa`).
- Evidence:
  - `oa` reads an admission's status and its three terms only. It never reads `revision`, `predecessor` or
    `organization_id` to validate them, and nothing else in the crate does either.
  - Uniqueness is per candidate terms, not per organization. The builder's own test shows this:
    `g2_oa_two_committed_admissions_of_the_candidate_are_ambiguous_heads` (`tests/admission.rs:1705`) makes its
    "two heads" with two *different* organizations (`org-1`, `org-2`).
- At fixture standing, OA therefore admits each of the following exactly as it admits a valid admission:
  1. A revision-2 admission issued with `issue` instead of `supersede`. Its predecessor stays committed, and if
     the two admit different candidates, both candidates stay admissible under one `organizationId`. That is a
     branch the gate cannot see.
  2. A "successor" that names an unrelated issuance or a non-admission as `predecessor`, carries another
     `organizationId`, or has a non-consecutive revision.
  3. A second revision-1 admission for an existing `organizationId`.
- How it reads: the bullet ends "so one organization has at most one committed head (no branching)".
  `OrganizationHeadsAmbiguous` also suggests head detection. The approved D1 derivation says the gate resolves
  the unique committed, unrevoked *head*. A reader of the committed O-1 owner record would conclude that the gate
  enforces this. The gap appears only in RESULT.md's Unfinished section, which is outside the repository.
- Answer to packet Q4: trusting the fixture issuer is acceptable at fixture standing.
  - It does not weaken A2's claims: the organization is record-derived and never caller-supplied, and only
    `fixture`-feature code can issue records.
  - The plan's §4 outcomes and negatives did not require chain checks.
  - Gate-side enforcement is not a P1 and can wait until a non-fixture issuer exists.
  - It is a P2 only because the merged owner definition must not imply enforcement that does not exist (Truth).
- Fix (minimum, required before merge):
  - Add to the records.rs:110-112 bullet a sentence such as: "At fixture standing the gate does not check this
    chain; it relies on the issuer. `OA` refuses only when more than one committed admission matches the same
    candidate terms, which may belong to different organizations."
  - Carry the gap into the committed O-1 records entry as a prerequisite for any non-fixture issuer, alongside
    O6 and K-c.
- Optional upgrade (if the root prefers enforcement now): after selecting the admission in `oa`, check that:
  - `revision == 1` exactly when `predecessor` is `None`;
  - for `revision > 1`, `state.resolve(predecessor)` is `Superseded { by: <this id> }`;
  - the predecessor's record is an `OrganizationAdmission` with the same `organization_id` and
    `revision - 1`.

  Refuse otherwise with a new reason (for example `OrganizationChainInvalid`), and add three negatives and one
  mutant. Do **not** add per-organization head uniqueness across candidates without an owner decision (see the
  open question below).

### P3-1 (optional): no test covers ambiguity introduced at regate time

- Where: `src/gate.rs:1401-1412`.
- Evidence: suppose a second committed admission of the same candidate is issued after admission. The point check
  passes, because the pinned admission is still committed. The re-evaluation then refuses OA
  `OrganizationHeadsAmbiguous`, and O8/R3 cascade, so the result is `AdmissionState::Refused`. That is correct by
  reading, but no test pins it. Revocation and supersession at regate are pinned (I6, I7).
- Fix: add `state_refused_when_a_second_admission_of_the_candidate_is_committed_after_admission`. It should
  assert `Refused(outcomes)` with `outcomes.refused() == [OA, O8, R3]`.

### P3-2 (optional): OA matches the caller's claimed digest, not the derived candidate's

- Where: `src/gate.rs:697` (`a.candidate_content_digest == request.candidate_digest`).
- Evidence:
  - The two values are equal whenever O3 passes, and `admit` requires every check to pass. No admitted request
    can differ.
  - When O3 refuses with `CandidateDigestMismatch`, OA, O8 and R3 are computed against an admission of the
    *claimed* digest paired with the *derived* candidate's scope.
  - `g1_o3_candidate_digest_of_another_candidate_refuses_only_o3` (`tests/admission.rs:630`) admits the claimed
    terms so that only O3 refuses.
  - This is what the plan specified ("candidate digest from the request").
- Fix (optional): use `c.content_digest()` in `oa`, so OA's outcome does not depend on O3. Have that test admit
  the derived candidate's terms.

## What I checked and found sound

1. **OA correctness.** `oa` collects the committed `Record::OrganizationAdmission` entries whose three terms
   equal the request's: the candidate digest, `request.requirements_digest()`, and `ProblemScopeRef::of` the
   derived candidate. `[one]` pins and returns `AdmittedOrganization`. `[]` gives `OrganizationUnadmitted`, and
   two or more give `OrganizationHeadsAmbiguous`. There is no self-comparison.
   - When there is no derived candidate, OA is `CandidateUnavailable` (it is in the `None`-branch list).
   - No short circuit: O4, O5, O8 and R4 are still evaluated, and `out.remove(..).expect` would panic otherwise.
   - `admit` builds an `EffectAdmission` only on `(true, Some(organization), Some(acting), Some(profile))`. So even
     with OA dropped from `ALL`, a missing organization refuses, and M9 is still killed through the exact outcome
     set.
   - Caller routes: `OperationRequest`, `FixtureAuthority` and `CallerAsserted` no longer carry an organization.
     Repository-wide, `organization_id` now appears only in records.rs, `AdmittedOrganization`, and test
     fixtures. `EffectAdmission.organization` is set only in `admit`, and `AdmissionSummary.organization` only in
     `assess`, from `admission.organization()`. `AdmittedOrganization` is publicly constructible but confers
     nothing, as its doc says.
2. **Pins and lifecycle.**
   - `AuthorityPins.organization_admissions` is first in `all()`. The point check maps a revoked pin to `Revoked`
     and a superseded pin to `Stale(IssuanceSuperseded)`. I6 and I7 test this, and M7 and M2 are killed.
   - At admission time, a revoked admission is unadmitted (I4), and after supersession the successor is selected
     (I7, second half).
   - At regate, the re-evaluation re-derives OA. If the regate passes, it must have selected the pinned
     admission, because the pinned admission is committed, still matches, and is unique. A different
     organization therefore cannot appear at regate.
   - Delivery takes the delivery and consumption admissions before the spawn regate. Any change to the admission
     between the two observations makes the delivery regate fail, so the two summaries cannot silently name
     different organizations.
3. **Bindings.** `vantage_bindings` filters on `Some(b.organization) == organization`, and `None` yields none.
   O8 and R3 both use it, so M6 covers both.
   - I5 tests that bindings pinned to another admission (another scope's, under another `organizationId`) are
     not accepted. I7 tests that a predecessor's bindings do not carry over to the successor.
   - There is no cross-organization acceptance. Principals and acceptor lookup stay global, as before A2 (out of
     scope).
   - The binding encoding is fixed-width (`bytes(organization)`). The admission's `body_digest` encodes every
     field, with an `Option` tag consistent with `DecisionScope`.
4. **Delivery.** `discharge` names OA for Separation, ComponentBinding and ObserverIdentity, and the
   discharge-table test pins all three (M10).
   - `CALLER_ASSERTED_LABEL` now names `actor_identity` only. That is truthful: O8 never used the actor identity,
     and R3 compares it.
   - The `FixtureAuthority` input inventory (`native.rs:1367-1386`) states that no organization is an input.
   - `AdmissionSummary::organization()` is unlabelled, and so is every value that S-A derived from fixture
     records. The summary's domain is `Fixture` (`FIXTURE_ADMISSION_LABEL`).
5. **Tests and mutants.** Each of the 11 negatives maps to its claim and has a positive control.
   - The three-term test varies one term at a time, so M3, M4 and M5 are each killed independently.
   - I also considered these mutants by reading, and each would be killed:
     - OA reporting pass on failure: the exact sets include OA.
     - Treating `Superseded` as committed: I7 expects OA to pass after supersession, but two matches would refuse.
     - OA missing from the no-candidate list: the panic is hit in `g2_o3_no_derived…`.
     - Passing `None` to O8 or R3: the positive tests fail.
   - The updated isolation tests (O3, O5, O4 boundaries) now admit their own request's terms. That is legitimate:
     it keeps the claim that only one check refuses.
   - The `g2_o3_no_derived…` cascade (OA unavailable, R3 `VantageUnbound`) is honest.
6. **Owner definition (O-1).** It is consistent with D4 (the envelope's organization, stable, not a tenant) and
   D5 (a new S-A kind, organization derived, request field removed, fixture first). The gate bullet describes
   `oa` exactly, and the fixture-only issuer statement is true (`fixture_constructors` is
   `#[cfg(feature = "fixture")]`). The exception is the chain bullet (P2-1).
7. **Scope.** `work-engine-review-hosted` is untouched. The hosted crate builds in both gates, and nothing in it
   references the removed fields. No non-fixture record constructor exists. The production policy is unchanged
   and still refuses on O4, O5, O8 and R3.
   - OA is domain-neutral like O4's authority half. Production states cannot hold records, because records can
     only be constructed under `fixture`.
   - `Cargo.lock` is unchanged, and no dependency edge was added.

## Rebase onto A1 (judged on `f772612f`)

I read A1's worktree diff read-only. A1 does not change `OperationRequest::new`'s arity, so the `compile_fail,E0061`
doctest stays valid.

Expected conflicts are mechanical:
- `CheckId::ALL` (`[CheckId; 10]` on both sides becomes 11);
- both `evaluate` branches;
- the `discharge` table;
- the exact refusal sets, which gain S1 only if a base world fails S1. That would show up as a test failure,
  not silently.

A2's test admissions derive their terms from the request: `Terms::of`, delivery `organization_admission()` from
`operation_request()`, and preparation from the constructed request. `ProblemScopeRef` has no digest field, so
A1's regenerated `scope.digest` and candidate digests flow through automatically.

After the rebase, the mutant edit strings need regenerating (M9 embeds `[CheckId; 10]`), and the mutants need
re-running. I found nothing that makes the rebase semantically unsafe.

## Open question (owner, not a finding against A2)

D4 defines one organization per problem, stable across revisions, with no branching. But an admission is keyed by
`problemScope`, which includes `slice_number`. If two slices of one problem must be admitted at the same time,
"at most one committed head per organization" forbids it. The owner should settle this before anyone adds
per-organization head uniqueness to the gate (the P2-1 optional upgrade deliberately excludes it).

## Not verified

- I did not run `cargo` myself. I relied on `runs/gate-2` and the root's `root-gate-1`, whose file hashes equal
  the reviewed worktree, and on the mutant records.
- I did not verify RESULT's call-site counts (11 + 4) beyond grep presence, or every gate-2 step's stdout beyond
  the summaries and the `test-four-plus-sa` count.
- The Stage 0 rev6 §18 / Amendment 6.1 text is not in the repository, so I could not check the doc's citation of it.
- A1 was read only far enough to judge the rebase.
