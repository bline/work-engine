# Independent review: hosted transport after review 2, live routes and cutover evidence

Subject: `acb80187`..`b50017ab`, restricted to `subject-manifest.txt`.

**Subject identity is not verified.** This session had file read and search tools only, with no `git show/diff/log`. The diff tool reported "Reviewer-launch HEAD changed or is unavailable". I reviewed the working tree as it stood when read, so line numbers are working-tree lines. I could not confirm the tree equals `b50017ab`, nor see what `cab80a3a`..`b50017ab` contains.

Nothing was built, run or modified, and no credential file was read. I read the retained run directories for r7, the live probe, the two `qualify.py` live runs and the two cutover runs.

Path shorthand (under `rust/crates/work-engine-manual-host/src/` unless noted):

| Short | Path |
| --- | --- |
| `hosted.rs` | `pi_coding_hosted.rs` |
| `provider.rs` | `pi_coding_provider.rs` |
| `https.rs` | `pi_coding_https.rs` |
| `pi_stream.rs`, `pi_docker.rs`, `pi_route.rs` | same names |
| `qual.rs` | `rust/crates/pi-pressure-context-host/src/bin/coding-qualification.rs` |
| `job.rs`, `prepare.rs` | `rust/crates/work-engine-server/src/job.rs`, `…/src/bin/coding-job-prepare.rs` |
| `README`, `check.py`, `qualify.py`, `cutover.py`, `live_check.py` | `records/dev-vertical-hosted-qualification-20261117/` |
| live runs | `~/.local/state/work-engine/dev-vertical-hosted-live-20261117`, `…/dev-vertical-cutover-20261117` |

## 1. Disposition

**Accept with required changes** as the hosted transport and its qualification record. It is **not yet sufficient to call the vertical complete.**

I found no P1. In the code as read:
- No path releases money without a definitive refusal or a confirmed non-dispatch.
- No unsettled attempt can authorize another.
- No dispatch happens outside the gate.
- I found no path that puts access material into the container, the journal or a retained file.
- N1–N6 and Q-a are implemented as decided.
- The live evidence I could inspect matches the record's table. All 16 completed live attempts have `drained: true`, `heldDropped: 0`, `released-after-upstream-end` and `unforwardedAfterRelease: 0`.

Required before the vertical is called complete:
- **M1** (receipt provider and cost labels) and **M2** (the connect-failure release path has no test).
- Operator decisions on **R-a** (request-body fields on real routes) and **R-b** (credential-source provenance).
- The record corrections in **M6**.
- The gates in section 4 that this review cannot close.

M3–M5 are small and belong in the same pass.

## 2. Findings

### Confirmed defects

**M1 (P2). The receipt's provider and cost labels are static and wrong for attempts served by the API route.**
- Evidence:
  - `pi_docker.rs:1325-1330` sets `provider: "openai-codex"` and `cost_evidence: "pinned_catalog_proxy_only_not_subscription_billing"` for every job with a provider plan.
  - In `c2-transition`, all four completed requests were served by `secondary` (`https:openai-responses`, payer `openai-api`). Its `coding-host-result.json` still carries those two labels (lines 468 and 1167).
- Why it matters: provider identity and fallback provenance are core invariants. The attempt records and the job's route table are truthful, so the detail is there. The summary a consumer reads first names a provider that served nothing and describes API-key spend only as "not subscription billing".
- Minimal change: derive both labels from the attempts. For example, emit the distinct `(route, endpoint, payer)` of completed attempts and a cost label per payer kind. Alternatively rename the fields to say they describe the container-facing catalog identity.

**M2 (P2). The connect-failure release path has no test at any level.**
- Evidence:
  - `hosted.rs:763-799` settles the attempt `NotDispatched` (reservation released) when `transport.connect` fails, sets a reason, sets `provider_failed` and answers `route_unreachable`.
  - `route_unreachable` and "route connection failed" appear nowhere else under `rust/crates`. The unit-test transport's `connect` always succeeds (`hosted.rs:1431`). The observer always accepts.
- Why it matters: this is the third way money is released, added in `38803cff`. By reading, it is correct: both transports write no request byte in `connect`, the gate is not consulted, and no later attempt follows. It rests on reading alone.
- Side effect: `check.py`'s invariant "every attempt gate-admitted" would fail on such an attempt, because `gate_entered` stays false.
- Minimal change: one hosted unit test with a failing `connect`. Assert `NotDispatched`, the reason, zero charge, no gate intent, `route_unreachable`, and that request 2 is refused. Relax the `check.py` invariant to exclude `not_dispatched`.

