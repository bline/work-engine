# S1 design recheck: R1's acceptance model

Verdict: `revise_acceptance_model`

Reviewer: Claude session `4be2fcf8-c471-4c25-a454-c3807433ed51` (observed model `claude-fable-5-2`), read-only, 2026-11-16. Advisory; the root owns disposition.

Paths: `N` = `/home/bline/cbm-work/org-pi-review-continuation-20261114`, `P1` = `/home/bline/cbm-work/wt-p1-vertical-20261113`, `D` = `P1/rust/crates/work-engine-review-delivery`, `Q` = `N/realization-r9/evidence-gate`.

## Summary

The product seam is not what keeps being rejected. Of the 21 open findings, 12 concern only the private qualifier and build-input custody, 5 concern product behaviour and have a repair visible in current source, and 4 concern the reliability or timing of negative controls in the test suite.

The claims that generate the rejections are four production-path claims that the root's own plans added after R1 was approved. I found no owner requirement for any of them in the material I read, and the failures they guard against are the hostile-writer and hermeticity cases that every plan since R7 excludes. Two of them cannot converge under the standing holds, for structural reasons given in section 2.

My recommendation:

- **Withdraw the four claims openly.** Record them as unestablished and no longer asserted, and record the 12 machinery findings as superseded with reasons, not closed.
- **Accept R1 on three things:** a committed subject, recorded test runs at that subject, and one fresh semantic review of the product seam.
- **Confirm with the operator first.** The S1 item text in `/home/bline/cbm-work/org-pi-slices-20261116/assignment.json` names the R10 route and the 20 review obligations, and that record is marked `limits_set_by: user`. Whether the operator wrote those items is not established.

## 1. Which claims R1 requires, and who owns each

The owner-approved basis is the R1 section of `/home/bline/cbm-work/org-codex-recovery-plan-20261113/PLAN.md` (lines 33–46). It asks for semantic consequences that tests can observe, and it says "A packet hash proves identity, not satisfaction."

| Claim R1 requires | Owner | Evidence that fits |
|---|---|---|
| Compiled meaning reaches the runtime boundary: the text the SDK saw at the request boundary and holds in history equals the frozen contract | Organization and preparation for meaning; delivery for byte correspondence (`BoundAttempt::verify`, `D/src/lib.rs:273`) | Positive native test, pure refusal tests, semantic review |
| A changed requirement produces the matching instruction, enforcement or refusal | Preparation (`prepare`) | `P1/rust/crates/work-engine-review-preparation/tests/preparation.rs:195–296` (read) |
| The supplied runtime is checked against compiled requirements; the provider mismatch stays unsatisfied | Preparation's realization evaluation (`realization.rs:77–105`, read) | Preparation tests (names read, bodies not) |
| A result is validated by implementation-review/v1 against the exact subject; a wrong contract or subject is refused | `review-episode-core` (`validate_result`, `reducer.rs:105–138`, read) through `consume_fixture_result` | Consumer tests and native consumer-refusal controls |
| Missing or unestablished evidence cannot advance the episode | Fixture consumer gate (`fixture.rs:531–590`, read) and reducer | Consumer tests |
| One fixture path through the same owners, with no second route to consumption | Delivery (`ExperimentBinding`, `deliver_and_consume`, `D/src/native.rs:1264`) | Native positive test, binding tests |
| No authority is minted; ordinary-admission refusals stay | Organization and authority-store owners | Existing tests unchanged, review |
| The tests and the review are about the revision being accepted | Root, as acceptance owner | Actor recording at a committed tree |

The last row is the only production-path claim R1 acceptance needs. It is structural under §3.9 because accepting revision X on tests of revision Y is a failure nameable without reference to any plan. Recording is sufficient for it, because the standing boundary is actor-attributed.

The four claims the current plans impose, tested against §3.9 condition 2:

| Imposed claim | Another owner requires it? | Consumer failure it prevents | Judgment |
|---|---|---|---|
| Complete material build-input inventory | None found (basis below) | A binary that differs from what the tree would build because an unlisted input changed. That is the hermeticity and hostile-writer case, excluded by PLAN-R7, R9 and R10 | Not structural. Unrealizable under the holds |
| Executable custody joins (five retained executables matched to Cargo output and direct runs) | None found | "Ran a different binary than was built." The gap exists only because the route builds, retains and directly executes binaries as separate steps | Route ceremony; it goes with the route |
| Qualifier re-appraisal of every emitted artifact | None found | Retained files that contradict what the test asserted in-process. That needs either a bug in the capture code, which exists only to feed the qualifier, or a post-write substitution, which is excluded | Not structural. Its only consumer is its own gate |
| Cold/common build reconciliation | None found | An unqualified verifier binary checking the qualifier's custody | Third-order machinery; drop |

