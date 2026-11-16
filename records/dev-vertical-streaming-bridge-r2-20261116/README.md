# Dev vertical: streamed bridge revision 2, 2026-11-16

Standing: bounded offline implementation result; successor to
[revision 1](../dev-vertical-streaming-bridge-20261116/README.md) (commit
`0fa4b2bc`), which stays as written. Not acceptance, not the required
Astra/xhigh review, not live qualification. Same session, authority and budget
limits as revision 1; operator said "proceed" to the proposed next steps and
supplied a code review of `0fa4b2bc`.

## Review of `0fa4b2bc` and dispositions

| Finding | Disposition |
| --- | --- |
| P1 CRLF SSE could hide `response.completed`, causing false uncertainty | Fixed: `pi_stream::sse_data` parses CRLF/LF/CR lines, blank-line dispatch, multi-line `data`, optional space; cancel drain keys on `completed`, not usage presence. Regression test: CRLF events + native cancel → `released-after-upstream-end`. |
| P2 mutable stream identity during release | Fixed: `createResponseStream` captures an immutable `{binding, sequence, stream}` per response. Node test releases an undrained stream 1 after stream 2 exists. |
| P2 128-frame cap | Reduced, not closed: content-free `pull` frames are no longer retained individually (stream controls summarize chunks); the full workflow now uses 19 frames (was 44). Tool/fetch/snapshot frames still accumulate, and snapshot entries are separately capped at 256. A bounded retention design and a test crossing those caps remain open. |
| P2 cancel coupled to blocking upstream reads | Fixed: upstream reads run on a dedicated thread with a one-chunk handoff; the host polls native readiness while a pull is outstanding. A native `cancel` may overtake the pull; the host confirms it, then `abort` closes the upstream socket. |
| Raw screened SSE not retained | Open: stream evidence keeps digest, per-chunk timing and normalized usage, not raw bytes. Protected custody for raw screened SSE, or an explicit reconciliation of that plan requirement, is still needed. |

## Other changes

- Receiver: response-stream logic is an exported factory; the entrypoint runs
  only as `/receiver.mjs`, so tests import it without side effects. A
  per-pull idle timer (`streamIdleMs` in `init`, probe env
  `PI_PROBE_STREAM_IDLE_MS`, default 30 s) calls the actual `session.abort()`.
- `Workload::readable` (nonblocking poll); observer `stall-first` fault reports
  when the host closes the connection.

## Evidence (same inputs: [stream9-inputs.sha256](stream9-inputs.sha256); [evidence.json](evidence.json))

- Rust `tests/pi_stream.rs` 10/10, including stalled-upstream cancel (relay
  returns and the endpoint sees close well under 3 s) and the CRLF regression.
  Node `tests/pi-native-stream.test.mjs` 5/5.
- Docker `stream9`: full workflow passes (tools, build + 3 tests, export, three
  snapshots, sealed readers; 6 receipts/5 effects); all streams consumed
  incrementally ([join](stream9-join.json)).
- Docker `fault-stall2`: endpoint stalls after two events. Pi's own abort fired
  after 2 s idle; the cancel overtook the pending pull (waited 2015 ms) and the endpoint saw the
  host close the connection 2017 ms into the stall. The Pi session recorded `stopReason: aborted`;
  `provider:1` stays `unknown`.
- Docker `fault-truncate4`: unchanged behavior (`interrupted-uncertain`,
  Pi error `relay_upstream_truncated`, effect `unknown`).
- Regression (`--no-fail-fast`, both touched crates): only the four known
  `claude_boundary` ProgramDrift failures. The lifecycle failure recorded in
  revision 1 recurred once more in a full run before this revision's final run,
  and passed in the final run and in two isolated whole-file runs; it remains an
  intermittent under-load failure, unattributed. Node suite: the two
  `pi-memory-guard`-related failures are pre-existing (same failures with this
  change stashed).

## Still open

The lease recheck window still needs a decision. Head-phase cancel is also open: Pi's abort before
`response-head` waits for the upstream head or socket timeout. Frame/entry
retention, raw screened evidence, Docker slow-reader and malformed-native-frame
controls, phases 3–6, the 13-class matrix, the broad gate and Astra/xhigh
review are also still open.
