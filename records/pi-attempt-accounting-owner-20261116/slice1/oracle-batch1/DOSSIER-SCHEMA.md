# Dossier record schema proposal `pi-accounting-oracle-dossier/0.1-proposal`

Standing: a proposal for root to bind. It realizes derivation-stage `INTERFACES.md`: the claim variants, `ClaimStatus`, `SourceRef`, `CausalAdmission` and `PayloadPremise`. Every instance in `dossier/` follows this shape. The record is closed and typed. It has no executable paths and no expressions. `pointer` strings only say where a value is in the file. The byte span plus the token/raw digest is what binds it. Opaque IDs (case, evaluation, objective, origin, unit, scope, link) are equality selectors only.

## Top level (one file per case: `dossier/<case_id>.json`)

| Field | Type | Meaning |
|---|---|---|
| `schema` | const `"pi-accounting-oracle-dossier/0.1-proposal"` | version |
| `case_id` | string | opaque selector |
| `author` | `{role, owner_session}` | authorship (oracle, separate from spec/generator) |
| `standing` | string | always "derivation proposal, not consumer qualification" |
| `subject` | `{manifest_sha256_prefix, case_file:{path, sha256}, inputs_canonical_sha256}` | binds the raw case file. `inputs_canonical_sha256` = SHA-256 of sorted compact `case.inputs` with `ensure_ascii=true` (EXACT-WIRE) |
| `policy_refs` | `[PolicyRef]` | `{policy_id, path, sha256}` for every policy/rate document cited |
| `causal_admission` | `CausalAdmission` | below |
| `origins` | `[OriginRecord]` | complete ordered raw origin inventory with field states |
| `premises` | `[Premise]` | named premises shared by claims (payload premises, pinned rates, trusted fixtures) |
| `claims` | `[Claim]` | every family claim |
| `family_status` | `{<family>: FamilyStatus}` | exactly the seven families |
| `scope_lows` | `[ScopeLow]` | exactly the required scopes of the case (2 × objectives × evaluations) |
| `excluded_channels` | `[{channel, reason}]` | channels deliberately outside the seven families (e.g. K finding codes) |
| `reuse` | `[{source, what, verified_how}]` | attributed reuse of earlier material |

## SourceRef (closed tagged union on `ref`)

* `case_token`: `{file, file_sha256, pointer, byte_start, byte_end_exclusive, token_sha256, presence: value|null, json_type, lexeme?}`. For an absent member it is `{file, file_sha256, pointer, presence: "absent"}`, with no span. `lexeme` keeps exact JSON text for numbers (`"1.0"`, `"0.015"`), the decoded value for strings, and `"null"`, `"true"` or `"false"` for literals.
* `raw_line`: `{file, file_sha256, origin_id, pointer, byte_start, byte_end_exclusive, token_sha256, raw_utf8_sha256, raw_utf8_length}`. `token_sha256` covers the JSON string token in the case file. `raw_utf8_sha256` covers the decoded line bytes, which is the E origin digest.
* `raw_field`: `{origin_id, raw_utf8_sha256, field, presence: value|null|absent, json_type?, lexeme?}`. A field inside a raw log line. `field` is the in-line member location (`/usd/charged_usd`). The binding is to the line digest.
* A premise or claim may also be cited by its ID string (e.g. `"P9/PREM-SCOPE"`, `"P9/VA1"`).

Arrays keep their order. Duplicate members are rejected at extraction: the scanner fails closed. Absent, null and value stay distinct everywhere.

## CausalAdmission

`{top_inventory:[{key, classification: governing_premise|supplied_outcome|noncausal_metadata, reason, source_ref}], trusted_fixture_inputs: SourceRef (absent allowed), profile, contract, schema, role: SourceRef, inputs_canonical_sha256, log_final_newline:{<alias/turn>: bool}, required_scopes:[ScopeId], additional_channels:[...]}`.

Every top-level member is classified with a reason. Output members (`expected`, `revisions*`, `rationale`, `historical`, `findings`) are supplied outcomes and are never premises. `evaluations` is a governing premise with each `expected` member removed.

## Premise

`{premise_id, kind: raw_observation|policy_clause|payload_premise|trusted_fixture, statement, status: ClaimStatus, source_refs:[SourceRef], policy_clauses:[PolicyClause]}`.

A `payload_premise` (INTERFACES `PayloadPremise`) is `RequiredUnmet(root)` until root admits the exact declaration. Locating a payload, raw consistency, and matching supplied bundles never admit it.

## Claim

