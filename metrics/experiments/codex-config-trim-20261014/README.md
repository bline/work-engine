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

## Dogfood protocol

Baseline is already frozen in `../context-replacement-20261014/observations.json`:
worker first-call floor **min 24K, median 28K**; roots **76K and 80K**; cost per
call by context band from 0.0733 at 25–50K to 0.2222 at 225–250K.

1. **Attribute the surface.** In the first turn of one ordinary session, print
   each available tool's name and the byte length of its schema, then stop. This
   is the only step that needs a live session and it costs one call. It turns
   tier 2 from a guess into a list.
2. **Apply tier 1.** Then launch one worker on a task comparable to a 2026-10-07
   builder and record its **first call's `input_tokens`**. Compare against the
   24–26K floors of `ce1_builder`, `sc1_builder`, `rust_lifecycle_s0_20261007`,
   `ce3_builder` and `sc2_core_builder`.
3. **Apply tier 2** from step 1's list and repeat step 2.
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
