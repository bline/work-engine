# Native Claude batch transport

This is an experimental, manually selected transport for unattended,
disposable native Claude Code calls. It keeps the real `claude` executable and
agent loop while adapting each synchronous Anthropic Messages turn to
OpenRouter's asynchronous Batch API. It is not an automatic quota failover and
does not yet satisfy retained-review continuity.

The implementation is
[`claude_batch_loopback_proxy.py`](../scripts/claude_batch_loopback_proxy.py).
It binds only to loopback, requires a separate bearer token for the local
Claude process, keeps the OpenRouter credential inside the proxy process,
requires a fresh Anthropic-1P routing attestation, and emits content-free
events. Prompts, tool schemas, responses, and credentials are not written to
the event log.

## Established compatibility boundary

Live tests on 2026-08-29 used Claude Code 2.1.237 and
`anthropic/claude-sonnet-5-20260630`, pinned by an OpenRouter guardrail to the
Anthropic first-party provider.

- Direct synchronous Claude Code use of a `:batch` model returned 404.
- OpenRouter Batch accepts Anthropic-format `/v1/messages` items, but its batch
  validator rejects Claude Code's `context_management` field.
- Use the canonical model ID without a `:batch` suffix inside the asynchronous
  batch envelope. The Batch endpoint establishes the pricing tier.
- Native Claude Code succeeds through the batch proxy when launched with
  `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1`. A paid smoke completed in 112
  seconds with `service_tier: batch`; OpenRouter charged $0.01101 for 10,870
  prompt tokens and 28 completion tokens.
- A paid two-turn tool smoke completed in 279 seconds: batch 1 returned a
  `Read` tool call, native Claude Code executed it, and batch 2 consumed the
  tool result and returned the exact sentinel. OpenRouter charged $0.0245154
  across the two batches; Claude Code's standard-price estimate was
  $0.09056745.
- A semantic-preserving hybrid is available with
  `--realtime-context-management`, but the context-managed call was the main
  model turn in the tested Claude version. That mode preserved behavior but
  did not provide meaningful batch savings for the main inference.

The compatibility environment variable is an observable harness configuration
change. Claude Code removes Anthropic beta headers and beta tool-schema fields,
disables MCP tool search by default, and loads all MCP tools upfront. Standard
tool fields and prompt-cache controls remain. Record this route as `native
Claude Code; experimental betas disabled`, never as default-harness parity.
Ordinary Claude auto-compaction has separate controls and is not disabled by
this setting, but server-side beta context editing is unavailable.

## Start the proxy

First produce a fresh routing attestation as described in
[native-claude-transport.md](native-claude-transport.md). Then provide a random,
per-run loopback token and start the proxy:

```bash
CLAUDE_BATCH_PROXY_TOKEN='<random local bearer token>' \
OPENROUTER_API_KEY='<inference credential>' \
OPENROUTER_API_KEY_HASH='<non-secret key hash>' \
python3 skills/claude-recon-implementation/scripts/claude_batch_loopback_proxy.py \
  --port 0 \
  --batch-model 'anthropic/claude-sonnet-5-20260630' \
  --guardrail-model 'anthropic/claude-sonnet-5-20260630' \
  --routing-attestation '<routing-attestation.json>' \
  --collection-window-seconds 2 \
  --event-log '<content-free-events.jsonl>'
```

The readiness object printed on stdout contains the ephemeral base URL. The
default poll timeout is 3,600 seconds and covers queueing plus inference. A
timeout stops local waiting; it never authorizes resubmission of a paid batch.
OpenRouter may transiently return 404 while a valid batch is still processing;
the proxy records and retries that poll without submitting again.

That no-resubmission guarantee is scoped to one live proxy process. Batch IDs
and digest coordination are not yet durable across a crash or restart. After an
unclean stop, inspect and reconcile every submitted batch ID in the event log;
do not restart and replay the Claude request. Automatic failover remains
prohibited until a durable reattachment ledger closes this boundary.

## Run native Claude Code

Use an isolated, frozen `CLAUDE_CONFIG_DIR`. The local bearer token is not the
OpenRouter key:

```bash
CLAUDE_CONFIG_DIR='<isolated config>' \
ANTHROPIC_BASE_URL='http://127.0.0.1:<printed port>' \
ANTHROPIC_AUTH_TOKEN='<same local bearer token>' \
ANTHROPIC_API_KEY='' \
ANTHROPIC_DEFAULT_SONNET_MODEL='anthropic/claude-sonnet-5-20260630' \
CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1 \
CLAUDE_CODE_DISABLE_TERMINAL_TITLE=1 \
claude -p \
  --model sonnet \
  --no-session-persistence \
  --output-format json \
  '<bounded prompt>'
```

Disabling terminal-title generation removes a non-review background model call
in `claude -p`. It does not alter the review prompt. Do not add the experimental
beta switch silently to the ordinary realtime launcher.

## Evidence and current limitations

The event log records request digests, tool counts, serialized tool-schema byte
counts, deferred-tool counts, context-management presence, batch IDs, poll
state, request counts, and OpenRouter's aggregate usage and cost. It contains
no request or response content. Claude Code's final JSON may estimate standard
model cost even when its returned service tier is `batch`; use the terminal
Batch API usage record as billing evidence.

The proxy preserves Messages bodies except for setting the selected canonical
model and `stream: false`, then reconstructs Anthropic SSE from the final
Message. Text, tool-use, and signed thinking blocks are covered by deterministic
tests. Unsupported content blocks fail closed before a synthetic success is
emitted. Retries with an identical request digest join the original result,
and concurrent distinct turns are collected into one batch.

Before production or research use beyond a disposable review, add and retain:

- a multi-turn and compaction test under the exact frozen config;
- disconnect/restart recovery for already-paid batch IDs;
- bounded cache eviction and maximum microbatch size;
- an immutable execution receipt binding Claude version, config digest,
  compatibility flags, routing attestation, event-log digest, batch terminal
  record, and exact review subject.

Until those gates pass, do not auto-select this transport after a subscription
quota error. Manual selection may use it for disposable work when the caller
accepts the explicit beta-disabled harness configuration and asynchronous
latency.
