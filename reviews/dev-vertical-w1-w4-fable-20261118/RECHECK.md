# Recheck: W1-W4 remediation 48908c99

Reviewer: Fable 5.1, read-only, 2026-11-18. Inputs: `remediation.diff` (48908c99 vs 8876b435), working tree for the touched paths, the live run `live/live-c1` and the new run set `w4b` under `~/.local/state/work-engine/dev-vertical-h2-route-20261117/`. Limitations: no shell, so `cargo test` was not run — the supervisor should run `cargo test -p work-engine-manual-host --lib` in `rust/` before merging. No Docker, no network.

## Verdict: accept

All five P1/P2 findings are resolved as the disposition states; the P3 dispositions are correct. The remediation introduces no defect that changes money, containment or evidence; three P3 notes below.

## Per-finding status

### P1-1 — one-hour cache writes underpriced; reservation not worst case — **resolved**

- **Extraction** (`pi_stream.rs` L516-535): the breakdown is taken from the last `message_delta` when it carries a `cache_creation` object, else from `message_start`; the split is used only when `5m + 1h == cache_creation_input_tokens` (checked with `checked_add`), otherwise `(0, total)` — every write is priced as one-hour. When the total itself is absent both split fields are `Null`, which later fails settlement (same as before for a missing total). In the live streams the delta has no `cache_creation` object and the start's breakdown sums to the delta's total (e.g. request 3: `ephemeral_1h_input_tokens: 58868` = `cache_creation_input_tokens: 58868`), so the split comes from the start and is consistent; any disagreement falls to the dearer class. Test added for split / non-adding split / absent (`pi_coding_hosted.rs` L2975-3004).
- **Raw terminal** (`pi_coding_hosted.rs` L1392-1395) carries `cache_creation_5m_input_tokens`/`cache_creation_1h_input_tokens`; the settlement (`pi_coding_provider.rs` L996-1010) requires both to be `u64` and to add up to `cache_creation_input_tokens`, else `"raw completed stop reason/usage unavailable"` with the full reservation retained.
- **Pricing** (`pi_coding_provider.rs` L706-723): tier chosen by `input + 5m + 1h + read`; 5m writes at the tier's `cacheWrite`, 1h writes at `2 × input` of the tier, ceil + 1. Unit values check: 217 = ⌈1 + 50 + 120 + 20 + 25⌉ + 1; 201; 126; 100 002 (100 001 writes cross the 100 000 threshold → 1.0/M).
- **Reservation** (`messages_full_reservation`, L735-760): max over base and every tier of {input, 2×input, cacheRead, cacheWrite} for the window, max output for `maxTokens`. With the pinned catalog (base 0.1/0.5/0.01/0.125, tier 0.5/2.5/0.05/0.625 — confirmed by the receiver's own `cost` figures in `live-c1/state/0002-rpc-0.stdout` L12/L16 and by the recomputations below) that is 10⁶ × 1.0 + 128 000 × 2.5 + 1 = **1 320 001**; the w4b receipt reserves exactly that on every attempt (`w4b/ok/state/analysis-host-result.json` L152-301). `MESSAGES_ANALYSIS_BUDGET` = 43 000 000 ≥ 32 × 1 320 001 = 42 240 032 (`MAX_REQUESTS` = 32, `pi_coding_analysis.rs` L12). `analyze.py` L43 uses 1 320 001 × MAX_REQUESTS. No stale `945 001`/`32 000 000` remains in Rust or the record other than the deliberate historical sentence (README L202).
- **Responses untouched**: `request_reservation` (L487-493) dispatches on wire; `full_reservation`/`rates()`/`cost` are unchanged and the Responses tests (L1077-1079: 164 001 / 984 006) still describe them. The one remaining direct `full_reservation` caller is a Responses-only test (`pi_coding_hosted.rs` L2506). All three production reservation sites (`reservation_microusd` L512, grant L854, hosted dispatch L995) go through `request_reservation`.
- **Record numbers**: recomputed from `live-c1` `relay.json` usage with all writes one-hour (the streams report `ephemeral_5m_input_tokens: 0` on all three):
  - request 1: 2×0.1 + 1658×0.2 + 0 + 36×0.5 = 349.8 → **351** ✓ (was 227)
  - request 2: 0.2 + 1058×0.2 + 1658×0.01 + 905×0.5 = 680.88 → **682** ✓ (was 603)
  - request 3: 0.2 + 58 868×0.2 + 2716×0.01 + 1145×0.5 = 12 373.46 → **12 375** ✓ (was 7 960); Pi's own figure 12 373 ✓ (truncation vs ceil+1).
  - w4b synthetic (40 in, 3000 writes, 1000 reads, 120 out; no breakdown → all 1h): 4 + 600 + 10 + 60 = 674 → **675** ✓, three settled requests at 675 in the receipt and `summary.json`.

### P1-2 — receipt names `openai-codex` — **resolved**
`pi_docker.rs` L1382-1391: `container_provider_identity = plan.provider`; `cost_evidence` chosen by `plan.wire()`. w4b receipt L351-352 shows `claude-sdk` and the Anthropic cost basis. The `_ =>` arm (unknown wire) is unreachable after `validate` (`wire_checked()` is required there).

### P2-1 — local forwarder refusals invisible — **resolved**
Every local answer goes through `forwardRefuse` (`pi-coding.mjs` L122, 126, 130, 131, 134, 147), which increments `forwarderRefused`; 413 is counted once (the `over` latch). The `complete` frame carries `counts.forwarderRefused` only when `claudeSdk` (L440), which is `hosted && config.provider==='claude-sdk'` (L34) — the same predicate as the Host's `analysis_counts` (`pi_coding_analysis.rs` L337-349), so the two sides' expectations are symmetric. `analysis_complete` compares whole `counts` objects (L366). The w4b run reported `{"providerRequests":3,"resets":0,"forwarderRefused":0}` (`w4b/ok/state/0002-rpc-0.stdout` L50) and the receipt is `component_passed`. See P3-new-1 for a naming/doc note.

### P2-2 — missing overage header accepted — **resolved**
`pi_coding_hosted.rs` L1643: `overage == Some("rejected")`; doc comment L1603-1612 records the assumed semantics; test L3013-3030 now requires `rejected` for the `allowed_warning` case and adds the status-only case as refused. Record L233-240 states the assumption.

### P2-3 — retry header may be absent — **resolved**
L765: `headers.get("x-stainless-retry-count").map(String::as_str) != Some("0")`; header names are lowercased on ingest (L717) and the node forwarder lowercases too, so the live/w4a `"0"` values continue to pass. Test for the absent header added (L2879-2886). Record L256 reworded.

### P3-a — thinking attribution — **resolved** (README L307-313 names the `thinkingLevelMap` and the untried `disabled` step).
### P3-b — canonical copy — **accepted as disposed** (no change required).
### P3-c — API-key failover untested — **resolved** (README L320-323: do not enable live before one check).
### P3-d — `max_tokens` continuation — **resolved** (README L256-259).
### P3-e — `unwrap_or(Wire::Responses)`; forwarder coverage — **accepted as disposed**
Keeping the fallback in the uncertain-close cleanup is a defensible choice (that path must not fail on a plan read); unreachable for a validated plan. The coverage statement is in README L323-325.

## New findings

- **P3-new-1** — `forwarderRefused` also counts the 502 "host relay failed" path (`pi-coding.mjs` L147), where the request *was* admitted (`slot.used`, `provider_fetch` emitted) and the Host's relay failed before headers. The Host doc comment (`pi_coding_analysis.rs` L332-336) and the record (README L247) describe the count as requests "answered locally … outside a grant". Effect is bounded and fail-closed: such a job cannot complete anyway (the attempt is refused/uncertain or Claude Code surfaces the error), so no false completion is possible; only the name and prose are imprecise. Smallest fix: one clause in the comment/record ("or a relay the Host did not answer").
- **P3-new-2** — No Rust unit test exercises the `claude-sdk` branch of `analysis_counts` (`pi_coding_analysis.rs` L337-349); the only `counts` tests are the claims-bridge ones (`pi_coding.rs` L1582, L2434, L2498). The positive path is covered end-to-end by w4b; the negative path (non-zero count → refuse) is by JSON inequality. Suggest one unit assertion with `forwarderRefused: 1`.
- **P3-new-3** — The completion requirement `forwarderRefused == 0` has not been exercised live (the record says the live run was not repeated, README L298). If Claude Code ever issues an auxiliary request to `ANTHROPIC_BASE_URL` (e.g. a token count), live jobs will now fail closed at completion rather than silently. That is the intended direction; note it as a known condition for the next live run.

No P0/P1/P2 new findings. No stale numbers found; no change to the Responses wire; the guard and retry changes are strictly fail-closed on all observed evidence.