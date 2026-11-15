# Transition specification (normative, `pi-accounting-contract/1.4-s1`)

## Why factorised tables are complete

The owner asked for every combination of seven dimensions:
- reservation (3 states);
- settlement (4);
- usage (3);
- correlation (3);
- lane (2);
- lifecycle (3 terminal states × reopened yes/no);
- attribution (4).

That is 3,456 combinations.

The contract makes each stage a function of **only its declared inputs** (CONTRACT §1). So the combined result is the
composition of the stage tables below. A dimension a stage does not read cannot change that stage's output.

Completeness is shown per stage: each table below covers the full product of its own inputs. Composition rules state
which outputs flow where. The invariants INV-F7 (window independence) and INV-F4 (finding inertness) are the
executable check that no undeclared input leaks in.

| Stage | Reads | Does not read |
|---|---|---|
| S (decision, obligation) | reservation, settlement, correlation | usage valuation range, lane, lifecycle, attribution, findings |
| V (valuation) | usage, identities, tier, completeness | settlement decision, lane, lifecycle, attribution, findings |
| X (exposure) | S output, V output, lane | lifecycle, attribution, findings |
| K (findings) | C to X outputs | — (it is a sink) |
| L (lifecycle) | lifecycle rows, proxy-row stamps, reference time | money |
| T (ownership, placement) | X time facts, L, admission records, windows | amounts (copied, never changed) |
| R (rollup, admission facts) | T, X, K, L, commitments | — |

## Table S1: settlement decision and obligation

Reservation: `0` absent, `1` present, `2+` conflicting. Settlement: `0` absent, `V` valid (all §5 conditions),
`X` rejected, `2+` conflicting. Correlation is the settlement's join to the reservation.

| Reservation | Settlement | Correlation | Decision | Obligation | Amount source |
|---|---|---|---|---|---|
| 0 | 0 | — | `NotApplicable` (unbudgeted or orphan) | `None{Unbudgeted \| Orphan \| Subscription}` | valuation or unresolved (Table X) |
| 0 | V or X | `Row` key (C-2), no reservation of that `rid` | `Rejected[NoReservation…]` | `None{NoReservation}` | claimed charge or valuation, `AtLeast` |
| 0 | 2+ | — | `Conflicting` | `None{NoReservation}` | claimed or valuation, `AtLeast` |
| 1 | 0 | — | `Absent` | `Outstanding` | reservation bound |
| 1 | V (Usage), `Replacement::Eligible` (usage bound) | Exact | `Valid(Usage)` | `Closed{SettlementAmount}` | **settlement and valuation replace the reservation; `Exact`** (FB-1) |
| 1 | V (Usage), `Replacement::Ineligible` (incomplete or unpriceable usage) | Exact | `Valid(Usage)` | `Closed{ReservationBound}` | **reservation retained**; known = max(R, charged, usage high or priced high), `AtLeast`; supported usage low kept (A-01) |
| 1 | V (Reservation, UpstreamError) | Exact | `Valid(..)` | `Closed{ReservationConfirmed}` | reservation as a known amount (the proxy kept it as its charge), `AtLeast` (billing unresolved) |
| 1 | V (ErrorStatus) | Exact | `Valid(ErrorStatus)` | `Closed{ReservationBound}` | **reservation retained, not replaced** (D4); reported usage kept: known = max(R, usage high or priced high) (A-02) |
| 1 | X | Exact | `Rejected[reasons]` | `Outstanding` | reservation, raised to claimed and to any valuation high (R-05) |
| 1 | 2+ | Exact | `Conflicting` | `Outstanding` | reservation, raised to claimed and to any valuation high (R-05) |
| 2+ | 0 | — | `Absent`, plus blocking `ReservationConflict` | `Outstanding{ReservationBound}` | R = largest distinct reservation (A-09); all stay visible |
| 2+ | V or X | Exact | `Rejected[ReservationConflict…]` | `Outstanding{ReservationBound}` | max(largest distinct reservation, claimed, valuation high) |
| 2+ | 2+ | Exact | `Conflicting` | `Outstanding{ReservationBound}` | max(largest distinct reservation, claimed, valuation high) |
| `rid` unusable | — | `Row`, `Unattributed` (C-1) | `Absent` | `Outstanding` | well-formed R or 0, `AtLeast`; `MalformedEvidence`, `UnaccountedAttempt` |

