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
Sources given as `scripts/...` refer to files in
`baselines/pure-codex-20261009/`; later measurement scripts are named by
their full path under `metrics/scripts/`.

## Metrics

| ID | Metric | Baseline | Latest | Source |
|---|---|---|---|---|
| M1 | Wait-only calls, share of estimated spend | ~11% (3,320 calls); root supervisors 31–32% | — | `scripts/act.py`, `drill.py` |
| M2 | `wait_agent` calls that timed out | 1,097 of 2,080 waits (mostly 30–60 s) | — | log query, see baseline |
| M3 | Root orchestrator share of spend | 23% | — | `scripts/report.py` |
| M4 | Cached-context re-send share of spend (estimated) | ~73% | 52–71% of weighted cost per call (9 sessions, 10-07 to 10-13) | `scripts/drill.py`, `metrics/scripts/cost_decomposition.py` |
| M5 | Full doctrine reads per session | DESIGN 1.5, PHILOSOPHY 1.7 (167 / 186 over 112 sessions) | — | `scripts/reread.py` |
| M6 | Redundant within-session re-reads, share of spend | ~4.2% (+1.6% re-read-only calls) | — | `scripts/reread.py` |
| M7 | Re-reading after compaction, share of spend | ~4% | — | `scripts/reread.py` |
| M8 | Average context per call | root ~144K, sub-agent ~126K tokens | 76–140K across 10 sessions (10-11 to 10-13) | `scripts/drill.py`, `metrics/scripts/session_profile.py` |
| M9 | Candidate attempts per accepted slice | mean 2.8 (14 slices, range 2–5) | — | `baseline.json` quality |
| M10 | Review findings resolved per slice | ~6 (11 slices with stated counts) | — | `baseline.json` quality |
| M11 | Settled-fact contradictions among independent review findings and contract revisions | ~42% clear / 36% arguable / 22% novel of 36 items, S4–S8 | — | `classification/` |
| M12 | work-engine code lines per Plus-unit | 24 (Rust, W41); 25 Python/Node build-out | — | `scripts/era.py` |
| M13 | Startup chatboard read size | ~107 KB, oldest 100 of 1,503 messages | 19.5 KB, newest 20 (2026-10-09) | `codex_chatboard.py read` |
| **M14** | **Estimated Plus-units per accepted slice** (primary outcome) | lifecycle S4 150, S5 212, S6/S7 368 (joint), S0–S3 355 (four slices); orchestration adds ~25% on top | — | `scripts/report.py` per-slice table |
| M15 | Codebase Memory coverage observations recommending source fallback | 652 of 3,480 (19%), including 195 `metadata_changed` (2026-10-04 to 10-08) | 47 of 792 (6%), all unindexed files, 0 `metadata_changed` | `metrics/scripts/coverage_outcomes.py` |
| M16 | Tool output discarded by harness truncation before the model reads it | not measured | 123 truncations, ~1.40M tokens, 44 of 110 payloads codebase-memory results | `metrics/scripts/output_truncation.py` |

**M11 denominator.** Each independent review finding or contract/route revision
in S4–S8 counted once; later candidate attempts, bundled multi-change revisions,
revisions dependent on an earlier item, and undetermined items are excluded.
Classified blind by an independent model
([results](baselines/pure-codex-20261009/classification/results-chatgpt-s4-s8.md)).
It is the share of those items, not of development effort or quota; some
revisions implement findings from the same slice, so treat it as approximate.

**M15 and M16 scope.** M15 counts path observations inside
`check_index_coverage` results, not calls; one call reports many paths. Its two
values come from different Codebase Memory builds, which is the point of the
metric, but they are not the frozen baseline window: the reference value covers
2026-10-04 to 10-08 and the latest covers 2026-10-11 to 10-13. A fallback
recommendation is not a defect when the file is genuinely unindexed; what M15
watches is fallback caused by a tracked file that cannot be shown to match its
indexed content. M16 counts harness truncation events and the token counts the
harness reports as discarded. Truncation is not a saving: the call that was
truncated is charged in full, and recovering the lost part costs further calls,
so M16 falls correctly by bounding what each request asks for rather than by
splitting one request into several.

**M14 companions.** Record acceptance coverage (which required evidence was
actually established), candidate and remediation attempts (M9), findings (M10)
and defects that escaped review alongside every M14 value. Fewer findings can
mean better implementation or weaker review; M14 alone cannot tell them apart.

