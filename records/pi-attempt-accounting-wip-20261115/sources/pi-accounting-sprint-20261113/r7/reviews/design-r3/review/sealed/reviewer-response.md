{
  "verdict": "REVISE_PLAN",
  "primary_boundary": "Evidence association and eligibility changes across C/V/F/X, including aggregate conservation and source-scope continuity",
  "ordinary_repair_recommended": false,
  "implementation_under_plan_recommended": false,
  "required_design_changes": [
    "Extend the identity design to monetary effects of auxiliary reassignment, not only Row-to-Reservation rekeying.",
    "Resolve newly discovered earlier reservations against the pure evidence-set model, settlement transition rules and unchanged I1.",
    "Specify typed scope, multiplicity, co-reference and evidence-continuity interfaces through their actual serialized or explicitly referenced evidence locations.",
    "Require a compositional conservation argument and independent boundary witnesses, including qualification of the existing Python validator."
  ],
  "owner_decisions": [
    "Explicitly disposition the changed same-reference orphan aggregate and conditional declared-fixture source assumptions.",
    "Resolve late-evidence eligibility without silently adding an I1 exception or restricting I1 to append-only discovery.",
    "Bind replacement input subjects to accounting continuity; changing subject identity must not reset retained exposure.",
    "If full executable raw-to-output conformance is required in S1, explicitly resolve that requirement against D3; static agreement cannot substitute for it."
  ],
  "evidence_limits": [
    "All 50 file boundaries and the final review-set sentinel are present. Hashes and byte counts were not independently recomputed.",
    "No tools, commands or tests were executed. Planner census, bindings and reproductions remain attributed evidence.",
    "The complete corpus, frozen raw audit facts, inventory entries, unexcerpted producer implementation and live authority are unavailable or unestablished.",
    "This review cannot clear Module A/B/C, establish implementation correctness or qualify live enforcement."
  ]
}

## Causal diagnosis

R7’s demonstrated failure is **partition instability**: two contributions previously summed become one contribution evaluated by maximum. Correct reservation retention inside each X branch does not preserve aggregates when C changes the accounting partition. Its origin remains unestablished; this is not identified as a repair-caused regression.

`candidate/PLAN.md` makes substantial improvements. Stable ticket claims remove the particular primary-key collapse, explicitly acknowledge changed initial orphan totals, separate evidence cardinality from amount equality, and retain fallback atoms when reference scope is unestablished.

However, stable primary keys are insufficient. Monetary evidence associations and replacement eligibility can still change after evidence arrives. The plan does not yet establish conservation across those transitions.

## Three material issues

### 1. P1 — Auxiliary reassignment can still lower aggregate exposure

**Boundary:** `candidate/PLAN.md:78–84,143`; inherited `spec/ck.json`, CORRELATION C-5–C-8 and VALUATION V-1/V-8.

The plan prevents auxiliary attachment from changing primary validity. It does **not** prevent attachment changes from removing an identity that previously raised valuation.

A decisive synthetic witness:

- One reservation: rid1, n1, R=50M.
- One rejected reservation-basis request: n1, charge50M, null status, partial Anthropic usage of 10,000 input tokens, zero other counts, standard tier.
- Launch/request/response identity is `claude-sonnet-5`.
- An earlier identity-refusal row names `claude-fable-5-2`, also n1.

The request does not match the refusal’s served model. Under RS-3’s fallback, the refusal attaches to the unique unsettled reservation. The usage prices to 20M–80M across the two identities; exposure is **AtLeast(80M)**.

Now add a reservation with rid2, n1 and R=1M. The uniqueness condition fails. The refusal becomes an orphan without usage:

- rid1 loses the dearer identity: high50M;
- rid2 contributes1M;
- refusal contributes AtLeast0.

Total becomes **AtLeast(51M)**. No eligible usage settlement was added.

This is the same invariant failure through a different association boundary. Merely asserting “conflicting additions retain aggregate high” does not supply the mechanism.

**Required change:** define conservation of monetary evidence when auxiliary links attach, detach, become ambiguous or change candidates. Preserve uncertainty as an accounting-relevant relation—not just an orphan diagnostic.

### 2. P1 — Late reservation discovery conflicts with unchanged eligibility and transition semantics

**Boundary:** `candidate/PLAN.md:60–62,130,158`; `spec/ck.json`, SETTLEMENT and INV-F3.

The plan explicitly rejects assuming evidence additions are only physical append extensions. Consider, therefore:

1. A complete, correctly priced usage settlement is available without its reservation. It is rejected for NoReservation and has AtLeast exposure.
2. Its matching reservation, occupying an earlier original source position, becomes available.

The final evidence set satisfies the ordinary valid-usage conditions. A pure set-based decision therefore produces Valid/Eligible and Exact exposure. Yet:

- the added item was a reservation, not an eligible usage settlement;
- AtLeast→Exact is a decrease under MONEY’s order;
- SETTLEMENT prohibits Rejected→Valid without removing evidence.

Stable TicketClaim identity does not resolve this contradiction.

**Required change:** specify the lawful evidence-extension relation and eligibility transition together. If this discovery is excluded by the admitted input contract, establish that exclusion and reconcile the plan’s rejection of prefix-only reasoning. Otherwise, root must disposition the relevant no-reservation/eligibility semantics while preserving I1 and ordinary eligible positives. An ingestion-history latch would not satisfy the claimed pure evidence-set model.

