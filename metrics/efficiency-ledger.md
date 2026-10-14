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
| M1 | Wait-only calls, share of estimated spend | ~11% (3,320 calls); root supervisors 31–32% | 4.5% of the window; supervisors 17–18% (stricter definition, see Supervision) | `scripts/act.py`, `drill.py`, `metrics/scripts/supervision_profile.py` |
| M2 | `wait_agent` calls that timed out | 1,097 of 2,080 waits (mostly 30–60 s) | 7 of 29 on one supervisor, 0 of 70 on the other; longest wait requested 60 s → 3,600 s | `metrics/scripts/supervision_profile.py` |
| M3 | Root orchestrator share of spend | 23% | 18.4% of 1,012 units (2026-10-12 to 10-13) | `scripts/report.py`, `metrics/scripts/supervision_profile.py` |
| M4 | Cached-context re-send share of spend (estimated) | ~73% | 52–71% of weighted cost per call (9 sessions, 10-07 to 10-13) | `scripts/drill.py`, `metrics/scripts/cost_decomposition.py` |
| M5 | Full doctrine reads per session | DESIGN 1.5, PHILOSOPHY 1.7 (167 / 186 over 112 sessions) | 2.4 whole-document reads per 100 model calls (56 over 2,313 calls, 2026-10-13 22:55Z to 10-14 04:00Z), against 3.3 before; 48 of 51 arrive within 25 s of a context reset | `scripts/reread.py` |
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
| M17 | Tool invocations per model call | not measured | 93% carry exactly one; median 1, max 3 (1,154 tool calls, `s9_native_capture_builder`, 2026-10-13 22:55Z on) | `metrics/scripts/` ad hoc; see Per-call cost |
| M18 | Output budget requested per call | not measured | median 1,800 tokens, mean 3,112, 60% at or below 2,000, against a mean context of 136K | `metrics/scripts/` ad hoc; see Per-call cost |
| M19 | Plus-units per call by context band | not measured | 0.087 at 65K → 0.134 at 137K → 0.185 at 187K → 0.272 at 230K; calls ≥150K are 47% of calls and 58% of spend | `metrics/scripts/` ad hoc; see Per-call cost |
| M16 | Tool output discarded by harness truncation before the model reads it | not measured | 3.4 truncations per 100 model calls (79 over 2,313, 2026-10-13 22:55Z to 10-14 04:00Z), against 3.1 before; **75 of 76 truncated calls had set an explicit `max_output_tokens`**; 32% touch a doctrine or skill document and 38% fall within 120 s of a context reset | `metrics/scripts/output_truncation.py` |

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

### Per-call cost, 2026-10-13 22:55Z to 10-14 04:00Z

`calls × context` is not a figure of speech. Binning 2,313 calls of the round by
their own `input_tokens`:

| context band | calls | Plus-units per call | share of the round's 338 units |
|---|---|---|---|
| 50–75K | 183 | 0.087 | 4.7% |
| 100–125K | 331 | 0.117 | 11.5% |
| 125–150K | 417 | 0.134 | 16.5% |
| 150–175K | 472 | 0.160 | 22.3% |
| 175–200K | 415 | 0.185 | 22.7% |
| 200–225K | 178 | 0.206 | 10.8% |
| 225–250K | 32 | 0.272 | 2.6% |

Calls at or above 150K are **47% of the calls and 58% of the spend**, at 0.180
units each against 0.116 below. So the two lever families are: remove a call, or
lower what every call carries.

**What a context window actually costs, and why the trigger alone is worth
nothing.** These sessions do not use native compaction:
`~/.codex/supervisor.config.toml` sets `model_auto_compact_token_limit =
16000000` and enables `[features.token_budget]`, so the window is emptied rather
than summarized. The emptying is nonetheless not a reset to zero. Reading the
supervisor's 2026-10-13 23:43:17Z replacement: a 237K window was replaced by a
`retained_context` of 7,611 bytes, discarding 131K tokens of
`replacement_history` and 85K of `guardian_history`. The next call opened at
**80K** — system prompt, `AGENTS.md`, tool schemas and re-sent messages — and
cost **0.536 units**, four times a normal call, because none of it was cached
yet. The following thirteen calls climbed 80K → 116K reloading doctrine. **One
replacement cycle costs about 2 units and leaves a 116K floor.**

