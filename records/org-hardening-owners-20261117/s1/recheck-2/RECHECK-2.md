# S1 recheck 2 (R-1 only, after remediation 2)

Reviewer: Claude, requested Fable, observed `claude-fable-5-2`. Read-only: no builds, no mutating git, no network.
Cost and context: `usage_unavailable` (no binding given; root reports the budget cap was reached before writing).
Paths are under `rust/crates/work-engine-review-preparation/` unless absolute. Custody is `s1/remediation-2/`.

## Verdict: `accept_private_readiness` (R-1 closed; one custody limit, one P3 wording nit)

## Check 1: the admission is real and the refusal is the consumer's (established)
- `gate::admitted` panics on any S-A refusal (`tests/common/mod.rs:381-382`), and the test then asserts the
  admission's vantage, candidate digest, boundary kind, three subject fields and S-A `Current` state before the
  refusal (`tests/fixture_consumer.rs:898-916`).
- `"vantage"` is emitted at one site, `src/fixture.rs:295` (grep of `src/`: 4 `mismatch` sites in `check_proof`;
  the sites at `:708-733` use other field names). The test requires exactly `GateProofMismatch { field: "vantage" }`.
- `mutants/runs-2`: mutation is the single edit `return mismatch("vantage")` -> `let _ = ...`; 15 passed, 1 failed;
  panic at `fixture_consumer.rs:192:24`, ``case `second obligation's vantage` must be refused``. That panic means the
  mutated consumer returned `Ok` for the second-vantage admission, so S-A admitted it and every other check passed.
  `fixture.rs` is `caa4d925` in pre and final hashes. n = 1 mutant, n = 1 two-obligation shape, synthetic world.
- `mutants/runs` (the pre-fmt run, MATRIX "both runs"): not read by me, `not established` here.

## Check 2: the control is meaningful (established)
Control and refusal use the same candidate, declaration, admissions and `bound_vantages`; only `Ask.vantage`
differs (`:880-884` vs `:892-897`). The control reaches `Phase::Reported`, so the refusal is not caused by the
two-obligation candidate, the digest update or the extra binding.

## Check 3: test-support changes (established for the named changes; one limit)
- `Ask::default()` has `bound_vantages` empty (`common/mod.rs:316`), so the loop at `:257-265` issues nothing.
  The other constructions (n = 4: `fixture_consumer.rs:128`, `:802`, `:810`, `:818`) take the default; grep of
  `rust/crates/**/tests/**` for `gate::Ask|gate::admitted|bound_vantages` shows no other user.
- vs HEAD: `common/mod.rs` +273/-0 (no HEAD line removed); `fixture_consumer.rs` +222/-4, the 4 removed lines
  being two import lines and `refused_with`'s `expect_err` message and state message (read in the `-U0` diff).
- **Limit (custody):** the entry blobs `85e50a3b`, `85fe6c64`, `57f01833` are not in the object store
  (`git cat-file -t 85e50a3b`: "Not a valid object name"; `git diff <blob> <file>` fails for all three), so the
  remediation-2 delta could not be diffed. I reviewed the current blobs (`c856f41e`, `ecf9f80e`, `097936f2`, equal to
  `gate-2/hashes.stdout`; the other 8 hashes not re-hashed by me). That nothing else in the two test files changed
  between the recheck and now is `not established`; S1-P1..P7 were not re-run on the final test blobs. The S1 tests
  as they now read (`:744-833`, `:924-947`) each assert an exact refusal. Closing action, no code change:
  re-run S1-P1..P5 on the final blobs, or hash entry files with `-w` in future.

## Check 4: wording (truthful, one nit)
- Comment `fixture_consumer.rs:773-783`, test doc `:862-866`, `Cargo.toml` `fixture` comment (consistent with
  `src/fixture.rs:225-229`), `s1/MATRIX.md:76, 81-88, 99-105`, `remediation-1/RESULT.md:21, 79-81`: read, accurate.
  The comment's S-A R2 premise rests on recheck 1's reading of `gate.rs:1090-1096`; not re-read by me.
- P3 nit: `s1/RESULT.md:137-138` still says "no admissible other-vantage world was constructed" with no pointer to
  Remediation 2 (declared as left-as-history in `remediation-2/RESULT.md`). A one-line forward pointer would do.
- `gate-2/summary.json`: `passed`, 12 steps outcome 0, leftovers 0; `test-preparation-fixture` 16 / 24 / 2, 0 failed,
  new test `ok`.

Carried unchanged from the recheck (not re-reviewed): P3-1 narrowing, P3-4, M6 stronger variant, S1-P6/P7, M1-M11.
