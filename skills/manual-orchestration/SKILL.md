---
name: manual-orchestration
description: Coordinate manually decomposed engineering work across Codex, Claude Code or Pi sub-agents as the top-level supervisor, a worker or a reviewer - decomposition, packets, dispatch, waiting, the retained-versus-fresh context choice, handoff, review collection and integration. Use whenever a session coordinates sub-agents or is launched as one of these roles outside a configured Slice Builder campaign. It does not plan automatically or replace campaign gate and receipt contracts.
---

# Manual orchestration

Every model call re-sends the caller's whole context; in the 2026-10-05 to 10-09 baseline about 73% of estimated quota spend was that re-send. Most rules in this skill either remove a call or lower what each call carries.

Use model judgment to divide an authorized objective into coherent outcomes whose evidence and ownership can be checked. Keep the accepted objective, authority, source identity, unresolved work and review requirements visible. Packet precision is not proof that all obligations were found; test the proposed decomposition against the owning contract and relevant boundaries before dispatch and when new facts appear. The route, slice size and order may change when evidence warrants it.

## Load by role

The supervisor is the top-level role (the root), and the user is its parent. It dispatches builders directly and launches reviewers separately; no coordinator sits above it and no supervisor sits between it and its workers.

Read the reference for the current role and the applicable operation. Links provide navigation; do not recursively load linked documents or every host and lifecycle variant. Read applicable constraints before the decision they govern, together with their reasons. Reuse current material already loaded; retrieve an omitted or truncated relevant passage before relying on it.

| Role or decision | Load |
| --- | --- |
| Supervisor startup or recovery | [Supervisor entry](references/supervisor.md); its decision references apply when that work arises |
| Worker or reviewer | [One coherent assignment](references/worker.md) |
| Every role | [Call cost](references/call-cost.md) |
| Form or revise an organization | [Organization formation](references/organization-formation.md), including revisions during implementation without a new launch |
| Admit substantial work to retained context or transfer unfinished child work | [Retained dispatch](references/retained-dispatch.md) |
| Consider context replacement or root transfer | [Context boundary](references/context-boundary.md); load the chosen operation's reference |
| Codex token_budget is active, Claude's native context plugin is available, or Pi exposes `new_context` | [Native recovery](references/native-context.md): shared preservation/recovery and host-specific invocation; Claude native subagents cannot use the main-conversation clear adapter |
| Prepare intentional replacement | [Preparation](references/context-preparation.md) and the applicable host settings reference |
| Recover after replacement | [Recovery](references/context-recovery.md) and the applicable host settings reference |
| Operate on Codex | [Codex operation index](references/codex-host.md) |
| Operate on Claude Code | [Anthropic host](references/anthropic-host.md) |
| Launch, continue or collect a Pi role, or supervise from a Pi session | [Pi host](references/pi-host.md) |
| Measure a run | [Measurement](references/measurement.md); [demonstrated forms](references/demonstrated-forms.md) only when a concrete example helps |
| Select or invoke a helper script | [Command inventory](references/commands.md) and that operation's reference |

Repository instructions and owning contracts still govern each role. Conditional loading reduces unrelated context; it does not waive an applicable contract.

## Context pressure: every role

At startup and after recovery, establish an owned route that delivers this
role's verified context pressure to the deciding agent in time. This includes
supervisors, workers, reviewers and investigation roles. Load
[session control](references/context-control.md) to choose or repair the route
and apply its explicit-check fallback. Neither a topic-only wait nor the Pi
Chatboard inbox observes context. Uncollected output cannot warn the agent
before rollover. Keep the binding and delivery owner in the continuation;
the observation does not transfer ownership or decide when to replace context.

## Launching a child

At launch, give the child its packet and tell it to load this skill for its role: invoke `$manual-orchestration`, or read `skills/manual-orchestration/SKILL.md` in the repository when the skill is not installed. It loads its own references; do not paste this skill or other guidance documents into its brief.

After launch, verify the role's actual session ID and rollout from recorded lineage, using the host reference's lookup for its launch mechanism, then send its identity binding: session ID, rollout, parent/collection owner, continuation path, and the pinned context-status binary path and SHA-256. An ID cannot be verified before the session exists, and a role name or inherited `CODEX_THREAD_ID` can name another session. The role acknowledges the binding at its next boundary and records it in its continuation. For a retained role, deliver the binding explicitly once and expect it in the continuation thereafter.

## Invariants

Separate worker completion, unfinished continuation, acceptance of a bounded subject, private readiness, and integration; the supervisor makes the last three, and they remain separate decisions. Carry exact source, subject, binary/schema and evidence identity where those affect the claim; preserve failures and uncertainty. Refresh the *meaning* of affected claims when source or assumptions change, not just their hashes. Record requested model/effort separately from observed identity, and leave unavailable measurements unknown. The supervisor retains responsibility for collecting required child results and owned processes.

This is a manual coordination adapter. In a configured Slice Builder campaign, use its owning [skill and receipt contract](../slice-builder/SKILL.md) unless that contract's owner accepts compatible continuation semantics. An unfinished manual `handoff_ready` is not its terminal `audit_receipt`, `handoff_receipt`, or gate success. The skill grants no new mutation, provider, review, acceptance or publication authority.

## Deterministic outer host

The optional [manual host](references/manual-host.md) composes native Codex dispatch, owned observations, exact-turn collection and acknowledged receipts for an explicitly authorized objective. It leaves semantic decisions and existing domain records with their owners.

## Scripts

Find helper capabilities in the [command inventory](references/commands.md) when selecting or invoking one.
