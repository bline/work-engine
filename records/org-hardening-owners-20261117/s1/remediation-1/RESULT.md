# S1 remediation 1: result (`map_ready`, one finding partly met)

Role: bounded builder, requested model Sonnet 5.5 (`claude-sonnet-5-5`). Context usage and cost:
`usage_unavailable` (no context binding was given; spend not observed by me). Subject: worktree
`/home/bline/cbm-work/wt-org-hardening-20261117`, uncommitted on `0fe9dc74`. Nothing staged,
committed or branched. Authority: `s1/review/REVIEW.md` (`changes_required`).

## Entry check

The gated sources equalled `s1/runs/gate-1/hashes.stdout` before any edit: 9 of 9 blobs equal
(`4d8feb23`, `48f8d280`, `d6956684`, `9e0c4b1e`, `db708737`, `03da1081`, `cece6d30`, `57f01833`,
`97b2ac95`), by the same `git hash-object` list as the last step of `gate-plan-1.json`, cwd `rust/`.

## Changes mapped to findings

| Finding | Change | Where |
|---|---|---|
| P2-1 false absence sentences | Rewritten to the reviewer's true statement: without S-A `fixture` no episode can advance (S-A production policy admits nothing); with it any caller can mint a fixture consumption admission and consume outside review-delivery's gate (no delivery gate, spawn, native correspondence or blocker evaluation); the consumer checks that S-A found *a* consumption admission current for its candidate, subject and vantage, not whose it is nor how fresh the observation is | delivery `src/native.rs` module docs; delivery `src/lib.rs` module docs; preparation `src/lib.rs` (fixture bullet); preparation `src/fixture.rs` module docs |
| P2-2 freshness unlabelled | Constant `GATE_OBSERVATION_FRESHNESS_ASSERTED_LABEL = "caller_asserted_regate_observation_freshness"` and `GateCheck::caller_asserted_regate_observation_freshness()` (a method like `time_label`, not a field, so the mutants' fabricated `GateCheck` still compiles); asserted in the positive test against the literal and the constant; `ConsumptionProof` and module docs say freshness is a caller assertion | preparation `src/fixture.rs`, `tests/fixture_consumer.rs` |
| P3-1 boundary kind only | Documented in the module docs and `ConsumptionProof` docs: only the boundary kind is compared, operation and epoch are not (O10 class); the proof is reusable, the episode owner refuses a second use. No code change | preparation `src/fixture.rs` |
| P3-2 vantage negative case + mutant | **Partly met.** Mutant `S1-P8-vantage-unchecked` added and run; no negative case could be built (see Mutants). The uncovered check is stated in a test doc comment and classified defense in depth (corrected in Remediation 2: not defense in depth). No mutant killed | `tests/fixture_consumer.rs` doc comment; `mutants/mutate.py` copy |
| P3-3 loop case unnamed | `refused_case(case, ..)` names the case in the failure message; `refused_with` delegates with an empty name; the loop and the two not-current cases pass their names | `tests/fixture_consumer.rs` |
| P3-4 | Left as is; carried (below) | - |

`tests/common/mod.rs` was edited during the P3-2 attempt and restored by reverse edits; its blob is
in `runs/gate-3/hashes.stdout` (line 11, `85fe6c64`), but no entry-time blob of it was recorded, so
"restored exactly" is by construction and the compile and tests, not by hash.

## Diff stat against `0fe9dc74`

12 files, 788 insertions, 73 deletions (review baseline: 731/-71). Changed relative to entry: delivery
`src/native.rs` (documentation only), delivery `src/lib.rs` (documentation only), preparation
`src/fixture.rs`, preparation `src/lib.rs` (documentation only), preparation
`tests/fixture_consumer.rs`. Blobs after: `native.rs` `82ca23e7`, delivery `lib.rs` `9b3209db`,
`fixture.rs` `caa4d925`, preparation `lib.rs` `7fcca1f6`; `gate.rs`, `binding.rs`, both
`Cargo.toml` and `Cargo.lock` equal to entry.

## Runs

Plan `remediation-1/gate-plan-2.json` = `gate-plan-1.json` with the hashes step extended by three
test files; run by `custody.py run --keep-going`.

- `runs/gate-2`: **failed 10/12** (`test-four-plus-sa`, `test-preparation-fixture`): the P3-2
  attempt's world. `fixture_consumer.rs` `s1_a_gate_proof_for_another_candidate_subject_or_boundary_is_refused`
  panicked in the test world: `R2: Refused([VantageUnknown "vantage:fixture-other-review"])`. Kept as recorded.
- `runs/gate-3`: **passed 12/12**, after the vantage world was reverted: fmt, three clippy variants
  with `-D warnings`, the preparation fixture lib alone, `test-four-plus-sa` (25 `test result: ok`
  lines, none failed), `test-preparation-fixture` (consumer 15, preparation 24, doctests 2),
  `test-default-features`, `test-authority-store`, `native-1` (33 tests, includes the real Pi SDK child).
  Test counts for `gate-1` were not compared (`not established`).

## Mutants (`mutants/runs`, `mutants/mutate.py` is a copy of `s1/mutants/mutate.py` plus the new id)

Pre and final blobs of `gate.rs`, `native.rs`, `fixture.rs` equal each other and `gate-3`
(`4d8feb23`, `82ca23e7`, `caa4d925`); every restore step passed its sha256 check. n = 1 mutant per row.

| Mutant | Mapped test | Result |
|---|---|---|
| S1-P1 candidate-digest refusal neutralized | loop test | killed: `fixture_consumer.rs:192`, `case \`other candidate\` must be refused` |
| S1-P2 subject refusal neutralized | loop test | killed: `case \`other subject\``|
| S1-P3 boundary-kind refusal neutralized | loop test | killed: `case \`delivery boundary\`` |
| S1-P4 `Unknown` accepted | not-current test | killed: `case \`no observation\`` |
| S1-P5 skip the proof check | not-current test | killed: `case \`revoked pin\`` |
| **S1-P8** vantage refusal neutralized (new) | whole consumer suite | **survives**: 15 passed, 0 failed |

S1-P8 is the one place the remediation does not match its request, and the reason is evidence, not
effort: S-A's R2 (`work-engine-organization-authority/src/gate.rs`, the `find(|v| v.vantage == vantage)`
that returns `VantageUnknown`) admits only a vantage that the candidate's requirements name. Two
other names were tried (a made-up reviewer vantage in `gate-2`; the candidate's builder participant
`vantage-builder` bound with its own realization and acceptance, in one diagnostic
`cargo test` run outside custody, so not custody evidence) and both gave `VantageUnknown`. So in
this world no S-A admission exists for another vantage with this candidate's digest, and the
consumer's vantage comparison cannot be exercised through the test API. A candidate with two
vantage requirements and a consumer world built on it was not attempted (it reshapes the
requirements and every world helper). Whether any candidate can carry two vantages: `not established`.

## Carried

- **P3-2**: vantage comparison uncovered by a mutant. *Corrected by Remediation 2 (`remediation-2/RESULT.md`):
  "defense in depth" was wrong; the comparison is load-bearing for a two-obligation candidate, and is
  now tested and mutant-killed.*
- **P3-4**: `native_vertical` requires `test-support`, so S-B gated-path tests run only where
  `test_support` exists; behaviour unchanged. Not changed.
- M6 stronger variant (the replay also becomes the proof) from the review: not run, `not established`.
- S1-P6, S1-P7, M1-M11 not re-run (anchors and mapped tests text-unchanged; only `native.rs`
  documentation changed). Their `gate-1`/`runs-1`/`runs-2` evidence stands on the prior blobs.
- Custody housekeeping: `rm` and `mv` were refused in this session, so an identical duplicate
  `remediation-1/mutate.py` remains beside `remediation-1/mutants/mutate.py`; the original
  `s1/mutants/mutate.py` is unchanged.
- Process: no child agents, no listeners, no commits.
