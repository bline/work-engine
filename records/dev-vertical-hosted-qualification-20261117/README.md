# Dev vertical: hosted-profile offline qualification (phase 4 piece (c)), 2026-11-17

Standing: offline qualification of the hosted Pi coding profile, piece (c)
of the [phase 4 design r2](../../planning/dev-vertical-phase4-hosted-transport-r2-20261116.md),
updated for the required changes of the
[second independent review](../../reviews/dev-vertical-phase4-piece-c-fable-20261117/REVIEW.md).
It is not acceptance, not independent review and not live qualification.
Every run uses synthetic loopback routes and a synthetic access token held by
the Host. No live credential, provider inference, push or deploy was involved.
The remediation below has not been independently reviewed.

Subject: `96fe54de`. Run set r6 (`~/.local/state/work-engine/dev-vertical-hosted-qual-20261117-r6`,
driven by its `matrix.sh`). Earlier run sets: r5 on `30d0fc1c` (the same
scenarios, before the label fixes in `96fe54de`) passed; r4 on `bd8478ed`
(14 scenarios) was the subject of the second review.

[matrix.json](matrix.json) is the output of `check.py <r6> 96fe54de`; rerunning
that command over the run set reproduces it. It records the SHA-256 of the
release `coding-qualification` binary and the observer binary, and checks
that every run used them (the runtime admission's `binary_sha256` and each
run's summary). Nothing binds those binaries to the source commit beyond
having been built from this checkout at `96fe54de`.

## Method

- [qualify.py](qualify.py) prepares a fresh hosted job and starts
  `examples/pi_hosted_coding_observer.rs` with one `HOSTED_OBSERVER_SCENARIO`.
  The primary route refuses request 1 with 429, so request 1 fails over
  first. Secondary faults apply to its first served response unless the
  scenario names another (`truncate@3`: the third, the first turn after the
  reset). After a fault the observer keeps accepting, so any later dispatch
  shows up as a receipt.
- For `crash` and the cancel variants, the harness acts once the observer
  reports it is holding a stream. `crash` SIGKILLs `coding-qualification`,
  runs `pi-docker reconcile`, and re-executes the same job, with the observer
  still running; the ledger is read again after the replay. The cancel
  variants run `pi-docker cancel`.
- [check.py](check.py) encodes the expected outcome of each scenario and
  checks it against each run's summary and state directory.
- Money: the Host does not export its charge. Checks compare the dispositions
  and settlements the Host recorded. Checks labelled "(derived)" apply this
  harness's own copy of the ledger rule to them, so they restate the
  disposition checks rather than test the Host's arithmetic (unit tests cover
  `charged_microusd`).

Invariants checked in every scenario:
- The run used the checked binaries.
- No containers are left; every retained hosted artifact verifies.
- Every attempt has `gate_entered`.
- Released (refused or not-dispatched) attempts relayed nothing; every other
  attempt is completed or uncertain.
- Every attempt's relayed-byte digest matches its retained, registered
  `stream.sse`.
- Except after the SIGKILL, the gate's own log matches the attempt records
  per attempt: request, attempt, route, payer and deadline.
- The Host's access token appears in no run file; `private-access` is empty.
- The inference container has `NetworkMode=none` and no `/auth` mount.
- The observer's dispatch count equals the ledger's attempts.

## Settlement rule (operator decisions K-a and Q-a, 2026-11-17)

A relayed attempt is Completed when its status is 200, it was not
interrupted, it carries no provider failure event, and its terminal event is
clean: the first screened `response.completed` was relayed, only SSE comments
or blank lines follow it (`pi_stream::terminal_is_last`), and every provider
byte after it was read and kept (the framed end was observed or the release
drain ran, and no held-back byte was dropped). The tail rule applies whether
or not the framed end was observed (Q-a). If the framed end is not observed
within the 1 s release drain, the attempt still completes, with
`framed_end_observed: false` and the reason "completed on terminal event;
framed end not observed" (K-a). Everything else is Uncertain. A backend that
sends a `data: [DONE]` sentinel would make every turn Uncertain, so phase 6
must check the live backend for one before live use.

## Results: all 19 scenarios pass (run set r6)

Every attempt's reason below is as the Host recorded it.

