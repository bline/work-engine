# Native parent resumption experiment — 2026-10-13

The bounded experiment supports the theory: a waiting parent continued on child
completion, while an ended parent turn did not resume. Explicitly starting a new
parent turn delivered the queued result. No runtime or orchestration guidance was
changed.

## Results

| Treatment | Observed result |
|---|---|
| `wait_agent(timeout_ms=60000)` | Returned after 55.175 seconds with `timed_out=false`; parent collected `WAIT_COMPLETE`. |
| End parent turn while child is running | Child completed at 18:10:11.901 UTC. At 18:11:30.569, 78.668 seconds later, parent remained completed with its earlier `IDLE_ARMED` response. No parent rollout events occurred in that interval. |
| Explicit `followup_task` recovery | New parent turn began at 18:11:41.098 UTC. The queued child message arrived with `trigger_turn=false`, followed by the root input with `trigger_turn=true`. Parent confirmed that the child result was a separate received message. |
| `wait_agent(timeout_ms=3600000)` | The declared maximum was accepted. Returned after 15.196 seconds with `timed_out=false`; parent collected `MAX_WAIT_COMPLETE`. |

The wait results were recorded 8 ms and 10 ms after their corresponding child
terminal events. These are local event timestamp differences, not general
latency guarantees. Neither wait expired. Parent-authored intervals in
`parent-events.jsonl` include surrounding model/tool overhead; the table uses
paired native tool-call/output timestamps instead.

Each pending wait interval contains one token usage record, for the response
issuing that wait, and no additional parent response records or tool calls.
Those dispatch responses used 43,706 input tokens (43,264 cached) and 45,437
(44,928 cached), respectively. This supports absence of repeated inference
while blocked; it does not mean initiating a wait or processing its result is
free. No monetary or quota saving was measured.

## Method and provenance

The real root retained responsibility for integration. A disposable nested
parent spawned one leaf, then reused it for all three child turns. The parent
waited for the first result, ended its turn after dispatching the second, and
waited with the maximum timeout for the third. The observer did not message the
idle parent during the observation window; it inspected local logs and made one
read-only agent-status observation before explicit recovery.

The initial route was recorded in [protocol.json](protocol.json); the maximum
wait extension is in [protocol-extension.json](protocol-extension.json).
The explicit recovery instruction itself precedes that extension file in the
root transcript. This is an adaptive follow-on, not a preregistered initial arm.

- Root: `01a1366f-44ff-73f3-9d3b-858d18782d47`.
- Parent: `01a13672-db3c-75d0-b6ac-5961671f996e`,
  `/root/resumption_probe_parent`.
- Leaf: `01a13673-5b5f-77a0-9920-393a297757e1`,
  `/root/resumption_probe_parent/leaf`.
- Native session metadata: Codex `0.162.1`. The separately installed PATH CLI
  reports `0.160.1` and was not used to run the experiment.

[final-trace.json](final-trace.json) contains content-minimized native events,
per-response usage, source rollout paths, line numbers and hashes.
[idle-observation-final.json](idle-observation-final.json) freezes the observation
before recovery; [observer-status.json](observer-status.json) records the status
tool result. Parent and leaf event files are actor-authored corroboration,
distinct from native rollout evidence. [extract.py](extract.py) reproduces the
extraction from local rollouts; a later extraction is a new snapshot, not the
same frozen evidence.

## Upstream corroboration

[Issue #46120](https://github.com/openai/codex/issues/46120) was open when read
on October 13, 2026. Its October 12 comment references commit
`a9b0815167c27ab13719dd12d13b63e02b70d221`. At that revision,
[completion.rs](https://github.com/openai/codex/blob/a9b0815167c27ab13719dd12d13b63e02b70d221/codex-rs/core/src/agent/control/completion.rs#L111-L118)
sets the completion communication's trigger flag to false, and
[handlers.rs](https://github.com/openai/codex/blob/a9b0815167c27ab13719dd12d13b63e02b70d221/codex-rs/core/src/session/handlers.rs#L88-L97)
attempts continuation for a true flag or outstanding durable sleep.
[upstream-source.json](upstream-source.json) retains exact excerpts and file
hashes. Installed-binary equivalence to that commit was not established; the
native experiment independently observed matching behavior.

## Interpretation and limits

Long blocking waits preserve result collection in this tested runtime and
avoid timeout-driven activations until the ceiling expires. Ending a turn
removes that active continuation path. Result delivery and starting a new turn
are separate behaviors; reducing routine child messages does not establish an
idle-parent wake mechanism.

The existing brief already recommends long waits. Its permission to end a turn
when the human monitors agents needs reconsideration for work that still needs
parent integration. Human visibility alone did not supply continuation here.
The evidence supports revising that operating default before relying on it for
autonomous integration. This experiment does not implement the revision.

This is one trial per treatment, using a nested parent as a proxy for an idle
top-level session. It establishes a bounded absence of resumption, not that
resumption can never occur. It does not test hour-long waiting, timeout expiry,
cancellation, simultaneous completions, crashes, durable sleep, or other
clients. It does not evaluate a runtime patch or prove the full proposed
scheduling design. Coalescing, cancellation and duplicate suppression remain
separate design and qualification questions.

The historical 144K figure is average root input per call in the frozen
baseline, not a per-wait constant or money spent. This small experiment measures
liveness and recorded response activity, not an efficiency improvement over
that workload. No existing baseline values or policy files were changed.
