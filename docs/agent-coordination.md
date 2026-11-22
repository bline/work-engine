# Shared repository coordination

Authority: repository operating instructions delegated by [AGENTS.md](../AGENTS.md).
Scope: shared-session resource coordination and an active human-supervised role.
Claims coordinate actors; they do not create workflow or human authority.

Load when: coordinating agents, claiming a shared resource, publishing transitions,
or reconciling a configured durable role schedule. The initial board read remains
in AGENTS.md so every session can discover current claims before acting.

## Coordinating sub-agents

Outside a configured Slice Builder campaign, a Codex or Claude Code session
that coordinates sub-agents, or that a parent launches under
[the manual-orchestration skill](../skills/manual-orchestration/SKILL.md), uses
that skill and loads only the references it names for the active role. A
child launched under another skill or review contract follows that contract.

## Durable role schedules

Scheduled obligations can outlive model context, but a schedule does not create
authority or activate an agent. When a durable scheduler capability is
configured and this session has an active human-supervised role, read and
follow [the role-scheduler skill](../skills/role-scheduler/SKILL.md) at startup.
Reconcile overdue, due, and near-term items before ordinary work that could make
them stale or invisible. Do not emulate durable scheduling by storing future
obligations only in model context or an unobserved terminal.

## Shared Codex coordination board

Concurrent Codex sessions share files but do not share conversation context.
At session startup, after compaction recovery, before claiming a shared
resource, and before any paid or externally consequential operation, read the
repository chatboard:

```bash
python3 skills/durable-state/scripts/codex_chatboard.py --repository . read --limit 20 --active-claims
```

This returns the newest messages and unexpired claims. When a task needs older
history, page forward with `--since <sequence>`.

Post concise transition messages for resource claims, launches, pauses,
interruptions, replacements, and terminal results. Include exact artifact or
campaign references. Use `claim` before operating on a shared campaign,
ordinal, reviewer episode, or publication resource, and `release` when done.
Claims coordinate actors but do not create workflow or human authority. The
board is CAS-published through private Git refs and does not modify a branch,
index, or working tree.

For directed coordination between enrolled Pi sessions on the same board and
group, prefer [Pi Chatboard](codex-chatboard.md#directed-coordination-in-pi).
The extension owns subscription, rearming and inbox delivery; do not run a
second CLI waiter for the same directed delivery. This does not replace the
board read above, claims, typed workflows, or separate routes for topic-only
traffic, context pressure, or objective budget.

For long commands, use the host's owned background-operation collection when
available. If using the CLI board job route, run
`skills/durable-state/scripts/chatboard_job.py` and add its job topic to the
owning CLI waiter; see [Wait for a background job](codex-chatboard.md#wait-for-a-background-job).
The Pi Chatboard inbox does not subscribe to that job topic.

Command lines (generated from `--help`):

<!-- cli-synopsis script=skills/durable-state/scripts/codex_chatboard.py subs="post claim release read" display="python3 skills/durable-state/scripts/codex_chatboard.py --repository ." -->
```text
python3 skills/durable-state/scripts/codex_chatboard.py --repository . post --author AUTHOR --session-id SESSION_ID --topic TOPIC --body BODY [--message-id MESSAGE_ID] [--ref REF] [--wake] [--kind KIND] [--role ROLE] [--to TO] [--reply-to IN_REPLY_TO] [--supersedes SUPERSEDES] [--effects-file EFFECTS_FILE] [--data-file DATA_FILE]
python3 skills/durable-state/scripts/codex_chatboard.py --repository . claim --resource RESOURCE --author AUTHOR --session-id SESSION_ID --note NOTE [--claim-id CLAIM_ID] [--ttl-seconds TTL_SECONDS]
python3 skills/durable-state/scripts/codex_chatboard.py --repository . release --resource RESOURCE --session-id SESSION_ID --claim-id CLAIM_ID
python3 skills/durable-state/scripts/codex_chatboard.py --repository . read [--since SINCE] [--limit LIMIT] [--active-claims] [--kind KINDS] [--role ROLES] [--format {json,compact}]
# other subcommands (put --help after one for its usage): wait, topics, roles, requests, handoff-publish, handoff-status, owner-boundary-io, kinds, adopt
```
<!-- /cli-synopsis -->