**M3 (P3). Bytes read by an unfinished drain are lost uncounted when the Host closes the attempt, and `relay.json` carries their digest.**
- Evidence:
  - The drain accumulates raw bytes in `Relay.unforwarded` and screens them only in `drain_done` (`pi_stream.rs:811-836`, `:627-643`).
  - `hosted_close_uncertain` (`hosted.rs:1302-1327`) takes the relay of an unfinished session and writes `relay.evidence()`. It writes no `unforwarded.bin`.
  - `evidence()` emits `unforwardedSha256` whenever `unforwarded` is non-empty (`pi_stream.rs:275`), with `heldDropped: 0`.
  - Reachable in the drain window (up to 1 s) by run-loop exit (`coding_hosted_abort`), by a stray stream frame while draining (`hosted.rs` `hosted_native`, the `_ =>` arm), or by the 2 s bound in `coding_hosted_before_complete`.
- Why it matters: the outcome is Uncertain and no unscreened byte reaches disk, so money and the screen are fine. The evidence names a digest of bytes that were not kept, and of bytes that were never screened. The same close also drops screen-held bytes (`held`) without counting them.
- Minimal change: in `hosted_close_uncertain`, when the session was unfinished, add `unforwarded.len()` (and any held bytes) to `held_dropped` and clear `unforwarded` before computing the evidence.

**M4 (P3). A chunk whose terminator fails is discarded, and in the drain the attempt can still complete.**
- Evidence:
  - `Upstream::next` reads the chunk data, then returns an error if the trailing CRLF is missing or wrong (`pi_stream.rs:171-178`). The data bytes are dropped with the error.
  - In the drain the error becomes `drain_error` and the drain ends (`pi_stream.rs:829-832`).
  - With an empty `unforwarded`, `terminal_last` is true and the attempt completes with "(drain ended: …)" (`provider.rs:253-266`).
- Why it matters: `[completed]`, then a chunk holding a `data:` event, then a cut before that chunk's CRLF, settles as Completed. The settlement rule says every provider byte after the terminal was read and kept, and that a data event after it is Uncertain. It needs a cut exactly between chunk data and its terminator, so it is unlikely.
- No test covers a drain error at all (`drain_error` appears in tests only as `None`).
- Minimal change: return the chunk's bytes and check its terminator at the start of the next `next()` call. Add a drain-error test.

**M5 (P3). The live class admits loopback routes, and the docs say it does not.**
- Evidence:
  - `verify_live_job` (`job.rs:218`) and `coding-job-prepare` (`prepare.rs:176`) call `HostedRoutes::validate`, which accepts `loopback:<port>` (`hosted.rs:126-135`).
  - The live `JobProfile` is not required to carry `WORK_ENGINE_HOSTED_CREDENTIALS` (`job.rs:435-459`).
  - Without that variable the producing Host binds the synthetic loopback transport (`qual.rs:237-244`).
  - `docs/coding-job-operating-guide.md:54-55` and `work-engine-server/docs/coding-preparation.md:77-78` say live routes are the two HTTPS endpoints.
- Why it matters:
  - A hand-written `coding-job-local-live-v1` profile can run a synthetic route and return `component_passed`, "bounded_live_continuity_completed" and `provider: openai-codex`.
  - Through `coding-job-prepare` (which always sets the variable), a loopback table is admitted, the one-shot authorization is consumed, and the job then fails at transport binding.
- Minimal change: in `verify_live_job` and `prepare.rs`, require every endpoint to be a non-loopback `Endpoint`. Require the live profile's `env` to name the credentials file.

**M6 (P3). Record inaccuracies.**
- "largest live response here about 8 KiB" (README, Open). The largest relayed live response is 30,936 bytes (`live-1` request 1) and 26,295 bytes (`live-transition-1` 1-2). 8,147 bytes is the probe's payload.
- "The transition exercised is a revoked credential (401)". The primary's access was a synthetic random string (`https.rs:131-143`). The retained refusal body reads "Could not parse your authentication token". No revoked or expired real credential was exercised.
- "Secrets were searched for in every file of every run directory: none found". `live_check.py` prints its result and nothing retained records it. I could not verify the claim (section 5).
- `c1` has no retained harness summary; only `c2-transition.json` exists. The `c1` row is supported by `c1/result.json` and `c1/state/pi-state.json`, which I checked.
- `cutover.py:59-61` writes the API key from the environment to `~/.local/state/work-engine/credentials/openai-api-key` if absent. That is a persistent secret created by a qualification script, and the record does not say so.
- Stale text: `qualify.py`'s docstring says synthetic routes only; `qual.rs:90-91` still lists "auth-source path" in the admission.

