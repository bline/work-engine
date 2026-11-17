# Independent review: hosted Pi transport, remediation and offline qualification (phase 4 pieces (b) remediation and (c))

Subject: `c66f2baf`..`acb80187`. All 17 manifest blobs match both `acb80187` and the working tree. Nothing was built, run or modified. I also read the retained r4 run directories under `~/.local/state/work-engine/dev-vertical-hosted-qual-20261117-r4`.

Path shorthand (all under `rust/crates/work-engine-manual-host/`, except the last three, which are under `records/dev-vertical-hosted-qualification-20261117/`):

| Short | Path |
| --- | --- |
| `hosted.rs` | `src/pi_coding_hosted.rs` |
| `provider.rs` | `src/pi_coding_provider.rs` |
| `pi_stream.rs`, `pi_docker.rs` | `src/…` |
| `observer` | `examples/pi_hosted_coding_observer.rs` |
| `qualify.py`, `check.py`, `matrix.json` | the qualification record |

## 1. Disposition

**Accept with required changes** as the basis for phase 6.

I found no P1. No path releases money without a definitive refusal or a confirmed non-dispatch. No unsettled attempt can authorize another, and no dispatch happens outside the gate. D1, D2, D3, D5 and D6 are fixed as claimed.

The required changes are in the new K-a and drain code, plus one retention gap the crash scenario demonstrates but the record does not disclose. N1–N3 should land, or be explicitly decided by the operator, before any live provider use. N4–N6 are small and belong in the same pass.

## 2. Findings

### Confirmed defects

**N1 (P2). Bytes read while draining are retained unscreened.**
- Evidence:
  - `pi_stream.rs:770-773` stores the drained chunk in `Relay.unforwarded` without calling `screen.hold`; the `screen` argument is unused in the `Draining` arm.
  - `hosted.rs:856-859` writes it to `coding-hosted-<r>-<a>.unforwarded.bin` and registers it as an artifact.
  - Forwarded bytes and refusal bodies are screened; this is the only retained provider byte path that is not.
- Why it matters: a provider that echoes the bearer after the terminal event puts access material into the state directory and the journal's artifact set. That breaks "complete screened SSE retention" and the record's "access token appears in no run file" invariant. The matrix cannot see it, because the observer never echoes the token.
- Minimal change: call `screen.hold(&b)` in the `Draining` arm. On error, drop the bytes and set `Outcome::Interrupted(code)`, which ends Uncertain.

**N2 (P2). K-a settles releases that never drained, including ones that dropped held-back bytes.**
- Evidence:
  - `provider.rs:259` completes any `status == 200 && !interrupted && terminal_last`.
  - `terminal_last` (`provider.rs:167-168`) looks only at `forwarded` and `unforwarded`.
  - `(AwaitOp, cancel)` with non-empty `held` skips the drain (`pi_stream.rs:670-679`) and ends `Cancelled{upstream_end:false}` (`:596-601`). The held bytes are provider bytes after the terminal event; they are neither forwarded nor retained.
  - `(Upstream, cancel)` (`pi_stream.rs:681-689`) ends the same way with no drain at all.
  - Before `bd8478ed` both paths were Uncertain.
- Why it matters: this answers the packet's question. A stream can be settled and not fully retained. A chunk ending `…response.completed…\n\n` plus the first byte of a following line that matches a token prefix (for example `e` of `event:` against a JWT) settles as "completed on terminal event". K-a says a partial line stays Uncertain.
- Reachability: the held-bytes case needs only a conforming receiver and an unlucky read boundary. The `(Upstream, cancel)` case needs a consumer that pulls after the terminal event, which the current receiver (`highWaterMark: 0`) does not do.
- Minimal change: record on `Relay` whether the drain ran, and how many held bytes were dropped at release. Require "drain ran and zero held bytes dropped" for `terminal_last`. Do not write `held` to disk; it can be a token prefix.

**N3 (P2). A Host crash loses the bytes already relayed to the container, and the record does not say so.**
- Evidence:
  - `Relay.forwarded` exists only in memory until `hosted_settle_relay` or `hosted_close_uncertain`.
  - In the r4 `crash` run the observer had served two events (272 bytes in the sibling `cancel` run). After reconcile, attempt 2 has `relay_sha256: null`, `http_status: null` and no `stream.sse`; the only hosted artifact is `coding-hosted-1-1.refusal.body`.
  - `check.py` has no retention check for `crash`, and the README row is silent on bytes.
- Why it matters: the binding contract is complete screened SSE retention. The attempt that ends Uncertain with money retained after a crash is the one with no bytes and no status.
- Minimal change, one of:
  1. Append each screened chunk to a create-only per-attempt file before emitting its `response-chunk`, and have reconcile register and digest what is there.
  2. Record an operator decision that retention does not survive Host loss, with reconcile setting a reason that says the relayed bytes were not retained.

