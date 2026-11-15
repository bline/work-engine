# R5 candidate: fixed ownership, local coverage, additive money

Status: **new architectural candidate; independent acceptance and implementation remain blocked**. Root policy `r7/ROOT-POLICY-R5.md` SHA25657303fdb466c49ebe8f0d6fa095628c57c3f2018603114e12949af663f65eec5 explicitly selects the new subject's late-prerequisite release, initial-reference-scope and monotone financial-relevance policies. This plan applies that owner decision. It does not claim old INV-F3 was unchanged. Fresh review must judge safety/coherence under the selected amendment; root policy is authority, not correctness evidence.

Packet18898bcdf675e4ce19e4413c57b7c53cefa2da7ef4003b09bc1d8c03244d768e; retained planner01a1d77d-43dc-7210-a674-615860bfb6d2; root01a1d6e2 sole collector. R7, R3 and R4 artifacts remain immutable. No R8/code/ordinary repair/S2a authority follows.

## Decision and removal of the R4 layer

Use fixed origin-owned financial units, monotone pricing context, nonmerging C refinements and exactly covered F replacement units. X computes a maximum **within each active unit**; R sums those unit exposures. Remove retention interpretations, their supplied lists and joins of alternative aggregate totals. No oracle list may choose policy or a financial partition.

This selects the factorized conservative policy. In the review's abstract alternatives `(80,20)` and `(20,80)`, if each unit genuinely has both price contexts, X's independently conservative unit highs are80 and80 and R holds160. It does not secretly substitute a joint-world100. This is conservative provisional exposure, not verified provider billing. A contextual identity may inform two units; a monetary usage/charge origin belongs to only one active unit. No joint attribution or solver is introduced.

## 1. Separate authoritative input facts, computed results and expected assertions

**E/input provenance owner** supplies immutable raw origins and admitted source declarations:

```
Origin = { lineage_id, source_origin_id, record_origin_id,
           original_position, raw_line_sha256, source_revision_ref,
           initial_scope_ref }
InitialScope = FixtureDeclared { scope_id, case_world, session_id, turn_log,
                                 raw_manifest_ref, assumption_ref }
             | ProducerBound { scope_id, incarnation_id, realization_ref,
                               observer_owner, observed_extent_ref }
             | Unestablished { observation_scope_id, reasons, evidence_refs }
IdentityFact = { origin_ref, producer_attempt_id?, identity_provenance_refs }
ContinuityInput = { lineage_id, predecessor_subject_refs,
                    predecessor_origin_inventory_ref, exact_origin_map,
                    prior_scope_declarations_ref }
```

A digest authenticates referenced bytes, not the truth of a producer binding. FixtureDeclared is conditional test input, never live incarnation proof. ProducerBound requires the provenance promised by its external owner; absent evidence gives Unestablished. No raw producer row can mint those grants. Current175 cases have186 supplied log containers,282 usable dollar-rid claims and no explicit producer-incarnation fields, as the preserved census establishes. Their existing C11 model scopes remain conditional fixtures under the selected root policy. No new live incarnation service is assumed.

`initial_scope_ref` is an immutable origin-contract declaration, not a variable chosen when a reducer happens to see a row. Later scope evidence is additional provenance; it cannot overwrite that declaration. Source hashes identify revisions; the session/turn/log origin relation identifies continuity. Original raw173/175 inputs stay unchanged; conditional declarations are separately pinned fixture inputs.

**C computes** origin-to-unit assignment, duplicate classes, definite co-reference facts, monotone financial candidates, distinct reservation/settlement cardinality and the single primary validity result. **F computes** eligible coverage and replacement results. **V/X compute** valuation/component selection/exposure. None is accepted as an oracle-authored policy input. Independently authored oracle facts and expected assertions compare these computed records to raw evidence. A file containing expected witnesses is evidence for tests, not runtime release authority.

## 2. Canonical base units and refinement

The complete origin inventory always preserves every row, including duplicates. Assignment is deterministic from immutable declarations and origin facts:

1. An established ProducerAttempt identity supplies a producer-attempt key. Independent incarnation/sequence identities remain distinct even for equal bytes/rid/amounts.
2. Otherwise, a usable dollar rid within an admitted **initial** scope selects `TicketClaim(scope_id,rid)` from the first raw claim, including before R exists. Admission rid uses a different namespace. This is the selected initial orphan500M-versus-old1000M amendment, not proof of one authentic physical attempt.
3. Otherwise use `OriginAtom(source_origin_id,record_origin_id)`. This includes an attempt-evidencing orphan/refusal/error with no known amount: it has an unresolved zero-known hold, not an implicitly established zero bill. A context row that is expressly non-attempt evidence remains in the evidence inventory without a financial amount. C9 preflight Refused retains its existing non-attempt status.

A duplicate equivalence is scoped exact raw-line equality or reobservation of the same origin, under the admitted duplicate contract; normalized JSON equality is insufficient. Explicit independently distinct identities override equality of bytes. In Unestablished scope, two different origin positions are not collapsed merely because their bytes match. Reobservation of the same origin is idempotent.

Later evidence may **split** one parent unit, never merge two C units. For a consistent definite identity refinement, origins with a proved identity enter the deterministic child `(parent,producer_attempt_id)`; unassigned origins enter `(parent,residual)`. Every monetary/attempt anchor origin is assigned exactly once; empty children disappear. Multiple inconsistent identity assignments or scope contradictions cause the affected parent to refine into individual origin atoms, with provenance conflict and eligibility blocked. This is conservative failure, not selection of a preferred conflicting witness. The contradictory facts remain; later favorable assertions cannot silently remove them.

Every pricing component is anchored to a usage/amount origin and its context refs. Refinement transfers it with its owning origin; all old candidate contexts remain financial reads. Thus a parent `max(c1,…,cn)` becomes children whose component sets partition those origins: `sum(child maxima) >= parent maximum` for nonnegative amounts. No parent exposure is added again. Noneligible children remain AtLeast, so refinement cannot clear unresolvedness. New independently evidenced units remain additive. A row's evidence and current display parent can change without moving or deleting its monetary origin.

Base units are fixed by origin-contract provenance; refinements only split. A stronger scope certificate cannot turn fallback atoms into a single TicketClaim. Proven co-reference across different base units is a validity fact for F, not a C merge permission. This is how fixed ownership survives qualified replacement subjects without a hidden discovery-order latch.

## 3. Monotone financial context, distinct from RS display attachment

C computes FinancialCandidate at the **origin pair** level. Its predicates use immutable positive raw facts: same admitted logical source and matching usable n, or an explicit applicable ticket/measurement reference. Missing/malformed n retains an unresolved same-source candidate scope, including all compatible usage origins when no narrower relation is evidenced. No first-unclaimed, first-later, uniqueness, unsettled-reservation or nearest-row filter removes financial relevance. Those remain display-association rules only. No cross-source identity is inferred from equal amounts or strings.

V prices each owned usage observation over intrinsic and candidate identity/tier/split contexts. Adding candidates cannot remove a previously supported priced high. A malformed or unpriced identity stays explicit; unpriceable output retains the maximum of all computable priced components. Existing V8 min-of-supported-lows/max-of-highs and unpriceable dominance remain. Complete established fully priced multiidentity bounds remain permitted; uncertainty cannot erase their known priced high. A pricing-context fact alone creates no positive usage amount.

Example: R1/charge50M with incomplete10000-token Anthropic usage has intrinsic sonnet20M and refusal-context fable80M. Financial relevance survives RS3 detachment. Add same-n R2=1M: u1 staysAtLeast80M, u2 holdsAtLeast1M, any unresolved zero-known auxiliary anchor contributes0 but can retain unresolvedness. Total **AtLeast81M**, not51M. Both discovery routes and all-at-once give the same final facts/result. Identity context is not counted as a second usage observation.

Zero-known attempt anchors are not removed by RS attachment alone. A complete F coverage may absorb one only when its co-reference to that exact completed obligation is affirmatively established. For legacy priced-multiidentity positives, qualification must demonstrate that coverage or expose a separate unresolved anchor; it may not silently drop it to preserve a convenient expected total. This is a material unit-ownership consequence to inspect in the full corpus, while the approved complete priced valuation bound itself remains finite.

## 4. Exact F coverage and active replacement units

