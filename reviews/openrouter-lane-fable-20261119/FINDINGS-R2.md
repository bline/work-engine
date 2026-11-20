# OpenRouter/DeepSeek Pi Host lane — R2 completion check after remediation (Fable)

- Target: branch `openrouter-lane-20261119`, HEAD `fa0ac42c` (plan Revision 9).
  Post-review remediation range reviewed: `31903c4b..fa0ac42c` (six remediation
  commits + plan Revision 9: c69f4a12, 69c5aabe, dfd95f9a, 0cfb5f9f, 79a92c9c,
  16effd05, fa0ac42c).
- Prior review: `reviews/openrouter-lane-fable-20261119/FINDINGS.md` at
  `31903c4b` (CHANGES REQUIRED). This document checks each P1/P2/P3 finding's
  remediation at current HEAD and the remediation's regressions.
- Scope: foreground-only lane. Background execution is declared OUT OF SCOPE
  (plan Revisions 6/7; `background_gate` refuses any `allow_background=true`
  admission at validation). Not assessed as working.
- Method: offline reading of the remediation diffs and the pinned Pi 1.5.0
  runtime; full crate test run; no live inference, no Docker, no provider calls.

## Commands run (verified by running)

| Command | Result |
| --- | --- |
| `cargo test -p work-engine-manual-host --lib` at HEAD | `130 passed; 0 failed; 6 ignored` |
| `cargo test -p work-engine-manual-host --lib -- --list` | new tests present: `completions_lane_settles_end_to_end_with_high_effort`, `completions_policy_refuses_effort_below_high`, `fixture_bridge_tool_request_is_not_refused_by_shell_capability`, `messages_body_policy_and_url_are_enforced`, `openrouter_keys_require_the_sk_or_prefix` |
| `cargo test -p work-engine-manual-host` (full) | lib 130 ok; lifecycle `49 passed; 1 failed` — the lone failure is `mutation_during_snapshot_collection_is_a_conflict` (10 s snapshot-wait assertion, pre-existing flake, passes in isolation; see P3-R5) |
| `cargo test -p work-engine-manual-host --test pi_stream` | 26 passed |
| `cargo test -p work-engine-manual-host --test pi_route / pi_native_transport / pi_native_bridge / pi_native_memory / pi_native_result` | 8 / 4 / 4 / 3 (1 ignored) / 2 — 0 failures |
| `cargo build -p work-engine-manual-host` | only pre-existing warnings (`grammar.rs strings`, `pi_native.rs PiOwner` — both unchanged at `31903c4b`) |
| `node --check receiver/pi-coding.mjs` | syntax ok |
| Pinned runtime reads (see below) | `RuntimeCredentials.read` returns `{type:"api_key",key}` for `setRuntimeApiKey`; `openai-completions.js:187` passes `options?.fetch` into `createClient`; pinned `openrouter.json` `thinkingLevelMap.high == "high"` |

## Prior-finding dispositions (verified at HEAD)

