# Pi Host lifecycle protocol, version 1

This file is the protocol. Its SHA-256 is the protocol digest a lifecycle
session binds for the Pi Host realization (`native-realization`
`pi_host::PROTOCOL_DIGEST`). Changing a byte changes the protocol.

## Transport and parties

The controller (`lifecycle-service`) spawns the Pi Host (`work-engine-pi-host
serve`) and speaks JSON-RPC 2.0 to it, one JSON object per line, on the Host's
stdin and stdout. The Host runs Pi, through its pinned receiver and SDK, in
the Host's pinned Docker container (no network, read-only, Host-held
access). One container serves one thread for its whole life; each turn is
one Pi prompt in the same Pi session, so later turns see earlier ones. Every
model request Pi makes leaves through the Host, which retains its exact body
before any upstream byte and dispatches it on the job's provider route
(`claude-sdk` over Anthropic Messages, `openai-codex` over Responses). The
Host relays the model's tool calls to the controller. The Host is not Codex
and never answers Codex's handshake (`initialize`, `config/read`, account or
auth methods); it answers them with a JSON-RPC error.

## Requests from the controller

`pi-host/initialize` `{}` → `{"realization", "protocol_digest",
"evidence_class", "host_sha256", "job_sha256", "provider", "model",
"wire"}`. Must be the first request and is accepted once. `job_sha256` is the
digest of the admitted Host job the thread runs under; `wire` is `messages`
or `responses`.

`thread/start` `{"model", "dynamicTools", ...}` → `{"thread": {"id",
"ephemeral": false}}`. `model` must equal the job's model and `dynamicTools`
must equal the pinned tool set exactly (namespace `workspace` with
`snapshot_read`; namespace `task` with `question_open` and `result_publish`;
argument schemas as pinned in `pi_host::tools`). Other fields are ignored.
The reply is sent once the container's Pi session is ready; if the container
cannot start, the reply is an error and the Host exits. One thread per Host
process.

`turn/start` `{"threadId", "input": [{"type": "text", "text"}]}` →
`{"turn": {"id"}}`. Exactly one non-empty text input, sent to Pi unchanged as
the prompt. Turns are domain work: the Host-ledger items carry no
`userMessage`. At most one turn is active, and at most 16 turns per thread.
After a failed turn the thread accepts no further turn. Controller requests
sent while a turn is active are refused with an error while a relayed call
awaits its answer, and otherwise answered after the turn ends. End of the
controller's input closes the thread: between turns the Host closes Pi's
session and removes the container; during a turn the Host ends the job at
the next provider attempt or Pi frame, and the turn is reported failed.

`thread/read` `{"threadId", "includeTurns": true}` → `{"thread": {"id",
"historyMode": "legacy", "turns": [{"id", "status", "itemsView": "full",
"items": [...]}]}}`. Under this protocol `thread/read` is a **Host-ledger
view**: the turns and items are those the Host emitted, assembled from the
relayed calls and Pi's native events, not a read of Pi's session file.
`historyMode` carries the token the lifecycle native settlement grammar
requires; here it names this Host-ledger view. Pi's own session history for
each turn is retained by the Host and named by digest in the capture. The
lifecycle store refuses a raw frame above 1 MiB, so the Host fails a turn
(status `failed`) instead of reporting it completed when the `thread/read`
reply including that turn would exceed 1 000 000 bytes.

`pi-host/capture` `{"threadId", "turnId"}` → `{"capture_sha256", "path"}`:
the per-turn capture the Host wrote under its run directory, which the
controller owns and reads itself.

`pi-host/observe` `{"threadId"}` → schema `work-engine.pi-host-observation.v1`
`{"container_id", "container_name", "image", "running", "network_mode",
"readonly_rootfs", "user", "mounts", "auth_mount", "containment_verified",
"inspect_sha256", "observed_unix_ms"}`: a fresh inspection of the thread's
inference container, answered only if the Host's launch containment check
holds again (identity, image, labels, user, no network, read-only root,
capabilities, limits, exact mounts and environment), the container is
running and nothing is mounted at `/auth`; otherwise an error. It is evidence
for the controller's startup hold and grants nothing. Not answered during a
turn.