Completeness basis for "none found": I read all of `PLAN.md`, `N/ARCHITECTURE-GUARD.md`, the operator grant text quoted in `N/ROOT-R19-STOP.json`, `N/CONTINUATION.md`, `N/PLAN-R7.md`, `N/PLAN-R9.md`, `N/PLAN-R10.md`, the R10 decision and the S1 `assignment.json`. The grant text is a budget and a scope ("review/remediation and continued development"). I did not read operator transcripts (excluded), `PLAN-R8.md`, the earlier campaign's plans R2–R6, or the rev7 handoff. Absence is established for the read set only.

Two passages show the claims are self-referential:

- `Q/src/outputs.rs:1–15` describes the qualifier as rebuilding `ExperimentBinding::new` and re-running `BoundAttempt::bind`/`verify` and `consume_fixture_result` over retained files, and says "It is not a newly observed native execution."
- The CR9-001 report says its counterexample "does **not** show invalid input reaching the consumer. It exposes a qualifier correspondence gap."

This matches the anti-pattern in `docs/contracts/system-design.md` §14: "a production-path claim whose only consumer is the constraint that requires it." It also meets the §1.5 test: the requirement can be violated while every protected R1 outcome stays valid.

Withdrawing a claim that no owner requires is a route revision (`core.md` §10), not the narrowing §3.9 forbids. It stays truthful only if the record says the stronger claims are unestablished and that acceptance does not rest on them.

## 2. Is the R10 local correction credible?

No. Three separate lines of evidence point the same way.

**The recurrence rate is not falling.** Counted from the event list in `N/architectural-risk-history.json` (revision 37):

- The complete-appraisal invariant recurred at six successive review events: IR-001, IR-004, R6A-002/003, WR7-002, QR8-001/002, CR9-001/002.
- The build-input binding invariant recurred at eight: RISK-001, RA-001, ARTIFACT-001, META-001, RLA-005, RL-003, R9D-001, AR10-004.
- New findings in the last three rounds were 2 (R7), 3 (R8) and 3 (R9), plus two new prerequisites from the R10 design review. Each of those reviews ended partial, so these are lower bounds.

**The history records eleven design diagnoses between candidate2 and R10, each concluding retain or amend locally** (n=11 by event `kind`; I read only the last in full). The one I read, `N/architecture-r6/REVIEW-MESSAGE-73.md`, treats the claims as fixed: "Omitting the input from the claim would silently narrow acceptance." It did not ask who requires the claim. Whether the earlier ten had the same premise is not established.

**Two of the claims cannot be established by the permitted mechanisms.**

- Complete build inputs quantifies over every byte Cargo, rustc, build scripts and the linker read. Without independent observation (tracing) or constraint (a hermetic sandbox), only a declared superset remains, and a declaration cannot establish its own completeness. PLAN-R10 item 3 states both halves: "Neither an archive hash nor an empty uncovered list alone proves the bytes actually read were bound" and "Do not introduce kernel tracing, hostile-writer protection, hermeticity". R9D-001 is the first instance of an open class.
- AR10-004 shows the same claim needs reads of protected configuration. `N/R10-REALIZATION-ASSESSMENT.md` says of its own alternative: "That candidate is not ready for execution." Under §3.9 an unrealizable claim is reported to its owner, not admitted as executable.
- Complete applicable appraisal is a totality claim over the relationships among retained artifacts, realized by hand-written predicates. One missing predicate falsifies it, and each repair adds artifacts (`failure.json` and scratch listings arrived with R9), which adds relationships.

The surface is already larger than the product. By `wc -l`: delivery source 2,698 lines and its tests 3,577; qualifier source 5,409 and its tests 3,554; campaign Python 2,163.

Evidence that would show convergence, none of which R10 offers:

1. A predicate set generated from a schema of every artifact field, not hand-enumerated. R10 declines this ("a generic workflow engine … is unnecessary").
2. A pre-registered mutation campaign over that set with consistent hash updates and zero surviving semantic mutants.
3. Two consecutive complete, not partial, fresh reviews with no new instance.
4. For inputs, an independent observation mechanism, which the holds exclude.

## 3. Minimal truthful acceptance model

