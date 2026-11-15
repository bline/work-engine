# R4 candidate: conserved constituents and monotone association envelopes

**Status: architectural candidate, not adopted.** Packet97f961a1d34ec331e3b1d747bb18bd77f428d4e965012f7d4320f8c665595fc3; retained planner SID01a1d77d-43dc-7210-a674-615860bfb6d2; root01a1d6e2 sole collector/policy owner. R7 remains immutable/rejected; no R8, ordinary repair, reducer, production or S2a authority follows. R3 PLAN24312298 is preserved but superseded as a candidate by this complete R4 decision. Fresh Astra xhigh judgment and explicit root adoption remain required.

## Decision

Retain stable primary ticket claims, but make **financial constituent provenance and conservative association envelopes** the owner of monetary conservation. Current RS attachment is a presentation/diagnostic association, not the exclusive channel through which identities can affect money. C-validity still owns one result and S forwards it. Add one explicit **EligibilityWitness** that can become established when its final missing prerequisite arrives, regardless of discovery order. Its coverage is exact; it cannot release another obligation or silently delete charges.

This is a bounded specification architecture, not another collection of repaired rows. A stable-key-only design was inadequate: changing auxiliary attachment can remove monetary information even when every primary key survives. The R3 review also correctly exposes a contract contradiction: a pure final-set decision cannot both admit a newly established earlier reservation and prohibit all Rejected→Valid transitions or reserve I1's exception solely for additions whose row kind is settlement.

The proposal explicitly amends those semantics. It also makes source scope and accounting continuity real inputs/outputs. It does not claim full executable conformance under D3. If the owner requires that claim in S1, implementation remains blocked until D3 is dispositioned.

## Evidence and preserved findings

Immutable R7 subject `/home/bline/cbm-work/p137-s1r7/s1`, contract1.6-s1/r7, freeze manifestc1c40727648b40f3494b5444065cdef3a7824a42249426dc860dece95423ceda. R3 `SOURCE-BINDINGS.json` binds185 frozen source/corpus files and all175 current cases. New `SOURCE-BINDINGS-R4.json` binds the additional validator/rate evidence and complete DESIGN_R3 response SHA256e075d0c82fd46258e3a1a64ff747e54c5c743b52272609c37f2ebc9e3a1786af (seal d0cdecf4). The new review is a design rejection, not another implementation rejection or a history reset.

R3's custody observations remain valid: actual checker fd44debd impact of USAGE-PRESENCE omits contract/R; raw identity census finds175cases/186sources/282 positive-integer dollar-rid claims/four conflicting-reference groups/no incarnation fields. No unchanged probe was rerun here. All fresh numerical witnesses below are specification-derived bounded arithmetic, not reducer executions.

Direct source additions: `validator/validate_corpus.py:438–449` deduplicates parsed reservations using normalized JSON, whereas byte-distinct rows remain distinct under the contract. `:473` explicitly requires low0 for rejected/conflicting settlements, contradicting supported usage lows. `:466–480` demonstrates that this is an actual financial checking consumer. Graph coverage is outside_project for this file and the pinned rate file; exact source fallback was used. No full validator bypass or newly executed test is claimed.

## 1. Typed origin, scope and evidence interfaces

Every source supplied to the pure core is part of an **AccountingLineage**, which identifies the underlying set of obligations across subject revisions. A subject digest identifies bytes, not a fresh accounting universe. The following is fixed typed structure, with no arbitrary predicates or expression language:

