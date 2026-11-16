# Dev vertical: piece-(a) repairs after Fable review, 2026-11-16

Standing: bounded offline repairs to the event-driven stream session. Successor
to [stream session](../dev-vertical-stream-session-20261116/README.md). The
operator authorized the design revision and these bounded repairs. The design
revision is in
[phase 4 design r2](../../planning/dev-vertical-phase4-hosted-transport-r2-20261116.md).

## Repairs (review ids from [Fable review](../../reviews/dev-vertical-phase4-design-fable-20261116/REVIEW.md))

| Id | Change |
| --- | --- |
| R4 | `pi_stream::Unavailable` answers a stream that has no permitted attempt. The next native frame is offered to `on_next`: it confirms a crossing `cancel(1)` and hands every other session envelope back. Only stream-protocol frame types are judged, because session envelopes carry the native's stream counter. |
| R5 | `pi_stream::pre_dispatch`: after admission and before any upstream byte, a crossed `cancel(1)` is confirmed and the attempt settles `NotDispatched`. The router releases its reservation and ends the request with no switch. The lease is observed as `not-dispatched:<attempt>`. |
| K2 | Release after a terminal provider event no longer blocks `on_native`. A polled `Draining` state observes the framed end for up to 1 s. `accepts_native()` is false once the consumer has finished, so later native frames go to the session loop. |
| K4 | Receiver: an unexpected body frame now errors **and** releases, through the cancel handshake. |

## Evidence (one subject: [inputs.sha256](inputs.sha256); [evidence.json](evidence.json))

- Unit tests: `pi_stream` 19/19, adding non-blocking drain, unavailable
  ownership and pre-dispatch tests. `pi_route` 8/8, adding NotDispatched
  release without a switch. Node `openResponse` 9/9, adding the
  unexpected-frame handshake.
- Docker with actual Pi:

  | Run | Result |
  | --- | --- |
  | `stream16` | full workflow passes |
  | `failover8` | primary 1 receipt, secondary 6 |
  | `unavailable7` | `route_unavailable_attempt_limit` |
  | `predispatch2` | **new**: Pi head timeout 1 s fired during host admission; host confirmed the cancel before dispatch; endpoint received **0** requests; attempt `not-dispatched`, reservation 0 |
  | `fault-headstall5` | head-phase cancel → uncertain |
  | `fault-stall8` | body-phase abort → uncertain |
  | `midquota7` | uncertain, secondary 0 receipts |
  | `fault-truncate9` | interrupted → uncertain |

- An intermediate subject failed in `unavailable6`. The first `on_next` treated
  the snapshot envelope as an interleaved stream frame. It was fixed and
  re-run as `unavailable7`, and is kept as history.
- Regression: only the four known `claude_boundary` ProgramDrift failures and
  the two pre-existing Node `pi-memory-guard` failures.

## Not changed here

The financial ledger attempt records (R1), effect key (R2), raw-SSE retention
(R3/decision 4), bounds policy (K1) and host `rawTerminal` (K3) are piece (b),
as specified in the design revision.