**Correlation note.** Settlements join only by an `Exact` `rid` key (C-2): that is the attempt's `key_join`. Shape
rules RS-1 to RS-3 join error and refusal rows as `auxiliary` rows (C-5 to C-7); `Ambiguous` and `Unattributed` keys
apply only to error rows that join nothing (C-8). Auxiliary rows never close an obligation: they add evidence
(diagnostics, identity, forwarding) but cannot produce `Valid` or change `key_join` (R-06).

## Table V: valuation

| Usage evidence | Identities | Tier | Valuation | Complete |
|---|---|---|---|---|
| none | — | — | `NotReported` | — |
| well-formed, every identity priced | 1 | OpenAI recognised; Anthropic standard, null or absent | `Valued{low, high}` | terminal success with consistent counts: true; else false |
| well-formed, every identity priced | ≥ 2 | as above | `Valued{low = cheapest, high = dearest}` (contradictory) | as above |
| well-formed, some identity has no rate | any | any | `Unpriceable{IdentityWithoutRate, supported, priced_high}` | — |
| well-formed | any | OpenAI missing or unrecognised | `Unpriceable{TierUnestablished, supported: None, priced_high}` (R-03) | — |
| malformed count, malformed tier, non-standard Anthropic tier | any | any | `Unpriceable{reason}` | — |
| OpenAI cache count present (including 0) | any | recognised | `Valued` with that count | as above |
| OpenAI cache count null or absent key (A-04) | any | recognised | `Valued` over the vertices of `cached + write ≤ input`, known counts fixed; never assumed 0 | as above |
| OpenAI cache count malformed, or known counts above input | any | any | `Unpriceable{MalformedCount}` | — |
| Anthropic residual writes (split ≤ writes) | any | standard | `Valued`, residual at min (low) and max (high) (R-15) | as above |
| Anthropic split > writes | any | standard | `Valued` with writes at min and max rate; diagnostic `InconsistentCacheSplit` | false (counts contradict) |
| several usage observations, all valued | union | any | V-8: min of lows, max of highs | all complete |
| several usage observations, any unpriceable (A-04) | union | any | `Unpriceable{union of reasons, supported: None, priced_high: max of valued and priced highs}` | — |

## Table X: exposure (decision × valuation × identities × lane)

"1 id" / "≥2 id" is the identity count. `R` is the reservation amount. Bases are in brackets.