```
OriginRef = { accounting_lineage, source_origin, record_origin,
              raw_line_sha256, source_revision_sha256, source_position }
ScopeWitness = FixtureDeclared { scope_id, fixture_world, session_id, turn_log,
                                raw_manifest_ref, assumption_ref }
             | ProducerBound { scope_id, producer_incarnation,
                               realization_ref, observer_owner,
                               observed_extent_ref, provenance_digest }
             | Unestablished { observation_scope, reasons, evidence_refs }
UnitIdentity = ProducerAttempt { producer_incarnation, attempt_seq }
             | TicketClaim { admitted_reference_scope, dollar_rid }
             | AdmissionClaim { admitted_reference_scope, admission_rid }
             | EvidenceAtom { source_origin, record_origin }
IndependenceWitness = DistinctProducerAttempts { left, right, evidence_refs }
                    | DistinctAdmittedTickets { scope, left_rid, right_rid }
                    | DistinctAdmittedSources { left_scope, right_scope,
                                                origin_noncopy_evidence_refs }
CorrelationEvidence = { display_attachment, financial_candidates,
                        independence, scope_witness_ref, duplicate_classes,
                        unresolved_multiplicity, constituent_refs }
```

FixtureDeclared is conditional model input, not ProducerBound. A producer row cannot mint either trusted provenance or a continuity certificate. Current fixtures derive their conditional scope from the existing `(case world, session_id, turn log)` container under C11; all175 cases remain conditional cases, not newly authenticated logs. Pinned writer facts still establish rid's per-budget serial/copy behavior, not one incarnation per arbitrary appended path. n is reusable after release and is not an independent identity. These limits remain explicit.

Deterministic identity rules:

1. A supplied affirmative ProducerAttempt identity distinguishes units by incarnation and sequence. Equal bytes, rid or amounts never collapse independently witnessed identities. No current175 raw row supplies that producer ID.
2. Within an admitted fixed reference scope, a usable dollar rid selects TicketClaim before any reservation is present. Admission rid has a separate namespace. Distinct admitted tickets remain additive. This is the proposed C2/F2 amendment from R3.
3. Without an admitted reference scope or usable reference, preserve EvidenceAtom constituents for every original monetary observation. Reservation and settlement atoms remain separate unless an exact EligibilityWitness later covers their proven common obligation. Ordinary scope improvement, an n match or an amount match cannot authorize that collapse.
4. Byte-identical repeated rows are one evidence item only within the same admitted duplicate scope and without affirmative independent origin identity. The equivalence key includes that scope/origin and the exact raw line bytes; parsed/normalized JSON equality is insufficient. Within unestablished scope, re-observing the **same OriginRef** is idempotent; equal bytes at independently distinct origins do not establish duplication. Keep uncertain duplication/multiplicity explicit and conservative.
5. SourceRevision hashes remain on exact observations. They do not replace source_origin or scope_id when a file grows. Appended rows retain old origin identities; changed raw bytes require an explicit change relation, not an evidence-addition claim.

No runtime E implementation is authorized. The core contract must expose which of these inputs it consumes, and the S1 schema/literal witnesses must represent each variant. Existing inputs lacking live provenance receive Unestablished rather than fabricated producer identity. Whether conditional FixtureDeclared admission matches the intended S1 acceptance remains a root decision.

## 2. Constituents, primary units and financial associations

A **FinancialConstituent** is an immutable referenced observation with its monetary role: reservation amount, claimed settlement charge, usage counts/observation, or pricing-context evidence such as a served model. It retains its OriginRef, field states, measure, candidate unit references and provenance. A price-context fact with no usage creates no dollar amount by itself. A row carrying actual measured usage or an amount keeps its own monetary origin even if its display attachment moves.

Within a fixed admitted scope, primary TicketClaim groups do not change because R appears. Without that scope, separate origin atoms remain constituents of the same accounting lineage. Their possible co-reference is recorded; it is not authority to erase them.

Two association relations have distinct semantics:

* **DisplayAttachment** is the current RS1–3 unique/preferred/ambiguous/orphan result. It may detach or choose another display parent as evidence grows.
* **FinancialCandidate** is a conservative pairwise may-affect relation. It does not apply RS uniqueness, first-unclaimed, no-valid-settlement or nearest-row filters. A same-scope usable n match between an auxiliary identity/error observation and a primary observation/reservation is a candidate even when another same-n unit exists. An explicit usable ticket/measurement reference supplies its corresponding candidate. Missing/malformed n supplies an unresolved candidate scope rather than a false exact attachment; where no narrower relation is evidenced, all compatible primary units in that source scope remain candidates. Cross-source associations are not invented; unestablished source continuity is handled by the origin/continuity structure.

