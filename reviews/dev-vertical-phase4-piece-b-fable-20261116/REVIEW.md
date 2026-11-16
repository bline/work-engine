# Independent implementation review: phase 4 piece (b), hosted Pi provider transport

Subject: `9131a20d`..`c66f2baf` (base `0933a91e`). All 17 manifest blobs match both `c66f2baf` and the working tree. Read-only: nothing was built, run or modified.

Path shorthand (all under `rust/crates/`):

| Short | Path |
| --- | --- |
| `hosted.rs` | `work-engine-manual-host/src/pi_coding_hosted.rs` |
| `provider.rs` | `work-engine-manual-host/src/pi_coding_provider.rs` |
| `pi_coding.rs`, `pi_docker.rs`, `pi_stream.rs`, `pi_route.rs` | `work-engine-manual-host/src/…` |
| `receiver` | `work-engine-manual-host/receiver/pi-coding.mjs` |
| `gate` | `pi-pressure-context-host/src/coding.rs` |
| `qual` | `pi-pressure-context-host/src/bin/coding-qualification.rs` |
| `store` | `lifecycle-store/src/pi_coding.rs` |

## 1. Disposition

**Accept with required changes** for proceeding to piece (c).

I found no P1. No path releases money without a definitive refusal or a confirmed non-dispatch. No path lets an unsettled attempt authorize another. No upstream write happens outside a gate-admitted dispatch. The required changes are all about evidence on failure paths, which is what (c) is meant to qualify. D1–D3 should land before (c) relies on that evidence. D4 must be fixed or explicitly deferred before any live-provider run.

## 2. Findings

### Confirmed defects

**D1 (P2). Gate-admission evidence is volatile, success-only, and logged before the currentness check.**
- Evidence:
  - `gate:323-335` consumes the attempt and pushes to `hosted_log` before `with_pi_coding_turn_send` runs. A store refusal, a cancel scan hit and a failed dispatch are all logged the same as an admitted dispatch.
  - `gate:757-761` attaches the log only to the phase-3 success report.
  - `qual:247-252` calls `continuity.report()?`, which errors when the run is unfinished, so nothing is written on failure.
  - `hosted.rs:646` computes `entered` but never persists it.
  - `hosted.rs:674-682` records one reason, "attempt admission or dispatch failed", for both a gate refusal and a write failure.
- Why it matters: on every failing run (uncertain attempt, cancel, crash) there is no record that the gate admitted the attempt. The ledger record is written before the gate is consulted (`hosted.rs:614`), so it cannot tell "reserved, gate refused, nothing written" from "dispatched".
- Minimal change:
  1. Push to `hosted_log` only inside the closure the store runs, after `parent_binding` passed, and include the deadline.
  2. Add an accessor for the log and have the embedder write it on every exit.
  3. Persist `gate_entered`, the deadline and the policy digest on the `AttemptRecord`, saved before `Session::start`, with distinct reasons for "gate refused" and "dispatch failed".

**D2 (P2). Bytes already relayed to the container are discarded on protocol-error and abort paths.**
- Evidence:
  - `hosted.rs:463-469`: when `on_native` errors (bad ordinal or interleaved frame, possibly mid-body), the code calls `hosted_uncertain` and sets the phase to `Closed`. The session and its `Relay.forwarded` are dropped with no `stream.sse` and no `relay_sha256`.
  - `hosted_uncertain` (`hosted.rs:887-914`) takes no relay.
  - `pi_docker.rs:805-818`: on deadline, cancel, write failure or an unrelated frame error, the loop kills the container with no hosted cleanup. The attempt stays `pending` with no reason, and the upstream socket stays open until the `Host` is dropped.
- Why it matters: the operator contract requires complete screened SSE retention. The attempts that end uncertain with money retained are the ones that lose their bytes.
- Minimal change: before closing an `Active` phase on the `on_native` error path, retain `session.relay().forwarded` and record its digest on the attempt. Add one `coding_hosted_abort(reason)` on run-loop exit that does the same, marks the attempt `Uncertain`, and drops the session.