**Subject.** One private checkpoint commit of P1 (commit and tree OID) containing the crates on the path, `Cargo.lock`, the adapter and fixtures. External inputs are named, not attested: the Pi SDK root with its handoff manifest digest and entry digest, and `rustc`, `cargo` and `node` version strings.

**Test evidence.** From a clean export of that tree, one `custody.py run --plan` that records the tree OID, runs `cargo test --locked` for the organization, preparation, delivery and episode-core crates with the `fixture` feature, and records the tree OID again. Cargo builds and runs the test binaries in one step, so no build-to-run join is needed. Run the native suite three times, since RL-001 and RL-002 were intermittent.

**Discriminating-check matrix.** A short table mapping each of R1's five discriminating checks and the cross-module requirement to named tests. For each, one recorded mutant: remove the product guard in a scratch copy and show the mapped test fails. This turns "do the tests discriminate" from a reviewer judgment into a mechanical check, at about six guard sites.

**Independent semantic review.** One fresh xhigh reviewer on the frozen commit, covering:

- the R1 bullets, Exit and discriminating checks;
- the five product findings in section 4;
- label truthfulness (`FixtureObservationOnly`, `NotEstablished`, no authority minted);
- the matrix.

Custody and the qualifier are out of scope. Independence claimed is a separate session and context, nothing more.

**Explicitly unclaimed.** The acceptance record should say each of these in words:

- Fixture standing only: synthetic declaration, admissions and result; no live provider, model quality or real review.
- No `AdmittedOrganization` or `DispatchAuthorization`; preparation blockers and the realization mismatch remain.
- No hermetic or reproducible build and no complete build-input closure. The binary is whatever Cargo built at the recorded tree under the ambient registry, toolchain and configuration.
- No tamper evidence for retained logs; they are actor-recorded.
- No confinement of the SDK child. The descendant check is one point-in-time sample.
- No exhaustive SDK audit. "Provider-disabled" rests on the adapter's stream replacement plus zero counters, empty tools and one request in the observed run.
- No timing guarantee, and no claim that the peer consumed a sent frame (pipe acceptance only).
- Private readiness, not integration.

The R9 qualified run stays as attributed historical evidence. It is not the acceptance basis.

## 4. Disposition of the 21 findings

None is closed by this review. "Superseded" means the claim the finding was measured against is withdrawn and recorded as unestablished.

| Finding | Class | Disposition |
|---|---|---|
| IR-004, R6A-002, R6A-003, WR7-002, QR8-001, QR8-002, CR9-001, CR9-002 | Qualifier appraisal | Superseded: complete appraisal of retained artifacts is no longer asserted |
| R8N-001 | Test capture for the qualifier | Superseded with the bundle. Keep raw frames in failure output as ordinary diagnostics |
| RLA-005, RL-003, R9D-001 | Build-input and tool custody | Superseded: complete input closure is unestablished and unasserted. AR10-004 lapses with it |
| IR-005 | Product: trailing frames must refuse before consumption | Repair visible: `D/src/attempt.rs:425–431` and `completed()` at `D/src/native.rs:1245`. Confirm by test at the S1 subject and by review |
| WR7-001 | Product: duplicate JSON members | Repair visible: `D/src/wire.rs:54–64`, used at `D/src/lib.rs:474`. Confirm as above; see P-3 |
| R6A-001 | Product: deadline owner and collection | Repair visible: single-owner poll loop, `D/src/native.rs:581–843`. Confirm as above |
| RLA-002 | Product: wait failure suppressed cleanup | Repair visible: `D/src/native.rs:583–604`. Confirm as above |
| RLA-003 | Product: elapsed time measured too early | Repair visible: `D/src/native.rs:877`. Confirm as above |
| RL-001, RLA-001, RLA-004 | Negative-control reliability | Open, not R1-blocking. The reports say they show "required-control unreliability, not false production send accounting or unsafe consumer invocation". Three clean suite runs are the evidence; a control that stays intermittent is fixed or removed |
| RL-002 | Wall-clock bound breached | Open question: no owner requires a timing bound. If none does, remove the wall-clock assertion instead of tuning it |

Counts: 12 superseded, 5 product with repair visible, 4 open on controls and timing.

The RL and RLA dispositions rest on finding headlines and the history record. I did not read those reports in full.

## 5. Bounded S1 scope

Scope, in order:

