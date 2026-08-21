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
