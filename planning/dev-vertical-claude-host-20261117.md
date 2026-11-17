# Dev vertical: Claude Host core

2026-11-17, revision 1. Plan for the protocol-independent core of the Claude
Host recommended in
[the Claude realization note](research-lifecycle-h2-claude-realization-20261117.md)
(shape 2). Owner: dev vertical. Authority: the operator's direction to build
what H2 needs on a non-Codex route and the decisions relayed on
`aspect:research-lifecycle` (5845). This slice touches no research-lifecycle
crate and does not choose the frame vocabulary (placement A or B) the Host will
speak to `lifecycle-service`.

## Scope

New crate `rust/crates/work-engine-claude-host`, one binary:

- `run --job <job.json> --out <dir>`: one bounded turn. Starts the recording
  proxy and the relay socket, spawns Claude Code headless (stream-json, no
  built-in tools, one strict MCP server, no settings sources, no session
  persistence, the job's system prompt and model), sends the prompt, serves
  each relayed tool call through a tool-server interface, ends at Claude
  Code's `result` event or a deadline, reaps the child, writes the capture.
- `mcp-relay --socket <path> --tools <spec.json>`: the MCP stdio server Claude
  Code launches; it lists exactly the pinned tools and forwards each call to
  the Host over the socket. It executes nothing itself.
- `fixture-upstream --port <n>`: scripted Anthropic SSE upstream for offline
  qualification.

Proxy invariants: only `POST /v1/messages` is forwarded; each request body is
bounded, parsed, and checked against the job's pinned model and the exact
`tools` array (the relay tools plus Claude Code's `DeferredToolPlaceholder`);
the request count is bounded; the body is persisted create-only with its
SHA-256 in the ledger **before** any upstream byte; the response is retained as
streamed; a refused request gets an error response and is recorded; the
`Authorization` value is never written. The upstream is `api.anthropic.com`
over TLS, or a loopback fixture only when the job names one.

Capture (per turn): every request as a payload (`wire`, `identity`, digest,
`inference_call_id` = request ordinal, `physical_send` = 1), the calls bundle
(`call_id`, `inference_call_id`, `physical_send`, `response_id`, `namespace`,
`tool`, `arguments`) joined from the response streams and the relay, raw
stream-json and relay transcripts by digest, an outcome, and an eligibility
flag (`result` success; every request retained with a matching digest; tools
check passed on every request; no refused or failed request; every relayed
call matched to exactly one `tool_use` in a retained response).

The tool server for qualification is a snapshot server over a manifest-pinned
directory with the `snapshot_read` result shape `verify_delivery` parses, and
recording `question_open`/`result_publish` stubs. The lifecycle-facing tool
server and frames come later, after the owner's ruling.

## Acceptance

1. Unit tests: tools check (exact match, extra tool, changed schema), SSE
   parsing (tool_use assembly, message id, stop reason), ledger create-only,
   body bound, path refusal, chunked decoding.
2. Offline run set against the fixture upstream with the real Claude Code
   binary: completed turn with eligible capture; tools mismatch refused before
   forwarding (pinned spec differs); request bound exceeded refused; no
   `Authorization` value in any run file.
3. One live run on the subscription (small model) over the H2 treatment
   snapshot with a non-H2 qualification task: settled, eligible capture,
   rate-limit status allowed with no overage, no credential in run files.
4. Review on the Claude subscription (Opus xhigh, normal code review).
