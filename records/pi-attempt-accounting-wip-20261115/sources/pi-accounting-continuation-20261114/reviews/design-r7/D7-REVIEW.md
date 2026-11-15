**Verdict: LOCAL_DESIGN_REVISION — REVISE_PLAN before implementation.**

Implementation under the retained plan and proposed correction **as presently specified is not recommended**. A bounded correction is credible within D3, but the revised plan must make reference applicability and qualification consumption mechanically explicit. The evidence does **not presently require reopening the financial architecture**, introducing a reducer, or changing monetary policy.

Root `01a1d6e2-32ba-7701-9115-740dcb41af11` retains design disposition, implementation admission and acceptance authority.

Reviewed subject: frozen manifest SHA256 `21df51b69391110beb6952c0eaebfb3c5bf99ac0ef19ce78a225fd24412afb36`. All **941 manifest entries matched**. Retained PLAN SHA256 matched `43d0e7c8df270eb77d9aff32c87e03f5000bc89163da676c8a50daf7affdbe61`.

The recurrence is consequential. R6 already required exact witness application, independent raw assertions, actual Python qualification and consistent wrong-output controls. Adding another statement of those intentions would repeat the repair pattern recorded in the architecture guard. The missing realization is a binding between **the particular claim, its authoritative inputs, the result fields it constrains, and the consumer that establishes it**.

The inspected counterexamples do not refute immutable initial bases, monotone atomization, separate supported-low accounting or affirmative F release. Those mechanisms remain credible at the declaration boundary under the adopted policy. Their general execution and live premises remain unestablished.

1. **D7-01 — P1: Raw-charge preservation is absent from the consuming qualification boundary.**

   This retains **C2-R8-01**, now strengthened by the completed root probes.

   In [validate_corpus.py:451](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/validator/validate_corpus.py:451), `table_x` consumes supplied `settlement.charged`. `raw_settlement_checks` does not establish that this amount preserves the applicable raw charge. Consistent supplied exposure and commitment arithmetic can therefore validate a false premise.

   P9 contains raw reservation **400M**, claimed settlement reservation **500M**, raw charge **700M**, and supported usage **15M**. Root’s four-field mutation reduces supplied charge/high to **500M** and increases commitment from **300M to 500M**. Both full corpus commands exit zero. I checked the exact four-field delta and the custody-record/stdout hashes. This is accepted loss of **200M retained exposure**, not a policy success.

   **Owner and correction:** E preserves raw fields and origins; X owns retention of applicable charge components; R consumes the qualified high; the independent oracle owner establishes the bounded raw-to-result assertion. Neither supplied rollups nor a raw-input digest establishes the charge’s semantic preservation. Bind the named assertion to the exact raw `usd.charged_usd`, applicable origin, expected retained component and consuming result fields.

   **Falsifier `P9-RETAINED-CHARGE`:** unchanged raw P9 with the four consistent wrong fields must fail the actual financial qualification entrypoint. Also challenge omission or alteration of related supplied component records. A failure caused only by an unrelated snapshot digest does not establish monetary checking.

   Root’s proposal needs one distinction: Rust `corpus` advertises finite supplied-reference checking. Its acceptance is evidence that the **composed qualification** misses P9; it does not, by itself, require that structural command to acquire financial-reducer semantics. The revised plan must name where Rust’s independently required finite monetary assertion is actually consumed.

2. **D7-02 — P2: Reference existence is being consumed as release applicability.**

   This retains **A1-R8-01 and C2-R8-03**.

   [accounting.rs:197](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/src/accounting.rs:197) checks cardinality, reference existence and several ownership relations, but does not connect a selected primary-validity witness’s evidence to the particular primary origins being released. Its anchor check accepts any result link incident to the anchor, independently of the coverage’s cited links and the other endpoint. Python’s reference consumer has corresponding gaps.

   The supplied reports establish acceptance of:

   - VC11 primary evidence exchanged between obligations;
   - coverage redirected from a rejected obligation to another valid witness;
   - VC07 anchor release with an **a→a** link;
   - anchor release after removing the coverage’s cited link.

   These are supplied-record coherence defects. Detecting them needs no arbitrary raw reducer.

   **Owner and correction:** C-primary owns exhaustive logical-obligation membership under admitted evidence equivalence; C-validity owns the witness for that membership; C exposes admitted exact links; F owns their application to the released origins.

   The revised contract must specify:

   - how a witness identifies the complete primary evidence of its obligation, including equivalent-origin membership;
   - how that membership matches the primary origins released by coverage;
   - how every released anchor is connected, through the coverage’s cited admitted relation, to the exact covered request/primary obligation;
   - how valuation usage origins apply to that same coverage.

   Existing origin, witness and coverage fields can reject the demonstrated substitutions. If they cannot express exhaustive obligation membership and origin roles unambiguously, add the **minimal C-owned membership relation**, or define its unique derivation from existing records. Do not infer it from an opaque ID’s spelling, and do not place a chosen financial decision in `AccountingInputProof`.

   A grant identifier alone is insufficient as a link identity if it can authorize multiple endpoint pairs; applicability must bind the exact pair or an unambiguous referenced relation.

   **Falsifiers `VC11-WITNESS-SWAP`, `VC11-REDIRECTED-VALIDITY`, `VC07-SELF-LINK`, `VC07-UNCITED-LINK`:** reject each mutation through the actual supplied-bundle consumer and applicable corpus-reference consumers. Preserve the exact-link positive. A mere non-self-link check is insufficient: a link to another request must also fail.

