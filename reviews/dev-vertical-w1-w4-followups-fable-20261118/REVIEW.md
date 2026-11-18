## Review: W1–W4 follow-ups — `ab4a22db` + `f84d4afb` vs `ed30ad1f`

Reviewer: Fable 5.1, read-only, 2026-11-18. Inputs: `subject.diff`; working tree for the touched paths (the only working-tree delta vs HEAD is `skills/manual-orchestration/references/native-context.md`, so the touched files equal the subject head); live runs `live/live-c1`, `live/live-c2`, offline `w4c/ok`, probe `claude-probe/run1` under `~/.local/state/work-engine/dev-vertical-h2-route-20261117/`; the pinned `pi-claude-subscription` 0.2.4 source under `~/.pi/agent/npm`. Limitations: no shell — `git show` and `cargo test` not run; the supervisor should run `cargo test -p lifecycle-store --test pi_analysis` and `cargo test -p work-engine-manual-host --lib` in `rust/`. `coding-hosted-3.body.json` (≈250 KB, one line) could only be checked by substring, not positionally.

### Verdict: accept — no P0/P1. Two P2 follow-ups (record/test accuracy of what the pre-dispatch check establishes; durable provenance of the new refusal) and three P3 notes.

---

## Answers to the packet's questions

### 1. Are the six thinking-block conditions met exactly, without widening the closed grammar or touching the coding grammar?

**Yes.**

| Condition | Evidence |
|---|---|
| (1) closed set as observed | `pi_analysis_proof.rs` L55-60: `type == "thinking"`, object with **exactly 3 keys**, `thinking` and `thinkingSignature` strings. Pi's recorded block in live-c2 (`state/exports/sessions/…jsonl` L8, L21) is exactly `{"type":"thinking","thinking":"","thinkingSignature":"CAQS…"}`. `redacted_thinking`, extra key, missing signature, `image` all refuse (`tests/pi_analysis.rs` L383-418). |
| (2) non-semantic | Tool calls are selected by `type == "toolCall"` only (L160); the answer is `answer_text` (L63-76), text blocks only; a thinking-only final yields `""` → `answer.is_empty()` → refuse (L223-226; test "thinking-only final" L411). The Host-side projection `check_answer_bound` sums only `c["text"]` (`pi_coding_analysis.rs` L56-61), so thinking never enters any bound. Delivery checks (`delivery`, L27-48) never read content blocks. |
| (3) byte-exact / exact in outgoing requests | Store: `turn_end.message`, `agent_end.messages`, `session_history.messages` and every `pi.provider_request` are compared against the recorded message including the thinking block (L203-205, L231-236, L240; `pi_coding_proof::request` L99-104). Host: `canonical::message` L517-530 restores the original `thinking`/`redacted_thinking` block after `strip_cache_control`, so equality is exact; the test at `pi_coding_hosted.rs` L2970-2984 shows a moved `cache_control` on a thinking block refuses while the prior user-block case (L2965-2969) still passes. Bodies are retained create-only (L697-706). |
| (4) thinking tokens in settlement/bounds | Live-c2 streams carry `output_tokens_details.thinking_tokens` as a part of `output_tokens` (request 2: 94 of 947; request 3: 103 of 1098; `coding-hosted-{2,3}-1.stream.sse` L443/L629). Settlement uses `outputTokens` (relay 3 usage 1098) → 2×0.1 + 58 910×0.2 + 2 716×0.01 + 1 098×0.5 = 12 358.36 → ⌈⌉+1 = **12 360** ✓ (351 and 703 likewise). No settlement code changed. |
| (5) tests | Positive: a2 = [thinking, text, call2, call3], final = [thinking, text] → `CompletedSettled` (L367-381). Negatives: unknown type, redacted, extra key, missing signature, thinking-only final → `EnteredUncertain` (L383-418). Continuity test as above. |
| (6) effective thinking setting | README L377-386 records adaptive/`display: updates`, `effort: low`. Live-c2 bodies 1–2: `"thinking":{"type":"adaptive","display":"updates"}`, `"output_config":{"effort":"low"}`; Pi's message carries `"thinkingLevel":"low"` (session L8). |

Scope: the lifecycle-store change touches only `src/pi_analysis_proof.rs` and its test file; `pi_coding_proof.rs` is not in the diff and `text()` (L21-34) is unchanged — the coding grammar keeps its own separate `valid_thinking` (L541). The Host `canonical` change is the "outgoing requests" half of condition 3, used only by `messages_policy` (L884-885; Messages wire only). No Codex/Responses path is touched.

### 2. Does the continuity change keep both canonical rules for every other block and compare thinking blocks exactly (including under the string-content rule)?

**Yes.** `strip_cache_control` (L501-512) and the string→one-text-block rule (L519-521) are unchanged; the new loop (L522-529) only overwrites positions whose *original* block is `thinking`/`redacted_thinking`, with `zip` over equal-length arrays (strip preserves array length). String content has no blocks, so `as_array()` is `None` and nothing is touched; a string on one side vs `[thinking, text]` on the other differs in length → unequal → refuse (fail-closed). Existing `is_prefix_extension` (L533-549) is untouched. Compared exactly means a moved `cache_control` on a thinking block refuses — that is the owner's stated condition, not a defect (see P3-1 for live coverage).

