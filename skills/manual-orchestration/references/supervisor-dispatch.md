# Supervisor dispatch and remediation

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: launching or substantially continuing a role, assigning collection, or planning remediation.

## Dispatch and the retained-or-fresh choice

Before dispatch, designate who collects each result and who resumes you. End your turn only when no required collection or integration remains, or a **verified** external mechanism owns resumption; human visibility is not automatic wakeup. If neither waiting nor external resumption is available, surface the blocked handoff rather than assuming completion will restart you. Use `spawn_agent` with `fork_turns="none"` for a new context and `followup_task` only for an intended retained continuation; a message to an idle agent is not a launch. **Before any substantial `followup_task`, run the pre-dispatch check** in [context boundary](context-boundary.md) and record retain or fresh with its reason. Request the model and effort from the active user selection ([model selection](../../../docs/model-selection.md)); record requested and observed identity separately.

Assign each worker one coherent outcome with explicit file/process ownership and the current packet revision. After launch, verify its session and send its identity binding as [SKILL.md](../SKILL.md#launching-a-child) describes. Compare the requested model and effort with the launch configuration promptly; a mismatch needs an explicit disposition before that worker does engineering, and its consumed work remains part of accounting. When a child session identity matters for handoff, review continuity or measurement, bind it to actual runtime lineage; an inherited shell thread ID may name the parent. Distinguish launch configuration and recorded turn context from backend identity if the latter is not exposed.

Keep a durable, compact assignment record as the deterministic source for owners, packet revisions, context checks, expected deliverables and received results. Update it on actual assignment, message, completion and handoff events; do not call a model or `list_agents` to rediscover unchanged state.


## Next task, remediation and handoff

Read and apply [architectural escalation](architectural-escalation.md) when
collecting implementation reviews and before dispatching repairs. Repeated
rejection, repair-caused regression and recurring invariant failures can require
design review before another repair; inherited defects remain evidence.

Before giving a retained worker a substantial new task - **including remediation after review** - run the pre-dispatch check in [context boundary](context-boundary.md). For Codex token-budget mode, apply the [recovery guidance](same-session-context.md#codex-token-budget-refresh-deadline) and the owning configuration; a recorded reason to retain cannot waive an explicit configured deadline. Otherwise record retain or fresh with its reason: H2 is a pilot recommendation you may overrule with a recorded reason. For a finished builder over that trigger, write the remediation packet from its result, continuation and review findings, and give it to a fresh worker. If transferring unfinished work, receive a durable continuation naming live ownership, decisions, failures, evidence pointers, remaining obligations and next action. Check it is safe to transfer, then follow the transfer steps in that reference.