Candidate predicates use immutable positive facts. Adding another reservation can add candidates but cannot delete an existing pair. Contrary evidence may remove confidence from DisplayAttachment or block eligibility; it cannot erase a priced candidate's contribution. A malformed or unpriced model remains explicit unpriceable evidence, with the maximum of previously priceable components retained as priced_high. Absence of a model is different from a malformed reported model.

V's conservative financial context is the union of intrinsic identities and all financial-candidate identities for its usage observations. Its range spans the permitted priced identity/tier/split combinations; adding priced alternatives cannot lower the high. Partial/unpriceable observations preserve priced high components. This expands financial relevance beyond the current selected RS attachment; it is an explicit conservative policy change. It may hold more than the most likely attribution. The measure remains conservative provisional exposure, never a verified provider bill. Sharing a contextual identity does not duplicate a monetary observation: each usage/charge constituent has one owner in each accounting interpretation.

Preserve approved complete established fully priced multiidentity finite bounds. A finite established candidate identity set may supply such a bound; a malformed/unpriced/unestablished set may not. Attribution ambiguity alone does not erase an already supported priced high. No K finding selects or suppresses components.

## 3. Conservative accounting interpretations and no reset

A stable primary key handles ordinary same-scope facts. To cover unestablished scope, later independent identities and replacement subjects uniformly, the financial owner also exposes **retention interpretations** of the constituent set. These are policy holds, not assertions that every interpretation remains a possible physical history.

Each interpretation assigns every monetary constituent to a unit exactly once, preserves affirmative independent units, and applies per-unit maximum rules. Reservation plus its settlement is not summed inside one proven unit; distinct independent units are summed. Unknown co-reference admits the separate-atom interpretation; a scope upgrade cannot simply delete it. A refinement showing independent attempts supplies a refined interpretation with those independent units. The conservative result joins their totals, rather than adding an old aggregate to its own refined constituent totals.

For a set of High values, define the conservative join as `High(tag, max nano)`, with tag AtLeast if any operand is AtLeast, otherwise Exact. This is an upper bound in the full MONEY order, including Exact100 versus AtLeast50. An accounting envelope is the join of sums over the admitted retention interpretations. Basis histograms remain attached to sums and interpretation/constituent references; a join identifies its determining contributors without inventing one aggregate basis.

This is a declarative mathematical contract, not permission to implement a partition enumerator or solver in S1. A future implementation may exploit the stable ordinary units and monotone pairwise relations; it must establish equivalent consequences. S1 can state the constituent/interpretation interfaces, render fixed declarations and challenge finite literal examples. It cannot claim an executable universal envelope or a production cost bound.

Conservation is required of an interpretation extension: every old monetary origin remains represented, with nondecreasing component envelope, and new independently known units remain additive. If a new identity refinement would lower a previously held interpretation, retain that interpretation as a floor alternative. Do not count its aggregate additively alongside the same constituents. It may be discharged only by the covered eligibility rule below. A generic scope or co-reference certificate is not that release rule.

### Continuity consumption interface

```
ContinuityWitness = SameOriginExtension {
   accounting_lineage, predecessor_subject, successor_subject,
   predecessor_origin_inventory_digest,
   origin_map: [old OriginRef -> successor exact OriginRef],
   preserved_constituent_refs, interpretation_extension_refs }
 | Reobservation { accounting_lineage, subject_refs, same_origin_refs }
 | QualifiedReplacement {
   accounting_lineage, predecessor_subjects, successor_subject,
   complete_origin_map, retained_interpretation_refs,
   eligibility_discharge_refs }
 | Unestablished { accounting_lineage, predecessor_refs, reasons }
```

