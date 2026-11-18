# Review: Pi Host W1-W4 (Anthropic Messages family, `claude-sdk`) — `c4a8ecfc`, `814e2436`, `8876b435` vs `e527c36e`

Reviewer: Fable 5.1, read-only, 2026-11-18. Inputs: `subject.diff`; working tree for
context (cited where consistent with the diff); live run `live/live-c1` and offline run set
`w4a` under `~/.local/state/work-engine/dev-vertical-h2-route-20261117/`; the pinned Pi
runtime, the `pi-claude-subscription` 0.2.4 source and the bundled `claude` binary under
`~/.pi/agent/npm`. Limitations: no shell (no `git show`, `cargo test` not run — supervisor
should run `cargo test -p work-engine-manual-host --lib` in `rust/`); the
`native-realization` crate is not in this checkout, so the canonical-rule copy could not be
compared with its source.

## Verdict: changes required (two P1), containment and custody accepted

## Correct (verified)

- **One admitted attempt per upstream request.** Receiver forwarder admits one `POST
  /v1/messages[?beta=true]` per open grant (`slot.used`, `hostedFrames.open`, `slot.until`;
  `pi-coding.mjs` L117-146) and answers everything else locally; the check-and-set runs
  synchronously before the first `await`, so concurrent requests cannot both pass. Host
  re-validates URL/method against the pinned catalog `baseUrl`, drops access headers
  (`x-api-key` added, `pi_coding_hosted.rs` L49-55), refuses `content-encoding`, parses
  Messages bodies strictly (`lifecycle_wire::parse_strict_json`, duplicate keys refuse,
  `lifecycle-wire/src/json.rs` L80-91), enforces the key allowlist, `tool_choice` auto,
  custom tools only, the pinned analysis tool digest and prefix continuity. Offline `w4a/ok`:
  retry count `"0"` on all four upstream requests, identical `bodySha256` on the failover
  re-dispatch, tools digest `4dc958d9…` on every request; `w4a/retry-head`: attempt 2
  Uncertain after the 500, reservation retained, no later request in 20 s.
- **Credential custody.** The container presents only the dummy
  (`0002-rpc-0.stdout` L9: `authorization: Bearer sk-ant-oat01-DDD…`); Host injects the real
  bearer at dispatch only; current and every earlier token are screened
  (`Credential::secrets`, test `messages_routes_need_their_anthropic_access`); access is
  re-read per connection with a 5-minute margin. My scan of `live-c1` for
  `sk-ant-o[ar]t01-[^D]` and `sk-ant-api` found nothing. `account_uuid` is empty in the
  retained body. Dummy credential is harmless (far-future expiry; Claude Code never refreshes
  it; `childEnv` strips `ANTHROPIC_API_KEY`/`AUTH_TOKEN`).
- **Retry switches** `CLAUDE_CODE_MAX_RETRIES`, `…DISABLE_NONSTREAMING_FALLBACK`,
  `…NO_MODEL_FALLBACK`, `…DISABLE_MODEL_ACCESS_FALLBACK`, `…DISABLE_REFUSAL_FALLBACK`,
  `…DISABLE_REFUSAL_RETRY`: all six strings are present in the bundled binary (grep).
- **Settlement arithmetic at the implemented rates** reproduces every recorded figure:
  w4a 450 = ⌈40·0.1+3000·0.125+1000·0.01+120·0.5⌉+1; live 227, 603, 7 960 likewise; stop
  reason agreement (`stop`↔`end_turn|stop_sequence`, `toolUse`↔`tool_use`); `error`
  events fail the relay; `message_stop` under the tail rule. 945 001 = ⌈10⁶·0.625 +
  128 000·2.5⌉+1 where `rates()` takes the max of input/cacheRead/cacheWrite over base and
  tiers (`pi_coding_provider.rs` L620-643, `full_reservation` L711-720) — see P1-1 for why
  that is still not the worst case.
- **Billing guard** tested for allowed / allowed_warning / overage allowed / rejected /
  missing; all three live heads: `unified-status: allowed`, `overage-status: rejected`
  (`coding-hosted-{1,2,3}-1.relay.json` `headHeaders`). A tripped guard closes the attempt
  Uncertain, sets `provider_failed`, and new grants are refused while a reservation is
  unsettled.
- **Record claims checked and supported:** 8 new tests (6 in `pi_coding_hosted.rs`, 2 in
  `pi_coding_https.rs`); the `pi-ai` fetch-override correction (`dist/api/anthropic-messages.js`
  L424, L770-836 pass `fetch` to the SDK clients); analysis grammar accepts only `text` and
  `toolCall` (`lifecycle-store/src/pi_analysis_proof.rs` L124-128); live request 2/3 streams
  carry `thinking` blocks (`coding-hosted-2-1.stream.sse`); two Host deliveries in `w4a/ok`;
  live ledger and attempt dispositions match `analysis-host-result.json`.
