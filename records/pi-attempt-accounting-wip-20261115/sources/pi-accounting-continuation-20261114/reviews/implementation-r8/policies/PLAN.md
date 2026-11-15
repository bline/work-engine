# Candidate: immutable holds, monotone atomization, independent low credits

Status: design formation only; fresh Astra xhigh judgment and root adoption are required. This document does not accept R7, admit implementation, clear Module A/B/C, or establish a live producer. Root01a1d6e2 is sole collector and acceptance owner. Input authority is this directory's immutable PACKET.md, ROOT-POLICY-R5.md and the remaining P1–P6/D3 dispositions. Exact evidence is bound in SOURCE-BINDINGS.json. The former R5 plan is rejected material, not architecture authority.

## 1. Outcomes and explicit policy changes

The protected outcomes are: evidence-only correlation changes cannot lower held highs or turn an unresolved high into a finite bill; complete affirmative coverage may release exactly its covered obligations; independently distinct obligations remain additive; supported usage survives without pretending that evidence positions are independent bills; final admitted facts produce the same ownership and output in either discovery order or all at once. Temporal closure, identity diagnostics and budget admission do not decide money.

Retain root's explicit amendments: initial usable dollar rid claims share a TicketClaim in an expressly admitted immutable initial reference scope even before a reservation exists; complete eligibility may become established when any last missing prerequisite arrives. Thus initial scoped orphan500M+500M is AtLeast500M, an unresolved policy hold, and late source-earlier R can turn orphan AtLeast15M into Exact15M when the complete witness exists. These are changes to the old literal INV-F3/I1 restrictions, not assertions those restrictions were preserved unchanged.

This candidate additionally proposes two explicit semantics for root/reviewer judgment:

1. Later producer identity is refinement evidence, never a replacement base key. A positive identity/distinctness fact triggers atomization of the entire affected initial base. This intentionally can raise an incomplete r500M/s15M base from500M to515M even when both later prove to name P. They remain515M until F supplies complete eligible coverage. Atomization conserves holds; it does not assert two physical bills.
2. R no longer sums the local exposure lows of conservative hold units. It computes a supported floor from separately justified low credits. Uncertain copies worth15M keep two15M holds but one15M aggregate floor; proven independent bills keep30M. This changes aggregate-low semantics and its evidence/wire contract. Local A02 exposure lows remain15M/15M/20M/60M. It does not change usage valuation into proxy money.

Neither change authorizes a general interpretation layer, raw reducer, coverage solver or arbitrary raw evaluator. The model below needs fixed assignment, two Boolean refinement predicates, finite memberships, max/min and additive arithmetic. Existing E→C→{S,V}→F→X→K remains; T owns attribution and R owns aggregation/provisionality.

## 2. Owned facts and admission

Three schema kinds remain distinct and are rejected if interchanged:

- AccountingInputProof: immutable raw origin inventory and externally admitted reference/duplicate/physical-identity/linkage guarantees. No money, C decision, eligible flag, chosen partition or output coverage.
- AccountingResult: computed C/V/F/X/T/R records, including derived ownership, eligibility/coverage, components and supported-low calculations.
- AccountingExpectedAssertion: independently authored raw-policy expectations used to challenge the result. It is never supplied as input proof or release authority.

E owns framing, exact original source order and byte-preserving field states. An Origin is (lineage_id, source_origin_id, original_record_position, raw_bytes_digest). Source revision digest is a locator, not a new lineage. Reobserving that same origin is always idempotent. Equal decoded JSON, equal amount, response_id, rid or n is not origin identity.

The external provenance owner supplies an immutable InitialReferenceContract per origin: admitted scope/namespace, duplicate rule and applicable writer/fixture qualification references. The binding is not inferred from which row arrived first. A later source revision cannot replace it. Admission states are conditional_fixture, established_producer, or unestablished, each with exact payload/extent/assumption references. Raw labels, hashes or verified:true cannot mint admission.

A conditional fixture declaration is a test harness input describing the synthetic world and exact origins. It establishes behavior conditional on that declaration only. An established producer grant must bind the actual incarnation, producer revision, relevant execution/recording extent, observer/authority, and the guarantee being consumed. Namespace uniqueness, exact-copy equivalence, physical-bill independence, source completeness and row co-reference are separate guarantees. Supplying one grants none of the others.

