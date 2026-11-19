# A1 review: scope.digest is the exact-subject digest

Reviewer: fresh-entry adversarial code reviewer (Claude Opus 5.5, read-only).
Subject: worktree `/home/bline/cbm-work/wt-org-owners-a1-20261118`, uncommitted diff vs `f772612f`.
`git diff f772612f | sha256sum` = `412c07a3…` = `sha256(a1/a1-1.patch)`, so the patch and the worktree are the same bytes.
Status: COMPLETE.

## Verdict

**accept_with_changes**

The S1 logic, the digest derivation, the delivery discharge, the fixtures and the tests are correct. I found no P1. One P2 needs fixing before merge, and the fix is records-only: the tree keeps a hosted rule that contradicts the new owner definition, and nothing records it. The P3s are optional.

## Findings

### P2-1: the hosted crate still enforces the superseded rule `scope.digest == patchIdentity`, and nothing records the contradiction

- Evidence:
  - `rust/crates/work-engine-review-hosted/src/subject.rs:189-194`: `materialize` refuses with `ScopeDigestMismatch` unless `scope_digest == patch_identity`. The error text is at `src/lib.rs:77`.
  - `src/check.rs:604-607`: check `subject_bound_to_scope` passes only when `scope_digest == patchIdentity`.
  - `CLAIMS.md:42-45` claims that binding. Hosted test `tests/subject.rs:91` pins it, and `fixtures/hosted-route-qualification-r{1,2}/DERIVATION.md` use the stand-in.
  - The A1 owner definition says the opposite: `rust/crates/work-engine-organization-authority/src/gate.rs:240-246`, "`scope.digest` is the exact-subject digest".
  - After merge, the two rules exclude each other. Every D1-conforming requirements world is refused by hosted `materialize`, and every hosted world is refused by S1.
- What the records say now:
  - `RESULT.md` "Explicitly unclaimed" says only that hosted's stand-in digests "were not parsed (hosted never runs S-A)".
  - PLAN §1 R1.3 calls hosted "unaffected".
  - Both statements are true for S-A, but they leave out that hosted actively enforces the contrary rule. This is an unresolved condition that must stay visible (core §16 Truth).
- Fix: records only. O-5 forbids changing hosted. Add a "Known contradiction" item to the records entry (RESULT.md, "Owner definitions" or "Explicitly unclaimed") that:
  - names the lines above;
  - states the consequence;
  - states that moving hosted's scope binding to the exact-subject digest (`SubjectRef::scope_digest` or `review_episode_core::codec::digest(subject)`) is a precondition of any hosted admission route (D7).

  The root should raise it with the operator alongside O-5.

### P3-1: the owner definition cites a records entry with no locatable path

- `gate.rs:240-241` says "recorded here and in the O7 owners records entry".
- That entry is `a1/RESULT.md`, under `/home/bline/cbm-work/…`, outside the repository. No `records/` file is in the diff.
- Fix: when the root lands the entry under `records/`, cite that repo path in the doc comment.

### P3-2: the owner-definition wording could be tighter

- `gate.rs:244` says "canonical JSON". Name the codec instead: `review-episode-js-json-v1` (`review_episode_core::codec::CODEC_ID`). Then the definition does not depend on what a reader takes "canonical JSON" to mean, for example RFC 8785.
- `gate.rs:242` says "the candidate under review". This can be misread as the `OrganizationCandidate`. Prefer "the review subject (the campaign candidate)".

### P3-3: the field owner's crate has no pointer to the definition

- `rust/crates/work-engine-organization/src/schema.rs:211-215` (`Obligation.scope_digest`) has no doc comment.
- A reader of the field owner's crate cannot find the meaning, which lives in S-A.
- Optional fix: add a one-line plain-text doc pointing to `work_engine_organization_authority::SubjectRef::scope_digest`. This touches the 1A crate, which is outside the A1 crate list in plan §5, so it is the root's call.

### P3-4: the preparation blocker basis text is now stale

- `rust/crates/work-engine-review-preparation/src/prepare.rs:534` says "no owner states the rule that relates scope.digest to the review subject". After A1, an owner rule (S1) exists.
- Keeping the blocker unconditional is correct: S-B design, D1.
- Changing the text changes preparation output, and the text also appears in records `e2e/*/contract.json`. So it is a follow-up, not an A1 change.

