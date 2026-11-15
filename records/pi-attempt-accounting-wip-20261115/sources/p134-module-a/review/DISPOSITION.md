# P1.34 Module A disposition (written after sealing; independent and coordinator judgments kept apart)

**Sealed:** response `2b22303081931e7486a64960419ca6706581d4c93fb9e9ee60704d32ab65bd67`, SEAL
`d515e5f3b76a3439ba3b974aae450a609f26695ec47cb5478245ebe32d531583`, sealed 2026-11-13T17:03:45Z, before any comparison.

**Reproduction:** `repro/repro_p134.py` (sha256 `8f45d9a7…`) against the frozen revision 5 (`c2017453…`) and release
`92a03789`. All 9 reproduced (`repro-output.txt`, `2d8a802d…`). Revision 5 was not modified.

## Independent judgment (the reviewer's, unchanged)

- **Financial contract:** REJECT. **S2a financial readiness:** NOT_READY.
- **Architecture assessment:** `local_corrections`: "replacing the overall architecture is not justified".
- **Architectural challenges:**

  | Challenge | Result |
  |---|---|
  | Exposure versus billing | sound. The name `Billing::Established` is "hazardous", but its stated semantics do not establish provider billing |
  | Authenticated bounds | defect (A-03) |
  | Conflicting reservations | sound: the maximum with blocking, distinct rids kept separate, no double counting |

- **Examination:**

  | Result | Areas |
  |---|---|
  | sound | 5 (exposure versus billing), 10 (independence from diagnostics and time), 11 (refusal total), 12 (profiles) |
  | not established | 6 (full-range floating-point tolerance proof) |
  | defect | the other 7 |

- **Derivations:** 24, of which 15 agree and 9 disagree. In every one the money agrees with §8; all 9
  disagreements concern classification, reachability or an unspecified rule.

## Findings, reproduction and coordinator view

| ID | Sev | Class | Finding | Reproduced | Coordinator view |
|---|---|---|---|---|---|
| A-01 | P1 | local | §6.2 (CONTRACT.md:431–435) prescribes `low 0, AtLeast(R)` for every error-status attempt. §8 (:512) keeps supported usage. Following §6.2 lowers P133-A02 from 1.0/AtLeast(1.0) to 0/AtLeast(0.8) | yes | Agree. **Regression caused by the P1.33 repair:** revision 4 had §6.2 :363 and §8 :441 agreeing; P1.33 changed §8 only |
| A-02 | P1 | local | N03: launch `anthropic/claude-fable-5-2`, row provider `openai`, no OpenAI rate. The expected result is Eligible and Exact(28M), which releases R = 300M | yes; N03 is in the S2a grading subset | Agree, and stronger. Under `mc-1` (MC-2/MC-3) the identities differ, so §5 condition 8 gives `Rejected(ModelMismatch)`, not only ineligibility. TRANSITIONS.md:105 ("as above, plus LaneContradiction") contradicts MC-2/MC-3. The expectation is inherited from revision 4; the P1.33 rule was not propagated, because the validator's raw model check compares the id only (validate_corpus.py:509) |
| A-03 | P1 | local | G-1 to G-8 have no authenticated-verification prerequisite; the synthetic positives admit on assertions and hashes | yes: forged receipts (sha256 of arbitrary text) still pass the full validator check and admit | Agree. It blocks later gate integration, not the gate-free C–K core. This was predicted in the pre-registration (item 4) |
| A-04 | P2 | local | `amount_source` is built by S from replacement, which needs the valuation range S may not read (TRANSITIONS.md:25). `Outstanding` is a unit variant yet carries `amount_source` | yes | Agree |
| A-05 | P2 | local | An all-null usage object maps to `NotReported` with no stated rule (S02-control, P133-A06-error-status-null) | yes | Agree |
| A-06 | P2 | local | Null-terminal rows with non-null model, tier or usage cannot be written: `response = {}` (pi_container.py:1549). This affects VC03 and S02-control (labelled reachable) and P133-A02. Non-200 bodies are parsed for terminal events (:1531–1543, :1569), so error-status-with-usage is writer-reachable | yes | Agree. It corrects my P1.33 reachability argument for A-02 (see the correction below) |
| A-07 | P2 | local | P133-A01's reservation of 0.01 is impossible: the reserve floor is ≥ 0.0375 for input bound 6000 (:689–694). The case is labelled `producer_reachable` | yes | Agree |
| A-08 | P2 | local | P133-A04-cached-malformed omits `MalformedEvidence`, which CONTRACT.md:574–575 requires | yes | Agree (or narrow the rule explicitly) |
| A-09 | P2 | local | §5 condition 4 ("the reservation's amount") has no unique referent under conflicting reservations, so "every failing reason" is undetermined | yes | Agree |