Missing or conflicting provenance remains explicit; it cannot release money. A contradicted immutable scope or duplicate guarantee stays a conflict, triggers conservative refinement below, and blocks F. No live grant/verifier/authority channel is established by this S1 candidate. Current corpus scope declarations remain conditional as root selected.

### Concrete record boundaries

All IDs and refs below are nonempty canonical strings; monetary nano values are nonnegative decimal strings; arrays of refs are canonical sorted sets except the original-position inventory. Missing optional evidence is explicitly absent/unestablished, never inferred true. Digests authenticate exact referenced bytes. These are declaration interfaces, not an executable raw implementation.

    AccountingInputProof {
      kind: accounting_input_proof, version, lineage_id,
      origins: [{origin_id, source_origin_id, original_position,
                 raw_bytes_sha256, source_revision_ref, initial_contract_ref}],
      initial_contracts: [{contract_id, scope_id?,
                 reference_namespace: dollar_ticket|admission|unestablished,
                 duplicate_rule: scope_exact_line|same_origin_only, grant_ref?}],
      grants: [{grant_id, guarantee_kind, admission: conditional_fixture|established,
                 exact_payload_ref, producer_revision_ref?, incarnation_ref?,
                 covered_origin_refs, covered_extent_ref, authority_ref,
                 observer_or_harness_ref, assumption_refs}],
      identity_bindings: [{origin_ref, producer_attempt_id, grant_ref}],
      exact_links: [{from_origin_ref, to_origin_ref, grant_ref}],
      independence_frame?: {frame_id, currency: USD, grant_ref,
                 slots: [{slot_id, bill_occurrence_ref, grant_ref}],
                 origin_slots: [{origin_ref, slot_id, grant_ref}],
                 distinct_origin_slot: [{origin_ref, slot_id, grant_ref}]},
      continuity: {predecessor_subject_refs, predecessor_origin_inventory_ref?,
                 preserved_initial_contract_refs, preserved_fact_refs,
                 exact_origin_map_ref}
    }

guarantee_kind is a closed schema enum: initial_reference_scope, exact_copy_equivalence, producer_attempt_binding, exact_row_coreference, physical_bill_independence, origin_slot_binding, origin_slot_distinctness, or continuity. Each grant's payload must define the promised relation/extent for that kind; no grant of another kind substitutes. Unestablished claims remain raw evidence/diagnostics without an admitted grant. An external admission channel—not a row in this payload—determines which conditional/established grants the consumer may trust. A conditional harness may supply several kinds only with separately stated assumptions.

    AccountingResult {
      kind: accounting_result, version, input_proof_ref, lineage_id,
      origins: [{origin_ref, base_id, evidence_atom_id, active_high_unit_id}],
      bases: [{base_id, origin_refs, atomize, atomize_fact_refs,
                 break_copies, break_copies_fact_refs}],
      units: [{unit_id, state: active|inactive, origin_refs,
                 successor_ref?, exposure_ref}],
      primary_validities: [{obligation_id, distinct_reservation_refs,
                 distinct_settlement_refs, validity_result, applicable_check_refs}],
      financial_candidates: [{usage_origin_ref, context_origin_ref, predicate_ref}],
      definite_links: [{from_origin_ref, to_origin_ref, admitted_input_link_ref}],
      coverage_decisions: [{coverage_id, obligation_id?, status: accepted|rejected,
                 unit_refs, monetary_origin_refs, anchor_origin_refs,
                 validity_ref?, valuation_ref?, definite_link_refs,
                 retained_component_refs, result_unit_ref?, reason_refs}],
      low_rollups: [{scope_ref, seed_origin_refs, alternative_origin_refs,
                 single_floor, credits: [{slot_ref, alternative_origin_refs,
                 low, valuation_refs}], independent_floor, selected_low,
                 credit_mode: independent_slots|single_ambiguous|none, basis}],
      continuity_result, failure_refs
    }

AccountingExpectedAssertion has kind accounting_expected_assertion, version, raw_subject_ref, policy_refs, authored_by, exact_subject_refs and finite assertion records {claim_id, raw_derivation_refs, expected_fields_or_relations, negative_control_refs}. It refers to expected values, never masquerades as grants/results. Its author owns expected assertions; C/F/V/R own the computed fields. The schema and independent checks enforce total/unique origin ownership, exact-ref application and the distinctions above.

## 3. Canonical high ownership and refinement

C derives initial base IDs solely from the immutable origin contract and each origin's raw primary reference:

- A usable dollar rid in an admitted initial reference scope maps to TicketClaim(scope,rid), including orphan settlements.
- Admission references use a separate namespace.
- An unestablished scope, unusable reference or attempt-evidencing auxiliary row maps to OriginBase(origin). An unresolved zero-known attempt contributes an AtLeast0 anchor. NonAttemptEvidence, including the approved preflight Refused class, remains inventoried without a financial hold.
- Producer identity does not override these base IDs, even when its proof is delivered in the first batch. It is always a refinement fact under the same rule.

Within a consistent admitted exact-copy rule, EvidenceAtom(scope,raw_line_bytes) contains all byte-identical origins in that scope; every raw origin remains in the map. Without that duplicate grant, the atom is the original origin. Whitespace/key-order changes produce different evidence atoms. Same-origin reobservation never adds an atom. Distinct independently bound incarnations are never exact copies under one grant.

For each initial base B, C computes these final-fact predicates:

- Atomize(B) is true iff the admitted fact set contains any non-base producer-attempt/bill-identity binding for an origin in B, an explicit independently-distinct-origin fact affecting B, or a recorded contradiction to B's initial scope/duplicate guarantee.
- BreakCopies(B) is true iff positive distinctness or a recorded scope/duplicate contradiction separates origins that the initial exact-copy rule would equate. BreakCopies implies Atomize.

These are presence predicates over immutable admitted facts and retained conflicts. New evidence can make them true, never false. Later matching P labels do not remove the first binding fact or the predicate. With no predicate, B is one hold unit. With Atomize only, every immutable EvidenceAtom in B is one hold unit. With BreakCopies, every original origin in B is one hold unit, except true same-origin reobservation. IDs are deterministic from lineage/base/atom identity, not an observed transition counter. No (P,residual) grouping exists.

This deliberately atomizes all evidence atoms when the first refinement fact is admitted. A later proof for another origin cannot reunite siblings. A complete batch containing both P proofs selects the same atoms. New origins either join their already fixed exact-copy atom or introduce a new atom; they cannot join two formerly distinct atoms. BreakCopies only subdivides an exact-copy atom. The normal form is therefore a refinement under evidence inclusion, not a memory latch. Its inputs include the immutable initial contract; inventing a different initial contract is a different subject requiring admission and continuity validation.

Every positive reservation/charge/usage-high component follows its owning evidence origin. A context origin is read-only pricing evidence, not a second copy of usage. Splitting partitions owned components and anchors. Nonnegative arithmetic gives sum(child maxima) >= parent maximum. The parent is not also emitted. No C merge, later P match, subject rename or scope upgrade can reverse this increase. Only F coverage may retire these holds.

C also maintains logical primary obligation groups separately from hold units. Primary origins with the same admitted initial ticket key form one logical group. Origins without such an admitted key may join a logical group only through an admitted exact ticket/producer-attempt link; transitive closure of those exact links is ordinary equality closure, not a financial partition search. A link crossing incompatible admitted ticket namespaces, independently distinct physical identities or contradictory producer grants is a conflict, not an instruction to pick a cheaper group. Auxiliary links only establish anchor coverage and context; they cannot change primary R/S membership or validity. A unique R/S obligation can therefore span several hold atoms or fallback bases while C still owns one validity witness. Logical co-reference never merges high holds.

Distinct-row reservation cardinality is counted by admitted evidence equivalence, never by distinct amounts. R=max well-formed reservation amounts. A newly linked extra primary row can make the logical witness conflicting and invalidate eligibility; it cannot erase a row. Auxiliary display attachment cannot mutate that witness. Equality closure is a declared result relation; S1 does not implement a general raw correlator to compute it.

Concrete r/s refinement: r500M and s15M, incomplete usage, initial TicketClaim. No identity proof: one hold AtLeast500M. Add r→P: two atoms, AtLeast500M plus AtLeast15M = AtLeast515M. Add s→P: same two atoms, same515M. Reverse proof order gives515M then515M; all-at-once proofs give515M. Both matching identities merely support F co-reference; incomplete usage prevents release. If complete eligibility later arrives, F can cover both and produce Exact15M. This is the only515→15 reduction in that sequence.

Scoped exact copies remain one evidence item under a consistent conditional duplicate contract, whether discovered together or separately. Two unknown-scope copies remain two high holds. If later independently distinct incarnation evidence contradicts their apparent copy status, BreakCopies yields separate holds and blocks the contradictory subject's release. None of these high rules proves low independence.

