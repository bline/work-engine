{"module":"A","verdict":"CHANGES_REQUIRED","module_complete":true,"s2a_readiness":"NOT_READY","p1_count":1,"p2_count":5,"p1_touching_CK":1,"architecture_assessment":"bounded_correction","limits":["All 73 file boundaries and the end sentinel are present; hashes were not independently verified.","Read-only review; no commands or tests executed.","Full tooling source, complete corpus, predecessor inventory and projected-out evaluations were unavailable as declared.","Raw-axis qualification does not establish full correlation, aggregation or classifier correctness.","Trusted-fixture gate positives establish conditional mock policy only; production verification and enforcement remain unestablished."]}

## Findings

### A-01 — P1: Contradicted-lane exposure can discard dollar evidence
**Location:** `spec/ck.json:2`, `X-TABLE` row “Subscription attempt, lane contradicted”; `INELIGIBLE-EXPOSURE`, `INV-F9`; `contract/TRANSITIONS.md:375–379`.

**Counterexample:** A subscription-labelled session contains a well-formed reservation of 500,000,000 nano and contradictory dollar evidence. `X-TABLE` explicitly assigns the contradicted subscription attempt `PiApi, AtLeast(0)`. Conversely, `INELIGIBLE-EXPOSURE` requires retaining `R`, and `INV-F9` requires dollar evidence to remain in PiApi. There is no declared precedence limiting the zero-valued row to attempts without monetary components.

An implementation following that table row discards 500,000,000 nano merely because lane evidence is contradictory. Blocking admission does not repair the understated conservative exposure or successor accounting.

**Required disposition:** Establish one authoritative contradicted-lane rule that preserves applicable monetary components; classification cannot select a weaker amount.

**Origin:** Unestablished; the current contradiction is directly visible.

### A-02 — P2: X’s declared partition omits valid but ineligible usage settlements
**Location:** `spec/ck.json:2`, `X-EXPOSURE-1` through `X-EXPOSURE-11`, `X-TABLE`; `contract/TRANSITIONS.md:315–379`.

**Counterexample:** `cases/R6-C01-writer-reservation-floor-control.json:43–55` has `Valid(Usage)` with incomplete valuation. `cases/U08-missing-observed-tier.json:44–61` has `Valid(Usage)` with unpriceable valuation. Neither satisfies `eligible_usage`, any other named settlement basis, rejection/conflict, nor an unbudgeted/orphan/subscription fact.

The prose table supplies both outcomes, but the supposedly finite financial partition has no explicit corresponding fact or mapping. Treating `reservation` as a catch-all would require an undeclared interpretation and would conflate distinct table rows and monetary components.

**Consequence:** Symbolic guard coverage can pass while two central reservation-retention branches remain unspecified at the declared interface.

**Origin:** New revision6 partition defect.

### A-03 — P2: Declared stage dependencies do not cover normative consumers
**Location:** `spec/ck.json:2`, `CORRELATION C-6`, `CLASSIFICATION`, `stages`; `contract/TRANSITIONS.md:11–17`; `contract/CONTRACT.md:100–108,191–195`.

**Counterexamples:**
- C-6 selects a reservation that “no valid settlement closes,” although validity is S-owned and S consumes C.
- K must emit `AttributionContradiction` from ownership/window placement, `ReopenedWithoutAdmission` from lifecycle, and commitment findings from R. Its declared inputs contain none of T, L or R.

Thus identical C/S/V/F/X facts evaluated against different ownership/windows can require different K findings, despite the declared K signature.

**Consequence:** The architecture requires either hidden dependencies, duplicated semantic evaluation, or omitted findings. Registering consumer names does not resolve these read-set conflicts.

**Required disposition:** Separate attempt-local K from consumer-owned classification, and explicitly locate the pre-correlation validity predicate needed by auxiliary pairing.

**Origin:** New canonical-interface mismatch; origin of the underlying prose dependencies is unestablished.

### A-04 — P2: Canonical valuation type and serialization disagree
**Location:** `spec/ck.json:2`, type `Valuation` and `SERIALIZATION.valuation.state`; `spec/interface.json:2`; `contract/TRANSITIONS.md:572–576`.

