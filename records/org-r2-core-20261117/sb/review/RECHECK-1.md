accept_private_readiness

# S-B recheck 1 (same review episode, retained context; Opus 5.5 `claude-opus-5-5`; read-only)

**Subject.** Checkpoint `77c57abf50949f158844a9e54b97d893f23c401e`, tree `06e9f75e…`, parent `dd68df6c`.

**Read.** I read the full remediation diff `dd68df6c..77c57abf`: 5 files, +558/−152, covering `src/gate.rs`,
`src/native.rs`, `tests/common/mod.rs`, `tests/gate.rs` and `tests/native_vertical.rs`. I also read:
- `ROOT-SB-DESIGN-DECISION.json`;
- the design review, §4–§7 only;
- `remediation-1/RESULT.md`;
- the MATRIX and RESULT addenda;
- mutant stdouts N1 and N4;
- `ROOT-GATE-2.json`;
- the export-2 `lib.rs` module doc and a grep of the crate for absence wording.

**Evidence.** I ran no build or test. The executed facts below come from the root gate and the builder's custody
records:
- Root gate at `77c57abf`: passed. Fixture tests 330, default 42, authority-store 37, native 32/32 ×3, fmt and
  clippy clean.
- N1 record: under the other-vantage mutant, T1 failed with refused set `[R2]` against the expected `[R3]`.
- N4 record: without the delivery regate, T4's replaced-pin scenario was not refused and the delivery ran.

Everything else is static reading of the diff.

## Checks requested

**P2-1: resolved by construction.**
- `FixtureAuthority` (`src/native.rs`, struct after the `Delivered` doc) no longer has `request` or `operation`.
  Its fields are `source`, `clock`, `organization_id`, `requirements`, `actor_identity` and `installed`.
- The private `construct_request` builds the request from the binding with a single `OperationRequest::new` call:
  - vantage = `binding.preparation().model_context.vantage.id`;
  - subject and candidate digest = the binding's;
  - organization id = the caller's labelled value;
  - requirements = the caller's preimage, refused with `AdmissionMismatch { field: "requirements" }` unless
    `requirements_digest()` equals the candidate's.
- `construct_request` runs before any admission, and both admissions use its result. `compare_with_binding` is
  deleted.
- With the candidate digest now host-set, S-A O3 becomes a real check that the preimage derives the binding's
  candidate.
- The caller-chosen inputs that remain are the organization id and the declared actor identity (both labelled
  `caller_asserted_unanchored`) and the declared installed digests (labelled by role).
- The positive test (T2) shows the summary's vantage equals the preparation's and equals `VANTAGE`, so the
  admission passed R2 for it. This settles REVIEW-1's open question about identifier compatibility.
- T1 pins the exact R3 `ActorMismatch` subject to the preparation's vantage, and N1 kills it.

**Design review §4 inventory: holds at the subject.** Every input that reaches `admit`, `evaluate` or
`admission_state` on the host path:

| Input | Status at subject |
|---|---|
| Request vantage | Host-derived |
| Subject | Host-derived |
| Candidate digest | Host-derived |
| Requirements | Digest-bound preimage, checked before any observation (T3 shows 0 observations) |
| Organization id | Labelled assertion |
| Realization digest | Host-derived, unchanged |
| `actor_identity` | Declared; labelled in the summary |
| `installed` | Declared; not surfaced, since S-A has no accessor (stated) |
| Boundary operation | Host-derived via `boundary_operation(binding)` (T7) |
| Time | Sampled from a held `TimeSource` at both admissions and both regates |
| Authority source | Supplied by the caller and held in `HeldAdmission`; `regate()` takes no argument |

`cfg` reaches only `realization_description`, which `prepared_blockers` requires to equal the binding's
description. The `node`, `sdk_root` and `control` inputs are carried.

**P3-1: resolved.**
- `delivery.regate()` runs after `assess` and immediately before `run_native`, and only `Current` permits.
- T4 uses a scripted source that supersedes a pin before the second observation. It expects
  `NotCurrent(Stale(IssuanceSuperseded([pinned])))`, spawn 0 and exactly 3 observations.
- N4 confirms that the scenario delivered when the regate was removed. This is the REVIEW-1 P3-1 case, now
  observed.

**P3-2: resolved.**
- `HeldAdmission::admit` and `regate` each call `clock.now_millis()`. No sample is stored or reused.
- T5 sets the grant to expire at 2000 with a scripted clock `[1000, 1000, 1000, 3000]`. It expects `Refused` with
  `[O4]` at consumption, consumer 0, the run retained and 4 samples. N5 kills it.