| Field | Type |
|---|---|
| `claim_id` | `<CASE>/<n>`, unique per dossier |
| `family` | one of `raw_E_identity_cardinality`, `namespace_grants`, `primary_membership_validity_exact_F`, `valuation_retained_amounts`, `T_ownership_candidates_lifecycle_placement`, `gate_commitment_provisionality`, `finite_continuity_order` |
| `subject` | object of opaque selectors (origins, obligation, unit, evaluation, objective, admission, gate) |
| `status` | `ClaimStatus` |
| `premise_refs` | `[SourceRef or premise/claim ID]` |
| `policy_clauses` | `[{policy_id, section}]`; `policy_id` must occur in `policy_refs` |
| `argument_ref` | anchor in `DERIVATIONS-BATCH1.md` |
| `expected` | `TypedConsequence`. Required iff status is DerivedNominal, otherwise null |
| `consequence_if_admitted` | `TypedConsequence` or null. Set only for RequiredUnmet: the fully authored consequence that would hold if root admits the unmet premise. It is not a nominal claim |
| `controls` | `{positive:[string], negative:[{mutation, intended_predicate}]}`: decisive control ideas for later consumers |
| `findings` | `[{finding_id (one of the eleven), relation: direct|bears_on}]` |

`ClaimStatus = {kind:"DerivedNominal"} | {kind:"RequiredUnmet", reason, decision_owner, unmet_premise_refs}`. `Unexamined` is not allowed in a closed dossier.

`TypedConsequence` is tagged by `variant`. The variants used in this batch, each a fixed record:

* `RawOriginInventory {origin_count, origins:[{origin_id, original_position, evidence_kind, raw_utf8_sha256}], unreadable, non_attempt, final_newline, ...}`
* `EvidenceClassCardinality {reservation_classes:[[origin]], settlement_classes, *_count, exact_copy_classes, byte_identical_pairs}`
* `IdentityStates {pinned, canonical_identity_set, malformed_identities, ...}`
* `RawReferenceNamespace {per_origin:{origin:{namespace: dollar_ticket|admission, rid, usable}}}`
* `DeclaredPayloadRawConsistency {declared, raw_consistent, mismatches, establishes_admission:false}`
* `NamespaceApplication {initial_base, atomize, break_copies, hold_units}`
* `OriginLocalChecks {reservation:{...}, settlement:{...}, local_failures}`
* `PrimaryCoverage {primary_membership, settlement_validity, temporal_obligation, obligation, replacement, execution, forwarding, accepted_coverage, definite_links}`
* `ObservationValuation {origin, state, identities, tier_set, splits, complete, low, high, derivation}`
* `RetainedComponents {components:[{kind, origin_ref, nano, role}], invariant}`
* `UnitExposure {unit_origins, pool, components, high:{kind, nano, basis}, low:{nano, basis}, billing, blocking_must_include}`
* `SessionLifecycle`, `Ownership`, `Placement`
* `GateQualification`, `CommitmentState`, `CommitmentAmount`, `ObjectiveStanding`, `Provisionality`, `PredecessorObligations`, `AdmissionDecision`, `ObjectiveStandingAndAdmission`
* `FiniteContinuity {continuity_applicable, reason, cross_evaluation_equalities, order_invariance}`

Money is always a nonnegative decimal string of nano-USD (`{"nano":"700000000"}` or a bare string inside component records). It is converted exactly from producer lexemes: at most 6 fractional digits, multiplied by 10^9 with no binary float. `*_must_include` fields are subset relations. They are used where the canonical vocabulary (the K/§9 catalogue) is not present in `spec/ck.json`, so an exact set would be asserted without an owner.

## FamilyStatus

`{status: DerivedNominal|RequiredUnmet, claim_ids, unmet_claim_ids, decision_owners}`. A family is DerivedNominal iff every one of its claims is. If a family does not apply, that is a DerivedNominal claim whose consequence carries `applicable:false` and a reason.

## ScopeLow

`{scope_id, evaluation, objective, label: owned_closed|owned_outstanding, status: ClaimStatus, premise_refs, policy_clauses, argument_ref, expected_low: Low|null, high:{status: ClaimStatus, expected: High|null}, consequence_if_admitted|null, note?}`

* `Low = {owned_unit_origin_sets:[[origin]]|null, U:[origin], seed:[origin], slots:[...], single_floor, independent_floor, credits:[...], selected_low, credit_mode: independent_slots|single_ambiguous|none, basis: usage_valuation|none}`. `owned_unit_origin_sets` is null when the low does not depend on the unit structure but the unit sets do. Those sets are then carried in `consequence_if_admitted`.
* `High = {kind: exact|at_least, nano, basis_histogram:{basis: count}}`.
* The low status and the high status are kept separate, so a low that holds in every branch is not blocked by an unmet high premise.

## Rules a consumer must enforce (proposal)

1. The required scope set comes from raw evaluations/objectives (2 per objective configuration), never from outputs. The dossier must list exactly that set.
2. A claim with RequiredUnmet status can never produce a Qualified outcome. `consequence_if_admitted` may be consumed only after root records admission of every `unmet_premise_refs` premise, in a separate root-owned admission record that the subject cannot supply.
3. Each `raw_line`/`raw_field` SourceRef must re-verify against the current case bytes (digest and lexeme). A mismatch means OutsideAdmission, not Failed-financial.
4. Negative controls must fail for their `intended_predicate`. A failure on an unrelated digest does not count.