**D3 (P2). Retained SSE and refusal bodies are not registered as artifacts or verified at collection.**
- Evidence:
  - `hosted_retain` (`hosted.rs:916-936`) writes `coding-hosted-<r>-<a>.*` into `state_dir`, with no directory fsync.
  - `receipt()` verifies only `journal.artifacts` (`pi_docker.rs:1270-1273`). The retained files are not among them.
  - The per-attempt evidence JSON (`hosted.rs:826-830`) is written with `atomic` and is not digest-bound anywhere.
- Why it matters: the binding to the attempt digest holds at write time only. A missing or altered file does not affect `component_passed`. Design §5 asked for a create-only checked store artifact.
- Minimal change: push `Artifact{path, sha256}` for each retained file into `journal.artifacts`, or verify them against the attempt records in `receipt()`. Fsync the directory. Bind or drop the `.json`.

**D4 (P2 before live use; does not block offline (c)). Design §6 response bound is not implemented.**
- Evidence: `MAX_RESPONSE` is still the 1 MiB constant at `pi_stream.rs:19`. There is no profile-set bound and no recorded bytes-per-token measurement. The reservation covers 128k output tokens.
- Why it matters: hitting the bound correctly yields `Uncertain`, but that retains the full reservation and fails the job. Whether a long turn exceeds 1 MiB is not established.
- Minimal change: do the measurement and set the bound in the profile, or record an explicit deferral to phase 6.

**D5 (P3). Foreign and out-of-phase stream frames are bare refusals.**
- Evidence: `hosted.rs:445-447` and `:494` refuse without marking the attempt. It stays `pending` and `provider_failed` is not set.
- Why it matters: design §3 says these are never a bare protocol error. Financially it is equivalent, because `pending` is charged and blocks.
- Minimal change: route these through `hosted_uncertain` when the phase is `Active`.

**D6 (P3). A missing transport is discovered after reservation, and the gate is lost.**
- Evidence: `run()` checks the gate but not the transport (`pi_docker.rs:989-998`). `hosted.rs:636-643` takes the gate, then returns with `?` on the absent transport, so the gate is dropped and the attempt is already reserved.
- Minimal change: require the transport in `run()` for hosted profiles, and take the transport before the gate.

### Risks and open questions

**K-a (P2 for (c) coverage). Completion depends on a framed upstream end within the 1 s drain.**
- Pi always releases the body right after `response.completed`: `mapCodexEvents` returns, then `parseSSE`'s `finally` calls `reader.cancel()` (`openai-codex-responses.js:562-571`, `:636-638`).
- So every success goes through `(AwaitOp, cancel)` into `Draining` (`pi_stream.rs:426`, `:723-735`), and `from_relay` (`provider.rs:134-145`) requires `Cancelled{upstream_end:true}`.
- Trailing bytes or a close later than 1 s yields `Uncertain`, full reservation retained and a failed job, even though a screened `response.completed` was relayed.
- The offline observer writes the terminator immediately, so this is untested. (c) should add delayed-close and trailing-byte variants.
- Open question for the operator: is the conservative rule intended for live providers?

**K-b (P3, fix before phase 6). Payer binding is nominal.**
- `LoopbackTransport` applies one access token to every route and ignores `route.payer` (`hosted.rs:185-192`).
- Container headers are filtered by a four-name denylist (`hosted.rs:31-36`, `:427`). Under an API-key route, headers such as `OpenAI-Organization` or `OpenAI-Project` could change the billing context.
- Hop-by-hop filtering lives only in the loopback transport (`hosted.rs:192-195`).
- Suggested change: an allowlist in `hosted_fetch`, and a transport contract that selects the credential by `(route, payer)`.