Two details about the retention. On root `01a088a4`, `retained_context` was
**117 bytes** for windows 42 through 61 — blank, which is the designed
behaviour — and became 10,300, 10,912, 15,070 and 14,294 bytes from window 62
(2026-10-05) onward. Whatever changed then, the current regime retains ~2–4K
tokens rather than nothing, and that is worth knowing for a design that assumes
nothing. Second, **every replacement observed fires at the ceiling**: 208–244K
across 65 windows on `01a088a4` and 216–238K across 11 on `01a1294e`. None was
taken at a chosen boundary.

That makes the arithmetic of a lower handoff trigger unfavourable on its own.
Projecting the supervisor's 423 calls and 67.7 units onto the band curve above:
at the observed 237K ceiling the cycle averages ~176K and 4 compactions; at a
155K ceiling it averages ~135K but the 39K of headroom above a 116K floor buys
only ~34 calls per cycle, so about 12 compactions. The context saving is ~18
units and the extra compactions cost ~17. **Net approximately zero.** An earlier
version of this section estimated 64 units from holding calls near 120K; that
estimate assumed a cheap floor and is withdrawn.

### Replacement economics the Rust lifecycle has to beat

The organized evidence package is
[`metrics/experiments/context-replacement-20261014/`](experiments/context-replacement-20261014/):
`extract.py` re-derives it from local rollouts, `summarize.py` recomputes every
figure its README quotes, and `observations.json` is the frozen extraction.
Campaign-state context for the slices this feeds is in
[`notes/rust-context-lifecycle-slice-state-20261014.md`](../notes/rust-context-lifecycle-slice-state-20261014.md).

T30 in
[`experiments/context-lifecycle-rust/DESIGN_EVIDENCE.md`](../experiments/context-lifecycle-rust/DESIGN_EVIDENCE.md)
requires that "no claimed optimum relies on an invented future workload", and
the pressure policy in that experiment's `DESIGN.md` is declared as
"revision-bound pressure" without tuned thresholds. These are measured values
from a real workload for exactly those slots, taken before the Rust host hosts
anything, so the comparison is pre-registered rather than reconstructed.

| Measured Codex-native, 2026-10-13/14 | Value | Where it belongs |
|---|---|---|
| post-replacement floor | **116K** (~80K reconstituted + ~36K doctrine re-read) | pressure threshold selection; T30 payback |
| first call after replacement | **0.536 units**, ~4× a normal call, nothing cached | T30 cache accounting; rehydration cost |
| one replacement cycle | **~2 Plus-units** | T30 conditional payback per replacement |
| cost per call by context | **0.087 u at 65K → 0.272 at 230K** | threshold choice; the curve the controller optimizes on |
| ceiling-driven replacement interval | **42–80 min, median 50**, all at 208–244K | M4 retained-workflow scenario realism |
| retention across a replacement | **117 bytes** (windows 42–61) rising to **7,611–15,070 bytes** (window 62 on), against 131K `replacement_history` + 85K `guardian_history` discarded | T13 semantic-preservation corpus: the control case is what a blanked window loses |
| rediscovery before productive work | **5 doctrine calls, ~36K tokens, 18 s** | "time until productive work resumes"; rediscovery cost |
| completion-started turns | **0 of 172** (`turn_trigger` always `user`) | S8 plan's "waiting alone does not trigger replacement": the native baseline it is written against |

**One boundary limits the claimable win.** The pre-Rust design's ownership
section disclaims "the exact hidden model context assembled inside Codex Core",
and ~80K of the 116K floor is exactly that: system prompt, tool schemas and
re-sent messages. While Codex Core assembles the prompt, the host's reachable
share of the floor is the ~36K rediscovery plus owning *when* replacement
happens. The remaining 80K becomes addressable only if the Rust host owns prompt
assembly. Any lifecycle benefit claim should state which of the two it is.

**The floor is the lever, not the ceiling.** The 116K floor is ~80K the harness
reconstitutes plus ~36K of doctrine the session re-reads. The second part is
ours. Projected on the same curve: a 60K floor with the ceiling left alone saves
~15% of the supervisor's spend; a 60K floor **and** a 155K trigger saves ~30%,
because cheap cycles can be taken often. A 155K trigger with a 116K floor saves
nothing. So the handoff packet is the lever and the trigger is a multiplier on
it, in that order — and the packet pays on every compaction, which happens
whether or not anyone plans a handoff. These are projections from the measured
cost-per-context curve, not measurements.

