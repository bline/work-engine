# Read-only cross-check: production Pi ledger vs S1 invariants

Snapshot: `snapshot/` at commit `b3577d7a` (`snapshot/COMMIT`). Reviewer: read-only subagent, 2026-11-17.
No files edited, no tests run (no shell available). Code paths below are relative to
`snapshot/rust/crates/`; `provider.rs` = `work-engine-manual-host/src/pi_coding_provider.rs`,
`hosted.rs` = `work-engine-manual-host/src/pi_coding_hosted.rs`.

## Summary

| # | Invariant | Result |
|---|---|---|
| 1 | Retained raw charge | **gap (P3)**: G1 |
| 2 | Malformed identity | upheld |
| 3 | No self-declared authority | upheld (recorded decision K-c noted) |
| 4 | Raw-interval placement | upheld (objective/time-window placement: not applicable) |
| 5 | Distinct evidence, not amount cardinality | upheld |
| 6 | Uncertainty is not release | upheld |

Verdict: OK with notes. One P3 gap; no P1/P2.

## 1. Retained raw charge: gap (P3)

Upheld parts:
- Settlement happens once and uses raw usage produced by the Host. `settle_completed` needs a Completed, unsettled attempt (provider.rs:297-308). A second `provider_complete` is refused (provider.rs:697-699). When attempts exist, the container's usage is stored labelled `evidence:false` (provider.rs:703-707).
- The settled cost is conservative. `cost()` charges every input token at the highest of the input, cacheRead, cacheWrite and tier rates, then rounds up and adds 1 (provider.rs:512-545).
- Pending, Uncertain and Completed-but-unsettled attempts hold their full reservation (`charged_microusd`, provider.rs:176-188). The request-level mirror is the sum of the attempts (provider.rs:313).
- Nothing outside the ledger rolls these figures up. A grep for `microusd` finds only `pi_coding.rs:1623` and `pi_coding_analysis.rs:302`, and both read `settled_microusd` only as a completion gate.
- The router's units only record provenance: allowance is `u64::MAX`, and the financial owner alone admits money (hosted.rs:114-140).

**G1: a raw-usage cost above the reservation is not kept as the held amount.**
- *Location:* provider.rs:760-768. `provider_complete` refuses when `actual > r.reserved_microusd` or when tokens exceed the catalog bound, and records no amount. provider.rs:309-311 (`settle_completed`) refuses the same case. `charged_microusd` (provider.rs:176-188) then returns `reserved_microusd` for the Completed, unsettled attempt.
- *Second path:* at provider.rs:760, `cost(...)?` returns early when the cost is above 1e9 µUSD ("catalog cost overflow"). This happens before the bound check, records nothing and leaves `provider_failed` unset. The request still blocks through `unsettled()`.
- *Scenario:*
  1. A hosted attempt relays a clean `response.completed` with `model:"gpt-6-luna"` and usage `{input_tokens:20000, output_tokens:230000, total_tokens:250000}`. The provider has gone past the catalog's `maxTokens` of 128000, for example because the catalog is stale.
  2. The Host classifies the attempt Completed and writes `raw_terminal` (hosted.rs:1135, 1158).
  3. `provider_complete` computes actual = ceil(20000·0.25 + 230000·0.75) + 1 = 177501 µUSD. That is more than the reserved 164001, so it refuses and sets `provider_failed`; `settled_microusd` stays `None`.
  4. Result: `charged_microusd()` = 164001. The ledger holds 13500 µUSD less than the raw-usage cost, and no field records 177501.
- *Expected (S1 terms):* hold = max(reservation, raw-derived cost). Keep failing the job, but record the raw cost (for example a `raw_microusd` on the attempt, saturating on overflow) and have `charged_microusd` return the maximum.
- *Bounded impact:*
  - `provider_failed`, together with the unsettled check, blocks every later request in the job (provider.rs:623-626) and blocks job completion (pi_coding.rs:1614-1626). No new spend is admitted against the understated figure.
  - The raw evidence survives in `raw_terminal`, the stream artifact and the `relay.json` usage.
  - It is only reachable when the provider goes far enough past the catalog bounds that the max-rate cost exceeds the full reservation. At small input, that means output above about 1.66× `maxTokens`.
- *Coverage:* no test reaches provider.rs:760-768; the string "catalog bound" appears only in the refusal itself. `relayed_attempts_keep_the_reservation_until_raw_settlement` only asserts that `settle_completed(2, 164002)` errors (provider.rs:1396), not what amount is then held. G1 is not in `docs/dev-vertical-open-items.md`. Decision R-c (catalog rates as a proxy) is about rate parity, not overruns, so G1 is not a re-raise of it.

## 2. Malformed identity: upheld

- **Requested model.**
  - `provider_request` requires `frame["model"] == plan.model` and `provider == "openai-codex"` (provider.rs:623-633). This is a `Value`-vs-`String` comparison, so a null, missing, number or empty value fails.
  - `plan.model` is pinned to `gpt-6-luna` (provider.rs:443).
  - The catalog entry's id, provider, api and baseUrl must match (provider.rs:408-415).
  - The runtime admission binds the model (provider.rs:498).
  - The fetch body's model must equal `plan.model` (hosted.rs:611).