| Decision | Valuation | Ids | low | high | Blocking added by K |
|---|---|---|---|---|---|
| Valid(Usage) | Valued, complete | 1 | val.low [UsageValuation] | Exact(max(charged [ProxyCharge], val.high)) | — |
| Valid(Usage) | Valued, complete | ≥2 | val.low | Exact(max(charged, val.high)) | IdentityContradiction |
| Valid(Usage), Ineligible | Valued, not complete | any | val.low | AtLeast(max(R [ProxyReservation], charged [ProxyCharge], val.high [UsageValuation])) | UnresolvedExposure (≥2 ids: IdentityContradiction) |
| Valid(Usage), Ineligible | Unpriceable (any reason) | any | supported or 0 | AtLeast(max(R, charged, priced_high or 0)) | UnresolvedExposure |
| Valid(Reservation) | any | any | val.low or supported or 0 | AtLeast(max(R [ProxyReservation], val.high or priced_high or 0)) | UnresolvedExposure (≥2 ids: IdentityContradiction) |
| Valid(UpstreamError) | any | any | 0 | AtLeast(R) | UnresolvedExposure |
| Valid(ErrorStatus) | any | any | val.low if Valued, else supported or 0 | AtLeast(max(R [ProxyReservation], val.high or priced_high or 0)) | UnresolvedExposure (diagnostic NonBillingUnverified) |
| Absent | any | any | 0 | AtLeast(R) | UnresolvedExposure (2+ reservations: ReservationConflict; joined refusal: IdentityContradiction; unusable `rid`: MalformedEvidence, UnaccountedAttempt) |
| Rejected or Conflicting, with reservation | any | any | 0 | AtLeast(max(R, well-formed claimed, val.high or priced_high)) | SettlementRejected or SettlementConflict, UnresolvedExposure |
| Rejected or Conflicting, no reservation | any | any | 0 | AtLeast(max(well-formed claimed, val.high or priced_high) or 0) | SettlementWithoutReservation, UnresolvedExposure |
| NotApplicable unbudgeted | Valued, complete | 1 | val.low | Exact(val.high) | — |
| NotApplicable unbudgeted | Valued, complete | ≥2 | val.low | Exact(val.high) | IdentityContradiction |
| NotApplicable unbudgeted | Valued, not complete, or Unpriceable | any | supported or 0 | AtLeast(val.high or priced_high or 0) | UnresolvedExposure |
| NotApplicable unbudgeted, non-200, no usage | — | — | 0 | AtLeast(0) | UnresolvedExposure{UnverifiedNonBilling} |
| NotApplicable orphan or ambiguous error | — | — | 0 | AtLeast(0) | UnaccountedAttempt, UnresolvedExposure |

**Lane overlay** (applied after the rows above):

| Lane evidence | Pool | Effect |
|---|---|---|
| API session, labels consistent or absent | PiApi | as above |
| API session, subscription labels or measurement, or another provider | PiApi | as above, plus LaneContradiction |
| Subscription session, labels consistent or absent where the producer writes none | Codex | no dollar exposure; billing Unknown; an unfinished admission is an unknown Codex call |
| Subscription session, contradicting label on any event, dollar evidence, `IdentityRefusal`, unadmitted measurement | PiApi | low 0, high AtLeast(0) (or the dollar rows above), plus LaneContradiction (and UnaccountedAttempt for admissions and errors) |

## Table L: lifecycle

| Lifecycle rows (append order) | Proxy-row stamps | State | Reopened | Clock |
|---|---|---|---|---|
| launch (+ delivers), no retire | consistent | Open | no | Trusted |
| … ending in a dated retire, no deliver after any retire | consistent with L-2 and L-3 | Retired{at} (a fact, not a gate: L-7) | no | Trusted |
| … ending in an undated retire | consistent | Open (bounds nothing) | no | Trusted |
| retire followed by deliver, last row a deliver | consistent | Open | **yes** (D2: blocks admitting objectives) | Trusted |
| retire, deliver, …, ending in a dated retire | consistent | Retired{at} (releases nothing without a qualified gate, L-7) | **yes** (still blocks, R-10) | Trusted |
| backwards stamps (L-1) | any | Contradicted | as rows show | Untrusted |
| turn rows before their start (L-2), or after a following retirement (L-3) | — | Contradicted | as rows show | Untrusted |
| any | a row after the reference time, or proxy rows backwards | (state as above) | as above | Untrusted |

## Table T: ownership × placement → attribution

| Ownership | Clock | Interval vs owner window | Attribution |
|---|---|---|---|
| Owned(O) | Trusted | inside [start, closure or open] | Inside(O) |
| Owned(O) | Trusted | reserved inside, ends after O's closure | Inside(O), O `provisional` (C1) |
| Owned(O) | Trusted | begins before O's start | Straddles{Start}: blocks O |
| Owned(O) | Trusted | wholly before start or after closure | Before or After: still O's money (P-5; §12.1 sums owned attempts whatever their placement), plus blocking `AttributionContradiction` for O (an admission/time contradiction) |
| Owned(O) | Untrusted | — | Unplaceable: blocks O |
| Uncertain{cands} | any | — | Uncertain: blocks every candidate (OwnershipUncertain). Candidates always include every admitting objective (O-3a, R-08) |
| Unadmitted{cands} (O-6) | any | — | Unadmitted: blocks every candidate (HistoricalUnadmitted). Candidates = tree ∩ temporally possible on the obligation interval; unplaceable keeps (O-7, R-12) |
| Unowned | any | — | affects no objective |

