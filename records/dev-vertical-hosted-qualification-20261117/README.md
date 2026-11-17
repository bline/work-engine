# Dev vertical: hosted-profile offline qualification (phase 4 piece (c)), 2026-11-17

Standing: offline qualification of the hosted Pi coding profile, piece (c)
of the [phase 4 design r2](../../planning/dev-vertical-phase4-hosted-transport-r2-20261116.md).
It is not acceptance, not independent review and not live qualification.
Every run uses synthetic loopback routes and a synthetic access token held by
the Host. No live credential, provider inference, push or deploy was involved.

Subject: `bd8478ed` (hosted code at `286474df`, plus the fixes listed below,
including the operator's K-a decision). Run set r4. An earlier run set (r3, on
`28b321a7`, before K-a) also passed its then-expected outcomes.
[matrix.json](matrix.json) binds the release `coding-qualification` binary
(its SHA-256 equals the runtime admission's `binary_sha256` in each run) and
the observer binary.

## Method

- [qualify.py](qualify.py) prepares a fresh hosted job and starts
  `examples/pi_hosted_coding_observer.rs` with one `HOSTED_OBSERVER_SCENARIO`.
  The primary route refuses its first request with 429, so request 1 always
  fails over first. Faults apply to the secondary route's first served
  response. After a fault the observer keeps accepting, so any later dispatch
  shows up as a receipt.
- For `crash` and the cancel variants, the harness acts once the observer
  reports it is holding a stream. `crash` SIGKILLs `coding-qualification`, then
  runs `pi-docker reconcile` and re-executes the same job. The cancel variants
  run `pi-docker cancel`.
- [check.py](check.py) encodes the expected outcome of each scenario and
  checks it against each run's summary and state directory.

Invariants checked in every scenario:
- No containers are left.
- Every retained hosted artifact verifies against its digest.
- Every attempt has `gate_entered`.
- Only refused attempts release their reservation.
- Every relayed-byte digest matches its retained file.
- The Host's access token appears in no run file, and `private-access` is
  empty.
- The inference container has `NetworkMode=none` and no `/auth` mount.
- The observer's dispatch count equals the ledger's attempts.

In every faulted scenario: the job fails closed, only request 1 exists, and
no tool effect follows an unsettled request. In every completing scenario the
attempt records whether the framed end was observed.

## K-a decision (operator, 2026-11-17)

A relayed attempt is Completed when its status is 200, it was not interrupted,
it carries no provider failure event, a screened `response.completed` was
relayed, and only SSE comments or blank lines follow it
(`pi_stream::terminal_is_last`). If the framed end is not observed within the
1 s release drain, the attempt still completes, with
`framed_end_observed: false` and the reason "completed on terminal event;
framed end not observed". A `data:` event after the terminal event, a partial
line, or anything ending before a terminal event stays Uncertain. Phase 6
measures the live backend's close timing and may tighten this.

## Results: all 14 scenarios pass (run set r4)

| Scenario | Outcome |
| --- | --- |
| `ok` | Exit 0. Request 1 refused, then completed. All four requests settle, one tool effect, 5 gate-admitted attempts. |
| `late-close-short` | Framed end 0.5 s after `response.completed`, inside the 1 s release drain. Completes like `ok`, with the framed end observed. |
| `late-close-long` | Framed end 3 s after `response.completed`. Completes on the terminal event (K-a), with `framed_end_observed: false`. |
| `trailing` | An SSE comment after `response.completed`. Completes on the terminal event (K-a). The comment is never relayed and is retained as `unforwarded.bin`. |
| `refuse-all` | 429, then 402: both attempts refused, nothing charged, attempt limit respected. |
| `primary-500` | Uncertain, no failover; the secondary is never contacted. Full reservation retained. |
| `truncate`, `stall-body` | Uncertain, relayed bytes retained. The stall ends at the Host's 30 s upstream idle bound, which closes the connection. |
| `stall-head` | Uncertain at the same 30 s bound; the upstream connection is closed. |
| `midstream-failed` | `response.failed` after output: Uncertain, and the reason names the failure. |
| `trailing-data` | A `data:` event after `response.completed`: Uncertain, reservation retained. The event is never relayed and is retained as `unforwarded.bin`. The container's completion is refused with a clear reason. |
| `crash` | The attempt stays durable with `gate_entered` and is still charged. `reconcile` removes both containers and marks the attempt Uncertain ("unsettled at restart"). Re-running the job is refused ("reconcile instead of launch replay"). |
| `cancel`, `cancel-head` | Uncertain, relayed bytes (possibly empty) retained, and the relay evidence reads `interrupted-uncertain`. The upstream connection is closed. |

Accounting across all scenarios: an attempt's charge is 0 if it was refused,
its settled cost if completed (266 µUSD each), and its full reservation
(164001 µUSD) otherwise. Nothing was dispatched after an uncertain or pending
attempt.

## Defects found and fixed

| Commit | Finding |
| --- | --- |
| `a702a2d7` | A relayed stream carrying `response.failed` was labelled Completed. It is now Uncertain. After a crash, a pending attempt stayed pending through `reconcile`; it is now Uncertain, still charged. |
| `bd8478ed` | K-a: settle on the terminal event when the framed end is not observed; record `framed_end_observed` on every relayed attempt; add `trailing-data`. |
| `28b321a7` | Q1: the container's completion of an Uncertain attempt failed with the unclear "attempt is not an unsettled completion"; it now names the attempt, disposition and reason. Q2: bytes read while draining after release were counted but discarded; they are now retained. Q3: relay evidence of a Host-closed stream said "completed"; it now says interrupted (`closed_by_host`). |

## Provenance note

Two copies of this Pi session (`01a1e2a4`) ran at the same time in this
checkout (board 5803–5804) until the operator stopped one. Both started piece
(c). The variants here merge both plans: the other copy contributed
`late-close-short`, `cancel-head`, the access-token and dispatch-count
invariants, and findings Q1–Q3. `a702a2d7` also contains that copy's
then-uncommitted parallel `--fault` observer code, which its message does not
mention; `28b321a7` removes it. The other copy's plan and harness files were
never committed and are superseded by this record.

## Tests

- manual-host lib: 76 pass.
- Integration tests: all pass, including the new `pi_stream` test for
  unforwarded bytes (20 pass).
- `tests/pi_stream.rs` adds `terminal_is_last` edge cases (CR/LF, comments,
  partial lines, `event:` fields, a data event after the terminal).
- `tests/lifecycle.rs`: the two known pre-existing failures.
  `actual_subprocess_timeout_collects_descendants` failed once while the Docker
  matrix was running and passed when re-run alone.

## Open

- Phase 6 measurement of the live backend's close timing and trailing bytes
  (K-a), and the response bound (D4).
- Independent review of piece (b) remediation and piece (c), under
  subscription quota.
- Phase 6: public HTTPS transport, the response bound (D4), credentials per
  payer and a header allowlist (K-b), then removal of the credential-mounted
  profiles.