### P3-5: `evaluate` does not check that nothing evaluated is left unlisted

- `gate.rs:602-609` drains `out` only for `CheckId::ALL`. A check that is evaluated but missing from `ALL` is silently dropped. This is the A1-M5 route.
- A1-M5 is killed by the listing test `s1_the_base_admission_passes_s1_and_lists_every_check` (`admission.rs:1597`). In delivery, `Outcomes::passed(S1)` would also panic on the missing key (`gate.rs:160-162`).
- Optional hardening: after collecting, `assert!(out.is_empty(), "every evaluated check is in CheckId::ALL")`. Then the plan's expected panic becomes real.

### P3-6: one mutant label in the matrix is wrong

- `MATRIX.md` I8 / A1-M7 says "unknown vantage passes".
- Removing the participants guard does not make S1 pass. It turns the reason into `ScopeDigestUnserved` (the empty `served` path), so M7 kills a reason-code change.
- The real pass route removes both guards. By inspection, the same exact-set test kills it, because S1 would be `Passed` where `VantageUnknown` is expected. I did not run it.
- Fix: relabel the mutant.

### P3-7: cosmetic doc wrapping

- `rust/crates/work-engine-review-delivery/tests/native_vertical.rs:17` is a 142-character doc line.
- `rust/crates/work-engine-review-delivery/src/lib.rs:24-26` has a ragged wrap.
- rustfmt does not wrap comments. Rewrap both.

## Answers to the packet questions

1. **Correctness: holds.**
   - `s1` (`gate.rs:1101-1136`) filters participants by `id == request.vantage` and compares the `to_hex()` of every served obligation's `scope_digest` with `SubjectRef::scope_digest()`.
   - Each mismatch gives one reason, and a served id with no obligation also refuses. There is no early exit after the first mismatch.
   - An unknown vantage refuses with `VantageUnknown`; an empty vantage string is unknown. A participant that serves nothing refuses with `ScopeDigestUnserved`.
   - S1 is inserted in both candidate branches (`:538` unavailable, `:557`), and no `return` precedes the `ALL` drain. `admission_state` re-runs `evaluate` (`:1398-1409`), so a regate re-checks S1.
   - Derivation (`gate.rs:248-254`) is `codec::digest(JsValue::object{commit, tree, patchIdentity})`. That is the same object as preparation's `Subject::to_js_value` (`prepare.rs:31-37`) and the same digest the consumer compares with the episode's `subject.sha256` (`fixture.rs:632-644`).
   - The codec sorts keys by UTF-16 code units (`BTreeMap<JsString>`) and emits lowercase `{:x}`. `Digest::from_hex` accepts only lowercase and `to_hex` emits lowercase (`digest.rs:76-101`), so comparing hex strings cannot give a false pass.
   - I recomputed the pinned base-subject digest independently: the literal canonical string in `admission.rs:1582` matches the codec order.
2. **Owner choices: accept all four.**
   - (a) **Fail-closed for unknown or unserved vantages.** Today it does not change admissibility: R2 already refuses any vantage without a vantage requirement, and derivation emits those only for reviewer vantages (`derive.rs:481-488`). It is defence in depth. Without it, S1 would pass vacuously and bind no scope.
   - (b) **Hex comparison.** `Digest` has no public constructor (`from_hex`/`frame` are `pub(crate)`), and both sides are canonical 64-character lowercase strings.
   - (c) **Public `SubjectRef::scope_digest()`.** It is the single owner function. Delivery tests use it, and hosts need it to author conforming requirements.
   - (d) **New dependency S-A → `review-episode-core`.** That crate depends only on `sha2` and `ryu-js` (`review-episode-core/Cargo.toml:8-10`), so there is no cycle. It is pure, which matches S-A's "no I/O" description. It is plan option (a). The same `{ path = … }` form is used in 12 other crates.
