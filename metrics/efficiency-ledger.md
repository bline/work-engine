# Efficiency ledger

Living record of Work Engine development efficiency: what is measured, which
changes are expected to move it, and what measurement showed. The frozen
reference point is the
[pure-Codex baseline](baselines/pure-codex-20261009/README.md) (2026-10-05 to
10-09, Codex on Pro, gpt-6). Its README defines units, assumptions and caveats.

## How to use this ledger

- When a change is intended to move a metric, add or update its lever row
  before or as it lands: target metrics, expected effect and its source.
- Record results only from measurement, with window, workload, script and
  caveats in the log. Label estimates as estimates.
- Judge a lever on outcomes as well as cost. A cheaper run that needs more
  attempts or lets more defects escape is not an improvement.
- Keep the baseline frozen. New reference points go in new baseline folders.

Units: **Plus-unit** = 1% of a 1x Plus weekly Codex quota (Pro = 10x, assumed).
Costs are estimated from token counts at assumed cached and output price ratios.
Sources refer to files in `baselines/pure-codex-20261009/`.

## Metrics

| ID | Metric | Baseline | Latest | Source |
|---|---|---|---|---|
| M1 | Wait-only calls, share of estimated spend | ~11% (3,320 calls); root supervisors 31–32% | — | `scripts/act.py`, `drill.py` |
| M2 | `wait_agent` calls that timed out | 1,097 of 2,080 waits (mostly 30–60 s) | — | log query, see baseline |
| M3 | Root orchestrator share of spend | 23% | — | `scripts/report.py` |
| M4 | Cached-context re-send share of spend (estimated) | ~73% | — | `scripts/drill.py` |
| M5 | Full doctrine reads per session | DESIGN 1.5, PHILOSOPHY 1.7 (167 / 186 over 112 sessions) | — | `scripts/reread.py` |
| M6 | Redundant within-session re-reads, share of spend | ~4.2% (+1.6% re-read-only calls) | — | `scripts/reread.py` |
| M7 | Re-reading after compaction, share of spend | ~4% | — | `scripts/reread.py` |
| M8 | Average context per call | root ~144K, sub-agent ~126K tokens | — | `scripts/drill.py` |
| M9 | Candidate attempts per accepted slice | mean 2.8 (14 slices, range 2–5) | — | `baseline.json` quality |
| M10 | Review findings resolved per slice | ~6 (11 slices with stated counts) | — | `baseline.json` quality |
| M11 | Settled-fact contradictions among independent review findings and contract revisions | ~42% clear / 36% arguable / 22% novel of 36 items, S4–S8 | — | `classification/` |
| M12 | work-engine code lines per Plus-unit | 24 (Rust, W41); 25 Python/Node build-out | — | `scripts/era.py` |
| M13 | Startup chatboard read size | ~107 KB, oldest 100 of 1,503 messages | 19.5 KB, newest 20 (2026-10-09) | `codex_chatboard.py read` |
| **M14** | **Estimated Plus-units per accepted slice** (primary outcome) | lifecycle S4 150, S5 212, S6/S7 368 (joint), S0–S3 355 (four slices); orchestration adds ~25% on top | — | `scripts/report.py` per-slice table |

**M11 denominator.** Each independent review finding or contract/route revision
in S4–S8 counted once; later candidate attempts, bundled multi-change revisions,
revisions dependent on an earlier item, and undetermined items are excluded.
Classified blind by an independent model
([results](baselines/pure-codex-20261009/classification/results-chatgpt-s4-s8.md)).
It is the share of those items, not of development effort or quota; some
revisions implement findings from the same slice, so treat it as approximate.

**M14 companions.** Record acceptance coverage (which required evidence was
actually established), candidate and remediation attempts (M9), findings (M10)
and defects that escaped review alongside every M14 value. Fewer findings can
mean better implementation or weaker review; M14 alone cannot tell them apart.

## Levers

Each lever is judged on two separate questions: **was it adopted** (did the
behavior or mechanism actually occur), and **when adopted, did cost and outcomes
improve**. A lever that was not followed has not been tested; do not let that
discredit the underlying optimization, and do not credit savings to a lever that
did not operate.

| Lever | Targets | Expected effect | Adoption check | Status | Reference |
|---|---|---|---|---|---|
| Chatboard `read` shows newest messages; `--active-claims` | M13, M4 | ~22K fewer startup tokens per session; current coordination visible | sessions use the new read command | shipped, M13 measured | `67621bd` |
| PHILOSOPHY.md tiered to design/planning/doctrine work | M5, M8 | ~9K fewer standing tokens for implementation agents | PHILOSOPHY full reads by implementation agents | shipped, unmeasured | `67621bd` |
| Codex orchestration brief: long waits, consequential messages, settled-fact briefs, root handoff, dependency-closure validation | M1, M2, M3, M8, M11, M14 | most of M1/M2 if behavior follows; unknown otherwise | `wait_agent` timeout distribution; messages per sub-agent; briefs containing settled facts | shipped, first test after quota reset 2026-10-14 | `a72ab76` |
| Work Engine wake policy: no inference while waiting, wake eligibility, batching | M1, M2, M3 | ~10 points of spend (structural, central estimate) | turns recorded by wake reason; none while waiting | policy recorded, not implemented | `docs/context-lifecycle-wake-policy.md` |
| Deterministic coordination owners (campaign, claims, scheduler) | M3, M14, agent-messaging spend | ~7 + 3 points (central estimate) | dispatch and bookkeeping performed without model calls | partly built in Rust, not hosting development | baseline scenario model |
| Compiled role packages and event-scoped organizational judgment | M5, M8 | ~4 + 8 points (central estimate) | roles start from compiled packages, not full doctrine | architecture accepted, not implemented | `organizational-compilation.md` |
| Cache-aware accounting and conditional payback (T30) | M4, M7 | ~0–3 points after structural savings; mainly visibility | payback reported per replacement | in S8 coverage, unproven | lifecycle `DESIGN.md` R28/T30 |
| Relevance projection of settled facts into contract formation | M11, M9, M10, M14 | clear-bucket contradictions toward zero without suppressing novel discovery | contracts cite applicable settled facts before freeze | hypothesis | baseline README, `classification/` |
| Doc↔code claim projection in Codebase Memory (`sync_projection`); evidence-anchor impact nomination | M11, M6 | code changes nominate dependent claims and documents, and the reverse | nominations raised for changed anchors | proposal meaning approved, placement probable, not implemented | `proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/` |
| Plan IR and execution-strategy model routing | M12, M14 | ~6 points from routing, ~2 from compression (central estimate) | routed attempts by model and effort | hypothesis, pilot planned | Plan IR idea §13, §20 |

Central estimates come from the scenario model recorded with the baseline
(structural ~40%, with ideas ~56% of today's estimated spend). They are
predictions to test, not results, and they are **not individually additive**:
compiled role packages, relevance projection, doc↔code projection and Plan IR
all reduce repeated acquisition of settled information and overlap
substantially. Only the scenario model's combined figures account for that
overlap.

## Measurement log

| Date | Window and workload | Metrics | Result | Notes |
|---|---|---|---|---|
| 2026-10-09 | Pure-Codex baseline, 2026-10-05 to 10-09, Rust lifecycle and domain slices | M1–M12, M14 | baseline values above | frozen reference |
| 2026-10-09 | Live board after chatboard fix | M13 | 107 KB → 19.5 KB; messages 1484–1503 instead of 1–100 | single read, not a session-cost measurement |