| Scenario | Outcome |
| --- | --- |
| `ok` | Exit 0. Request 1 refused, then completed. All four requests settle, one tool effect, five gate-admitted attempts. |
| `late-close-short` | Framed end 0.5 s after `response.completed`: the drain sees it. Completes like `ok`, framed end observed. |
| `late-close-long` | Framed end 3 s after `response.completed`, after the 1 s drain. Completes on the terminal event (K-a), `framed_end_observed: false`. |
| `trailing` | An SSE comment after `response.completed`, then the framed end, both inside the drain. Completes with the framed end observed (Q-a: the tail is clean). The comment is never relayed; it is screened and retained as `unforwarded.bin`. |
| `refuse-all` | 429, then 402: both attempts refused and released; attempt limit respected. The container's completion is refused with "that attempt's reservation was released". |
| `primary-500` | Uncertain, no failover; the secondary is never contacted. |
| `primary-429-echo` | A 429 whose body carries the access token: the screen refuses the body, the attempt is Uncertain ("closed uncertain: hosted_access_exclusion"), no failover, and the body is not retained. |
| `primary-429-truncated` | A 429 whose body ends before its length: not a definitive refusal. Uncertain (`upstream_truncated`), no failover. |
| `truncate` | Uncertain (`upstream_truncated`), relayed bytes retained. |
| `midstream-failed` | `response.failed` after output: Uncertain, and the reason names the failure. |
| `trailing-data` | A `data:` event after `response.completed`, then the framed end: Uncertain under the tail rule (Q-a), with the framed end observed. The event is never relayed; it is retained as `unforwarded.bin`. |
| `echo-body` | A text delta carrying the access token before the terminal event: the screen interrupts the relay (`hosted_access_exclusion`); the bytes relayed before it are retained. Uncertain. |
| `echo-trailing` | An SSE comment carrying the access token after the terminal event: the drain screens it, drops it (`heldDropped`), and the relay is interrupted. Uncertain; no `unforwarded.bin`. |
| `stall-head` | No head: Uncertain at the Host's 30 s upstream bound (`upstream_head_read`), no status recorded, zero bytes; the upstream connection is closed. |
| `stall-body` | Two events, then silence: Uncertain at the 30 s bound (`upstream_interrupted`), relayed bytes retained; the connection is closed. |
| `truncate-3` | Truncation on request 3, the first turn after the reset. Requests 1 and 2 settle; request 3 is Uncertain with its bytes retained; no request 4; one tool effect. |
| `crash` | SIGKILL mid-body. The attempt was durable with `gate_entered` and status 200, and its 272 relayed bytes were in `stream.sse`. `reconcile` removes both containers and marks it Uncertain ("unsettled at restart; producing process gone; 272 relayed bytes retained up to the end"), registering and digesting the file. Re-running the job is refused ("reconcile instead of launch replay"). The observer saw no dispatch after the kill, and the ledger after the replay equals the ledger after reconcile. |
| `cancel`, `cancel-head` | Uncertain, relayed bytes (empty for `cancel-head`, which has no head and no status) retained; the relay evidence reads `interrupted-uncertain`; the upstream connection is closed. |

Money, from the recorded dispositions: a refused attempt is released, a
completed attempt is charged its settled cost (266 µUSD each), and every other
attempt keeps its full reservation (164001 µUSD). Nothing was dispatched after
an uncertain or pending attempt.

## Review 2 required changes

