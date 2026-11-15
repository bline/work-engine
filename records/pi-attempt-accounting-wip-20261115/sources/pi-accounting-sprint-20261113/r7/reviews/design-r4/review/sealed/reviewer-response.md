{
  "verdict": "REVISE_PLAN",
  "primary_boundary": "Eligibility/release authority and conserved monetary ownership across C/V/F/X and R",
  "ordinary_repair_recommended": false,
  "implementation_under_plan_recommended": false,
  "required_design_changes": [
    "P1: Remove the proposed non-settlement release exception; reconcile late discovery with unchanged I1 and settlement transitions.",
    "P2: Define canonical retention-interpretation membership, witness coverage, overlap rules and input/output ownership rather than assuming conservation extensions.",
    "P2: Specify how interpretation envelopes project into per-attempt exposure, supported lows, attribution and existing additive rollups."
  ],
  "owner_decisions": [
    "Resolve the incompatible late-discovery requirements without treating this review as permission to relax I1. Any admissible-input restriction needs an evidenced owning contract.",
    "Explicitly disposition conditional initial reference scopes, changed same-reference orphan totals and expanded financial-candidate relevance.",
    "Choose the smallest complete monetary ownership model. If full executable raw-to-output proof is required in S1, its acceptance owner must resolve that requirement against D3."
  ],
  "evidence_limits": [
    "All 51 file boundaries and the final review-set sentinel are present; hashes and byte counts were not independently recomputed.",
    "No tools, commands or tests were executed. Planner census and root reproductions remain attributed evidence.",
    "The full corpus, raw audit FACTS, individual inventories, full production implementation and live authority are unavailable or unestablished.",
    "This review does not clear Module A/B/C, establish full raw-to-output conformance or qualify live enforcement."
  ]
}

## Causal diagnosis

R7’s demonstrated failure is changing the monetary partition: two separately summed charges become one maximum. Its origin is unestablished—not a demonstrated repair-caused regression. Recurring I1 nevertheless requires design review, including inherited defects. Infrastructure, quota failures and timeouts contribute zero implementation rejections.

**R4 materially improves on R3.** Its monotone financial-candidate relation prevents RS display reassignment from deleting the dearer identity. The supplied witness consequently becomes **AtLeast80M → AtLeast81M**, not51M. This addresses monetary association, not merely primary keys.

Its High join is also correct: joining Exact100 with AtLeast50 produces AtLeast100, preserving both the numeric floor and unresolvedness.

But R4 solves late discovery by **explicitly changing the prohibited transition**, and its general interpretation layer lacks a complete consumption contract.

## Three material issues

### 1. P1 — R4 expressly relaxes I1

**Location:** `candidate/PLAN-R4.md:110–136,159–165,215`.

The proposed exception permits release whenever *any* missing prerequisite establishes an EligibilityWitness. That is broader than current `INV-F3`, whose exception requires adding an eligible usage settlement.

R4’s own decisive example demonstrates the difference:

- Settlement alone: Rejected/NoReservation, AtLeast15M.
- Add its earlier-position reservation: Valid/Eligible, Exact15M.

AtLeast15M → Exact15M is a decrease under the binding High order, despite unchanged nano. It also violates the current Rejected→Valid prohibition. A late scope certificate could similarly authorize previously unavailable replacement without adding a settlement.

This is candidly labelled a policy amendment; it is **not preservation of unchanged I1**.

**Required change:** withdraw this behavior from the candidate accepted under this review. The owner must resolve the incompatible requirements:

1. arbitrary subset discovery;
2. final-set ordinary eligibility;
3. unchanged release/transition prohibitions.

An evidenced complete-source input contract might distinguish an inadmissible partial observation from an admissible orphan. Merely asserting completeness, restricting tests to convenient append sequences, or renaming discovery as a new subject is insufficient. If no permitted realization reconciles these requirements, report the contract conflict rather than admit the proposed exception.

### 2. P2 — Conservation remains partly an assumed witness property

**Location:** `candidate/PLAN-R4.md:76–108,123–149,177`.

The argument assumes every old interpretation has a conservation extension. That is a useful obligation, but the plan does not completely define:

- the canonical family of admitted retention interpretations;
- their assignment-record structure and completeness conditions;
- how the family is derived independently of discovery history;
- overlap/disjointness rules for multiple eligibility discharges;
- which records are trusted inputs, computed outputs, or independent expected assertions.

This matters because the pure function allegedly consumes interpretation evidence, while F owns EligibilityWitness and the oracle bundle contains both. An oracle-authored interpretation list must not become an unrestricted monetary-policy input. Keeping every interpretation that happened to be previously supplied is not, by itself, a route-independent construction.

The stated fallback1000M-plus-unmatched-R500M result is concrete and conservative. It does not define every mixed scoped/unscoped case.

**Required change:** declare one owner and fixed admission rules for interpretation records, including complete origin-to-unit assignments, retained-parent references and exact discharge coverage. Separate input provenance from C/F/X results and independently authored expectations. Specify omission, overlap, contradictory-scope and forged-witness rejection obligations.

