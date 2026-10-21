# Measuring a run

Measure from rollouts and receipts after meaningful work, not during it.
Measurement design and scoring for a prospective comparison belong to the
measurement role, outside both arms.

## Instruments

- `token_count.rate_limits.used_percent` is on every call and is the account's
  own charge. Record it at session start and end per weekly window, grouping
  `resets_at` by minute. Prefer a whole window measured from its reset.
- Per-call context is `last_token_usage.input_tokens`; `total_token_usage` is
  cumulative and never a thread measure.
- Rollouts record no provider error payloads, so reliability questions need an
  instrument created in advance.
- [`metrics/scripts/`](../../../metrics/scripts/): `session_profile.py`,
  `supervision_profile.py` (dispatch-normalized), `cost_decomposition.py`,
  `coverage_outcomes.py`, `output_truncation.py`, and `handoff_threshold.py`
  (replays a trigger policy against real traces).
- [Baseline scripts](../../../metrics/baselines/pure-codex-20261009/scripts/):
  wait-only calls and timeouts, root share, re-read share, settled-fact
  contradictions per accepted slice.

## What to report

Units, waits, messages and status probes per dispatch; timeout rate; context
tokens per call; whether a parent resumed after completion; tool invocations
per call and output budget requested per call (starting values 93%
single-invocation and a 1,800-token median). For context decisions: each
check's reading, decision and reason, compactions per thread from lineage,
and fresh-versus-retained dispatch counts.

Track outcomes with cost: quota per accepted result, candidate and remediation
attempts, escaped defects. A cheaper run that misses important issues is not
an efficiency improvement.

## Accounting rules

- Distinguish counters not yet captured from unavailable ones; recoverable
  rollout counters are not "unavailable".
- A total includes coordination, preparation, failed or wrong-configuration
  launches, replacement, review, repair, collection and adjudication. Keep
  prior reusable skill work as separately identified investment.
- Keep same-definition comparisons apart from broad categories and state task
  mix; a before/after on different work is descriptive.
- For a prospective comparison, freeze the actual next subject and controls
  before either arm launches; if omission recall is scored, freeze an
  independently adjudicated sealed key too. Never label an already-started run
  preregistered.

The [efficiency ledger](../../../metrics/efficiency-ledger.md) records what
each measurement showed. The superseded
[orchestration brief](../../../docs/codex-orchestration-brief.md) keeps the
history behind the call-cost, waiting and opening-context numbers. The
compaction band and the 2026-10-21 dispatch case are derived in
[`compaction-band-20261021.json`](../../../metrics/probes/compaction-band-20261021.json)
(script `metrics/scripts/compaction_band.py`) and the
[dispatch timeline](../../../metrics/probes/auth-worker-dispatch-timeline-20261021.md).