### Risks and open questions

**R-a (P2, operator). The container's request body is forwarded to real routes with only model, `stream` and size checked.**
- `hosted.rs:553-561` checks `model`, `stream == true` and the 45,000-byte bound. Headers are allowlisted and the URL is fixed, but every other body field is the container's choice.
- The retained live streams show the backend echoing `service_tier: "auto"`, `store: false` and a `tool_usage.web_search` counter. Those fields are live on this API.
- If the backend honours them, three consequences follow:
  1. A provider-hosted tool such as `web_search` gives a contractually networkless container provider-mediated network content.
  2. Per-call tool charges and a non-default `service_tier` fall outside the token-based settlement and budget.
  3. `store: false`, which the phase 6 plan names for this flow, is Pi's choice and not the Host's.
- Not verified: whether either backend honours such fields for these accounts. The pinned Pi runtime does not send them today.
- Question: should the Host pin `store: false`, refuse `background`, bound `service_tier`, and admit only `tools[*].type == "function"`? It is about ten lines in `hosted_fetch`.

**R-b (P2, operator). The payer in the grant is a name; what it is bound to is unpinned and unrecorded.**
- The credentials file is read at Host start (`https.rs:93-147`, `qual.rs:237-241`).
- Nothing records its digest or each payer's source kind. The runtime admission binds route id, payer name and endpoint only.
- So a receipt cannot show which account or key paid. It also cannot show that a payer was bound to `chatgpt-invalid-for-qualification`, which makes every job fail over to the paid route.
- The file is nonsecret by design. Suggested: record its SHA-256 and each payer's source kind in the journal at bind and in the receipt, or bind the digest in the runtime admission.

**R-c (P3, operator). Money on the API route is the Codex catalog proxy.**
- Settlement and the budget use the pinned `openai-codex.json` rates on both routes.
- The record says the model is listed for the API key. It does not say the API price equals the catalog rate.
- Until that is checked, the 700,000 µUSD cap bounds a proxy figure and not real API spend.

**R-d (P3). D4 now has a number.**
- In the retained live streams each delta event is 223–252 bytes and carries about one token. Fixed overhead per response is about 12 KiB.
- The unchanged 1 MiB bound is therefore reached near 4,000 output tokens. That estimate comes from four small runs with reasoning off.
- The reservation covers 128,000 output tokens. A longer answer ends `upstream_response_bound`: Uncertain, full hold, job failed. This is harmless for the 100-token continuity task and decisive for real coding turns.

**R-e (P3). Live fragility, none of it observed.**
- The response head bound is 8 KiB (`pi_stream.rs:20`, `:71`). The Codex backend sends about 30 headers. I did not measure the head size. Over the bound, the attempt is Uncertain after dispatch.
- Chunked trailers are refused (`pi_stream.rs:143`).
- `https.rs:180-184` resolves with no deadline and takes only the first address. An unroutable first address ends the job (released, no failover).
- Any head 401, 402, 403 or 429 is a definitive refusal (`pi_route.rs:66-68`), including one produced by the CDN. The router then keeps the refused route out for the rest of the job. In both transition runs, requests 2–4 went straight to the API payer. This is within the grant, but one transient 429 moves the rest of a job to paid funding.

**R-f (P3). Small hardening gaps in `https.rs`.**
- `accountId` is length-checked but not charset-checked before it goes into a header (`:77`). It comes from the operator's own auth store.
- Only `api-key-file` is checked for owner and mode (`:113-121`). The credentials file and the Pi auth store are read as found.
- `chatgpt-invalid-for-qualification` ships in the production transport with nothing but its name to mark it (see R-b).

**R-g (note). The grant deadline no longer reaches the socket.**
- Timeouts are set at connect from the job deadline (`hosted.rs:766`, `https.rs:185-195`).
- The admitted deadline is checked once before the write (`hosted.rs:812-815`).
- These are equivalent while grants expire at the job deadline (`qual.rs:204`).

**Residual from Q-b (note).** At the drain bound, a chunk that was not yet requested from the reader thread is not seen. This is inherent to a time bound and consistent with K-a.

**N3 wording (note).** After a crash, the retained file can hold one chunk that was appended but not yet written to the container. The reason and `relay_sha256` call it "relayed". The bytes are screened either way.

### Focus-area answers not covered above

