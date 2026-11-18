# Probe: Pi with `claude-sdk` inside the Pi Host's Docker container

2026-11-17. Answers the research-lifecycle owner's open question (board 5874,
item 4) after the operator corrected the direction back to the Pi Host. Probe
only; no product code. Run files (request bodies, responses, ledger) stay in
`~/.local/state/work-engine/dev-vertical-h2-route-20261117/pi-claude-probe/`.

## Setup

- Image: the Pi Host's pinned `node@sha256:4b150b48…`; Pi 1.5.0 from the Pi
  Host's runtime root (read-only); the `pi-claude-subscription` extension, the
  Claude Agent SDK and its bundled `claude` binary from `~/.pi/agent/npm`
  (read-only). `--network none --read-only --cap-drop ALL
  --security-opt no-new-privileges --user 1000:1000`.
- [forwarder.mjs](forwarder.mjs) (the container's entry): a loopback HTTP
  listener, set as `ANTHROPIC_BASE_URL` for Pi; each request becomes one JSON
  line on the container's stdout, and response head, chunks and end come back
  on stdin. It writes a dummy credential (`sk-ant-oat01-DDD…`, far-future
  expiry, real scopes and plan type) into a tmpfs `HOME`, then runs Pi in
  print mode with `--model claude-sdk/claude-haiku-5-5` and the `read` tool.
- [host.py](host.py) (Host side): forwards only `POST /v1/messages` that carry
  exactly the dummy bearer, swaps in the real OAuth bearer, sends over TLS to
  `api.anthropic.com`, streams the response back, and retains each exact
  request body, the response, and the rate-limit headers. The real token is
  never written.

## Results

| Run | Limits | Result |
| --- | --- | --- |
| `run1` | pids 256, 2 GB, 2 CPUs, tmpfs exec 512 MB | Completed; correct answer; two `/v1/messages` requests |
| `run2` | **the Pi Host's own**: pids 64, 512 MB, 1 CPU, tmpfs `noexec` 64 MB | Completed; same |

- Both requests in each run: HTTP 200, `anthropic-ratelimit-unified-status:
  allowed`, `overage-status: rejected` (`org_level_disabled`): billing stays
  in the subscription plan with the Host-injected bearer.
- The container presented only the dummy bearer; the real token appears in no
  run file.
- Only the two model requests reached the forwarder; nothing else went
  through the base URL, and with `--network none` nothing else can leave. The
  extension sets `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`.
- **No account identifier in the requests**: with the dummy credential and no
  Claude config in the container, the bodies carry no email system-reminder
  and `metadata.user_id.account_uuid` is empty (the device id and session id
  remain). The earlier Claude Host route carried the email in every body.
- Request shape: Anthropic Messages; tools `DeferredToolPlaceholder` plus
  `mcp__pi__<tool>`; system blocks are the billing header, the Agent SDK
  identity and Pi's Claude prompt.

## Consequences for the Pi Host

The Claude CLI runs inside the container and all its inference goes through
the Host, as the OpenAI routes do: the receiver gains the loopback forwarder
(HTTP to framed fetch), and the Host gains Anthropic routes (OAuth bearer
injection for `claude-sdk`, `x-api-key` for the `anthropic` API route) with the
same retention, ledger and failover. Docker containment is unchanged.
