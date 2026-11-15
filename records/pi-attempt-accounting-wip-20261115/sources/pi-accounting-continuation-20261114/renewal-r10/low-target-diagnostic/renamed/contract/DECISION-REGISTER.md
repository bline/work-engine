# Owner decision register (P1.28 to P1.33, contract `1.4-s1`)

| # | Owner decision (P1.28) | Where applied | Residual question for the owner |
|---|---|---|---|
| D1 | C1 reservation-time ownership. A closed objective with unresolved obligations keeps provisional totals. Ownership is never inferred from ancestry; an uncertain owner blocks the affected objectives. Predecessor obligations constrain successors without transferring or double-counting settled spend | CONTRACT §11.1 O-1 to O-4, §11.2 P-4, §12.2; TRANSITIONS Table T | **Q1:** successors are defined by an explicit `predecessor` field on the objective record, which does not exist today. Shared-root objectives without the link keep XO-1 open |
| D2 | R1 reacquire before execution. Reopening is refused until an atomic admission mechanism exists; retrospective reinstatement is not enforcement | CONTRACT §10 L-5, §11.1 O-2, §12.3 A-4 | none for S1. The enforcement point (refusing delivery to a retired session) is in the admission authority, outside the core |
| D3 | One Rust crate with an I/O-free core; adaptation and integration outside it. No crate in S1 | CONTRACT §1 (stage E outside the core) | none |
| D4 | Unknown provider billing is `AtLeast(0)`. `Exact(0)` only for an authoritatively nonbillable error class; a proxy assumption alone is insufficient | CONTRACT §6 (ErrorStatus closes but does not replace), §8 (unbudgeted non-200: `AtLeast(0)` blocking) | **Q2 (P1.28 text, superseded by the P1.29 disposition: `AtLeast(R)`):** with a reservation present, an error-status attempt keeps `Exact(R)` (the producer's worst-case bound) rather than `AtLeast(0)`, so it does not block. Confirm, or require `AtLeast(R)`. **Q3:** the nonbillable class is empty until authoritative provider evidence is supplied. Real records contain 0 error-status settlements and 0 non-200 request rows (89 logs), so neither choice changes a historical result |
| D5 | `AtLeast(x)` is not a finite maximum; admission cannot use it as headroom; blocking | CONTRACT §2, §8 X-2, §9 `UnresolvedExposure`, §12.1 | none |
| D6 | Every unresolved identity contradiction blocks; diagnostics never modify amounts | CONTRACT §7.2 V-1, §8, §9, X-3, K-1 | none |
| D7 | Additive `attempt_id = (incarnation, sequence)`; `n` stays separate; no producer change | CONTRACT §4 (`ProducerAttempt` key, `attempt_id` feature flag) | none for S1 (S6 unauthorised) |
| D8 | A sanitised, local-only corpus derived from the real logs; digests, provenance and mappings kept; no raw logs committed | corpus `real/` (derived aggregates and sanitised per-session cases; raw logs stay in place) | none |
| D9 | Plan IR recovery stays independent; no resumption in S1 | — | none |
| D10 | Checked `i128` nano-USD; exact rates; directional rounding where needed; overflow and precision fail explicitly | CONTRACT §2; MONEY.md | none: all 163 current rate × multiplier combinations are exact integers in nano-USD per token |

## P1.29 owner dispositions (revision 2)

| Question | Owner disposition (P1.29) | Applied as | Status |
|---|---|---|---|
| Q2 | Keep reservation-based conservative exposure for error-status attempts. Distinguish reserved, proxy-charged, an evidenced bill, and unknown additional billing. A reservation is not automatically a proven bound; a finite bound needs producer-enforced conditions; never `Exact(0)` because of an error | CONTRACT §2.1 (FB-1, FB-2, the four amounts), §6 (error-status), §8 Table X; `RESERVATION-BOUNDS.md` (only the output cap B-1 is producer-enforced) | **closed.** Consequence: Q10 |
| Q5 | Missing OpenAI tier: an interval over all supported plausible tiers with exact rates; high ≥ the dearest rate and ≥ the proxy charge; no unsupported low; if the tier set cannot be established, unresolved exposure | V-3: missing gives every table tier (low cheapest, high dearest); an unrecognised string gives `Unpriceable{UnknownTier}`, so `AtLeast` with no supported low | closed in P1.29; **superseded by the P1.30 R-03/Q5 disposition** (a missing tier no longer establishes the tier set) |
| Q6 | An identity contradiction blocks; verified financial evidence is kept independently; non-forwarding is distinguished; diagnostics never change money | §6.1 forwarding fact (identity refusals are `Forwarded`; `refused` rows are non-attempts); X-3 and K-1 unchanged; refusal exposure follows FB-2 (`AtLeast`) | closed |
| Q8 | No synthesised authority for historical unadmitted requests. They are historical, unadmitted evidence with financial data kept; only possible owners block; future vertical execution needs admission | O-6 (`Unadmitted`), O-7 (candidates = tree ∩ temporally possible), O-8 (`UnadmittedExecution` for vertical-produced attempts); reason `HistoricalUnadmitted` | closed |

**New owner question from revision 2:**
- **Q10.** FB-2 with D5 means any owned attempt whose exposure rests on a reservation alone makes its objective
  `NotEstablished`. That covers in-flight requests, unsettled reservations, error statuses, upstream errors,
  reservation-basis settlements and rejected settlements.
  - **Operational effect:** a new session cannot be admitted while another session of the same objective has a request
    in flight or an unsettled reservation.
  - **Historical effect:** the frozen Step D objective (two reservation-basis charges) would be `NotEstablished` under
    this contract.
  - **Options:**
    - accept this (strict, consistent with Q2 and D5);
    - define producer evidence that makes a reservation a bound (B-2, B-4 and B-5 enforced; a future producer slice);
    - allow admission against a stated assumption class, as an explicit owner-authorised exception, contrary to the
      current Q2 wording.

  Revision 2 encodes the strict option.
- **Q11. Refusal versus `not_established` when refusal is already certain.** §12 decides `not_established` for any
  objective that is not established, before the limit test. Under revision 2, two cases that revision 1 refused now read
  `not_established`. Both decisions deny admission.
  - `D1-closed-objective-provisional`: the objective is closed. The revision-1 validator refuses admission to a closed
    objective, a rule the §12 text does not state.
  - `D1-successor-constrained-by-predecessor-obligations`, evaluation `successor`: the known amounts alone (predecessor
    line `AtLeast(1.0)` plus the new 0.1) already exceed the 1.0 limit.
  - **Options:** keep the current order (any unresolved exposure gives `not_established`); or refuse first when the
    objective is closed or when the known amounts already exceed the limit. Either way, state the closed-objective rule
    in §12 or drop it from the validator.
  - Revision 2 leaves the §12 text and the validator unchanged.

**Remaining open after P1.29:** superseded by the P1.30 section below.

## P1.30 owner dispositions (revision 3)

| Item | Owner disposition (P1.30) | Applied as | Status |
|---|---|---|---|
| R-03 / Q5 | The rate table is not evidence that every service tier is known. A finite interval needs evidence of the complete applicable tier set; otherwise keep known valuation and represent unresolved additional exposure. Never manufacture an upper bound from an incomplete table | V-3: only a recognised `observed_tier` establishes the tier set. Missing or unrecognised gives `Unpriceable{TierUnestablished}`, with `priced_high` kept and no supported low; §8 gives `AtLeast`. Supersedes P1.29's missing-tier treatment | closed |
| R-04 / Q10 | A pending request whose exposure is not provably bounded prevents established admission for objectives that may own it. An unenforced session cap is not a billing bound. Distinguish an enforced constraint, an internal commitment, an unenforced declared cap and unknown additional exposure. Bounded concurrency may be qualified later; for S1, unbounded or unverifiable in-flight exposure blocks. Keep positive controls | §2.2 (the four amounts); FB-2 extended to declared caps; §12.3 A-5: open and pending commitments are `AtLeast` (`UnenforcedCommitment`) unless a `QualifiedBound` exists; no S1 evidence qualifies. Controls: 17 admitted (minimum 15 unchanged) | closed; interpretation OQ-1; **amended by P1.32** (OQ-1: retired current-producer sessions also keep their commitment; QualifiedBound replaced by ExecutionGate) |
| Q11 | Authoritative closure gives REFUSED before monetary headroom; it is an authorization result, not complete accounting. Unresolved predecessor obligations stay recorded and constrain successors. Uncertain closure gives NOT_ESTABLISHED | §12.4 step 1 and Table A; `ClosureUncertain`; §12.2 (closure discharges nothing); A7 | closed; authority definition NEW-1; **amended by P1.32** (OQ-4: closure needs a consistent close entry (legacy) or authority record (vertical); ClosureUncertain renamed ClosureNotEstablished) |
| R-07 | Terminal execution does not settle billing. Under C1, predecessor obligations persist until their financial disposition is established. Keep execution completion, valid proxy settlement, usage valuation, verified billing and unresolved exposure distinct. A successor gains no headroom because a predecessor attempt closed | §6.2 (`Execution`, `Billing`); §12.2 and Table P (line selects by billing); F12; A4 | closed |
| R-08 | No filtering or precedence rule may discard conflicting authority evidence before classification. Contradictions stay visible and block | O-3a; O-7 for `Uncertain`; T2; validator check | closed |
| R-02 | Resolve FB-1 versus upward-rounded charges; exact i128 nano and rates; directional rounding and tolerance including the 6-decimal producer representation; rounding never lowers conservative exposure without supporting settlement evidence | FB-1 reworded around the usage bound; §2 rounding direction; `T_round` integer argument and `ToleranceRange`; MONEY.md | closed |

The other P1.29 findings are dispositioned in `FINDINGS-REGISTER.md`.

**Open owner questions after P1.30:** OQ-1 to OQ-4 are decided in P1.32 (below). The text that follows is historical.

**Open owner questions after P1.30:**
- **OQ-1 (R-04 scope).** Revision 3 treats every open-session or pending commitment as unbounded, not only sessions with
  a request in flight, because R-04's overrun happens after admission on an idle open session. Cost: VC14,
  S08-control-pending-commitment and C1-control-before-reference no longer admit. The alternative, counting an idle
  session's commitment as finite and blocking only on outstanding reservations, admits R-04's counterexample. Confirm
  or choose.
- **OQ-2 (known amounts over the limit).** When an objective is `NotEstablished` but its known amounts already exceed
  the limit (`D1-successor-constrained-by-predecessor-obligations`), revision 3 decides `not_established`, not
  `refused`. Both deny admission. Confirm, or make a certain refusal take precedence.
- **OQ-3 (`QualifiedBound` evidence).** What producer evidence should qualify a bounded commitment (B-1 to B-6
  enforced, served tier verified)? It is needed before any concurrent admission can be established. No S1 evidence
  qualifies.
- **OQ-4 (closure authority).** Revision 3 takes a well-formed `closed_at` no later than the reference time as
  authoritative closure, and a malformed or later value as uncertain (NEW-1). Confirm, or name another authority
  (for example a closure journal row).
- Still open from P1.28: **Q1** (predecessor links and XO-1), **Q3** (nonbillable error class, empty), **Q4**
  (unbudgeted incomplete usage, now also covered by R-01's rule), **Q7** (final line without newline), **Q9**
  (session with no launch row).


**Further owner questions raised by the contract** (P1.28 report §10, historical text; Q2, Q5, Q6 and Q8 are superseded by the tables
above, and Q5 again by P1.30):
- **Q4: unbudgeted incomplete usage** (`terminal` null or failed, no `usd`) is `AtLeast`, blocking. Real records hold 2
  such rows, already flagged by every reader.
- **Q5: missing OpenAI tier** (U-08) is priced over {cheapest, dearest} multipliers instead of being rejected. This is a
  normative repair of a known open defect; confirm it belongs in S2 scope.
- **Q6: identity-refusal reservations** are `AtLeast(max(R, dearest))` (unknown additional exposure) rather than a
  finite range, because the reservation bounds only the pinned model. This is stricter than A1.3.4 (which reported a
  finite high). Both block.
- **Q7: a final line without its newline** (a complete JSON object included) is unreadable evidence, blocking, on every
  lane. Previously only the subscription lane did this; the known U-02 decision is taken this way. Real logs: 0 of 90
  lack a final newline.
- **Q8: unbudgeted API sessions** have no admission record in the current producer, so under D1 their attempts have
  uncertain ownership and block every candidate objective (contract O-6). The alternative is to admit unbudgeted
  sessions through an explicit record (a producer or launcher change). Real records: 134 completed and 2 incomplete
  unbudgeted API request rows (89 logs).
- **Q9: a session with no launch row but an admission record** (R-06). Can stage E treat a log found only by the
  host-directory inventory as evidence, or only as an inventory finding? The corpus encodes this as an ambiguity
  (`R06-launch-row-missing`).

## P1.32 owner dispositions (revision 4)

| Item | Owner disposition (P1.32) | Applied as | Status |
|---|---|---|---|
| OQ-1 (option C) | Strict current producer, qualified future gate. Under release `92a03789` retirement does not establish that a Pi session cannot incur future exposure (`role_host deliver` resumes a retired session without objective admission). Session history is not gated because a session is idle or retired. Outstanding exposure and the ability to resume are separate facts. A declared cap is not a billing bound. A typed `ExecutionGate` for the future vertical must establish authority, identity, scope, effective period and enforcement, separately for new forwarding, retries and reopening. A plain flag, a declaration or a same-UID claim is not evidence | §2.3 (producer fact, `ExecutionGate`, G-1..G-7, capability profiles); L-7; §12.3 A-3..A-5 (`retired_resumable`, gate-dependent release and bound); Table G; A3, A8, A9; corpus profiles `current_producer` / `future_qualified_gate` | closed |
| OQ-2 | Refuse when the supported known components of the conservative measure exceed the limit (owned exposure, known parts of `AtLeast`, valid commitments, attributable unresolved predecessor obligations, the new commitment), without double-counting a reservation and its settlement. Refusal is budget policy, not invoice knowledge. Known ≤ limit with possible unknown excess gives NOT_ESTABLISHED. Ownership or limit contradictions stay uncertain, never speculatively attributed. REFUSED and NOT_ESTABLISHED are equally non-permissive | §12.4 steps 2-4; Table A; A5 (denials equal); A10 | closed |
| OQ-3 | Request-count stops, cost-proxy stops, proxy reservations, commitments, declared caps and actual billing bounds are different mechanisms. A finite billing bound needs evidenced enforcement across forwarding, failures, retries and incomplete settlements. No current declared cap or cost proxy qualifies. No expectation change from renaming | §2.2 (six mechanisms); G-5 (`billing_bound` with `retries`) | closed; no expectation changed by it |
| OQ-4 | Current tooling: a consistent `scope_budget close` entry with matching `closed_at`, valid timestamps, coherent lifecycle and no contradictory closure evidence is operationally established closure under **legacy cooperative trust** (not tamper-resistant; same-UID writable; `set_by: user` proves nothing). Future vertical: closure evidence from its admission authority, never inferred from the legacy format. An established closure refuses new admission; obligations remain and totals may stay provisional. Conflicting or incomplete closure evidence gives NOT_ESTABLISHED | §12.4 step 1 by profile; `ClosureNotEstablished`; A7; corpus closure cases | closed |

**Unchanged:** D1–D10; Q1, Q3, Q4, Q7, Q9 (open); the Q2, Q5, Q6 and Q8 dispositions (P1.29), with Q5 as amended by
R-03/Q5 (P1.30).

**Recorded consequence (OQ-1).** Under the current producer no objective that has admitted a metered session can
establish a new admission. Every such session keeps an `AtLeast` commitment until a qualified gate exists, so the
current-producer admitted controls are only sessionless or Codex cases. Metered positive controls exist only in the
`future_qualified_gate` profile (`corpus/REVISION-4.md`).

## P1.33 owner dispositions (revision 5; P1.32 Module A findings)

| Item | Owner disposition (P1.33) | Applied as | Status |
|---|---|---|---|
| A-01 | A settlement with incomplete or unbounded usage valuation must not reduce or replace the retained reservation's conservative exposure. Keep terminal execution, structural validity, complete priceable usage, replacement eligibility, proxy amounts and independently established billing apart. Preserve supported partial usage and unresolved additional exposure; never make an open-ended amount finite. State which measure a bound belongs to; do not apply `max(R, usage)` across measures | §2.1 monetary measures (six facts; exposure is one measure, conservative provisional exposure); `Replacement` (Eligible / Ineligible{reason}); §8 ineligible usage rows keep R; `ReservationBound`; X-2 restated; F3, F5, F13 | closed |
| A-02 | An error-status settlement may close the attempt while keeping reservation-based exposure (D4), but must not discard supported usage. Keep proxy charge, supported usage and unknown billing apart, with provenance. Determine producer reachability; mark synthetic cases as synthetic | §8 `Valid(ErrorStatus)`: low = usage valuation low, high = AtLeast(max(R, usage high)); the error-status-with-usage case is synthetic, reachability not established (the OpenAI writer takes usage only from a terminal SSE event, and a non-200 status settles as error status) | closed |
| A-03 | A qualified billing-bound gate must establish an authenticated, enforced numerical limit, bound to cap, identity, authority, period, currency and covered activity (retries, partial streams, failures). A declared cap or same-UID assertion does not qualify. A bound above, or inconsistent with, the cap, or without established coverage, does not establish finite exposure. Keep this distinct from S2a | §2.3 `BillingBound` type and G-8; A-2, A-3, A-5 use the bound as ceiling; signature verification is a stage-E obligation of the future vertical (S1 checks the binding digest only) | closed; S1 authentication limit recorded |
| A-06 | Absent or null HTTP status is permitted only for producer-supported shapes that otherwise satisfy the settlement conditions; it is neither an automatic failure nor a universal exemption. Malformed and contradictory status stays invalid. Decide per path | §5 condition 7 table: usage, error-status and reservation request rows need an integer (the producer always writes one); only `upstream_error` rows may lack a status; A06 (null status on a request row) becomes `Rejected[StatusType]` and producer-unreachable | closed |

The other Module A findings (A-04, A-05, A-07, A-08, A-09) are repaired as P2 corrections without owner decisions. See
`CHANGELOG-1.4.md`.

**Unchanged:** D1–D10; Q1, Q3, Q4, Q7, Q9 (open); Q2, Q5, Q6, Q8 (as amended in P1.30); OQ-1 to OQ-4 (P1.32). A-02
refines Q2's error-status row: it adds supported usage to the conservative measure and keeps `AtLeast(R)` as the floor.


## Revision6 explicit D3 amendment and authority

Root01a1d6e2 approved POLICY-AND-PLAN-APPROVAL.md SHA25607303d5c5c36276f6038bf9368f3a5fe660e39ee9160b8b3d92049de98abb981. D3 “No crate in S1” is explicitly amended only for standalone tooling/spec-check Rust structural checker/renderer plus separately authored independent tests; no reference reducer/interpreter/solver/production adapter/workspace integration. P1-P6 owner dispositions in POLICY-R6 supersede contradictory predecessor text. Other historical decisions retain their exact historical standing.
