# Defect 0001: task drain omits the cancellation request and cannot record its close disposition

| field | value |
|---|---|
| status | **QUALIFIED - awaiting authorized investigation.** Not an authorization, not a fix, no severity assigned. |
| reported | 2026-10-21 |
| reported against | `7f72a17b4156fbaa26d5a57c8125a39829e86618`, tip of `origin/rescue-detached-20260911` |
| also present at | `4071955` (accepted S2 checkpoint) and `7fdf392f` (S2 candidate 2) |
| component | `rust/crates/lifecycle-runtime/src/executor.rs` |
| contract basis | `experiments/context-lifecycle-rust/DESIGN.md`, "Shutdown and process ownership" |
| origin | Claude reviewer arms on S2 candidate 2, Test R; qualification in `../s2-review-pilot-consequences-20261019.md` |

## Contract

`DESIGN.md` states the shutdown protocol as a sequence:

> refuse new entry and respawn; snapshot owned attempts; **request supported
> cancellation where authorized**; drain local tasks/results; account for remote
> effects; close transport; observe/reap exact children; persist the shutdown
> report.

and the reporting requirement separately:

> `CloseReport` separately records attempted/not-attempted,
> resolved/rejected/timed-out, completion time/late status, each process exit,
> local tasks, remote settlement, and unsafe-at-close history.

`s2-plan-v1` invariant 6 states the same two requirements and is cited here as
corroboration only. That plan governs a closed slice; the governing basis for
current source is `DESIGN.md`.

## Observation 1: the cancellation-request step is absent from task drain

`OwnedExecutor::close_and_drain` (`executor.rs:343`) performs the sequence as
close gate, snapshot `registry.tasks.keys()`, then per attempt
`observe_terminal()` under `timeout_at`. No cancellation is requested.

The mechanism exists and is public: `request_abort` (`executor.rs:324`) takes an
`AttemptId`, calls `task.abort.abort()` and returns whether the task was known.
Its only caller in the tree is `tests/owned_execution.rs:367`.

Verified at all three revisions: zero `abort` calls within the body of
`close_and_drain`.

**"Where authorized" does not dispose of this.** `close_and_drain` takes no
authorization input and `TaskDrainReport` has no field for the decision, so the
implementation neither requests cancellation nor represents the condition under
which it would. Both branches leave a required record absent: if cancellation is
authorized at drain the step is missing, and if it is not, nothing records that it
was withheld, which `DESIGN.md`'s "attempted/not-attempted" requires either way.

## Observation 2: the close disposition cannot be expressed, though the sibling layer models it

```rust
pub struct TaskDrainReport {
    pub joined: Vec<AttemptId>,
    pub unresolved: Vec<AttemptId>,
    pub pending_results: Vec<AttemptId>,
    pub unsafe_at_close: bool,
}
```

Identical at all three revisions. The classifying expression is:

```rust
Ok(true) => joined.push(attempt),
Ok(false) | Err(_) => unresolved.push(attempt),
```

`Ok(false)` is "observed, not terminal" and `Err(_)` is "deadline expired". They
are merged, and no field distinguishes them afterwards. Of the required
distinctions, only unsafe-at-close history is representable.

**The process layer already models exactly what is missing.** In `process.rs` at
the same revision:

```rust
pub enum CloseDisposition { Resolved, ResolvedLate, TimedOut, Rejected }
pub enum ActivationState { NotAttempted, Requested, Confirmed, Uncertain }
pub struct ChildClose { child, activation, disposition, exit }
```

`ProcessShutdownReport` carries a `ChildClose` per child, so
attempted/not-attempted, resolved/rejected/timed-out and late status are all
recorded for processes and none of them for tasks. `ActivationState::NotAttempted`
is already the representation that Observation 1's withheld branch needs.

This makes the defect layer-local and gives the fix an in-crate template rather
than a new design.

## Consequence

A task that will not terminate within the drain deadline is reported `unresolved`
and left running, with no cancellation requested and no record of whether one was
attempted. A consumer of `TaskDrainReport` cannot tell a deadline expiry from an
observed non-terminal task, so it cannot distinguish a shutdown that ran out of
time from one that found work still live.

## What is not claimed

- **No reproduction.** Every observation here is source-read at pinned revisions.
  No test was written and nothing was executed.
- **No severity.** Assigning one requires knowing whether a production caller
  closes an executor with live tasks; that was not investigated.
- **No fix.** The in-crate template is named to scope the investigation, not to
  prescribe a patch.
- **Not a reopening of S2.** The accepted record is a true record of what was
  known; `s2-plan-v1.md` is unmodified and stays so.

## Deliberately excluded

Three sibling concerns from the same source failed or changed qualification on
2026-10-21 and are **not** part of this report:

| concern | disposition |
|---|---|
| deadline machinery disconnected | Split. `close_and_drain` does enforce an absolute deadline via `timeout_at(Instant::from_std(deadline))`; `process.rs` additionally checks `Instant::now() < cleanup_deadline` after its await. Actionable remainder is invariant 4's "before final success" on the ordinary completion path (`executor.rs:505-545`). The authority-expiry clause is a contract referral - no expiry reaches S2. |
| non-consuming `recv` | No invariant violated. `acknowledge_result` (`executor.rs:289`) is the documented consuming path. Head-of-line behaviour remains a liveness hazard; clarification request, not a defect. |
| constant `TaskExit` values | Fails qualification. `local_task_ended: true` is assigned after `inner.await` on the task's `JoinHandle`, so it is true by position; `child_exit_code: None` is accurate where no child exists. Surviving concern is that `task_results.child_exit_code` cannot distinguish "no child" from "unknown" - store/service referral. |

## What would falsify this report

- A production caller that requests cancellation before calling
  `close_and_drain`, making the step present at a different layer.
- A `DESIGN.md` revision scoping the cancellation step to processes only.
- An authorization rule under which task cancellation is never permitted at
  drain **and** a record elsewhere of that decision.
