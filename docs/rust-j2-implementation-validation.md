# J2 implementation and validation

Implementation work started 2026-10-13 approximately 03:01 UTC under the user's
explicit J2 authorization and Astra/high selection, alongside independently owned
H0/H1 and lifecycle work. This report describes source and tests, not review or
campaign acceptance, provider attestation, publication, deployment, or migration.

## Implemented boundary

The permanent `slice-campaign` owner now composes initial publication and exact
builder evaluation through the existing campaign, CE3, ER1 and execution-evidence
owners. J1 `complete_initial` still returns `OwnerReadsJoined`. Added APIs are
`finalize_initial_completion`, `recover_initial_outcome`, `locate_initial_outcome`,
`prepare_builder_evaluation`, `complete_builder_evaluation`,
`recover_builder_evaluation`, and `locate_builder_evaluation`.

`InitialOutcomeLocator` and `BuilderEvaluationLocator` are serializable,
authority-free descriptions of immutable campaign registrations. Checked outcome
construction is private and cannot be deserialized. Optional
`CompletionSelection::new` plus `Campaign::install_completion_selection` is a
trusted startup capability, bound to the existing CE3 owner selection and current
campaign epoch. Evidence-unestablished publication requires no finding authority.
The separate CE2 factory remains refused.

All native values use canonical domain JSON strings, including lone-surrogate
finding IDs, limitations, and unresolved questions. The finding bridge retains
stable episode-based claim identity, exact source integrity, native status and
initial/current revision references. Finding command sets register atomically
before any CE write; evaluation reliance and projection intent register before
reliance. Real CE reconciliation precedes filling confirmed absence. A fresh
exact CE projection gates publication; zero findings use the campaign-owned
`no_findings` witness and never a fictional empty CE projection.

`Reported` remains publication-only. Native phase, verdict and unresolved details
remain visible. Both valid and invalid decisions record real reliance; valid does
not resolve an open finding. All-invalid can promote `AwaitingBuilder` to
`Reported`. The original initial episode and publication receipt remain intact.
Only the builder-projection consumption marker changes at usable publication.
The terminalization marker stays unconsumed. Failed evidence maps to
`FailedWithResult`/`failed`; usable publication maps to `Completed`/`completed`.
The consumed dispatch and active request slot remain the identity/fencing anchor.

Current and historical readers validate registered stage commands, original
admissions, request digests, selected owner/grant bindings, immutable checked
progress receipts, publication request digests and disposition receipts. Reserved
correction remains refused. Existing schema-1/2 roots remain unwritable; schema 3
and its SQL tables are unchanged. No Cargo dependency or lockfile edit was made by
this lane.

## Evidence and bounded limits

Tier 2 graph verification used project `home-bline-code-work-engine`, initially
generation `2026-10-11T22:56:21Z`, store `ude7dece53a17b29eg6`. Exact-name discovery,
a bidirectional `complete_initial` trace, its exact snippet and path coverage were
checked. The graph's first path-filtered search returned no matches; removing the
filter recovered the symbols. Direct source established owner-injected behavior
that sparse static traces cannot establish. Relied-on source paths were checked
again at generation `2026-10-13T03:13:44Z`, store `u6a45c59757e119a8g2`, without
recorded gaps. Final coverage included the eleven campaign/test/evidence paths
at generation `2026-10-13T03:38:49Z`, store `u5d6e91a4dd94b81eg4`, and three
additional CE/native-store paths at `2026-10-13T03:48:53Z`, store
`u5c6fd2ed35bb052fg3`, all content-verified with no recorded gaps. Clean
coverage is best-effort evidence, not completeness proof.

The accepted fixed controlled-fixture owner proves established and negative
domain branches. The actual same-UID E1 child proves truthful
`EvidenceUnestablished`; its exact false/unestablished statuses and reasons are
preserved. These tests provide no live-provider or independent read-only proof.

CE's existing evaluator returns `false` only for `mutationAuthorized=true`, which
dominates both claims sharing the exact J1 observation. Mixed false/established or
false/unestablished is therefore unreachable in this initial profile without
altering an owner or falsifying evidence. Reachable established/unestablished
pairs in either order, false/false and unestablished/unestablished are preserved.
This limitation was escalated to the integration owner, not hidden by a permissive
fixture.

The complete initial set is capped at 100 findings and the unchanged campaign
encoded-state limit is 1 MiB. A conservative aggregate preflight reserves space
for subsequent receipts, projection and publication. The tested 100 large native
findings are refused before registration by that byte-capacity preflight; 101
are refused by the count bound. No truncation or partial batch is presented as
complete. Positive publication is exercised for zero, one and multiple findings.

## Validation record

Pinned toolchain: Rust `1.92.0`, Cargo `--locked`, separate target and disposable
roots under `/home/bline/code/.work-engine-tmp`. Logs are retained in
`/home/bline/code/.work-engine-tmp/j2-logs`.

The final production source passed the full optimized serial `test-faults`
suite: **54 passed, 0 failed, 1 existing ignored performance workload**, including
four compile-fail doctests. The subsequent owner-store-unavailability test passed
separately in release and debug (one test each). It adds validation only; no
production source changed after the full optimized run. The final isolated full default suite passed **43 tests, 0 failed, 1 existing
ignored performance workload**, including the additional owner-store-read test
and four compile-fail doctests. Production source remained unchanged throughout
these final gates.

The final checks cover:

- Real E1 kill/reopen at initial registration and pre/post outcome CAS, with
  original-ID locator reconstruction and the independent no-second-execution
  output witness.
