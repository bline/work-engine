# Outer-context failure trials — 2026-10-28

Use same-session compaction as a supervised option with durable continuation and visible collection. These trials do **not** qualify an autonomous root controller or show that compaction is cheaper or better than a root handoff. A handoff transfers ownership; compaction changes context. Both still need durable owners for authority, pending questions and obligations.

The published planning guidance was applied: Terra/medium implemented the private harness, fixed Sol/medium/default fixtures exercised it, and a fresh native Opus 5.5 review was requested at xhigh and retained for report corrections. The observed reviewer model and session matched. No skill or Rust implementation was changed in this phase.

## What was observed

Eight fixture sessions used 28 model turns and eight compaction attempts: seven completed native compactions and one client killed before sending. The limits were eight sessions, 40 turns and ten attempts. Root independently collected all eight native histories; every turn was terminal. Owned clients, listeners and reviewer processes were collected, and the evaluator and fixtures were released.

The first run contained four qualified direct-path results, two unestablished results, one scripted hold and one failure. Subsequent retained trials changed the recovery instructions and prototype guard. Their results cannot retroactively make the first run pass or establish fresh replication.

| Case | Evidence and limits after review |
| --- | --- |
| F1: pause after stale check | The retained trial captured the active check, later pause, stale delivery and zero effects. Missing telemetry and admission also independently prevented work, so pause-specific causality remains unestablished. Later resume and validation produced one effect; prototype admission was inferred rather than explicit. |
| F2: kill before compact send | The declared pre-send barrier, kill/reap and independent history supported non-entry on that controlled path. |
| F3: kill after compact send | The existing compaction completed without replay. Reminded recovery loaded Wind Walker. A later, separately authorized control explicitly admitted the prototype gate and produced one effect after a settled declined control. |
| F4: completion hidden from recovery | The harness scripted a no-actor hold; this did not discriminate recovery decisions. The evaluator manually posted and collected a later unresolved notice. It was not an integrated automatic visibility mechanism. |
| F5: duplicate wakeup | Two identical logical wakeups produced one invocation through a counter that did not deduplicate. No compaction occurred in this case. |
| F6: dropped wakeup | A collected board notice and explicit fixture-human recovery preceded one effect. No compaction occurred; the board code's revision at execution is unestablished. |
| F7: wakeup client lost after send | The existing native turn was independently reconciled and collected with one effect and no blind replay. No compaction occurred. |
| F8: listener gap | The first prototype falsely admitted work after an exit-zero telemetry command returned unavailable usage/window. The revised profile held work, performed recovery-only observation, and later produced one effect with usable telemetry. Its separate prototype admission signal remained ambiguous. |

## Lessons and remaining uncertainty

An exit code establishes that the telemetry program ran; usable observation requires its actual session, boundary, request and window data. Earlier zero-input gaps can remain on both sides of a compaction boundary while a later valid request is usable. Those facts must be distinguished.

Recovery-only work can establish observation before a dependent effect. The successful loading evidence here used explicit reminders in the triggering turns; unprompted or checkpoint-only recovery remains unestablished. Durable continuation is still necessary.

The initial scorer falsely passed missing evidence and unsafe prototype admission. Its later boolean and substring checks did not establish a reliable general scorer. Raw evidence and outside review were necessary. The counter observed only its declared effect channel; these were not confined operating-system environments.

The context references and native helper were pinned to release `d16eeba55d4927922d8298cd487800faf7fd8f29`. Board operations used working-tree code whose bytes were not bound at the actual calls. The induced pressure thresholds were delivery tests, not threshold calibration. Production input authentication, arbitrary concurrent ingress, active-child collection across replacement, automatic restart/replay handling and net economic benefit remain unqualified.

## Review and evidence

Opus identified eight findings. Its retained review resolved seven; the final P3 gap-wording correction was verified mechanically and integrated by root afterward. This is an accepted, qualified assessment, **not** an `acceptable_as_is` reviewer verdict for the final revision or approval to adopt the controller.

The [evidence index](outer-context-failure-20261028.json) binds the plans, immutable subjects, reports and local raw evidence. The [consolidated assessment](/home/bline/.local/state/work-engine/outer-context-failure-20261028/report-v5.md) preserves all qualifications. The [first review](/home/bline/.local/state/work-engine/outer-context-failure-20261028/review/report-v1.md) and [retained review](/home/bline/.local/state/work-engine/outer-context-failure-20261028/review/report-v2.md) remain unchanged.