C's one validity witness reports unique distinct R/S cardinality, applicable local checks, original source order, definite co-reference and missing-prerequisite versus immutable-contradiction reasons. S forwards this witness by identity and owns only time/execution/forwarding. No independent S decision constructors remain.

F derives a coverage only for a definite obligation satisfying the selected root policy: exactly one distinct reservation/settlement, all applicable immutable checks, correct original order, established scope/co-reference, complete usage and fully established priced identity/tier set. A weak FinancialCandidate edge by itself is not definite co-reference. Explicit ticket/producer identity evidence or the declared applicable unique RS co-reference witness may establish an auxiliary anchor's relation; that witness retains its rule/source facts and cannot alter primary validity. Ambiguous RS membership cannot be used for release.

```
Coverage = { obligation_id, complete_active_unit_ids,
             monetary_origin_ids, zero_known_anchor_origin_ids,
             reservation_ref, settlement_ref, primary_validity_ref,
             definite_coreference_refs, complete_valuation_ref,
             retained_charge_components, applicable_context_components }
```

The coverage is the closure of all active units containing origins definitely belonging to that obligation. Every member unit must be covered **whole**; a witness cannot release a convenient subset of a unit containing other unresolved origins. If C cannot refine it by definite evidence, eligibility fails. Every relevant known charge/context component is retained in the new maximum. A second distinct reservation/settlement for that obligation invalidates the unique witness, even if an output omits it. Independently distinct units cannot be covered together. Uncovered atoms stay active/additive.

F is the only constructor. Raw `eligible:true`, oracle-authored coverage, hashes alone and scope labels are not witnesses. F output is mechanically checked for full origin/unit closure against C; independent tests separately challenge its semantic premise. Omitting a known origin, lying about cardinality, forging a provenance reference or suppressing an unpriced candidate fails coverage and leaves the original units held.

Overlap rules apply after validity/closure checking:

* Disjoint covered unit/origin sets apply together.
* Identical obligation, exact coverage and derived financial payload are one canonical coverage, regardless of repeated proof-record references. It is idempotent.
* Nonidentical intersecting coverage—including strict subsets—or identical coverage with contradictory payload is rejected as CoverageConflict. No order-based winner is selected; every affected original unit stays held. Shared read-only pricing-context refs do not constitute release overlap because they are not monetary ownership/anchor coverage.

For accepted coverage, replace its active units with one deterministic `ResolvedUnit(lineage, sorted covered origin IDs, policy version)`; the same complete coverage always has the same ID. Retired unit IDs remain in the accounting evidence mapping, without a second financial exposure row. Its high is `Exact(max(all retained covered charges, complete usage/context high))`; low is the supported usage low, never a proxy amount. It cannot absorb an uncovered unknown bill or erase independent charge evidence. Every active monetary/anchor origin has exactly one active owner before and after F.

This is a local normalized sum, not a global interpretation envelope. A disjoint set of coverage applications produces the same active units in every processing order. If new immutable conflict invalidates eligibility, compute held original/refined units from the same complete input facts; retained charges/context and reservation components ensure Exact→AtLeast does not drop a known numeric component. No prior-status latch participates.

### Selected late-prerequisite amendment

Settlement S at original source position2 is complete/priced, charge15M/reserved500M, rid/n1. R at position1 has500M and matching fields. S-only: Rejected/NoReservation/AtLeast15M. Add R: all conditions become established, producing Valid/Eligible/Exact15M. R-first: AtLeast500M; add S: the **same** final coverage and Exact15M. The tag decrease is explicitly authorized by ROOT-POLICY-R5, replacing the old row-kind exception and blanket Rejected→Valid prohibition.

NoReservation is missing prerequisite. Malformed local fields, actual source-later order, undercharge, an unpriced identity or multiple distinct R/S cannot be repaired by adding a favorable row. Incomplete usage does not release. Merely adding scope provenance does not release. Ordinary VC01 remainsExact15M. Genuine unbudgeted complete usage retains its existing separate CompleteUnbudgetedUsage finite-bound route; a rejected no-reservation dollar settlement cannot be relabeled unbudgeted to use it.

## 5. X, low/basis, T/R and continuity

