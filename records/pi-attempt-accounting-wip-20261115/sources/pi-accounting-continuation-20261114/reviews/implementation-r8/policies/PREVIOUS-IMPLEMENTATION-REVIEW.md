{"module":"A","verdict":"CHANGES_REQUIRED","module_complete":true,"s2a_readiness":"NOT_READY","p1_count":2,"p2_count":7,"p1_touching_CK":2,"architecture_assessment":"design_review_required","limits":["All 83 file boundaries and the end sentinel are present; hashes and byte counts were not independently verified.","Read-only review without tools or test execution.","Only 50/175 cases are supplied; evaluations, complete frozen audit facts, inventory entries and predecessor bytes are omitted.","Legacy validator implementation and complete sabotage output are unavailable; reported passes do not establish semantic correctness.","Raw-axis qualification is not complete raw-to-output conformance.","Trusted-fixture positives establish conditional mock policy only; live verification and enforcement remain unestablished."]}

## Findings

### A-01 — P1: Adding a reservation can halve retained aggregate exposure

**Location:** `spec/ck.json:2`, `CORRELATION C-2`, `X-BRANCH-022`, `X-BRANCH-027`, `X-COMPONENTS`, `INV-F2`, `INV-F3`.

**Counterexample:** Begin with two distinct request settlements in one source, both naming `usd.rid=1`, each claiming a well-formed 500M charge, with reservation basis and no reported usage. No reservation row exists. C-2 assigns separate `Row` keys; each rejected orphan contributes `AtLeast(500M)`, totaling `AtLeast(1000M)`.

Append a 500M reservation with that rid. C-2 now joins both settlements into one reservation group. Their decision is `Conflicting`; X-27 takes `max(R, claimed charges)=500M`. Aggregate exposure becomes `AtLeast(500M)`.

The added evidence is not an eligible usage settlement. This violates I-1 and INV-F3 despite every individual X branch retaining its available reservation.

**Consequence:** Per-tuple declaration coverage does not protect exposure across correlation changes. The C identity/merge-to-X aggregation boundary needs design review.

**Origin:** Unestablished; demonstrated directly from current rules.

### A-02 — P1: Qualified expectations erase supported usage contrary to X’s owning result

**Location:** `spec/ck.json:2`, `X-BRANCH-025`; `cases/A06-reservation-usage-null-status-no-id.json:2`, `cases/P133-A09-conflicting-reservations-with-settlement.json:2`, `cases/P130-R05-conflicting-keeps-valuation-high.json:2`, `cases/V01-cheaper-response-model-undercharge.json:2`.

**Counterexamples:** X-25 requires `low=usage_low` for rejected/conflicting settlements with valued usage. Nevertheless:

- A06 and P133-A09 record valuation low 15M but exposure low zero.
- P130-R05 records valuation low 20M but exposure low zero.
- V01-undercharge records valuation low 60M but exposure low zero.

For A06, independent arithmetic is `1000×10000 + 100×50000 = 15M` nano.

`tooling/tests/independent_raw_axes.rs:429–447` compares valuation fields, not their exposure-low projection; the reported joined check therefore leaves this disagreement intact.

**Consequence:** Following the canonical branch fails the supplied expected result; matching the expectation discards supported financial information. Retaining the high does not reconcile these authorities.

**Origin:** Unestablished introduction; current specification/corpus disagreement is explicit.

### A-03 — P2: Lawful orphan evidence cannot reach an X result

**Location:** `spec/ck.json:2`, `CORRELATION C-7`, `S-zero-zero`, `V-PRESENCE-INTERFACE`, `X-BRANCH-005`, `X-BRANCH-016`, `INV-F9`.

**Counterexample:** A subscription-launched source contains only an Anthropic `response_model_refused` row, with no reservation, settlement or usage. C-7 produces an orphan Row. S-zero-zero supplies `not_applicable`; absent usage produces `not_reported`.

Keeping the subscription dimension hits excluded X-16. Reclassifying the contradicted lane as PiApi hits excluded X-5. Forcing valuation `not_applicable` instead reaches clean Codex, contrary to lane-contradiction policy.

**Consequence:** Explicit subscription-plus-reservation branches do not establish total applicability. This zero-known-component orphan requires an unresolved PiApi result, but reaching one requires an undeclared override of S.

**Origin:** New R7 exclusion/witness-interface interaction.

### A-04 — P2: “Total projections” still do not describe the external representations

**Location:** `spec/ck.json:2`, `PROJECT-Exposure-pi_api`, `PROJECT-High-exact`, `PROJECT-Valuation-valued`; `spec/interface.json:2`; `tooling/src/decisions.rs:329–336,442–467`; `cases/U08-missing-observed-tier.json:2`.

**Failure conditions:**

- Exposure’s declared wire discriminator is `variant`; corpus exposure uses `pool`.
- High declares `variant` plus nested `bound`; corpus high uses `kind`, `nano`, `basis`.
- Valuation projects Bound-valued lows/highs and `splits`; corpus uses amount strings and `derivations`.

No declared transformation resolves these differences; projection validation requires identical internal field names.

Additionally, U08 still emits `TierUnestablished`, while the interface permits only the qualified Missing/Unrecognised codes.

**Consequence:** Internal constructor coverage is not total external projection coverage. Correct corpus values cannot be mapped by the supplied declarations alone.

**Origin:** Inherited interface-agreement failure, including the previously identified U08 code; the insufficient projection realization is new in R7.

### A-05 — P2: S’s supposedly immutable witness projection remains independently editable

