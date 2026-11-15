# Native context replacement and recovery

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: using Codex token_budget, using Claude's native context plugin, preparing new_context, or recovering after native replacement.

## Codex token-budget refresh deadline

When the session uses Codex's `token_budget` feature, `new_context` replaces
conversation history without generating a summary. The automatic limit takes
the same route. Missing preparation can leave the continuing agent unable to
recover its assignment or outstanding obligations. Apply this rule to every
role, including native workers and retained reviewers; ordinary compaction and
its summarization are not the recovery fallback in this mode.

## Native tool invocation

### Codex

In Codex, invoke the directly exposed model tool **`functions.new_context`**
with empty arguments **`{}`**. This is a direct tool call, not JavaScript for
`functions.exec`.

The nested `tools` object and `ALL_TOOLS` inventory inside `functions.exec`
are a different tool surface. An undefined `tools.new_context` or a failed
`tools.new_context()` call does not establish that the direct model tool is
unavailable. Check the currently exposed direct tool definitions before
declaring native refresh unavailable. If `functions.new_context` itself is
absent or its direct invocation fails, preserve that exact limitation and
apply the stop-and-report rule below; do not substitute a compaction helper.

### Claude Code

The [Claude context plugin](../claude-context-plugin/README.md) registers
**`mcp__work-engine-context__new_context`**, called with **`{}`**, when loaded in
a Claude Code process. Use the tool actually exposed by that host; this is not
an MCP server to launch and the Codex tool inventory says nothing about its
availability. Plugin presence does not enable Codex's `token_budget` feature
or disable Claude's automatic compaction.

Before calling, publish the recovery note on the exact chatboard topic
`context-recovery:<verified-current-session-id>`. The plugin carries that
predecessor ID into the new prompt because native `/clear` changes Claude's
session ID and does not run the mod's `session.start` hook again. The durable
owners and note must exist before requesting reset: the plugin records control
state, not a semantic checkpoint, and cannot certify preservation.

The result means **pending until this turn ends**. End the turn promptly after
the acknowledgement so that the adapter can run `/clear` outside a hook that
blocks the active turn. Before native command submission, new input, an
interrupted/failed requesting turn, or a plugin reload cancels a pending
request; it is not replayed against later work. While native clear and recovery
submission are in progress, incoming prompts are refused with a retry message
because the submitted native command cannot be cancelled by this adapter.
`/context-reset-status` shows the recorded native reset and continuation states.
A confirmed reset requires a matching native clear event and a changed session
ID. A submitted recovery prompt is not proof that required meaning was recovered.

This adapter targets only the **main conversation**, including a separately
launched Claude worker/reviewer process. Native `Agent` subagents cannot use it
to clear their own contexts: `/clear` would reset their parent's conversation.
Preserve their assignment and use the owning host's supported continuation or
handoff control instead. Retained review and collection obligations remain.

An interactive process can accept the continuation; a headless process needs
open input and remaining turn/spend allowance. Native commands or prompts may
be refused when input closes, a limit is reached, or the user stops the turn.
A failed or unconfirmed reset remains such; if clear succeeded but continuation
failed, recover the current context from the predecessor note before requesting
another clear. The adapter does not retry automatically.

## Preparation and recovery

Choose preparation and completion levels for the session's actual model,
rollover configuration, expected call growth and preservation work. These
levels are configurable operating margins, not universal token cutoffs or
provider timing guarantees. An explicit deadline in the owning session or
campaign configuration remains binding. A preparation warning calls for a
capacity decision and making recovery safe; it need not mean an immediate
reset unless the owning instructions require one. Account for recovery and
rediscovery cost as well as remaining headroom when choosing when to refresh.

A configured deadline is not a cost-optimal reset interval. The
[M22 replay](../../../metrics/efficiency-ledger.md#what-an-earlier-handoff-is-worth-replayed-on-44-builder-threads)
found that earlier replacement's modeled benefit depends on total restart and
rediscovery cost; its older models and host do not calibrate this route.

- Before intentional replacement, invoke Wind Walker, preserve required meaning
  through its existing owners, and publish a concise chatboard recovery note
  on `context-recovery:<verified-session-id>` with the assignment and continuation reference. Ensure
  the note can be found after reset; the latest 20 messages may not contain it.
- Leave enough headroom for preservation and invoke your own host's
  `new_context` tool before a configured completion deadline. If the
  deadline is near or ordinary work could consume preservation headroom,
  suspend ordinary work and make recovery safe. A supervisor may request this
  action, but cannot execute the
  context control for its child. For Claude native subagents, use the supported
  alternative described above. Do not substitute compaction for a requested
  history-discarding reset.
- In the new window, follow AGENTS.md, invoke Wind Walker and recover through
  the board note and its existing owners before resuming the assignment. An
  unchanged session ID or lower reading alone does not establish recovery.
  Missing continuation-critical meaning is a recovery failure: report it to
  the current collection owner and stop dependent work instead of guessing.

Keep the continuation usable during ordinary work so automatic rollover also
has a recovery path. This uses the existing continuation, board and pressure
listener; it requires no successor handoff or transfer of ownership. Preserve
review episodes and process/child collection responsibilities. If the tool or
its recovery route is unavailable, stop ordinary work and report the limitation
while preservation headroom remains. These instructions do not enable the
feature or set its numeric thresholds. The [native child probe](../../../metrics/probes/codex-native-new-context-20261114.md)
established one supervised child recovery, not autonomous pressure handling or
an optimal reset interval.
