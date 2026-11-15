# Change log: `pi-accounting-contract/1.2-s1` → `1.3-s1` (revision 4, P1.32)

Revision 3 is preserved byte-for-byte (`~/cbm-work/p130-s1r3/s1`, manifest `e278f66c…`). The complete textual diff
of contract, corpus and validator is `evidence/s1-rev3-to-rev4.complete.diff` in the P1.32 workspace. Each change
below names the owner decision behind it. D1–D10, Q1, Q3, Q4, Q7, Q9 and the Q2/Q5/Q6/Q8 dispositions are unchanged.

## CONTRACT.md

| Section | Change | Decision |
|---|---|---|
| header | version `1.3-s1`; status | — |
| §2.1 consequence | every admitted current-producer session, retired or not, keeps an `AtLeast` commitment; only a qualified `ExecutionGate` releases or bounds | OQ-1 |
| §2.2 | renamed "Spending mechanisms are kept apart"; six mechanisms (request-count stop, cost-proxy stop, proxy reservation constraint, internal commitment, unenforced declared cap, enforced billing bound); renaming changes no expectation | OQ-3 |
| §2.3 (new) | producer fact (retired Pi sessions resumable without admission, with line references); `ExecutionGate` type; rules G-1..G-7 (authority, identity, scope, period, mechanisms separately for forwarding, retries and reopening, receipts, consistency); capability profiles | OQ-1 |
| §9 | `UnenforcedCommitment` covers retired-resumable; new blocking `GateContradicted` | OQ-1 |
| §10 L-7 (new) | `Retired{at}` is a fact, not a gate | OQ-1 |
| §11.2 P-1 | closed window only when closure is established | OQ-4 |
| §12.1 | reason `ClosureUncertain` → `ClosureNotEstablished` (claimed but not established); `GateContradicted`; Established condition reworded | OQ-4, OQ-1 |
| §12.2 | open commitments include `retired_resumable`; "established closure" wording | OQ-1, OQ-4 |
| §12.3 | `Commitment` gains `gate` and state `retired_resumable`; A-2 (released: nothing can resume), A-3 (`max(0, cap − owned high)`; only a qualified release gate retires), A-4 (a reopening never releases), A-5 (gate-dependent kind, `NoGateEvidence` for the current producer) | OQ-1, OQ-3 |
| §12.4 | order: closure (by profile: legacy cooperative close entry, or vertical authority record) → limit → **known over-limit refusal** → accounting → headroom; `refused` and `not_established` equally non-permissive | OQ-2, OQ-4 |

## Other normative documents

- **TRANSITIONS.md:**
  - Table L notes (retirement is not a gate);
  - Table R wording;
  - Table A rewritten (closure, limit, known total);
  - Table G added (session commitments by gate);
  - Table P open commitments.
- **INVARIANTS.md:** A1, A3, A5, A7 and A8 restated; A9 (gate qualification) and A10 (known over-limit refusal) added.
- **DECISION-REGISTER.md:**
  - P1.32 dispositions table;
  - the P1.30 R-04/Q10 and Q11 rows marked amended;
  - the OQ list marked historical;
  - the recorded OQ-1 consequence.
- **RESERVATION-BOUNDS.md:** caps paragraph (retired sessions resume under the cap; `billing_bound` evidence across
  forwarding, retries, failures, incomplete streams and settlement).
- **S2A-ENTRY-CRITERIA.md:**
  - contract version;
  - revision 4 changes no S2a stage;
  - gate 2 updated;
  - the subset rule restricted to `current_producer`.
- **FINDINGS-REGISTER.md:** a historical note only.
- **MONEY.md:** unchanged.

## Corpus and validator

- **Schema:** `profile`, class `future_vertical`, `revisions_r4`, `inputs.gates`, objective `close`,
  `admission_authority`, `closure_record`, admission `gate_id`, commitment `gate`, state `retired_resumable`.
- **Revision-3 cases (121):**
  - all become `current_producer`;
  - every changed field is logged in `revisions_r4`;
  - inputs and historical blocks are untouched;
  - 91 admission decisions change, all derived in `corpus/REVISION-4.md`.
- **Added (36):** 29 `future_qualified_gate` (14 admitted positives; gate negatives; closure by authority and by a
  legacy entry only; a permitted-release pair) and 7 `current_producer` (closure entries, known over the limit and
  unknown below it, a closed predecessor with outstanding exposure, a retired session resumed without admission).
- **Validator:**
  - profile rules;
  - gate qualification from raw gate evidence (G-1..G-7);
  - Table G commitment derivation;
  - closure by profile;
  - the §12.4 order with known over-limit refusal;
  - A5 with denials equal and permitted releases requiring a qualified gate;
  - per-profile control minimums (current ≥ 1, future ≥ 11 metered, total ≥ 15 as before);
  - a revision-4 log check;
  - the S2a subset limited to current producer.
- **Negative tests:**
  - self-test: 29 defects (24 carried, 5 new);
  - sabotage: 76 (all 61 carried by name, plus 15 new). Where an expectation legitimately changed, 3 carried sabotages
    were retargeted to another case (R-09 to `P132-CP-closed-valid-close-entry`, the R-11 non-retirement row to
    `P132-FG-VC01-gated`, the historical-rationale check to VC08). 3 had their expected message updated (R-04 and the
    contradicted-lifecycle release now fail on Table G; Q11 on `ClosureNotEstablished`). The revision-log sabotage now
    drops a revision-4 entry.
