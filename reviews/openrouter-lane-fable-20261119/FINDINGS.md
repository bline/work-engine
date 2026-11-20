# OpenRouter/DeepSeek Pi Host lane — independent code review (Fable)

- Target: branch `openrouter-lane-20261119`, frozen commit `31903c4b` (worktree HEAD), full lane `df5ec90f..31903c4b` (7 files, +1571/-43).
- Authority: `planning/openrouter-lane-deepseek-20261119.md` (Revisions 1–8). The author's self-review (Revision 8) was not used as evidence.
- Scope: foreground-only lane. **Background execution is declared OUT OF SCOPE** (deferred by Revision 6/7; `background_gate` refuses any `allow_background=true` admission at validation). The receiver's background-plugin code paths (`pi-coding.mjs:29,60-70,330,455`) are unreachable on a valid admission and were not assessed as working.
- Method: offline reading of the diff and the unchanged seams it depends on; `cargo test -p work-engine-manual-host --lib` run with `CARGO_TARGET_DIR=/tmp/wt-openrouter-review-target`; the pinned Pi 1.5.0 runtime copy at `/home/bline/.local/state/work-engine/s10-supervision-20261103/pi-docker-base/builder/pi/node_modules/@earendil-works/` (the receiver self-identifies as `producer:'pi-1.5.0'`) read to check the receiver's assumptions. No live inference, no Docker, no provider calls.

## Commands run (verified by running)

| Command | Result |
| --- | --- |
| `cargo test -p work-engine-manual-host --lib` | `126 passed; 0 failed; 6 ignored` (matches the claimed lib total) |
| `cargo test -p work-engine-manual-host --lib messages_body_policy_and_url_are_enforced` | `0 passed … 132 filtered out`; compiler warning `function messages_body_policy_and_url_are_enforced is never used` |
| `git show df5ec90f:…/pi_coding_hosted.rs \| grep -B1 "fn messages_body_policy_and_url_are_enforced"` | base had `#[test]` on line 2849; HEAD line 3013 is blank (attribute gone) |
| `grep -rn "Completions\|openrouter\|OpenRouter" rust/crates/work-engine-manual-host/tests` | no matches (no integration fixture for the lane) |
| `#[test]` count, 5 lane source files, base vs HEAD | 63 → 80 (+17 new tests, 1 existing test de-attributed, 1 duplicate attribute) |

---

## Findings

### P1-1 — Host: Completions settlement never records the raw terminal or backend model, so every Completions request is refused at `provider_complete`

**Location.** `src/pi_coding_hosted.rs:1492-1495` (terminal lookup), `:1540-1558` (Completions arm), `src/pi_coding_provider.rs:1162-1183` (`backend_ok`), `:1252-1276` (Completions raw-usage arm), `src/pi_stream.rs:392` (`Wire::Completions => "[DONE]"`).

**Evidence (by reading).** The settle path finds the terminal event as
```rust
let terminal = stream::sse_data(&relay.forwarded).iter()
    .filter_map(|d| serde_json::from_str::<Value>(d).ok())
    .find(|v| v["type"] == wire.terminal_event());   // :1492-1495
```
For `Wire::Completions`, `terminal_event()` is the literal `"[DONE]"` (`pi_stream.rs:392`). `data: [DONE]` is not JSON and is dropped by `filter_map`; no `chat.completion.chunk` carries a `type` field (they carry `object`). So `terminal` is always `None`, the `if let Some(v) = terminal…` block (`:1524`) is never entered, and the `Wire::Completions => (…)` arm at `:1540-1544` is dead code: `r.backend_model` and `r.raw_terminal` stay `None`.

At completion, `pi_coding_provider.rs:1162-1170` requires `r.backend_model.as_deref().is_some_and(|resolved| completions_model_in_family(..))` → `false` → `state.provider_failed = true` and `refuse("provider result unknown/error; reservation retained")`. Independently, the Completions raw-usage arm (`:1252-1276`) filters `r.raw_terminal` by `eventType == "chat_completion"`, which is likewise never set.