### 3. Pre-dispatch check: can a non-Claude-Code request still reach the subscription route? Can a genuine Claude Code request be falsely refused? Is the refusal fail-closed without ledger side effects?

- **Reachability.** Every upstream dispatch goes through `hosted_attempt` (the only `transport.send` site, L1198), including failover re-dispatch (L1012-1019), and the check runs on `fetch.body` — the same bytes that are sent (L1032-1044). So no body lacking the two blocks is dispatched on `https:anthropic-oauth`. **But the check is a text-shape test on container-authored bytes**: any body carrying `system[0].text` starting `x-anthropic-billing-header: ` with `cc_entrypoint=sdk-ts;` and `system[1].text == "You are a Claude agent, built on Anthropic's Claude Agent SDK."` passes. In particular the extension's **pi prompt mode** — the one shape the receiver comment cites as having drawn extra usage — produces exactly the same first two blocks (probe `run1/0005.req.body`: the `PI_CLAUDE_SDK_PROMPT=pi` run; third block is Pi's default preamble). The check therefore cannot distinguish prompt modes; the controls that do are the receiver's `delete process.env.PI_CLAUDE_SDK_PROMPT` (`pi-coding.mjs` L205), the pi-config allowlist (`pi_coding_provider.rs` L588-603: only `settings.json`/`models.json`, so no `/config/claude-sdk.json` or `SYSTEM.md`; the project `.pi/claude-sdk.json` cannot set `promptMode`, `config.ts` L33-38), and the response-side billing guard. See **P2-1**.
- **False refusal.** All six live bodies (c1 1–3, c2 1–3) and all six probe bodies contain both strings; c2 bodies 1–2 verified positionally (`system[0]`, `system[1]`, cache_control on [1] is ignored by the check). The identity line is a Claude Code 2.1.325 constant and the binary is pinned; a version change would fail closed, not open. A non-array `system` or <2 blocks → `block(i)` is `None` → refuse. No false-refusal path found for the pinned binary.
- **Fail-closed, ledger.** Order in `hosted_attempt`: `router.next` (router-level reservation) → check → `router.settle(routed, NotDispatched)` releases it (`pi_route.rs` L220-226) → `provider_failed = true` → `save` → `response-unavailable`. `reserve_attempt` is never reached, so no attempt record and no attempt reservation (test L3184). `provider_failed` blocks further grants (`pi_coding_provider.rs` L842) and completion (`pi_coding_analysis.rs` L361) → job fails. **Caveat:** the *grant-time* unknown reservation (`provider_grant`, L854-871) remains on the request with no attempt; `Reservation::charged_microusd` then reports the full 1 320 001 and `unsettled()` is true (L210-234). So "neither reserved nor sent" holds at attempt level only — see **P2-2**.

### 4. Record: are the live-c2 claims supported by the run files?

**Yes, with two small precision notes.**

| Claim (README L388-399) | Run-file evidence |
|---|---|
| exit 0, `CompletedSettled`, 23 s | `summary.json` `exit: 0`, `elapsed_s: 23.2`, `result.settlement: CompletedSettled` |
| 3 requests, HTTP 200, `unified-status: allowed`, `overage-status: rejected` | `coding-hosted-{1,2,3}-1.relay.json` `status: 200`, `headHeaders` carry both |
| settled 351 / 703 / 12 360 | `summary.json` ledger; recomputed above from relay usage at catalog rates |
| all writes one-hour; req 3: 58 910 written, 2 716 read, 1 098 output | `message_start.cache_creation.ephemeral_1h_input_tokens` = total in all three streams; relay 3 usage matches |
| 13 Host deliveries (list, then 12 reads in one response) | `summary.json` `snapshot_deliveries: 13`; session L8 shows 12 `snapshot_read` calls in one assistant message |
| `forwarderRefused: 0` | `0002-rpc-0.stdout` L294 |
| receipt names `claude-sdk` / `claude-haiku-5-5` | `analysis-host-result.json` L403, L465, L513; `component_passed` L722 |
| answer: one line per member | `summary.json` `result.answer` (12 bullet lines) |
| no access/refresh token in run files; `private-access` empty | grep for `sk-ant-o[ar]t01-[^D]`, `sk-ant-api`, `refresh_token` over the whole `live-c2` tree: none; `state/private-access/` empty. File count by listing is **92** (incl. `private/hosted-credentials.json`), not 94 |
| every body passes the pre-dispatch check | bodies 1–2 positionally; body 3 by substring presence only (P3-2). `inputs/pi-coding.mjs` carries the `delete PI_CLAUDE_SDK_PROMPT` line, so the run used the post-`ab4a22db` receiver |
| offline `w4c/ok` passes | `w4c/ok/summary.json` `exit: 0`, `completed: true`, settled 675 |