**Location:** `spec/ck.json:2`, `SETTLEMENT-WITNESS`, `S-unique-one-pass`, `S-unique-one-fail`; `tooling/src/lib.rs:534–551,576–586`.

**Executable failure condition:** Replace `S-unique-one-fail.outputs` with the output constructor from `S-unique-one-pass`. Keep its failing-check guard unchanged. Constructor fields and partition coverage remain valid; neither `validate` nor wire-interface checking verifies that this branch projects a rejected witness rather than `Valid`.

**Consequence:** The textual C-validity owner and S’s result table can disagree while declaration checks pass. This preserves the synchronization failure class at an expressly protected causal boundary—not merely an unimplemented raw reducer feature.

**Origin:** New R7 witness-projection realization.

### A-06 — P2: Semantic changes still bypass affected consumer invalidation

**Location:** `spec/dependencies.json:2`; `spec/ck.json:2`, `USAGE-PRESENCE`, `VALUATION`, stage `V`; `tooling/src/lib.rs:160–233,787–822,1060–1093`.

**Executable failure condition:** Change `USAGE-PRESENCE`’s completeness policy and compare consumer digests or call `affected_consumers` with that node. `view-contract` and `consumer-R` are not reached: V’s stage edge points to C, not its valuation predicates, and those consumers lack another relevant dependency path.

Yet that predicate changes replacement, billing, predecessor exposure and financial provisionality.

**Consequence:** Whole-file generated-byte checking detects different bytes, but registered consumer acknowledgements need not become stale. Current null acknowledgements are not approvals; the invalidation mechanism remains insufficient.

**Origin:** Inherited dependency-completeness failure class; exact edge introduction unestablished.

### A-07 — P2: Independent arithmetic silently ignores malformed model evidence

**Location:** `tooling/tests/independent_raw_axes.rs:106–120,151–162,245–249,895–905`; `spec/ck.json:2`, `VALUATION V-1`.

**Executable failure condition:** Take VC01’s request and replace `response_model` with numeric `7`. `identities` silently skips it, retaining the valid launch/request identity. `price_range` still returns `(15000000,15000000,true)`, and `accepts_usage_axis` accepts the unchanged eligible expectation.

V-1 instead makes a malformed model name an unpriceable identity.

**Consequence:** This is a false positive on the oracle’s declared identity/completeness axis, potentially supporting replacement without an established identity set. It is not an objection to approved fully priced multi-identity bounds, nor a demonstrated full-gate bypass.

**Origin:** Unestablished.

### A-08 — P2: Unique-reservation applicability is reported from unique amounts

**Location:** `tooling/tests/independent_raw_axes.rs:342–359`; `spec/ck.json:2`, `CORRELATION C-1`, `SETTLEMENT-WITNESS`; `root/POLICY-AND-PLAN-APPROVAL.md:13`.

**Counterexample:** Two reservation rows share rid and amount 500M but differ in timestamp. They are distinct conflicting reservations. Their distinct-amount list nevertheless has length one.

The report consequently emits both `conflict=true` and `comparison_applicable=true`, because applicability uses `nums.len()==1`.

**Consequence:** An axis labelled established authorizes reservation-relative comparisons without the required exactly-one-distinct-reservation witness. Arithmetic uniqueness is not evidence uniqueness.

**Origin:** Unestablished.

### A-09 — P2: The published JSON schema rejects the canonical specification

**Location:** `spec/ck.schema.json:2`, `properties.boundaries`, `$defs.Stage.properties.id`; `spec/ck.json:2`, `boundaries`; `tooling/src/lib.rs:125–131`.

**Executable failure condition:** Validate `spec/ck.json` with a Draft 2020-12 validator. Boundaries reuse `Stage`, whose identifier enum permits only E/C/S/V/F/X/K. Consequently `C-primary`, `C-validity`, `C-auxiliary` and `S-consume` fail schema validation.

The Rust loader compares schema bytes and deserializes into String-valued identifiers; it does not enforce that schema.

**Consequence:** Standard schema consumers and the qualification tool disagree about the canonical declaration’s validity.

**Origin:** New R7 boundary-schema defect.

## Coverage, design implications and uncertainty

The delivered packet is complete, but its qualification evidence is deliberately bounded. Missing evaluations prevent independent checking of objective rollups and mock-gate residual outputs; omitted validator source prevents assessing its actual semantic checks.

The architecture does not yet prevent the ownership failure class. Result-bearing X branches constrain component selection **after classification**, but cannot repair changing correlation identities, unreachable input combinations, or independently maintained expected projections. A-01 demonstrates an I-1 failure beyond the eight previously identified examples.

Retain the financial distinctions. Revisit two boundaries: **C identity/merge accounting versus aggregate exposure**, and **owned declarations versus external/corpus qualification**. Stable provisional correlation identities or explicit merge accounting compete with the current Row-to-Reservation rekeying under max-and-sum arithmetic. Explicit external projections and independent boundary assertions compete with internal-constructor agreement plus disconnected expected outputs. Neither issue calls for a disguised reference reducer.

Status/terminal completeness, reservation-relative applicability, error usage, unknown-cache extrema and multi-identity bounds were examined. The findings distinguish malformed/unestablished identity from the permitted complete priced identity set. Supported arithmetic examples do not establish writer reachability, all combinations, full classifier correctness or live authority.

The raw audit’s pre-implementation freeze remains useful evidence for its stated axes only. Current tests and shared serialization mechanics do not establish complete semantic independence or full raw financial conformance.