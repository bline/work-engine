# F1.1: service tier and time in the usage ledger (2026-10-28)

Script: [`metrics/scripts/usage_ledger.py`](../scripts/usage_ledger.py), sha256
`450287d8b67d2ca613c93e6425889bc4282fe41728eacfae3a97578c2b8ddffa`. Tests:
[`metrics/scripts/tests/test_usage_ledger_f11.py`](../scripts/tests/test_usage_ledger_f11.py), 4 cases.
F1 record: [`usage-ledger-f1-20261026.md`](usage-ledger-f1-20261026.md).

**Status: approved by the owner on 2026-10-28** ("proceed"), after Codex Fast
came into use for supervisors and builders. F1.1 only adds; every F1 field,
total and file is unchanged (checked below).

## Why

Codex Fast (`service_tier` `priority` in rollouts) draws included usage at 2.5x
the standard rate (Codex speed docs). F1 priced every call at list price and
read no tier, so quota modelling from 2026-10-28 on would under-count Fast calls.
F1 also recorded no durations, so whether Fast is faster could not be measured.

## What F1.1 adds

| Output | Field | Meaning |
|---|---|---|
| `calls.jsonl`, Codex rows | `service_tier` | the thread's tier at the call, from the latest preceding `thread_settings_applied` of that thread; null when the rollout records none |
| | `quota_multiplier` | 2.5 for `priority`, 1 for `default`, otherwise null |
| | `latency_s`, `latency_basis` | call record time minus the latest preceding input in the thread (turn start, user message, tool output); null when no input came after the previous call |
| `calls.jsonl`, Claude rows | `service_tier`, `speed` | from the message usage |
| | `quota_multiplier` | null (not established for Claude) |
| | `latency_s`, `latency_basis` | the message's last record time minus the preceding user record |
| `turns.jsonl` (new) | one row per completed Codex turn | Codex's own `duration_ms` and `time_to_first_token_ms`, with tier, model and effort at completion |
| `summary.json` | `f1_1` (new key) | per provider, model, effort and tier: calls, list-price `usd`, `quota_weighted_usd`, latency count and median; turn count and median duration |

`usd` stays the frozen Standard API scenario. `quota_weighted_usd` is an
estimate built from the documented multiplier, not a quota reading.

## Unchanged: checks run

| Check | Result |
|---|---|
| `reconcile-trial` on the frozen trial input (sha256 `cffb785b9d20805f…`) | output byte-identical to [`usage-ledger-reconcile-trial-20261026.json`](usage-ledger-reconcile-trial-20261026.json) (`b2c292734a9b1403…`), exit 0 |
| F1 and F1.1 builds of the frozen window (`--since 2026-10-01T00:00:00Z --until 2026-10-27T04:30:00Z`, 77 stream files from the F1 input manifest), run side by side on the same inputs | `calls.jsonl`: 110,680 rows, 0 differ once the F1.1 keys are removed; `quota.jsonl`, `level1.json`, `issues.jsonl` byte-identical; every existing `summary.json` key identical |
| The F1 build in that comparison against the frozen F1 hashes | `calls.jsonl` and `quota.jsonl` match, so the inputs had not drifted |
| Other metrics tests (`test_task_profile_record.py`) and `run_registry.py`, which imports F1 helpers | 19 OK; `run_registry.py --help` exits 0 |

## Coverage in the frozen window

- Codex: latency for 95,390 of 96,115 calls; 724 had no input since the previous
  call, 1 is a fallback record.
- Codex tier: 82,381 calls `default`, 3,660 `priority`, 10,074 not recorded.
  **Fast was in use before 2026-10-28:** the 3,660 `priority` calls fall on
  2026-10-03 (1,907), 10-04 (740), 10-07 (884) and 10-08 (129), in 10 rollouts,
  $313 at list price. Models: `gpt-5.6-sol` (1,614), `gpt-6-astra` (1,560),
  `gpt-6-sol` (486).
- Claude: latency for 14,563 of 14,565 calls.
- 2,998 completed Codex turns, all with a recorded duration.

F1.1 outputs of that build (not committed, size): `~/cbm-work/usage-ledger-f11/new/`
(`calls.jsonl` 104,224,493 bytes, sha256 `4f85a40bf038bc02…`; `turns.jsonl`
1,537,364 bytes, `80fce8ec390f072b…`; `summary.json` `ab55aa336da15c52…`).

## Analyses that treated every call as standard

Not re-run here; each is `not established` until its owner rechecks it with the
tier:
- the per-model Codex quota fit in the experiment plan §1 (2,319 readings across
  5 weekly windows; the plan does not give their dates, so whether they include
  10-03 to 10-08 is not established);
- the pure-Codex efficiency baseline (`metrics/baselines/pure-codex-20261009`),
  whose hardcoded window 2026-10-05 to 10-09 includes 10-07 and 10-08.

## Limitations

- Latency is estimated from record times. It includes queueing and any client
  delay, and a call that follows another call without new input has none.
- The 2.5 multiplier is documented, not measured here.
- Claude `speed` is recorded but has no quota multiplier.
- Rollouts without `thread_settings_applied` have no tier; older history stays
  unknown, not `default`.
