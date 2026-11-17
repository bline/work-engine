# Slice 2a oracle result: oracle-side historical findings

Branch `oracle-r2`, base `2729107`. All commits are authored by `pi-accounting-oracle-author`.

| Commit | Finding |
|---|---|
| `8ce4d3a` | R8-FIN-01 |
| `90ac28d` | R8-IDENTITY-01 |
| `b5d94a0` | R8-RESERVATION-01 |
| `837a1f4` | R8-GATE-IDENTITY-01 |
| `895e853` | R8-LOW-INVENTORY-01 |
| `83b87cb` | R8-PLACEMENT-01 |
| `7f9ea87` | R8-NAMESPACE-01 |
| `9e00f55` | R8-DOC-01, slice-1b P3-4, revision ledger |

The working tree is clean, and no raw inputs or evaluation premises changed: a script checked every case's `inputs` and `evaluations` minus `expected` against base. All findings stay OPEN until review and a root disposition. Full S1 stays OPEN.

## How the evidence was produced

**Actual consumers:**
- Python `validator/validate_corpus.py`. New predicates sit inside `check_case`/`check_attempt`/`check_evaluation` and `check_accounting_references`.
- Independent Rust `tooling/spec-check/tests/independent_raw_axes.rs`. New `r11_*` predicates and tests; nothing is imported from the spec crate.

**Controls:**
- Python: `validator/test_r11_findings.py`, 13 tests.
- Rust: `r11_` tests in `independent_raw_axes.rs`, 6 tests.
- Every negative asserts its named predicate tag, never just "some failure". Supplied-bundle mutants run on an isolated copy of the subject through the real `check_accounting_references`.

**Final runs:**
- `final-python-r11.txt`: 13/13 ok.
- `gates-final/`: `replay_gates.sh` on `9e00f55`. All 15 gates rc=0, including `no_panic_mutations` (`gates-final.log`).
- Rust `cargo test` (inside `cargo-test.out`): the r11 tests 6/6 ok.

**Base runs** (`2729107`, same tests):
- Python, `base-python-r11.txt`: 9 of 13 fail where the defect exists. The 4 that pass are the corpus positives, P9 (already rejected by the R10 pilot claim) and RESERVATION (already fixed).
- Rust, `base-rust-r11.txt`: 5 of 6 fail. That run used base helper semantics: malformed identities dropped silently and ids compared without a presence check.

**Full path.** `check corpus oracle/rates` gives 0 failures from `check_case` with all r11 predicates across 175 cases. It still exits 1, with 476 accounting-reference failures. All of those are bundle-shape gaps against the slice-1b schema, identical in count and kind on base (`base-full-check.txt`, `final-full-check.txt`). They are full-S1 work, out of scope.

## Per finding

### R8-FIN-01 (P1): retained raw charge
- **Current state on base.** The R10 pilot already rejects the coherent P9 700→500 mutant through `r9_pilot.case_claim` ("R9 P9 raw retained charge700M violated"). There was no general raw-charge predicate: `table_x` used the *supplied* `settlement.charged`. Base accepts a coherent non-pilot rewrite of P130-R02 (charge 1000→525, high, rollup and commitment all agreeing) with zero failures.
- **Change.** Python `raw_charge()`: the largest well-formed raw `usd.charged_usd` among the attempt's distinct settlement rows. It must equal `settlement.charged`, and money terms use the raw value. Rust `r11_raw_charge_violation`, with an exact `r11_dollars()` that also parses exponent lexemes (P130-R02's `1e-06` = 1000 nano). Charge equality and high ≥ charge are enforced over all 175 cases.
- **Controls.**
  - Positive: all 175 cases pass, Python and Rust.
  - Negatives: P9 coherent 700→500 ("retained raw charge 700000000 (R8-FIN-01)"), and the non-pilot P130-R02 1000→525 (Python and Rust).
  - The P130-R02 negative passes on base, i.e. the defect is shown.
- **Limits.** Retained-component records in bundles are checked only through the R10 P9 claim.

### R8-IDENTITY-01: malformed identity
- **Current state on base.** Python's raw range check dropped non-string identities. Rust `identities()` dropped them too, and `accepts_usage_axis` *required* a valued range, so a numeric response_model with valued/complete outputs was accepted and the conservative model7 repair was not accepted by that axis.
- **Change.** A present `model`/`response_model`/`authorized_model` that is not a canonical nonempty string forbids valued, complete, eligible, or supported-minimum outputs. Rust also requires `priced_high` to equal the high over the string identities.
- **Controls.**
  - Positives: model7 VC01/A04 private controls; V01 priced multi-identity 20–80M stays finite.
  - Negatives, both consumers: coherent valued-complete 15M outputs on model7; an invented supported_low; injected `""`, `true`, `{}`, `" gpt-6-astra"` and `7` into P132-FG with its favourable outputs unchanged.
  - All fail on base.
