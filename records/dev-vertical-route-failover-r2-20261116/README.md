# Dev vertical: route/stream review fixes, 2026-11-16

Standing: bounded offline revision. Successor to
[route failover](../dev-vertical-route-failover-20261116/README.md) (`5e51f12b`),
which stays as written. Not acceptance, not the Astra/xhigh review, not live
qualification. Session spend at the start of this revision: $7.25 of $40.

## Review findings on `5e51f12b` and dispositions

| Finding | Disposition |
| --- | --- |
| P1: a cancel racing a terminal frame could wait forever for an acknowledgment | Fixed with a closing handshake. After `response-end`/`response-failure` the host waits for one closing frame at the next ordinal: `ack`, or a `cancel` the consumer sent before it saw the terminal frame. The host confirms that cancel with `response-cancelled`. The receiver settles synchronously on a terminal frame, writes `ack` before any later envelope, and sends no ack when its own cancel crossed the terminal. Relay evidence records `closing`. |
| P2: an exhausted per-route limit could move to the secondary without a refusal | Fixed. The active route is the first authorized, model-compatible route without a recorded definitive refusal. Leaving it requires that refusal. Revocation returns `route_revoked` and an exhausted limit returns `route_limit`; neither switches. Moving past them needs a new, authorized policy. |
| P2: unchecked allowance arithmetic | Fixed. Reservation uses `checked_add` and fails closed as `allowance_exhausted`; release uses `checked_sub` and errors on accounting inconsistency. |

## Evidence (one subject: [inputs.sha256](inputs.sha256); [evidence.json](evidence.json))

- Unit: `pi_route` 6/6, adding route-limit-no-switch, revoked-primary-no-switch
  and extreme-allowance tests. `pi_stream` 12/12, adding the cancel-crossing-end
  test and single-ack checks. Node receiver adapter 7/7, adding the crossing
  cancel without ack or hang and release-after-terminal tests.
- Docker on this subject: `stream11` full workflow passes. `failover3` passes
  (primary 1 refusal, secondary 6 receipts). `unavailable3` returns
  `route_unavailable_attempt_limit`. `midquota3` is uncertain with the secondary
  at 0 receipts. `fault-stall4` is cancelled and aborted. `fault-truncate6` is
  interrupted with `closing: ack`.
- Actual Pi releases after `response.completed` rather than pulling to the
  framed end, so in Docker the ack path is exercised via the failure terminal.
  The crossing-cancel race is covered at unit level only.
- Regression on the touched crates: only the four known `claude_boundary`
  ProgramDrift failures. Node: the two pre-existing `pi-memory-guard`-related
  failures.

## Open

Unchanged from the predecessor's open list: restart during a switch,
per-route credentials, provider-specific refusal codes, cache affinity,
head-phase cancel, frame/entry retention, raw screened SSE custody, phases 4–6
and Astra/xhigh review.