Removing calls is the smaller but immediately available family. In
`s9_native_capture_builder`, 150 of the round's 338 units over 1,180 calls:
**93% of tool calls carried exactly one tool invocation** (median 1, max 3) and
**60% requested 2,000 output tokens or fewer** (median 1,800) against a mean
context of 136K — a 136K charge to receive 1,800 tokens, over a thousand times.
339 of its calls were hash or verify bookkeeping: 138 separate `sha256sum`
invocations, 42 `jq` reads, 21 `git` reads, 10 `rg` sweeps. 17% of the hash calls
recomputed a path set an earlier call had already established, one seven-path set
fourteen times.

**Correction to the first estimate.** An earlier reading of this session
extrapolated from the 93% single-invocation figure to roughly 110 units
recoverable by batching. That does not survive checking the dependency
structure: the per-path hashing is already batched (median 3 paths, max 19 per
call), the whole-tree snapshot work is already a single in-process call, and
write-then-verify steps are genuinely sequential. The defensible figure is the
211 small read-only checks at ~0.127 units each, of which perhaps three quarters
could share a call, plus the 24 exact repeats: **about 20 units, 6% of the
round**. The batching lever is real and cheap; it is not the big one.

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
| Codex orchestration brief: long waits, consequential messages, settled-fact briefs, root handoff, dependency-closure validation | M1, M2, M3, M8, M11, M14 | most of M1/M2 if behavior follows; unknown otherwise | `wait_agent` timeout distribution; messages per sub-agent; briefs containing settled facts | long waits and consequential messages adopted and measured (see Supervision); root handoff **not** adopted, root context 166–180K against 144K baseline | `a72ab76`, `1586663` |
| Work Engine wake policy: no inference while waiting, wake eligibility, batching | M1, M2, M3 | ~10 points of spend (structural, central estimate) | turns recorded by wake reason; none while waiting | policy recorded, not implemented | `docs/context-lifecycle-wake-policy.md` |
| Deterministic coordination owners (campaign, claims, scheduler) | M3, M14, agent-messaging spend | ~7 + 3 points (central estimate) | dispatch and bookkeeping performed without model calls | partly built in Rust, not hosting development | baseline scenario model |
| Compiled role packages and event-scoped organizational judgment | M5, M8 | ~4 + 8 points (central estimate) | roles start from compiled packages, not full doctrine | architecture accepted, not implemented | `organizational-compilation.md` |
| Cache-aware accounting and conditional payback (T30) | M4, M7, M19 | ~0–3 points after structural savings; mainly visibility | payback reported per replacement | in S8 coverage, unproven; measured Codex-native constants for it are in Replacement economics below | `experiments/context-lifecycle-rust/DESIGN_EVIDENCE.md` T30, `experiments/context-lifecycle-rust/DESIGN.md` R28 |
| Relevance projection of settled facts into contract formation | M11, M9, M10, M14 | clear-bucket contradictions toward zero without suppressing novel discovery | contracts cite applicable settled facts before freeze | hypothesis | baseline README, `classification/` |
| Doc↔code claim projection in Codebase Memory (`sync_projection`); evidence-anchor impact nomination | M11, M6 | code changes nominate dependent claims and documents, and the reverse | nominations raised for changed anchors | proposal meaning approved, placement probable, not implemented | `proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/` |
| Coherent, content-verified index observation in Codebase Memory (upstream snapshot-coherence fixes, local publication 2026-10-11) | M15, M6, M8 | agents rely on the graph instead of re-reading source to establish freshness | share of coverage observations recommending source fallback; `content_verified` and `generation_matches` present and true | shipped locally, adopted, M15 measured; cost effect not isolated | upstream `#2585`, PRs `#2600`–`#2613`, `proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/evidence/` |
| Bounded Codebase Memory requests (output budget, limit, field and path selection, narrow `edge_types`), batched only up to what fits | M16, M6 | graph evidence reaches the model instead of being truncated away, without spending extra calls on paging | truncation events whose lost payload was a codebase-memory result; requests carrying an explicit output budget | recorded in the orchestration brief 2026-10-13, unmeasured | `docs/codex-orchestration-brief.md` |
| Status-probe discipline: no polling, `list_agents` only for a concrete coordination question that the wait result, messages and assignment record cannot answer | M1, M3 | removes the residue the long-wait lever left behind, about a third of remaining wait-only calls | `list_agents` calls per dispatch, and whether each one names the decision that needed it | **adopted, no saving measured** (2026-10-13 22:55Z on): probes 0.39 → 0.04 per dispatch, `list_agents` 39 of 828 calls → 1 of 208, `sleep` and `wait` to zero; units per dispatch 1.26 → 1.29 and wait-only share 17.2% → 18.1%, both flat | `docs/codex-orchestration-brief.md` |
| Context-boundary handoff: check context at each dispatch or integration boundary, plan a handoff at ~60% of the model window, transfer supervision only once the new root has accepted it | M8, M4, M3 | caps the standing-context charge that every coordination call pays | peak context per call against 60% of `model_context_window`; handoffs performed at a boundary with verified ownership transfer and no stranded children | **not actuable in these sessions**: no `new_context` tool appears in any measured session's tool list (`exec`, `spawn_agent`, `wait_agent`, `followup_task`, `send_message`, `list_agents`, `sleep`, `wait`, `request_user_input_async`, `interrupt_agent`), and all 76 observed window replacements fired at a 208–244K ceiling rather than a boundary. Mean context 166K → 168K, peak 238K → 237K. This is a missing capability in the session's tool set, not guidance that was ignored | `docs/codex-orchestration-brief.md`, `skills/wind-walker/SKILL.md` |
| Durable assignment record as the deterministic roster: owners, expected deliverables and received results updated on real events | M1, M3 | roster maintenance stops consuming inference; judgment about results stays with the supervisor | assignment record updated on assignment, message, completion and handoff, with `list_agents` not used to rediscover unchanged state | **adopted**: board messages naming a collector or resumer rose from 19 of 344 (6%) to 13 of 56 (23%), and two post-revision claims carry "Root collects" in the claim note | `docs/codex-orchestration-brief.md`, `skills/durable-state/` |
| Batched evidence collection: independent checks in one cell, `skills/evidence-battery/` as the declared form, re-verification by digest comparison instead of restating evidence | M17, M18, M19, M14 | removes a subset of calls outright; ~20 of the 338 units in the 2026-10-13 round, from 211 small read-only checks and 24 exact repeats | tool invocations per call and output budget per call; share of hash or verify calls recomputing a path set an earlier call established | shipped 2026-10-14, starting values pre-registered, untested | `skills/evidence-battery/`, `docs/codex-orchestration-brief.md` |
| Codex configuration trim: drop MCP servers and plugins whose tools are never invoked, so every session's standing tool-schema surface shrinks | M8, M19, M4 | ~2.7% of a round per 5K of standing context removed, plus the per-window uncached hit; 5–8% if the unused plugin surface is nearer 15K | first call's `input_tokens` for a fresh worker against the frozen 24–26K floors; tool inventory printed from one live session | audited 2026-10-14, 4 of 5 MCP servers unused across 101 sessions, proposed not applied (operator-owned config) | `metrics/experiments/codex-config-trim-20261014/` |
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
| 2026-10-13 | Root `01a1150c` compared with itself, 2026-10-05 to 10-09 against 2026-10-12 to 10-13, plus root `01a1294e` and 15 workers and reviewers in the current window | M1, M2, M3 | supervisor units per dispatch 1.82 → 1.21; waits per dispatch 3.34 → 1.04; messages per dispatch 3.13 → 0.21; timeouts 38% → 24%, and 0 of 70 on the second supervisor | full table and caveats in Supervision below. The long-wait and consequential-message levers are adopted; root handoff is not. Task mix differs between the windows and the current one was still running |