## 4. Financial context and affirmative co-reference

C computes financial relevance independently of RS display attachment. An identity-context origin c is a financial candidate for usage u when one of these positive predicates holds:

- u and c have the same admitted logical source and the same usable raw n;
- an admitted explicit ticket, measurement or producer-attempt link names both;
- they share a logical source and at least one lacks a usable n, unless their immutable initial origin contracts contain a qualified positive unrelatedness relation. This is a fixed contract field, not a fact chosen according to arrival order.

The last rule is intentionally broad. Later display uniqueness, another reservation, nearest-row/order choice, an unsettled filter or a later narrower candidate set cannot remove a candidate already justified by these raw facts. Later proof of unrelatedness is recorded and may help low independence or F but does not delete a previously justified priced high. Scope qualification does not rewrite initial source identity. No cross-source candidate follows from equal amounts/model strings alone. Malformed/unpriced context is retained, and a priced context is not itself an additional usage amount.

V applies existing P1/P2 pricing, V8 alternative-observation treatment and complete fully priced multiidentity bounds. A malformed model7 remains unpriceable; independently computable priced highs survive. Financial candidate expansion cannot erase a known high. A known positive usage low can be reduced by a newly unpriceable alternative under the existing V8 rule; high conservation and low support are different properties.

DefiniteLink is stronger than FinancialCandidate or an RS attachment. It is admitted only from:

1. positive immutable ticket/producer-attempt identity on both origins, under an established or conditional immutable namespace/writer grant that links those exact rows; or
2. an externally qualified execution/recording witness mapping the exact auxiliary origin to the exact request/ticket, with closed relevant extent and the writer/execution guarantees sufficient to exclude reuse, omitted competing attempts and duplicated origin identities.

The second is a bounded evidence interface, not permission to infer a mapping from observed uniqueness. A source-completeness label alone, observed n uniqueness, response_id equality alone and RS1–3 alone never suffice. The current threaded producer increments accepted and decrements it on preflight failure (original1351–1360), so n can be reused; it supplies no usable n-only uniqueness theorem. Current auxiliary rows commonly lack ticket IDs. No live DefiniteLink for those rows is established here.

The admitted link payload contains guarantee_kind, source/producer/extent refs, exact from/to origins and external authority/observer refs. C verifies admissible scope/application and exposes the computed link relation with provenance; F consumes it. The fixture expected lane cannot manufacture that relation. Absence means retain the separate anchor, including its unresolved tag even when its numeric amount is zero.

RS example: R1/charge50M with incomplete10,000-token Anthropic usage has sonnet20M/fable80M financial high. A refusal's display detachment after adding same-n R2=1M cannot remove fable context. Final owned holds are80M and1M, plus any unmatched AtLeast0 anchor: aggregate AtLeast81M, never51M. R1-first, R2-first and all-at-once have the same final candidate pairs and result.

## 5. Supported lows: an explicit separate interface

An exposure row's low remains its local supported usage bound, computed by V/V8 from its own monetary observations and applicable identity contexts. It is never a reservation/charge amount or forced zero because S rejected/conflicted. A reservation-only or zero-known anchor has low0. Atomization may produce several rows with the same local low; those rows are conservative holds, not independently billable attempts.

R receives an optional single immutable IndependenceFrame per accounting lineage and rollup currency. Its external owner guarantees that admitted physical-bill slots are pairwise distinct in the respect needed to add bill floors; supplies positive exact slot-to-origin bindings; and binds the physical occurrence, scope, extent and writer/observer authority. A frame lists no monetary values or chosen outputs. Different frame namespaces are not automatically additive. Missing frame or missing slot proof means unknown independence, not zero usage. Response IDs, source positions, separate files, distinct rid and conservative atom IDs alone do not establish physical-bill independence.

C exposes the frame's admitted slots and a finite IndependentlyDistinct(slot,origin) relation. A positive distinctness fact must carry the same admissible physical-bill evidence; absence means the origin might describe that slot. A slot has at least one positively bound usage observation. All slot bindings trigger Atomize in their high bases, so independently supported low multiplicity cannot hide beneath one collapsed high unit.