X emits one standard corpus-shaped exposure for every **active financial unit**. Noneligible API units hold `AtLeast(max(R,known charge,usage high/priced_high))`, with zero when no positive component exists. Low is the applicable supported usage valuation (including V8), or0 when no minimum is supported. A reservation-only/zero-known anchor has low0. All active units use current basis/tie policy: proxy_reservation/proxy_charge/usage_valuation/none. Several observations within one unit use V8; independently split units retain their own observation lows. No aggregate or parent floor is substituted for a supported low.

`attempts[]` financially enumerates active units once, with stable keys and parent/replacement references in accounting evidence. Inactive parents and covered units appear only in the referenced ownership map. Existing exposure wire is retained: pool, low{nano,basis}, high{kind,nano,basis}; valuation uses amount strings/derivations; obligations preserve state/amount_source/by. The new unit-granularity evidence is a schema amendment, not a claim of unchanged presentation.

T assigns ownership/admission/candidate labels to those unit IDs and cannot change X amounts. A resolved unit carries the exact union of its origins and applicable attribution evidence; unresolved or contradictory ownership remains blocked/candidate under the existing T policy. It does not pick a cheaper unit or count the same active ID twice. R sums active unit exposures once per applicable rollup, with existing High sum and basis histogram. Parent/successor aliases cannot be added as separate amounts. R owns financial provisionality; retirement/closure cannot clear an AtLeast amount or commitment.

ContinuityInput must map every predecessor origin to the same origin bytes, retained original scope declaration and current unit/refinement/replacement owner. All prior origins must survive; every changed ownership relation must be either a complete nonmerging C split or exact accepted F coverage. A source hash change alone does not create a new obligation universe. A successor lacking the prior inventory, omitting origins, rewriting an initial scope or presenting an unexplained many-to-one map fails continuity. The consumer retains the last admitted financial subject and marks successor accounting unestablished; it does not publish a lower fresh balance. New raw evidence remains disclosed as unqualified, not silently ignored or declared free.

This is explicit accounting-lineage provenance consumed by the pure function, not hidden discovery memory. Identical complete normalized origins, immutable declarations, continuity facts and raw evidence yield identical active ownership/output in both discovery routes. A qualified new source subject cannot wash away fixed fallback atoms. No independent observed live continuity producer is established here.

Concrete consequences: admitted-scope same-rid orphan500+500 remains one unresolved500 group after R; unknown-scope two charge atoms remain1000 plus unmatched R500 =1500 until exact eligible coverage. Multiple settlements prevent a convenient coverage, so that1500 cannot become500. Distinct independent ticket/incarnation units with500 each remain1000. External unrelated R stays additive even if another unit is eligible.

## 6. Evidence placement and ownership

E provenance is a separately pinned input inventory (`inputs.accounting_provenance` or an exact referenced fixture-input inventory) with raw origin/scope/continuity facts only. C/F/X outputs live under a per-subject computed accounting record referenced by `expected.accounting_evidence` and each active attempt's `accounting_evidence_ref`. Its fixed schema includes origin-to-unit mapping, duplicate/independence facts, refinements, financial candidates, primary validity, coverage decisions, inactive-to-active mappings and per-unit component provenance. No retention-interpretation lists exist.

The independent oracle bundle contains **assertions about those results**, identified as expected assertions and bound to raw source/provenance; it does not supply C/F outputs to the implementation. The spec owner owns contracts, fixed schemas, declarations and serializers. The oracle owner owns raw/provenance witness derivations and expected arithmetic/coverage assertions. Runtime input proof, computed result and independent expected evidence have separate schema kinds and references; interchange is rejected. Fixture author labels do not establish independent production provenance.

## 7. Required bounded qualification and all findings

The preserved source bindings/custody remain evidence: SOURCE-BINDINGS and SOURCE-BINDINGS-R4; scope census175/186/282/four collisions/no incarnation fields; actual checkerfd44debd USAGE-PRESENCE impact omits contract/R. No unchanged probe is rerun for R5. The complete R4 review10679c678e8baea3a9d47566b26d8b91a92f1a0e9a768547855d678817e9f8f7 is accepted as predecessor design evidence; root policy resolves its authority conflict for this subject, while ownership/consumer issues remain to falsify.