**P3-3: the absence sentences are now truthful.**
- The feature split was not done; the reason is in the RESULT decision 1.
- `native.rs` module doc (lines 14–24): it says `test_support` is public under `fixture` and outside the gate,
  says a route around the gate exists through it or through `consume_fixture_result`, and claims absence only
  for "this crate's binding-taking function".
- `run_attempt` doc (line 615) and `deliver_and_consume` doc (line 1511): narrowed the same way.
- `gate.rs:46` ("Production admission is not reachable") is still true.
- I found no other spawn-route absence claim in `src/` (grep for `only (route|spawn|caller|function|way)` and
  `not reachable`).
- The open route itself is carried in RESULT "Added by remediation 1" and in the root decision's `carried`.

**P3-5: resolved.** The comment now says the supersession half does not separate finality from recomputation
(`tests/gate.rs` finality test).

**P3-6: resolved.** The `realization:`-prefix assertion on `realization_evaluation` is restored in
`the_assumption_world_screens_into_admitted_and_assumed_mapping_kinds`.

**P3-4 and P3-7.**
- P3-4 is superseded. The F-7 comparison is a replaced claim (MATRIX row 2b, retired with M5, not counted as
  coverage kept). The requirements case it lacked is now T3.
- P3-7 is addressed. RESULT's addendum lists the organization id (with a plan-level anchor item), the declared
  actor/installed values, `node`/`sdk_root`/`control`, `consume_fixture_result`, `test_support`, and the status
  of the requirements preimage.

**No weakened guard.**
- Unchanged: `run_native`'s `bound_to` check, `prepared_blockers`, `screen`/`assess` refusal order, the
  consumption regate (the last step before `consume`, `Current` only, Unknown fails closed), the F-3 finality
  logic, and the private `consume`.
- `compare_with_binding` is gone, but nothing it covered was lost:
  - subject and candidate are set by construction;
  - requirements are checked earlier, before any observation;
  - its realization branch was already unreachable.
- `HeldAdmission::admit` stays public with a caller request. It yields no discharge token; `assess` is
  `pub(crate)`.

## P3s to carry

1. **The delivery regate's revision is missing from the success output.** `src/native.rs`, the delivery regate
   match, reads `AdmissionState::Current { .. } => {}` and discards the revision. The positive test still
   asserts `admitted_before_spawn.admission().regate_revision() == None` (`tests/native_vertical.rs`,
   `positive_assertions`). The success record therefore reads as if the delivery admission was never regated,
   although it was. This is the same class as S3 F-4: evidence that does not reach `Delivered`. Fix: apply
   `with_regate(revision)` after the regate, assert `is_some()`, and update the `AdmissionSummary` doc
   ("consumption only"). Confidence high (static).
2. **S-B mutants are not re-established on the corrected code.** M4, M6, M7 and M8 anchor on text that changed;
   M1–M3 and M9–M11 were not re-run. Their mapped tests pass in the root gate. This is disclosed in MATRIX.
   N4 and N5 cover the new regate and time paths. M6 (replay of the admission-time observation) has no
   re-established kill on the new `regate()`.
3. **T1 does not exhibit a vantage V′ that would otherwise be admitted.** In the swap world the builder vantage
   fails R2, as N1 shows. This is disclosed. Construction plus T2 plus the exact R3 subject are sufficient for the
   claim.
4. **Carried by the root decision and recorded truthfully:**
   - organization anchor (owner not yet named);
   - O8 declared actor/installed values;
   - `node`/`sdk_root`/`control` presumed inputs;
   - `consume_fixture_result` and `test_support` routes outside the gate;
   - O10 (same binding → same boundary identity, now concrete);
   - fixture standing;
   - assumed mapping kinds;
   - doctest error codes not enforced by stable rustdoc (T6, N-1 class).
5. **Minor test strength gap.** T5 asserts `[O4]` refused but not the `AuthorityExpired` reason code. Expiry is
   the only variable in that test, so the gap is small.

**Verdict.** `accept_private_readiness` for `77c57abf`. P2-1 is resolved by construction, P3-1, P3-2, P3-5 and
P3-6 are resolved, and the P3-3 wording is now truthful. Item 1 above is the only new finding; it is a small fix
that can be carried or folded into the next touch of this file.
