# Native Claude transport

Keep `claude-code` as the harness for every provider selected by this skill.
Changing the inference gateway must not silently substitute Codex, an
OpenRouter agent loop, or another generic harness for Claude Code's system
prompt, tools, compaction, permissions, or session machinery.

## Transport identities

- `anthropic`: the ordinary native Claude route with no gateway override.
- `openrouter`: the same native `claude` executable using OpenRouter's
  Anthropic-compatible endpoint.
- `auto`: try `anthropic` once, then consider `openrouter` only when the
  launcher recognizes a quota or rate-limit failure.

The logical review provider may remain `claude`, but every receipt must record
the Claude Code version, gateway, requested model, launcher digest, command
digest, attempt results, and whether the upstream provider was actually
observed. A requested OpenRouter model is not evidence of its upstream route.

## Launcher

Use the deterministic launcher rather than reconstructing environment changes
at each call:

```bash
python3 skills/claude-recon-implementation/scripts/claude_transport.py \
  --transport auto \
  --continuity disposable \
  --allow-paid-failover \
  --receipt '<transport-receipt.json>' \
  --stdin-file '<prompt.txt>' \
  --stdin-sha256 '<prompt-file-sha256>' \
  --openrouter-model 'anthropic/claude-sonnet-5-20260630' \
  -- \
  claude -p \
    --effort medium \
    --model sonnet \
    --no-session-persistence \
    --tools 'mcp__codebase-memory-mcp,Read,Glob,Grep' \
    --output-format json \
    --json-schema '<schema>' \
    --dangerously-skip-permissions
```

Use the integrity-bound stdin route for generated or potentially large
prompts. The launcher verifies the exact UTF-8 file bytes before process entry,
forwards those same bytes to every permitted attempt on stdin, and records the
digest and byte count separately from the bounded command digest. This avoids
the operating system's per-argument size ceiling without weakening request
provenance. Supplying either stdin flag without the other fails before provider
entry.

`OPENROUTER_API_KEY` must be present only in the process environment. The
launcher supplies `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, an empty
`ANTHROPIC_API_KEY`, and `ANTHROPIC_DEFAULT_SONNET_MODEL` to the OpenRouter
attempt. It never stores the credential in a receipt.

Possession of `OPENROUTER_API_KEY` is not spend authority. Auto mode requires
`--allow-paid-failover` from the governing invocation before a quota failure
may cause an OpenRouter call. Selecting `--transport openrouter` is itself an
explicit paid-route selection.

Use `--transport openrouter` when the route is selected before provider entry.
Do not point this synchronous launcher at a `:batch` model. Live testing with
Claude Code 2.1.237 established that direct synchronous use returns 404 and
that OpenRouter's asynchronous Batch API requires a loopback transport. See
[native-claude-batch-transport.md](native-claude-batch-transport.md). Standard
and batch routes are distinct execution configurations even when their model
family and native Claude executable are the same.

### Research-grade upstream pinning

An `anthropic/...` model slug identifies the model family; it does not by
itself restrict OpenRouter to Anthropic's first-party endpoint. For a research
run, use a dedicated API key assigned to a guardrail whose provider allowlist
is exactly `['anthropic']` and whose model allowlist contains only the exact
requested model. Immediately before provider entry, observe both the guardrail
and its key assignment through OpenRouter's management API and normalize them
to
[openrouter-routing-attestation.schema.json](openrouter-routing-attestation.schema.json).

Produce that artifact with the read-only collector:

```bash
OPENROUTER_MANAGEMENT_KEY='<management credential>' \
OPENROUTER_API_KEY_HASH='<non-secret inference-key hash>' \
python3 skills/claude-recon-implementation/scripts/openrouter_routing_attestation.py \
  --guardrail-id '<guardrail UUID>' \
  --model 'anthropic/claude-sonnet-5-20260630' \
  --output '<routing-attestation.json>'