All retained constituents/interpretations are bound to raw origins and policy identity. Carrying an arbitrary actor-written numeric floor is insufficient. Every predecessor origin must map unchanged, remain explicitly retained, or have a named admissible discharge; an unexplained missing origin fails continuity. If successor scope is stronger, it may add an interpretation but cannot delete the fallback interpretation merely by changing a subject hash. If no continuity witness is established, the consumer cannot publish a lower replacement financial view for that accounting lineage; it retains the prior admitted constituent/envelope subject and reports the successor unestablished.

This is not a discovery-order latch in the financial decision. The pure function consumes the complete normalized constituent, scope, interpretation and continuity evidence set. Two discovery routes with the **same final input facts and fixed lineage contract** produce the same envelope. Lineage inputs may include independently retained predecessor origin facts; they are provenance data, not hidden reducer memory. A new source contract cannot be substituted as if the same obligations had no predecessor. Discarding that provenance would change the subject/claim, not prove a fresh zero balance.

Where the lineage's initial reference-scope contract is already admitted, same-rid orphan claims begin in one stable TicketClaim, so initial500M is preserved. Where origin scope is unestablished, separate charge atoms establish1000M and stronger scope alone cannot release that hold. This difference is explicit input-contract/provenance, not first-seen processing order. Root must accept both the conditional initial-scope policy and the continuity contract; they are not inferred from raw file layout.

## 4. One affirmative EligibilityWitness and lawful late discovery

```
EligibilityWitness = {
  unit_or_covered_origin_set,
  unique_reservation_ref, unique_settlement_ref,
  reference_scope_witness_ref, primary_validity_witness_ref,
  complete_valuation_witness_ref,
  retained_charge_constituents, covered_reservation_constituents,
  covered_context_envelope_refs, discharged_interpretation_refs
}
```

Eligibility requires all existing applicable local validity checks, exactly one distinct reservation and one distinct settlement, established co-reference and scope, complete provider-reported usage with established priced identity/tier sets, and absence of unresolved monetary constituents outside its finite bound. Every known charge remains a max component even if above usage high. Coverage cannot combine affirmatively independent attempts, skip another settlement, ignore malformed evidence, hide an unpriced candidate or consume a different unit's fallback atom. A scope assertion alone is insufficient. Several distinct settlements remain Conflicting; conflicting reservations remain invalid; amounts do not establish cardinality.

A witness may normalize the retention interpretations only for its exact covered obligation: replace covered reservation holds by `Exact(max(known covered charges, complete usage/context bound))`, count proven same-obligation constituents once, and preserve all uncovered units/interpretations. If some interpretation or constituent remains unresolved, the relevant aggregate remains AtLeast. Complete established fully priced multiidentity bounds remain permitted. This is the only exposure-lowering permission proposed here.

Amend I1/F3 to:

> Evidence extension cannot lower conservative exposure in the full High order, except for the exact obligations covered by a newly established affirmative EligibilityWitness. Its establishment may be completed by any missing prerequisite; its row-kind or discovery order is immaterial. All known charges, independently distinct units and uncovered unresolved constituents remain represented.

Distinguish failures in the owning C-validity witness:

* **Missing prerequisite**: NoReservation, unavailable referenced scope, or not-yet-supplied evidence needed for a relative check. These block current validity/eligibility but may be discharged by affirmative evidence without deleting an immutable contradictory fact.
* **Immutable local contradiction**: malformed amount/model/status, bad basis, undercharge, actual source-order violation, conflicting reservation or settlement evidence. Merely adding a different valid-looking row cannot erase them. More settlements still conflict.

Preserve the existing external Rejected/NoReservation result for an orphan. Internally record reason applicability and failure class. Amend SETTLEMENT's blanket Rejected→Valid prohibition to permit a result whose only failures were discharged missing prerequisites to become Valid once all conditions hold. Valid/Rejected→Conflicting and Conflicting precedence remain. Earlier-source-position evidence is checked against original positions, not discovery order. No global history latch and no append-only restriction are introduced.

## 5. Compositional conservation argument

Fix one accounting lineage, immutable raw origin identities, and its admitted initial scope contract. Consider E⊆E′:

