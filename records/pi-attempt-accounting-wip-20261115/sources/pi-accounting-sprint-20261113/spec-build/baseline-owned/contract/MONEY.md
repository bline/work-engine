# Money precision and representability (D10 findings)

**Script:** `evidence/money/representability.py`; output `evidence/money/representability.txt`. Run against the
release rate files:
- `openai_api_rates.json` `c0bdfd66ee69f1644eaf188f87f6af5e47beacc0739d20aac5846c923d66fd58`;
- `anthropic_api_rates.json` `8a286433d9e08b37533f2f908f2db60c4357e1a566018c9ca8364937652bdef5`.

Rates are parsed from the JSON text as exact decimals, never as binary floats.

## Findings

1. **Every rate is exact.** All 163 rate × multiplier combinations the producer can apply are exact integers in
   nano-USD per token, with largest denominator 1:
   - OpenAI: 4 models × 4 rate fields × 4 tiers × long/short = 128;
   - Anthropic: 7 models × 5 fields = 35.
2. **The literals involved:** rates `0.05 … 50` USD per million tokens (two decimal places at most); multipliers 0.5,
   1, 2, 1.5.
   - The finest product is 0.05 × 0.5 (flex cached) = 0.025 USD per million = **25 nano-USD per token**.
   - The finest long-output product is 5 × 0.5 × 1.5 = 3.75 USD per million = 3,750 nano-USD per token.
3. **Valuations are therefore exact integer arithmetic**, with no rounding: tokens × integer.
4. **Producer amounts** (`charged_usd`, `reserved_usd`) are logged with at most 6 decimals, so they are exact in nano
   (× 1000).
5. **Overflow headroom.**
   - The largest rate is 150,000 nano per token (50 USD per million × 2 priority × 1.5 long output).
   - Even 2^40 tokens (about 10^12) give 1.6 × 10^17 nano, against an `i128` maximum of about 1.7 × 10^38.
   - Overflow can only come from malformed counts (e.g. 10^30 tokens), which checked arithmetic reports as
     `ArithmeticFailure` (blocking).
6. **Conclusion:** nano-USD `i128` is sufficient for the current rate set. No alternative is needed.

## Rules for rates that are not exact (future-proofing; none today)

- A rate file is loaded as exact rationals. If any rate × multiplier is not an integer in nano per token, the table is
  `UnsupportedPrecision` and loading fails, so all valuations are `Unpriceable{RateTableUnusable}` (blocking). This is
  the S1 rule, so nothing is ever silently approximated.
- **If the owner later wants to admit such rates,** the specified rounding is: compute the whole valuation as an exact
  rational, then round once, `low = floor`, `high = ceil`.
- **Proof (corrected in revision 3).** For a rational `p`: `floor(p) ≤ p ≤ ceil(p)`, `p − floor(p) < 1` and
  `ceil(p) − p < 1`. (`ceil(p) − floor(p)` is 0 for an integer `p` and exactly 1 otherwise; revision 2 wrongly stated
  `< 1`.) So each bound errs by under 1 nano-USD, toward the conservative side. A sum of k bounds rounded separately
  errs by under k nano, still directional. Rounding once per valuation keeps it under 1.

## Producer-consistency tolerance (`T_round`)

The proxy computes a usage charge in binary floating point and logs `round(charge, 6)`.

**Bound.**
- `|logged − p_float| ≤ 0.5 × 10^-6 USD` = 500 nano (6-place rounding).
- `p_float` comes from at most about 10 float operations on exact inputs, so `|p_float − p| ≤ 10 × 2^-53 × |p|`. For
  `p < 10^15` nano (10^6 USD) that is under 1.2 nano.
- So `|logged − p| < 501.2` nano. Both `logged` (a multiple of 1,000 nano) and `p` (exact rates × integer counts) are
  integers in nano, so their difference is an integer, and `|logged − p| ≤ 501`.

**Rule.** `T_round = 501` nano, applied only when `p < 10^15` nano. For `p ≥ 10^15` nano the proof above does not
hold, so the settlement is `Rejected[ToleranceRange]` (CONTRACT §5), never excused and never checked with an
unproven tolerance. Revision 2's scaling formula `T_round(p) = 500 + ceil(1.2e-15 × p)` stated no unit for `p`; it is
removed.

**Direction (R-02).** The logged charge can sit up to 500 nano above or below the producer's float value, and rounding
up can move a sub-micro-USD charge from below 500 nano to 1,000 nano (for example gpt-6.1-terra flex, 21 cached
tokens: exact price 525 nano, logged 1,000). This never lowers conservative exposure, because the charge enters a high
only as a term of `max(charged, usage bound)`, and a high is finite only through its usage bound (CONTRACT §2.1 FB-1).

**Direction.** The tolerance can only accept a charge up to `T_round` below the exact price. The high bound always uses
`max(charged, exact valuation)`, so the tolerance never lowers exposure.

## Bases on ties (revision 5)

When two terms of a maximum are equal, the high's basis is `UsageValuation`, then `ProxyReservation`, then
`ProxyCharge` (CONTRACT X-0). Revision 4's text put `ProxyCharge` before `ProxyReservation`, contradicting the 9 KeepAll
cases (charged = R), whose basis is the reservation. The amount is unchanged; only the provenance label was in question.