For each R rollup scope, Seed contains the usage observations whose bill ownership is established for that scope, once by origin. U is exactly the usage-origin set in T's admitted-or-candidate ownership inventory for that rollup; candidate origins are included as possible alternatives, even when their own usage is malformed/unpriceable. Candidate ownership never creates a seed or a positive owned slot. T's existing scope contract owns exclusion from this inventory. A discovered possible copy outside it invalidates that exclusion and the dependent low qualification; a filename or display detachment cannot exclude it. This does not turn unrelated, E-excluded/non-evidence usage into a financial observation. V8Low(A) is the existing V8 minimum of supported observation lows for nonempty alternatives A; it is0 if any alternative has no supported minimum, and0 for no supported usage. Alternatives are observations of uncertain co-reference, not summed bills.

For each admitted slot p whose bill ownership is established for this scope and which has a positively bound usage seed, Alternatives(p) consists of every u in U not affirmatively proved physically distinct from p. This includes its definite observations and all ambiguous potential copies; it excludes a proven other slot's observations. V prices the complete applicable context for each alternative. Define:

    single_floor = V8Low(U) if Seed is nonempty, otherwise0
    independent_floor = sum over admitted p of V8Low(Alternatives(p))
    R.low = max(single_floor, independent_floor)

The second sum is permitted only because the slots themselves are pairwise independently billed occurrences. Unknown observations cannot be added as another slot and cannot be silently ignored as possible alternatives. Taking min for each slot is deliberately conservative: it may understate the tightest possible floor but cannot double uncertain usage. The max combines two independently valid lower bounds, not alternative high worlds. No partition search, maximum independent set or general financial solver is needed. If a frame/binding is contradictory or unestablished, no derived slot addition is admitted; the single-floor route remains available and the failure is disclosed.

Examples in M nano:

| Observations / proof | Local lows | Aggregate supported low |
|---|---|---|
| two unknown-scope identical15 observations, no frame |15,15|15, not30 |
| same-origin15 reobserved |15 once|15 |
| two scoped exact copies under consistent duplicate contract |15 once|15 |
| two exact-byte15 observations with independent physical slots P,Q |15,15|30 |
| known P15 and Q20, plus unbound15 possible copy of either |15,20,15|30 via min(15,15)+min(20,15); single floor15 |
| no supported usage, only reservations/anchors |0|0 |
| A06 / P133-A09 / P130-R05 / V01-undercharge individually |15 /15 /20 /60|same respective floor, never forced0 |

This plan explicitly favors a safe floor over tightness in ambiguous cross-slot cases. It retains V8's behavior if an unpriceable possible alternative removes support; no positive floor is invented from proxy holds. Low basis is usage_valuation when positive and none when0. The result records winning mode, slot/alternative membership, per-credit values and derivations; exact ties prefer independent_slots when nonempty, otherwise single_ambiguous. R's low basis histogram counts chosen credits, not local hold rows. High basis/ties remain UsageValuation, ProxyReservation, ProxyCharge. Low is not a finite spending ceiling and cannot clear an AtLeast high.

Attribution: T supplies a rollup's admitted/possible origin memberships. R recomputes this formula within each scope, rather than summing child session/objective lows or reusing a global credit whose ownership is ambiguous. A slot contributes to an owned rollup only when its bill ownership is established for that owner; possible-owner reports may show it as candidate but cannot establish owned low or admission headroom. Same active unit is counted once in each applicable high rollup. This avoids duplicate lows across candidate ownership and parent/successor views.

## 6. Exact F coverage and release

F alone constructs covered replacements, after C-validity, S forwarding and V. A BudgetedCoverage requires exactly one distinct reservation and settlement for the logical obligation, all applicable local checks, source-earlier R, admitted reference scope, complete provider usage, established fully priced identity/tier sets, and affirmative co-reference for every covered auxiliary anchor. Multiple distinct R/S, bad source order, malformed local model/status/basis, undercharge or unpriced candidates cannot be discharged by favorable added rows. NoReservation is a missing prerequisite and may be discharged under root's explicit amendment.

Coverage identifies obligation, complete current hold-unit IDs, all monetary/anchor origin IDs, C-validity ref, complete valuation ref, DefiniteLinks and retained known charge/context components. It covers each selected hold whole; partial overlap with another unresolved origin prevents release. Independent physical obligations cannot be absorbed together. Every monetary/attempt origin has exactly one active high owner before and after F.

Eligible output replaces exactly those holds with one deterministic resolved unit identified by sorted origins/obligation/policy. High is Exact(max(retained actual covered charges, complete usage/context high)); reservation components are released only as the exact covered obligation's holds. Other charges/contexts/anchors remain. Low evidence is recalculated under section5, not added by replacement count. Inactive parents/atoms stay in the ownership map, not as another exposure row.