3. **D7-03 — P2: Malformed identity still disappears before independent qualification.**

   This retains **A-07**, **A1-R8-02**, **A2a-R8-01** and **C2-R8-02**.

   [independent_raw_axes.rs:106](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/tests/independent_raw_axes.rs:106) discards non-string identity values during extraction. `price_range` consequently retains complete **15M** pricing, and `accepts_usage_axis` accepts the unchanged eligible expectation.

   Root’s completed model7 probe confirms this execution. The probe source preserves the frozen source as an unchanged prefix and appends the observation test. Its successful test result means **the defect reproduced**.

   The F2 literal control changes a simplified request **`model`**, while the reported counterexample changes the actual raw **`response_model`**. `test_r8_assertions.py` compares supplied records against literals; it does not establish that the defective identity extractor rejects this path.

   **Owner and correction:** E preserves presence/type state; V owns identity establishment and completeness; the oracle owns independent qualification of those semantics. Preserve malformed evidence before collecting canonical identities. Malformed identity prevents complete replacement while independently supported priced highs and conservative reservations remain retained. Preserve approved fully priced multiidentity bounds.

   **Falsifier `RESPONSE-MODEL7-ACTUAL`:** the actual VC01/A04 raw response-model mutation must reject the unchanged complete eligible result through the retained Rust axis and applicable Python checks. Test the repaired positive separately. A new simplified literal, or deleting the helper without an equivalent consumed replacement, cannot close A-07.

4. **D7-04 — P2: Evidence uniqueness remains conflated with amount uniqueness.**

   This retains **A-08 and A1-R8-03**.

   In [independent_raw_axes.rs:269](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/tests/independent_raw_axes.rs:269), the report sets `comparison_applicable` from `nums.len()==1`. Two distinct reservations with the same amount can therefore be conflicting evidence while still authorizing reservation-relative comparisons.

   **Owner and correction:** C-validity owns `UniqueReservation`, using admitted evidence equivalence. Reservation maximum is a separate arithmetic fact. The independent qualifier must preserve that distinction in both assertions and reported standing.

   **Falsifier `EQUAL-AMOUNT-DISTINCT-RESERVATIONS`:** timestamp-distinct 500M reservations must make reservation-relative comparisons inapplicable; same-origin reobservation and admitted exact copies must retain their specified idempotence. Amount equality cannot decide either result.

**Required revised-plan boundary**

Root’s correction is directionally sound, but should become a concrete amendment to the retained plan:

- Bind each retained finite claim to its raw subject/origin fields, policy owner, constrained result fields, consuming entrypoint, positive control and consistent wrong-output control. Extend existing assertion/dependency records; no new qualification framework is necessary.
- Distinguish schema validity, supplied-reference coherence, exact raw-field preservation and monetary correctness in consumer results. Passing one must not discharge another.
- Specify the C→F membership/link application contract above. Keep structural enforcement with the specification consumer and expected financial values with the independent oracle owner.
- If retiring a helper, preserve its finding history and replace its **claim at the same consumption boundary**, including actual consumer registration and invalidation. Retiring an executable is not retirement of the acceptance obligation.
- Preserve required corpus, revision, selftest, sabotage, generated-output and exact-current modular review obligations. Their passing status supplements the named falsifiers; it cannot substitute for them.

These changes fit D3: supplied-set coherence and independently authored finite raw assertions do not require computing arbitrary C/V/F/X/T/R results. If satisfying a retained claim instead requires a general correlator, financial reducer or partition solver, report that incompatibility to root. Do not silently narrow the claim.

Another serious recurrence after this correction remains architecture-guard evidence. This review neither resets history nor converts interrupted reviews into rejection counts.

**Coverage and retained uncertainty**

I read the complete proposed boundary, retained PLAN, R5/R6 policies, original P1–P6/D3, previous implementation report, full architecture guard, and current A1/A2a/C2 reports. I inspected the three requested sources, targeted Rust corpus/control paths, relevant accounting schema/declarations, P9/VC11/VC07/F2 records, and both completed root-probe records.

I independently verified the frozen manifest entries and relevant probe custody/stdout hashes. I did **not** rerun tests, compile code, audit the whole 175-case corpus, or establish independent authorship history. A1’s supplied-bundle executions remain attributed review evidence supported by source inspection.

A2a’s **P3 stale qualification-count finding** remains for reconciliation; I do not close it. Earlier findings remain open except for their already stated bounded dispositions. In particular, restored local lows do not clear unrelated declaration, attribution, schema or dependency obligations. No additional financial/attribution/schema reviews were supplied for integration, and **no all-A/B/C clearance is claimed**.

Full arbitrary raw conformance, executable confluence, live provenance/authority/currentness, predecessor accounting continuity and S2a readiness remain unestablished.

Graph Tier 2 observation was generation `2026-11-14T05:31:36Z`, store `u4190552c44efc473g2`. External source coverage was `outside_project/unavailable`; conclusions use exact-source fallback, with no graph-completeness claim.

**Identity and disposition**

Acknowledged verified reviewer SID **`01a1d88a-9a44-71e1-9dc5-b558108f35de`**, rollout `/home/bline/.codex/sessions/2026/11/13/rollout-2026-11-13T22-32-05-01a1d88a-9a44-71e1-9dc5-b558108f35de.jsonl`, and collector **`01a1d6e2-32ba-7701-9115-740dcb41af11`**, from `control/D7-IDENTITY-BINDING.md`.

Recorded applied settings: **gpt-6-astra / xhigh / default**, read-only, approval never, restricted network. Backend fulfillment and cost are not established by those settings. Latest context observation: **127,215 input tokens, epoch 0**, below the user’s preparation boundary; no refresh occurred.

No files modified, implementation code produced, children delegated, providers launched or settings changed. All read-only processes terminated; **no owned process or child remains outstanding**. This native final is returned to root for sealing and disposition.