| 2026-10-14 | Per-call cost structure of the same round, 2,313 calls binned by their own context | M17, M18, M19, M8 | units per call 0.087 at 65K → 0.272 at 230K; calls ≥150K are 47% of calls and 58% of spend; 93% of one session's tool calls carried a single invocation and 60% asked for ≤2,000 output tokens | table and correction in Per-call cost. Shipped `skills/evidence-battery/` the same day so the batching lever has a declared form and a countable adoption check before the next round. The handoff trigger on its own is worth approximately nothing once the 116K post-compaction floor and the ~2 unit cost of a cycle are counted; the handoff **packet** that lowers that floor is the lever, projected at ~15% alone and ~30% with the trigger |

| 2026-10-14 | The `25a5164` revision tested, supervisor `01a1294e` against itself, 2026-10-13 22:55Z to 10-14 04:00Z, plus 5 workers and reviewers in the same window | M1, M5, M8, M16, all three pre-registered levers | status probes 0.39 → 0.04 per dispatch with units per dispatch flat at 1.26 → 1.29; durable assignment record adopted (collector named in 6% → 23% of board messages); context handoff not adopted (peak 237K of 258K); truncation 3.1 → 3.4 per 100 calls with 75 of 76 truncated calls having set a limit | table and reading in Supervision after the `25a5164` revision. Two of the three levers were adopted within minutes of the commit and neither moved spend, which is the result: the probing and the roster were not the cost. 2.3 hours and one supervisor, still running when measured; the after window is five times more dispatch-intense, so waits per dispatch is not comparable across it. The first reading of this window, taken at 903 calls, reported doctrine re-reads at 4.9 per 100 calls and truncation at 4.0; at 2,313 calls both fell to 2.4 and 3.4. The window opened minutes after several sessions had reset their context, so its early calls were reload traffic. A window that begins just after a context reset oversamples reload the way a session measured mid-run oversamples whatever it is doing |