1. Operator confirmation of the revised model (one line).
2. Root appends history revision 38: this report's path and digest, the four withdrawn claims marked unestablished, the 12 superseded ids with reasons, `findings_closed: []`.
3. One builder repairs P-1 and P-2 below, removes bundle production from the acceptance path, writes the matrix with mutant evidence, and commits a private checkpoint.
4. Root runs the recorded test plan three times from a clean export.
5. One fresh semantic review, then remediation with the same reviewer retained for the recheck.

The qualifier, the preparation scripts and `verifier_bootstrap.py` are frozen as history and not maintained.

| Line | Estimate (proxy USD) | Basis |
|---|---|---|
| This recheck | 8 | Packet target; actual is the root's to read |
| Builder | 6–9 | R9 repair cost 3.30 on a different model for a larger change |
| Root qualification and coordination | 3–5 | Mechanical; no private verifier |
| Semantic review | 14–18 | Reviewer A's comparable scope cost 8.64 unfinished, on a different model, with an SDK audit this model does not claim |
| Remediation and recheck | 6–8 | — |
| Closure | 3 | — |
| Total | 40–51 | |

These are estimates and unverified: the record I read holds no Fable cost for these task types. Reprice the review line from this recheck's actual cost before dispatch. A stop rule fits: if less than 10 remains after the first review round, stop and report.

The R10 route does not fit. Its own assessment priced it at 50 with the realization route not ready, and the prior window spent 24.25 on three unfinished reviews with a fourth unlaunched.

## Product-code observations

I found no product-semantic defect that blocks R1 in what I read.

- **P-1, repair in S1.** 12 of the 21 native tests call `run_env()`, which panics unless `WE_RUN_DIR`, `WE_INVOCATION_ID`, `WE_SOURCE_MANIFEST_SHA256` and `WE_MAP_SHA256` are set (`D/tests/common/mod.rs:257–271`; call sites in `D/tests/native_vertical.rs` at 422, 445, 547, 579, 612, 636, 710, 913, 967, 2069, 2125, 2175). R1's cross-module check cannot run outside the private pipeline.
- **P-2, repair in S1.** `SDK_ROOT` is a hardcoded absolute path (`D/tests/native_vertical.rs:39`). The pinned digest at line 40 covers the entry file only, not the SDK tree. Make the root an explicit input and say what the pin covers.
- **P-3, question for the reviewer.** `Allocation` and `NativeObservation` decode without `deny_unknown_fields` (`D/src/lib.rs:176`, `195`; decoded at `485`, `497`). A frame with extra members counts as permitted success and the members are dropped. Same class as WR7-001, lower consequence.
- **P-4, design debt, not R1-blocking.** The 17-variant `Control` switch lives in `D/src/native.rs:57–89`, and the delivery driver emits a wrong session or duplicate start under it (`984–999`). `AttemptDriver` is already the seam for test-supplied drivers; move this before R2 or R3 reuse the path.
- **P-5, note.** After an unestablished reap the guard is disarmed and the child handle dropped (`D/src/native.rs:872`), recorded as `NotReaped` or `WaitFailed`. Consistent with no confinement claim.
- **P-6, not established.** Whether `ReviewResult::parse` refuses a result with another contract version; I did not read `implementation_review_v1.rs:26–115`.

## Confidence and coverage

- Claim ownership (section 1): high within the read set; the limit is the unread owner material listed there.
- Non-convergence (section 2): high for the build-input claim, which is a structural argument; medium-high for appraisal, which rests on the recurrence record.
- Product seam: medium. Reviewer A's R9 report independently found no blocking product defect.
- RL and RLA dispositions: low to medium.

Read in `D` and neighbours: `lib.rs`, `binding.rs`, `wire.rs` whole; `attempt.rs:180–433`; `native.rs:1–260` and `440–1294`; `fixture.rs:1–44` and `376–648`; `reducer.rs:100–144` and `250–309`; `realization.rs:1–180`; `preparation.rs:195–316`; `native_vertical.rs:1–236`; `common/mod.rs:236–355`; `outputs.rs:1636–1725` plus its function list.

Not read: `attempt.rs:1–179`, `native.rs:260–440`, `prepare.rs`, `select.rs`, the organization crate, the adapter, most test bodies, `gate.rs`, `verifier_bootstrap.py`, the R7 and R8 reports, architecture reviews r3–r5, the root R7–R9 decisions, and all telemetry.

Not established by me: that the P1 worktree still equals checkpoint `2617a6c6` (git commands were unavailable), and the packet's $280 total.

Process: no builds, tests, network or git. The startup board read was attempted and denied by this session's permission mode, so it was not performed. No context-pressure listener was established; this was one bounded turn. No process of mine remains.