- **Continuity across failover** is correct: `last_messages` is set when the
  `provider_fetch` is accepted (`pi_coding_hosted.rs` L673-675), and failover re-dispatches
  the retained `Fetch`, so the rule is applied once per logical request.

## Findings

### P1-1 — Messages settlement understates one-hour cache writes; the 945 001 reservation is not a worst case
- **Where:** `pi_coding_provider.rs` `messages_cost` (L661-697: `cache_write · rate(tier,
  "cacheWrite")`), `rates()`/`full_reservation` (L620-643, L711-720), `pi_stream.rs`
  `messages_usage` (diff L483-531: reads only `cache_creation_input_tokens`).
- **Evidence:** Claude Code requests 1-hour caching on every cached block
  (`coding-hosted-1.body.json`: `"cache_control":{"type":"ephemeral","ttl":"1h"}` on the
  system blocks and the system message; header `anthropic-beta: …extended-cache-ttl-2025-04-11`).
  The stream reports it: `coding-hosted-2-1.stream.sse` `message_start.usage.cache_creation
  = {"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":1058}`. Anthropic bills 1-hour
  writes at 2× base input (5-minute at 1.25×); Pi's catalog `cacheWrite` is the 5-minute rate
  (0.125 / tier 0.625). The Host settled request 3 at 7 960 µUSD (58 868 writes × 0.125);
  Pi's own reckoning in the same receipt (`usage.value.cost.cacheWrite = 0.0117736`, i.e.
  0.2/M, `cacheWrite1h: 58868`) totals 12 373 µUSD. So a stream settled `Completed` with
  usage priced ~36 % below list. Worst case under 1h writes at the tier: 10⁶ × 1.0 +
  128 000 × 2.5 = 1 320 001 > 945 001; 32 such requests = 42.2 M > the 32 M ceiling.
- **Consequence:** on `https:anthropic-oauth` the ledger is a proxy (not dollars) but the
  budget guarantee is wrong; on `https:anthropic-api` (added by this diff and permitted for a
  `claude-sdk` plan) it is real money and an overrun fails the job only after it is spent.
  The record's "full reservation … upper tier" and "settled … at catalog rates" statements
  are therefore misleading.
- **Fix:** in `messages_usage`, also pick `cache_creation.ephemeral_1h_input_tokens` /
  `ephemeral_5m_input_tokens` (from `message_delta` when present, else `message_start`);
  in `messages_cost` price 1h writes at 2 × `input` of the chosen tier, and when the
  breakdown is absent treat all writes as 1h (conservative); in `rates()` include
  `2 × input` so `full_reservation` becomes 1 320 001; raise `MESSAGES_ANALYSIS_BUDGET`
  (≥ 32 × 1 320 001) or lower the Messages request maximum; update `analyze.py`
  `BUDGET` and the record. Add a unit test with a 1h breakdown.

### P1-2 — Receipt provenance names the wrong provider for `claude-sdk` jobs
- **Where:** `pi_docker.rs` L1382-1386 hard-codes
  `container_provider_identity: "openai-codex"` and `cost_evidence: "pinned openai-codex
  catalog rates applied …"`.
- **Evidence:** `live-c1/state/analysis-host-result.json` carries exactly those two values
  for a job whose plan is `provider: "claude-sdk"`, catalog `anthropic.json`, routes
  `https:anthropic-oauth`.
- **Consequence:** a durable receipt misstates provider identity and cost provenance (core
  contracts §1.1: "provider identity and configuration provenance" is an invariant).
- **Fix:** `value["container_provider_identity"] = json!(plan.provider)` and a per-wire
  `cost_evidence` string naming the pinned catalog file; add a receipt assertion for a
  `claude-sdk` plan.

### P2-1 — Requests refused locally by the forwarder leave no evidence
- **Where:** `pi-coding.mjs` `forwardRefuse` (L120) and its callers (404 / 503 / 413 / 415 /
  502) emit nothing; `complete.counts` has only `providerRequests`/`resets`.
- **Why it matters:** the packet asks for nothing that "could leave a request unaccounted".
  Claude Code can issue a request outside the grant model — its own continuation after
  `max_tokens`, its mid-stream retry (`pi-claude-subscription/src/provider.ts` readStep
  comments, L255-262), an auxiliary call — and the Host cannot tell from any retained file
  whether that happened. Containment holds (nothing leaves), accounting does not.
- **Fix:** count refusals by reason in the receiver and emit them in the `complete` frame
  (`counts.forwarderRefused`), optionally one observation frame per refusal; have the Host
  journal the count.

### P2-2 — Billing guard accepts a missing `overage-status` header
- **Where:** `pi_coding_hosted.rs` `subscription_guard` (~L1611-1651):
  `matches!(overage, None | Some("rejected"))`.