**Counterexample.** Feed the `clean()` stream from `pi_stream::completions_tests` (deltas, usage chunk, `data: [DONE]`) through the hosted settle: `usage_for` reports `completed:true, observed:true, model:"deepseek/deepseek-v4-flash-0731"`, but `terminal == None`, so `backend_model == None` and the subsequent `provider_complete` is refused. The router is settled `Completed` (`:1562-1572`) while the Host-held reservation is retained as failed — the lane cannot settle a single successful request.

**Why tests missed it.** `completions_usage`/`terminal_is_last_for` are unit-tested in isolation; nothing drives a Completions stream through `hosted_settle` → `provider_complete`. There is no integration fixture for the lane at all (see P2-3).

**Smallest fix.** Select the terminal per wire, e.g. for Completions take the last JSON chunk carrying a `usage` object (the same chunk `completions_usage` uses), or branch: `let terminal = if wire == Wire::Completions { events.iter().rev().find(|v| v["usage"].is_object()).cloned() } else { …type == terminal_event()… }`. Add a hosted settle → `provider_complete` test over the `clean()` stream asserting `backend_model == Some("deepseek/deepseek-v4-flash-0731")` and `settled_microusd.is_some()`.

---

### P1-2 — Receiver: the `openrouter` lane has no API-key marker, so Pi refuses to issue the request before the Host-served `fetch` is reached

**Location.** `receiver/pi-coding.mjs:244-253` (openrouter branch), `:377` (stream override passes no `apiKey`/`authorization`), versus `:256-259` (openai-codex marker) and `:269` (fixture `setRuntimeApiKey`).

**Evidence (by reading the pinned Pi 1.5.0 runtime).**
- `pi-coding-agent/dist/core/model-runtime.js:453-462` `prepareRequest`: `const resolution = await this.getAuth(model, {apiKey: options?.apiKey, …}); if (!resolution) throw new ModelsError("auth", "Provider is not configured: openrouter")`. The receiver's runtime is created with `credentials:{read:async()=>undefined,list:async()=>[]…}` (`:247`), never calls `setRuntimeApiKey`, and the container must not carry `OPENROUTER_API_KEY` (pi-ai `env-api-keys.js:93`) under the custody contract. So `getAuth` resolves nothing.
- `pi-ai/dist/api/openai-completions.js:35-41` `getClientApiKey`: throws `No API key for provider: openrouter` unless `options.apiKey` or an `authorization` header is present; it runs before `createClient(...fetch...)` (`:182-187`).
- `pi-coding-agent/dist/core/agent-session.js:2027-2029` `setModel` throws `No API key for openrouter/…` when `checkAuth` fails.

The OpenAI lane solves exactly this with a nonsecret marker credential (`:256-259`, "Nonauthenticating marker only; the Host holds and applies real access"), and the fixture lane with `setRuntimeApiKey(…,'FAKE-FIXTURE-NOT-A-CREDENTIAL')` (`:269`). The openrouter branch has neither. Result: `original(requestModel, …)` rejects before the `fetch` override emits `provider_fetch`; every turn fails as `outside host admission unavailable`/error. Fail-closed (no custody impact), but the lane is unreachable.

