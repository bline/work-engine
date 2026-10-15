# Codex config trim — audit and dogfood protocol, 2026-10-14

The post-replacement floor is paid by every call of every session. The lowest
observed floor is **24K tokens** (a worker restarting fresh, from
[`../context-replacement-20261014/`](../context-replacement-20261014/)), and the
repository's own text is a small part of it: `AGENTS.md` is 4,058 bytes and
`developer_instructions` in `~/.codex/config.toml` is 378 bytes, together under
1.2K tokens. The rest is the provider's system prompt and **tool schemas**, so
the tool surface is the only material client-side lever on the base.

This package audits that surface against measured use and proposes a trim. It
changes nothing: `~/.codex/config.toml` is environment-wide and affects sessions
this repository cannot see, so applying it is the operator's call.

## What the configuration currently loads

`~/.codex/config.toml`, 6,629 bytes: **12 enabled plugins**, **5 MCP servers**,
18 project trust entries, and one `[hooks.SessionStart]` augmentation.

| MCP server | Calls observed in 101 sessions |
|---|---|
| `codebase-memory-mcp` | **2,898** across 8 of its tools |
| `playwright` | **0** |
| `notify` | **0** |
| `node_repl` | **0** (and `startup_timeout_sec = 120.0`) |
| `openaiDeveloperDocs` | **0** |

MCP tools are invoked as `tools.mcp__<server>__<tool>(...)` inside the exec cell,
so a server with no `mcp__<server>__` prefix anywhere in 101 sessions was never
used. That evidence is conclusive for the MCP side.

Everything actually invoked across those sessions:

- named harness tools: `exec` (29,703), `wait_agent` (4,733), `send_message`
  (3,518), `followup_task` (781), `wait` (355), `list_agents` (293),
  `spawn_agent` (153), `sleep` (73), `request_user_input_async` (23),
  `interrupt_agent` (7);
- in-cell: `exec_command` (30,052), `apply_patch` (4,195), `write_stdin`
  (2,094), the eight `codebase_memory_mcp` tools, `clock__curr_time` (251),
  `web__run` (137), `mcp__codex_tui__*` (65), `view_image` (33).

## What cannot be concluded from the logs

Plugin tools carry no plugin prefix, so `web__run`, `clock__curr_time` and
`view_image` cannot be attributed to a specific plugin from rollout evidence,
and the per-tool schema cost is not recorded anywhere a log reader can reach.
The twelve plugins are therefore **not** safe to cut on this evidence alone.
Step 1 below closes that gap with one session.

## Proposed change, tier 1 — evidence-backed

Remove or comment out four MCP server blocks in `~/.codex/config.toml`:
`[mcp_servers.playwright]` (with its `.env`), `[mcp_servers.notify]`,
`[mcp_servers.node_repl]` (with its `.env`), `[mcp_servers.openaiDeveloperDocs]`.
Keep `[mcp_servers.codebase-memory-mcp]`.

Secondary observation, not a context cost:
`model_auto_compact_token_limit = 16000000` appears only in
`~/.codex/supervisor.config.toml`. The main `config.toml` does not set it, so
sessions launched without the supervisor profile do not express the same
intent. If no-built-in-compaction is meant to hold everywhere, it belongs in
both.

## Proposed change, tier 2 — after step 1

Disable the plugins whose tools step 1 shows are unused for this workload.
Candidates by workload rather than by evidence: `documents`, `pdf`,
`spreadsheets`, `presentations`, `template-creator`, `visualize`, `chrome`,
`unified-computer-use`, `code-review`, `github`. Do not cut whichever plugin
step 1 attributes `web__run`, `view_image` or `clock__curr_time` to.

## Applied, 2026-10-14T22:57Z

Both tiers were applied to `~/.codex/config.toml` after the operator asked for
plugins and MCP servers to be cut and the next rounds observed. Backup:
`~/.codex/config.toml.bak-20261014-preplugintrim`. The file still parses as TOML.

- **10 plugins set to `enabled = false`**: github, visualize, documents, pdf,
  spreadsheets, presentations, template-creator, chrome, unified-computer-use,
  code-review.
- **Kept enabled**: `codex-app-tools` and `browser`, because `view_image`,
  `clock__curr_time` and `web__run` are observed in use and cannot be attributed
  to a plugin from rollout evidence. Cutting these two would be a guess.