- **N1–N6 and Q-a.** Verified in source; the table in section 3 has the lines. `unforwarded.bin` is written only after `drain_done` has screened it, so N1 is closed on every path that retains.
- **HTTPS transport.**
  - Origins and paths are fixed per endpoint, and the credential kind must match the endpoint (`https.rs:157-169`).
  - Every payer's secret is screened whichever route is active (`:232-235`).
  - TLS uses the platform verifier with the fixed server name and offers no way to weaken it (`:149-156`, `:188-193`).
  - Nothing refreshes; access expiring within 10 minutes is refused at load (`:80`). That exceeds the 240 s and 300 s job windows.
  - The refresh token is never read.
  - Error strings that reach the journal carry no secret.
  - Docker children get a cleared environment (`pi_docker.rs:567`).
- **K-e.** Connect and TLS run before the gate and outside the store transaction. The write stays inside the admitted dispatch, after `gate_entered` is saved (`hosted.rs:841-848`).
- **Embedder binding.** With the variable set, the HTTPS transport refuses loopback routes; without it, the loopback transport refuses HTTPS routes (`qual.rs:237-244`). The two cannot mix.
- **Live class admission.** `ProviderPlan::validate` (`provider.rs:420-490`) and `verify_live_job` (`job.rs:198-265`) both bind binary, job, model, task, catalog, config, requests, budget and the route-table digest. Retired schemas are refused by name before anything else (`pi_coding.rs:325`). The server spawns the Host with a cleared environment plus the profile's `env`, and refuses `env` names containing `TOKEN` or `KEY` (`job.rs:841-848`, `:452-455`). The one gap is M5.

### Evidence adequacy

**What the evidence supports**
- **Offline matrix r7.** The checks support the record's claims for the 19 scenarios. `matrix.json` is now reproducible from `check.py`. The cutover Host binary equals the r7 binary: `54c91e78…` in `matrix.json`, `c1/bundle/job-profile.json` and `c2-transition.json`.
- **Live probe.** Four tool-free runs of about 8 KiB each show no sentinel and no tail. The 16 completed live attempts, including tool-call turns, agree.
- **Four live runs.** The ledgers, gate logs, `NetworkMode: none` and the absence of an `/auth` mount match the table for the runs I inspected.
- **Crash cuts.** The two new tests simulate the cuts by discarding in-memory state and reconciling. They are unit-level, as the record says.

**Limits**
- The two `qualify.py` live runs used binary `e6674b56…` (`38803cff`), which no retained matrix ran. Only the cutover runs share the matrix binary.
- The sample is one run per configuration, one model, one task, with reasoning off.
- Both real transitions come from the synthetic invalid token (M6).
- No live run exercised a fault, a cancel, a crash, a tail over 1 s, the K-a branch, or a response near the bound.

**Still to qualify or decide before calling the vertical complete**
1. M1 and M2, and the decisions in R-a and R-b.
2. A real funding refusal (429 or 402) and a genuinely expired or revoked credential, with the refusal bodies kept.
3. D4 against a realistic output size (R-d), and price parity on the API route (R-c).
4. One live fault: a mid-stream cancel or crash against a real route, showing the upstream closed and the hold retained.
5. The pre-dispatch cancel and a gate refusal between attempts in the Docker flow; both are still unit-only.
6. A retained result for the live secret scan.
7. The plan's phase 6 item on registration and permission. The primary route is Pi's Codex OAuth token sent by the Host. The plan says reusing that token "without qualification is insufficient" and asks for the actual registration and permission checks. The record is silent on this.
8. K-c: grants are still self-issued by the producing binary (`qual.rs:192-204`), now for real money.
9. The phase 5 review gate, the broad regression, and owner acceptance (section 4).

## 3. Previous findings

