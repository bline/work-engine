# Review: Pi 1.6.0 Codex `max_output_tokens` strip (uncommitted, base 5cb83020)

Scope: `git diff HEAD -- skills/manual-orchestration/{scripts/pi_container.py,tests/test_pi_subscription.py,references/pi-host.md}`.
The concurrent `planning/owner-derived-coding-checkpoint-20261120.md` edit is outside this review.
Read-only review. The only file written is this one.

## Review

### Correct (verified by reading and running)

- **Root cause confirmed by reading.** In Pi 1.6.0, `/snap/pi-coding-agent/93/.../ai/dist/api/openai-codex-responses.js:426-427` sets `body.max_output_tokens = Math.max(options.maxTokens, 16)` whenever `maxTokens` is truthy and `compat.supportsMaxOutputTokens` is not `false`. The Codex catalog sets no such compat flag (`providers/data/openai-codex.json`). Pi 1.5.2's `buildRequestBody` (`/snap/pi-coding-agent/91/...:375-435`) never writes the field. `current -> 93`.
- **Stripping matches the qualified 1.5.2 behaviour, not just "no cap".** Pi 1.5.2 already computed the same `maxTokens`: `simple-options.js` uses the same `clampMaxTokensToContext`, and `cache-warming.js:49` uses `maxTokens: 1`. Its Codex transport then dropped the value. Removing the field upstream therefore reproduces what 1.5.2 sent for every request. This includes context-clamped and cache-warm requests.
- **Consistent with the lane contract.** Codex has no output cap in three places:
  - `EgressProxy.__init__` refuses `max_output_tokens` for `openai-codex` (`pi_container.py:1406-1407`).
  - `role_host._usd_limit` refuses `--max-output-tokens` and `--usd-strict-tier` for the Codex subscription (`role_host.py:1105-1107`).
  - The proxy clamp block (`pi_container.py:1939-1943`) is unreachable for Codex because `proxy.max_output_tokens` is always `None`.

  So removing Pi's advisory value cannot weaken any configured cap. It is also not silent: the request row records it.
- **Still fails closed.** The strip runs after `check_request`, which already allows the key, and before the unchanged Codex field gate (`pi_container.py:1923-1934`). Only `type(cap) is int and cap > 0` is popped. Any other value leaves the key in place and gets the existing 403 `subscription_request_fields_refused`, and `release()` returns the request slot as before. The strip touches only this one key. The `stream`, `store`, `service_tier`, model, tool and `previous_response_id` checks are unchanged.
- **zstd.** Decoding, the single-frame rule and size bounds run before parsing and are unchanged. The strip works on the parsed dict. Forwarding re-serializes `json.dumps(body)` with no `content-encoding`, as before (`pi_container.py:2015-2025`). Encoding makes no difference to the strip.
- **Duplicate keys.** `json.loads` keeps the last value, and the forwarded bytes are re-serialized from that same dict, so there is no parser differential.
- **SubscriptionProxyBudget.** `admit()` and `settle()` never read the request body (`pi_container.py:1193-1240`), and the strip happens before admission. Admission, durable rows, the threshold and the request count are unaffected. The docs already say one admitted response can overshoot.
- **Other lanes are unaffected.** `anthropic` and `claude-sdk` return at `pi_container.py:1912-1915`, before the new code. The new block is guarded by `proxy.provider == "openai-codex"`, so the API `openai` lane still clamps or forces `max_output_tokens` at `:1939-1943`.
- **Log consumers.** The new key is additive. `subscription_proxy_totals` and `subscription_proxy_records` (`pi_container.py:1068-1118`) and `metrics/scripts/usage_ledger.py` read fields with `.get` and have no unknown-key rejection.
- **Tests.** `test_pi_160_catalog_output_cap_is_removed_not_forwarded` checks the 200 response, that upstream does not receive the field, and the row value. The extended refusal cases cover `0`, `"64"`, `True`, `1.5` and `None`.

### Commands run

1. `cd skills/manual-orchestration/tests && timeout 300 python3 -B -m unittest test_pi_subscription.SubscriptionTests test_pi_subscription.SubscriptionBudgetTests`: **Ran 47 tests, OK (skipped=2).** In verbose mode both new and changed tests pass in both classes.
2. An inline, in-memory probe subclassing `SubscriptionTests`. Nothing was written to disk. Results:
   - `max_output_tokens: -5` returned 403.
   - `10**300` returned 200; the field was stripped and the 301-digit value was logged.
   - Duplicate keys `"x"` then `64` returned 200; `64` then `"x"` returned 403.
   - A zstd body with `max_output_tokens: 16` returned 200; it was stripped, the row recorded 16, and upstream never saw the field.

   All forwarded bodies lacked the field.