## Messages from the Host

Notifications, in order for a turn:

- `turn/started` `{"threadId", "turn": {"id"}}`.
- For each model tool call Pi executes, a server request `item/tool/call`
  `{"threadId", "turnId", "callId", "namespace", "tool", "arguments"}` where
  `callId` is Pi's tool call id, `tool` is the pinned tool name and
  `namespace` is `workspace` or `task`. The controller answers `{"success":
  bool, "contentItems": [{"type": "inputText", "text"}]}`; the Host returns
  the text to Pi unchanged as the tool result. Then `item/completed` with
  `{"type": "dynamicToolCall", "id", "namespace", "tool", "arguments",
  "status": "completed", "success", "contentItems"}`. A controller error or a
  malformed answer fails the turn.
- After the turn ends, for each non-empty assistant text block in stream
  order, `item/started` and `item/completed` with `{"type": "agentMessage",
  "id", "text"}` (`item/started` carries `"text": ""`). The last one is the
  final answer.
- `turn/completed` `{"threadId", "turn": {"id", "status", "error"}}` where
  `status` is `completed` and `error` is `null` only if the turn's capture is
  eligible; otherwise `status` is `failed` and `error` is `{"message"}`
  naming the first reason. `completed` is the Host's verdict; the
  controller's settlement applies its own rules (for example, a final text
  that equals an earlier text block of the turn).

The capture is complete before `turn/completed` is written.

## Capture

Schema `work-engine.pi-host-capture.v1`, one per turn: for every physical
send of the turn (each Host attempt of each Pi provider request) the retained
request body's path and digest as a payload (`inference_call_id`
`request-<n>`, `physical_send` the attempt number, `representation` `wire`,
`original_codec` `identity`); every attempt of each of the turn's provider
requests, including one never dispatched, as `attempts` (`request`,
`attempt`, `disposition`, `route`, `payer`); the retained response stream of each attempt by
path and digest; a calls bundle (`call_id`, `inference_call_id`,
`physical_send`, `response_id`, `namespace`, `tool`, `arguments`) where each
relayed call is joined to exactly one model tool call in the retained
response of exactly one completed attempt of the turn; the relayed result
texts by digest; Pi's session history artifact by digest; `realization`,
`evidence_class`, `provider`, `wire`, `contains_account_identifier` (false
only when every request body of the turn was read and passed the account
identifier check below), the identities the Host ran with (Host binary, job and its nonsecret route
table, provider, model) and the turn's prompt digest; the `pi-host/observe` observation taken when
the turn ended; and an eligibility verdict with reasons.
Eligible means: the turn ended with a final answer; every relayed call was
answered with `success: true` and joined; every physical send's body is retained with a matching
digest and passed the Host's request policy (model, pinned tools, body
bounds, no account identifier); every attempt relayed to Pi has its response stream retained; every
attempt is settled and none is uncertain; on `claude-sdk`, the receiver's
forwarder answered no Claude Code request locally, by the count the receiver
reports with each provider request and at the turn's end; the observation
succeeded; and the turn is within the bounds below.

## Account identifier

On the Messages wire the Host refuses, before dispatch, a request body whose
`metadata`, when present, does not have the closed shape Claude Code was
observed to send: an object with exactly the key `user_id`, whose value is a
string holding a JSON object without duplicate keys; that object's keys are
among `device_id`, `account_uuid` and `session_id`; `device_id` and
`session_id` are strings without `@`; and `account_uuid` is `null` or an
empty string. In addition, every string under `metadata` that is itself JSON
is decoded, strictly and at most four levels deep; JSON with duplicate keys,
or nested deeper, refuses. The body also refuses if, at any level, a key
names an email, an `account_uuid` holds anything other than `null` or an
empty string, or a string contains `@`. Device and session ids in
`metadata.user_id` remain and are retained with the body.

## Bounds

Per turn: at most 64 physical sends; at most 8 MiB of retained request bodies
in total; the final answer at most 64 KiB; the `thread/read` frame bound
above. The thread runs under the job's deadline and provider budget.
