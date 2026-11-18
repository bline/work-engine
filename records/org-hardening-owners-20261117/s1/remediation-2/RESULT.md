# S1 remediation 2: result

Role: bounded builder. Packet: operator-authorized option (a) for the Fable recheck finding R-1
(`s1/recheck/RECHECK.md`). Requested model `claude-sonnet-5-5` (observed identity not independently
read). Cost and context usage: `usage_unavailable` (no binding given). No commit, stage, branch or
mutating git; no network, Docker or live provider; none of the forbidden paths read.

## Verdict

R-1 is repaired by test, not by carrying the gap. S-A admitted the second obligation's vantage in the
test world, so no S-A change or stop was needed. Mutant `S1-P8` is now killed by the new case, and for the intended reason.

## Entry hashes (cwd `rust/`, `git hash-object`, same list as gate-3's last step)

Entry equals `remediation-1/runs/gate-3/hashes.stdout`, 12 of 12. Blobs of the three files I touched
(recorded before editing):

| File | Entry blob | Final blob |
|---|---|---|
| `crates/work-engine-review-preparation/tests/fixture_consumer.rs` | `85e50a3b` | `c856f41e` |
| `crates/work-engine-review-preparation/tests/common/mod.rs` | `85fe6c64` | `ecf9f80e` |
| `crates/work-engine-review-preparation/Cargo.toml` | `57f01833` | `097936f2` |

Final `gate-2/hashes.stdout` vs gate-3: 9 of 12 identical (delivery `gate.rs` `4d8feb23`, `native.rs`
`82ca23e7`, `binding.rs` `d6956684`, `lib.rs` `9b3209db`, `Cargo.toml` `db708737`; preparation
`fixture.rs` `caa4d925`, `lib.rs` `7fcca1f6`; `Cargo.lock` `97b2ac95`; `native_vertical.rs` `6f0a952c`).
The 3 that differ are the three above. `fixture.rs` (the code under test) is unchanged.

## Changes

1. `tests/common/mod.rs` (test support only): `gate::Ask` gains `bound_vantages: Vec<String>` (default
   empty); `issue` binds each extra vantage like the first reviewer vantage (same principal, realization
   digest and actor, so the existing acceptance covers it). Every other construction of `Ask` uses
   `..Default::default()`, so no other call site changed (grep: three, all in `fixture_consumer.rs`).
2. `tests/fixture_consumer.rs`:
   - `two_obligation_requirements()`: the frozen requirements plus a second obligation
     `rp3-second-implementation-review` with its own campaign-boundary condition, derived the way
     `tests/preparation.rs:551-575` `with_second_obligation` does (a local copy; `preparation.rs` untouched).
   - New test `s1_a_gate_proof_for_another_obligations_vantage_is_refused`:
     a. world candidate = the two-obligation candidate; declaration digest updated; declaration still names
        the first obligation.
     b. Control: first obligation's vantage on the same two-obligation candidate is consumed (phase
        `Reported`), so the refusal is about the vantage and not about the two-obligation candidate.
     c. S-A admission for `vantage:rp3-second-implementation-review` through `Ask.vantage` at a
        Consumption boundary (`admitted` panics on an S-A refusal; it did not). Asserted before the
        refusal: request vantage equals the second vantage; candidate digest, boundary kind and the three
        subject fields equal the consumer's; S-A's own `admission_state` is `Current`.
     d. `refused_case` expects `ConsumeRefusal::GateProofMismatch { field: "vantage" }` and the input
        state unchanged.
   - Comment on `s1_a_gate_proof_for_another_candidate_subject_or_boundary_is_refused` rewritten: the
     frozen candidate has one obligation, which is why that test has no vantage case; with two obligations
     the comparison is load-bearing and is covered by the new test. "Defense in depth" removed.
3. `Cargo.toml` (preparation) `fixture` feature comment: the observation is one "the caller asserts is
   the host's fresh one (S-A's point check finds it current; its freshness is not checked here)".
4. Wording: `s1/MATRIX.md` row 1f and its remediation-1 row, and the remediation-1 inventory line;
   `remediation-1/RESULT.md` (P3-2 table row and Carried item). `s1/RESULT.md` lines ~137 and ~162 (the
   original slice result) are left as the history they record; the correction is in MATRIX and this file.

## Runs

- `runs/new-test`: `fixture_consumer` 16 passed, 0 failed (the new case passes first run).
- `runs/gate-1`: **failed 11/12**: `fmt --check` only (one `assert_eq!` line in the new test needed
  wrapping); all other steps passed. Kept as recorded. Fixed by hand, formatting only.
- `runs/gate-2`: **passed 12/12** on the final blobs: fmt, three clippy variants, preparation
  fixture lib alone, `test-four-plus-sa`, `test-preparation-fixture` (16 / 24 / 2 per binary, 0 failed),
  `test-default-features`, `test-authority-store` (37), `native-1` (33), hashes.
  The `gate-plan.json` copy is `remediation-1/gate-plan-2.json` unchanged: its hashes step already
  lists all three files I touched.
- Gate-1 vs gate-2 test counts, and counts against remediation 1's gate-3: `not established` (not compared
  beyond the per-step lines above).

## Mutants (`mutants/`; `mutate.py` is a copy of `remediation-1/mutate.py`, unchanged)

S1-P8 `return mismatch("vantage");` -> `let _ = mismatch("vantage");` against the whole
`fixture_consumer` suite:

| Run | Outcome |
|---|---|
| `mutants/runs` (blob before the fmt fix) | 15 passed, 1 failed; exit 101 |
| `mutants/runs-2` (final blob) | 15 passed, 1 failed; exit 101 |

Both: the one failure is `s1_a_gate_proof_for_another_obligations_vantage_is_refused`, panic at
`fixture_consumer.rs:192:24`: ``case `second obligation's vantage` must be refused``. That is the
`refused_case` "nothing refused" panic in the new case, not the control or an S-A refusal. `fixture.rs`
blob `caa4d925` before and after each run (restore verified). Result: **killed, for that reason**
(n = 1 mutant, n = 1 world).

S1-P1..P5 not re-run: their anchors (lines in `fixture.rs`) and mapped tests are text-unchanged by this
remediation (only a doc comment above the loop test changed).

## Diff stat vs `0fe9dc74` (working tree)

12 files, 893 insertions, 73 deletions (was +788/-73 at the recheck). This remediation: `fixture_consumer.rs`
+226 total, `common/mod.rs` +273 total, `Cargo.toml` (preparation) 11 lines changed; three files together
504 insertions, 6 deletions vs `0fe9dc74`.

## Not established / carried

- Test world is synthetic (fixture S-A, assumed time, assumed observation freshness); the new case shows
  the comparison is reachable and refuses in that world, not a production property.
- Only a two-obligation candidate (n = 1 shape, second obligation a copy of the campaign condition) was
  built; other multi-obligation shapes are not exercised.
- Carried unchanged from the recheck: owner acceptance of P3-1 narrowing, P3-4, the stronger M6 variant,
  S1-P6/P7 and M1-M11 resting on gate-1 blobs, `remediation-1/mutate.py` duplicate.
- Process: no child processes started by me remain (custody `leftovers=0` on every step).