3. Reading only, against the Pi 93 and 91 snap sources: `buildRequestBody`, `simple-options.js`, `cache-warming.js`, `core/cache-warmer.js` and the Codex model catalog.

No Docker tests, network, credentials or repo edits. The parent owns the full regression; `RealSubscriptionContainerTests` is the test that drives real Pi 1.6.0 through this path.

### Finding P2-1: docs and comment say "catalog" max, but Pi sends a computed, possibly small, limit

- **Location:** `skills/manual-orchestration/references/pi-host.md:332-333`, `pi_container.py:1925-1927` (comment) and the new test's docstring.
- **Evidence:** In Pi 1.6.0, `options.maxTokens` comes from `buildBaseOptions`:

  ```
  clampMaxTokensToContext(model, context, options?.maxTokens ?? model.maxTokens)
  ```

  (`/snap/pi-coding-agent/93/.../ai/dist/api/simple-options.js:13-22,36`). That is the catalog 128000 **clamped to the remaining context window**. The transport then raises it to at least 16. Cache-warm replays use `maxTokens: 1`, which goes out as 16 (`cache-warming.js:49`). With catalog Codex models, cache warming is inactive because they have no `promptCache` (`cache-warmer.js:67-72`). An operator `--container-model-file` that sets `promptCache` can enable it.
- **Consequence:** The behaviour is still correct, because 1.5.2 dropped the same computed value. But the docs suggest the stripped value is always a harmless catalog ceiling. In fact it can be a deliberate context-fit or cache-warm cap that the backend never sees.
- **Smallest fix:** Reword to: "Pi 1.6.0 sends its computed output limit (the model's maximum clamped to the remaining context, at least 16) as `max_output_tokens` ... Pi 1.5.2 computed the same value but did not send it." Make the same change to the code comment.

### Finding P2-2: "the upstream request keeps its qualified shape" overclaims

- **Location:** `pi-host.md:334-335` and `pi_container.py:1926-1927`.
- **Evidence:** Pi 1.6.0 also changed the body in other ways.
  - It no longer sends top-level `instructions`. The system prompt now goes in a leading developer `input` item; see `/snap/pi-coding-agent/93/.../openai-codex-responses.js:404-424`, comment at 404-406.
  - Pi 1.5.2 sent `instructions: instructions || "You are a helpful assistant."` (`/snap/pi-coding-agent/91/...:399`).

  The proxy forwards both shapes, since `instructions` is optional in `check_request`. So stripping restores the 1.5.2 shape only for this one field. Live backend acceptance of the rest of the 1.6.0 shape is not shown by this change; validation is still a local mock.
- **Smallest fix:** Change it to "so, as with Pi 1.5.2, the upstream request carries no output cap".

### Finding P2-3: the stripped value is logged on only one of the three terminal row types

- **Location:** `pi_container.py:2096` adds the key only to the `request` row.
- **Evidence:** It is missing from:
  - the `upstream_error` row (`pi_container.py:2034-2036`), where "the request may have reached the provider";
  - the pre-header `stream_error` row (`:2048`).

  The docs say without qualification that the proxy "records it as `stripped_max_output_tokens`".
- **Consequence:** Audit only. The forwarded body was still modified on those paths.
- **Smallest fix:** Add the same conditional spread to those two `finish(...)` rows, or say in the docs that the request row records it.

### Note (pre-existing, not caused by this diff; not a finding)

- `test_native_pi_sse_transport_through_proxy` is pinned to `/snap/pi-coding-agent/77`, which no longer exists, so it is always skipped. Even pointed at `current`, it calls low-level `stream()` without `maxTokens`, so it would not reproduce this failure. Only the Docker `RealSubscriptionContainerTests` exercises real Pi 1.6.0 on this path, and it checks only that the turn completes, not the new row field. Worth a later refresh; out of scope here.

### Merge verdict

**OK with notes.** The code change is minimal, correct and fails closed. It is consistent with the lane's no-output-cap contract and with the qualified Pi 1.5.2 upstream behaviour, and it leaves the other lanes and budget accounting unchanged. Only the three P2 wording and audit notes remain; none blocks merge. This assumes the parent's full regression, including `RealSubscriptionContainerTests` against Pi 1.6.0, passes.