3. **The delivery discharge is sound.**
   - `construct_request` (`native.rs:1409-1431`) uses the preparation's vantage, which is `served_by` of the selected obligation (`prepare.rs:400-407`), together with the binding's subject.
   - So S1 passing at that vantage means exactly that the selected obligation's `scope.digest` is H(admitted subject). That is the blocker's proposition (`prepare.rs:529-535`).
   - The consumer then binds the gate-proof subject to the declaration, and the declaration to the episode's `subject.sha256`.
   - `ProblemScopeToEpisodeIdentity` and `ConsumptionEvidenceMapping` stay `AssumedDischarge` (`delivery gate.rs:106-108`), and the module and crate docs say so.
4. **Fixtures: verified.**
   - Python (`-I`): `sha256('{"commit":"96808de3…","patchIdentity":"5212141d…","tree":"9c5d94ba…"}')` = `18108e56e12f8428ce69da714ba7fc10e93990226717e987bf228c269951d9c1`. That equals frozen `episode-authority.json` `initialSubject.sha256`.
   - The assumption world equals the frozen requirements with `realizationConstraints` removed and that `scope.digest` set: semantic equality is `True`.
   - `git diff f772612f --quiet -- …/frozen/` is clean, and there are no untracked files.
   - Frozen-world admissions are named negatives: SA `s1_a_patch_identity_stand_in_scope_digest_refuses_only_s1` and DG `the_frozen_stand_in_scope_digest_refuses_only_s1` (`refused() == [S1]`). No remaining test admits over the stand-in. Preparation moved to `scoped_req`, and the delivery tests use the assumption world, except T3, which refuses before observation.
5. **Tests and mutants: claims are killed.**
   - All 7 mutants ran against the current bytes: mutation `original_sha256` `0c4ad1b8…` / `4825904b…` equal the current files, and the pre- and post-hashes are equal.
   - Each mutant was killed with `rc=101` by an assertion panic in the named test.
   - No test compares a value with itself:
     - I1 pins against an independent sha2 of a literal;
     - NV ties the function to the frozen authority file;
     - the assumption fixture is a stored literal.
   - Routes I checked by reasoning (not run): S1 compares `patchIdentity`, i.e. the stand-in rule; S1 compares every vantage's obligations; the discharge is `Admitted(&[])` or names the wrong check. Each would fail an existing test: the patch-identity negative, I4 and the discharge table respectively.
   - `assert_exact` iterates `CheckId::ALL` (`admission.rs:419-432`), so every pre-existing exact-set test now also asserts that S1 passed.
   - The plan's "first served obligation only" mutant is equivalent, because derivation gives one served obligation per vantage (`derive.rs:589-593`). M3 is a sound substitute. M5 is covered (see P3-5) and M7 is mislabelled (see P3-6).
6. **Preparation test edits (deviation 1): necessary and test-only.**
   - Only `tests/common/mod.rs` and `tests/fixture_consumer.rs` change.
   - `gate::admitted` panics on any S-A refusal (`common/mod.rs:407-408`). Every world built on `REQ` with the frozen subject would now fail S1.
   - In the "other subject" case, the consumer's candidate is deliberately scoped to the other subject, so that only the consumer's subject comparison refuses (`GateProofMismatch{subject}`). The consumer does not, and per S-B should not, re-check the scope, so that synthetic inconsistency is acceptable.
7. **Owner-definition doc comment: accurate and consistent with D1 = (A).**
   - It matches D1's text: the preimage, the codec, one subject per obligation, and a new requirements revision per remediation round.
   - Wording nits: P3-1 and P3-2. Repository consistency: P2-1.

## Could not verify

- I ran no cargo commands. Instead I relied on:
  - the builder's gate (`runs/gate-1`, 13/13);
  - the root gate (`root-gate-1`, `status: passed`; 345 tests passed, 0 failed, over 26 binaries, plus native_vertical 33/33);
  - the mutant records.
- Neither gate tests `work-engine-review-hosted`. By grep it uses neither S-A nor the delivery `discharge`, so I expect no impact.
- I did not execute the double-guard mutant (P3-6) or the other unrun routes in answer 5; they rest on reasoning.
- PLAN itself was not reviewed (out of scope). I did not read contract 3.1, which is not in the repository.
- I edited nothing, staged nothing, and committed nothing. `git diff --cached` is empty. This file is my only write.
