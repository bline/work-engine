# Change log: `pi-accounting-contract/1.3-s1` → `1.4-s1` (revision 5, P1.33)

Revision 4 is preserved byte-for-byte (`~/cbm-work/p132-s1r4/s1`, manifest `3af24453…`). The complete diff of
contract, corpus and validator is `evidence/s1-rev4-to-rev5.complete.diff` in the P1.33 workspace. D1–D10, Q1–Q9
dispositions and OQ-1 to OQ-4 are unchanged, except where an approved Module A disposition says otherwise (A-02 refines
the Q2 error-status row).

| Finding | Change | Where |
|---|---|---|
| A-01 (P1, owner decision) | six monetary facts kept apart; exposure defined as one measure (conservative provisional exposure); `Replacement` fact; only an eligible usage settlement replaces the reservation; ineligible usage keeps `R` (`ReservationBound`) with its supported low; X-2 names the only `AtLeast` → `Exact` transition; F3 restated; F13 added | CONTRACT §2.1, §6, §6.2, §8 rows, X-2; TRANSITIONS S1, X; INVARIANTS F3, F5, F13 |
| A-02 (P1, owner decision) | `Valid(ErrorStatus)` keeps supported usage: low = usage valuation low, high = AtLeast(max(R, usage high)); the error-status-with-usage case is synthetic, reachability not established | CONTRACT §8; TRANSITIONS S1, X; DECISION-REGISTER |
| A-03 (P1, owner decision) | `BillingBound` record and rule G-8 (numerical USD amount ≤ cap, session scope, full coverage, binding digest); the cap ceiling becomes the bound in A-2, A-3, A-5; signature verification is a stage-E obligation of the future vertical | CONTRACT §2.3, §12.3; TRANSITIONS G; INVARIANTS A9; RESERVATION-BOUNDS |
| A-04 (P1) | V-4 cache-count states (present, null, absent key, malformed), never assumed zero, with unknown counts enumerated over vertices; V-8 mixed observations (no supported low; priced high = max of all highs) | CONTRACT §7.2; TRANSITIONS V |
| A-05 (P2) | model canonicalisation `mc-1` (MC-1..MC-3: split the launch model at the first `/` as the producer does, exact byte equality, no fuzzy matching, `ModelUnresolved`) | CONTRACT §3, §5 condition 8, V-1 |
| A-06 (P2, owner decision) | status by settlement path (integer on request rows; absent only on `upstream_error` rows); A06 rejected `StatusType` and marked producer-unreachable; S06 valuation re-derived (15M, incomplete) | CONTRACT §5 condition 7; DECISION-REGISTER |
| A-07 (P2) | V01-usage-settlement-other-response-model reclassified adversarial-synthetic, producer-unreachable | corpus |
| A-08 (P2) | `amount_source` typed: `SettlementAmount`, `ReservationConfirmed`, `ReservationBound`, with construction and absence rules | CONTRACT §6; TRANSITIONS S1; schema |
| A-09 (P2) | conflicting reservations stay visible and block; `R` is the largest, never the sum or the smaller; C-1 aligned with Table S1 | CONTRACT C-1, X-0; TRANSITIONS S1 |
| coordinator-found | X-0 tie order corrected to UsageValuation > ProxyReservation > ProxyCharge (matches 9 KeepAll cases; no amount change) | CONTRACT X-0; MONEY.md |

**Corpus.**
- Revision-4 cases (157): 144 carry a logged change. Every attempt gains `replacement`.
- Non-structural changes:
  - U08, Q5, P2, P130-R01: the reservation is retained;
  - R04: 0.9 → 0.8;
  - A06: rejected `StatusType`;
  - S06: valuation;
  - V01-other: reachability;
  - P132-FG-VC14-open-session-billing-bound: becomes a negative control;
  - P132-FG-VC01-before-retirement: the bound is missing.
- Admission changes: one (P132-FG-VC14, admitted → not_established).
- Added cases (16): one or more per finding, each with a single-condition control.

**Validator.**
- **New checks:**
  - replacement and the typed `amount_source`;
  - Table X 1.4 rows with the X-0 basis tie order;
  - raw-row checks of the status by path, canonical model equality, `R` from the reservation rows, the OpenAI
    valuation range (including unknown cache counts), and priceable usage never `not_reported`;
  - G-8 in gate qualification, with the bound ceiling in Table G;
  - a revision-5 log check.
- **Self-test:** 34 defects (29 carried, 5 new).
- **Sabotage:** 90 (76 carried, one retargeted, plus 14 new).