- **Limits.** No historical case carries a malformed identity (scan: 0). `served` on refusal rows is not covered.

### R8-RESERVATION-01: cardinality
- **Current state.** Already fixed. Python counts byte-distinct raw lines (`raw_settlement_checks`). Rust axis 368 uses `distinct_rows==1` (no `nums.len()`). The R10 equal-reservation, same-origin and admitted-copy controls run in gates `python-main-pilot` and `rust-r10-controls`.
- **Added.** A full-path Python control: the equal-reservations control with a coherent Valid settlement is rejected with "valid settlement without UniqueReservation (P3)". It also passes on base, as expected since the fix predates base. No re-fix was made.

### R8-GATE-IDENTITY-01
- **Base.** Triple-absent with a recomputed payload digest qualified in Python `gate_qualification` (None==None) and in Rust `mock_binding`.
- **Change.** All three ids (objective `admission_authority.id`, gate `authority.id`, witness `authority_id`) must be present, nonempty and whitespace-canonical before equality. Python reports `GateAuthorityMismatch`. Rust adds `canonical_id` and `gate_identity_triple`.
- **Controls.**
  - Positive: P132-FG-VC15 qualifies (test_mock).
  - Negatives: absent, `""` and `" "` triples with recomputed payload; the actual check path also rejects the unchanged supplied `qualified:true/retired` commitment. Witness mismatch also tested.
  - All fail on base.

### R8-LOW-INVENTORY-01
- **Base.** Five scopes had empty U and both consumers accepted them: R08 o/p, R12 o, and the two A04 cases' o, all `owned_closed`.
- **Change.** Python `low_inventory()` and Rust `r11_low_violation`. U ⊇ the usage origins of every PiApi unit the objective owns or is a candidate for on that line, and Seed ⊆ owned usage. The predicate is derived from the raw-checked attribution, independently of LOW-ASSERTIONS.
- **Corrections.** The five scopes in `r8-completion-A/LOW-ASSERTIONS.json` and their bundles now carry the candidate origin. Seed stays [] and low stays 0.
- **Controls.**
  - Positive: all cases.
  - Negatives: jointly omitted U in both the bundle and the assertion, and a candidate promoted to Seed, for each scope at both consumers. All fail on base.
- **Not changed.** `r8-completion-A/DERIVATIONS.json` scope_claims still omit these origins. The accepted R10 admission (`validator/r10_admitted.json`) embeds that whole file as the P132-FG-VC11 payload, so editing it breaks the pilot gate (verified). Root re-admission is needed if it must change. This is recorded in `REVISIONS.json` `not_changed`.

### R8-PLACEMENT-01
- **Base.** Python derived reasons from the supplied placement only.
- **Change.** Python `raw_placement()` (the existing `obligation_interval` + `closure_state`) must equal the supplied placement of every owned attempt. Rust `r11_placement_violation` covers fully dated owned attempts on trusted, uncontradicted lifecycles, with no deliver or undated rows; other attempts are outside that finite predicate.
- **Correction.** P132-CP, both evaluations: placement `inside`→`straddles_closure`, and p's reasons gain `PlacementStraddles`. This follows the batch-1 dossier P132CP/T3 and G3. It was the only disagreement over 175 cases.
- **Controls.**
  - Positive: corrected P132-CP; P130-T keeps `before` (its contradiction stays).
  - Negatives: P132-CP `inside` with matching wrong reasons, and P130-T `before`→`inside`. Both fail on base.

### R8-NAMESPACE-01 (oracle side)
- **Base.** No namespace predicate existed. Six bundles declared a `dollar_ticket` contract and grant on admission-namespace origins: VC08, U06, V03-control and V03-api (2 origins each), and N03-contradicted-admission and N03-control-unlabelled-admission (1 origin each). N03 was found by the predicate, not named in the packet.
- **Change.** Python `namespace_application()` is called from `check_accounting_references`, and Rust `r11_namespace_violation`. A contract namespace must equal the raw reference namespace when the raw namespace is dollar_ticket or admission.
- **Corrections (root option B).**
  - VC08, U06 and V03 ×2 are split into OriginBase units `adm:S1/turn-1:rid=1` and `row:S1/turn-1:line=2`, with expected, attribution and bundle (bases, units, origins, valuation_records) rewritten.
  - In all 6 cases the input proof now carries one `unestablished` initial contract with no grant (superseded at the join; the first version used an `admission` contract without a grant, see JOIN-REMEDIATION-RESULT.md), and the provenance records the withdrawal. Money and standing are unchanged.
