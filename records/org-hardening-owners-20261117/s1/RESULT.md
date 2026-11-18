# S1 result: post-R2 gate hardening (fixture)

Builder: Claude (requested and observed model Opus 5.5, `claude-opus-5-5`), one worker, no
children, no listeners. Packet: `s1/PACKET.md`. Authority: operator 2026-11-17 approval under
`assignment.json` (objective be25b55a). Parent/collector: Pi root
`01a1e56e-be12-7424-977b-c7c7983887d0`. Cost: not observed by me (`not established`); the parent
observes the budget.

## Entry check

`git -C /home/bline/cbm-work/wt-org-hardening-20261117 status --porcelain` was empty; HEAD
`0fe9dc745f7c4157d3980ab6d6dd686fef36ad5c` on `org-hardening-20261117` (contains merged R2
`a765298e`). Board read at entry (seq up to 5844). No commit, stage, branch or other mutating git
was run.

## Design note (invariant 1)

**Route.** The consumer runs the gate check itself on unforgeable S-A values. `consume_fixture_result`
takes a new last argument `&ConsumptionProof { admission: &EffectAdmission, regate_observation:
&Result<SealedObservation, Unobserved>, regate_now_millis: u64 }`. Immediately before the episode
transition (after every existing check, so existing refusal identities are unchanged) it:

1. requires `admission.boundary().kind() == Consumption`;
2. requires the admission's request candidate digest to equal `candidate.content_digest()` (S-A
   O3 already decided that the request's requirements derive that digest, so the requirements
   are bound too);
3. requires the request subject to equal the declaration's subject (which the consumer already
   checks against the episode state's subject digest);
4. requires the request vantage to equal the obligation's serving vantage, which the consumer
   derives itself from the candidate (`Selection::served_by`);
5. runs S-A's own point check `admission.admission_state(regate_observation, regate_now_millis)`
   and proceeds only on `Current`.

Refusals: `ConsumeRefusal::GateProofMismatch { field }` and `GateProofNotCurrent(AdmissionState)`.
On success `FixtureConsumption.gate: GateCheck` carries the boundary, admission revision, regate
revision and the labelled regate time.

Why this is unforgeable outside S-A: `EffectAdmission` has no public constructor and is not
`Clone`; only `admit` produces one, and S-A's production policy admits nothing. `SealedObservation`
likewise is produced only by `observe_for`, is not `Clone`, and S-A treats one sealed to another
boundary as `Stale`. (Both facts are S-A's, pinned by S-A's own compile_fail doctests in
`work-engine-organization-authority/src/lib.rs`.) `ConsumptionProof` itself is a public bundle of
borrows; it confers nothing without those two values. Review-delivery passes its own consumption
admission, the fresh observation its `HeldAdmission` regate decided on, and that regate's time
sample (`regate_observed`, crate-private), so the consumer re-decides on exactly the host's inputs.

**Dependency edge added (one):** `work-engine-review-preparation` -> `work-engine-organization-authority`,
optional, enabled by preparation's `fixture` feature (`fixture = ["dep:work-engine-organization-authority"]`).
Preparation's `fixture` does **not** enable S-A's `fixture`, so a build of preparation with
`fixture` alone has the consumer but no way to obtain an admission. A dev-dependency enables S-A
`fixture` for preparation's own tests. No cycle (S-A depends only on `work-engine-organization`).
`Cargo.lock`: one line (the new dependency entry).

**S-A additions: none. S-A semantic change: none** (no S-A file changed; diff stat below).