**K-c (P3). Grants are self-issued.** `qual:186-199` derives the policy from the job and installs the matching grants itself. Here the grant adds no authority beyond the runtime admission's `hosted_routes_sha256`. That is fine offline, but (c) should not cite it as issuer evidence.

**K-d (P3). Bounds are not reconciled at admission.** The plan admits `max_context_bytes` up to 65536 (`provider.rs:390`). The hosted body bound is 45000 (`hosted.rs:27`, `receiver:243`) and the frame bound is 65536 (`pi_coding.rs:961`, `receiver:53`). An oversized turn fails closed, but only after the reservation and grant.

**K-e (P3). Blocking and throughput.**
- Connect and write (up to 5 s each, `hosted.rs:180-181`) run inside an IMMEDIATE store transaction (`store:202-222`) and the Host loop. This must not carry over to a TLS transport.
- `coding_hosted_before_complete` can block up to 2 s (`hosted.rs:947`).
- One chunk is delivered per pull per 20 ms tick or slower. Throughput on long streams is unmeasured.

**K-f (P3). Admission vestiges.** Hosted admission still requires `allow_native_access_projection: true` (`provider.rs:485`), a `native_auth_source` and an `access_adapter`, none of which it uses.

**K-g (P3). Container-reported usage is still stored.** `provider.rs:786` stores the container's `usage` in the journal and receipt, unlabeled, next to the Host evidence.

**K-h (note). Cancel in `Fetched` keeps the request-level reservation** (`hosted.rs:474-482`). This is conservative and inconsistent with `NotDispatched`. No change needed.

## 3. Focus areas

**1. Financial correctness: sound.**
- Reservation is durable before any byte: `reserve_attempt`, then `save()` at `hosted.rs:614`, then the gate, then `open`.
- Release happens only on `Refused` or `NotDispatched`. `Refused` requires a refusal status, zero bytes relayed, and a drained and screened body, re-checked by the ledger at `provider.rs:192`. `NotDispatched` is set only when the cancel scan hit before `open`.
- Connect failures, gate refusals and expiry all become `Uncertain`, which is conservative.
- `Pending` and `Uncertain` block via `unsettled()`, `provider_failed` and "previous attempt not refused" (`provider.rs:287`).
- Budget: the request-level hold is not double-counted with attempt 1, and a refused attempt is released durably before the next is reserved.
- Settlement: `settle_completed` applies only to the last attempt, only if `Completed`, and only up to its reservation. It uses the Host-produced status, model and raw terminal.
- The router allowance is `u64::MAX` (`hosted.rs:114`).
- Restart: `run()` refuses anything not `Admitted`, and `reconcile()` never touches the ledger. An unsettled attempt stays charged and is never released or re-dispatched. It is not relabelled as restart-unsettled.

**2. Grant coverage: yes, with caveats.**
- The only `transport.open` call is `hosted.rs:669`, inside the gate closure and inside the store transaction that checks the grant (not revoked, unexpired, current revision, active ownership).
- The gate enforces the request match, strict attempt sequence, the per-request limit and the `(route, payer)` pair (`gate:296-321`).
- The policy digest (pairs plus limit) is in both turn grant ids, including the rehydration turn.
- The deadline is min(grant expiry, configured expiry, job deadline) (`gate:335`, `hosted.rs:652`). It bounds dispatch and the per-read idle timeout, not total stream duration; the job deadline bounds that.
- Caveats: endpoints are bound by admission, not the grant. See K-b and K-c.

**3. The deviation: adequate as a fence, not as evidence.**
- Control flow guarantees no dispatch without the gate, and the ledger record is durable.
- As evidence it fails on exactly the failure paths (D1).
- A lifecycle-store record is not strictly required if the three D1 changes land.
- It becomes required if an acceptance owner needs gate admission provable independently of the Host journal, since the Host is the fenced party.
- A possible route is an ingress row outside the turn (no `turn_id`). The settlement query selects by `turn_id` (`store:307`). I did not verify that this avoids the grammar check.