## Table R: admission facts

| Condition | Status |
|---|---|
| no blocking finding, no AtLeast line, no `AtLeast` commitment, no Straddles, Unplaceable, Uncertain or Unadmitted affecting O, commitments valid, no claimed closure that is not established | Established |
| any of the above | NotEstablished{every reason} |

## Table A: admission decision (§12.4)

| Closure | Limit | Known total vs limit (OQ-2) | Accounting | Decision |
|---|---|---|---|---|
| established (legacy cooperative or vertical authority) | any | any | any | **refused** (authorization; obligations remain) |
| claimed, not established | any | any | O is `NotEstablished{ClosureNotEstablished}` | **not_established** |
| none | malformed | any | any | not_established |
| none | valid | known > limit (owned amounts only, known parts of `AtLeast` included) | any | **refused** (conservative budget policy; invoice not known) |
| none | valid | known ≤ limit | NotEstablished | not_established |
| none | valid | known ≤ limit | Established | admitted |

## Table G: session commitments (§12.3, §2.3)

| Session | Gate (qualified per G-1..G-7) | State | Committed / kind |
|---|---|---|---|
| pending (no launch, no release) | none, or no `billing_bound` | pending | cap, `AtLeast` |
| pending | `billing_bound` (with a G-8 numerical bound ≤ cap) and `retries` enforced | pending | bound amount, `Exact` |
| launched, open | none, or no `billing_bound` | launched_open | max(0, cap − owned high), `AtLeast` |
| launched, open | `billing_bound` (with a G-8 numerical bound ≤ cap) and `retries` enforced | launched_open | max(0, bound − owned high), `Exact` |
| launched, `Retired` | none, or not all of `new_forwarding`, `retries`, `reopening` | **retired_resumable** | max(0, cap − owned high), `AtLeast` |
| launched, `Retired` | `new_forwarding`, `retries`, `reopening` enforced, not contradicted | retired | 0, `Exact` |
| launched, reopened, not retired | any (a claimed `reopening` gate is contradicted, G-7) | reopened | max(0, cap − owned high), `AtLeast` |
| released before launch | — | released | 0, `Exact` |
| malformed cap | — | invalid | 0, `Exact` |

## Table P: predecessor line (§12.2)

| Predecessor-owned attempt | Billing | In the successor's line |
|---|---|---|
| `Outstanding` | Unresolved | yes, its high (`AtLeast`) |
| `Closed`, high `AtLeast` (upstream error, error status, reservation basis, incomplete usage) | Unresolved | **yes** (R-07) |
| `Closed`, high `Exact` | Established | no (settled spend) |
| open commitment (pending, launched-open, retired-resumable, reopened) | — | yes, `AtLeast(committed)` unless a qualified `billing_bound` makes it `Exact` (A-5) |

**Rule FB (rev 3; rev 5 adds replacement eligibility).** A high is `Exact` only with a usage bound: complete usage over established identity and tier
sets, which also makes the usage settlement eligible to replace the reservation. Reservations, proxy charges and declared caps are known amounts, never bounds (CONTRACT §2.1, §2.2).

## No second fallback

Table X is total. Every cell names its amount source, and no cell is chosen by the presence or absence of a finding.
The previous mechanism (choose a reservation-only representation when "something went wrong while pricing") has no
counterpart: a pricing failure is the `Unpriceable` valuation state, an input to a fixed cell. INV-F4 and the
validator's `suppress-diagnostics` self-test check this.