Disjoint coverages compose. Identical obligation/coverage/financial payload is idempotent. Nonidentical overlapping or contradictory payloads are CoverageConflict: retain affected original/refined holds, no order winner. Shared read-only context refs do not make monetary coverage overlap. If new immutable conflict invalidates previous eligibility, normal form returns to its held units; known charges/context are retained. No previous-status latch is involved.

The existing genuine complete unbudgeted-usage route remains a separate UnbudgetedCompletion result. It cannot relabel a dollar settlement rejected for NoReservation as unbudgeted. A complete unbudgeted request can be locally Exact while an unlinked zero-known auxiliary anchor keeps aggregate AtLeast. Discharging such an anchor requires the same affirmative DefiniteLink and complete coverage; weak RS attachment cannot borrow this route to erase it.

Late discovery example: S at original position2 has complete15M usage, valid local fields and dollar charge15M/reserved500M; source-earlier R at position1 is initially absent. S-only is Rejected/NoReservation/AtLeast15M. R-first is AtLeast500M. With the same final admitted facts each route becomes Valid/Eligible/Exact15M. Bad original order or a second distinct R/S leaves AtLeast. Compare high tags as well as numbers.

Continuity binds every predecessor origin, immutable initial scope/duplicate declaration, atomization-trigger fact and retained conflict into the successor. Changes are added evidence, canonical refinement or exact F coverage. A changed digest alone does not create fresh bases. Omission, rewritten initial bindings, discarded trigger facts or unexplained many-to-one ownership fail admission; the consumer retains the last admitted financial subject and discloses new unqualified evidence rather than publishes a lower fresh balance. This is explicit input provenance, not hidden discovery history. Continuity/writer guarantees remain conditional/unestablished live.

## 7. Exact owners, wire and qualification consumers

| Owner | Sole semantic output / allowed consumption |
|---|---|
| E/provenance admission | Raw origin/order/field states and externally admissible guarantees; no decisions or money |
| C-primary/C-validity | Logical primary evidence/cardinality and one immutable validity witness |
| C-accounting | Fixed bases, duplicate evidence classes, two refinement predicates, high-owner map, financial candidates, admitted links/low slots; no release |
| S | Time, execution, forwarding; forwards validity_ref exactly, with no separately editable SettlementDecision constructor |
| V | P1/P2 billing/auxiliary presence, identity/tier/split valuation, V8 alternatives and credit pricing |
| F | Complete exact coverage and amount source; no raw eligible input |
| X | One exposure per active high unit; per-unit max, retained local supported low, basis |
| K | Findings only, downstream of money |
| T | Ownership/admission/candidate attribution by origin/unit/slot; cannot alter amounts |
| R | Unique active high sums, supported-credit lows by scope, commitment/provisionality; cannot discard AtLeast0 |
| Specification builder | Canonical declarations, exact serializers/schema/rendering/static checker; no oracle destination |
| Independent oracle builder | Raw-policy assertions/finite controls, preserved input audit and actual Python consumer qualification; no importing generator financial decisions |

The actual corpus wire is the authority to map: exposure.pool; high.kind/nano/basis; decimal nano strings; valuation low/high strings and derivations; obligation state/amount_source/by and qualified reason strings. Use explicit external field maps and conversions in declarations, not an internal variant-enum round trip. U08 emits TierUnestablished(Missing) or (Unrecognised) exactly. None obligations preserve cause/amount_source:none; Closed preserves by.

Add an AccountingResult reference per active attempt and an explicit rollup supported_low_evidence_ref. The fixed accounting record declares initial bases, all origins/equivalence classes, trigger facts, active/inactive ownership, contexts/links, validity ref, F decisions, slot alternatives and R credit_mode/value/basis. This new evidence and R-low meaning is a versioned schema/interface amendment. Existing local exposure shapes stay concrete; attempts enumerate financial units, and schema states that a unit is not necessarily an independent physical attempt. An R consumer may not recover aggregate low by summing attempts[].exposure.low.

Remove S decision outputs from writable tables and generate forwarding of C's witness by reference. The static tool must reject an attempted S constructor, not merely compare a duplicated pass/fail mapping. Independently wrong C validity remains a financial qualification negative.