```

The collector issues only `GET` requests: one exact guardrail lookup and a
complete paginated read of key assignments. It fails closed unless the
guardrail permits only `anthropic`, permits only the requested model, and the
selected key hash has exactly one direct assignment to that guardrail. Keep the
management credential separate from the inference key; neither credential is
written to the artifact. The key hash is an identifier, not a secret.

Pass that fresh artifact with `--require-anthropic-1p`,
`--routing-attestation`, and the non-secret `OPENROUTER_API_KEY_HASH`. The
launcher checks the provider/model restrictions, key assignment, and freshness
before either side of an auto route runs. It records the attestation digest but
still reports the upstream provider as requested rather than observed. Bind
the post-run OpenRouter generation record separately to confirm which provider
actually served the request.

Do not treat a dashboard preference that merely prioritizes Anthropic as a
research pin: priority plus enabled fallbacks can still select another
provider.

## Failover boundaries

Automatic failover is not a generic retry policy. Authentication, permission,
tool, malformed-request, model, and repository failures return without a paid
fallback. A disposable request is replayable only with
`--no-session-persistence`; the launcher buffers the failed attempt so partial
output is not mistaken for the result.

A retained review has a stronger identity contract:

- A fresh retained entry using `--session-id` does not automatically replay.
  After a quota failure, either select OpenRouter with a new authorized
  reviewer/session binding or reconcile the existing episode explicitly.
- A remediation call using `--resume` may cross gateways only after a smoke
  test has established that the installed Claude Code version reopens the same
  local transcript correctly. Until then, omit
  `--allow-retained-resume-failover` and stop truthfully.
- If a cross-gateway resume fails or its continuity cannot be trusted, mark the
  episode uncertain and use the `independent-review-state` successor-writer
  path. Describe it as reconstructed continuation, never same-session review.

The local session ID preserves a Claude Code transcript reference. It does not
prove identical inference service, upstream provider, cache behavior, or
gateway processing.

## Exact-model guard (opt-in)

`--exact-model-guard` runs the same native `claude` through the `anthropic`
transport while the launcher reads its `stream-json` output as it is produced.
It exists for work that must run on one raw model identifier and must treat a
safety refusal or a model switch as terminal for that invocation, not a retry trigger.
Without the flag, nothing in this section applies and the launcher, receipt and
buffering are unchanged. The Claude builder skill enables it on every turn to
preserve the caller's model selection.

```bash
python3 skills/claude-recon-implementation/scripts/claude_transport.py \
  --transport anthropic --continuity retained --exact-model-guard \
  --receipt '<transport-receipt.json>' \
  --stdin-file '<prompt.txt>' --stdin-sha256 '<prompt-file-sha256>' \
  -- claude -p --session-id '<uuid>' --model claude-sonnet-5-5 --effort high \
     --output-format stream-json --verbose