- Ten fixed-fixture kill/reopen cuts: outcome/consumption registration, finding
  commit, initial projection read, pre/post outcome CAS, evaluation registration,
  reliance commit, evaluation projection read and pre/post evaluation CAS.
- Four downstream-commit/read cuts with deliberately unreadable campaign state.
  Returned in-memory finding, reliance and projection receipts survive, and
  restoring custody permits recovery through original locators.
- Zero, one and multiple exact findings; incomplete empty publication; surrogate
  finding IDs, result text, limitations and questions; distinct claim statuses;
  count/capacity pre-effect refusal; valid/invalid dispositions, opposite replay,
  stale member revision and duplicate disposition refusal; disjoint pending
  evaluations completed in reverse order; preservation of initial receipt.
- Current/historical registration and publication receipt corruption, locator
  digest/root substitution, actual CE store unavailability after a committed
  view, unchanged schema-1/2 refusal, prior history/slot/receipt regressions and
  P1's current-read counter checks.
- Affected-package Clippy, all targets plus `test-faults`, warnings denied; owned
  Rust formatting and `git diff --check`.

Raw logs remain under `/home/bline/code/.work-engine-tmp/j2-logs`:
`final-release-faults.log` includes the named crash ledger and **54** passing
cases; `owner-read-recheck2.log` and `owner-read-recheck-debug.log` contain the
additional owner-read proof; `final-clippy2.log` contains the final lint gate.
`configuration-evidence.json` records an additional paused fixed-fixture
registration capture with exact claims config/source, native result/observation,
registration state, test executable and actual controlled-child executable
hashes. `config-capture-registration-state.json` retains that exact registration.
The capture is configuration evidence, not another recovery or provider proof.

Earlier failed attempts remain in raw logs: incorrect test expectations for the
native phase/status, a helper rejecting intentional replay, a fixture destructor
removing recovery artifacts, and a test that initially attempted private owner
field access. All were corrected in the test implementation. One later debug
suite (`final-default.log`) failed because I rebuilt its running test executable
while a focused test compiled concurrently. Native admission correctly attempted
to read the old now-unlinked executable and received `ENOENT`
(`review-episode-store/src/sqlite.rs::native_selection_value`). This validation
orchestration error required no production change; `final-default-isolated.log`
is the replacement with no concurrent binary rebuild.

These are functional and bounded fault proofs. They do not establish live
provider entry, independent read-only review, general performance, every
possible corruption permutation, or exact numerical acceptance at every byte
size below the unchanged state limit. The 100-item large result is specifically
a capacity refusal proof. No temporary host, permissive admission port,
provider attestation, schema migration, Cargo edit, commit or publication was
introduced by this lane.

## Exact source custody

Source, workspace configuration, toolchain and test executable hashes are retained
in `source-toolchain-executable-manifest.json` under the log directory. The
production source hashes are:

| Path | SHA-256 |
| --- | --- |
| `rust/crates/slice-campaign/src/application.rs` | `ad9d750826ae78bac35fa6d4fc600a62709dc9b82e722e970b57825478a8dac5` |
| `rust/crates/slice-campaign/src/completion.rs` | `eaeb6d8b27cd4eaacee53baff1e2755ff1dd482f646cae13d765de3dcc237f1a` |
| `rust/crates/slice-campaign/src/owner_readback.rs` | `85baf8dd3996c2b52f39345702562d9edcc2020c6f65c2281685982a7d398ddb` |
| `rust/crates/slice-campaign/src/store.rs` | `f089b1114fb4dd967f32ade34c71e78257bb1cf4940449e5d84cf4d9ba5ea921` |
| `rust/crates/slice-campaign/src/lib.rs` | `db19cc13b53c4741005c59c655d06d2b37d68337c88437fa24665449f3ffa101` |
| `rust/crates/slice-campaign/src/initial_outcome.rs` | `61aaa6a9f416b2d27c8ca3daf8cfe4dd9543812ac65d5a83f27dfc7f4eed3162` |
| `rust/crates/slice-campaign/tests/owner_join.rs` | `76c10832e7dfbda3524e88fca941970974ce30c2a1dfb923ccb340821cc1aa0b` |

The manifest was taken from shared HEAD `7068bb029e44e4543e27091fd3c89daab0b5e3c7`; it binds
these dirty J2 source bytes rather than claiming a clean Git subject. Other
concurrent work and its staged/index state are preserved.

## Integration handoff

H1 may use the exported J0 signatures above. Before an initial-completion locator
exists, recovery can use the existing owner APIs: after a successful campaign
read, replay the exact original `Campaign::admit` operation/request and require
`Effect::Replayed`; its original receipt supplies `profile_digest`. Construct the
descriptive dispatch locator from the retained request/dispatch and pass it to
`replay_dispatch` for owner validation. Existing state makes an absent admission
operation return `NoEffect`, never a new admission; no new dispatch permit is
created. This route was source-checked and handed to H1; H1 owns its executable
qualification. No additional dispatch API or shared-owner edit was needed.

## Completion

Completed implementation and validation at 2026-10-13T03:53:36.194725+00:00
(approximately 53 minutes elapsed from startup). Final default owner-join group:
15/15; optimized fault owner-join group:22/22, plus the subsequent owner-store-read
case1/1 in both release and debug. Full package totals above include the required
history/read-count regressions and compile-fail checks. Final report and machine
validation receipt preserve the exact counts instead of summing repeated runs.

No J2 source was staged, committed or published. The parent owns review and any
subsequent publication decision; H1 owns combined host qualification.
