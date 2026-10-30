# Budget observation pilot: S10 slice (2026-10-30)

Phase 4, run 1, of
[the budget observation plan](../../planning/work-engine-budget-observation-implementation-plan.md).
One slice (n=1).

## Setup

- **Objective:** the S10 root's slice `budgeted-holder-closure-phase4`
  (`objective_id` `a67df815…`). The record is
  `~/.local/state/work-engine/s10-session-quota-repair-20261021/resume-20261022/root-successor-20261028-01a18305/budgeted-holder-closure-phase4/assignment.json`.
- **Limits:**
  - started 05:18:04Z, with a user limit of 0.5 hours;
  - a user limit of $5 on the Codex pool;
  - S10 later added its own $1 mark on its root node.
- **S10's side:** its root bound the budget source to its own board listener,
  one-shot `wait` calls with `--budget-*` and a cursor. Reader
  `usage_ledger.py`, sha256 `45f509a5…`.
- **Shadow logger, read-only:** the supervisor's logger ran
  `usage_ledger.py control` on the same record every 60 s, from 05:19:01Z to
  06:04:01Z.

## Observations

| Item | Value |
|---|---|
| Logger runs | 46, no errors |
| `control` run time | median 0.61 s, range 0.51 to 0.65 s |
| Events seen by the logger | 05:32 `node_mark_crossed` ($1, supervisor) at $1.86; 05:43 `limit_fraction` (50% of $5) and `hours_fraction` (80%); 05:49 `hours_reached` |
| Events delivered to S10 | one: `node_mark_crossed` at 05:31:40.48Z (snapshot 05:31:39.90Z), same value as the logger's ($1.856) |
| S10's action | recorded the event: "informational … no additional dispatch or authority change; close slice" |
| Model turns caused by budget events | one (the 05:31:40 wake) |
| Slice closed by S10 | 05:33:45Z, at $1.856 (28 Codex calls: root 16 at $1.52, worker 12 at $0.34) |
| Objective spend at the logger's end | $3.325 (42 calls) |
| Unrelated issues | 6 `conflicting_native_counter` from another Claude session, counted under `outside`, so `status` stayed `complete` (fix `292c1165`) |

## Finding: an objective has no end

After S10 closed the slice at 05:33:45Z, its root kept working on other tasks
(night-close accounting).
- The tree kept charging the closed objective: 14 more root calls, $1.47, which
  is 44% of the final $3.325.
- The 50%-of-limit and hours events fired against an objective that was already
  closed. S10 had stopped listening, so they reached no one; a longer-lived
  root would see them as false alarms.
- S10 wrote its own `closed_at` into the record, outside `scope_budget`.

**Needed:** a close boundary that `scope_budget` owns. `tree` and `control` end
the window there, and `control` emits no spend or hours events after it. This is
the minimal form of the plan's deferred item "Splitting reused sessions by
assignment" (§6.5).

## Other §6.5 items, from this run

- **Allocations, reserves and leeway:** not needed here. A user limit plus a
  supervisor mark covered the slice.
- **Expected cost per assignment:** not needed yet.
- **Incremental reading:** not needed; a run takes under 1 s.
- **Quota conversion:** not exercised.
