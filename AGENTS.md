# Repository Instructions

Before working in this repository, read and follow:

- [DESIGN.md](DESIGN.md) for the system design and architectural constraints.
- [PHILOSOPHY.md](PHILOSOPHY.md) for the project's guiding principles and development philosophy.

Treat both documents as required context for all planning, implementation, review, and documentation work.

## Durable role schedules

Scheduled obligations can outlive model context, but a schedule does not create
authority or activate an agent. When a durable scheduler capability is
configured and this session has an active human-supervised role, read and
follow [the role-scheduler skill](skills/role-scheduler/SKILL.md) at startup.
Reconcile overdue, due, and near-term items before ordinary work that could make
them stale or invisible. Do not emulate durable scheduling by storing future
obligations only in model context or an unobserved terminal.

## Continuous Wind Walker

In the first turn of every new context window created by a session restart,
context reset, or compaction, invoke and follow `$wind-walker` before resuming
unfinished work. Perform this invocation in the new context window so the
skill governs recovery and continuation there.

Before intentionally replacing context, invoke `$wind-walker` and ensure all
continuation-critical objectives, authority boundaries, commitments,
unresolved obligations, and references have durable owners. If that cannot be
established, do not replace context.

If replacement occurs outside the agent's control, invoke `$wind-walker` in
the new context window and stop rather than continue when correct recovery
still depends on meaning lost with the prior context.