No general evaluator is required in S1: these can be normative definitions, fixed schemas and bounded literal assertions. But “a conservation extension exists” cannot remain the mechanism that supposedly ensures one.

### 3. P2 — Global envelopes do not yet fit the external additive model

**Location:** `candidate/PLAN-R4.md:80–84,173–179,203`; `contract/CONTRACT.md:152–180`.

Current R sums per-attempt exposures; T cannot change money. R4 instead introduces a join of interpretation totals, without fully defining its projection into those outputs.

These operations do not generally commute. If alternative assignments yield per-unit highs `(80M,20M)` and `(20M,80M)`, joining totals gives100M, whereas summing per-unit joins gives160M. This is an interface stress case, not a claimed reproduced raw case. The design must either rule it out through its assignment semantics or represent the shared interpretation explicitly.

Likewise, R4 defines the envelope’s high but not its complete supported-low projection, aggregate provenance or attribution behavior when retained interpretations use different units. References to witness inventories do not themselves specify these results.

**Required change:** define the X result consumed by T/R and its exact serialization. Establish either:

- factorization into stable, independently additive per-unit exposures; or
- an explicitly owned joint envelope with compatible attribution, predecessor and rollup rules.

Define supported lows separately from conservative policy holds. Do not let R silently recompute or strengthen X money to reconcile the representations.

## Alternatives

**Stable scoped reference identity** is the smallest primary-group correction. Under an expressly admitted initial scope, same-rid orphan charges begin at500M and remain500M when R arrives. That changes the old initial1000M result and needs root adoption; it does not establish physical multiplicity as one.

**Explicit merge accounting** can preserve constituent contributions without ingestion-history dependence. It requires exact ownership and release coverage, but not necessarily possible-world enumeration.

The **smallest credible next candidate** is stable scoped units, monotone financial context, fixed fallback-origin ownership and explicit continuity, retaining existing max/sum arithmetic where possible. R4 should show a lawful raw witness requiring its more general interpretation layer before making that layer the default architecture.

The existing distinctions—settlement validity, execution, valuation, replacement, exposure and commitments—remain useful. Broader replacement is not justified merely by repeated implementation failures.

## What should be retained

R4 correctly distinguishes:

- distinct reservation evidence from distinct amounts;
- scoped byte-identical copies from independently witnessed incarnations;
- missing models from malformed reported identities;
- financial relevance from mutable display attachment;
- conditional fixture scope from authenticated producer provenance.

Equal500M reservations with different timestamps remain conflicting and cannot supply UniqueReservation. Amount equality must never repair identity.

Removing independently writable S constructors is a real ownership improvement. The proposed wire targets—`pool`, `kind`, decimal nano strings, basis strings, valuation `derivations`, obligation `amount_source/by`, and parameterized reasons—address the actual external mismatch, not just internal round trips.

R4 also correctly identifies the actual Python consumer: `validator/validate_corpus.py:473` forces the wrong zero low, and `:438–449` normalizes JSON for duplication. No identified consumer gives those rejected behaviors backward-compatibility authority.

## Concrete acceptance witnesses

Before implementation acceptance, require independently authored checks covering:

1. **Identity and continuity:** scoped500→500; unestablished charge subtotal1000 plus unmatched R500; distinct tickets/incarnations additive; equal-amount distinct reservations conflicting; exact scoped copies idempotent; malformed references and source growth preserve origins.
2. **Association conservation:** RS80→81 through both discovery routes; missing/malformed n, read errors, measurements, ambiguity and scope changes cannot remove retained financial effects.
3. **Lawful eligibility:** late-earlier-R handling under an actually adopted I1-preserving contract; compare High tags as well as amounts. Ordinary eligible VC01 remains Exact15M.
4. **Raw financial projection:** exposure lows15M/15M/20M/60M for the four cited cases, correct highs/bases, mixed-observation controls, malformed model7 preventing eligibility, and approved complete priced multiidentity bounds remaining finite.
5. **External applicability:** actual corpus-shaped orphan/refusal/missing-launch/subscription-dollar witnesses, including unresolved PiApi zero-known exposure and positive retained reservations. No convenient exclusions or S/V overrides.
6. **Actual consumers:** wrong S-constructor injection fails; wrong owning validity assertions fail independently; Python and Rust checks reject self-consistent wrong expectations with matching rollups. Predicate changes invalidate V/F/X/R, views and validator acknowledgements. Standard schema validation agrees with Rust loading.

Independent provenance remains an establishment obligation, not merely an author label. Mock gates must remain separately supplied, exact-payload-bound `test_mock` evidence; neither hashes nor fixture passes establish live authority.

D3 permits declaration agreement and bounded independent arithmetic—not a partition solver or financial reducer disguised as qualification. Static agreement must remain labelled accordingly.

Root owns adoption. This recommendation grants no mutation, contract-change, production or S2a authority. Current findings and review history remain unresolved.