- **Backend model.**
  - It is taken from the first `response.completed` `response.model`, and only when that value is a string. A different string is refused and fails the job (hosted.rs:1148-1157).
  - A missing, null or wrong-type value leaves `backend_model` as `None`. `provider_complete` then requires `backend_model == Some(plan.model)` and `http_status == Some(200)` (provider.rs:730-737), so it refuses, keeps the full reservation and fails the job.
- **Unpriceable or malformed usage.**
  - The raw status must be `completed`.
  - Token counts must be `u64`; strings, floats and negatives fail.
  - input + output must equal total (provider.rs:738-759). Otherwise it refuses and keeps the hold.
  - Missing catalog rates or bounds are refused during preparation (provider.rs:512-557).
- **Tests:** `sdk_zero_defaults_…` (provider.rs:1006), the alias model in `error_unknown_identity_…` (provider.rs:873), and `fetch_identity_url_model_…` (hosted.rs:1951). No hosted test sends a missing, null or mismatched `response.model`; that path is verified by reading only.

## 3. No self-declared authority: upheld

- **Container frames.**
  - Container `provider_observation` frames are refused in hosted profiles (pi_coding.rs:1098-1103).
  - Status, backend model and raw terminal are written only by the Host, from the screened relay (hosted.rs:1146-1161).
  - A container `provider_complete` can settle only an attempt the Host classified Completed (provider.rs:710-729), and only for the Host's own stream request (hosted.rs:1462-1468).
- **Attempt order.** The ledger assigns the ordinal: next = last + 1, and only after a Refused attempt (provider.rs:341-345). No frame sets it. The lifecycle gate separately enforces strict sequence and the per-request limit (pi-pressure-context-host/src/coding.rs:301-312).
- **Route and payer.**
  - The intent is built from the Host's pinned route table (hosted.rs:774, 819-824). The runtime admission binds that table's digest (provider.rs:505).
  - The gate admits only a `(route, payer)` pair in its policy (coding.rs:313-319). That policy is built from the same table (bin/coding-qualification.rs:145-158).
- **Non-empty checks before equality.**
  - Route and payer: provider.rs:329-331, `HostedRoutes::validate` (hosted.rs:78-99), `HostedProviderPolicy::validate` (coding.rs:208-227).
  - Session id: pi_coding.rs:1124-1128.
  - The container's request id is checked: it must be the next in sequence, within `max_requests`, with no other request unsettled (provider.rs:613-616, 623-626).
- **Provider data** is used only as gating evidence (status, model equality, usage). It never picks a route, payer or attempt. A refusal status may release a reservation, but only under the design's definitive-refusal rule (see §6).
- **Recorded decisions, not re-raised:**
  - K-c: grants are self-issued by the producing binary (open items, "Other open items").
  - The lifecycle gate, not `PiOwner`, fences provider entry (gate contract, "Operator decisions"). As a result, `Effect::ProviderAttempt` in session-task-admission (pi_pilot.rs:577-600) is called only by tests, never by the hosted path.
  - Subscription route registration and permission is deferred (open items).
- *Note:* the stream `Binding` carries constant empty `admission` and `owner_profile` fields (hosted.rs:490-499). It is a correlation id minted by the Host, not authority. A cancel releases only because the Host itself observed that no request byte was written. Not a gap.

## 4. Raw-interval placement: upheld

- Relayed bytes are tied to the Host's stream request and to the attempt number the ledger returned (hosted.rs:777-796, `StreamPhase::Active`).
- Each chunk is appended to `coding-hosted-{request}-{attempt}.stream.sse` before it is relayed (hosted.rs:1309-1350). Settlement checks that the retained bytes equal the relayed bytes (hosted.rs:1121-1130).
- Settlement targets `attempts.last()` (provider.rs:770-776). That is the only attempt that can have relayed bytes, because a switch happens only after a zero-byte, drained refusal (provider.rs:212-231; hosted.rs:1040-1050).
- `reason` strings are only labels. The disposition comes from which evidence variant was observed (provider.rs:201-295), and `Unknown{reason}` is always Uncertain.
- Recorded times are the Host's own: the gate's deadline is read inside the store transaction (coding.rs:333-342), and the opened/upstream-end times come from the Host's `relay.json`.
- Objective/time-window placement does not apply: this ledger feeds no objective budget or window rollup (no `microusd` references outside work-engine-manual-host).
- *Fragility note (not a current defect):* the request-level `raw_terminal`, `backend_model` and `http_status` are not stored per attempt. hosted.rs:1158 overwrites `raw_terminal` without the duplicate check that provider.rs:684 uses. Correctness depends on the rule that only one attempt per request can relay.

## 5. Distinct evidence, not amount cardinality: upheld

