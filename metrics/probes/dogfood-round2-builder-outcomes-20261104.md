# Round 2 builder outcomes: Terra and Sol, 2026-11-04

Dogfood round 2 assigned each eligible builder task to `gpt-6.1-terra` or
`gpt-6.1-sol`, both at `medium`, by a coin flip at enrollment
(`os.urandom(1)[0] & 1`; [plan](../dogfood-selection-plan-20261027.md)). The
round paused on 2026-11-01T08:41Z when Terra reached its repair-round budget
(10 of 10). With 19 tasks enrolled, this is the only randomized comparison of
the two builders on production work. It supports the 2026-11-04 change of the
default builder in [model selection](../../docs/model-selection.md).

This is a read-only analysis. No agent was launched, and no round record was
changed.

## Result

**Terra needed more repair rounds and was not cheaper to build.**

| | Sol (n = 9) | Terra (n = 10) |
|---|---|---|
| First review: accept / changes required / none | 5 / 4 / 0 | 3 / 6 / 1 |
| Repair rounds (per task) | 4 (0.44) | 10 (1.00) |
| Tasks with a P1 at first review | 0 | 1 (3 P1) |
| Accepted / open / abandoned | 8 / 1 / 0 | 7 / 1 / 2 |
| Builder spend per session: median (total, sessions) | $0.93 ($9.50, 8) | $1.08 ($18.47, 8) |
| First-review spend per session: median (total, sessions) | $4.49 ($39.68, 8) | $4.62 ($45.55, 8) |
| First review ÷ build, per builder/reviewer pair: median (range) | 4.0× (1.8–17.9×) | 4.1× (0.8–22.3×) |

Neither difference is statistically significant at this n:

- repair rounds per task, Terra minus Sol, 0.56: one-sided permutation
  p = 0.152 over all 92,378 splits;
- first-review accept, 5/9 against 3/9: one-sided Fisher p = 0.319.

Terra's lower token price did not lower its build cost: it made more calls per
task (40–420 against 18–90). A first review costs about four times a build.
Each extra repair round therefore costs more than the build itself: another
review, plus repair and root coordination, which are not attributed here.

## Per task

| Arm | Task | First review | P1/P2/P3 | Repair rounds | Disposition | Builder $ | First review $ |
|---|---|---|---|---:|---|---:|---:|
| 6.1-sol | `b3-private-readiness-observer-001` | changes_required | 0/1/0 | 1 | accepted | 1.13 | 4.13 |
| 6.1-terra | `b3-qualification-package-assembly-sroot7-20261029` | changes_required | 0/2/0 | 2 | accepted | 10.45 | 8.74 |
| 6.1-terra | `budget-obs-ledger-001` | changes_required | 0/5/1 | 2 | accepted | 1.19 | 5.27 |
| 6.1-terra | `budget-obs-record-001` | changes_required | 0/1/1 | 1 | accepted | 0.32 | 3.10 |
| 6.1-terra | `dot-codex-mcp-bridge-20261101` | changes_required | 0/2/1 | 2 | open | 3.67 | 9.67 |
| 6.1-sol | `r2-screen-first-001` | changes_required | 0/1/1 | 1 | accepted | 0.60 | 2.63 |
| 6.1-sol | `rp3-guard-failure-diagnostics-001` | changes_required | 0/1/0 | 1 | accepted | 2.65 | 9.05 |
| 6.1-terra | `rp3-installation-packet-001` | changes_required | 3/4/0 | 3 | accepted | 0.36 | 7.99 |
| 6.1-terra | `rp3-linked-replacement-package-20261028-r1` | accept | 0/0/0 | 0 | accepted | 0.32 | 3.86 |
| 6.1-sol | `rp3-offline-package-qualification-001` | accept | 0/0/1 | 0 | open | 1.80 | 5.80 |
| 6.1-sol | `rp3-pipe-transport-correction-20261028-01a18390` | accept | 0/0/0 | 0 | accepted | 0.30 | 1.30 |
| 6.1-sol | `rp3-proposal-derived-episode-correction-001` | accept | 0/0/0 | 0 | accepted | 0.48 | 8.65 |
| 6.1-sol | `rp3-replacement-live-launch-001-adaptation` | accept | 0/0/0 | 0 | accepted | 0.74 | 4.85 |
| 6.1-sol | `rp3-replacement-protocol-repair-001` | accept | 0/0/0 | 0 | accepted | 0.74 | 4.85 |
| 6.1-sol | `s10-epoll-guard-repair-20261030` | changes_required | 0/2/1 | 1 | accepted | 1.79 | 3.27 |
| 6.1-terra | `s10-ledger-repair-20261029` | accept | 0/0/0 | 0 | accepted | 1.14 | 2.95 |
| 6.1-terra | `s10-morning-operand-20261030` | none | - | 0 | abandoned | 1.02 | - |
| 6.1-terra | `s10-operand-repair-slice2-20261030` | changes_required | 0/1/0 | 0 | abandoned | 1.02 | 3.97 |
| 6.1-terra | `s10-writer-followup-20261029` | accept | 0/0/0 | 0 | accepted | 1.14 | 2.95 |

## Caveats

- **n = 19, one round, mixed task types** (RP3, S10, B3, budget tooling, Dot
  bridge). Randomization balances type in expectation only.
- **Shared sessions.** Three pairs of tasks share a builder session:
  `rp3-replacement-*`, `s10-ledger-repair`/`s10-writer-followup`, and the two
  `s10-*operand*` tasks. The first two pairs also share a reviewer. The
  per-task table repeats the shared session's cost; arm totals and medians
  count each session once.
- **Escalation.** `b3-qualification-package-assembly` (Terra arm) moved to Sol
  mid-task: 228 of its 420 builder calls are `gpt-6.1-sol/medium`. Its cost
  stays in the Terra arm, as assigned.
- **Abandoned tasks** count 0 repair rounds, which flatters Terra's mean.
- **Spend** is the first review and the builder sessions only. Re-reviews and
  root coordination are not attributed. Costs are the ledger's API-equivalent
  USD under `usage-ledger-standard-api-v1`, not subscription quota.
- **Reviewers** were `gpt-6-astra` throughout, so findings are relative to
  that reviewer.

## Inputs and reproduction

```text
python3 -I metrics/experiments/dogfood-round2-builder-outcomes-20261104/round2_outcomes.py \
  ~/.local/state/work-engine/task-profile-pilot-2 LEDGER/calls.jsonl
```

- Round directory `~/.local/state/work-engine/task-profile-pilot-2`. Status
  read 2026-11-05: 19 enrolled, 17 closed, 2 open, paused, 0 inconsistencies.
  Records written by the recorder at sha256 `b9e2d7dd…`.
- Ledger: `usage_ledger.py build --since 2026-10-20T00:00:00Z`, built
  2026-11-05T03:24Z, 81,558 calls. Script sha256
  `ed026f40f93801c9fffab4a9d5d5fc32745fd1d94a6137edd22ed875b5a67291`.
- `round2_outcomes.py` sha256
  `42b56f5a40a413a6873b03b5e618f6419a7ba879427fb8527b538d09a26da7a0`.
