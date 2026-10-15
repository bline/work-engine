# SC3-D domain implementation and validation

This records the bounded schema-4 campaign remediation implementation against
[`rust-campaign-sc3-remediation-plan.md`](rust-campaign-sc3-remediation-plan.md),
SHA-256 `4cd4aee887613a76e13db78551b98d22b479397f3f3c50202d7eedde52415b26`.
The user authorized the plan and RP0/SC3-D parallel implementation. The
schema-4 root is fresh and private; existing schema-3 roots remain initial-only.
No provider call, credential access, retained live session, host integration,
activation, staging, commit, or publication is part of this receipt.

## Domain boundary

`TrustedConfig::remediation_v4` binds the accepted plan, initial identity,
baseline, workspace, writer, and exact initial gate evidence to a new profile.
The initial episode/CE join remains unchanged. A remediation round then binds
an immutable changed-tree checkpoint, physical profile, same-tree gate, freshly
accepted specialist selection, retained-session reference, initial outcome,
current episode result, and generation. The campaign retires the old request
slot at preparation, registers one dispatch, and returns an Applied-only entry
permit. Replay and recovery return records and original locators without a new
permit.

The episode reader admits a post-remediation subject or result only through the
original authority and the complete native subject-transition lineage. Round
completion registers and reconciles CE observation, establishment, admission,
and episode read effects. An unestablished pair yields an explicit unestablished
outcome. An established pair publishes exact finding revisions and a fresh
projection, or a campaign-owned zero-finding proof. Prior native finding IDs
must all remain present; CE revisions use predecessor CAS and preserve stable
claim identity and the initial revision reference. Builder dispositions publish
fresh reliance and projection; aggregate validation rechecks every earlier
committed exact reliance. Terminalization consumption remains unused.

## Evidence and interpretation

All native tests use a real temporary episode store, CE store, immutable Git
checkpoint, physical profile, and exact-tree gate execution. The E1 result and
retained-session reference are deterministic fixture records. They qualify the
domain join and refusals, **not** an actual retained provider session or SC3-H
host/provider-entry witness. A live provider and private retained capability
are separate future authority and validation requirements.

The isolated Cargo target and raw logs are under
`/home/bline/code/.work-engine-tmp/sc3d-20261013/`. Cargo operations using
that target are serial because native tests execute their exact test binary as
a child for episode writes. A concurrent rebuild can replace that binary and
produce a false child-execution failure.

The raw receipts below cover the implementation and its owner boundaries. The
focused case logs precede the final small Clippy-driven representation change
(`RemediationStageOutcome::Ready.stage` became boxed); the full package suites
exercise that final source.