- **Equal amounts count separately.** Holds are keyed by request id (`BTreeMap`) and attempt ordinal (`Vec`). Totals are plain sums with no value-based dedup (provider.rs:176-188, 352-362, 636-641). Test `definitive_refusal_releases_…` (provider.rs:1226) holds attempt 2's 164001 after attempt 1's equal 164001 is released.
- **Byte-identical duplicates are refused, not counted twice:**
  - duplicate `provider_request`: provider.rs:624
  - duplicate completion: provider.rs:697-699, 303
  - re-settling an attempt: the Pending check in `settle_attempt`
  - duplicate fetch: hosted.rs:553-561 requires the Granted phase, and the body file is create-only (hosted.rs:519-531)
  - retained files are create-only (hosted.rs:1353-1366)

## 6. Uncertainty is not release: upheld

- **Release happens only for:**
  - **Refused:** status 401/402/403/429 at the head, zero bytes relayed, body drained and screened (provider.rs:212-231). `Progress::Refused` is set only after the framed end and the screen pass (pi_stream.rs:836-866).
  - **NotDispatched:** confirmed before any write (provider.rs:232-244). There are two callers:
    - connect failure (hosted.rs:860); `connect` does only TCP and TLS and writes no request byte (pi_coding_https.rs:219-244)
    - a cancel seen inside the gate before `send` (hosted.rs:967)
- **Everything else holds and blocks:**
  - provider failure events, non-200 status, interruption, bytes after the terminal event, held-back or dropped bytes, drain errors (provider.rs:146-172, 245-290; pi_stream.rs:358-389)
  - refusal-body read errors and over-bound bodies (pi_stream.rs:858-866)
  - protocol or foreign frames, streams the Host closed, retention failures (hosted.rs:1380-1440)
  - gate refusal, expiry or cancel after reservation, dispatch failure (hosted.rs:810-818, 936-958)
  - restart or end of run (reconcile, hosted.rs:1218-1270)
  - a cancel before any attempt keeps the request-level reservation (hosted.rs:665-675)
  - job completion requires every request settled and `provider_failed` unset (pi_coding.rs:1614-1626)
- **Recorded decisions, not re-raised:**
  - K-a/Q-a: an attempt completes on a clean terminal event even without the framed end (pi_stream.rs:352-357; provider.rs:262-276).
  - The refusal set is fixed at 401/402/403/429; 5xx is never added (design r2 §1).
  - Live refusals, cancels and crashes are unexercised and monitored (open items).
- **Tests:** provider.rs:1251, 1283, 1304, 1447; hosted.rs:2028, 2078, 2120, 2194.

## Observations (report-only, not gaps)
- provider.rs:703-707 labels the container's usage `evidence:false` only when attempts exist. For a hosted request that was granted but never fetched, the container's usage is stored unlabelled in `r.usage`. Nothing reads `r.usage` for money, and that completion is refused (`http_status` is `None`), so no invariant is affected.
- Untested paths that the code does uphold: a hosted missing, null or mismatched `response.model`, and the G1 over-bound branch.

## Read
- `provider.rs` (all), `hosted.rs` (lines 1-1493, plus selected tests around 1754-1862 and the test index), `pi_stream.rs` (head/body framing 40-230, `Relay`/`sse_data`/`terminal_is_last`/`usage` 230-430, refusal/drain polling 780-890), `pi_route.rs` (refusal set, `next`, `settle`: 50-270), `pi_coding_https.rs` (`connect`/`send`: 215-275), `pi_coding.rs` (profile predicates 31-64, provider frame routing 1060-1140 and 1480-1515, completion gate 1606-1630).
- `pi-pressure-context-host/src/coding.rs` (`HostedProviderPolicy` 200-245, `provider_attempt` 285-344, `CodingContinuity::new` 345-388, trait impl 1060-1140); `bin/coding-qualification.rs` 120-180.
- `session-task-admission/src/pi_pilot.rs` (`enter`, `Effect`, start of `attempt_order`: 425-640). Grep for `Effect::` and `pi_owner` usage in work-engine-manual-host.
- `docs/dev-vertical-open-items.md`, `planning/dev-vertical-hosted-gate-contract-20261116.md`, `planning/dev-vertical-phase4-hosted-transport-r2-20261116.md`.

## Not read
- The rest of `pi_stream.rs` (Session internals beyond the excerpts, tests), `pi_route.rs` recovery, the rest of `pi_coding_https.rs` (credential loading, tests), the hosted tests beyond those named, `pi_coding_analysis.rs`, `pi_coding_export.rs`, and the rest of `pi_coding.rs`.
- The rest of session-task-admission (`accounting.rs`, `admission.rs`, `store.rs`, etc.): the hosted path does not use it for provider entry.
- The receiver JS and the metrics probe script.
- Files referenced but not in the snapshot: operator decisions file, Fable review, qualification records.

## Suggested test commands for the supervisor (not run)
`cargo test -p work-engine-manual-host pi_coding_provider`, `cargo test -p work-engine-manual-host pi_coding_hosted`,
`cargo test -p work-engine-manual-host pi_stream`, `cargo test -p pi-pressure-context-host coding`.