| Finding | Disposition | Evidence |
| --- | --- | --- |
| P1-1 Completions terminal never recorded | FIXED | `pi_coding_hosted.rs:1517-1527`: Completions terminal is the last chunk with a `usage` object (`[DONE]` is non-JSON and never browsable); `provider_complete` family gate at `pi_coding_provider.rs:1162-1170`. Proven by the end-to-end test (`:3087`): `backend_model == deepseek/deepseek-v4-flash-0731`, attempt Completed, `settled_microusd == Some(4)` (12×0.07 + 4×0.5 = 2.84, ceil + 1 = 4). |
| P1-2 openrouter lane missing API-key marker | FIXED | `receiver/pi-coding.mjs:249,257`: `providerOptions={thinkingLevel:'high'}` and `await modelRuntime.setRuntimeApiKey('openrouter','nonsecret-marker')` before `getModel`. Verified against the pinned 1.5.0 runtime: `ModelRuntime.create` wraps the overlay in `RuntimeCredentials`, whose `read()` serves runtime keys (`{type:"api_key",key}`) back to `models.getAuth` → `resolveProviderAuth` → `getClientApiKey` passes; `openai-completions.js:187` passes `options?.fetch` into `createClient`, so the Host-observed `fetch` override is reached. The marker is never applied upstream: `HttpsTransport.send` injects the real `Authorization: Bearer <key>`; end-to-end test asserts no `Bearer nonsecret-marker` reaches the wire. |
| P1-3 body policy vs real Pi body; effort high | FIXED | `pi_coding_hosted.rs:92-107` allowlist now admits `store` and `max_completion_tokens`; `:952` `store` must be exactly `false` (Pi sends it for openrouter, `buildParams`); `:955-961` `max_completion_tokens` validated only when present, upper-bounded by the catalog `maxTokens` (Pi's agent loop sends none); `:983-986` requires `reasoning.effort == "high"` (effort `none` refused). Receiver sends `thinkingLevel:'high'`; pinned `openrouter.json` maps `high -> "high"` (`thinkingFormat:"openrouter"` writes `reasoning:{effort}`). Two tests: refusal test (`:3137`) and body assertions in the end-to-end test (`forwarded["reasoning"]["effort"]=="high"`, `:3123-3129`). |
| P1-4 shell guard regressed fixture bridges | FIXED | `pi_coding.rs:1413-1425`: guard is now `if is_live_bridge(schema) && !live_shell`; provider-disabled fixtures (`provider: None`, which gate binding at `:878-881` requires) are no longer refused by the capability check. Test `fixture_bridge_tool_request_is_not_refused_by_shell_capability` (`:2205`). Live-hosted no-shell refusal remains (existing test). |
| P1-5 Messages policy test lost `#[test]` | FIXED | `pi_coding_hosted.rs:3219` `#[test]` on `messages_body_policy_and_url_are_enforced` (was lost at `31903c4b`); the URL test above it (`:3205`) has its own single attribute; no duplicate. Test present and green in the run. |
| P2-1 receipts lack provider cost/cached tokens | FIXED | `pi_stream.rs:542-558` extracts `usage.cost` and `prompt_tokens_details.cached_tokens` as `providerReportedCost` / `providerCachedInputTokens`; `pi_coding_hosted.rs:1576-1580` carries them into the raw terminal as `provider_reported` (labelled non-authoritative); misleading `request_reservation` comment corrected (`pi_coding_provider.rs:568-574`); settlement still prices from the pinned catalog (conservative). |
| P2-2 family check loose | FIXED | `pi_coding_provider.rs:801-811`: in-family requires equality with the family or a digit-led version segment after `family-`; `deepseek/deepseek-v4-flash-vision-exp` (a distinct catalog model/price row) is refused — asserted in `admission_tests`. |
| P2-3 no end-to-end fixture | FIXED | `pi_coding_hosted.rs:2893-3041` hosts/grant/fetch helpers + `completions_completed()` clean SSE; `completions_lane_settles_end_to_end_with_high_effort` drives the full lane (policy → Layer B rewrite → dispatch → settle → `provider_complete` → raw-usage settlement) and `completions_policy_refuses_effort_below_high` pins the refusal. The plan's acceptance ("lane reachable from a Pi Host coding job (offline fixture)") is now met. |
| P3 stop_ok duplicate arm | FIXED | `pi_coding_provider.rs:1186`: the redundant `(Completions, Some(stop|toolUse))` arm is gone. |
| P3 sk-or- prefix | FIXED | `pi_coding_https.rs:172-184`: OpenRouter key sources enforce `sk-or-` + `valid_token` at load; test `openrouter_keys_require_the_sk_or_prefix` (`:455`). A non-OpenRouter token can no longer be sent as a Bearer to `openrouter.ai`. |

## Findings at HEAD (report-only; no P1/P2)

### P3-R1 — `completions_model_in_family` predicate is looser than its doc comment
- Location: `src/pi_coding_provider.rs:801-811`.
- Evidence: the doc says "all digits (e.g. `-0731`)", but the predicate only
  checks that the **first** character of the suffix after `family-` is an
  ASCII digit. A resolved id such as `deepseek/deepseek-v4-flash-0731beta` (or
  `-0731-vision`) would be admitted as in-family though it is a non-version
  suffix. Real OpenRouter checkpoints are `-<digits>`, so practical risk is
  negligible; the doc/predicate mismatch is the only defect.
- Smallest fix: require `rest.chars().all(|c| c.is_ascii_digit())`, or keep the
  current behavior and correct the doc.

### P3-R2 — stale comments still claim `store` is refused
- Location: `src/pi_coding_hosted.rs:95-96` (`COMPLETIONS_KEYS` doc) and the
  `completions_policy` doc at `:916-919` ("so no store/background/service-tier").
- Evidence: `store` is now allowlisted (exact `false` required, `:952`). The
  comment "Anything else (e.g. `store`, ...) is refused" is false for `store`.
  Cosmetic only; the enforced predicate is correct (`store != false` refuses,
  so the received value is pinned to false).

### P3-R3 — Completions URL test is still a free-string freeze, and the offline fixture runs the placeholder upstream path
- Location: `src/pi_coding_hosted.rs:3206-3217`; `Endpoint::target`
  `pi_coding_hosted.rs:243-252`.
- Evidence: `completions_url_uses_the_catalog_base_not_a_doubled_path`
  asserts a local closure `|url| url == "{base}/chat/completions"`, not
  `hosted_fetch`'s arm (`:734`). The end-to-end test drives the correct URL
  through the real arm, but the doubled-URL refusal is not exercised at the
  seam. Separately, the end-to-end fixture uses `loopback:` routes
  (`host()` helper, `:2148-2155`), for which `Endpoint::target(Loopback,
  Completions)` still returns `/unsupported-completions` (`:250-251`);
  the production OpenRouter upstream path `/api/v1/chat/completions` is
  therefore exercised only via the static arm + string freeze, not by a live
  loopback hit. Offline-only concern: `Loopback` routes cannot reach
  production dispatch (the Https transport serves `https:` endpoints), so no
  functional hole. (The prior P2-3 fix suggestion to map
  `(Loopback, Completions) => /chat/completions` was not taken.)
- Smallest fix: assert the doubled-URL refusal through `hosted_fetch` with an
  openrouter plan, and/or map Loopback+Completions to the completions path so
  the offline fixture hits the same upstream path the real endpoint serves.

### P3-R4 — P2-1's provider-reported values have no test pinning them end-to-end
- Location: `src/pi_stream.rs:556-558`, `src/pi_coding_hosted.rs:1576-1580`.
- Evidence: `completions_usage_comes_from_the_include_usage_chunk`
  (`pi_stream.rs:1378`) asserts tokens/model but not `providerReportedCost` or
  `providerCachedInputTokens`, and the end-to-end settle test asserts
  `raw_terminal.eventType` only (`pi_coding_hosted.rs:3110`). Implementation
  is correct by reading (the fields mirror the OpenRouter usage chunk), but
  nothing asserts the labelled values survive into the raw terminal / journal.
- Smallest fix: extend the usage unit test and/or the end-to-end test with a
  `usage.cost`/`cached_tokens` chunk and assert `raw_terminal["provider_reported"]`.

### P3-R5 — pre-existing lifecycle flake; plan's "full crate suite green" did not reproduce
- Location: `tests/lifecycle.rs:411` (`mutation_during_snapshot_collection_is_a_conflict`).
- Evidence: in the full run it failed once (`assertion failed:
  started.elapsed() < Duration::from_secs(10)`) while waiting for the
  snapshot file datapoint; it passes in isolation and is unrelated to this
  lane (unchanged test, pre-existing flake noted in Revision 8). Revision 9's
  "full crate suite green, 0 failures" is therefore environment/timing
  sensitive, not a lane regression.

### P3-R6 — `provider_observation` still enforces strict `backendModel == plan.model`
- Location: `src/pi_coding_provider.rs:1096-1123` (handler keeps the strict
  equality), contra the Layer-C relaxation that lives only in
  `provider_complete` (`:1162-1170`).
- Evidence: the hosted receiver never emits `provider_observation`, so the
  strict path is unreachable for the Completions lane (only the non-hosted
  signed fixture does). The Revision-2 three-layer contract says the equality
  is "relaxed only for the Completions family"; the relaxation is instead
  implemented entirely in `provider_complete`. Consistency/documentation only.

## What is correct (verified at HEAD; not re-measured where unchanged)
- Security/authorization boundaries, credential custody (`sk-or-` enforced at
  load, key only in `HttpsTransport.send`, header stripped by `ACCESS_HEADERS`,
  marker never applied upstream), capability matrix + `background_gate`, and
  Host-held shell authority are unchanged from the prior review and remain
  sound and fail-closed.
- Three-layer identity (A `~deepseek/deepseek-v4-flash-latest` in the body and
  `selected()`; B `…-latest:floor` forwarded at dispatch; C resolved checkpoint
  recorded and family-checked at completion) is now coherent end-to-end and
  proven by the end-to-end fixture, which also asserts Layer B (not A) left
  the Host (`coding-hosted-1.body.json` model == `…:floor`).
- Stream semantics (typed terminals for Responses/Messages, `[DONE]` +
  last-usage-chunk for Completions, error chunk is a failure, missing usage →
  observed:false) are unchanged and unit-tested.
- Accounting: reservation math and budget ceilings unchanged; settlement
  prices from the pinned catalog (`cost()` = ceil + 1 µUSD), overruns fail the
  job and retain the raw cost.

## Verdict

**APPROVE** for the foreground-only scope. Every prior P1 (P1-1..P1-5) and P2
(P2-1..P2-3) finding is genuinely remediated with a test that exercises it from
the real seam (the end-to-end Completions fixture is a true Host
grant→fetch→settle→complete lifecycle, not a helper-level test), and the
remediation introduced no regressions in the reviewed code. The outstanding
items are six report-only P3 notes (doc/predicate mismatches, a free-string
test, unasserted provider-reported values, a pre-existing lifecycle flake, and
the unchanged unreachable `provider_observation` strictness) — none blocks merge
or the offline qualification gate. Foreground-only acceptance stands: the lane
is reachable from a Pi Host coding job offline with `reasoning.effort == "high"`
enforced and Layer C family-checked.

No live inference / no E1a run was performed.
