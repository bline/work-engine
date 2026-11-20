# Pi context routes: long wait, session-file reading and epoch rollover, 2026-11-16

Headless `pi -p` (Pi 1.5.2, `pi-context-epochs`, `pi-claude-subscription`,
`pi-subagents` loaded) on four inference routes, each with tools limited to
`bash`, `new_context`, `context_notes` and `context_history`, no AGENTS.md,
skills or MCP. Each run read about 30–48k tokens of repository text through
three `cat` calls, ran one 130-second chatboard `wait` on a private
`probe:pi-route-wait-<route>` topic (no posts), then called `new_context`.
The reader was the pinned `work-engine-context-status` `d5209c61…`
(source `0933a91e`). Small models were used on purpose; this establishes route
mechanics, n = 1 per route, not model recovery quality.

| Route | Model / api | 130 s wait | Rollover | Reader after |
| --- | --- | --- | --- | --- |
| claude-sdk (subscription) | `claude-haiku-5-5` / `claude-agent-sdk` | 131.3 s, `wait_exit=2` (run 1) | epoch 1; 48,511 → 3,437 (run 2) | epoch 1, 3,437 |
| Anthropic API | `claude-haiku-5-5` / `anthropic-messages` | 131.0 s, `wait_exit=2` | epoch 1; 48,492 → 3,110 | epoch 1, 3,110 |
| OpenAI API | `gpt-6-luna` / `openai-responses` | 130.8 s, `wait_exit=2` | epoch 1; 30,129 → 1,652 | epoch 1, 2,111 |
| Codex quota | `gpt-6-luna` / `openai-codex-responses` | 130.9–132.8 s, `wait_exit=2` (×8) | 8 epochs (loop, below) | epoch 8, 30,111 |

Token figures are the reader's `input_tokens` (input + cacheRead +
cacheWrite). On every route the reader's value equalled the session file's
sum for the latest usable request, and `recorded_model`/`recorded_api` named
the route. The model passed no `timeout` argument; Pi's `bash` has no default
timeout and, on claude-sdk, the provider lifts Claude Code's MCP tool timeout.
In the interactive claude-sdk root session, a 150-second chatboard `wait`
returned on a delayed post at +150 s (board sequence 5760).

## Findings

**Codex-quota recovery loop.** After each rollover the model re-executed the
whole probe from the boundary marker's verbatim latest user request, including
another `new_context` call, until the run was stopped after eight epochs and
51 requests. The same model on the OpenAI API read `context_history` twice and
stopped. The probe's request itself asked for `new_context`, so this is a
worst case, but the mechanism applies to any task: a recovering agent that
treats the quoted request as fresh assignment state repeats completed work,
including non-idempotent effects. The skill's statement that the quoted
request is a continuation anchor, not assignment state, addresses this; it
does not prevent it.

**Silent no-boundary rollover.** In a first claude-sdk run the 30k-token
filler arrived as one user message. `new_context` acknowledged the request,
but no compaction entry was written and no notice reached stderr; the extension
reports rollover failure only through an interactive UI. With the same text
spread over three tool results the rollover succeeded. A tool acknowledgement
therefore does not establish a boundary; the reader's epoch does.

**Print mode completes the continuation.** On the three non-looping routes
`pi -p` ran the continuation turn in the new epoch and exited 0.

## Not established

Behavior when a user types during a long tool call; automatic threshold
rollover (only model-triggered rollover was exercised); semantic recovery
quality; larger models; the claude-sdk Claude Code session lifecycle across a
rollover. Spend recorded by Pi's catalog: Anthropic API $0.0075, OpenAI API
$0.0047; the claude-sdk and Codex-quota runs are subscription-billed.

Session files, stdout/stderr, per-run `analysis.json` and the probe scripts are
retained locally at `/home/bline/cbm-work/pi-context-routes-probe-20261116/`.