**Rejected alternatives.**
- *S-A-issued regate result* (e.g. `EffectAdmission::regate(observation, now) -> CurrentAdmission`):
  an S-A API addition, not strictly needed because the consumer can run S-A's existing public
  point check on S-A-sealed inputs; it would carry no more than this route (same caller-supplied
  time and observation freshness, same absence of review-delivery's finality).
- *`AdmissionState::Current` as the proof*: publicly constructible from `admission.revision()`
  (that is the M7 shape).
- *A proof type issued by review-delivery*: preparation cannot depend on review-delivery
  (review-delivery depends on preparation; cycle).
- *A proof type defined in preparation with a constructor reserved to review-delivery*: Rust has no
  per-crate constructor restriction; any dependent could construct it.
- *Moving the consumer into review-delivery*: moves the episode-consumption boundary between owners;
  larger than this slice.

## Files changed (reasons)

| File | Reason |
|---|---|
| `review-preparation/Cargo.toml` | optional S-A dependency under `fixture`; S-A `fixture` dev-dependency for tests |
| `review-preparation/src/fixture.rs` | `ConsumptionProof`, `GateCheck`, `GATE_TIME_ASSERTED_LABEL`, two refusals, `check_proof`, the new argument, `FixtureConsumption.gate`; module doc + compile_fail doctest |
| `review-preparation/src/lib.rs` | crate doc: the consumer advances only with a current S-A proof |
| `review-preparation/tests/common/mod.rs` | `gate` module: a synthetic S-A world admitting the frozen candidate/subject/reviewer vantage (frozen realization constraints satisfied by the acceptance's classes) and the proof built from it |
| `review-preparation/tests/fixture_consumer.rs` | every consumption passes the world's proof; 3 new `s1_*` tests |
| `review-delivery/Cargo.toml` | `test-support = ["fixture"]` (not implied by `fixture`); `native_vertical` `required-features = ["fixture", "test-support"]` |
| `review-delivery/src/gate.rs` | `regate_observed` + `RegateInputs` (the regate also returns its fresh observation and time); summary docs |
| `review-delivery/src/native.rs` | `test_support` behind `cfg(feature = "test-support")` + cfg-gated compile_fail doctest; delivery regate revision carried; proof passed to the consumer; absence sentences rewritten (module doc, `run_attempt`, `test_support`, `deliver_and_consume`, `Delivered`) |
| `review-delivery/src/binding.rs` | `consume` takes the proof |
| `review-delivery/src/lib.rs` | spawn routes by feature; "a feature states build intent, not access control" |
| `review-delivery/tests/native_vertical.rs` | positive test asserts both regate revisions and the consumer's `gate`; new `s1_both_regate_revisions_are_carried_and_follow_their_admissions` (ticking source); T5 asserts the `AuthorityExpired` reason |
| `rust/Cargo.lock` | the new dependency line |

Diff stat vs `0fe9dc74` (`git diff --stat 0fe9dc74`): 12 files, +731 / -71 (preparation tests
`common/mod.rs` +259 is mostly the synthetic world).

## Runs (custody; `CARGO_TARGET_DIR=/home/bline/cbm-work/org-r2-core-20261117/target`)

- **Gate `runs/gate-1`** (plan `gate-plan-1.json`): **12/12 passed**, summary sha256
  `304c88eb8b26a70fd785e4d87a844b719bbde0da9718a9164ee9f7e132579acf`, every step `leftovers=0`.
  Steps: toolchain; fmt (5 crates); clippy `-D warnings --all-targets` with fixture, default, and
  with `test-support`; clippy of preparation `--features fixture --lib` alone (the new combination:
  consumer present, S-A without `fixture`); tests: five crates with fixture features **303 passed**
  (`native_vertical` is not in this step now: it requires `test-support`), preparation `--features
  fixture` **41**, default features **85**, authority-store `--test store` **37**, `native-1`
  (`--features test-support --test native_vertical`) **33 passed** (32 + 1 new), run once; final
  `hashes` step. Counts are summed from each step's `test result:` lines; no comparison with
  earlier gates' totals is claimed (different plans).
- **Mutants `mutants/runs-1`** (plan `mutant-plan-1.json`, generated by `gen_plan.py`): 17
  mutants; 16 killed by every mapped test; M6's second test passed (spec defect, below).
  `mutants/runs-2` (plan `mutant-plan-2.json`): M6 re-specified, killed by both tests. Pre- and
  final hashes equal in both runs and equal to the gate's blobs.

## Matrix summary (`s1/MATRIX.md`)

- New claims: S1-P1..P7 killed (other candidate, other subject, delivery boundary, Unknown accepted,
  proof check skipped, `test_support` under `fixture`, delivery regate revision dropped).
- Re-established: M1, M2, M3, M4, M6 (runs-2), M7, M8, M9, M10, M11 killed on the corrected code; M5
  stays retired (replaced claim). No S-B claim was found dissolved, so none was recorded as replaced.
- Under M7 and M8 the consumer's own point check still refused: the tests fail on refusal identity
  and consumer count, not on a consumption.
- Correction of my own mutant spec: in runs-1 I specified M6 to replay only on the first regate.
  Error class: a weakened mutant (narrower than the S-B M6 it re-establishes). Consequence: the gate
  test survived, which on its own would have read as a weak test. The check that would have caught
  it: comparing the spec with S-B M6 (`fresh = &self.replay` on every call) before running. Fixed in
  runs-2; the runs-1 record is kept (`M6-.../mutation-runs-1.json`).

## Inventory rows (acceptance frame, S-B design review §4)

Inputs the consumer gained (review-delivery gained no input; it builds the proof from its own
admission and regate):

| Input | Handling |
|---|---|
| `proof.admission` | **S-A-issued**, unforgeable outside S-A; its request identity is compared with the consumer's own inputs: candidate digest **digest-bound** to `candidate.content_digest()`; subject compared with the declaration's subject (itself digest-checked against the episode state); vantage compared with the **consumer-derived** serving vantage; boundary kind must be `Consumption`. Its organization id and declared actor identity remain S-B's labelled caller assertions (`caller_asserted_unanchored`) and are not re-checked here |
| `proof.regate_observation` | **S-A-sealed** observation of the authority source (synthetic in fixture standing, as already labelled in S-B), bound by S-A to the admission's boundary. Freshness is not checkable by the consumer: on the review-delivery path it is **host-derived** (`HeldAdmission` observes on every regate; M6 runs-2); for any other caller it is a caller assertion labelled in the `ConsumptionProof` documentation only (no runtime label) |
| `proof.regate_now_millis` | **labelled caller assertion** `GATE_TIME_ASSERTED_LABEL = "caller_asserted_regate_time"`, carried to the success output (`FixtureConsumption.gate.time_label()`); on the review-delivery path host-sampled from the synthetic `TimeSource` |

## Deviations

- `native_vertical` now requires `test-support`, so the five-crate fixture step no longer runs it;
  the native suite runs once, in `native-1`, with `test-support`. The delivery tests therefore run
  in a build in which `test_support` also exists (it adds a module only; the gated path is unchanged).
- The vantage comparison (1f) has no negative test and no mutant: no admissible other-vantage world
  was constructed.
- The consumer does not check the boundary *operation* (derived from the binding by review-delivery,
  S-B T7): it has no binding. "This binding" is checked as candidate, subject, vantage and boundary
  kind.
- The observation-freshness label is documentation only (see inventory).
- A compound shell command was refused by the permission layer after its first step had already
  edited `fixture_consumer.rs`; I applied the remaining edits with the edit tool and checked by
  search that each new test exists once. No other effect.

## Unfinished

- No independent review; no checkpoint/freeze (packet: no commit).
- Optional item 5 done (T5 reason), not mutated.

## Explicitly unclaimed

- Fixture standing only; S-A records, source and time are synthetic.
- Assumed mapping discharges (O7) unchanged.
- O10 single-use gap unchanged (the same binding reserves the same boundary identities; the proof is
  not single-use).
- Organization id caller-asserted and unanchored; declared actor identity/installed digests (O8).
- No hosted path, storage, issuance; production admission refuses everything.
- A feature is a statement of build intent, not access control: any dependent can enable
  `test-support`.
- The proof binds "an S-A admission for this candidate, subject, vantage and a consumption boundary,
  found current by S-A over the supplied observation and time". In a build with S-A's `fixture`
  feature any caller can build its own synthetic world and obtain such an admission; the proof does
  not show that review-delivery's gate produced it, and it does not carry review-delivery's finality
  of a settled regate (S-A F-3). Freshness of the observation is the host's obligation.
- compile_fail doctests assert non-compilation only, not the error code (N-1 class).

map_ready