Register actual semantic dependencies, including usage presence/type/cardinality, identity/tier, duplicate evidence identity, atomization, candidate predicates, DefiniteLink, low frame/alternatives, exact coverage/continuity and wire projections. V/F/X/R, contract/schema/corpus views, Python checks, independent assertion qualifications and applicable acknowledgements must be reached. Mutate each protected dependency and verify actual affected_consumers/digest invalidation; null acknowledgements remain unestablished. Whole-file byte drift alone does not establish consumer invalidation.

A09: publish a real Draft2020-12 schema with distinct StageId and BoundaryId. Validate canonical and wire/evidence positives/negatives with a standard validator and require the Rust consumer to agree. Fixed schema bytes and permissive String deserialization are not schema enforcement. Input/result/expected kind interchange and dangling exact refs fail.

The actual Python validate_corpus.py is a required consumer. Its table_x rejected/conflicting branch currently forces low0, and raw_reservation normalizes JSON for duplicate counting. The accepted implementation must correct those actual entry points for the new policy and R credit semantics, or explicitly remove a superseded semantic check and register the replacement consumer with equivalent tested coverage. Keeping the wrong Python check and qualifying only Rust is forbidden. Preserve check/selftest/revisions/sabotage requirements; independently construct correct positives and consistent wrong attempt+rollup outputs for those exact entry points.

## 8. Full-corpus effects and finite falsification

The predecessor census reports175 cases/186 source containers/282 usable dollar-rid claims/four collisions/no incarnation fields. It is attributed census evidence, not proof of live scope, all raw classifications or current full-corpus conformance. Existing raw inputs stay immutable; profile projections and conditional harness grants are separate.

Complete priced multiidentity bounds remain finite: V01-usage-settlement-other-response-model has one primary request with20M–80M valuation, unique R/S and no auxiliary anchor; it must remain Exact80M, low20M, downstream IdentityContradiction. Numeric model7 must instead remain unpriceable/ineligible. N01's partial refusal+request keeps80M (or100M reservation above both), low20M, and unresolvedness; a separate unmatched0 anchor can add a row without changing the numeric high.

VC07 currently uses RS1 alone to present an unbudgeted complete request plus stream error as one Exact15M attempt. Under this candidate, its unchanged raw input has a locally Exact15M request and separate AtLeast0 anchor, hence aggregate AtLeast15M. That is an intentional stronger release-proof consequence requiring root/reviewer adoption, not a false claim the prior positive's aggregate tag is unchanged. A separate exact-payload conditional DefiniteLink positive can demonstrate discharge toExact15M without modifying historical raw bytes or claiming live proof. C3 exact reservation copies remain idempotent under consistent duplicate scope; byte-distinct equivalent JSON remains conflicting. Unknown copies remain separate highs and one low floor.

Qualification must classify every corpus case's changes by protected policy, including extra zero anchors, full multiidentity positives, local versus rollup lows and attribution. Do not replace raw source with an expected-key selector or hide effects through convenient fixture grants. Independent raw selection remains P6 and unreadable E-owned inputs stay disclosed. No profile pass is promoted to full raw conformance.

Every row below challenges a self-consistent wrong output, including matching wrong rollups/ownership records where applicable. Assertions derive from pinned raw facts and owner policy before expected outputs are revised; they do not merely replay declaration outputs.

