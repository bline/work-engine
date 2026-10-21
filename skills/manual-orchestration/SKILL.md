---
name: manual-orchestration
description: Coordinate manually decomposed engineering work across Codex sub-agents as coordinator, supervisor, worker or reviewer - decomposition, packets, dispatch, waiting, the retained-versus-fresh context choice, handoff, review collection and integration. Use whenever a session coordinates sub-agents or is launched as one of these roles outside a configured Slice Builder campaign. It does not plan automatically or replace campaign gate and receipt contracts.
---

# Manual orchestration

Every model call re-sends the caller's whole context; in the 2026-10-05 to 10-09 baseline about 73% of estimated quota spend was that re-send. Most rules in this skill either remove a call or lower what each call carries.

Use model judgment to divide an authorized objective into coherent outcomes whose evidence and ownership can be checked. Keep the accepted objective, authority, source identity, unresolved work and review requirements visible. Packet precision is not proof that all obligations were found; test the proposed decomposition against the owning contract and relevant boundaries before dispatch and when new facts appear. The route, slice size and order may change when evidence warrants it.

## Load by role

- **Coordinator** (root): [coordinator](references/coordinator.md) - decomposition, packets, dispatch, collection, integration.
- **Supervisor**: [supervisor](references/supervisor.md) - one engineering group, launch, validation, review collection.
- **Worker or reviewer**: [worker](references/worker.md) - one coherent bounded assignment.
- **Every role**: [call cost](references/call-cost.md) - waiting, batching, request sizing.
- **Coordinator and supervisor**: [context boundary](references/context-boundary.md) - retained or fresh before dispatch, transfer, roots. Workers carry their own mid-task check.
- After a run: [measurement](references/measurement.md). For a concrete artifact example: [demonstrated forms](references/demonstrated-forms.md).

Repository instructions and owning contracts still govern each role.

## Launching a child

At launch, give the child its packet and tell it to load this skill for its role: invoke `$manual-orchestration`, or read `skills/manual-orchestration/SKILL.md` in the repository when the skill is not installed. It loads its own references; do not paste this skill or other guidance documents into its brief.

After launch, verify the child's actual session ID and rollout from recorded lineage (`<pinned context-status binary> agents --parent-thread <your id> --json`), then send its identity binding: session ID, rollout, parent/collection owner, continuation path, and the pinned context-status binary path and SHA-256. An ID cannot be verified before the session exists, and a role name or inherited `CODEX_THREAD_ID` can name another session. The child acknowledges the binding at its next boundary and records it in its continuation. For a retained child, deliver the binding explicitly once and expect it in the continuation thereafter.

## Invariants

Separate worker completion, unfinished continuation, supervisor acceptance, private readiness, and root integration. Carry exact source, subject, binary/schema and evidence identity where those affect the claim; preserve failures and uncertainty. Refresh the *meaning* of affected claims when source or assumptions change, not just their hashes. Record requested model/effort separately from observed identity, and leave unavailable measurements unknown. The coordinator retains responsibility for collecting required child results and owned processes.

This is a manual coordination adapter. In a configured Slice Builder campaign, use its owning [skill and receipt contract](../slice-builder/SKILL.md) unless that contract's owner accepts compatible continuation semantics. An unfinished manual `handoff_ready` is not its terminal `audit_receipt`, `handoff_receipt`, or gate success. The skill grants no new mutation, provider, review, acceptance or publication authority.

## Scripts

- `scripts/provision_context_status.sh [repo]` - build and pin the context-status binary outside the shared target; prints path and SHA-256.
- `scripts/child_context.py` - `check` one session or `tree` a parent's children against the trigger and observed compaction band; writes a record to retain.