1. **Constituents/independence:** old origins survive with the same bytes and measures; newly proved independent units cannot be folded away. Repeated observation of the same admitted origin adds no amount.
2. **Association:** pairwise financial candidates can grow; RS display reassignment cannot remove them. Intrinsic and candidate pricing envelopes therefore retain every previously supported priced high.
3. **Per-unit hold:** without eligibility, max over retained R, applicable known charges and valuation/priced-high components cannot decrease. Unpriceable transitions retain priced components. K does not enter this function.
4. **Accounting interpretation:** each old retention interpretation has a conservation extension. Sums of corresponding nonnegative unit holds cannot decrease; their conservative join cannot decrease in MONEY's complete High order. Refinement alternatives are joined, not double-counted.
5. **Eligibility:** if no new valid coverage witness is established, steps1–4 apply. A newly established witness may reduce only its named coverage. If eligibility is later invalidated by conflict/unpriceability, reservation and candidate components return, and Exact→AtLeast with retained numeric floor is nondecreasing. Uncovered units retain steps1–4.
6. **Continuity:** a new subject is substitutable only when its origin/interpretation mapping establishes these premises or identifies the exact eligibility discharge. A renamed subject cannot bypass them.

The argument exposes proof obligations; it is not a machine-checked universal proof. In particular, completeness of financial-candidate predicates, constituent coverage and interpretation extension is a semantic review obligation. S1's bounded assertions test those boundaries; their passes alone cannot establish arbitrary raw conformance.

## 6. Decisive witnesses, both discovery routes

### RS auxiliary witness

R1=50M, rid1/n1; rejected reservation-basis request charge50M, null status, incomplete Anthropic usage10000 input and zero other counts, standard tier. Intrinsic sonnet input rate2 gives20M. Identity-refusal served fable with input rate8 gives80M. Rates are exact pinned producer/oracle values. FinancialCandidate(refusal,u1) holds by same admitted scope and n; V high80M, X AtLeast80M.

Add R2=1M, rid2/n1. Display RS3 may detach; FinancialCandidate(refusal,u1) remains, and (refusal,u2) is also a candidate. u2 has no usage, so an identity fact alone adds no usage amount. u1 stays AtLeast80M; u2 holds AtLeast1M. Total **AtLeast81M**, not51M. All-at-once and either order of introducing R2/refusal normalize to this same final result. Include read-error attachment, ambiguous/no-n and directly referenced measurements, not just the identity-refusal subtype. Do not independently count the same monetary observation in two units inside an interpretation.

### Earlier reservation/eligibility witness

Let S be a complete priced usage settlement, charge15M, reporting reserved500M, n/rid1, source position2; R at original position1 has500M and matching n/rid. Scope and all other conditions are established.

Route A: first S only → Rejected/NoReservation, no eligibility, AtLeast15M. Add R → unique primary witness and complete coverage become established → Valid/Eligible/Exact15M. The decrease in High order is explicitly authorized by **new eligibility**, not because the added row was a settlement.

Route B: first R only → Absent/Outstanding/AtLeast500M. Add S → the same final Valid/Eligible/Exact15M. All-at-once has identical primary witness, coverage and wire output. Without terminal/completeness, with wrong original order, with malformed model7, or with a second distinct reservation/settlement, no witness exists and no reduction is permitted. A late scope upgrade without complete eligibility also cannot release fallback constituents. Test the tag order as well as numeric amounts.

### Scope/orphan witnesses

Under FixtureDeclared/admitted initial scope, two same-rid orphan500M claims remain one unresolved TicketClaim500M before/after R500M; explicitly retain the changed C2/F2 policy. Under Unestablished scope, the two charge atoms retain a1000M subtotal, and an unmatched new R500M adds its hold. A newly qualified subject must carry this retention interpretation; it cannot reset to500M. Distinct admitted rid/incarnation units with500M each remain additive1000M even if raw row bytes match. A proven origin-copy is idempotent; semantic JSON equality alone does not deduplicate. Literal source-hash extension retains old origin refs.

## 7. Preserve R3 causal and wire improvements