**Executable failure condition:** Round-trip the interface-authorized value `{"state":"not_applicable"}` through canonical `Valuation`. The interface allows it, but the type has only `not_reported`, `valued`, and `unpriceable`. No projection rule explains an alternative representation.

**Consequence:** A legal external state cannot inhabit the declared typed interface. The claimed “every legal variant” round-trip test (`evidence/joined-rust-tests.txt:4`) cannot establish agreement between these two domains merely by passing over one domain.

**Required disposition:** Reconcile the legal state set and explicitly define the typed-to-corpus projection.

**Origin:** New canonical representation defect.

### A-05 — P2: Reachability labels still contradict mandatory producer floor updates
**Location:** `cases/U08-missing-observed-tier.json:106,118`; `cases/P130-R02-rounded-up-charge-admitted.json:119,133`; `producer/pi_container.py.excerpts:223–225,265–279`; `producer/openai_api_rates.json:10`.

**Counterexamples:**
- U08 reports a missing observed tier and `multiplier_floor:0.0`. The producer selects multiplier 2 and updates the floor to at least 2.
- P130-R02 reports requested tier null, observed flex, and floor 0. The producer’s default requested tier differs from flex, requiring floor at least 1.

Both cases remain labelled `producer_reachable`. Their logged floor values cannot follow the supplied writer branch, regardless of reservation-floor feasibility.

**Consequence:** Qualification still presents impossible writer tuples as reachable controls. Their arithmetic expectations may remain useful synthetic evidence, but their provenance claims are incorrect.

**Origin:** Unestablished from the supplied predecessor evidence; current contradictions are demonstrated.

### A-06 — P2: Provisional reporting lacks a consistent financial stopping condition
**Location:** `contract/CONTRACT.md:124,216–218`; `contract/INVARIANTS.md:11`; `spec/ck.json:2`, `TEMPORAL-AND-FINALIZATION`.

**Counterexample:** Use `P130-R01-usage-incomplete-terminal`: its attempt is temporally closed, retired, but financially unresolved with `AtLeast(500000000)`. Give its owner an established closure.

P-4 describes provisional totals lasting until attempts are closed or retirement-bounded; §12.4 instead requires provisional totals while any owned attempt remains unresolved. These are different conditions for precisely this case. Neither the canonical boundary nor the residual rules specify whether these are separate flags or one combined predicate.

**Consequence:** Consumers can report financially unresolved totals as no longer provisional by using temporal completion as the stopping condition.

**Required disposition:** Define the single output predicate, or distinguish temporal completeness from financial provisionality explicitly.

**Origin:** Unestablished.

## Coverage, design implications and uncertainty

The packet is complete for this bounded review. Declared omissions prevent certification of tooling implementation, complete corpus qualification, or production behavior—not completion of this specification review.

Independent arithmetic checks support several sampled expectations: OpenAI’s unknown-cache vertices yield 6,000,000–17,500,000 nano for N06; Anthropic’s inconsistent-split fallback yields 18,000,000–24,000,000; 21 cached flex tokens cost 525 nano before producer logging; R6-C02’s 20,000 output tokens price to 1,000,000,000 nano. These calculations do not establish writer reachability or general aggregation correctness.

The current rules explicitly distinguish HTTP200 plus successful terminal completeness, unique-reservation comparison applicability, temporal closure, and financial replacement. Those declarations do not cure missing X partition states or conflicting lane treatment.

Canonicalization has centralized storage, but not fully localized semantic authority: tables, formulas, types, and boundary consumers still disagree inside or around that storage. The necessary correction is bounded: retain the financial model and post-V finalization, while making financial branch ownership and consumer classification boundaries consistent. Another manually synchronized collection of exceptions would not satisfy the architectural objective.

The supplied gate case includes a separate test-mock input, but its projected-out evaluations prevent checking commitment qualification/output provenance here. No live trust or enforcement conclusion follows. Remaining clearance also requires independent multi-observation, correlation, full-classification, and validator qualification beyond the raw audit’s stated axes.