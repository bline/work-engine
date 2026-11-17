# Slice 1a packet: independent oracle, measured whole-case derivation batch

Owner/root: Pi session `01a1e569-2805-72ff-b837-03d1577c405d` (operator-directed owner of the
Pi attempt accounting S1 campaign). You are the independent oracle author for this batch. Root
alone admits, joins, reviews and accepts. This packet grants no source mutation.

## Objective of this batch

Fully derive, independently of supplied outputs, the complete finite expected claims for these
eight whole historical cases and every one of their required scopes:

1. `P9-claimed-charge-above-reservation`
2. `A06-reservation-usage-null-status-no-id`
3. `V01-usage-settlement-other-response-model`
4. `VC08-subscription-measured-request`
5. `P130-R07-closed-unresolved-predecessor-constrains-successor`
6. `P132-FG-VC15-gated`
7. `P132-CP-closed-predecessor-outstanding`
8. `N07-control-retired-legacy`

"Fully derived" means every claim family of the case (`raw_E_identity_cardinality`,
`namespace_grants`, `primary_membership_validity_exact_F`, `valuation_retained_amounts`,
`T_ownership_candidates_lifecycle_placement`, `gate_commitment_provisionality`,
`finite_continuity_order`) and every scope low of the case reaches either `DerivedNominal`
(raw observations + an owning policy argument + the exact typed expected consequence) or
`RequiredUnmet(reason, decision_owner)`. `Unexamined` is not an acceptable end state for these
eight cases. Whether a claim family applies at all is itself part of the derivation: an
inapplicable family is a derived, reasoned outcome, not a skip.

This batch also measures cost and exposes whether the full 175-case derivation is realizable
within the D3 boundary. Report honestly if it is not.

## Inputs (read-only)

- Subject (immutable, accepted manifest `5e594cac…`, 967 files):
  `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/successor/subject`
  Raw case inputs are `corpus/cases/<case>.json` (`inputs`, `evaluations` minus `expected`,
  `trusted_fixture_inputs`, `profile`, selectors). Policy: `contract/**`, `spec/ck.json`
  (C–K canonical), `oracle/rates/**`, `oracle/POLICY-APPROVAL.md`.
- Governing plan (binding):
  `/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/full-realization-r1/planning/PLAN.md`
  — read "Finite truth and applicability interfaces" and "Finding and control matrix".
  Financial authority it names (base plan `43d0e7c8`, amendments `c8376f7a`, `90b6308e`,
  `3f79f910`) is located via
  `/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/full-realization-r1/derivation-stage/POLICY-BINDINGS.json`.
- Prior derivation stage (reuse with attribution, verify before relying):
  `/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/full-realization-r1/derivation-stage/`
  `INTERFACES.md` (root has bound its semantics: claim variants, `ClaimStatus`, `SourceRef`,
  `CausalAdmission`, `PayloadPremise`), `DERIVATIONS.md` (partial N1–N6 arguments),
  `LEDGER.json` (per-case family questions), `RAW-ORIGINS.json`, `FRONTIERS.json`,
  `UNIVERSE.json`, `OBLIGATIONS.json`.
- Earlier independent oracle material in the subject (`oracle/R8-RAW-DERIVATIONS.md`,
  `oracle/R8-LOW-ASSERTIONS.json`, `oracle/qualification.json`, `tests/independent_raw_axes.rs`)
  may be reused with attribution only after you check its premises.

## Invariants (must hold regardless of route)

- Truth comes from raw observations plus an owning policy argument. Supplied `expected`,
  result bundles, low literals, generator decisions, copied snapshots and the derivation-stage
  inventory cannot establish a nominal value. You may read supplied outputs only *after* your
  derivation for that claim is written, to record agreement or disagreement; never revise a
  derivation to match an output without a raw/policy reason recorded.
- No raw financial reducer, raw expression interpreter, correlation/coverage/partition solver or
  production adapter (D3). Small throwaway arithmetic checks are fine; they must not become the
  source of a claim. If a claim cannot be stated without such machinery, mark it
  `RequiredUnmet` with that reason and say so in the result.
- Missing grant/authority/rate/lifecycle facts stay `RequiredUnmet` with root as decision owner.
  Never invent a guarantee; a located dollar scope never grants the admission namespace.
- Preserve absence vs null vs value, JSON types and exact decimal lexemes. Money in nano-USD
  with exact decimal conversion.
- The eleven historical findings (`OBLIGATIONS.json`) stay OPEN; note where a derived claim
  bears on one (e.g. P9 700M retained charge → R8-FIN-01; VC08 → R8-NAMESPACE-01).
- Do not modify anything under the subject or the predecessor directories.

## Outputs (write only under `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/oracle-batch1/`)

- `dossier/<case_id>.json` — one record per case containing every claim family and every scope
  low with `ClaimStatus`, `SourceRef`s for each raw premise, the policy clause relied on
  (file + section), the typed expected consequence, and decisive positive / intended-reason
  negative control ideas for the later consumers.
- `DOSSIER-SCHEMA.md` (and optionally a JSON Schema) — your concrete proposal for the dossier
  record shape realizing `INTERFACES.md`. Root binds it after this batch; keep it closed and
  typed (no executable paths/expressions).
- `DERIVATIONS-BATCH1.md` — the authored arguments, per case, readable by a reviewer.
- `AGREEMENT.json` — after derivation only: per claim, supplied-output agree/disagree/absent,
  with the disagreement detail. Disagreements are findings, not things to fix here.
- `RESULT.md` — counts by status, every `RequiredUnmet` with owner, every D3 conflict, which
  historical findings each case touched, reuse attributions, and your estimate of effort for the
  remaining 167 cases (which parts were per-case vs reusable across cases).

## Working constraints

- Work independently: do not read other workers' outputs or any review reports newer than the
  predecessor's acceptance unless listed above.
- Stop and report rather than guess if the policy authority is contradictory or a case requires
  a new financial policy decision.
- No network, no paid APIs, no git commits, no child agents.