C-primary → one C-validity result → C-auxiliary/display enrichment → S fixed forwarding of validity. S has no independently writable SettlementDecision constructors. V consumes intrinsic and financial-candidate facts; F owns EligibilityWitness/replacement/amount_source; X owns conservative envelopes; K owns downstream findings. The C-validity table still needs independent semantic witnesses.

Keep the actual wire target: Exposure `pool`; low `{nano,basis}`; High `{kind,nano,basis}`; Valuation string amounts and `derivations`; obligation `state/amount_source/by`; exact parameterized reasons. Fixed typed serializers may be shared mechanics, never financial rule logic. No generic projection expression language. StageId and BoundaryId need separate schema domains; a standard Draft2020-12 validator must validate actual canonical declarations and generated wire shapes.

The new evidence is externally consumable, not context-only: the spec owns schemas and fixed literal examples for `corpus/SCHEMA.md`'s per-attempt `accounting_evidence` reference; that reference resolves into separately owned `oracle/accounting-witnesses.json` with `scope_witnesses`, `origin_inventory`, `unit_identities`, `duplicate_classes`, `independence_witnesses`, `financial_candidates`, `retention_interpretations`, `eligibility_witnesses` and `continuity_witnesses`. Every record has a stable ID, raw origin refs and policy/source digest. Top-level expected/evaluation rollup accounting references identify the same lineage and constituent/interpretation inventory; an absent required reference is unestablished, not implicitly empty. The file is fixture evidence, never live authority. Production may use a different storage owner, but must expose the same typed consumption contract before it is qualified.

Financial context preserves lawful refusal-only orphan PiApi/NotApplicable/NotReported/AtLeast0 without overriding S/V; genuine subscription has no USD; subscription+dollar stays PiApi; genuine complete unbudgeted usage retains its approved route. R remains financial provisionality owner; temporal closure cannot clear AtLeast or unresolved commitments. Trusted-fixture verifier evidence stays exact-payload-bound test_mock only.

## 8. Independent qualification includes the actual Python consumer

Spec owner owns result declarations, mathematical envelope/eligibility contracts, evidence/wire schemas, serializers and actual dependency edges. Oracle owner independently owns raw-origin/source assumptions, witness facts, arithmetic assertions and expected corpus; it imports no generator/rule-selection/pricing evaluator. Shared DTO mechanics are allowed. Raw173original/175current input bytes remain immutable. New literal adversarial cases are additions with declared synthetic scope, not relabeling old raw cases as reachable.

The existing Python validator is an actual consumer. Its table_x must implement the accepted declaration consequences or be replaced with a smaller correctly qualified checker under root's acceptance; keeping an incompatible check for compatibility is unjustified. Current :473 must stop forcing supported low0 for rejected/conflicting decisions. :438–449 must consume the accepted scoped exact-byte/origin duplicate contract, not normalized JSON. Register this validator in the semantic dependency graph; Rust assertions alone do not clear it.

Require independent tests that call the **actual** Python checking entry points against a new isolated accepted candidate and challenge self-consistent wrong expected objects with matching changed rollups. Challenge accepted positive objects as well: low15M/15M/20M/60M; byte-distinct equivalent-JSON reservations conflicting; identical bytes in distinct proven incarnations independent. Merely comparing to an existing fixture answer or mutating only one field is insufficient. Preserve first failures/custody; no mutation of the rejected validator is authorized by this plan.

All original A findings remain acceptance obligations:

| Finding | Boundary witness |
|---|---|
| A01 | Stable-scope500→500; unestablished1000 subtotal retained; distinct units1000; exact RS80→81; scope/subject continuity; every noneligible change retains the full High order |
| A02 | Independent raw-priced exposure lows15/15/20/60M, correct high max/basis, actual Python and Rust assertion positives/negatives, plus V8 mixed/unpriceable controls |
| A03 | Refusal-only orphan, missing-launch/dollar, subscription+dollar and noR/unknown-scope witnesses all have lawful typed applicability, without false S/V overrides |
| A04 | Validate actual175 corpus wire shapes, all constructors/parameterized reasons, basis mappings, amount/derivation encodings and accounting evidence references |
| A05 | Reject local S decision constructor injection; independently reject a wrong owning C-validity result; display attachment cannot change primary witness, and cannot erase financial candidates |
| A06 | USAGE-PRESENCE→V/F/X/contract/R and actual validator invalidation; ProviderFamily→Model/schema; cardinality→validity/eligibility/envelope; reason/continuity changes reach every true consumer |
| A07 | Malformed reported model fields are explicit unpriceable/ineligible; absence/null retain policy; complete established priced multiidentity positives remain finite; priced high survives uncertain attachment |
| A08 | Equal amounts on distinct raw rows do not imply UniqueReservation; normalized JSON equality does not dedup; scoped exact repeats positive; independent identical-byte incarnations remain distinct |
| A09 | Standard schema positive/negative validation agrees with Rust loading for Stage/Boundary and new scope/continuity evidence variants; embedded schema-byte comparison is insufficient |

Dependency edges derive from actual typed consumed records, formulas, predicates and serializers; residual dependencies are named and reviewed. Financial candidates and scope/cardinality flow to V/F/X, eligibility and interpretation coverage to X/R, continuity to rollup/admission consumers and contract views. Null acknowledgements remain unestablished. Finite partitions check declaration coverage only; exclusions require an owning constructor impossibility, never a convenient raw-case omission.

## 9. Alternatives and explicit semantic ledger

**Stable keys alone** are rejected by80→51. **Monotone pairwise financial relevance plus conserved constituent interpretations**, preferred here, fixes ordinary RS changes without a full association solver and covers scope/merge continuity declaratively. **Exact global possible-world association envelopes** can avoid loose conservative per-unit holds, but require exclusive-assignment semantics and potentially expensive evaluation; they are a credible future realization, not S1 machinery. **History-free explicit merge accounting** is a valid alternative: retain all origin constituents and prove every merge preserves the envelope except exact eligibility coverage. R3's implication that it inherently requires ingestion history is withdrawn. These approaches share the same conservation/coverage contract; an implementation may choose after the owner accepts that contract.

No indiscriminate sum of R+S within a proven unit, no subtraction from diagnostics, no NoReservation latch, no append-only weakening of I1, and no solver/reference reducer disguised as tests is proposed.

Root must explicitly disposition these changes:

1. C2/F2 same-reference orphan grouping and conditional initial scope admission; current500M initial group versus old1000M sum is substantive.
2. FinancialCandidate relevance separate from current RS attachment; possible identities remain financial context after ambiguity. This can conservatively increase held amounts and changes V's read interface, with provenance retained.
3. F3/I1 exception becomes newly established exact EligibilityWitness, whichever prerequisite arrives last. SETTLEMENT permits discharged-missing-prerequisite Rejected→Valid; immutable contradictions/conflicts stay blocking. Ordinary eligible positives remain possible.
4. Source/constituent/continuity evidence becomes a real core/consumer contract. Replacing a subject is not a balance reset; scope alone cannot discharge fallback interpretations. Fixture witnesses are conditional, not authenticated producer provenance.
5. Actual validator semantics and new evidence serialization are qualified, with no identified external consumer requiring preservation of the rejected generic shape or wrong zero-low check.

D3 remains intact: S1 writes fixed contracts/declarations/schema/rendering and independently authored bounded raw arithmetic assertions only. No raw classifier/reducer, interpretation enumerator, expression evaluator, solver, production adapter or workspace integration. Full executable raw-to-output conformance and live E realization remain unestablished. This architecture is coherent as a bounded contract candidate; its universal financial truth, efficient future implementation and accepted scope are not established by this planning result.

After fresh design judgment/root adoption only: new isolated subject; disjoint spec and oracle ownership; actual Python+Rust qualification; all required legacy check/selftest/revisions/sabotage; exact joined freeze; current A/C and relevant B reviews. History and all holds remain until those owners establish their respective claims. No implementation under this candidate is authorized now.