- **Controls.** The dollar_ticket contract re-declared (bundle, grant and refs agreeing) is rejected in all 6 cases at both consumers. The OriginBase structure is asserted.
- **Private positive control.** `oracle/r11-controls/admission-namespace/` has VC08's raw bytes, the grouped unit, and a conditional admission contract and grant. It is outside the 175. `admission_namespace_control()` grants the grouping only with an exact root admission bound from outside the subject (`R11_ROOT_ADMISSIONS`). It currently reports REQUIRED/UNMET; it passes with the proposed record and fails with a stale digest.
- **Spec.** No spec change was needed.

### R8-DOC-01 (P3)
- `oracle/README.md`: removed the "finite malformed identity controls" supported-axis overclaim and the "joined-2 passed both actual consumers" claim. Added a Qualification standing section with exact counts.
- `validator/README.md`: replaced the stale 24/350 statement with the measured 374/0 literal scopes and the 476 full-path reference failures.

### Slice-1b review P3-4
The four VC07 negatives were rebuilt from the current positives with only their named mutation. `evidence_controls` PASS:

| Negative | Now rejected by |
|---|---|
| `missing-completion` | "dangling unbudgeted completion" |
| `relabel-dollar-hidden` | "unbudgeted completion overlaps dollar primary evidence" |
| `unlinked-release` | "released anchor lacks cited exact link to selected covered request" |
| `relabel-dollar` | Draft2020-12, sole violation `unbudgeted_applicabilities[0].dollar_origin_refs` maxItems 0 |

The `relabel-dollar` constraint is the spec schema's own encoding of the relabel prohibition, so that is the intended predicate. Moving it into a Rust semantic predicate would need a spec change, which I left with the spec owner.

## Newly derived cases (`dossier/`, schema `pi-accounting-oracle-dossier/1`)

The 12 dossiers validate against `dossier/dossier-1.schema.json`:
- VC08, V03-control, U06, V03-api: NS, PM, VA, T, G and CO derived. The batch-1 VC08 NS3/PM2 claims are now DerivedNominal on the Q-NAMESPACE basis.
- N03 ×2: namespace claims only.
- P130-R08, P130-R12, P133-A04 ×2: candidate ownership and U scope lows.
- P130-T: placement, RequiredUnmet pending the scope admission.
- P130-R02: retained charge.

Families outside the controls are marked RequiredUnmet (oracle future batch). Arguments are in `DERIVATIONS-S2A.md`. `AGREEMENT-S2A.json` records 38 base disagreements and 0 at final: 72 agree, 1 agrees if admitted, 40 not compared. The batch-1 P132-CP dossier already gave the placement truth.

## Proposed admissions (`PROPOSED-ADMISSIONS.json`; the oracle did not add them)
1. **R11-CTL-admission-namespace-grouped** (private control): admission namespace, same_origin_only, origins S1/turn-1:1 (`052bdde4…`) and :2 (`1f03afd8…`). Declaration file sha256 `d37993b6…`, case file sha256 `956529eb…`. Raw consistency: both origins carry admission references, the extent is the complete source, and there are no copies.
2. **P130-T-owned-before-window-summed** (needed for its placement claim), **P130-R08** and **P130-R02** (optional; their claims are invariant): dollar_ticket, same_origin_only, covering S1/turn-1:1 and :2. Raw-consistency claims and digests are in the file.

## Root bookkeeping needed
- `ROOT-ADMISSIONS.json` entry ADM-ba3397134fb5085f (P132-CP) binds case_file_sha256 `7c057e10…`. The corrected supplied outputs changed the file to `e6c129e7…`. Raw origins are unchanged; the covered digests still match.
- ROOT-ADMISSIONS is not referenced by any Python/Rust consumer yet, other than the new `admission_namespace_control` (caller-bound path). Wiring D-SCOPE admissions into `check_corpus` is full-S1 work.

## Blocked on spec
Nothing is blocked. One item is noted only: the VC07 relabel negative is enforced by a schema constraint, not by a semantic predicate.

## Cost
Roughly within the target. Spending went mainly to targeted source reads, two cargo builds and the gate replays. No large files were re-read.