**N4 (P3). The release drain inspects at most one upstream chunk.**
- Evidence: `pi_stream.rs:770-774` finishes on the first chunk read. The r4 `trailing` run settled as soon as the comment arrived (`unforwardedAfterRelease: 28`).
- Why it matters: a comment chunk followed by a `data:` chunk settles as clean, even if the data chunk is already in the Host's buffer. "Only comments or blank lines follow" is established for one read, not for the 1 s drain.
- Minimal change: accumulate into `unforwarded` until the framed end, `until`, or a bound. Stop early only when the tail is already not clean.

**N5 (P3). A second `response.completed` is accepted as the terminal event.**
- Evidence:
  - `pi_stream.rs:329` keeps the last qualifying block.
  - `usage` and the raw terminal use `rfind` (`pi_stream.rs:365`, `hosted.rs:869`).
- Why it matters:
  - First in `forwarded`, second in `unforwarded`: `terminal_is_last` is true and the attempt settles on the first event's usage. That is a `data:` event after the terminal event, which K-a says stays Uncertain.
  - Both in `forwarded`: the Host settles on the second event's usage, while Pi stopped at the first.
- No test covers this.
- Minimal change: take the first `response.completed` as the terminal (`get_or_insert`). Any later data event, including a second terminal, then fails the tail check. Add the test.

**N6 (P3). Evidence labels on three paths are wrong or lossy.**
- `pi_stream.rs:775`: a read or framing error during the drain is swallowed. The attempt can complete under K-a, and `relay.json` shows no error. Record the code in the relay evidence.
- `hosted.rs:1027`: `closed_by_host` overwrites an existing `Interrupted(code)` when the session was awaiting its `ack`. Overwrite only when the outcome is not already interrupted.
- `provider.rs:839`: the refusal says "reservation retained" for a `Refused` attempt whose reservation was released. It appears in the `refuse-all` failures in `matrix.json`.

### Risks and open questions

**Q-a (P3, operator). The "only comments follow" condition is skipped when the framed end is observed.**
- `provider.rs:257` completes on the framed end without consulting `terminal_last`.
- So `[completed][data event][framed end]` is Completed if the data event arrives in the same read as the terminal, and Uncertain if it arrives in a separate chunk (as in `trailing-data`).
- A failure event is still caught either way.
- Question: should K-a's tail rule apply in both branches? That would be deterministic. It would also make a backend that sends a `data: [DONE]` sentinel Uncertain on every turn, so phase 6 should measure this first.

**Q-b (P3). A chunk that arrives exactly at the drain bound is dropped.** `pi_stream.rs:776` releases on timeout without a final non-blocking receive from the reader thread. Negligible, and fixed for free by N4.

**Q-c (note). Reconcile reuses one reason.** `coding_hosted_reconcile_unsettled` also runs at the end of every normal `run()`. An attempt left Pending by a retention I/O failure gets "unsettled at restart; producing process gone", which is untrue in that case. It is financially correct.

**Q-d (note). The gate log is still an embedder convenience file.** `coding-hosted-gate-attempts.json` is written non-atomically after the receipt, is not an artifact, and is absent after a crash. The durable evidence is `gate_entered`, `deadline_ms` and `routes_sha256` on the attempt record. That record carries the route-table digest, not the gate's policy digest, so linking it to the grant id needs recomputation.

### Qualification adequacy

The 14 scenarios support the record's main claims for the paths they exercise: failover only after a definitive refusal, Uncertain for everything else, no later dispatch within a run, bytes retained on non-crash failure paths, and both K-a outcomes. These checks are weaker than their labels:

- **`crash`: no witness for the replay.** `qualify.py:156` kills the observer before reconcile and the replay (`:167-171`), and the ledger snapshot is read before the replay (`:170`). "Nothing was dispatched after…" rests only on the refusal text `existing job: reconcile instead of launch replay`. That text is good evidence, since admission refused before any state was touched. Keeping the observer alive and re-reading the ledger after the replay would close it.
- **Charges are recomputed by the harness.** `qualify.py:182` applies its own copy of the ledger rule to the dispositions. So "full reservation retained" and "nothing charged" restate the disposition checks; they would pass even if the Host's own arithmetic differed. The Host does not export `charged_microusd()`.
- **"only refused/not-dispatched attempts released" checks something else.** `check.py:47-49` asserts that released attempts have no relay digest. It does not assert that other attempts stay charged.
- **Gate log comparison is count-only.** It is checked in the four completing scenarios and `refuse-all` (`check.py:73`, `:83`). Intents and deadlines are never compared with the attempt records, and fault scenarios do not check the log at all.
- **The token check never challenges the screen.** It is a real check, but no scenario echoes the token in a body, a refusal body or trailing bytes (see N1).
- **"relayed bytes retained" for `stall-head` and `cancel-head`** passes on the digest of zero bytes. That is correct, but trivially so.
- **`matrix.json` is not reproducible from `check.py`.** It carries `coding_qualification_sha256`, `observer_sha256` and `runs_root`, which `check.py:135` does not emit. This is the same class of issue as last time's `evidence.json`.