| Check | Raw receipt | Current observation |
| --- | --- | --- |
| Schema-4 initial native join | `raw/schema4-flow-05.log` | Initial episode/CE owner join passed. |
| Incomplete unestablished changed round | `raw/schema4-unestablished-corruption-01.log` | Native incomplete result with limitations/questions, absent CE observation, explicit unestablished outcome, recovery/replay, unconsumed terminalization, and corrupt publication receipt refusal passed. |
| Established zero-finding and second generation | `raw/schema4-second-generation-02.log` | Fresh no-findings projection, replay, and preparation of generation 2 against a distinct immutable tree in the same workspace passed. |
| Retained two plus new one | `raw/schema4-retained-new-final-02.log` | Historical initial evaluation, exact retained revision CAS, new UTF-16 native ID, dropped/rewritten prior ID refusals, fresh projection, and two builder dispositions passed (159.03 seconds). |
| CE `false` versus absent observation | `raw/schema4-false-02.log` | A present observation with unauthorized mutation produced two explicit `false` admissions and an unestablished campaign outcome; the absent-observation case remains separate. |
| Complete 100 / refuse 101 | `raw/schema4-positive100-release-final-source.log` | Final-source exact observed 100-finding native/CE publication and 101 refusal before campaign/CE completion effects passed (272.95 seconds). Earlier equivalent release run passed in `raw/schema4-positive100-release-final.log` (273.73 seconds). A preceding Cargo-wrapper run ended with SIGTERM 143 and no result; `raw/schema4-positive100-release-01.log` preserves it, followed by a successful direct-binary retry in `raw/schema4-positive100-release-direct-02.log`. |
| Native episode remediation read | `raw/episode-remediation-read-final.log` | Native transition lineage admitted; initial-only reader and forged authority/lineage rejected. |
| Fresh schema-4 profile | `raw/remediation-profile-final.log` | Initial admission and cross-profile open refusal passed. |
| Existing schema-3 R2 reliance guards | `raw/schema3-r2-regression-final.log` | Changed, missing, and unreadable older exact reliance admissions all refused (3 passed). |
| Existing schema-3 J2 dispositions | `raw/schema3-j2-evaluation-final.log` | Exact findings and both builder decisions passed. |
| Existing schema-3 J2 fault cuts | `raw/schema3-j2-fault-recovery-final.log`, `raw/schema3-j2-fault-recovery-release-final.log` | All ten existing outcome/evaluation cuts recovered original owner identities in debug and release; release passed in 31.49 seconds. |
| Schema-4 fault cuts | `raw/schema4-fault-cuts-01.log`, `raw/schema4-fault-cuts-release-final.log` | Ten preparation, dispatch, completion, CE read, and publication cuts passed in debug (463.15 seconds) and release (62.24 seconds); original slot/effect IDs and no new dispatch permit held. |
| Full episode checked readback file | `raw/episode-sc2-readback-final.log` | All 5 tests passed, including the original initial-only reader and the new native remediation lineage reader. |

## Final gate receipt

`cargo clippy -p slice-campaign -p review-episode --all-targets --locked --
-D warnings` passed in `raw/clippy-final-03.log`. The two preceding Clippy
receipts (`-01`, `-02`) found a large enum variant and an unnecessary explicit
test-fixture drop; both were fixed. `rustfmt --edition 2024 --check` over the
owned Rust files and `git diff --check` passed. The complete
`cargo test -p review-episode --locked` passed all 41 non-ignored cases in
`raw/review-episode-full-final.log`. `cargo test -p slice-campaign --locked --
--skip schema_four_preserves_complete_100_and_refuses_101_before_ce_effects`
passed all 53 non-ignored cases in
`raw/slice-campaign-suite-except-debug100-final.log`, exercising the remaining
package suite on the final source. An unskipped debug suite attempt
under `raw/slice-campaign-full-final.log` passed every reported case except
the unfinished 100-finding case; that case remained CPU-bound for more than
30 minutes and was terminated deliberately. This is a material debug-time
cost, not a passing full-suite claim. The exact capacity case is separately
qualified in release on the final source.

Both schema-4 and existing J2 fault-cut tests passed in debug and release
with `--features test-faults`; ordinary package tests omit that feature. Exact
final file hashes and command receipts are held under the isolated evidence
directory, including `source-sha256.txt`.
SC3-D cannot exercise SC3-H provider reservation, live retained session
custody, or host acknowledgement cuts. Those remain separate implementation
and proof obligations rather than claims from these fixture tests.

## Retained-review R1–R3 remediation

The retained fresh-entry Astra report at
`/home/bline/code/.work-engine-tmp/sc3d-astra-review-20261013/review-report.json`
(SHA-256 `551fffeeb99bd58f9ee3e9fa1ccabb6781430f7bf912ace7f5371e2c6d2d42bf`)
blocked the original 16-file subject on three P1 findings. Its original
archive (SHA-256 `5541e1ba5eb9a822138bc0df2901db7257ff147173145dc091d0ca39850b2d28`)
is immutable. This overlay changes only `remediation.rs`, `owner_join.rs`,
and this validation record.

Each finding first reproduced against the pre-fix source in
`retained-review/raw/`: `r1-prefx-02.log` admitted a legitimate generation-2
request after a reported nonempty result with two `verified_resolved`
retained findings; `r2-prefx.log` returned a finalized replay after its exact
CE finding operation admission was deleted; `r3-prefx.log` accepted a
consistently substituted initial session reference and dispatch session.
The preliminary `r1-prefx.log` stopped at R3's independent assertion, then
the isolated R1 rerun above reached its own path. These are expected failing
regressions, not qualification passes.

