# S-B result: consumption seam and real discharges (fixture)

Worker: Claude Code (Opus 5.5, `claude-opus-5-5`), background session; parent/collector Pi root
`01a1e56e-be12-7424-977b-c7c7983887d0`. No identity binding or context-status binary was delivered, so the
mid-task context check was not run (`usage_unavailable`: no binding). Coordination board not read (no claims or
consequential operations were made). Cost not measured by this worker (`not established`).

## Entry check

- Worktree `/home/bline/cbm-work/wt-org-r2-core-20261117`, branch `org-r2-core-20261117`, HEAD `561c6ffc`.
- `git diff --stat bfaa455c` at entry: only the 9 untracked S-A crate files appeared (as deletions); tracked
  `rust/Cargo.toml` and `rust/Cargo.lock` equal the checkpoint.
- All 9 S-A crate files hashed with `git hash-object`; each equals its blob in `git ls-tree -r bfaa455c`
  (9/9). Re-hashed at the end: still 9/9 equal (S-A crate unchanged).
- Result: working tree equalled the accepted S-A checkpoint at entry. Nothing committed, staged or branched.

## Files changed (all in `rust/crates/work-engine-review-delivery` plus the lockfile)

| File | Reason |
|---|---|
| `Cargo.toml` | depend on `work-engine-organization-authority`; its `fixture` only via this crate's `fixture`; description |
| `../../Cargo.lock` | the new dependency edge (+1 line) |
| `src/gate.rs` | new total `discharge` (`Admitted(&[CheckId])` / `AssumedDischarge` / `Undischarged`); token-free public `screen`; crate-private `assess(boundary, binding, blockers, &EffectAdmission)` keyed on `CheckId`, asserts `Domain::Fixture`; `AdmissionSummary`, `AdmittedDischarges`, bound `AssumedDischarges` (`bound_to` = frozen contract digest); `HeldAdmission` (reserve+observe+`admit::<Fixture>`; `regate` takes the source, never an observation; first non-current non-Unknown state final); new `GateRefusal` variants; compile_fail doctests |
| `src/native.rs` | `FixtureAuthority` input; host `realization_digest` (labelled fixture definition: SHA-256 of the description's JSON); `deliver_and_consume[_observed]` now: screen → delivery + consumption admissions (epoch 1 each) → F-7 `compare_with_binding` → `assess` → spawn → verify → seam → `assess` over consumption admission → fresh regate (`Current` only) → consume; `run_native` checks `bound_to`; `run_attempt*` crate-private with `#[doc(hidden)] test_support` wrappers; compile_fail doctests; doc wording |
| `src/lib.rs` | crate doc wording for the S-B gate |
| `tests/common/mod.rs` | `authority` module: fixture authority world (principals, vantage-actor bindings, component bindings, grants, decision scope, realization acceptance of the host digest), request builders, `Switch` source for the missing-grant case |
| `tests/gate.rs` | rewritten onto `screen` + new mapping and `HeldAdmission` tests |
| `tests/native_vertical.rs` | call sites take a `FixtureAuthority`; positive assertions for admission summaries/labels; 4 new `sb_*` tests; generic-runner tests import `test_support` |
| `tests/fixtures/assumption-world/DERIVATION.md` | S-B wording; the realization acceptance's `provides` labelled as a fixture statement |

Diff stat vs `bfaa455c` (tracked): 9 files, +1330 / −189. No S-A API additions; no S-A, organization or
preparation crate change.

## Runs (custody, `CARGO_TARGET_DIR=/home/bline/cbm-work/org-r2-core-20261117/target`)

- `runs/gate-1` (plan `gate-plan-1.json`, summary sha256 `1a91475f…b870`): **8/8 passed** — toolchain; fmt
  (5 crates incl. S-A); clippy `-D warnings` all targets with `work-engine-review-delivery/fixture` +
  `work-engine-organization-authority/fixture`; clippy default features; `test-four-plus-sa` (organization,
  review-preparation, episode-core, review-delivery, S-A, fixture features; includes the native suite, 28/28);
  default-feature tests (review-delivery, review-preparation, S-A); authority-store `--test store` (37/37);
  `native-1` (`native_vertical`, 28/28, 70.8 s).
- `runs/dev-1`: earlier targeted native run (7/7), superseded by gate-1.
- Note: the native suite ran twice in gate-1 (inside the five-crate test and as `native-1`); the packet asked
  for one native run beyond the crate tests, modelled on the S3 plan.

## Matrix

`MATRIX.md`: every invariant mapped to named tests; 11 mutants (M1–M11, n = 1 each), all killed by a
behavioural test failure or a compile_fail doctest failure, all restored to the exact pre-mutation blobs
(`gate.rs` `3d4ab245`, `native.rs` `7566332c`). M7's first attempt (runs-1) did not compile and is not
counted; its rerun (runs-2) was killed.

## Design decisions and deviations

1. **Consumption admission obtained before spawn.** `admission_state` needs an observation sealed to the
   admission's own boundary, so the regate needs an admission at a consumption boundary. It is admitted before
   spawn (own observation) so that its pins predate the spawn; a supersession between spawn and consumption then
   shows as `Stale` (a fresh admission after the run would silently pin the replacement). Discharges before
   spawn come from the delivery admission; before consumption from the consumption admission.
2. **No point check of the delivery admission** between its admission and the spawn (admitted immediately
   before spawn). Not required by the packet; stated here.
3. **F-7 compares exactly the four named fields.** The request's vantage is not compared with the
   preparation's vantage.
4. **P3 run_attempt\* closed by the packet's first option:** crate-private, with `native::test_support`
   wrappers under `fixture` (`#[doc(hidden)]`). Under `fixture` a generic caller-built-command runner is still
   publicly callable through `test_support`; it takes no binding and never reaches the consumer.
5. **`HeldAdmission` is public under `fixture`** (direct finality/fresh-observation tests need it). It wraps
   S-A `admit` and yields no discharge token.
6. `NotAdmitted` is unreachable through S-A (see MATRIX note); CheckId keying is by construction.
7. M4 is a two-file mutant (adds a mutant-only `mutant_assumed` helper in `gate.rs`); M10/M11 are killed via
   compile_fail doctests whose error codes stable rustdoc does not enforce (S-A P3 N-1 class).

## Unfinished

Nothing assigned is unfinished. Not done: independent review; checkpoint/freeze of this subject (packet:
no commit).

## Explicitly unclaimed

- Fixture standing only: the authority world, source, records and the realization acceptance are synthetic
  owner attestations. The acceptance's `provides` (mutation `none`, custody, observer class) is a fixture
  statement, not an established property of the unconfined child; process-level writes remain not established.
- The three mapping kinds pass only by labelled assumed discharge (no owner, O7); assumed discharges are not
  admissions.
- O10 gap: no single-use epoch ledger; both boundaries use epoch 1; a binding can be delivered again.
- R3 compares declared actor identity and installed digests supplied in `FixtureAuthority`; no host observation
  of the spawned process.
- The realization digest definition is a labelled fixture definition over the description, not the bytes run.
- No hosted path, storage, issuance, lifecycle integration or S-C. Production admission still refuses (S-A
  `Production` policy; review-delivery admits only via `admit::<Fixture>`, and the whole gate exists only with
  `fixture`).
- Private readiness only; not committed or integrated.

### Added by remediation 1 (design correction; see `remediation-1/RESULT.md`)

Decision 3 above (F-7 compares four fields) and the `compare_with_binding` description in the files
table are superseded: the host constructs the admission request from the binding and no
caller-supplied request exists. Not repaired by that correction, and not claimed:

- **Organization id** is caller-asserted and unanchored (`caller_asserted_unanchored`): no record in
  `work-engine-organization` or the binding relates an organization to a candidate or problem scope.
  Separation (O8) and ObserverIdentity (R3) are discharged *relative to that asserted organization*.
  Plan-level open item: an organization anchor record; owner not yet named.
- **Declared actor identity and installed digests** (owner decision O8): R3 passes for whatever the
  caller declares that matches the record; the installed digests are not surfaced in the success
  summary (S-A exposes no accessor).
- **`node`, `sdk_root`, `control`** are presumed inputs (S3 repair): the realization digest is the
  same for every value of them. Unrepaired remainder of the S3 F-2 occurrence of the same class.
- **`consume_fixture_result`** stays public in review-preparation under its `fixture` feature
  (S3 `p3_carried[1]`): a fixture caller can consume a result with no delivery gate. Belongs to
  the slice that introduces committed consumption.
- **`native::test_support`** stays public in every build where `native` exists (it is not behind a
  separate feature); a generic runner over a caller-built command exists outside the gate. The
  absence sentences were narrowed to "the only function in this crate that takes a binding".
- **Requirements as input**: the requirements preimage is caller-supplied and digest-bound to the
  binding candidate's requirements digest; S-A O3 then checks it derives the binding's candidate. It
  is consistency with the binding's candidate, not a committed admission anchor. Whether the
  host-side digest check is redundant with O3 is `not established` (mutant N3 shows the check
  changes refusal identity and observation count).
- **Binding is caller-built** (`ExperimentBinding::new` is public); it is the object evaluated, not
  a gate input.
- **Time source** is synthetic and sampled at each evaluation; the host has no real clock here.
- **T1 world**: the test world does not admit a second vantage V' (R2 refuses for it), so T1 shows
  refusal for the preparation's vantage, not that a caller-chosen V' would have been admitted.
- S-B mutants M1-M3, M9-M11 not re-run and M4, M6-M8 not re-runnable as written after the correction
  (MATRIX); M5 retired as a replaced claim.

handoff path: this file. Status: `map_ready`.