Important unexercised paths:

- Pre-dispatch cancel ending `NotDispatched`. This is one of the two release paths and is unit-tested only.
- Access-material screening in any position.
- The 1 MiB response bound.
- A non-definitive refusal: truncated or oversized refusal body, and 401/403.
- A gate refusal between attempts in the Docker flow (expired or revoked grant).
- A fault on requests 2–4, including the post-reset turn. Every fault hits request 1.
- Crash at other points: between reservation and gate entry, during the drain, and between relay settlement and `provider_complete`.
- Job-deadline expiry mid-stream.
- The K-a edges in N2, N4, N5 and Q-a.
- Any framing other than chunked.

## 3. Previous findings

| Item | Status |
| --- | --- |
| D1 | Fixed. The gate logs inside the store closure with the deadline (`pi-pressure-context-host/src/coding.rs:333-343`). The embedder writes the log on every non-crash exit. `gate_entered`, `deadline_ms` and `routes_sha256` are saved before `transport.open` (`hosted.rs:676-680`), and the three failure reasons are distinct. Residual: Q-d. |
| D2 | Fixed for every non-crash path: protocol error, foreign or out-of-phase frame, poll failure, run-loop exit and launch exit all go through `hosted_close_uncertain`, which retains `stream.sse` and `relay.json` and closes upstream. Residual: N3 (crash) and the N6 overwrite. |
| D3 | Fixed. Retention is create-only with file and directory fsync, files are registered in `journal.artifacts`, and `receipt()` verifies them (`pi_docker.rs:1293-1295`). The unbound `.json` is replaced by the registered `relay.json`. New gap in what gets retained: N1. |
| D4 | Deferred to phase 6 by operator decision. Unchanged (`MAX_RESPONSE` is 1 MiB). |
| D5 | Fixed with D2 when an attempt is active. With no attempt open it is still a bare refusal, which is correct. |
| D6 | Fixed. `run()` refuses a hosted profile without a transport (`pi_docker.rs:1003-1012`). The transport is taken before the gate, and the gate is no longer dropped. |
| K-a | Decided and implemented; `late-close-long`, `trailing` and `trailing-data` behave as decided. The implementation is wider than the decision in N2, N4 and N5, and narrower in Q-a. |
| K-b | Open, unchanged. Still a four-name header denylist and one token for every route. Required before phase 6. |
| K-c | Open, unchanged. Grants are self-issued by the qualification binary; the record does not cite them as issuer evidence. |
| K-d | Open, unchanged. |
| K-e | Open, unchanged. Connect and write still run inside the store transaction. |
| K-f | Open, unchanged. `qualify.py` still supplies `allow_native_access_projection` and `native_auth_source`. |
| K-g | Open, unchanged. Container usage is still stored unlabeled. |
| K-h | Unchanged. No action needed. |

Focus-area answers not covered above:

- **Financial correctness and grant coverage:** no regression. `settle_attempt` is still the only release rule. The new `provider_complete` guard (`provider.rs:831-847`) only tightens it. A provider failure event now forces Uncertain ahead of every completion branch.
- **Crash and reconcile:** nothing is released or replayed. Reconcile changes only `Pending` to `Uncertain`. It runs under the state-directory lock, so the producing process is known to be gone. `run()` and admission both refuse replay. What is lost is N3.
- **SSE parsing in `terminal_is_last`:** CRLF, bare CR, a CR/LF pair split across the two buffers, multi-line `data`, `event:`/`id:` lines and partial tails are handled correctly or conservatively. The terminal block must be complete in `forwarded`. The defects are semantic (N2, N4, N5), not in the parsing.

## 4. Not verified

- **Runtime behaviour.** Nothing was executed. Test counts and timings are as reported. I read the r4 run directories and `matrix.json`; I did not regenerate them.
- **Binary provenance.** The digests in `matrix.json` match the binaries currently on disk. Nothing binds those binaries to source `bd8478ed`. Equality with each run's `binary_sha256` holds by construction, because `qualify.py` writes that field from the same file.
- **Live backend behaviour.** Whether the Codex backend sends anything after `response.completed` (comments, a sentinel, a second event), how quickly it closes, and its bytes per token. This bears on K-a, Q-a and D4.
- **Lifecycle store after a crash.** No check examines the turn or grant state in the lifecycle store after SIGKILL. I did not read the store internals beyond what the previous review covered.
- **N2 trigger in practice.** The held-bytes case is derived from the code. It needs a read boundary that leaves a token-prefix byte after the terminal event; I did not construct one.
- **Not read in full.** The hosted unit-test bodies, `tests/coding.rs` beyond the diff, and the records under `dev-vertical-hosted-review-fixes-20261116` beyond the README.
