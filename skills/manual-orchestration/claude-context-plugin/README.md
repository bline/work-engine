# Claude native context adapter

This local Claude Code mod registers `mcp__work-engine-context__new_context`
with empty arguments `{}`. It is skills tooling for development agents; it
does not use or change the Work Engine Rust runtime. JavaScript is the Mods
host's required module format.

After deploying the skill release, launch from the repository root with the
released plugin and its tool permission:

```sh
claude --plugin-dir "$HOME/.claude/skills/manual-orchestration/claude-context-plugin" \
  --allowedTools mcp__work-engine-context__new_context
```

The tool's short registered name is `new_context`; Claude exposes the plugin
prefix shown above. This directory is a plugin, not another `SKILL.md` or an
MCP server. Creating it does not install it globally or release the surrounding
skill. The implementation is validated against Claude Code 2.1.325; consult
the installed validator and generated types when using another version.

Before calling, follow [native recovery](../references/native-context.md):
preserve continuation-critical meaning with its durable owners, invoke Wind
Walker, and post a recovery note on `context-recovery:<verified-session-id>`.
The plugin deliberately does not infer checkpoint success from a tool call.
The note remains the discovery pointer into existing task state.

The requesting tool returns an acknowledgement with
`checkpoint: agent_owned_unverified`, `reset: pending_turn_end` and
`continuation: not_requested`. End the turn after receiving it. A zero-delay
host timer then invokes native `/clear`, after the completed main turn has
released its hooks. Only a matching `session.end` with reason `clear` and a
changed session ID establish `reset: confirmed`. The adapter subsequently
submits a recovery prompt containing the predecessor ID, repository, recovery
topic and shared Wind Walker obligations. `continuation: submitted` means the
host accepted that prompt; semantic recovery still belongs to the agent.

Inspect `/context-reset-status` for the session-scoped control record in the
plugin's Claude store. The new session points back to the predecessor record.
This record contains control observations, not the assignment checkpoint.
Before native clear is submitted, new input, an interrupted/failed turn, a
directory/session change or a reload cancels the pending request. Once the
native command is submitted, the adapter refuses incoming prompts until clear
and recovery submission settle; retry that input after recovery. Duplicate
requests before completion do not produce duplicate clears. There is no
automatic retry. A confirmed reset with failed continuation must be recovered
before another reset.

The adapter handles the **main conversation**. A separately launched worker
or reviewer process has one; a native `Agent` subagent does not have an
independent `/clear` target. Calls from native subagents are refused.
Existing review custody, child/process collection and authority persist through
replacement. The reset changes the provider session ID, so observation needs
a new binding. No child ownership transfer is implied.

For `claude -p` or Agent SDK use, keep streaming input open until recovery is
accepted and leave enough turn/spend allowance. The host can refuse a command
or prompt after input closes, a limit is reached, or the user stops a turn.
One-shot print mode is not an unconditional autonomous continuation route.
The plugin does not set a token budget or change pressure-warning policy.

Run offline checks without sign-in or model calls:

```sh
claude plugin validate skills/manual-orchestration/claude-context-plugin --strict
claude plugin test skills/manual-orchestration/claude-context-plugin
```

The Mods test kit stubs host operations. Passing it does not prove a native
`/clear` or autonomous recovery in a live model session. The
[dated native-host probe](../../../metrics/probes/claude-native-new-context-20261115.md)
records the actual CLI qualification and its limits. The
[scripted driver](tests/native-driver/README.md) intercepts model inference;
load it only in a disposable test process.

API references: Anthropic's [tool and background API](https://code.claude.com/docs/en/plugins/mods/api),
[lifecycle reference](https://code.claude.com/docs/en/plugins/mods/reference),
and [offline test kit](https://code.claude.com/docs/en/plugins/mods/test).