**Derivation discrepancies not raised as findings, verified:**
- `P133-A04-cache-counts-both-absent` is `producer_unreachable` for `92a03789`, not `not_established`: the writer
  always emits both cache keys (:1562–1563).
- `P133-A05-model-case-variant` is `producer_unreachable`: `check_request` refuses a non-pinned model before any
  reservation (:533–534, :1392–1395).

## Corrections to P1.33 claims (CLAUDE.md rule 8)

1. **Consistency claim.**
   - **Wrong claim:** "Consistency: 17-phrase stale search … all eight required properties consistent" (P1.33 §5).
   - **Error class:** an absence claim without a completeness basis. The search listed stale phrases rather than
     enumerating every statement of the changed rule.
   - **Consequence:** CONTRACT.md §6.2 kept the old error-status formula. That is a P1 that lowers conservative
     exposure in the normative text, and it blocks S2a.
   - **Check that would have caught it:** enumerate every site stating error-status exposure (`ErrorStatus`,
     `error-status`, `Error-status`) and compare each with the new §8 row.
2. **A-05 model-rule claim.**
   - **Wrong claim:** A-05 repaired, with a "raw canonical model check".
   - **Error class:** a rule added without re-deriving existing cases. The validator's raw check implements a weaker
     rule (id only) than the contract (family and id).
   - **Consequence:** N03, in the S2a grading subset, releases a 300M reservation on a contradicted identity (P1).
   - **Check that would have caught it:** derive MC-1 to MC-3 identities, including the family, for every case.
3. **A-02 reachability argument.**
   - **Wrong claim:** "the producer takes usage only from a terminal SSE event and settles any non-200 as error
     status, so such a row is not established as producer-writable."
   - **Error class:** a wrong inference. Those two facts imply the writer can produce the row whenever the upstream
     streams a terminal event with a non-200 status.
   - **Consequence:** wrong reachability label (P2). The P133-A02 row as written is also unwritable (null terminal
     with usage).
   - **Check that would have caught it:** trace `do_POST` for `resp.status != 200` with a terminal event.
4. **P133-A01 reachability.**
   - **Wrong claim:** `producer_reachable`.
   - **Error class:** the fixture was not checked against `reserve_usd`.
   - **Consequence:** a positive reachability claim the producer cannot satisfy (P2).
   - **Check that would have caught it:** recompute the reserve floor from the input bound and the rate file.

## Comparison with the P1.32 findings (independent judgments of two reviews; no merging)

| P1.32 finding | Status in the P1.34 review |
|---|---|
| A-01 incomplete usage lowers R | Area 3: "normally retains R"; residual in N03 (P1.34 A-02) |
| A-02 error status ignores valuation | §8 now keeps usage; residual §6.2 contradiction (P1.34 A-01) |
| A-03 billing bound has no amount | The amount is now present; next layer: authentication (P1.34 A-03) |
| A-04 valuation totality | The cache states work (P133-A04 derivations agree); residuals: all-null mapping (A-05), MalformedEvidence (A-08) |
| A-05 model prefix | `mc-1` present; not propagated to N03 (A-02) or to reachability (A05 label) |
| A-06 status | Area 9: path rules "usable"; labels wrong (A-06) |
| A-07 reachability label | More reachability labels wrong (A-06, A-07, two derivation labels) |
| A-08 `amount_source` enum | Typed, but conflicts with the stage dependencies (A-04) |
| A-09 C-1 versus Table S1 | X-0 sound; condition 4 is undefined for multiple reservations (A-09) |

**Coordinator observation, a hypothesis and not the reviewer's:** in 8 of 9 cases the P1.34 finding sits at the edge of
a P1.32 repair. A rule changed in one place was not carried to its other statements, or to the corpus, the validator or
the reachability labels. That pattern points at the representation itself: the same rule stated in prose, in tables
and in a hand-built corpus, all from one author. That is the question for the architectural review. Against the
hypothesis, the reviewer judged the financial architecture sound and the defects local.
