# Pi Host capture hardening: F2 and U3 (Host side)

Status: implemented on branch `pi-host-capture-hardening-20261120` (from
`e272a8c2`). Operator direction 2026-11-20: "Please proceed with F2 and U3".
Owner: dev-vertical (`work-engine-manual-host`, `native-realization` Pi Host
protocol). Consumer: research-lifecycle (`lifecycle-service` `check_capture`,
store, task host).

## Sources

- F2 and U3 come from the research-lifecycle next-plan synthesis
  (`research-lifecycle-tool-registry-20261116/035-next-plan/SYNTHESIS.md`) and
  its proxy survey (`C-proxy-plugins.md` §2–§4), and from board 6015 (e).
- F2: non-Messages captures started `contains_account_identifier: true`
  without any check. The consumer refuses the flag only on Messages.
- U3: C1 hardening at the proxy. The parts are: (1a) a per-attempt request
  head; (1b) the Completions original body; (2b) card association at
  retention; (3b) live crossing via `provider_attempt`.

## Findings that shaped the scope

- The consumer (`check_capture`, `operations/pi_host.rs`) already refuses
  every non-Messages payload. F2 therefore leaks nothing into the store
  today; it matters once the consumer admits other wires.
- The consumer reads only the capture and payload fields it knows, so new
  fields are additive for it.
- `Identity::wire()` in `pi_host_serve.rs` labelled the OpenRouter lane
  `responses`. It now reports `completions`. Separately, `serve` cannot join
  tool calls in Chat Completions streams (`response_calls` knows Messages and
  Responses only), so a Completions turn through `serve` is still never
  eligible. That is a separate gap, not changed here.
- Pi's clients: Codex Responses sends `prompt_cache_key` (Pi's session id) and
  no `user`, `safety_identifier` or `metadata`. The Completions client sends
  none of those (`prompt_cache_key` only toward `api.openai.com` or with long
  cache retention). The Completions lane's body policy already admits a closed
  key set; the Responses lane has no top-level key allowlist.

## What changed

### F2: account identifier on every wire

- `native-realization` `pi_host::account_identifier_absent(wire, body)`,
  shared by the Host's capture and available to the consumer's re-check:
  - Messages: unchanged (`messages_account_identifier_absent`).
  - Responses and Completions: the body is an object; every top-level key is
    in the wire's closed list (`RESPONSES_ACCOUNT_KEYS`,
    `COMPLETIONS_ACCOUNT_KEYS`, the keys Pi's clients can send);
    `prompt_cache_key`, when present, is a string; and no value outside the
    model-visible content (`input`, `instructions`, `messages`, `tools`)
    names an email, holds a non-empty `account_uuid`, or contains `@` (JSON
    strings decoded as for Messages).
  - Any other wire: not verified.
- The capture's `contains_account_identifier` is now false only when every
  retained body of the turn passed this check, on every wire. On Completions
  the retained original (below) is checked too.
- Pre-dispatch refusal is unchanged: the Host refuses before dispatch only on
  Messages. On the other wires the flag records the check. This changes no
  live-lane behaviour.
- `PI_HOST_PROTOCOL.md` "Account identifier" states the rule;
  `PROTOCOL_DIGEST` is now
  `88eb5ac1b96cdb137d7e7d2f961dce5249ac2c582ab4b3be3f2d9ba76b5da69c`. Stores
  or records pinned to the previous digest (`56c00a88…`, e.g. the H2 records)
  are historical.

### U3 (1a): per-attempt request head

- `HostedProviderTransport::request_head(route, request, redacted)` returns
  the exact HTTP head `send` writes. With `redacted`, each access header
  value is `[redacted]`, including the ChatGPT account id. `send` uses the
  same function, so the retained head equals the transmitted head except for
  those values (tested for every credential kind).
- Before connecting, the relay retains
  `coding-hosted-{n}-{k}.request-head.json` (schema
  `work-engine.pi-host-request-head.v1`: request, attempt, route, the redaction
  marker, the head) and records its digest as
  `AttemptRecord.request_head_sha256`. If the head cannot be retained, the
  attempt closes uncertain and nothing is sent.
- The capture payload carries `request_head` (path, digest). A dispatched
  attempt whose head is missing or differs from its recorded digest adds an
  eligibility reason.
- This closes C-proxy-plugins gap 1(a): `anthropic-beta`, `anthropic-version`,
  `x-stainless-*`, `user-agent` and the other allowlisted headers are now
  attempt evidence, bound before dispatch.

### U3 (1b): Completions original body

- When the Host rewrites the Completions model to the transmitted slug, it
  retains the container's body as `coding-hosted-{n}.original.body.json`
  beside the forwarded `coding-hosted-{n}.body.json` (create-only,
  registered).
- The capture payload carries `original` (path, digest, `rewrite`
  `completions-transmitted-model`); it is null when the forwarded body equals
  the container's.

## Not done on the Host alone: proposals for research-lifecycle

- **(2b) Card association at retention.** The Host never sees the card or
  the reconstruction; the task host does. A Host-only change cannot record
  "this physical send carried card spans X..Y". Proposal: the task host
  computes the card digest and spans over the retained payload (it already
  re-reads it in `verify_delivery`) and records them in
  `NativeInputRecord.metadata` at retention. If the Host should help, the
  smallest useful Host addition is the byte offset of each `tool_result`
  block in the retained body, which the task host can match to a
  snapshot-read result. That needs an agreed field name in the capture.
- **(3b) Live crossing.** `CodingEffectGate::provider_attempt` already runs
  inside the Host before dispatch, under the Controller. Making the
  crossing visible to the lifecycle actor live needs a new Controller frame
  (for example `pi-host/attempt` with request, attempt, route, payer, body
  and head digests, before the write) and the actor's acceptance of it. Both
  sides change, and the protocol digest changes. Proposal: research-lifecycle
  decides whether the actor should admit (blocking) or only record
  (notification) and names the frame; dev-vertical then implements the Host
  side.
- **Consumer.** `check_capture` can call
  `pi_host::account_identifier_absent(expected.wire, body)` for every wire
  and refuse a capture whose flag is not false (RL's stated plan, 6015). It
  can also re-read and re-digest `request_head` and `original`.
