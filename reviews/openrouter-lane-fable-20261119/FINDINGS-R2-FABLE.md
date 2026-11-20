I have all the evidence I need; nothing further to request. Below is the complete report artifact (returned here rather than written, per the no-edit rule and the task's instruction that the parent handles the file).

---

# OpenRouter/DeepSeek Pi Host lane — Fable completion check (R2)

- **Subject:** branch `openrouter-lane-20261119`, frozen commit `fa0ac42c544f1c5c2e0f14df8cd0188dd067d48f` (worktree HEAD; remediation range `31903c4b..fa0ac42c`, 8 files, +513/−30; six source files in the lane overall `df5ec90f..HEAD`).
- **Authority:** `planning/openrouter-lane-deepseek-20261119.md` Revisions 1–9; prior findings `reviews/openrouter-lane-fable-20261119/FINDINGS.md`. The untracked `FINDINGS-R2.md` (different reviewer) and the Revision 9 self-review were **not** used as evidence.
- **Scope:** foreground-only. Background execution is deferred and must be refused; verified refused at admission (`background_gate`, `pi_coding_provider.rs:750`). Receiver background paths are unreachable on a valid admission and were not assessed as working.
- **Method:** read the remediation diff and every seam it touches; ran the full crate suite and clippy in an out-of-tree target dir; ran `node --check` on the receiver; drove the pinned Pi 1.5.0 `openai-completions` client **offline** (fetch stub, `env -i`, fake marker key, no network) to capture the exact body/headers Pi emits for the pinned catalog row. No live inference, Docker, credential access, E1a run, checkout edit, git mutation, or subdelegation.

## Verdict (foreground submission)

**OK with notes — accept for the foreground-only scope.** No P0/P1. Every prior P1 is disposed and proven by a running test and/or by the offline real-body capture; P2-1 and P2-2 are disposed; P2-3 is disposed for the Host core (end-to-end settle test) but the loopback-mock inconsistency it named remains (P2 below). Admission/runtime gates, secret custody, three-layer identity, emitted request shape, stream terminal/error handling, accounting and cleanup seams are consistent with the plan. The lane remains **not** live-authorized by this review beyond what the plan's own sequence allows (live qualification is an operator decision; background stays refused).

## Commands run (verified by running)

| Command | Result |
| --- | --- |
| `CARGO_TARGET_DIR=/tmp/wt-openrouter-review-target cargo test -p work-engine-manual-host --lib` | `130 passed; 0 failed; 6 ignored` |
| `… cargo test -p work-engine-manual-host` (all targets) | lib 130/0; `lifecycle` 50/0; `pi_native_bridge` 4/0; `pi_native_memory` 2+1/0; `pi_native_result` 2/0; `pi_native_transport` 4/0; `pi_route` 8/0; `pi_stream` 26/0; `pi_coding_actual` 1 ignored |
| `… cargo test --lib --no-run` (never-used scan) | no `never used` warning in `work-engine-manual-host` (only `review-execution-evidence/src/pi/grammar.rs:17`, unrelated) |
| `… cargo clippy -p work-engine-manual-host --lib --tests` | 35 warnings crate-wide; in lane files only `pi_coding_hosted.rs:537` (blame `1445a648`, 2026-11-16, pre-lane) and `pi_coding.rs:2284` (`drop(profile)`, blame `ab79b475`, lane commit before `31903c4b`); `tests/pi_stream.rs:310` pre-lane. The remediation introduced none. |
| `grep -n -B2 'fn messages_body_policy_and_url_are_enforced'` + double-attribute scan | `#[test]` at `:3219`; URL test `#[test]` at `:3205`; no doubled `#[test]` in any lane file |
| `node --check receiver/pi-coding.mjs` | `SYNTAX_OK` |
| Offline capture, `streamSimple(model=openrouter.json row, reasoning:'high', apiKey:'nonsecret-marker', fetch:stub)` against the pinned Pi 1.5.0 tree | URL `https://openrouter.ai/api/v1/chat/completions`, `POST`; body keys exactly `model, messages, stream, stream_options, store, max_completion_tokens, tools, reasoning`; `model:"~deepseek/deepseek-v4-flash-latest"`, `stream_options:{include_usage:true}`, `store:false`, `max_completion_tokens:943718`, `reasoning:{effort:"high"}`, `tools:[{type:"function",function:{name,description,parameters,strict:false}}]`; headers `authorization: Bearer nonsecret-marker`, `accept`, `content-type`, `user-agent`, `x-stainless-*`. With `reasoning:'off'` the body carries `reasoning:{effort:"none"}`. |
| `git diff df5ec90f..HEAD -- src/pi_docker.rs src/pi_native.rs` | empty (Docker/cleanup custody untouched by the lane) |
| Board read (`codex_chatboard.py … read --limit 20 --active-claims`) | claim `review:openrouter-lane-fa0ac42c-fable` held by the launching session; no conflicting claim |

## Prior-finding dispositions (each checked against code and tests)

| Finding | Disposition at `fa0ac42c` | Evidence |
| --- | --- | --- |
| **P1-1** Completions terminal never recorded | **Fixed.** Terminal for `Wire::Completions` is the last JSON chunk carrying a `usage` object; `backend_model`/`raw_terminal{eventType:"chat_completion",…}` recorded. | `pi_coding_hosted.rs:1508-1527, 1573-1580` (read); `completions_lane_settles_end_to_end_with_high_effort` (`:3087`) asserts `backend_model == "deepseek/deepseek-v4-flash-0731"`, `raw_terminal.eventType == "chat_completion"`, `settled_microusd == Some(4)` (run). `provider_complete` family gate `pi_coding_provider.rs:1171-1183`, raw arm `:1258-1280`. |
| **P1-2** no API-key marker | **Fixed.** `await modelRuntime.setRuntimeApiKey('openrouter','nonsecret-marker')` (`receiver/pi-coding.mjs:257`); method exists on Pi 1.5.0 `ModelRuntime` (`model-runtime.js:424`). The marker arrives as `authorization` (capture) and is stripped by `ACCESS_HEADERS` (`pi_coding_hosted.rs:63-69`); the e2e test asserts it never reaches the transport (run). Real key applied only in `HttpsTransport::send` (`pi_coding_https.rs:387-389`). |
| **P1-3** body policy vs real body; `high` not enforced | **Fixed.** `COMPLETIONS_KEYS` now `max_completion_tokens`+`store` (`pi_coding_hosted.rs:92-104`); `store` must be exactly `false` (`:952`); `max_completion_tokens` optional, `1..=catalog maxTokens` (`:955-961`); `reasoning.effort == "high"` required (`:983-988`); receiver `thinkingLevel:'high'` (`:249`) flows via `baseOptions` (`:343`) → `sdk.js` clamp → `agent.js:310 reasoning` → `streamSimple` clamp → `buildParams` openrouter branch. **Capture confirms** the real body is admitted key-for-key (943718 == bound, not above it) and that `off` would yield `"none"`, which `completions_policy_refuses_effort_below_high` (`:3137`) refuses (run). Real tool shape `{type:"function",function:{…}}` passes the `type=="function"` check (`:967-971`). |
| **P1-4** shell guard regressed fixture bridges | **Fixed.** Guard is `is_live_bridge(schema) && !live_shell` (`pi_coding.rs:1424`); fixture test `:2205` reaches the guard (schema/session checks at `coding_advance_lines` pass for `PROFILE`/`"native"`) and passes (run). See P3 on test strength. |
| **P1-5** Messages policy test de-attributed | **Fixed.** `#[test]` restored (`:3219`), duplicate removed (`:3205` single); test runs in the 130. |
| **P2-1** provider cost/cached tokens not in receipts | **Fixed.** `completions_usage` carries `providerReportedCost`/`providerCachedInputTokens` (`pi_stream.rs:541-558`); raw terminal records them under `provider_reported` (`pi_coding_hosted.rs:1577-1579`); settlement still prices from the pinned row (`raw_cost`, `pi_coding_provider.rs:872`); comment at `:568-573` corrected. |
| **P2-2** loose family prefix | **Fixed.** Suffix after `family-` must start with an ASCII digit (`pi_coding_provider.rs:801-811`); `-vision-exp` rejected, `-0731` and the bare family accepted (test `:1722-1735`, run). See P2-B on the alias echo. |
| **P2-3** no end-to-end fixture; loopback `target` | **Partly fixed.** Host-level e2e now exists via the scripted `FakeTransport` (`openrouter_host`/`openrouter_grant`/`openrouter_fetch`/`completions_completed`, `:2884-2972`) and runs. `(Loopback, Completions) => "/unsupported-completions"` (`:251`) while `serves(Loopback, _) == true` (`:224`) is unchanged → P2-A. |
| P3 `stop_ok` redundant arm | Fixed (`pi_coding_provider.rs:1184-1187`). |
| P3 `sk-or-` prefix | Fixed at load (`openrouter_key`, `pi_coding_https.rs:172-180`; test `:455`, run). |
| P3 `completions_forward_body` silent fallback | Unchanged; unreachable in `hosted_fetch` (bytes already parsed at `:774`). Carry-over, report-only. |
| P3 comment "checked by `provider_observation`" / strict equality in `provider_observation` | **Unchanged.** Comment still at `pi_coding_hosted.rs:1586`; `provider_observation` still refuses `name != plan.model` (`pi_coding_provider.rs:1111`). Unreachable on hosted lanes (receiver emits `provider_observation` only in the fixture path, `pi-coding.mjs:489`). Carry-over. |
| P3 `x-requested-with` allowlisted | Harmless; Pi does not send it (capture). |

## Findings (severity-ranked)

**P2-A — `Endpoint` admits a loopback route for the Completions wire but dispatches it to `/unsupported-completions`.**
`pi_coding_hosted.rs:224` (`serves(Loopback) => true`, used by `ProviderPlan::validate` at `pi_coding_provider.rs:700-706`) vs `:251`. Consequence: a `loopback:<port>` route on a Completions plan passes validation, then every attempt is sent to a path no mock serves, so the provider-disabled Docker proof pattern used by the other lanes cannot exercise this lane end-to-end; the first real exercise of receiver+Host+upstream is live OpenRouter. Smallest fix: `(_, Wire::Completions) => "/chat/completions"` for `Loopback` (or make `serves(Loopback, Completions)` false so validate and target agree). Not a blocker for the foreground submission; it is a qualification-coverage gap the plan should carry into live qualification knowingly.

**P2-B (conditional, fail-closed) — the family predicate rejects the alias slug itself as Layer C.**
`completions_model_in_family("deepseek/deepseek-v4-flash-latest", "deepseek/deepseek-v4-flash")` is `false` (`pi_coding_provider.rs:801-811`: rest `"latest"` does not start with a digit). If OpenRouter's streamed `model` echoes the alias rather than a dated checkpoint, `provider_complete` refuses every turn with `"provider result unknown/error; reservation retained"` (`:1171-1190`). I cannot verify the echo offline; Revision 2 records an observed `deepseek-v4-flash-0731` echo, which is why this is P2 rather than P1. Smallest fix without loosening toward other catalog models: additionally accept the plan's own base id (Layer B minus `:floor`, i.e. `transmitted_model()` with the suffix trimmed) as in-family, and make live qualification assert which form OpenRouter actually returns. Fail-closed either way; cost on failure is one retained reservation.

**P3 (report-only)**
- `pi_coding.rs:2205-2218`: the P1-4 regression test accepts `Ok(_)` and any `Err` other than the guard message, so it would also pass vacuously if an earlier check refused. Reading shows it does reach the guard; pinning the expected downstream refusal would make that explicit.
- `pi_coding_hosted.rs:88-91`: the `COMPLETIONS_KEYS` doc comment still says `store` is refused; it is now admitted-when-`false` (`:952`).
- `pi_coding_hosted.rs:1586` / `pi_coding_provider.rs:1111`: stale comment and unreachable strict `provider_observation` equality (carry-over above).
- `pi_coding.rs:2284` `drop(profile)` clippy warning (test-only, lane commit `ab79b475`). The plan's "no new clippy warnings in the lane files" holds for the remediation range, not the lane as a whole.
- Plan Revision 8 cites "pi_route 26"; the counts are `pi_route` 8 and `pi_stream` 26 (cosmetic).

## What is correct (verified)

- **Admission/runtime gates:** `validate()` order capability matrix → analysis/relay exclusion → `background_gate` (`pi_coding_provider.rs:731-735`), each with passing tests; Admission and plan must agree on both flags (`:721-722`); `allow_background=true` refused regardless of profile (run). Shell guard reads Host-held authority, scoped to live bridges (`pi_coding.rs:1424`); ready-frame tool set bound to `allow_shell`/`allow_background` (`ready_tools`, `:121-149`). Receiver exposes `workspace_exec` only when `!live || allowShell` (`pi-coding.mjs:335-345`) and loads `BG_EXTENSION` only on `bgPlugin` (`:465`), which the Host never admits.
- **Secret custody:** key loaded only via `openrouter_key_from_{env,file}` with `sk-or-` + `valid_token`; `identity()` records source kind only (`pi_coding_https.rs:230-231`); applied only in `send` (`:387-389`); screened from relayed bytes by `hold` (`:397`; `openrouter_routes_need_their_openrouter_access_and_screen_it`, run); container `authorization` stripped (`ACCESS_HEADERS`); non-allowlisted headers (`x-stainless-*`, `x-session-affinity`) dropped at `request_head` (`:367-389`).
- **Identity A/B/C:** A pinned by `selected()` (`pi_coding_provider.rs:530-558`) and the receiver's `getModel('openrouter', …)` check (`:261-262`); container body must carry A (`pi_coding_hosted.rs:784-788`, capture confirms Pi sends the `~` alias); B derived by `transmitted_model()` (`:508-524`), required by `OPENROUTER_MODELS` (`:16`), rewritten at dispatch (`:819`) and retained create-only as `coding-hosted-{n}.body.json` (`:684`; e2e asserts `model == …:floor`); C recorded from the usage chunk and family-checked only at `provider_complete`.
- **Request shape/URL:** `{base}/chat/completions` (`:734`) equals the captured URL; upstream path fixed `openrouter.ai` `/api/v1/chat/completions` (`:249`).
- **Stream terminal/errors:** `[DONE]` must be the last block, tail comments/blank only (`pi_stream.rs:415-456`; `: OPENROUTER PROCESSING` comment lines are tolerated by `sse_data`/tail rule); error chunk → failure, never completion; missing usage → `completed:true, observed:false` → `provider_complete` refuses with reservation retained (`pi_coding_provider.rs:1262-1280`); non-200 or non-`stop|toolUse` refused (`:1184-1190`); `from_relay_for` requires clean terminal + no held/dropped bytes (`:222-245`).
- **Accounting:** full-catalog reservation (`full_reservation`), `max_requests × reservation ≤ budget` (`:594-602`), ceilings 4 M / 20 M µUSD admit 6×344,982 (pinned 1.5.0 rates) or 6×545,259 (plan rates); settlement at pinned rates, dearest input-side rate, +1 µUSD (e2e: 12×0.07+4×0.5 → ceil 3 +1 = 4, run); provider-reported cost labelled non-authoritative.
- **Cleanup:** no lane change to `pi_docker.rs`/`pi_native.rs`; refusal/uncertain/retention paths are the shared ledger, unchanged by this diff.

## Validation limits

- No Docker, no container run of the receiver, no live OpenRouter traffic. The receiver was checked by `node --check`, by reading, and by driving pi-ai's `streamSimple` offline against the pinned 1.5.0 tree (same code path `buildParams`/`createClient`, not the receiver process itself).
- The Host e2e test uses a scripted `FakeTransport`; `HttpsTransport`'s TLS/HTTP path to `openrouter.ai` is covered only by unit tests (credential load, screening, head construction).
- OpenRouter's real stream (model echo, usage fields, keep-alive comments, non-200 bodies) was not observed; P2-B depends on it.
- Background capability: refused at admission (verified); its receiver code is unassessed and must remain out of scope until the workspace-bounded process host exists (Revision 6/7).
- Worktree carries untracked `rust/remediation-lib-test.log` and `FINDINGS-R2.md`; neither is in the target commit and neither was used.