| Item | Status |
| --- | --- |
| N1 | Fixed. The drain screens what it read; access material drops the bytes and interrupts (`pi_stream.rs:627-643`). Tested, and exercised by `echo-trailing`. Residual: M3. |
| N2 | Fixed. Held bytes seed the drain (`pi_stream.rs:721`). Completion needs a framed end or a finished drain, and zero dropped bytes (`provider.rs:154-161`). Tested at both levels. The read boundary is still not reproduced in Docker. |
| N3 | Fixed by incremental retention as decided. The chunk is appended and synced before it is queued, and the head status is saved with the file's creation (`hosted.rs:1231-1270`). Settlement compares file and relay (`:1023-1024`). Reconcile registers and digests (`:1140-1192`). The r7 `crash` check covers it. Residual: the wording note above. |
| N4, Q-b | Fixed. The drain accumulates to the framed end, 1 s or 64 KiB, with a final read at the bound (`pi_stream.rs:811-836`). The size bound has no test. Residuals: M4 and the Q-b note. |
| N5 | Fixed. The first `response.completed` is the terminal for usage, the raw terminal and the tail check (`pi_stream.rs:340-346`, `:382`; `hosted.rs` `hosted_settle_relay`). Tested. |
| N6 | Fixed. The drain error is recorded and named (`pi_stream.rs:830`, `provider.rs:262`). An existing interruption code is kept (`hosted.rs:1313-1318`). The completion refusal says a released attempt was released (`provider.rs:693-696`). The drain-error reason is untested. |
| Q-a | Implemented as decided. `terminal_last` is required with or without the framed end (`provider.rs:251-275`). `trailing-data` is Uncertain. The backend was checked for a sentinel before live use. |
| Q-c | Fixed (`hosted.rs:1167-1171`). The flag is set only on the run path. Unit-level only. |
| Q-d | Unchanged, as the record says. |
| K-b | Closed. Per-payer access with kind and endpoint matching; a per-endpoint header allowlist (`hosted.rs:148-167`); every secret screened. Body fields are a new question (R-a). |
| K-c | Open, unchanged. It now applies to real funding. |
| K-d | Closed. Hosted `max_context_bytes` is at most 32,768 (`provider.rs:435`), and 24,000 for the live class. |
| K-e | Closed. See R-g. |
| K-f | Closed. The admission has no native-access fields (`provider.rs:393-408`). The harness no longer supplies them. |
| K-g | Closed. Container usage is stored as `container-reported`, `evidence: false` (`provider.rs:677-680`). |
| K-h | Unchanged. No action needed. |
| D4 | Unchanged at 1 MiB. Now quantified (R-d); the record's size figure is wrong (M6). |
| Harness: crash witness | Fixed. The observer stays up and the ledger is re-read after the replay. |
| Harness: derived charges | Labelled as derived. The Host still does not export its charge. |
| Harness: release check | Reworded to what it checks. |
| Harness: gate log | Compared per attempt in every non-crash scenario (`check.py:78-84`). |
| Harness: screen challenge | Fixed in a body, a refusal body and trailing bytes. |
| Harness: zero-byte digests | Labelled as such for `stall-head` and `cancel-head`. |
| Harness: reproducible `matrix.json` | Fixed. |
| Previously unexercised paths | Now exercised: screening, a truncated refusal, a fault on request 3, a 401 (live, as definitive), two more crash cuts (unit). Still unexercised: pre-dispatch cancel in Docker, the 1 MiB bound, an oversized refusal body, 403, a gate refusal between attempts, a crash during the drain, deadline expiry mid-stream, and non-chunked framing. |

## 4. Gates this review cannot close

- **Phase 5 independent review.** The plan names Astra/xhigh and says a Claude implementation or Claude self-check cannot close that gate. This is a same-provider review, as the packet states. It closes the gate only if the acceptance owner has explicitly replaced the reviewer contract; I found no such record in what I read.
- **Broad regression.** "Recorded separately"; not reviewed here.
- **Owner acceptance.** Every result is `owner_pending`.

## 5. Not verified

- **Commit identity and range.** See the top of this review. I also could not confirm that the manifest lists every changed path.
- **Runtime behaviour.** Nothing was executed. Test counts are as reported. I counted 25 test functions in `tests/pi_stream.rs`, which matches.
- **Secrets in live run directories.** I did not scan for them and read no secret to compare against. No retained artifact records the scan.
- **Binary provenance.** Nothing binds any binary digest to a source commit.
- **TLS in practice.** The platform verifier's behaviour and the rustls crypto provider selection were read, not exercised. `Cargo.lock` was not audited.
- **Backend behaviour.** Whether either backend honours hosted tools, `service_tier` or `store` from this client; response head sizes; refusal shapes for real quota or revocation; API prices against the catalog.
- **M3 and M4 triggers.** Both are derived from the code; I did not construct either.
- **Retirement commit `8fa71188`.** Reviewed only where it touches the transport, ledger, gate and live admission. I did not read the receiver `.mjs` files, the removed adapters, `tests/coding.rs`, `tests/sol_wire.rs`, `job/tests.rs`, `tests/preparation.rs` or the session-events test.
- **Not read in full.** `matrix.json` beyond its header and pass flags; r7 per-run state; the observer example beyond its scenario list; `pi-pressure-context-host/src/coding.rs` beyond the gate's `provider_attempt`; `pi_route.rs` beyond `refusal_status`.