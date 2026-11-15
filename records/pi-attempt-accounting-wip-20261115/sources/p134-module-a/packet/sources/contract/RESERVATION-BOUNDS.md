# Is a proxy reservation an upper bound on provider billing? (Q2 analysis, contract 1.1-s1; commitments added in 1.2-s1; mechanisms in 1.3-s1; numerical bound in 1.4-s1)

**Producer:** release `92a03789`, `pi_container.py` `78083734`.

**The reservation formula.**
- **OpenAI:** `reserve_usd` (`:689-694`) = `(input_bound × max(r_input, r_cache_write) × m_in + max_output × r_output ×
  m_out) / 10^6 × multiplier`.
- **Anthropic:** `worst_case` (`:1038-1041`) = `input_bound × max(r_input, r_5m, r_1h) + max_output × r_output`.

For R to bound what the provider bills, every condition below must hold. The table records which ones the producer
**enforces**.

| # | Condition | Enforced by the producer? | Evidence |
|---|---|---|---|
| B-1 | output tokens ≤ `max_output_tokens` | **yes**: the proxy overwrites the request's `max_output_tokens` (OpenAI) or `max_tokens` (Anthropic) when absent or not below the cap, and refuses an Anthropic thinking budget not below it | `:1162-1170` (Anthropic); `:1404-1408` (OpenAI `max_output_tokens` rewrite) |
| B-2 | input tokens ≤ `input_bound` | **no**: a byte-count estimate (`rest_bytes + item bytes + overhead × items`, or a provider-reported prefix plus new bytes). The producer's own docstring says an inline image's token count staying below its base64 length is "assumed, not verified" | `input_bound` `:725-741`; `unpriceable_input` docstring `:743-748` |
| B-3 | the provider bills the pinned model's rates | partly: the request model is pinned (`model_not_pinned`), and the Anthropic path refuses an unverified response model. But billing follows the model the provider ran, and the OpenAI path never checks the response model | `check_request` `:526-545`; `anthropic_identity_refusal` `:625-639` |
| B-4 | the provider bills at no more than the reserved tier multiplier | **no**: the proxy requests a tier, and the provider may serve another. `UsdBudget` charges "a response served at another tier at that tier", and a missing tier at the highest multiplier | `UsdBudget` docstring `:948-960`, `settle` `:994-1021` |
| B-5 | no surcharge outside the rate table (e.g. long context) | **no**: Anthropic: "No long-context rule is applied" | `AnthropicUsdBudget` docstring `:1023-1033` |
| B-6 | an errored or aborted request costs no more than its worst case | **not established**: the producer assumes a non-200 status is unbilled ("assumed, not verified") and keeps the reservation for broken streams. It has no evidence of the provider's billing either way | `UsdBudget.settle` `:1003-1004`; `AnthropicUsdBudget.settle` `:1050-1051` |

**Conclusion.** Only B-1 is producer-enforced. B-2, B-4, B-5 and B-6 are assumptions; B-3 is partial. So the
reservation is a known amount the proxy set aside, not a proven upper bound (rule FB-2). The same holds for a proxy
charge that is not a usage price.

**What would establish a finite bound, as future producer evidence (not in S1):**
- the provider's own usage or bill for the attempt;
- a producer that refuses unbounded inputs (B-2), pins and verifies the served tier (B-4), and refuses any route with
  surcharges outside the rate table (B-5);
- provider documentation, recorded as authoritative evidence, for the billing of errored requests (B-6; this is D4's
  non-billable class).

**Not affected:** an exposure backed by complete provider-reported usage (FB-1) remains `Exact`, conditional on the
stated evidentiary standard (CONTRACT §2).

## Session caps and commitments (revision 3, R-04)

The same conditions decide whether an **admission cap** (`max_usd`) bounds a session's provider billing:
- `reserve()` refuses a new reservation when `settled + outstanding + worst case > max_usd` (`:984-988`). That is an
  enforced constraint on **reservations**, and a reservation is not a bound (B-2, B-4, B-5, B-6).
- `settle()` charges the served tier and usage with no cap (`:994-1019`). Spend can therefore exceed the cap. For
  example (P1.29 R-04, reproduced): gpt-6-astra, input bound 2,400 and output cap 9,000 reserve 0.48 at the default
  tier, inside a 0.5 cap; the same tokens served at priority cost 0.948.

So a declared cap is an unenforced declared cap (CONTRACT §2.2), and `cap − spend` is an internal accounting
commitment, not a bound. Revision 4 adds that a retired Pi session can resume under the same cap without objective
admission (CONTRACT §2.3), so retirement does not end the session's capacity to spend either. Every commitment
without a qualified `ExecutionGate` is `AtLeast` (CONTRACT §12.3 A-5). An enforced `billing_bound` needs the conditions
above enforced across forwarding, retries, failures, incomplete streams and settlement (OQ-3). It must also bind a numerical
USD amount no greater than the cap, with a binding digest (CONTRACT §2.3 G-8, R5-03). The current producer provides none.