- **4 MCP server blocks commented out**: playwright, notify, node_repl,
  openaiDeveloperDocs. Restore by removing the leading `# ` from each line, or
  copy the backup back.

Running sessions are unaffected; configuration is read at session start, so the
sessions that resume after this timestamp are the "after" cohort.

## Dogfood protocol

The comparable baseline is **session-start context**, not the post-replacement
floor: the 98 sessions started 2026-10-04 to 10-14 opened at **min 20K, p25 21K,
median 21K, p75 25K**. The 20–21K cluster is the bare surface before any task
brief — system prompt, `developer_instructions`, project doc and every enabled
tool's schema — and is the quantity this trim moves. `floor.py` prints the after
side and the implied per-call saving:

```
python3 -I floor.py                  # sessions started after the trim
python3 -I floor.py 2026-10-15       # or any ISO-8601 prefix
```

The post-replacement figures in `../context-replacement-20261014/` (24K min, 28K
median, roots 76–80K) are a different quantity: those windows also carry
retained context and re-sent messages. Do not compare the two.

1. **Read the after side.** Run `floor.py` once sessions have resumed. A fresh
   session's first call is the measurement; no special instrumentation is needed.
2. **Attribute what remains.** In the first turn of one ordinary session, print
   each still-available tool's name and the byte length of its schema. This says
   whether the remaining surface is worth further cutting, and whether
   `codex-app-tools` or `browser` is the larger of the two kept plugins.
3. **Watch for a missing capability.** A disabled plugin fails as an absent
   tool, not as an error in these logs. If a session reaches for a cut
   capability, restore from the backup rather than working around it.
4. **Report** the floor before and after, and the implied per-call saving:
   removed standing tokens are charged uncached once per context window and at
   one tenth thereafter, so the per-call effect is
   `removed/10 * 7.826e-6` Plus-units, plus `removed * 7.826e-6` once per window.

## Expected size, stated before measuring

Removing 5K tokens of standing context is worth about 0.0039 Plus-units per
call, which is roughly **2.7% of a 338-unit round** at 2,313 calls, plus the
per-window uncached hit. If the unused plugin surface is nearer 15K the trim is
in the 5–8% range. This is small, permanent and free, and it is not a substitute
for the two larger levers the replacement evidence identifies: a cheaper
continuation packet, and supervisors that do not live for days. The floor rises
from a 28K median at under 500 calls to 57K beyond 1,500, which no configuration
change reaches.

## Measured, 2026-10-15

Three sessions started after the change. Roots are the clean comparison: a
worker's first call also carries its spawn brief, whose size varies by a few
thousand tokens and swamps the effect.

| Fresh root session starts | first-call input tokens |
|---|---|
| pre-trim, 5 fresh starts 2026-10-01 to 10-13 | 19,760 · 19,835 · 20,056 · 20,328 · 20,560 |
| post-trim, 2026-10-15 | **17,729 · 17,716** |

**−2,333 tokens against the pre-trim median, −11.6% of the floor.** That is
0.0018 Plus-units per call: **1.25% of a 338-unit round**, against the 2.7%
predicted above. The prediction was high by a factor of ~2 because the unused
plugin surface was nearer 2.5K than 5K.

`standing.py` reports each standing component's size **in rollout bytes** —
not in tokens, for the reason given under the retraction below — and attributes
the change component by component:

| component | pre-trim bytes | post-trim bytes | Δ |
|---|---|---|---|
| `base_instructions` | 21,420 | 18,037 | **−3,383** |
| `world_state.host_skills` | 16,671 | 13,858 | **−2,813** |
| `world_state.permissions` | 31,627 | 31,627 | 0 |
| `world_state.agents_md` | 7,150 | 7,150 | 0 |

So the plugin trim did take effect, as a smaller harness base prompt and a
shorter skills index. `codex debug prompt-input` confirms the second half
independently: re-enabling the ten plugins adds 2,803 bytes to developer
message [0], against the 2,813-byte `host_skills` shrink seen here. The four
MCP servers contributed little: their schemas appear nowhere in the rollout, and
the residual once the two components above are accounted for is a few hundred
tokens — `playwright` was evidently never loading, which also explains its zero
calls.

`disabled_plugin_ids` in `turn_context` is **not** evidence either way: it reads
`[]` in pre-trim and post-trim sessions alike, so it does not list
config-disabled plugins.