### 3. P2 — Scope and multiplicity lack a complete consumption interface

**Boundary:** `candidate/PLAN.md:39–68,98–110`.

The proposed distinctions are useful, but important choices remain descriptions rather than concrete interfaces:

- What typed witness admits a single incarnation/reference scope?
- What distinguishes fixture-declared scope from established continuity?
- What constitutes affirmative independent attempt identity?
- When such identity conflicts with scope, which deterministic rule chooses “separate units” versus fallback?
- Where do multiplicity, possible co-reference and continuity provenance actually appear for consumers?

The money serialization section is concrete; these new financially consequential identity fields are not comparably specified.

Also, replacing an input subject cannot become an accounting reset. A consumer processing the same underlying obligations must not discard fallback atoms simply because E or an owner publishes a newly qualified subject.

**Required change:** declare these witness variants, reference/duplicate equivalence rules, transition consequences and external evidence locations. Byte equality must not deduplicate independently established incarnations. Equal amounts must never establish reservation uniqueness.

## Alternative comparison

**Stable scoped reference identity** remains the smallest promising primary-group change. Within a genuinely admitted immutable scope, it makes the original pair 500M→500M instead of 1000M→500M. That is an explicit policy amendment, not proof that physical multiplicity equals one. Known independent units must remain additive.

**Explicit merge accounting** can preserve the old orphan sum by retaining constituent monetary provenance and enforcing conservation during merges. It need not inherently depend on processing history: a set-based constituent model is possible. It does require more detailed ownership and no-double-count rules.

**Smallest credible refinement:** retain stable primary claim keys, but add explicit conservation semantics at the remaining unstable association boundaries, especially RS attachment and scope qualification. Compare that bounded refinement with constituent merge accounting before replacing the broader financial state model.

Keeping orphan row keys only until an earlier reservation becomes visible does not solve the general problem. Summing every reservation and settlement indiscriminately also violates the approved measure.

## What the plan gets right

Sections 2–5 are substantially stronger than R7:

- **S validity:** removing independently writable S constructors genuinely removes a competing constructor owner. The owning C-validity table still needs independent semantic witnesses.
- **Wire semantics:** `pool`, `kind`, nano strings, basis strings, valuation derivations, obligation `amount_source`/`by`, and parameterized reasons now have explicit target representations.
- **Applicability:** the refusal-only orphan has a lawful PiApi/NotApplicable/NotReported/AtLeast0 route without falsifying S or V.
- **Dependencies:** the proposed USAGE-PRESENCE→V→F/X→R propagation follows actual consequences rather than merely rendered bytes.
- **Qualification:** malformed identities and equal-amount distinct reservations are correctly distinguished from absent identities and duplicate evidence.

Preserve approved complete, established, fully priced multiidentity finite bounds. Preserve monetary versus temporal resolution, oracle provenance, and exact-payload-bound `test_mock` qualification without promoting it to live authority. No identified consumer justifies retaining the rejected generic external projection shape.

## Concrete acceptance witnesses

Before implementation acceptance, require independently derived witnesses covering:

1. **Association conservation:** the 80M→51M example above; auxiliary attachment/detachment; ambiguous candidates; source-scope contradictions; replacement-subject continuity.
2. **Primary identity:** admitted-scope orphan500M→500M; fallback charge subtotal1000M plus unmatched R500M; distinct rid/incarnation units totaling1000M; malformed references never silently repaired.
3. **Eligibility discovery:** the earlier-reservation example, with identical final evidence assembled through different discovery routes. Check the complete High order, not just nano amounts.
4. **Evidence cardinality:** two distinct same-rid 500M reservations remain conflicting and disable relative comparisons; a byte-identical repeat remains unique under the scoped duplicate contract.
5. **Financial projection:** exposure lows15M/15M/20M/60M for the four cited cases; eligible VC01 remains Exact15M; malformed model7 prevents eligibility; approved priced multiidentity positives remain finite.
6. **External qualification:** actual corpus wire objects, all reason encodings, orphan/refusal/subscription-dollar controls, and separate StageId/BoundaryId schema positives and negatives.
7. **Actual checking consumers:** qualify `validator/validate_corpus.py`, not merely new Rust assertions. Its `table_x` line473 explicitly demands zero low for rejected/conflicting settlements; `raw_reservation` normalizes JSON for deduplication. Mutants must challenge those semantics with self-consistent expected outputs and rollups, not only differ from existing answers.
8. **Invalidation:** predicate, identity, cardinality and serialization changes must stale every actual affected consumer acknowledgement through V/F/X/R and views.

These witnesses are necessary falsification evidence, not a universal proof.

## D3 and authority

A compositional argument—stable accounting units, conserved association effects, nondecreasing component envelopes, and lawful replacement exceptions—can be documented and challenged using permitted static tooling and separately authored bounded assertions. No reference reducer is required for that bounded claim.

Full executable raw-to-output conformance remains unavailable under D3. If required, its acceptance owner must explicitly resolve the scope conflict rather than disguise an evaluator as tooling.

Root owns adoption. This recommendation grants no mutation, production, contract-change or S2a authority. R7 remains rejected, unresolved findings remain, and infrastructure/quota/timeouts contribute zero implementation rejections.