## Cost structure

Estimated cost of one model call is
`7.826e-6 * (new_input + cached_input/10 + 5 * output)` Plus-units, from the
gpt-6 coefficient and price ratios the baseline README states. Measured across
nine sessions spanning 2026-10-07 to 10-13, two models, three effort levels and
both reviewer and builder roles, that lands in a narrow band of **0.10–0.20
Plus-units per model call**, split as 14–34% new input, **52–71% cached context
re-send** and 11–22% output
([`metrics/scripts/cost_decomposition.py`](scripts/cost_decomposition.py)).

Two consequences for how levers are judged. First, session cost is close to
`calls × context size`; effort level and task type act on the **number** of
calls, not on the price of a call, so a cheaper session is normally a session
that converged in fewer calls. Second, because standing context dominates every
call, a lever that removes tokens from startup context or shortens a session is
worth more than one that makes an individual answer shorter. This is the
per-call form of M4 and does not replace its spend-share measurement.

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
| Coherent, content-verified index observation in Codebase Memory (upstream snapshot-coherence fixes, local publication 2026-10-11) | M15, M6, M8 | agents rely on the graph instead of re-reading source to establish freshness | share of coverage observations recommending source fallback; `content_verified` and `generation_matches` present and true | shipped locally, adopted, M15 measured; cost effect not isolated | upstream `#2585`, PRs `#2600`–`#2613`, `proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/evidence/` |
| Bounded Codebase Memory requests (output budget, limit, field and path selection, narrow `edge_types`), batched only up to what fits | M16, M6 | graph evidence reaches the model instead of being truncated away, without spending extra calls on paging | truncation events whose lost payload was a codebase-memory result; requests carrying an explicit output budget | recorded in the orchestration brief 2026-10-13, unmeasured | `docs/codex-orchestration-brief.md` |
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
| 2026-10-12 | 10 sessions, 2026-10-11 17:57 to 10-13 05:05 local, context-lifecycle builders plus causal and fresh-source reviews, against 22 reviewer sessions of 2026-10-07 to 10-08 | M15 | source-fallback recommendations 19% → 6% of path observations; `metadata_changed` 195 → 0; `content_verified` true in 745 of 792, `generation_matches` true in 50 of 50 | the one un-confounded result in this window: independent of model, effort and task type. Graph use itself did not change — the earlier reviewers already queried it in 0.19–0.85 calls per exec call — so the new variable is verified freshness, not adoption. Source reading did **not** fall (1.04 and 1.13 reads per exec call in the new reviews, against 0.41–1.02 before): the graph is being used to choose what to read, not to avoid reading |
| 2026-10-12 | Same window, 9 sessions with per-call usage | M4, M8, cost model | 0.10–0.20 Plus-units per model call; cached re-send 52–71% of weighted cost | see Cost structure. The two new reviews cost 3.4 and 8.4 units over 24 and 54 exec calls. The full cohort of 22 reviewer sessions from 10-07 to 10-08 spans 1.3–24.1 units over 10–95 calls, so the new values sit **inside** the earlier range: against the whole cohort rather than a few large contract reviews, this window shows no cost effect at all, in either direction. It is also confounded — those sessions ran at **xhigh** and reviewed contracts rather than causal behavior |
| 2026-10-12 | Same 10 sessions | M16 | 123 truncations across 110 tool results, ~1.40M tokens discarded; 44 of those 110 results were codebase-memory output | `h0_astra_implementation` lost every coverage result it took (13.9K and 23.0K token payloads), so it contributes no M15 observations despite 14 coverage calls. Verified evidence cannot help a session that never receives it |

**Reading Codex rollout logs.** Two mistakes have produced wrong values here and
are now guarded against in
[`metrics/scripts/`](scripts/): a tool call's body is in `input`, not
`arguments`; and MCP tools are invoked *inside* the exec cell as
`tools.mcp__<server>__<tool>(...)`, usually batched in `Promise.allSettled`,
never as separate `function_call` entries. Counting calls by payload name
therefore reports zero graph use however heavily the graph was queried, which
is what an earlier reading of this window concluded before correction. Per-call
usage is `info.last_token_usage`; `info.total_token_usage` is account-lifetime
cumulative and reads about three orders of magnitude high if mistaken for a
session total. Sessions measured while still running give a snapshot, not a
final value: the figures above were taken at 2026-10-12 23:29 local and three
of those sessions were still active.