---

## Findings

### P2-1 — The pre-dispatch check does not establish what the comment, record and test name say
- **Where:** `pi_coding_hosted.rs` L1027-1031 ("Subscription routes carry only Claude Code's own requests"), L452-455; `records/dev-vertical-h2-route-20261117/README.md` L345-354; test case `pi_prompt` L3142-3143; `pi-coding.mjs` L203-204.
- **Evidence:** probe `run1/0005.req.body` (Pi's default prompt, `PI_CLAUDE_SDK_PROMPT=pi`) has `system[0]` = `x-anthropic-billing-header: cc_version=2.1.325.fba; cc_entrypoint=sdk-ts;` and `system[1]` = the Agent SDK identity line — identical to the Claude-prompt bodies. The test's `pi_prompt` body (one block, "You are an expert coding assistant.") is a shape the extension never produces. The check is a plain-text shape test on bytes the (untrusted) container authors.
- **Consequence:** the record reads as if the check enforces the operator constraint; it only excludes bodies that do not imitate Claude Code. The actual prompt-mode control is the env delete plus the pi-config allowlist; the actual billing control is the response guard (both correct and in place).
- **Smallest fix:** rename the test case (e.g. `no_claude_code_blocks`) and add one case asserting that a pi-prompt-mode body *passes* (documenting the limit); reword the Host comment and README to "a body shaped like Claude Code's; it does not distinguish prompt modes and is not an identity proof — the prompt mode is held by the env delete and the pi-config allowlist, billing by the response guard".

### P2-2 — The new refusal leaves no durable reason, and the grant-time reservation stays held for a request known not dispatched
- **Where:** `pi_coding_hosted.rs` L1035-1043 (no attempt record → no `reason`); `hosted_unavailable` L1278-1284 (frame only in `hosted.out`, transient); `pi-native-tools.mjs` L48 throws `route_unavailable_subscription_request_not_claude_code`, which `pi-coding.mjs` L147 `catch{}` swallows into a 502 "host relay failed"; `pi_coding_provider.rs` L210-234 (`charged_microusd` = full `reserved_microusd` and `unsettled() == true` when `attempts` is empty).
- **Evidence:** after the refusal the request's `Reservation` exists from the grant (L866-871) with `reserved_microusd: 1 320 001`, no attempt, no reason; completion is refused generically via `provider_failed`. Nothing retained in `state/` names `subscription_request_not_claude_code`. (The same shape pre-exists for `ledger_refused`; this diff adds the first operator-decision refusal that an auditor will need to find.)
- **Consequence:** a job that fails on this check has a receipt showing a request holding the full unknown reservation and no reason — contradicting "neither reserved nor sent" (README L349) at the request level and leaving the operator-decision outcome without provenance (core §1.1 "truthful preservation of unresolved state", §16 auditability).
- **Smallest fix:** in the new branch, before `hosted_unavailable`, record the code on the request (e.g. a `reason: Option<String>` on `Reservation`, set here and for `ledger_refused`), and — since no byte was written — settle the request as known-not-dispatched (`settled_microusd = Some(0)`) so `charged_microusd` reports 0; in the receiver, pass `e.message` into `forwardRefuse` so Pi's recorded error carries the code. Add the assertion to `subscription_routes_dispatch_only_claude_code_requests`.

### P3-1 — Exact thinking comparison is unit-tested but not yet exercised live inside a prefix
- In both live runs the first thinking block appears in the last assistant message of request 3's body (the extension part), so no live request has compared a thinking block within the common prefix. Claude Code places `cache_control` on system/user-side blocks (live bodies), so a false refusal is not expected; note it as a known-untested condition for the first ≥4-request job.

### P3-2 — Two record precision notes
- README L397 "94 run files": my listing of `live-c2` is 92 files. README L399 "every request body passes": body 3 verified by substring presence only here. Neither changes a conclusion.

### P3-3 — Test helper label
- `tests/pi_analysis.rs` L3184 asserts `attempts.is_empty()` under the message "nothing reserved"; per P2-2 the request-level reservation is present. Rename to "no attempt reserved" or extend per P2-2.

---

## Correct (verified)
- Closed grammar: exact-3-key `thinking()`; `text()` shared and unchanged; coding grammar untouched; no Codex/Responses or settlement change.
- `answer_text` and `check_answer_bound` both exclude thinking; thinking-only final refuses at settlement.
- Canonical rules unchanged for non-thinking blocks; thinking compared exactly on both sides; string-content rule unaffected; fail-closed on length mismatch.
- Pre-dispatch check sits on the single dispatch path, checks the exact bytes sent, releases the router reservation, creates no attempt, sets `provider_failed`, and the response guard remains as second layer.
- Receiver env delete is effective given the Host's pi-config allowlist and the extension's project-config restrictions.
- All live-c2 figures (status, headers, settlement arithmetic, deliveries, counts, receipt identity, secrets scan, `private-access`) are supported by the run files.