**Smallest fix.** In the openrouter branch: `await modelRuntime.setRuntimeApiKey('openrouter','nonsecret-marker');` (or supply a marker via the credential store's `read`). This is safe: the Host strips the container's `authorization` header (`ACCESS_HEADERS`, `pi_coding_hosted.rs:63-69`) and applies the real `Authorization: Bearer <key>` itself (`pi_coding_https.rs:368-370`).

---

### P1-3 — Host body policy vs the body Pi actually serializes: `completions_policy` refuses the real Pi 1.5.0 openrouter body, and does not enforce the operator's `high` effort

**Location.** `src/pi_coding_hosted.rs:92-104` (`COMPLETIONS_KEYS`), `:926-974` (`completions_policy`: `max_tokens required` at `:946`, `reasoning required (high effort)` at `:971`), `receiver/pi-coding.mjs:229` (`providerOptions={thinkingLevel:'off'}`, never overridden for openrouter).

**Evidence (by reading pi-ai 1.5.0 `openai-completions.js` and the pinned `openrouter.json` entry).**
- `getCompat` (`:1305-1340`) merges the catalog `compat` over `detectCompat`. The `chat:~deepseek/deepseek-v4-flash-latest` entry's `compat` sets only `supportsDeveloperRole, thinkingFormat:"openrouter", supportsStrictMode, sendSessionAffinityHeaders, requiresReasoningContentOnAssistantMessages`; `supportsStore` and `maxTokensField` come from detection.
- `detectCompat` (`:1220-1264`): `openrouter` is not in `isNonStandard`, so `supportsStore: true` and `maxTokensField: "max_completion_tokens"`.
- `buildParams` (`:568-581`): `if (compat.supportsStore) params.store = false;` → the real body carries top-level `store`, which is **not** in `COMPLETIONS_KEYS` → `refuse("provider fetch body policy: field store is not allowed")`. (The author's own comment at `:88-91` says `store` is refused on purpose, but Pi always sends it for openrouter.)
- `max_tokens`: Pi only emits a max-tokens field when `options.maxTokens` is set, and then as `max_completion_tokens` for openrouter. The agent loop passes no `maxTokens` (`pi-agent-core/dist/agent.js:310` builds only `reasoning`); so the body has neither field → `refuse("max_tokens required")`. Even if it were set, the field name would be `max_completion_tokens` (also not allowlisted).
- `reasoning` (`:666-676`): with `thinkingLevel:'off'`, `streamSimple` maps `reasoningEffort` to `undefined` (`:508-509`), and the openrouter branch then sends `reasoning: { effort: thinkingLevelMap.off }` = `{"effort":"none"}`. The Host checks only `!body["reasoning"].is_null()` (`:970-972`), so **effort "none" passes**, contradicting the operator decision "Effort `high`" (plan, Operator decisions table) and Revision 2 P2 ("the `high` effort set in providerOptions is verified in the transmitted body (a test asserts it, not just the plan)"). No such test exists; the receiver never sets `thinkingLevel:'high'` for this lane.

Net effect: the Host policy and the real Pi body disagree on `store`/`max_tokens` (every request refused), and agree on the wrong thing for `reasoning` (effort `none` accepted). Revision 2 also required "a test records the actual Pi-generated request body to confirm the required fields reach OpenRouter" — not present.

**Smallest fix.** (a) Receiver: set `thinkingLevel:'high'` for the openrouter lane. (b) Host: require `body["reasoning"]["effort"] == "high"`; admit `store` only with value `false`; either admit `max_completion_tokens` (pinning it ≤ catalog `maxTokens`) or have the receiver pass `maxTokens` and keep the field name Pi uses. (c) Add the plan-required test: a captured real Pi 1.5.0 body (the fixture receiver can produce one offline) passed through `completions_policy`, asserting acceptance and `reasoning.effort == "high"`.

---

### P1-4 — Host: the new `tool_request` guard refuses every shell call on the provider-disabled fixture bridges (regression), contradicting `ready_tools`

**Location.** `src/pi_coding.rs:1398-1422` (guard: `provider.as_ref().map(|p| p.allow_shell).unwrap_or(false)` → `refuse("shell execution not admitted")`), `:121-149` (`ready_tools`: fixture surfaces always advertise `workspace_exec`), `:878-879` (fixture bridges require `provider.is_none()`, live bridges require `provider.is_some()`), `receiver/pi-coding.mjs:325` (`execCapable=!live||allowShell`), `:278` (fixture receiver emits `tool_request`).

**Evidence (by reading).** Profile validation at `:878-879` admits a fixture bridge only with `provider: None`. The guard is unconditional on profile and treats `provider: None` as `allow_shell=false`, so on `PROFILE`/`BRIDGE_PROFILE`/`CLAIMS_BRIDGE_PROFILE` the Host advertises `workspace_exec` in the ready frame (`ready_tools` falls to `json!(["workspace_exec"])`, and the receiver exposes it because `!live`), then refuses the first `tool_request` with "shell execution not admitted". Before this lane the guard did not exist. The author's comments state the opposite intent ("Fixture surfaces … always advertise the workspace shell tool"; receiver: "provider-disabled fixtures (no admission) always expose it").

**Why tests missed it.** The only non-ignored `tool_request` tests are refusal cases (`:2157-2172` session/profile mismatch, `:2204` analysis, `:2263` hosted no-shell). No non-ignored test performs a successful `tool_request`; the provider-disabled Docker proofs (`pi_docker.rs:1861`, `recovery_interruption_reservations_and_final_stream_proof`) are `#[ignore]`.

**Smallest fix.** Scope the guard to live bridges, mirroring `ready_tools`: `if is_live_bridge(schema) && !provider.map(|p| p.allow_shell).unwrap_or(false) { refuse(...) }`. Add a positive test that a `PROFILE` job with `provider: None` journals a `tool_request` intent (the existing fixture harness around `:2157` can be reused with a matching session id).

---

### P1-5 — Test regression: `messages_body_policy_and_url_are_enforced` lost its `#[test]` attribute and no longer runs

**Location.** `src/pi_coding_hosted.rs:2994-3014`.

**Evidence (verified by running).** The inserted test `completions_url_uses_the_catalog_base_not_a_doubled_path` sits between the pre-existing `#[test]` (now at `:2994`) and a second `#[test]` (`:3000`); both attach to the new function, and `messages_body_policy_and_url_are_enforced` (`:3014`) has none. `cargo test --lib messages_body_policy_and_url_are_enforced` → `0 passed … 132 filtered out`; rustc warns `function messages_body_policy_and_url_are_enforced is never used`. At the base commit it was `#[test]` (`df5ec90f:…:2849`). The Messages-lane body/URL refusals (`mcp_servers`, `container`, `service_tier`, …) are therefore unguarded in the reported "126 lib tests".

Also, the replacement test is tautological: it asserts a local closure `accept = |url| url == format!("{base}/chat/completions")`, not `hosted_fetch`'s arm at `:733`.

**Smallest fix.** Restore `#[test]` on `messages_body_policy_and_url_are_enforced`, remove the duplicate attribute, and (optionally) make the URL test call `hosted_fetch` with a fixture plan.

---

### P2-1 — Receipts do not record OpenRouter's provider-reported cost or cached tokens (plan §7 / Revision 2 P2 unmet; code comment is false)

**Location.** `src/pi_stream.rs:513-551` (`completions_usage` extracts only `prompt_tokens`/`completion_tokens`/`total_tokens`/`model`), `src/pi_coding_hosted.rs:1540-1544` (raw terminal carries only those three counts), `src/pi_coding_provider.rs:566-572` (comment: "the recorded receipts split pinned vs provider-reported cost").

**Evidence.** OpenRouter's usage chunk carries `usage.cost` and `prompt_tokens_details.cached_tokens`; neither is captured anywhere in the retained `relay.json`/raw terminal/receipt. Settlement prices every prompt token at the dearest input-side catalog rate (`rates()` `:826-848` takes `max(input, cacheRead, cacheWrite)`), which is conservative (never under-charges the reservation) — acceptable for safety, but the plan's "receipts record pinned-catalog cost and, separately, the provider-reported cost" is not implemented and the `request_reservation` comment claims it is.

**Fix.** Carry `usage.cost` (and `cached_tokens` if present) into `completions_usage` and the raw terminal as labelled, non-authoritative provider-reported values; correct the comment.

---

### P2-2 — Layer C family check is a loose prefix test that admits distinct catalog models

**Location.** `src/pi_coding_provider.rs:772-803` (`completions_model_family`, `completions_model_in_family`).

**Evidence.** `completions_model_in_family(resolved, "deepseek/deepseek-v4-flash")` accepts any `resolved` that `starts_with("deepseek/deepseek-v4-flash-")`. The pinned `openrouter.json` contains `chat:deepseek/deepseek-v4-flash-vision-exp` as a separate model (different price row); it passes the check. The doc comment claims a "name boundary" that excludes other models, which is not what the predicate does. Practical risk is low (OpenRouter resolves `latest:floor` within the alias), but the recorded Layer C would be accepted as in-family for a different catalog model. (Currently unreachable because of P1-1.)

**Fix.** Require the suffix after `family-` to be a version segment (e.g. all digits, `^\d{4}$`) or check the resolved id against the pinned catalog's known checkpoints for the alias.

---

### P2-3 — No offline end-to-end fixture for the Completions lane; the plan's acceptance ("reachable from a Pi Host coding job (offline fixture)") is unmet, and `Endpoint::target` blocks a loopback mock

**Location.** `rust/crates/work-engine-manual-host/tests/` (no Completions/openrouter references); `src/pi_coding_hosted.rs:250` (`(_, Wire::Completions) => "/unsupported-completions"` for any non-OpenRouter endpoint, including `Loopback`, which `serves()` all wires at `:223`).

**Evidence.** Plan Sequence item 1 requires a "mock completions upstream fixture"; Revision 2 requires identity/body/budget tests over the real body. The lane's tests are helper-level only (`completions_forward_body`, `completions_usage`, `capability_ok`, …); `completions_policy` itself has no direct test. This is the root cause of P1-1/P1-2/P1-3 escaping the "223 passing" claim. A loopback fixture would additionally hit the `/unsupported-completions` path.

**Fix.** Map `(Loopback, Completions)` to `/chat/completions` (or `/v1/chat/completions`) and add a hosted lifecycle test: `provider_request` → `hosted_fetch` with a captured Pi body → loopback upstream serving the `clean()` SSE → settle → `provider_complete` settles with `backend_model` in family.

---

### P3 notes (report-only)

- `src/pi_coding_provider.rs:1175-1181`: `stop_ok` has two identical match arms (`(Completions, Some("stop"|"toolUse")) | (_, Some("stop"|"toolUse"))`); the first is redundant.
- `src/pi_coding_hosted.rs:32-42`: `completions_forward_body` silently forwards the original (Layer A) bytes if re-serialization fails. Unreachable in `hosted_fetch` (the same bytes already parsed), but a refusal would make the "Layer B is always what leaves the Host" invariant explicit.
- `src/pi_coding_hosted.rs:1546-1551` comment says the family check is done "by `provider_observation`"; it is done in `provider_complete`. `provider_observation` (`pi_coding_provider.rs:1100-1106`) still enforces strict `backendModel == plan.model`, which would refuse a Completions checkpoint; unreachable today because the hosted receiver never emits `provider_observation` (only the non-hosted fixture does, `pi-coding.mjs:479`), but it contradicts Revision 2's stated disposition.
- `src/pi_coding_hosted.rs:298-301`: `x-requested-with` is allowlisted but Pi's OpenAI client does not send it; harmless.
- `src/pi_coding_https.rs:162-178`: `valid_token` does not require the `sk-or-` prefix the doc comments promise; a mis-provisioned non-OpenRouter token would be sent as Bearer to `openrouter.ai`. Low impact.
- Plan "Confirmed facts" quotes catalog rates 0.07/0.5/0.008; the pi-ai 1.5.0 copy inspected carries 0.059/0.3/0.035 for `~deepseek/deepseek-v4-flash-latest`. Whichever tree the Job pins is what `selected()` reads; the budget ceilings hold under both (6 × 545,259 = 3,271,554 ≤ 4,000,000; 32 × 545,259 = 17,448,288 ≤ 20,000,000; with the 1.5.0 rates 344,982/request).

---

## What is correct (verified)

**Security boundaries / authorization**
- Shell refusal on Host-held authority for live bridges: `pi_coding.rs:1398-1422` reads the journal's plan, not the frame, and refuses before journaling intent or touching Docker; `tool_request_is_refused_when_shell_is_not_admitted` passes (run). Ready-frame tool set is bound to the Host-held `allow_shell`/`allow_background` (`tools_match`, `:1240-1262`), exact-ordered on hosted profiles.
- Capability matrix: `capability_ok` (`(false,true)` refused), `analysis_execution_ok` (analysis/relay admit no execution), `background_gate` (any `allow_background=true` refused) — each with passing tests (run). Admission and plan must agree on both flags (`pi_coding_provider.rs:721-722`); relay job.json forces both false (`:1017-1018`). The `#[serde(default)] = false` migration is the secure default; the plan documents it.
- Credential custody: the OpenRouter key exists only in `HttpsTransport` (`pi_coding_https.rs:243-248`), is applied only in `send` as `Authorization: Bearer` (`:368-370`), is screened from relayed bytes by `hold()` (test `openrouter_routes_need_their_openrouter_access_and_screen_it`, run), and never appears in job.json/journal (`identity()` records only the source kind). The container's `authorization` header is stripped by `ACCESS_HEADERS` (`pi_coding_hosted.rs:63-69`). Route/payer/endpoint binding is checked (`check`, test run). The receiver's `ModelRuntime` has no credential (`pi-coding.mjs:247`).
- No path to a background tool: receiver loads `BG_EXTENSION` only when `allowBackground` (`:29,455`), and the Host refuses such admissions outright; with `allow_shell=false` the receiver exposes no `workspace_exec` tool and the Host refuses `tool_request` regardless.

**Provider routing / three-layer identity**
- Layer A bound by the catalog (`selected()` `:530-558`: id/provider/api/baseUrl must match `openai-completions`/`openrouter`/`https://openrouter.ai/api/v1`), and by the container body (`parsed["model"] == plan.model`, `pi_coding_hosted.rs:783-787`; Pi sends `model: model.id` with the `~` alias, `openai-completions.js:556-558`).
- Layer B pinned by `OPENROUTER_MODELS` (`transmitted_model()` idempotent, tested) and rewritten by `completions_forward_body` before dispatch; the retained `coding-hosted-{n}.body.json` is the forwarded (Layer B) body (`:682-695`). URL rule `{base}/chat/completions` matches Pi's `baseURL: model.baseUrl` + OpenAI SDK path; upstream path is the fixed `/api/v1/chat/completions` on `openrouter.ai` (`:246`, test run).
- Layer C relaxation is limited to the Completions wire (`:1551`), with the family check at completion (`pi_coding_provider.rs:1162-1170`) — correct design, but see P1-1 (unreachable) and P2-2 (loose).

**Stream**
- `terminal_is_last_for(Wire::Completions, …)`: single-line literal `[DONE]` block, tail may contain only comments/blank lines (tests run). `completions_usage`: last chunk with a `usage` object; error chunk (`type:"error"` or `error` object) is a failure, never completion; missing usage → `completed:true, observed:false` → uncertain retention (tests run). `[DONE]` is not JSON and is correctly ignored by `usage_for`'s `filter_map`.

**Accounting**
- Reservation: `full_reservation` = window × dearest input-side rate + max output × output rate (+1 µUSD) from the pinned row; `reservation_microusd` refuses `max_requests × reservation > budget` (`:593-602`). Ceilings are finite and admit the plan's realistic multi-turn/analysis counts (test run; arithmetic above). Budget selection by `(analysis, messages, completions)` is exhaustive (`:633-640`). Per-request grant refuses when `spent + reserve > budget` (`:1058-1066`).
- Refund on refusal / uncertain retention: lane reuses the shared attempt ledger and router (`settle_attempt`, `Disposition::Uncertain`); nothing lane-specific weakens it.

---

## Verdict

**CHANGES REQUIRED** for the foreground-only scope. The security posture of the lane is sound and fail-closed (credential custody, Host-held shell authority, capability matrix, background refused at admission), and nothing reviewed exposes a path to unrestricted Host execution or leaks the OpenRouter key. But the lane is not functional end-to-end: three independent defects each stop every Completions request (P1-1 Host settlement never records the terminal; P1-2 receiver has no API-key marker so Pi never calls `fetch`; P1-3 the Host body policy refuses the body Pi 1.5.0 actually sends, and would accept effort `none` against the operator's `high`), the new shell guard regresses the provider-disabled fixture bridges (P1-4), and a Messages-lane policy test was silently dropped from the suite (P1-5). The plan's own acceptance ("the lane is reachable from a Pi Host coding job (offline fixture)") is unmet, which is why the reported 223-pass total did not detect P1-1..P1-4.

Recommended order: P1-5 and P1-4 (small, mechanical); then P1-1/P1-2/P1-3 together with the end-to-end offline fixture (P2-3) that would have caught them; P2-1/P2-2 before live qualification. No live inference or E1a run until an offline fixture settles one Completions request through `provider_complete` with `reasoning.effort == "high"` in the retained body.