| 2026-10-13 | Pre-change values for the `25a5164` brief revision, supervisor `01a1294e` over 2026-10-12 to 10-13 | M8, M1 | peak context **238K of a 258K window**, mean 166K, so the 155K handoff trigger was crossed and never acted on; status probes 0.39 per dispatch; `turn_trigger` was `user` for **172 of 172** turns | taken before the sessions that will exercise the revision, so the comparison is pre-registered rather than reconstructed. No turn in the window was started by a child completion, which is the upstream behavior of #46120 stated in the harness's own attribution field |

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

## Supervision, 2026-10-12 to 10-13

Root `01a1150c` has supervised continuously since 2026-10-07, so it covers the
baseline campaign and the current one. Comparing it with itself holds role,
model and client fixed; only the window and the task mix differ
([`metrics/scripts/supervision_profile.py`](scripts/supervision_profile.py),
whose `FROM`/`TO` bound the calls counted by entry timestamp).

| Root `01a1150c` | Baseline 10-05 to 10-09 | Current 10-12 to 10-13 |
|---|---|---|
| Plus-units per dispatch | 1.82 | **1.21** |
| waits per dispatch | 3.34 | **1.04** |
| `wait_agent` timeouts | 194 of 504 (38%) | 7 of 29 (24%) |
| longest wait requested | 60 s | **3,600 s** |
| wait-only calls | 548 | 46 |
| wait-only calls per dispatch | 3.63 | **1.64** |
| sub-agent messages per dispatch | 3.13 | **0.21** |
| wait-only share of its spend | 19.7% | 18.3% |
| context per call | 120K | 124K |

Supervisor cost per dispatch fell by a third, waits per dispatch by 69% and
sub-agent messages by 93%. The second supervisor, `01a1294e`, is cleaner still:
70 waits over 111 dispatches, **zero timeouts**, 237,840 seconds of wait
requested, 1.25 units per dispatch. Workers carry 0–4.8% wait-only spend, with
one residue: all six 10-second waits in the window expired, while the
supervisors' long waits returned on child completion.

Two inefficiencies are untouched, which is why further savings are likely
without being quantifiable yet. Status probing replaced timeout churn:
`list_agents` is 16 of 46 and 39 of 115 of the two supervisors' remaining
wait-only calls. And root context grew to 166–180K per call against the 144K
baseline, so the handoff lever is unused and cached re-send absorbs part of the
waiting win.

**Why M1's two values are not comparable.** The baseline 31–32% figure comes
from `act.py`, which spreads each call's cost across every activity it
performed and counts all coordination. The values here count only calls whose
every tool call was waiting or status probing, measured by one script on both
sides of the comparison. Read 19.7% → 18.3% as internally valid and 31–32% as a
different question. The current window was also still running when measured.

## Supervision after the `25a5164` revision, 2026-10-13 22:55Z to 10-14 01:13Z

Supervisor `01a1294e` re-read the revised brief at 22:55:34Z, 32 seconds after
the commit, so the window after that timestamp is a genuine post-exposure
window for it and the comparison below is the same supervisor against itself.
The window boundary is a timestamp, not a date: `FROM`/`TO` are ISO-8601
prefixes of any length, and the revision landed mid-day (local 16:55 MDT is
22:55Z).

| Supervisor `01a1294e` | Before, 10-12 to 10-13 22:54Z | After, 22:55Z to 01:13Z |
|---|---|---|
| model calls | 828 | 208 |
| dispatches | 114 | 27 |
| Plus-units per dispatch | 1.26 | 1.29 |
| **status probes per dispatch** | 0.39 | **0.04** |
| `list_agents` / `sleep` / `wait` calls | 39 / 5 / 1 | **1 / 0 / 0** |
| waits per dispatch | 0.65 | 1.52 |
| `wait_agent` timeouts | 0 of 74 | 0 of 40 |
| wait requested, every call | 3,600 s | 3,600 s |
| median wait held open | 72.5 s | 55.1 s |
| wait-only calls per dispatch | 1.04 | 1.63 |
| wait-only share of its spend | 17.2% | 18.1% |
| context per call, mean / peak | 166K / 238K | 168K / 237K |
| turn triggers | `user` 172 of 172 | `user` 11 of 11 |

