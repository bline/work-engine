# Disposition: Fable review of W1-W4

| Finding | Disposition |
| --- | --- |
| P1-1 one-hour cache writes underpriced; reservation not worst case | Fixed. `pi_stream` extracts `cache_creation.ephemeral_5m/1h_input_tokens` (from `message_delta` if present, else `message_start`); the split is used only when it adds up, otherwise all writes count as one-hour. `messages_cost` prices one-hour writes at 2 × the tier's input rate. `messages_full_reservation` (Messages only; the Responses reservation is unchanged) uses the dearest input-side rate of any tier, including 2 × input: 1 320 001. `MESSAGES_ANALYSIS_BUDGET` is 43 000 000 (32 × 1 320 001 = 42 240 032). The settlement requires the raw split to add up to `cache_creation_input_tokens`. `analyze.py` budget and the record are updated; tests cover the split, the fallback, both write prices, the tier and the reservation. |
| P1-2 receipt names openai-codex | Fixed: `container_provider_identity` is `plan.provider`; `cost_evidence` is per wire. Verified in w4b receipts. |
| P2-1 local forwarder refusals invisible | Fixed: the receiver counts every local answer and reports `counts.forwarderRefused` in the `complete` frame on `claude-sdk`; the Host's analysis completion requires it to be 0. |
| P2-2 guard accepts missing overage header | Fixed: requires `Some("rejected")`; the assumed semantics are documented in code and record. |
| P2-3 retry header may be absent | Fixed: the Host requires `x-stainless-retry-count: 0`; test added. |
| P3-a thinking attribution | Record reworded (catalog `thinkingLevelMap` without `off` is the operative fact; API acceptance of `disabled` untried). |
| P3-b canonical copy not comparable | Accepted; the unification note stands (H-phase). |
| P3-c API-key failover untested | Stated in the record: do not enable live before one check. |
| P3-d max_tokens continuation fails closed | Stated in the record. |
| P3-e `unwrap_or(Wire::Responses)` in close path; no forwarder unit test | Fallback kept: it is in the uncertain-close cleanup path, which must not fail on a plan read; unreachable for a validated plan. The forwarder's coverage by the offline sets is stated in the record. |