- **Evidence:** every head observed so far (probe runs, live-c1 3/3) carries
  `anthropic-ratelimit-unified-overage-status: rejected`, so requiring `Some("rejected")`
  costs nothing on the operator's org and is strictly fail-closed. Open question I could not
  settle from repository evidence: whether `overage-status: allowed` means the request drew
  extra usage or merely that overage is available; if the latter, an org with overage
  enabled would be falsely refused even inside its limits. Record the semantics assumed.
- **Fix:** require `Some("rejected")`; document the assumption on `allowed`.

### P2-3 — Host accepts an absent `x-stainless-retry-count` (record says "other than 0")
- **Where:** `pi_coding_hosted.rs` L762-767: `is_some_and(|v| v != "0")`.
- **Evidence:** Claude Code's SDK sends the header on every request (live L9, w4a receipts),
  so `== Some("0")` is safe; otherwise correct the record wording.

### P3-a — Thinking attribution in the record is imprecise; "cannot be turned off" is untested at the API
- `records/…/README.md` L278-290 attributes the adaptive thinking to the catalog's
  `forceAdaptiveThinking`. On the `claude-sdk` path the extension's model config carries no
  `compat` (`models.ts` `claudeModels`, L14-28) and `thinkingOptions("off")` would send
  `{type:"disabled"}` (`models.ts` L39-51). The live body shows `thinking.type: adaptive`
  with `output_config.effort: low`, so Pi resolved `off` to `low` before the provider — the
  operative fact is the catalog `thinkingLevelMap` without an `off` entry. The qualified claim
  ("as far as these catalogs show") is supported; whether the API accepts `thinking.disabled`
  for `claude-haiku-5-5` was not tried. Suggest rewording and naming the untested step.

### P3-b — Canonical rule copy: not comparable here; semantics as implemented
- `pi_coding_hosted.rs` `canonical` (L468-512): strips `cache_control` at any depth
  (including inside tool inputs — harmless for comparison), string `content` ⇔ one text
  block, requires `role` on every message, strict `Value` equality otherwise (1 vs 1.0 differ
  → fail-closed), `next.len() > prev.len()`. `last_messages` is in-memory only (not
  journaled), acceptable because a Host restart does not resume a hosted job. Faithfulness to
  `native-realization` on `claude-realization-20261117` could not be checked in this
  checkout; the unification note in the code and record is the right disposition.

### P3-c — The `https:anthropic-api` route for a `claude-sdk` body is untested
- The body and headers are OAuth-shaped (`anthropic-beta: oauth-2025-04-20,…,
  claude-code-20250219`, the `x-anthropic-billing-header` system block, `?beta=true`). "Both
  endpoints receive the same body" is true by construction, but acceptance and billing on
  the API-key endpoint are unverified, and P1-1 applies there in dollars. Do not enable that
  failover live before P1-1 is fixed and one offline/live check exists.

### P3-d — Autonomous Claude Code continuation can never complete on this path
- A `max_tokens` stop makes Claude Code issue a second request in the same turn
  (`provider.ts` L255-262); the forwarder refuses it (503) and the Host also refuses the
  `length` stop reason, so the job fails closed. Correct containment; worth stating in the
  record as a known limit (max_tokens is 128 000 so it is unlikely).

### P3-e — Minor
- `hosted_close_uncertain`/`hosted_stream_end` fall back to `Wire::Responses` when the plan is
  unreadable (`unwrap_or(Wire::Responses)`), which would mislabel `relay.json` usage;
  unreachable in practice but `wire_checked()?` would be cleaner.
- No receiver unit test covers the forwarder (only sandbox stubs were added to
  `pi-coding-hosted.test.mjs`); coverage is the w4a run with the real binary — acceptable,
  say so in the record.
- The live receipt lists four cascade failures after the gate conflict; the record names only
  the gate failure — fine, but a half-sentence would avoid a reader thinking they were
  omitted.

## Answers to the packet's questions
- Extra upstream requests: none found; two independent layers (forwarder, Host) each admit
  one request per grant/stream; encodings and duplicate keys are refused; auxiliary paths get
  404/503 locally (but see P2-1 for evidence of those refusals).
- Access material: none reaches the container, journal or retained files on the evidence
  reviewed; the dummy is inert.
- Settlement: extraction and stop-reason agreement are right; tier choice is conservative;
  **cost can be understated and the reservation is not a worst case under 1-hour cache
  writes (P1-1).**
- Billing guard: fail-closed on every observed case; the absent-overage acceptance is the one
  soft spot (P2-2); no false refusal observed.
- Canonical continuity: per-job application including failover is correct; faithfulness to
  the branch not checkable here (P3-b).
- Receiver races/hangs: completion correctly waits for `response-end`; an `onSettled` that
  never fires (frame-reader error) would hang until the Host deadline — fail-closed, bounded
  by the job deadline; no unaccounted *upstream* request, but local refusals are invisible
  (P2-1).
- Record: supported except the cost/reservation statements (P1-1), the receipt's provider
  identity (P1-2), the retry-header wording (P2-3) and the thinking attribution (P3-a).