```

### Preconditions

The guard refuses to start the child, writes a receipt with outcome
`unavailable` and exits 43, when any of these is false:

- `--transport anthropic` (no gateway or paid fallback);
- exactly one `--model`, a raw identifier such as `claude-sonnet-5-5`. Aliases
  (`sonnet`, `default`) and bracketed variants cannot be compared with an
  assistant message's `model` and are rejected;
- `--output-format stream-json` with `--verbose`;
- no `--fallback-model`;
- any caller `--settings` is one readable JSON object that does not set
  `switchModelsOnFlag` to anything but `false` or `availableModels` to anything
  but the singleton model.

### Scoped settings

The launcher merges the caller's `--settings` (file or inline JSON; none is
fine) with `{"switchModelsOnFlag": false, "availableModels": ["<model>"]}` and
passes one effective file with `--settings`, so user and project settings files
are not modified and the caller's other keys, permissions included, are kept.
Per [model configuration](https://code.claude.com/docs/en/model-config), the
first is meant to make a flagged request in non-interactive mode end with a
refusal rather than switching models, and a fallback target outside
`availableModels` is meant not to be used; the singleton is meant to constrain
native subagents too. The launcher does not observe Claude Code enforce either
key. The file is deleted afterwards. The receipt binds `source_sha256` (caller
settings), `effective_sha256` and `effective_sha256_after_run`; a difference
between the last two makes the run `incomplete_evidence`. Those digests bind
the one settings file this launcher supplied at the flag layer. They do not
attest the merged policy Claude Code actually applied: managed settings take
precedence over flag settings and `availableModels` merges across scopes, so a
managed or user layer can widen or override what the file says. Settings narrow
selection; they are not evidence of which model answered.

### What counts as evidence

| Stream record | Role |
| --- | --- |
| `assistant` message `model`, whether or not the record carries a `parent_tool_use_id` | Observation. Must equal the requested model byte for byte. |
| any record with `parent_tool_use_id` (for example `tool_progress`) | Counted neutrally in `observations.parent_tool_use_records`; the id is recorded on a finding as `parent_tool_use_id`. The marker is not evidence of a subagent and never a model observation unless the record is an `assistant` or `message_start` message. |
| `stream_event` `message_start` `model` (with `--include-partial-messages`) | Observation, available earlier than the final message. |
| `assistant` with `model` `<synthetic>` | Harness-authored notice (for example an API error). Counted with its `error` category (`observations.synthetic_assistant_messages`, `synthetic_error_categories`); never an observation and never a mismatch. It does not change the verdict by itself: a stream with exact observations, a notice and a successful result is `verified_exact` (exit 0). A notice with no exact observation is `incomplete_evidence`. The verdict speaks to the models of the messages observed, not to the turn's content being complete, so a consumer that treats a notice as a recovery decision reads the counts. |
| `system` `init` `model`, `result` `modelUsage` keys | Metadata only. On `--resume`, `modelUsage` is cumulative conversation history, so other models there do not describe this execution. |

Refusal signals: `stop_reason: "refusal"` on an assistant message, a
`message_delta`, or the result; `terminal_reason` containing `refusal`; and any
`system` subtype containing `fallback` or `refusal` (`model_refusal_fallback`).
The shapes of these signals were supplied by the assignment, and the
`<synthetic>` notice shape (`error` category, `stop_reason: stop_sequence`) comes
from one reported session's stream; the model-configuration and headless pages do
not document them, so a different spelling in a later Claude
Code release would go undetected by the signal checks (the model comparison is
unaffected).

### Outcomes and exit codes

`model_guard.outcome` is one of:

| outcome | meaning | launcher exit |
| --- | --- | --- |
| `verified_exact` | at least one observation, every observation exact, no signal, one well-formed result, stream drained and delivered, settings unchanged. Synthetic notices may be present (see the evidence table) | 0 when the native result is also a success |
| `violation` | a mismatching observation, refusal or fallback signal; `violation` holds its kind, line, byte offset, line digest, message id and raw (bounded) model string, never message text | 41 |
| `incomplete_evidence` | evidence cannot be read as complete: malformed, oversize or envelope-malformed line, missing model, no observation, no result, malformed or duplicate result, assistant/stream/result activity after the result, undrained or undelivered stream, modified settings, timeout, cancellation, a launcher failure (including a receipt that cannot be written); `reason` names it | 42, 124 (timeout), 128+signal (cancelled), or the native status if it failed by itself |
| `unavailable` | a precondition failed or the command did not start | 43 |

The native process status and the result event are reported separately
(`native.returncode`, `native.result_status`, `native.stopped_by_launcher`). A
zero exit never converts a finding into success, and an exact-model run whose
result has `is_error: true` or a non-`success` subtype is `failed` (exit 44)
even when the process exited 0. `receipt.result` is `success` only for exit 0.
`acceptance` is `not_invalidated_by_model_guard` only then.

A refusal or fallback is a terminal outcome for the invocation. The launcher never
replays or switches model after one; `--allow-paid-failover` has no effect here.
This ends the invocation, not necessarily its enclosing engineering objective.
For builder work, the caller owns any authorized successor decision and the
[failed-turn handoff](../../claude-builder/references/native-session.md#continuing-after-a-failed-turn);
the failed receipt remains failed even if a later builder succeeds.

### Process and stream control

The child starts in its own session and the launcher owns it from the moment it
exists: a failure while publishing the in-flight receipt, starting threads or
finalizing stops and collects the child before the launcher gives up. If the
receipt target cannot be written the run is not accepted: the child is stopped,
the launcher says so on stderr and exits 42 (a violation keeps 41). A final
receipt that cannot be written never leaves exit 0 and is a launcher failure: the
owned tree is stopped and collected even though the native process had already
exited, and the receipt is tried once more recording that cause.

The launcher is a Linux child subreaper, so descendants orphaned by `setsid` or
double fork reparent to it and stay discoverable. On a finding,
`--guard-timeout-seconds` or SIGINT/SIGTERM/SIGHUP to the launcher, it sends
SIGTERM, waits `--guard-stop-grace-seconds` (default 5), then SIGKILL to the
session and every descendant found. Cancellation and the deadline stay live
while the output drains (up to 5 seconds after the child exits) and while the
backlog is delivered; a signal received, or a deadline reached, at any point
before the final receipt write returns (a deadline crossed inside the write
rewrites the receipt as timed out) makes the run cancelled (exit 128+signal) or
timed out (124) and invalidates acceptance, even if the native process had
exited cleanly with every byte delivered. A timeout or cancellation stops
descendants, including one holding the output pipes open. A clean exit that left background processes reports them
without killing them.

`termination.process_collection` reports `none_remaining`,
`survivors_remaining`, `leftover_processes` or `not_established` (the subreaper
could not be set, as off Linux). Discovery is a `/proc` scan by session and
ancestry; it is best effort and describes only what the scans saw. It does not
reach a process that left the ancestry through something outside the launcher's
control (a service manager, `systemd-run`, a container runtime or another PID
namespace), a process created after the last scan, or a PID reused during the
stop; `tool_effects_before_stop` stays `not_established`.

Stdout and stderr are drained by separate threads, hashed, and passed through to
the launcher's own stdout and stderr as they arrive, so a stdin payload or
output of any size cannot deadlock the child. Judgment happens before
forwarding, and forwarding runs on its own thread with a bounded backlog
(`--guard-forward-backlog-bytes`, default 64 MiB). A consumer that stalls or
closes its pipe therefore never delays a stop or the final receipt. Each
stream's `delivery` account is frozen once, at the end, and reconciles:
`captured_bytes = delivered_bytes + undelivered_bytes`, and
`undelivered_bytes = dropped_bytes + pending_bytes + in_flight_bytes`
(`dropped`: refused over the backlog cap or after the consumer closed;
`pending`: queued, never written; `in_flight`: inside a write that had not
returned, of which a prefix may already sit in the pipe). `delivered_bytes`
means accepted by the launcher's own fd, not read by the consumer. Any
undelivered byte makes the run `incomplete_evidence` (`output_not_delivered`).
The stream the caller redirected is the attributable artifact; the receipt holds
its SHA-256 and `captured_bytes`. A receipt with `result: pending` and
`reason: in_flight` is written as soon as the child has a pid, naming it, so an
interrupted launcher leaves an explicit non-success record.

### Limits

- Detection reads a record after the child has emitted it. The guard stops the
  tree and invalidates acceptance, but it does not show that no tool call ran
  before detection or between detection and the stop; `lines_after_first_finding`
  and `tool_effects_before_stop: not_established` record that. After a violation
  reconcile the workspace and treat effects as present but unattributed.
- A message can be judged only if it appears in the stream. Lines over 16 MiB are
  not parsed and are `incomplete_evidence`.
- `identity.session_identity` reports whether observed `session_id` values match
  the requested one; a difference (possible on `--resume`) is reported, not
  judged.
- The synthetic tests (fake children emitting stream-json) cover the launcher.
  No real refusal, fallback or model switch was provoked, so enforcement of the
  scoped settings by Claude Code itself is not tested here; the observation checks
  are the independent control.