| ID | Exact scenario / wrong candidate to reject | Required outcome |
|---|---|---|
| H1 | scoped two orphan500 settlements; append R; wrong1000→500 history or claimed one physical attempt | initial policy hold500 and final500, unresolved conflicts explicit; all discovery orders identical |
| H2 | r500/s15 incomplete, r→P then s→P; wrong515→500 or all-at-once500 | two atoms515 until eligible coverage; reverse/all-at-once515 |
| H3 | prove distinct equal-byte incarnations; wrong copy-dedup500 | independent origins remain high-additive; duplicate contradiction blocked |
| H4 | same-origin repeated; or scoped exact-copy repeated; wrong extra hold | idempotent with raw origins retained; key-order/timestamp-different R rows conflict |
| L1 | two uncertain15 copies; wrong low30 with matching R totals | high holds retained, aggregate low15; local15 not forced0 |
| L2 | two certified physical15 slots; wrong low15 or use atom IDs as proof | low30 only with admitted independent frame; absent frame15 |
| L3 | P15,Q20 plus unbound15; wrong45 by adding unbound observation | low30 from explicit conservative alternatives, no extra slot |
| L4 | A06/P133/P130/V01-undercharge expected local lows0 and matching rollups | respective15/15/20/60M with usage basis, retained correct high |
| F1 | late source-earlier R; wrong permanent Rejected or premature incomplete Exact | both routes/all-at-once Exact15 only with complete coverage |
| F2 | malformed model7, second R/S, source-later R, undercharge, incomplete/non200/missing tier; wrong favorable F output | relevant immutable failures prevent release; retain highs and actual raw malformed state |
| F3 | partial reused-n stream-error belongs another attempt; wrong Exact15 via RS1 | separate AtLeast0 anchor keeps aggregate AtLeast15; positive admitted exact link permits discharge |
| F4 | omit charge/context/origin; partial/subset overlap; contradictory identical coverage payload | coverage rejected and original/refined holds retained; disjoint/identical exact coverage composes/idempotent |
| C1 | RS context80 detached after new R1; wrong final51 | final81 plus unresolved zero anchors in both routes/all-at-once |
| C2 | renamed revision drops trigger/origin or overwrites initial scope; wrong lower fresh balance | continuity unestablished; last admitted financial subject retained |
| A3 | subscription-only identity-refusal / missing-launch dollar / orphan / malformed rid; wrong excluded/codex/no amount | total PiApi unresolved applicability, legal S/V states, AtLeast0 where nothing numeric known |
| W1 | internally valid variant/bound/splits wire; wrong qualified U08 code | actual pool/kind/nano/basis/derivations/obligation names accepted, wrong external values rejected |
| O1 | inject Valid decision into writable S-fail branch; wrong owning C witness | S constructor rejected structurally; C mutation rejected by independent raw assertion |
| D1 | change usage presence/cardinality/type/link/low policy while old contract/R/Python ack still current | actual dependency closure invalidates each affected consumer; null never clears |
| S1 | canonical C-primary boundary rejected by standard schema or Rust accepts wrong boundary ID | Draft2020-12/Rust agree on valid/invalid canonical, wire and evidence kinds |
| P1 | only Rust rejects low/dedup mutant while actual Python accepts it | actual Python entry points independently reject consistent wrong low/dedup/rollup and accept repaired positives |
| M1 | complete priced multiidentity20–80 plus IdentityContradiction incorrectly made unpriceable | finite approved bound survives; unrelated/unlinked anchor remains separately unresolved |
| G1 | fixture verified:true or finite mock residual clears AtLeast/tag/unknown authority | only exact trusted test_mock input proves conditional P5; no live gate or false admission |

For H2/F1/C1/F3, keep explicit A-first, B-first and all-at-once fixture sequences ending with byte-identical admitted fact inventories. Compare full unit/coverage/candidate/credit maps, high kind/number/basis and lows, not just final amounts. Intermediate high decreases require the named complete F witness and exact covered origins. No generic all-subset reducer is proposed; these are independently authored bounded assertions.

## 9. Implementation boundary and what remains unestablished

After design acceptance/root adoption, use a new isolated subject; preserve R7/R3–R5 and all failure evidence. Keep coherent spec and oracle ownership disjoint as P1–P6/D3 already require. Spec owner implements declarations, exact static witness-forwarding interfaces, schemas/rendering and structural checks. Independent oracle owner derives finite raw assertions, updates corpus expectations with attributed reasons, and qualifies actual Python plus Rust consumers. Root owns join/freeze/review disposition. A shared serialization shape is permitted; generator logic cannot supply expected arithmetic or release decisions.

D3 permits one standalone Rust spec-check crate, declarations/schema/rendering/static checks and separately authored bounded raw assertions. Do not implement C/V/F/X/R as a production or reference raw reducer, coverage/partition solver, raw expression interpreter or workspace adapter. Structural partition/ownership/ref checks can validate supplied computed examples; that is not full executable raw-to-output conformance. Complete production behavior, live producer identity/co-reference/independence/currentness, genuine verifier authority and live budget enforcement remain unestablished. If acceptance requires full executable conformance now, root must explicitly resolve it against D3; a static pass cannot satisfy that claim.

The main cost of this simpler design is deliberate overholding after identity atomization, conservative ambiguous low floors, and additional unresolved0 anchors where old RS display pairing claimed resolution. Those consequences are explicit and finite to falsify. No irreducible mathematical contradiction is identified under the amended policy; acceptability of these semantic consequences remains with root and independent review. A failed premise must return to that owner, not be hidden behind another normalization mechanism.
