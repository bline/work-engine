# Dev vertical: streamed native Pi bridge, 2026-11-16

Standing: bounded offline implementation result from the Claude takeover
(pi harness, `claude-opus-5-5`, session `01a1e2a4-f353-743a-ba23-f42394958125`).
Not acceptance of the vertical, not independent review, not live qualification.

## Authority

The operator instructed this session to take the
[handoff](../dev-vertical-claude-handoff-20261115/README.md) and continue where
the paused Codex session left off, with a $40 session API-credit budget. That
budget is this session's own inference cost; it is not provider/live-route
funding. No credentials, provider inference, push, or deploy were used.
The inherited Astra/xhigh exact-subject review contract is unchanged; this
session did not perform or replace it.

## Phase 1 target (this slice)

Supported request: the existing native Pi `openai-codex-responses` coding
request (tools `write`/`bash`, history, function-call continuation) admitted by
`pi_native::request`, which reconstructs host authority. Result: an incremental
host→native response stream. Route: one host-owned synthetic route (the root
observer). Route pair, selection, and failover remain phase 3.

## What changed

- `rust/crates/work-engine-manual-host/src/pi_stream.rs` (new): consumer-driven
  relay. Frames: host `response-head`; native `pull`/`cancel` with ordinals; host
  `response-chunk` (≤16 KiB), `response-end`, `response-failure`,
  `response-cancelled`. One upstream read per pull bounds host buffering.
  Incremental HTTP/1.1 reader (content-length, chunked, close-delimited; rejects
  ambiguous framing, non-identity encoding, trailers). Outcomes:
  `completed`, `released-after-upstream-end`, `cancelled-before-upstream-end`,
  `interrupted-uncertain`. Normalized usage (cache/reasoning fields kept null when
  missing).
- `review-execution-evidence::pi::LiveStore::stream_hold`: incremental original-A
  screening. Withholds exactly the longest suffix that is a proper prefix of A, so
  no forwarded byte can belong to a later-completed occurrence; A stays inside
  the store. `stream-NNN` control labels added.
- `receiver/pi-native-tools.mjs`: native `fetch` returns a `ReadableStream`
  (highWaterMark 0) driven by `pull`; any open stream is released (`cancel`)
  before another envelope, so the host never sees interleaved frames.
- Probe/observer: probe relays through `pi_stream` and records per-stream
  evidence; observer serves chunked SSE with 150 ms gaps, logs send times, and
  has a `PI_OBSERVER_FAULT=truncate-first` control. Probe prints its error code.

## Evidence (same input digests: [stream7-inputs.sha256](stream7-inputs.sha256))

- Unit: `tests/pi_stream.rs` 8/8 — early chunk forwarded while a real TCP
  endpoint withholds the remainder; split-A never forwarded; truncated/malformed
  upstream → interrupted; cancel; slow-reader bound; foreign/interleaved refuse.
- Actual Docker native Pi, `stream7`: probe rc 0. Three prompts, two native tools
  (write, bash build + 3 tests pass), checked export, three snapshots, three fresh
  sealed readers; observer 6 receipts / 5 deduplicated effects. All five streams:
  native consumer demanded chunks 2–5 before the endpoint wrote its final event
  (~590 ms of endpoint output still pending after chunk 1 was forwarded); host
  pull waits ~140 ms show consumption tracking the open endpoint
  ([join](stream7-join.json)). Stream 1→2→3 within prompt 1 is the tool round
  trip and continuation requests.
- Fault `fault-truncate2`: endpoint closes mid-stream. Relay outcome
  `interrupted-uncertain` (`upstream_truncated`); actual Pi session recorded an
  assistant error `relay_upstream_truncated`; registry `provider:1` remains
  `unknown`; container removed. [evidence.json](evidence.json) binds both runs.
- Earlier runs `stream1`–`stream6`, `fault-truncate1` under
  `/home/bline/.local/state/work-engine/dev-vertical-pi-20261116/` are retained
  as development history (syntax error, control label, lease-check stall).
- Regression, touched crates (`--no-fail-fast`): all pass except the four known
  Claude `claude_boundary` ProgramDrift failures and one new observation —
  `lifecycle::mutation_during_snapshot_collection_is_a_conflict` failed in the
  full run, then passed 3/3 isolated. That original failure stands as recorded.

## Decisions surfaced

- **Lease recheck window.** `EffectLease::check_current` cost ~2 s per call here,
  so per-chunk rechecks made streaming unusable. The probe rechecks at most every
  5 s during one admitted provider response (and immediately before the request).
  Revocation therefore stops delivery within a bounded window, not per chunk.
  The acceptance owner should confirm or set this window.
- Consumer release after `response.completed` is accepted only when the host then
  observes the framed upstream end with nothing unforwarded.

## Open, not established here

- Cancel responsiveness: the host notices a native `cancel` only between upstream
  reads; a stalled upstream bounds cancel latency by the upstream read timeout
  (10 s). Needs a concurrent upstream reader (one-chunk bounded channel).
- Docker-level native-initiated cancel, slow-native-reader, and malformed native
  frame controls (unit-level only so far).
- `record_frame` caps ordinals at 128; pull frames count (44 used in stream7).
  Longer sessions need a retention policy for stream control frames.
- Old live credential-mount Pi path not replaced (phase 4); duplicate-request
  control still buffered; route selection/failover (phase 3) not started.
- All 13 original classes, broad gate, and Astra/xhigh review remain open; this
  slice adds evidence toward `useful_native_sessions`, `whole_interval`,
  `runtime_profile` and `independent_effects` only.
