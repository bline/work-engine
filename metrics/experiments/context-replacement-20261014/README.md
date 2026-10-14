# Context replacement evidence — 2026-10-14

A starting point for two deferred items: the metrics extension approved in
meaning as
[`proposals/agent-state/context-decision-observability/`](../../../proposals/agent-state/context-decision-observability/)
(`decision.json`: `approve_proposal_meaning`, 2026-08-24, with
`implementation_authorized: false`), and the replacement inequality in
[`ideas/01-semantic-durability-and-context-lifetime-judgment.md`](../../../ideas/01-semantic-durability-and-context-lifetime-judgment.md).

This package observes what already happens. It decides nothing, authorizes
nothing, and is not an acceptance, a placement claim, or a reset policy. The
proposal's own `decision.json` and `formation-evidence.md` remain untouched;
whether any of this is admitted as formation evidence is the decision owner's
call.

## Corpus

277 context replacements across 61 sessions, 2026-09-12 to 10-14, 44,233 model
calls and about 6,018 estimated Plus-units. Every figure below is printed by
`summarize.py` from `observations.json`; the README states no number that
script does not print.

```
python3 -I extract.py observations.json      # re-extract from local rollouts
python3 -I summarize.py observations.json     # recompute every quoted figure
```

## 1. What a host surface can authenticate today

The proposal's *Evidence required before an authority decision* asks to
"establish which current or prospective host surface can authenticate active
occupancy, invocation-effective limit, automatic-compaction transition, and
composite runtime identity — or truthfully mark each unavailable." From the
local rollout, with exact field names:

| Asked | Available from | Observed |
|---|---|---|
| active occupancy | `payload.info.last_token_usage.input_tokens`, per model call | 44,233 calls carry it |
| invocation-effective limit | declared: `task_started.model_context_window`; effective: context on the last call before a replacement | declared **258,400**; effective replacement band **181–244K**, median **216K** |
| replacement transition | a payload carrying `window_number`, with `previous_window_id`, `window_id`, `retained_context`, `compaction_response_id`, `resume_metadata` — **and no `type` field** | 277 observed |
| composite runtime identity | line 0 `session_meta`: `id`, `agent_path`, `agent_role`, `parent_thread_id`, joined to the window lineage | present for all 61 |
| actuation surface (`new_context`) | a tool in the session's tool list | **unavailable: 0 of 61 sessions offer it** |

The declared limit and the effective one differ, which is the distinction that
evidence bullet draws. `model_context_window` reports 258,400; no replacement
was observed above 244K.

## 2. Replacement is ceiling-driven, not chosen

`~/.codex/supervisor.config.toml` sets `model_auto_compact_token_limit =
16000000` and enables `[features.token_budget]`, so a window at the limit is
emptied rather than summarized.

| Context on the last call before a replacement | n=273 |
|---|---|
| minimum | 181K |
| 10th percentile | 201K |
| median | **216K** |
| 90th percentile | 233K |
| maximum | 244K |
| at or above 200K | **90%** |
| below 150K | **0%** |

No replacement in the corpus was taken at a low boundary. Combined with 0 of 61
sessions offering `new_context`, the controlled route is not operating in these
sessions: they run out the window. A context-boundary trigger is therefore not
actuable here, which is a missing capability rather than guidance ignored.

## 3. The retention regime changed on 2026-10-05

`retained_context` was **117 bytes** on every replacement from 2026-09-12
through **2026-10-04** — blank, which is the designed behaviour. From
**2026-10-05** it carries content: median 5,002 bytes across the corpus, maximum
16,382, and on one 2026-10-13 supervisor 7,611 bytes keyed
`verified_answers`, `incomplete`, `user_messages`, `assistant_messages`. 76 of
277 replacements are still blank; all of those predate 2026-10-05.

A design that assumes nothing survives a replacement should know that roughly
2–4K tokens now do. This package does not establish what populates it, whether
it is authenticated, or whether its content is trustworthy for continuation.

## 4. Both sides of the replacement inequality

The judgment idea states that context should be replaced "when continuing to
carry it is expected to cost more than preserving the remaining useful meaning
and rehydrating a fresh working set", and is explicit that it is "not a
fixed-threshold policy". Both sides are now measured.

**Carrying** — estimated Plus-units per model call by the call's own context,
over 44,233 calls:

| context band | calls | units per call |
|---|---|---|
| 25–50K | 1,790 | 0.0733 |
| 50–75K | 4,349 | 0.0795 |
| 75–100K | 5,984 | 0.0943 |
| 100–125K | 6,934 | 0.1142 |
| 125–150K | 7,000 | 0.1352 |
| 150–175K | 7,087 | 0.1608 |
| 175–200K | 6,600 | 0.1796 |
| 200–225K | 3,861 | 0.2036 |
| 225–250K | 535 | 0.2222 |

**Rehydrating** — the first call after a replacement is uncached and costs a
median **0.135** units, p90 0.313, maximum 0.539. Across the following fourteen
calls the context climbs from a median floor of **28K** to **70K** at a median
cost of **1.28 units**. A root supervisor sits well above the median: session
`01a1294e` opened its 2026-10-13 23:43:17Z window at 80K, paid 0.536 units for
that first call, and reached 116K over fourteen calls for 2.01 units.

The floor is what makes the inequality resolve against early replacement. At a
28K floor a lower trigger costs little; at a 116K floor the headroom under a
155K trigger is about 39K, so replacing more often costs roughly what the lower
context saves. The floor is a per-session property, and the sessions that
supervise have the worst one.

## Provenance and limits

`observations.json` is content-minimized: names, counts, token numbers, byte
sizes, timestamps, and a SHA-256 digest plus source line number for every
replacement record. No prompt, tool body, agent message or retained text is
copied. The configuration file is recorded by digest and by its three
non-blank settings.

- Estimated units use `7.826e-6 * (uncached + cached/10 + 5 * output)`
  Plus-units from the gpt-6 coefficient in
  [`metrics/baselines/pure-codex-20261009/README.md`](../../baselines/pure-codex-20261009/README.md).
  They are an estimate, not billing.
- Sessions still running when extracted give a snapshot. A window measured just
  after a replacement oversamples reload traffic; the band table is drawn from
  all 44,233 calls to avoid that.
- One rollout per session is read. Replacements before a session's first
  retained rollout line are not counted, so per-thread counts are lower bounds:
  `01a088a4` reports 65 replacements while its `window_number` reaches 65, and
  `01a1294e` reports 11 while reaching 76.
- This establishes what the current route costs and what it discards. It does
  not establish that a semantic checkpoint would preserve more, that any
  threshold is correct, or any causal claim about correctness or quality.
- No provider or network access; local rollout files only.
