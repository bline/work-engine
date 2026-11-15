**CHANGES_REQUIRED — bounded A2a review.** All 53 assigned current financial expectations and their 112 low-scope assertions were examined. I found no additional incorrect monetary expectation in those cases, but inherited finding A-07 remains unresolved in shared qualification code.

- **A2a-R8-01 — P2, inherited A-07: malformed response identity still passes the raw-arithmetic helper.**  
  [independent_raw_axes.rs:106](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/tests/independent_raw_axes.rs:106) ignores non-string `response_model` values. `price_range` at line 133 retains the valid launch/request identity, and `accepts_usage_axis` at line 895 accepts the resulting complete valuation.

  **Counterexample:** Take A04’s valid request and replace only `response_model` with numeric `7`. The helper still derives `(15000000, 15000000, true)` and accepts the unchanged eligible expectation. The owning `VALUATION/V-1` declaration instead makes a malformed identity unpriceable.

  The new F2 control compares supplied literal records through [test_r8_assertions.py:5](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/validator/test_r8_assertions.py:5); it does not exercise or repair this helper. This is a source-level counterexample, not an executed Rust mutant or demonstrated whole-gate bypass. Integrate with A1/C2’s disposition of A-07 before acceptance.

- **A2a-R8-02 — P3, origin unestablished: stale qualification counts.**  
  [R8-RAW-DERIVATIONS.md:5](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/oracle/R8-RAW-DERIVATIONS.md:5) still says 24 assertions and 350 unmet scopes, whereas the frozen assertion inventory and verified joined output report 374 assertions and zero missing named scopes. C1/C2 should reconcile this documentation.

Prior findings have these bounded dispositions:

| Finding | A2a disposition |
|---|---|
| A-02 | A06 repaired: independently derived 15M survives locally and in owned outstanding exposure; 250M remains held. Other affected cases require A1/A2b. |
| A-03 | Assigned subscription refusal case correctly retains unresolved PiApi `AtLeast0`; total applicability remains C1’s obligation. |
| A-04 | Inspected current financial wire fields agree; broader projection/schema clearance remains C1/C2’s. |
| A-07 | Still open, as detailed above. |
| A-01, A-05, A-06, A-08, A-09 | Not closed by this review. |

The examined financial distinctions include genuine unbudgeted completion, unresolved auxiliary anchors, conditional byte-copy equivalence, retained identity contexts, local versus aggregate lows, and exact versus unresolved highs. N06 retains 6M–17.5M; N01 retains the priced 20M–80M range; duplicate-response observations retain aggregate low 15M without physical independence evidence. ORD, session-file, cache-split and N01 internal twins were preserved.

Cross-module integration remains necessary for referenced controls outside A2a: VC01–VC08 where referenced, VC11, VC12, VC15, S02, U04, U08 and V01-control. This report does not clear their complete financial or attribution obligations.

The subject is manifest `21df51b69391110beb6952c0eaebfb3c5bf99ac0ef19ce78a225fd24412afb36`, with A2a projection `ec50df86c7b8bf2c7002ba776ee4ae65e9d0fdb97f9753b95083d348e5f22470`. All 941 subject-file hashes and projection bindings were verified. Review covered the assigned raw inputs, current expected financial fields, evaluation money, pinned rates, applicable declarations, linked financial premises and selected shared finite controls. Historical narratives/revisions, exhaustive reference closure, independent authorship provenance and the other 122 cases were not fully reviewed.

I inspected the six joined custody records and verified their stdout hashes, summary hash, validator hash and tested binary hash. They record successful completion without leftover processes. I did not rerun the suite, compile code or execute the malformed-identity mutant. Postgate metadata drift remains disclosed; full raw conformance, arbitrary confluence, live authority, predecessor accounting continuity and S2a readiness remain unestablished. Graph coverage reported the external evidence paths as outside the indexed project; conclusions use exact source reads.

Identity binding acknowledged: reviewer `01a1d87b-01de-7631-9623-0ab1026eba5d`; collector `01a1d6e2-32ba-7701-9115-740dcb41af11`. The post-refresh applied record reports `gpt-6-astra / xhigh / default`; backend fulfillment and cost are unknown. One same-session context refresh occurred; continuation was recovered from the saved review state and native transcript. No files were modified, no children were launched, and no processes remain owned. Root retains acceptance and architectural-guard disposition.