The repair checks awaiting-builder or reported-empty eligibility at every
preparation generation and matches the native current result to the exact
predecessor outcome. It binds owner, root, initial attempt, and session to
the initial checked execution, and the full retained reference to the first
round across later generations. Deep campaign history validation rechecks
those lineage rules. Finalized replay now reconciles every exact CE finding
publication and evaluation reliance, verifies the saved projection digest
and immutable exact revisions through a fresh CE projection, and refuses
missing, conflicting, or unavailable owner evidence. A later generation's
active slot does not make an older outcome a fresh finalized replay; the
older committed outcome remains recoverable as historical campaign state.

The repaired R1/R2/R3 native cases passed. Eligible awaiting-builder and
reported-empty predecessors each permitted generation-2 preparation; these
cases do not establish generation-2 dispatch or completion. Finalized
same-round replay also passed. The replay cases prove refusal for deleted,
conflicting, and unavailable CE finding admissions, and for a deleted
evaluation reliance admission after two builder dispositions. The restored
exact evidence replays successfully. The retained session substitution is
refused before effects in both the first changed round and a later generation.

### Retained-review gate receipt

All raw logs remain under
`/home/bline/code/.work-engine-tmp/sc3d-20261013/retained-review/raw/`.

| Check | Raw receipt | Current observation |
| --- | --- | --- |
| Pre-fix R1, R2, R3 reproductions | `r1-prefx-02.log`, `r2-prefx.log`, `r3-prefx.log` | Each failed its expected refusal assertion against the frozen pre-fix source. `r1-prefx.log` first stopped at the independent R3 assertion. |
| Repaired eligibility and predecessor binding | `r1-fixed-01.log`, `r1-eligible-awaiting-builder-01.log`, `r1-r2-eligible-awaiting-builder-02.log` | Reported nonempty resolved-retained generation 2 refused; eligible awaiting-builder generation-2 preparation admitted. This case does not dispatch or complete generation 2. |
| Repaired finalized evidence replay | `r2-fixed-01.log`, `r1-r2-eligible-awaiting-builder-02.log` | Missing, conflicting, and unreadable CE finding admissions and missing disposition reliance admission refused; restored evidence replays. |
| Historical versus active round | `r2-historical-reported-empty-02.log` | A later active slot refuses fresh finalization of an older round; historical committed recovery remains available. The preliminary `-01` log expected the wrong active-slot behavior and was corrected. |
| Original retained session | `r3-fixed-01.log` | Consistent session and dispatch substitution refused for both first and later changed rounds; valid unchanged reference admitted. |
| Full affected campaign suite | `campaign-suite-except-debug100-01.log` | All 56 non-ignored tests passed with only the CPU-heavy debug 100/101 case filtered; the release capacity case below covers that exact boundary. |
| Exact 100/101 capacity | `capacity-release-01.log` | Native and CE complete 100 accepted, 101 refused pre-effect in release (253.52 seconds). |
| Rust lint | `clippy-01.log` | `cargo clippy -p slice-campaign -p review-episode --all-targets --locked -- -D warnings` passed. |
| SC3-D fault recovery | `sc3-fault-release-01.log` | All ten exact crash cuts passed in release, preserving original slot and dispatch custody. |
| Existing J2 fault recovery | `j2-fault-release-01.log` | All ten outcome/evaluation crash cuts passed in release (29.20 seconds), preserving original owner identities. |

The original final-source debug and release fault-cut receipts above remain
frozen. The overlay's changed pre-effect checks and replay evidence run on the
affected campaign suite and release fault paths. The original 16-file archive
and receipts are unchanged; the overlay source and raw-log hashes are separately
frozen under `retained-review/` after these checks complete. Fixture sessions
still do not establish private live retained-session custody or SC3-H host
entry authority.
