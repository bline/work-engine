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

`standing.py` itemizes where the floor actually comes from and attributes the
change component by component:

| component | pre-trim bytes | post-trim bytes | Δ |
|---|---|---|---|
| `base_instructions` | 21,420 | 18,037 | **−3,383** |
| `world_state.host_skills` | 16,671 | 13,858 | **−2,813** |
| `world_state.permissions` | 31,627 | 31,627 | 0 |
| `world_state.agents_md` | 7,150 | 7,150 | 0 |

So the plugin trim did take effect, as a smaller harness base prompt and a
shorter skills index. The four MCP servers contributed almost nothing: their
schemas are not in the rollout, and the residual after accounting for the two
components above is a few hundred tokens — `playwright` was evidently never
loading, which also explains its zero calls.

`disabled_plugin_ids` in `turn_context` is **not** evidence either way: it reads
`[]` in pre-trim and post-trim sessions alike, so it does not list
config-disabled plugins.

## The larger lever this exposed: stale approval rules

`world_state.permissions.approved_command_prefixes` is **31,627 bytes, 32.6% of
the standing payload, an estimated 5,780 tokens** — the single largest item,
larger than the harness base prompt. It is byte-identical in all 236 sessions
from 2026-09-01 to 10-15 and is re-sent on every model call.

It is `~/.codex/rules/default.rules`: **133 approval rules, 36,590 bytes, last
modified 2026-08-05**. Not one mentions this repository. They are one-off
approvals from the `locus`/`geobridge` work — `ogrinfo` reads of specific INEGI
shapefiles, `docker logs` for `compose-locus-cli-run` container IDs, and
multi-hundred-byte `perl -0777 -i -pe s/.../.../` source rewrites of files in
another project. The five longest entries are 522–1,491 bytes each.

They are also inert here: **all 110 October sessions ran `approval_policy =
"never"`** (109 `danger-full-access`, 1 `read-only`), so no allow-rule is ever
consulted. This is dead weight at 2.4× the size of what the plugin trim removed.

`default.rules.trimmed` in this directory keeps the 8 short generic rules
(`cargo test`, `cargo fmt --all`, `docker ps|logs|inspect`, `set -euo pipefail`,
`dmesg`, `sort`) and drops the 125 project-specific ones: **461 bytes, down from
36,590**. To apply:

```bash
cp ~/.codex/rules/default.rules ~/.codex/rules/default.rules.bak   # if absent
cp default.rules.trimmed ~/.codex/rules/default.rules
```

Expected: about **−5,700 tokens**, 0.0045 Plus-units per call, **3.0% of a
338-unit round**. One fresh session measures it: run `floor.py` again and the
floor should land near 12K. The one cost is outside this repository — a `locus`
session with approvals enabled would prompt again for those commands.

## Standing-context inventory, 2026-10-15

Measured from one post-trim root session: 96,951 bytes of standing payload
against 17,716 first-call tokens, 5.47 bytes per token overall. Per-component
token figures below are that ratio applied, which assumes a component tokenizes
like the average; JSON-escaped shell strings do not, so only the total is
measured.

| component | bytes | share | est tokens | client-side lever |
|---|---|---|---|---|
| `world_state.permissions` | 31,627 | 32.6% | 5,780 | **yes** — stale, inert, see above |
| pre-first-call messages | 25,532 | 26.3% | 4,666 | partly — 15,458 of it is harness developer text |
| `base_instructions` | 18,037 | 18.6% | 3,296 | no |
| `world_state.host_skills` | 13,858 | 14.3% | 2,532 | partly — 41 skills, `pages` listed 4×; `[[skills.config]] enabled = false` works |
| `world_state.agents_md` | 7,150 | 7.4% | 1,307 | **duplicate** — see below |

`AGENTS.md` is sent **twice**: `world_state.agents_md.text` (6,929 bytes) is a
strict substring of the user-role message that also carries it (7,403 bytes).
About 1,300 tokens per window are pure duplication. This is harness behavior,
not configuration, but it means every byte trimmed from `AGENTS.md` is paid back
twice.