## Retracted: the approval rules cost nothing

**This section previously claimed the 133 stale approval rules were ~5,780
tokens and 3.0% of a round. That was wrong, and the measurement below is why.**

`world_state.permissions.approved_command_prefixes` is 31,627 bytes, 32.6% of
the standing *payload recorded in the rollout*, and it is byte-identical across
all 236 sessions since 2026-09-01. It is `~/.codex/rules/default.rules`: 133
allow-rules last modified 2026-08-05, not one of them for this repository,
mostly `ogrinfo` reads of specific shapefiles and multi-hundred-byte
`perl -0777 -i -pe s/.../.../` rewrites of another project's source. They are
also never consulted here — all 110 October sessions ran `approval_policy =
"never"`.

So they are stale and worth deleting for tidiness. They are **not** a token
cost. The operator installed the trimmed 461-byte file at 2026-10-15T15:56
local, which produced five `codex-tui` sessions byte-identical in every other
standing component:

| session | started | approval-rule bytes | first-call tokens |
|---|---|---|---|
| `01a1403f` | 15:47 | **31,627** | 18,348 |
| `01a140ba` | 18:02 | 248 | 18,348 / 18,344 |
| `01a140bc` | 18:04 | 248 | 18,348 / 18,344 |

**−2 tokens for 31,379 bytes removed.** Same base prompt (21,420 B), same
messages (25,532 B), same skills index (13,858 B), same `AGENTS.md` (7,150 B).

## The mistake, and the method that replaces it

The error was treating **bytes recorded in the rollout** as **bytes sent to the
model**. `world_state` is the client's own state snapshot, written to the
rollout so a reader can reconstruct what the session knew. Only part of it
becomes prompt input: `agents_md` does, as a user-role message;
`approved_command_prefixes` does not. Dividing total rollout payload bytes by
the first call's `input_tokens` produced one average ratio, and applying that
ratio back to a component silently assumed every component is prompt input.

Two claims die with it:

- The approval rules are not 5,780 tokens. They are zero.
- **`AGENTS.md` is not sent twice.** `world_state.agents_md.text` and the
  user-role message that carries it are the same content recorded in two places
  in the rollout; `codex debug prompt-input` shows the body appearing **once**
  in the model-visible list.

What survives is the plugin-trim measurement above, because both sides of it
were measured as first-call `input_tokens` rather than estimated from bytes.

### `codex debug prompt-input` is the right instrument, and it is free

It renders the model-visible prompt list as JSON without a model call, and `-c`
overrides let any configuration change be A/B tested before a single session is
restarted:

```bash
codex debug prompt-input "ok" > before.json
codex debug prompt-input -c 'plugins."chrome@openai-bundled".enabled=true' "ok" > after.json
```

Re-enabling the ten disabled plugins measures as **+2,803 bytes**, all of it in
developer message [0] — which matches the `host_skills` shrink of 2,813 bytes
seen in the rollouts, and confirms that message carries the skills index.

The model-visible list under the current configuration is 5 messages,
**23,181 bytes**:

| | role | bytes | content |
|---|---|---|---|
| [0] | developer | 13,105 | harness guidance (`wait_agent` timeouts) plus the skills index |
| [1] | developer | 2,400 | `<multi_agent_role>` |
| [2] | developer | 271 | `<multi_agent_mode>` |
| [3] | user | 7,403 | `AGENTS.md` for this repository, wrapped in `<INSTRUCTIONS>` and `<environment_context>` |
| [4] | user | — | the actual prompt |

What it does **not** render: the base prompt and the tool schemas. Those are the
remaining unattributed part of the floor, and the rollout does not record them
either, so the only way to size them is a matched-pair comparison of first-call
`input_tokens` across a config change — the method that worked above.

### Remaining client-side levers, on this basis

| lever | size | how to measure it for free |
|---|---|---|
| the skills index in message [0] | 2,803 B attributable to plugins, already removed; 41 skills still listed, `pages` four times | `-c '[[skills.config]]'` A/B through `prompt-input` |
| `AGENTS.md` | 7,403 B, sent once | edit and re-render |
| harness guidance in message [0] | the balance of 13,105 B | not configurable |

A fresh session's floor is about 17.7K tokens against 23,181 bytes of
model-visible messages, so most of the floor is still the base prompt and tool
schemas — neither of which this package has yet sized.