**Status-probe discipline was adopted completely and saved nothing measurable.**
Polling went to almost zero — one `list_agents` call in 208, issued to confirm a
just-spawned child was running — yet units per dispatch (1.26 → 1.29) and the
wait-only share of this supervisor's spend (17.2% → 18.1%) did not move. The
probes were not where the money was. Wait-only cost is dominated by the waiting
call itself, which pays the full 168K standing-context charge to learn one
completion, and that call is not optional under the current design.

**Waits per dispatch rose, and the metric is the problem, not the behaviour.**
`wait_agent` returns on one completion, so a supervisor holding three children
needs three waits regardless of discipline. The after window is five times more
dispatch-intense (one dispatch per 5.1 minutes against one per 24.7) with more
children in flight, which raises waits per dispatch mechanically. Read waits per
*completion collected*, which is ~1.0 by construction, and drop waits per
dispatch as a quality measure; keep units per dispatch and the probe and timeout
rates. The supervisor was blocked in a wait for 84 of the window's 138 minutes,
which is the intended behaviour: every one of the 40 waits requested the 3,600 s
ceiling and none expired. Two returned in under two seconds, collecting a result
that had already arrived.

**Bounded requests are being set and are still truncating.** Truncation did not
fall: 3.4 events per 100 model calls against 3.1 before, 1.71M tokens discarded
over 2,313 calls. **75 of the 76 truncated calls whose body was recoverable had
set an explicit `max_output_tokens`.** The guidance is being followed literally
and does not work, because a limit is a truncation point rather than a budget:
`cat DESIGN.md` with `max_output_tokens: 15000` loses the tail just as surely as
no limit at all. One session sized the request first with `wc -l` and truncated
anyway. The brief's wording needs the request sized to the limit — line ranges,
outlines, `detail: ids` — not the limit attached to whatever request was already
intended.

**What truncates is largely the mandated reload.** 32% of truncated calls touch
DESIGN.md, PHILOSOPHY.md, this brief or a skill reference, and 38% are issued
within 120 seconds of a context reset; inside the first 25 seconds after a reset
the truncated calls are mostly doctrine. The reload is the largest single read a
session makes — DESIGN.md, PHILOSOPHY.md and the brief are ~10K, ~9K and ~3K
tokens, and the skills add more — so it is the read most exposed to a limit, and
it recurs once per context window rather than once per session. See **Why the
reload recurs** below.

**Why the reload recurs.** A compaction is recorded in the rollout as a payload
carrying `window_number`, `retained_context` and `previous_window_id`, with no
`type` field, which is why a scan keyed on payload type finds none. Across the
sessions measured here, **48 of 51 context resets are followed by a read of
DESIGN.md, PHILOSOPHY.md, the brief or a skill reference within 25 seconds**,
median about 6 seconds. That is not drift: `AGENTS.md` requires `$wind-walker`
in the first turn of every new context window and treats DESIGN.md as required
context for all work, so the reload is the instruction being obeyed. The cost
recurs per context window, not per session, and the doctrine set is ~22K tokens
before skills.

The per-session cadence has not changed. Median interval between consecutive
context windows was 73 minutes on 2026-10-07 and 42 on 10-08; it is 50 on 10-14.
What rose is how many long-running sessions are in that state at once: about 1.6
resets per hour across the fleet on 10-07 against about 3.75 in the first four
hours of 10-14, and 0.66 → 0.91 resets per 100 model calls. One worker is on its
fifteenth context window and one reviewer its thirteenth. So the share of calls
spent on reload traffic rises with fleet concurrency and session longevity,
without any agent behaving differently — and it is the context-boundary handoff
lever, the one lever not adopted, that would bound it.

**The claim these measurements test.** An agent should not spend inference to
discover what its execution environment already knows. Both measured residues
are instances: a supervisor paying a full-context call to ask `list_agents` what
the scheduler already knows, and a reviewer re-reading source to establish a
freshness the file watcher already tracked (M15). The levers above are specific
mitigations; the claim is what makes them one problem rather than several.