| Item | Change (commit) | Evidence |
| --- | --- | --- |
| N1 | The release drain screens what it reads. Access material drops the bytes and interrupts the relay (`81c68b3b`). | `echo-trailing`; `pi_stream` test `release_drain_screens_what_it_reads`. |
| N2 | Bytes the screen held back at release seed the drain and are screened with what follows; a held-back suffix left at the end is dropped and counted (`heldDropped`). A completion needs the framed end or a finished drain, and no dropped byte (`81c68b3b`). | `pi_stream` test `bytes_held_back_at_release_…`; provider test `relay_evidence_is_clean_only_when_…`. The read boundary that triggers it is not reproduced in the Docker flow. |
| N3 | Each relayed chunk is appended to the attempt's create-only `stream.sse` before it is relayed, and the head status is persisted when the head is relayed. Settlement checks the file equals the relayed bytes and registers it; reconcile registers what is there and says how many bytes were retained (`81c68b3b`, `30d0fc1c`). | `crash`: 272 relayed bytes retained, registered and digested, status 200; hosted unit test `reconcile_marks_a_pending_attempt_…`. |
| N4, Q-b | The drain accumulates until the framed end, 1 s or 64 KiB, with a last non-blocking read at the time bound (`81c68b3b`). | `trailing`, `late-close-short` now see the framed end inside the drain; `pi_stream` test `release_drain_reads_until_its_bound_…`. |
| N5 | The first `response.completed` is the terminal event for usage, the raw terminal and the tail check (`81c68b3b`). | `pi_stream` test `the_first_response_completed_is_the_terminal_event`. |
| N6 | Drain errors are recorded (`drainError`) and named in a K-a reason; `closed_by_host` keeps an existing interruption code; the completion refusal for a released attempt says it was released (`81c68b3b`). The Uncertain reason carries the interruption code, no head records no status, and the refusal names the request (`96fe54de`). | `refuse-all` failure text; `echo-body` reason. |
| Q-a | Operator decision: tail rule in both branches (`81c68b3b`). | `trailing-data` (data event, then the framed end): Uncertain. |
| Q-c | Reconcile at the end of a run says "unsettled when the run ended", not "producing process gone" (`81c68b3b`). | Unit-level only. |

Harness weaknesses from the review: the crash replay is now witnessed (the
observer stays up; the ledger is re-read after the replay and equals the
post-reconcile ledger); charge checks are labelled derived; the release check
states what it checks; the gate log is compared per attempt in every non-crash
scenario; screening is challenged in a body, a refusal body and trailing
bytes; faults hit a later request (`truncate-3`); a non-definitive refusal is
exercised (`primary-429-truncated`); `matrix.json` is reproducible from
`check.py`.

## Defects found and fixed before review 2

| Commit | Finding |
| --- | --- |
| `a702a2d7` | A relayed stream carrying `response.failed` was labelled Completed. It is now Uncertain. After a crash, a pending attempt stayed pending through `reconcile`; it is now Uncertain, still charged. |
| `28b321a7` | Q1: the container's completion of an Uncertain attempt failed with an unclear message; it now names the attempt, disposition and reason. Q2: bytes read while draining were counted but discarded; they are retained. Q3: relay evidence of a Host-closed stream said "completed"; it says interrupted. |
| `bd8478ed` | K-a: settle on the terminal event when the framed end is not observed; record `framed_end_observed`; add `trailing-data`. |

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

On `96fe54de`, after the matrix:
- manual-host lib: 77 pass (6 ignored).
- `tests/pi_stream.rs`: 25 pass. New: the drain reads to its bound, screens
  what it reads, and screens or drops held-back bytes; the first terminal
  event wins.
- `tests/lifecycle.rs`: 50 pass. `mutation_during_snapshot_collection_is_a_conflict`
  failed once under load before the matrix (a 10 s timing bound) and passed
  alone and in the full run.
- All other manual-host integration tests pass; `pi-pressure-context-host`
  tests pass.

The observer binary in r6 (`f677f09f…`) rebuilds byte-identically from
`96fe54de`'s source (a later rustfmt-only change alters its digest). The release `coding-qualification` and observer binaries used
by r6 are retained in the run set's `bin/`, which `check.py` reads.
[matrix-r6.sh](matrix-r6.sh) is the run set's driver.

## Not exercised

- Pre-dispatch cancel ending `NotDispatched` (unit-tested only).
- The 1 MiB response bound (D4), 401/403 refusals, an expired or revoked
  grant between attempts in the Docker flow, job-deadline expiry mid-stream,
  and framings other than chunked.
- Crashes other than mid-body: between reservation and gate entry, during the
  drain, between relay settlement and `provider_complete`.
- The lifecycle store's turn and grant state after a crash.
- Q-d is unchanged: the gate log is still an embedder convenience file,
  written after the receipt and absent after a crash. The durable evidence is
  `gate_entered`, `deadline_ms` and `routes_sha256` on the attempt record.

## Open

- Independent review of this remediation (only if the operator asks; API cost).
- Phase 6: live backend close timing, trailing bytes and any `[DONE]`
  sentinel (K-a, Q-a); public HTTPS transport; the response bound (D4);
  credentials per payer and a header allowlist (K-b); then removal of the
  credential-mounted profiles. K-c to K-g remain as the review lists them.
