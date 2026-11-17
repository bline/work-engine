# Dev vertical: hosted-profile qualification, live routes and cutover, 2026-11-17

Standing: qualification record of the hosted Pi coding profile (phase 4
piece (c) of the [phase 4 design r2](../../planning/dev-vertical-phase4-hosted-transport-r2-20261116.md),
the required changes of the
[second independent review](../../reviews/dev-vertical-phase4-piece-c-fable-20261117/REVIEW.md),
and phase 6 of the [completion plan](../../planning/dev-vertical-completion-20261115.md)).
It is not owner acceptance. Live provider use was authorized by the operator
on 2026-11-17 ("I authorize anything we need to finish, including live tests
and budget"). Nothing was pushed or deployed by this work. The changes after
review 2 were reviewed by Fable and by Opus xhigh (section "Review 3"); the
remediation of that review (`7c103ad2`) has not been re-reviewed.

Subjects:
- Offline matrix: run set r8 on `20bb335c` (code `7c103ad2`)
  (`~/.local/state/work-engine/dev-vertical-hosted-qual-20261117-r8`, driven by
  [matrix-r8.sh](matrix-r8.sh)); 19 of 19 scenarios pass. r7 on `8fa71188`
  passed the same scenarios before the review 3 remediation. Earlier sets: r6 on
  `96fe54de` and r5 on `30d0fc1c` passed the same scenarios; r4 on `bd8478ed`
  (14 scenarios) was the subject of review 2.
- Live runs (section "Live qualification"): `38803cff` and `8fa71188`.
- Review 3 remediation `7c103ad2`: offline matrix r8 and live cutover `c3`
  (section "Review 3").

[matrix.json](matrix.json) is the output of `check.py <r8> 20bb335c`; rerunning
that command over the run set reproduces it. It records the SHA-256 of the
release `coding-qualification` binary and the observer binary (copies kept in
the run set's `bin/`), and checks that every run used them. Nothing binds those
binaries to the source commit beyond having been built from this checkout.

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

## Offline results: all 19 scenarios pass (run set r8)

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

## Live qualification (phase 6)

Real routes, Host-held access, `gpt-6-luna`, the four-request coding
continuity task (tool call, answer, reset with rehydration, continued turn),
the real tool and validator containers. [live_check.py](live_check.py)
searched every file of every live run directory for the subscription access
token, its refresh token and the API key, and retained its result as
`live-check.json` in each run directory: none found. The inference container had `NetworkMode=none` and no `/auth` mount in
every run.

**Backend stream end** ([probe](../../metrics/probes/codex_stream_end_probe.py),
raw bytes in `~/.local/state/work-engine/dev-vertical-live-probe-20261117`):
three runs against the ChatGPT Codex backend and one against the public
Responses API. In every run the chunked framed end arrived 20–120 ms after
`response.completed`, with no bytes between them and no `data: [DONE]`
sentinel. So K-a's "framed end not observed" branch and Q-a's tail rule are not
triggered by these backends at this size. The public Responses API accepted
the Codex-shaped request body unchanged and returned the same event shapes; the
same model is listed for the API key.

**Routes:** primary `https:chatgpt-codex` (ChatGPT subscription, access from
Pi's auth store), secondary `https:openai-responses` (API key). Each payer's
credential kind must match its route; every payer's secret is screened.

| Run | Path | Outcome |
| --- | --- | --- |
| `live-1` (`38803cff`) | qualify.py | 4 requests completed on the subscription route; one tool effect; validator passed; settled 268/267/203/200 µUSD (catalog rates; subscription-funded). |
| `live-transition-1` (`38803cff`) | qualify.py | Primary payer `chatgpt-revoked` (access the backend must refuse): real 401, definitive refusal, released; failover to the API route, which completed requests 1–4. |
| `c1` (`8fa71188`) | **operational entrypoint**: `coding-job-prepare` → `work-engine-server` (local-live class) → `work-engine-command` → producing Host | Operation `collected`, Host `component_passed`, acceptance `owner_pending`; 4 requests on the subscription route; the model's edit passed the validator. |
| `c2-transition` (`8fa71188`) | operational entrypoint | Real 401 on the primary, failover to the API route; collected, `component_passed`. |
| `c3` (`7c103ad2`) | operational entrypoint | After review 3: the real Pi request passes the Host body policy; 4 requests on the subscription route; the receipt names route, payer and endpoint per attempt; the journal records the credentials-file digest and each payer's source kind. Collected, `component_passed`. |

Runs: `~/.local/state/work-engine/dev-vertical-hosted-live-20261117` and
`~/.local/state/work-engine/dev-vertical-cutover-20261117`.
[live_check.py](live_check.py) checks secrets and results; [cutover.py](cutover.py)
drives the entrypoint runs; it persists the operator's API key once to an
owner-only file (`~/.local/state/work-engine/credentials/openai-api-key`) so
the server-launched Host, whose environment is cleared, can load it. API
spend: about 8 Luna requests of about 1 000 tokens each, well under one cent.
The transition exercised a **synthetic invalid** access token on the real
backend (401 "Could not parse your authentication token"); no genuinely
revoked or expired credential, and no quota refusal (429/402), was produced.
The largest relayed live response was about 31 KB (`live-1` request 1).

## Cutover and retirement

- `38803cff`: HTTPS transport (`pi_coding_https.rs`), per-payer access
  (K-b), a per-endpoint header allowlist (K-b), connection and TLS before
  gate admission and outside the store transaction (K-e), hosted
  `max_context_bytes` ≤ 32768 (K-d), container usage labelled (K-g).
- `8fa71188`: the credential-mounted live profiles are retired
  (`pi-coding-sdk.v1`, `lifecycle-live.v1`, `lifecycle-sol-high.v1`,
  `signed-subscription.v1`; admission refuses them by name), with the native
  access projection, the access adapter, `/auth` mounts and the Sol-high
  contract (K-f: the hosted admission no longer carries native-access fields).
  The operational server's live class and `coding-job-prepare` now prepare and
  verify hosted jobs; the job profile passes only the credentials-file path.
  Net −1 538 lines.
- `cab80a3a`: crash cuts before gate entry (charged, nothing relayed, never
  dispatched) and between relay settlement and the container's completion (the
  full hold stays; nothing is released or relabelled).

## Review 3

Subject `b50017ab`. Reviewers ran as pi subagents; both were launched on the
API-billed `anthropic/` model namespace by mistake ($7.36 and $7.92); the
operator's Claude quota is the `claude-sdk/` namespace.

- [Fable](../../reviews/dev-vertical-phase6-transport-fable-20261117/REVIEW.md)
  (complex review: transport, ledger, evidence): accept with required changes,
  no P1. N1–N6 and Q-a confirmed fixed; K-b, K-d, K-e, K-f, K-g closed.
- [Opus xhigh](../../reviews/dev-vertical-phase6-retirement-opus-20261117/REVIEW.md)
  (normal code review: retirement and cutover): OK with notes, no P1.

Remediation `7c103ad2`:

| Item | Change |
| --- | --- |
| M1 | Receipt: `provider: hosted-routes` with per-attempt route, payer, endpoint and disposition; the cost label says catalog proxy on every route; the transport identity is included. |
| M2 | Connect-failure non-dispatch unit test (release, no gate intent, no write, request ended); `check.py` exempts `not_dispatched` from the gate-admitted invariant. |
| M3 | An unfinished drain's unscreened bytes and held-back bytes are dropped and counted (`heldDropped`), never digested. |
| M4 | A drain that ends on a read or framing error is never a clean terminal (Uncertain). |
| M5, F6 | The live class admits only the fixed HTTPS endpoints, requires `WORK_ENGINE_HOSTED_CREDENTIALS` in the live job profile, and requires exactly the eleven admission fields (server and preparation). |
| M6 | This record corrected (transition, response size, secret scan retained, API key file). |
| R-a | Host body policy on real routes: `store:false`, no `background`, default `service_tier`, function tools only. |
| R-b | The journal and receipt record the credentials file's digest and each payer's access-source kind. |
| R-d | Response bound raised from 1 MiB to 32 MiB (the catalog's 128k output tokens at about 250 bytes per delta). |
| R-f | Credentials file and Pi auth store must be owner-only regular files; account id charset checked. |
| F1, F8 | Host, server and lifecycle docs describe the hosted profile; retired lanes marked. |
| F3 | Provider-disabled receiver VM test (pressure observation and checkpoint insertion) ported. |
| F4 | The lifecycle gate refuses live mode without a hosted policy. |
| F5 | Coding containers are always network-none (no bridge branch); a retired subscription submit is refused before the owner opens; stale Sol fields dropped. |
| F7 | Every retired schema refused by name (with and without a provider); the Sol-high gate test checks construction. |
| F2 | Coding-switch test restored; the switch itself is left (digest-pinned owner). |

Not changed: R-c (price parity), R-e (DNS deadline, head bound, CDN refusals;
none observed), R-g (the job deadline bounds the socket; grants expire with
the job).

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

At `8fa71188`/`cab80a3a`: manual-host lib (all pass, 6 ignored), all
manual-host integration tests including `tests/lifecycle.rs` (50) and
`tests/pi_stream.rs` (25), the hosted receiver test under
`--experimental-vm-modules`, `pi-pressure-context-host` tests, and
`work-engine-server` `preparation`, `coding_job` and `job::` tests. The broad
workspace regression is recorded separately.

## Not exercised

- Pre-dispatch cancel ending `NotDispatched` (unit-tested only).
- The 1 MiB response bound (D4), 403 refusals, an expired or revoked grant
  between attempts in the Docker flow, job-deadline expiry mid-stream, and
  framings other than chunked.
- A crash during the release drain (covered in kind by the mid-body crash:
  the attempt is pending and its relayed bytes are retained), and live crash
  cuts against real routes.
- A real 429 quota refusal; subscription access refresh (the Host refuses
  access expiring within 10 minutes rather than refreshing).
- Q-d is unchanged: the gate log is an embedder convenience file; the durable
  evidence is `gate_entered`, `deadline_ms` and `routes_sha256` on each attempt.

## Open

- Re-review of the review 3 remediation `7c103ad2` (optional).
- K-c: grants are self-issued by the producing binary (`coding-qualification`),
  now for real funding; an external grant issuer is not part of this vertical.
- Price parity of the API route with the pinned catalog (R-c): settlement and
  the budget use the catalog's rates on both routes, as a proxy.
- A real quota refusal and a genuinely revoked or expired credential; one live
  fault (cancel or crash) against a real route; pre-dispatch cancel and a gate
  refusal between attempts in the Docker flow.
- Registration and permission checks for the subscription route (completion
  plan, phase 6): the Host reuses Pi's Codex OAuth access; no separate
  registration or permission qualification was performed.
- Lifecycle-store Sol-high grammar removal (F4 follow-up); session-events
  subscription switch removal (F2: `session_events.py` is digest-pinned by
  `session-query`, so it was left unchanged).
- Owner acceptance of the exported results (`owner_pending`).
