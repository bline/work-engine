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

## Native context refresh

Preservation and recovery are shared across hosts. The adapter controls context
replacement; the agent's durable owners retain assignment meaning. A tool
acknowledgement is not evidence of a committed checkpoint or recovered meaning.

Codex's native token-budget refresh is the direct model tool
`functions.new_context`, called with `{}`. Its invocation and availability
checks are described in [native recovery](../../skills/manual-orchestration/references/native-context.md#native-tool-invocation).
The nested `tools` object inside `functions.exec` is a separate surface;
checking only that object can falsely report the native tool unavailable.

Claude Code can expose `mcp__work-engine-context__new_context` through the
[manual-orchestration plugin](../../skills/manual-orchestration/claude-context-plugin/README.md).
It requests native `/clear` after the main turn ends, then submits recovery
bound to the predecessor's `context-recovery:<session-id>` board note. `/clear`
changes Claude's session ID; this does not transfer the role or its obligations.
The adapter confirms native replacement separately from continuation submission;
semantic recovery still follows Wind Walker. Native Claude subagent reset is
unsupported because `/clear` acts on the main conversation.

## Scoped recovery reads

Recover the exact active objective, authority, durable continuation, and unresolved
ownership from their owners. Follow the [AGENTS.md loading map](../../AGENTS.md#loading-map)
for the current task and role; recovery does not require reading every doctrine,
rationale, or role document. Recovered task state cannot waive an applicable contract.

A truncated result has not supplied its missing text. Retrieve the relevant
omitted material with a sufficient output budget before relying on it.