Qualification must challenge computed ownership, coverage and actual checking consumers with self-consistent wrong expectations and matching wrong rollups:

| Boundary | Required witness |
|---|---|
| A01/conservation | Scoped500→500; fallback1000+R500; distinct units1000; C split partitions every component and never lowers max/sum; external R remains additive; exact RS80→81 both routes; scope/subject cannot reset ownership |
| New coverage | Disjoint/identical/partial/subset/contradictory candidates; omitted origin/unit/charge/context; forged input versus computed output; ambiguous co-reference; invalidated eligibility restores held units; all active origin ownership total and unique |
| Late eligibility | Both earlier-R discovery routes and all-at-once same Exact15M under selected policy; bad original order/malformed model7/incomplete/secondR/S retain AtLeast; compare High tags and numbers |
| A02/low | Actual raw-derived exposure lows15M/15M/20M/60M, correct high and basis; V8 mixed/unpriceable controls; split/resolved unit lows remain supported, not proxy values |
| A03/applicability | Refusal-only orphan PiApi/NotApplicable/NotReported/AtLeast0; subscription+dollar and missing-launch unresolved; genuine clean subscription/nonattempt rows; no convenient exclusions or S/V overrides |
| A04/wire | Actual175 corpus objects, all constructor shapes/reasons/bases/derivations and new ownership references; U08 Missing/Unrecognised distinction; no generic internal variant projection |
| A05 | No writable S decision constructors; injected one fails; wrong owning C-validity output fails independent assertions; primary validity separate from auxiliary display |
| A06 | USAGE-PRESENCE/type/cardinality/candidate/coverage/continuity changes reach V/F/X/R, views, actual validator and applicable consumers; stale acknowledgements rejected; null remains unestablished |
| A07/A08 | Malformed identities preserved, complete priced multiidentity positives finite; value uniqueness never evidence uniqueness; byte-distinct JSON-equivalent reservations conflict; scoped exact copies idempotent; independent identical-byte incarnations additive |
| A09 | Standard Draft2020-12 canonical/wire/evidence schema positives and negatives agree with Rust; distinct StageId/BoundaryId; input/output/assertion schema interchange rejected |

The actual Python validator must be qualified: `table_x:473` currently forces wrong zero low, and `raw_reservation:438–449` deduplicates normalized JSON. New Rust assertions cannot qualify that consumer. Update or simplify it only after design adoption, then independently test its actual entry points against correct positives and self-consistent wrong outputs/rollups. Preserve legacy check/selftest/revisions/sabotage requirements and first failures. Register its actual predicate/wire dependencies.

## 8. Conservation and remaining acceptance

The compositional argument is now local: origin ownership is total/unique; C changes only partition one parent into nonnegative component-preserving children; candidate/component highs never disappear; X takes the per-unit maximum; R sums active units. Only F's complete, nonoverlapping accepted coverage can lower those holds, under the explicit owner amendment. Identity/amount diagnostics never grant release. Scope-only changes cannot merge; continuity prohibits lost origins. Supported low and basis are independently defined per unit, so R does not reconcile a missing global envelope.

The residual semantic risks are concrete: completeness of financial-candidate predicates, admissibility of definite auxiliary co-reference for zero-known anchor coverage, continuity provenance realization, and full-corpus unit-granularity/attribution consequences. Their acceptance witnesses above must pass independent review; no convenient zero anchor may disappear merely to preserve a positive total. If those consequences contradict an approved case's protected semantics, report that exact conflict to root before implementation rather than restore global interpretations.

D3 remains fixed declarations/schema/rendering plus independently authored bounded raw arithmetic/ownership assertions. No financial reference reducer, coverage solver, arbitrary raw evaluator, production adapter or workspace integration is permitted. This is a coherent contract candidate and falsifiable local ownership model, not full executable raw conformance or proven live witness realization. Current output qualification and independent production provenance remain unestablished.

After fresh Astra xhigh acceptance/root adoption only: new isolated subject, disjoint spec/oracle work, exact joined qualification/freeze, currentA/C and relevantB clearance. Current architecture gate and all original rejection evidence remain; infrastructure and planning rejections do not add implementation rejection counts. No implementation is authorized by this document.