**4. Host-loop integration and races: no defect found for a conforming receiver.**
- Session polling is zero-wait. `Draining` is polled.
- The cancel scan runs inside the gate closure, immediately before `open` (`hosted.rs:657-668`). The residual time-of-check gap is inherent and ends `Uncertain`.
- Cancel in `Fetched`, a cancel crossing `response-unavailable`, and a cancel in the head or refusal state were each traced and are deterministic.
- Non-stream frames, such as `native_event`, are routed by type while a stream is open. This is necessary and harmless.
- Lifecycle commands are written before queued stream frames within one iteration. No inversion is reachable, because the receiver blocks on every stream reply except `ack`, and Pi emits `done` only after the release is confirmed.
- Gaps: D2 (loop exit), D5, K-e.

**5. Evidence and isolation: isolation is good; evidence has gaps.**
- Container `provider_observation` is refused (`pi_coding.rs:1050`).
- Status, model and raw terminal are Host-produced. The raw terminal is a status-plus-usage projection that mirrors `rawTerminalEvidence`, not the "full payload" design §4 words; the full bytes are retained.
- Credential headers are stripped, CR/LF is rejected, and `content-encoding` is refused. The receiver's zstd decompression is bounded at 45001 bytes.
- Relayed bytes and refusal bodies are screened for the access token.
- Network is `none` and there is no `/auth` mount, asserted in the profile test and re-checked by `verify`.
- Gaps: D2, D3, K-b, K-g.

### Earlier design-review items

| Item | Status |
| --- | --- |
| R1 | Resolved. Attempt records, a single ledger release rule, and the router allowance disabled. `router.next` runs before `reserve_attempt`, with a `NotDispatched` unwind; this is equivalent. |
| R2 | Superseded by operator decision: the lifecycle gate replaces `PiOwner`, and `Effect::ProviderAttempt` is unused. There is no outcome-observing lease, so "uncertain cannot authorize another" rests on the ledger alone. Residual: D1. |
| R3 | Partly resolved. Bytes are retained and digest-bound in the journal. Gaps: D2, D3. |
| R4 | Resolved. `Unavailable::answer` and `on_next` are used, and the crossing cancel is tested. |
| R5 | Resolved. The pre-dispatch cancel scan releases as `NotDispatched`, and is tested. |
| K1 | Read timeout resolved (`READ_IDLE_MS`, capped by the deadline). Response bound open: D4. |
| K2 | Resolved in piece (a) (polled `Draining`). A bounded wait of up to 2 s is reintroduced in `coding_hosted_before_complete`; acceptable. |
| K3 | Resolved. Host-produced and container observations refused; the raw terminal is a projection, not the full payload. |
| K4 | Resolved in piece (a) (`release()` on an unexpected frame). |
| K5 | Holds. `refusal_status` is 401/402/403/429 (`pi_route.rs:66-68`), re-checked by the ledger. |

## 4. Not verified

- **Runtime behaviour.** Nothing was executed. Test results, the Docker run and all timings are taken as reported.
- **Record derivation.** `evidence.json` is not produced by the committed `run.py`, which only prints. Its `state_dir` is outside the repository. I checked only that the five ledger digests equal the `retained_bytes` digests within the file.
- **Live backend behaviour.** I could not check whether the Codex backend sends bytes after `response.completed`, how quickly it closes, or its SSE bytes per token (K-a, D4).
- **Node behaviour.** `zstdDecompressSync` bounds and Streams cancel semantics are reasoned from source, not run against the pinned Node.
- **Lifecycle store internals.** I read `parent_binding`, `with_pi_coding_turn_send` and the settlement ingress query only. I did not confirm that a record outside the turn avoids the settled grammar.
- **Not read in full.** `pi-coding-hosted.test.mjs`, `pi_pilot.rs` and its tests (context only), `coding_complete`, the export paths, and `save()` durability.
