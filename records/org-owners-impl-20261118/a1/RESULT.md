# A1 RESULT: item 1, scope.digest is the exact-subject digest (S-A check S1)

Builder: Claude Opus 5.5 (`claude-opus-5-5`), worker subagent. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`. Packet: `a1/PACKET.md`. Status: **implemented, gate green, mutants all killed; not reviewed.** No commits, staging or mutating git commands. Context-pressure listener: none established. The packet says "no listeners", and no binding was supplied. The read volume stayed moderate.

## Entry check
- Worktree `/home/bline/cbm-work/wt-org-owners-a1-20261118`, branch `org-owners-a1-20261118`, HEAD `f772612ff8cf…`. `git status --short` was empty at entry. Claim file `a1/wt-claim.txt` (advisory) matches.
- Read: manual-orchestration `references/worker.md` and `call-cost.md`, AGENTS.md, and core.md (headings). PLAN §0, §1, §5 and §8. The S1 root-gate plan and `mutate.py`.
- Verified plan U item: `18108e56e12f8428ce69da714ba7fc10e93990226717e987bf228c269951d9c1` = codec digest of the frozen subject = frozen `episode-authority.json` `initialSubject.sha256` (recomputed in Python and asserted in NV).

## Design note
- **Route.** S-A check `CheckId::S1` sits in `work-engine-organization-authority/src/gate.rs`. `evaluate()` inserts it next to O5 when a candidate exists, and as `CandidateUnavailable` (with O4/O5/O8/R1/R2) when none does. It is never short-circuited, and `admission_state` re-evaluation covers it through `CheckId::ALL` (now 10). The check finds the participants whose id is the request vantage and compares the `scope.digest` hex of every obligation they serve with `SubjectRef::scope_digest()`:
  - one mismatch refuses with `ScopeDigestMismatch` (subject `"<obligationId>: <scopeHex>"`);
  - an unknown vantage refuses with `VantageUnknown`;
  - a vantage that serves nothing refuses with the new `ScopeDigestUnserved`. This fails closed: otherwise S1 would pass vacuously and the subject would bind to no scope.

  Delivery: `discharge(ScopeDigestToSubject) = Admitted(&[CheckId::S1])`. The other two mapping kinds stay `AssumedDischarge`.
- **Public API added.**
  - `CheckId::S1`; `CheckId::ALL: [CheckId; 10]`.
  - `ReasonCode::ScopeDigestMismatch` and `ReasonCode::ScopeDigestUnserved`.
  - `SubjectRef::scope_digest(&self) -> String` (lowercase hex).
- **Dependency edge added.** `work-engine-organization-authority → review-episode-core`: the codec owner, with only sha2 and ryu-js. It is acyclic and already depended on by delivery and preparation. `Cargo.lock` gains one line.
- **Rejected alternatives.**
  - (b) The function in review-episode-core, with S-A converting: no benefit, because the codec is already public.
  - A second canonicalizer in work-engine-organization: two codecs for one subject could drift.
  - Returning `Digest`: `Digest::from_hex` is `pub(crate)` in work-engine-organization. Making it public would widen that crate, so S1 compares `scope_digest.to_hex()` with the hex string instead.

## Owner definitions (records entry, operator decision O-1)
Recorded as the doc comment on `SubjectRef::scope_digest` in `rust/crates/work-engine-organization-authority/src/gate.rs`. `CheckId::S1`'s doc points to it. Exact text:

> The exact-subject digest: the value `obligations[].scope.digest` must carry for this subject.
>
> Owner definition (contract 3.1, field owner `work-engine-organization`; recorded here and in the O7 owners records entry, operator decision O-1, 2026-11-18): `scope.digest` is the exact-subject digest of the candidate under review, `review_episode_core::codec::digest({"commit", "tree", "patchIdentity"})`, that is the SHA-256 of the canonical JSON of the review subject object, as 64 lowercase hexadecimal characters. It is the digest the review episode uses for its subject (`subject.sha256`), so one obligation names exactly one subject; a new subject (for example a remediation round) needs a new requirements revision, hence a new candidate and admission (D1 = (A)).

`CheckId::S1` doc: "The subject is the scope: every obligation the vantage serves has a `scope.digest` equal to the request subject's exact-subject digest ([`SubjectRef::scope_digest`]). A vantage the candidate does not have, or one that serves no obligation, refuses."

## Files (paths under `rust/crates/`)
- `work-engine-organization-authority/src/gate.rs`: S1, the reason codes, `SubjectRef::scope_digest` with the owner definition, `s1()`, and its insertion in `evaluate`. Module doc updated.
- `work-engine-organization-authority/{Cargo.toml,src/lib.rs}`: dependency edge and crate description/doc mention S1.
- `work-engine-organization-authority/tests/admission.rs`: `Req.scope_digest` defaults to the base subject's digest, via the `subject()`/`parse`/`requirements_doc` helpers. Two existing exact sets are updated (unknown vantage, no candidate). Seven new `s1_*` tests.
- `work-engine-review-delivery/src/{gate.rs,lib.rs}`: the discharge arm and docs.
- `work-engine-review-delivery/tests/fixtures/assumption-world/requirements.json`: one value, `scope.digest` `5212141d…` → `18108e56…`. `DERIVATION.md` gives the derivation and the S1 note.
- `work-engine-review-delivery/tests/gate.rs`:
  - the discharge table;
  - the screen test (scope-digest is admitted by S1);
  - new `the_frozen_stand_in_scope_digest_refuses_only_s1`.
- `work-engine-review-delivery/tests/native_vertical.rs`: the derivation test also rewrites `scope.digest` and asserts that it equals `initialSubject.sha256` and that the frozen file keeps the stand-in. The S-B path asserts the scope-digest blocker is admitted by `[S1]` and never assumed. Module docs updated.
- `work-engine-review-preparation/tests/common/mod.rs` and `fixture_consumer.rs` (test-only; **deviation**, see below):
  - `scoped_req`, `scoped_req_for` and `scoped_edited` (the frozen REQ with the codec-digest scope);
  - the consumer world, `Ask` default, the edited worlds and the second obligation's scope use them;
  - the "other subject" case admits over requirements scoped to the other subject, with the consumer candidate matching, so the consumer's subject comparison still refuses.
- Unchanged: the frozen `work-engine-organization/tests/fixtures/frozen/*` (byte-identical) and `work-engine-review-hosted`.

## Diff stat vs `f772612f`
13 files, +459 / −73 (Cargo.lock +1; S-A gate.rs +81; admission.rs +250; delivery gate.rs 15; lib.rs 5; DERIVATION.md 20; requirements.json 2; tests/gate.rs 54; native_vertical.rs 37; prep common 32; fixture_consumer 28; S-A Cargo.toml 4, lib.rs 3).

## Runs
- Gate `runs/gate-1/`, plan `a1/gate-plan.json` (sha256 `1739669a…`), modelled on the S1 root-gate plan: same WE_PI_SDK_*, own `CARGO_TARGET_DIR=a1/target`. Result: **13/13 passed**, summary sha256 `9f669950…`.
  - Checks: toolchain; fmt; clippy `-D warnings` in the fixture, default and test-support sets; preparation fixture lib alone.
  - Tests: test-four-plus-sa (312 passed / 25 binaries); preparation fixture (42); default features (92); native_vertical once with test-support (33 passed).
  - Also: hashes, `git status`, `git diff --cached` (empty: no staged files).
  - authority-store `--test store` omitted: that crate was not touched and does not depend on S-A.
- Mutants `mutants/runs-1/`: plan `mutants/mutant-plan-1.json` (`9a6bc61f…`) and `mutants/mutate.py` (`c4d344fb…`, exact-once, backups and sha256 under `mutants/<id>/`). 7 mutants were killed, each by an assertion panic (checked in stdout/stderr). Pre-hashes equal post-hashes. Summary `4d09981c…`.

## Matrix summary
See `a1/MATRIX.md`: 9 invariants, 7 mutants, all killed, plus 5 negative controls/regressions. The plan's "first served obligation only" mutant is structurally equivalent (derive gives one served obligation per reviewer vantage). It is replaced by the reachable two-obligation-candidate mutant (A1-M3).

## Deviations from the plan
1. **review-preparation tests changed.** §5 lists them only for A2. Their fixture-consumer world admitted over the frozen stand-in world, and 16 tests broke under S1. The fix is test-only and mechanical: a scoped frozen world. This touches files A2 also edits, so expect a small rebase conflict in `tests/common/mod.rs` and `fixture_consumer.rs`.
2. S1 compares hex strings rather than returning `Digest`, because `from_hex` is crate-private.
3. The plan said dropping S1 from `ALL` panics at `gate.rs:572`. It does not: the unread map entry is ignored. The kill comes from the listing test `s1_the_base_admission_passes_s1_and_lists_every_check`.
4. Unknown and unserved vantages refuse S1 (`VantageUnknown` / new `ScopeDigestUnserved`). The plan did not specify this. I chose fail-closed and record it here.
5. The S1 placement in `evaluate` is beside O5 (candidate branch), not textually next to `r1_r2`. It does not depend on the realization.

## Unfinished
- Fable review / remediation / recheck and the root gate on a clean export are the parent's.
- No commit was made (the packet forbids it).

## Explicitly unclaimed
- Fixture standing only. Every admission here is a `Domain::Fixture` admission over synthetic records.
- Production admission still refuses unconditionally; S1 does not change that.
- `work-engine-review-hosted` stays `owner_pending` and was not changed. Its r1/r2 stand-in digests were not parsed (hosted never runs S-A).
- Items 2–4 were not touched.
- Contract 3.1 text in the repository: the definition lives only in the doc comment and this records entry (O-1).

## Known contradiction: the hosted crate still binds scope.digest to patchIdentity (root, review P2, 2026-11-18)

Recorded by the root after the A1 review (`review/REVIEW.md`, P2). Operator decision O-5 keeps `work-engine-review-hosted` unchanged in this campaign, so the contradiction is recorded, not fixed:

- `rust/crates/work-engine-review-hosted/src/subject.rs:189-194` (`materialize`) refuses any requirements whose `scope.digest` differs from the subject's `patchIdentity` (`Refusal::ScopeDigestMismatch`).
- `rust/crates/work-engine-review-hosted/src/check.rs:604-606` (check `subject_bound_to_scope`) passes only when `scope.digest` equals `patchIdentity`.
- `rust/crates/work-engine-review-hosted/CLAIMS.md:44` states the same rule.

This is the opposite of the owner definition this slice records (`work-engine-organization-authority/src/gate.rs:240-246`: `scope.digest` is the exact-subject digest). After A1 merges, a requirements world that follows the owner definition is refused by the hosted crate, and every hosted world (r1/r2) is refused by S-A check S1. RESULT's earlier statement that the hosted crate is "unaffected" means only that its code is unchanged.

Prerequisite: before any hosted admission route (D7), the hosted crate must move to the exact-subject digest, in `materialize`, `subject_bound_to_scope` and `CLAIMS.md`, with its route-qualification requirements fixtures regenerated.
