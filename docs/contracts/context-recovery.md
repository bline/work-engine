# Repository context recovery

Authority: repository operating instructions delegated by [AGENTS.md](../../AGENTS.md).
Scope: every agent resuming repository work after context loss, or preparing
intentional context replacement. This file adds no campaign or mutation authority.

Load when: preparing context replacement or recovering after restart, reset, or
compaction. The [Wind Walker skill](../../skills/wind-walker/SKILL.md) owns the
continuation-independence requirement; the rules below apply it to this repository.

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

## Scoped recovery reads

Recover the exact active objective, authority, durable continuation, and unresolved
ownership from their owners. Follow the [AGENTS.md loading map](../../AGENTS.md#loading-map)
for the current task and role; recovery does not require reading every doctrine,
rationale, or role document. Recovered task state cannot waive an applicable contract.

A truncated result has not supplied its missing text. Retrieve the relevant
omitted material with a sufficient output budget before relying on it.
