# Independent review: slice 2 (eleven historical findings), branch `join-r2` head `53e7db7`

Reviewer standing: fresh read-only adversarial reviewer, Fable 5.1 (`claude-fable-5-1`) via `claude-sdk/`,
standing in for Astra xhigh by operator direction. **Same-provider review**: root and both builders also ran on
this provider; authorship independence from the subject holds (I authored nothing in it), provider independence
does not. Root: Pi session `01a1e569`. Subject verified: clean checkout `/tmp/rootv-join` at `53e7db7` (worktree
`git status` clean); built binary `root-verify-target-final/debug/pi-accounting-spec-check`
sha256 `522298c2…d6bc`, test binary `independent_raw_axes-3ee1ad2ed02bb648` sha256 `bd75cc5b…3b14`. Base
`s1-successor` = `2729107`. No subject or evidence file was modified; all mutants live under `/tmp/rv-review/`.

## Verdict: `ACCEPT_WITH_CHANGES`

All nine slice-2 findings have their matrix-row decisive controls realized at the named actual consumers, the
corrected outputs are derived from unchanged raw inputs with attributed revisions, and the two slice-1b findings did
not regress. The change required before NAMESPACE-01 can be called closed on the stated authority terms is one
precise consumer gap (D-1 below): neither actual consumer binds root admissions, so a subject can declare its own
`admission` (or `dollar_ticket`) conditional scope and regroup VC08 exactly as root refused to admit, and both
actual consumers accept it. The gap is inherited and was disclosed by the oracle, but the join's Python message
and root's JOIN-REMEDIATION wording ("asserts an admitted scope") overstate what the consumers check. Either wire
the existing outside-bound `admission_namespace_control` into `check_accounting_references` for every
non-`unestablished` contract (oracle write scope, small), or root records this bound explicitly in the disposition.

`architecture_assessment`: **`bounded_repair_credible`**. Every repair is a local predicate at an existing actual
consumer plus truthful output corrections; nothing resembles a reducer/solver; the residual gaps are consumer
binding/representation gaps with local fixes, not design faults.

## Per-finding dispositions

| Finding | Disposition | Basis (executed unless stated) |
|---|---|---|
| R8-FIN-01 (P1) | `closed_bounded` | Python `table_x` now reads `raw_charge()` (largest well-formed `usd.charged_usd` among the attempt's distinct settlement rows) and uses it for money terms; Rust `r11_raw_charge_violation` over all 175 cases. P9 coherent 700→500 (settlement/high/rollup/commitment) rejected with `retained raw charge 700000000 (R8-FIN-01)` at Python and `raw charge 700000000` at Rust; non-pilot P130-R02 (`1e-06` lexeme = 1000 nano, supplied `1000`) 1000→525 rejected at both; V01 coherent 80M→40M (A2b's unexecuted hypothesis) rejected `supplied settlement.charged 40000000 != retained raw charge 80000000`. High-only reduction (charge kept) rejected by Table X. Omitting the settlement row from attempt evidence is caught by `INV-F6 evidence not accounted for`. Bound: retained-component records in bundles checked only via the R10 P9 claim; Rust checks `high ≥ charge` per attempt, not rollups; Python accepts ≤6-decimal charge lexemes, Rust ≤9 (all 19 current lexemes ≤6 decimals). |
| R8-IDENTITY-01 | `closed_bounded` | Python `raw_usage_policy_checks` and Rust `accepts_usage_axis`/`malformed_identity`: present `model`/`response_model`/`authorized_model` that is not a canonical nonempty string forbids valued/complete/eligible/supported-minimum outputs. model7 VC01/A04 positives accepted; coherent valued-complete 15M and invented supported_low rejected; `""`, `true`, `{}`, `" gpt-6-astra"`, `7` injected into P132-FG-VC15 rejected at both; V01 priced 20–80M stays finite. Launch `model` 7/""/absent rejected by MC-1 at Python. Bound: `null` identity is exempted at both consumers (D-3). |
| R8-COVERAGE-01 | `closed_bounded` (spec-coherence half); oracle raw-membership half **not attempted, stays full-S1 open** | `accounting.rs::validate_result`: exact monetary inventory, cited links must join a covered anchor to the covered request, inactive units retired only by an accepted coverage. `coverage_namespace_controls.py` 39/39 on joined binary (re-run), 11 fail on predecessor binary `spec-r2-target-pred` (anchor-as-monetary, hold released without/dropped/rejected coverage, 6 dollar-over-admission, bytes-changed) — discriminating; `coverage_r2.rs` 7/7 in cargo-test. Python actual path rejects C2-R8-03's witness swap on VC11 (`R9 validity membership representative mismatch`, from the R10 `reference_relations`). VC07-unlinked keeps `anchor:AtLeast0`. The matrix's "independently fix every raw primary/usage/anchor membership" was not in either packet and is not claimed. |
| R8-RESERVATION-01 | `closed_bounded` | Already fixed at base: Rust axis uses `distinct_rows==1` (no `nums.len()` remains, grep); Python counts byte-distinct raw lines. R10 controls run in gates (`python-controls` 15 ok incl. `test_omitted_reservation_and_validity`; `rust-r10-controls` 24 ok incl. high400/coherent400 mutants rejected `raw reservation maximum/conservative high`). New full-path control: equal-reservations coherent Valid rejected `valid settlement without UniqueReservation (P3)`; coherent high 500→400 rejected by Table X. |
| R8-DOC-01 (P3) | `closed_bounded` | README overclaims removed (finite malformed-identity axis, "joined-2 passed both actual consumers"). Counts verified: full check exits 1 with exactly 476 reference failures and 0 `R8-*` tags; kinds 149 schema / 139 primary membership / 94 retained component / 79 coverage witness / 15 valuation usage match the README exactly; 374 named assertions, 0 REQUIRED/UNMET; 13 Python r11 tests, 6 Rust r11 tests. |
| R8-NAMESPACE-01 | `closed_bounded` **with the authority bound D-1/D-2** | Dollar contract on admission-namespace rows rejected for its named predicate at Python `namespace_application` (6 cases), Rust `evidence` (`namespace application: … raw row is admission-namespace but its initial contract … declares dollar_ticket`, constructed with grant/scope/payload agreeing) and independent Rust `r11_namespace_violation`; predecessor accepts all six. Corrections: VC08/U06/V03×2 split into OriginBase units with `unestablished` contracts, N03×2 contracts withdrawn; money unchanged; outputs agree with DERIVATIONS-S2A (VC08/V03-control rows codex/no obligation; U06/V03-api row 2 PiApi AtLeast0 LaneContradiction/UnresolvedExposure, cause no_reservation; candidates o). R11 private positive bound to `ROOT-ADMISSIONS.json#ADM-bd1c5011d2a2c4d9` from outside; test fails without `R11_ROOT_ADMISSIONS` (verified). All 10 root-admission entries re-verified: canonical inputs digest, every covered origin's raw-line digest, raw namespace and complete extent match. Residual: see D-1, D-2, D-4, D-5. |
| R8-GATE-IDENTITY-01 | `closed_bounded` | Python `gate_qualification` requires all three ids present, nonempty, whitespace-canonical before equality (`GateAuthorityMismatch`); triple absent/""/" "/7 with recomputed payload, witness `authority_id` absent alone, and trailing-space objective id all rejected at the actual path via the commitment check; P132-FG-VC15 positive passes. Rust `mock_binding` adds `canonical_id` for witness and gate ids. Bound: the objective↔gate id comparison in Rust exists only in the test helper `gate_identity_triple` (line 606), not in a Rust pilot path. |
| R8-LOW-INVENTORY-01 | `closed_bounded` | Five scopes (R08 o/p `S1/turn-1:2`; R12 o, A04×2 o `S1/turn-1:1`) now carry the candidate in `alternative_origin_refs` with Seed `[]`, low `0` (LOW-ASSERTIONS + bundles). Python `low_inventory` and Rust `r11_low_violation` reject joint omission and candidate-promoted-to-seed at all five. The chain is protected: jointly deleting the candidate from attribution as well is rejected by the raw O-7 check (`unadmitted candidates [] != O-7 ['o'] (R-12)`). Bound: `r8-completion-A/DERIVATIONS.json` scope_claims still omit the five origins (root decision R8-COMPLETION-A-DERIVATIONS; embedded in the R10 admission). |
| R8-PLACEMENT-01 | `closed_bounded` | Raw P132-CP: reservation 06:30:00, no retirement/settlement, `p` closed 06:40 (`close.at == closed_at`) → `[06:30, open)` straddles closure; corrected to `straddles_closure` + `PlacementStraddles` in both evaluations, attributed to the batch-1 dossier. Python `raw_placement` rejects `inside`+matching reasons, `after`, `straddles_start`, `unplaceable` for the named predicate; P130-T keeps `before` and rejects `inside`/`straddles_start`. Rust `r11_placement_violation` rejects the `inside` mutant (`raw straddles_closure`). Bound: Rust predicate covers fully dated owned attempts on trusted uncontradicted lifecycles only; P130-T's `before` depends on the D-SCOPE extension root admitted (ADM-10dfd6275fd807c5). |
| R8-OWNER-01 | no regression | `c_owner_cli.py` rc=0 (re-run), `c_owner_domains` 12/12 in cargo-test, `lib.rs` change is only `mod namespace`. |
| R8-SCHEMA-DOMAIN-01 | no regression | `nano_domain.py`, `numeric_domains.py` (983 checks), `no_panic_mutations.py` rc=0 re-run on the joined binary; `numbers.rs` P3-1 stand-in change is documented with its limits and covered by `c_owner_domains`. |

## Defects

**D-1 — P2 — inherited (predecessor binary and base validator accept the same mutant); partly disclosed.
Actual consumers accept subject-declared conditional scopes; root's Q-NAMESPACE refusal is unenforceable at the
actual path.**
Location: `validator/validate_corpus.py::namespace_application` (requires only that `grant_ref` and `scope_id`
be present), `check_accounting_references` (never calls `admission_namespace_control`), `tooling/spec-check/src/
namespace.rs` (`admission` declared over admission rows is accepted whatever the grant's authority).
Executed counterexample (`/tmp/rv-review/p1`): take the base (`2729107`) VC08 case file, bundle and provenance
(one grouped unit `adm:S1/turn-1:rid=1` over both origins, one base, conditional grant, provenance payload) and
text-replace `dollar_ticket`→`admission` in bundle and provenance. Results: Python `check_case` → `[]`,
`check_accounting_references` → `[]`; joined Rust `evidence` → `structurally_valid`, rc 0; predecessor binary →
rc 0. Only the independent Rust test rejects it, and only through the hard-coded units-shape assertion at
`independent_raw_axes.rs:2135` (and its Python twin `Namespace01.test_origin_base_split…`), not through the
namespace predicate. Expected: a `dollar_ticket`/`admission` contract whose grant is not in the root admissions
record bound from outside is `REQUIRED/UNMET` on the actual path (ROOT-DECISIONS-R1 D-SCOPE: "consumers bind it
separately and a subject cannot supply or extend it"). The join's Python message "declares … without an admitted
scope grant" and JOIN-REMEDIATION's "asserts an admitted scope" describe a check that does not exist; the check is
grant-presence. Consequence for the finding: the dollar-over-admission misapplication is closed; the broader
C1-R8-04 title ("grouping lacks the guarantee it consumes") can be re-created by self-declaration. Local fix:
call `admission_namespace_control`-style matching for every non-`unestablished` contract inside
`check_accounting_references`, with `R11_ROOT_ADMISSIONS` required (not optional) on the full route.

**D-2 — P3 — inherited. Grouping without any scope is accepted by both actual consumers.**
Executed (`/tmp/rv-review/p15`): base VC08 case+bundle (grouped unit/base) with the contract set to
`unestablished`, no grant, provenance withdrawn. Python `check_case`/`refs` → `[]`; joined Rust `evidence` → rc 0;
predecessor binary rc 0; base validator `[]`. Expected per BASE §3 as root applied it in
ROOT-DECISION-R2 ("without an admitted scope or exact link each origin is OriginBase"): a multi-origin base/unit
under `unestablished` is not coherent. The OriginBase rule that justified option B is enforced nowhere on the
actual path; it is pinned only by test assertions for four named cases. `namespace.rs` states "grouping policy is
separate" but no consumer owns it.

**D-3 — P3 — introduced by design choice in the new predicates. `null` identity disappears into a complete
favourable set.** Location: `validate_corpus.py` `row[f] is not None` (identity check), `independent_raw_axes.rs`
`malformed_identity` (`!v.is_null()`). Executed: `response_model: null` injected into the request row of
P132-FG-VC15, A04 and VC01 with valued/complete/eligible outputs unchanged → no identity failure at Python (on
registered cases only the unrelated `raw input binding changed` digest failures appear, which the packet's
standard does not count). Rust `price_range` likewise ignores null. The matrix row enumerates
"malformed/missing/null/empty/wrong-type"; PLAN requires absence-versus-null preservation. No current case carries
a null identity (scan: 0). I did not locate canonical V-1 text settling whether null is "absent"; if policy
treats null as absent, record that explicitly instead.

**D-4 — P3 — introduced. Three raw-namespace classifiers disagree.** `namespace.rs::classify` classifies only
`usd_reserved`+`rid` as dollar (a `request` row carrying `usd.rid` is unclassified) and requires `rid` on
`subscription_proxy_admitted`/`subscription_proxy`; Python `raw_reference_namespace` and the independent Rust
`r11_raw_namespace` classify `usd.rid` as dollar and any `subscription_proxy` object as admission. Executed:
an `admission` contract over P9's settlement row `S1/turn-1:2` is rejected by Python
(`namespace admission does not apply to S1/turn-1:2 (raw namespace dollar_ticket)`); the same shape cannot be
driven through the CLI with one source because the payload resolver rejects two fragments per source
(`missing or ambiguous source fragment`), so the Rust-side acceptance is by source reading only. No current case
has a settlement-only dollar source or a rid-less admission row; this is a representation disagreement to align,
not a current-case bypass.

**D-5 — P3 — inherited. Python actual path ignores a dangling `initial_contract_ref`.** Executed
(`/tmp/rv-review/p17`): VC08 bundle origins pointed at contract `nope` → Python `check_accounting_references`
`[]` (`namespace_application` does `continue`); Rust `evidence` rejects `dangling initial contract`. Joint path
is covered, but no gate runs Rust `evidence` over the full bundle set (only the 10 changed bundles, the R11
control and VC07 controls), and 149/182 bundles fail `evidence` anyway (census re-run: 33 pass, 149 Draft2020-12).

**D-6 — P3 — disclosed, root-decided. Stale derivation record.** `oracle/r8-completion-A/DERIVATIONS.json`
`scope_claims` still assert empty U for the five LOW-INVENTORY scopes while LOW-ASSERTIONS/bundles/dossier say
otherwise (REVISIONS.json `not_changed`; ROOT-DECISION-R3). A supplied derivation record contradicting the
accepted truth remains in the subject.

Observations (not defects): the Rust independent r11 predicates return "no violation" on structural absence
(`?` on missing arrays/contracts), so they are meaningful only jointly with the structural consumers; Rust
`r11_placement_violation` uses `Box::leak` per call (test-only); `test_r11_evidence` shells out to `git show
2729107:` and therefore needs a git checkout of the subject.

## Truth, authority, independence, scope

* Raw inputs unchanged: for all 175 cases, `inputs`, `evaluations` minus `expected`, `trusted_fixture_inputs`,
  `profile` and every other top-level member are byte-equal to `2729107`; exactly five case files changed
  (P132-CP, VC08, U06, V03×2), outputs only. `oracle/r11-revisions/REVISIONS.json`: 52 entries, each naming the
  finding, field, from/to, derivation anchor and rule; R11 control case inputs equal VC08 inputs.
* Independence: `independent_raw_axes.rs` imports only `serde_json`, `sha2`, `std` (no spec crate, no
  `Command`); Python predicates read raw lines and supplied outputs, not generator outputs. `low_inventory`
  derives U from supplied attribution, which the raw O-7 check protects (verified by the joint mutant).
* Gate: `replay_gates.sh` runs `coverage_namespace_controls.py` and both `test_r11*.py`; root's final replay is
  17/17 rc 0; without `R11_ROOT_ADMISSIONS` the gate fails (`PythonControlRoute`), so the positive is really
  bound from outside. `evidence_controls`: 3 of 4 VC07 negatives reach semantic predicates; `relabel-dollar` is
  rejected by the schema's `maxItems 0` oneOf (disclosed as the intended encoding).
* Beyond-scope claims: none found except the "admitted" wording in D-1. The 476 full-path reference failures and
  149 stale bundles are inherited and correctly recorded as full-S1.

## What I read, executed, did not cover

Read: PACKET, both slice packets, JOIN-REMEDIATION, all four builder results, DERIVATIONS-S2A, root decisions
R1/R2/R3, ROOT-ADMISSIONS v2, PLAN.md matrix, all six R8 review reports, the full `git diff s1-successor
join-r2` for `validate_corpus.py`, `independent_raw_axes.rs`, `namespace.rs`, `accounting.rs`,
`applicability.rs`, `corpus.rs`, `lib.rs`, both r11 test files, `coverage_namespace_controls.py`,
`r9_spec_applicability.rs`, the READMEs, P132-CP/LOW-ASSERTIONS/R08 bundle diffs, VC08 bundle/provenance,
REVISIONS.json, the R11 control files.
Executed (joined binary/test binary, scratch copies under `/tmp/rv-review`): full Python check (175/374, 476
failures, 0 R8 tags); r11 Rust 6/6; the five regression/control scripts (all rc 0) and the predecessor-binary
comparison (11 discriminating failures); bundle census 33/149; `test_r11_evidence` without the admissions env;
mutants P1 (self-declared admission), P15 (unestablished grouping), P17 (dangling contract), P2 (admission over
settlement row), P3/P3b (FIN evidence omission/charge), V01 and high-only FIN variants, identity
launch/null/wrong-type variants, gate-identity variants, placement variants, LOW joint-attribution mutant at
subject level, VC11 witness swap at Python, root-admission digest/namespace/extent verification for all 10
entries.
Not covered: `coverage_r2.rs` and `namespace.rs` unit tests were not re-run by me (cargo-test gate output read
only); dossier JSON-schema validation not re-run; `numbers.rs` stand-in logic reviewed only through its
documentation and passing controls; no re-derivation of money for the four split cases beyond agreement with
DERIVATIONS-S2A; the other 165 cases' outputs were exercised only through the actual consumers' positives.
Cost: within the ~$20 target.

## Re-check of remediation (3883d04)

Same review episode, same reviewer and same-provider standing; read-only; scratch under `/tmp/rv-review2`. Scope:
`git diff 53e7db7 3883d04` only (commits `042eddc` oracle, `3883d04` spec; 13 files). Subject: clean checkout
`/tmp/rootv-final` (git status 0 lines before and after), binary `root-verify-target-r3/debug/pi-accounting-spec-check`
sha256 `2c17a107…`, test binaries from `root-verify-target-r3/debug/deps`. Root admissions bound from
`slice2/ROOT-ADMISSIONS.json` where stated. My original mutants were rebuilt on `3883d04` (p1, p15, p17) and re-run.

| Defect | Disposition | Executed evidence on 3883d04 |
|---|---|---|
| D-1 (P2) self-declared `admission` scope | **closed** (admission namespace); dollar_ticket remains the recorded full-S1 limit | p1 (base VC08 grouped under subject-declared `admission` contract + grant + payload): Python `check_accounting_references` → `REQUIRED/UNMET: no exact root admission (R8-NAMESPACE-01)` **with** root admissions bound, **without** any binding, with a nonexistent path, and with a forged admissions file **inside the subject** (`root_admissions_path` realpath rule). Only a caller-bound file outside the subject containing a forged VC08 entry is honoured (refs `[]`): that is the design boundary (the caller is the admission source), not a subject channel. R11 control: passes via `R11_ROOT_ADMISSIONS=…/ROOT-ADMISSIONS.json`, UNMET without it and UNMET with a byte-identical copy placed inside the subject. Rust `evidence` still accepts p1 (rc 0): no admissions input, recorded as a bound in `oracle/README.md` as root directed. Independent Rust `r11_namespace_application` still rejects p1 (structural pin). |
| D-2 (P3) grouping without scope | **closed** | p15 (grouped base/unit under `unestablished`, no grant): Python → `base base:adm:S1/turn-1:rid=1 groups [...] without an admitted scope or exact link/identity binding; BASE 3 OriginBase per origin (review-r2 D-2)`; Rust `evidence` → `groups 2 origins under an unestablished contract … no admitted exact link or identity binding`, rc 1. `coverage_namespace_controls.py` now 43/43 incl. the four regroup negatives; `coverage_r2.rs` 8/8 (new test: unlinked pairs rejected; link-joined, binding-joined and scoped groupings accepted; different attempt ids rejected). Rust D-2 reads `inv` = input origins (`accounting.rs:218`), so `initial_contract_ref` resolution is correct. Census: the same 33/182 bundles pass as at `53e7db7`; none newly rejected. |
| D-3 (P3) `null` identity | **closed** | `response_model: null` with unchanged valued/complete/eligible outputs → `malformed identity ['response_model'] … (R8-IDENTITY-01, V-1/V-6)` on P132-FG-VC15, A04 and VC01 at Python; absent member → no identity failure (V-1 handling kept). Rust `malformed_identity` no longer exempts null and `Value::Null` is in the injection control; r11 Rust 7/7. |
| D-4 (P3) classifier disagreement | **closed** | One rule table `oracle/r11-controls/raw-namespace-rules.json` (D1/D2/A1/A2/X/N) implemented identically in Python `raw_reference_namespace`, independent Rust `r11_raw_namespace` and spec `namespace.rs::classify` (diff read; `conflict` class added and rejected under any declared namespace at all three). Census of 333 corpus rows + 6 synthetic rows reproduced by Python (`test_r11_review` D4), independent Rust (`r11_raw_namespace_census_agreement`) and spec lib test (`classifier_reproduces_the_shared_census_over_every_corpus_row`, which also re-derives every corpus line from the table written out literally); all three executed, pass. My P9 counterexample (`admission` contract over the `usd.rid` settlement row) → `does not apply to S1/turn-1:2 (raw namespace dollar_ticket)` at Python. Bound (disclosed): the census file is generated by the Python classifier; agreement is consumer-to-census plus the Rust literal-table re-derivation. |
| D-5 (P3) dangling `initial_contract_ref` | **closed** | p17 → Python `origin S1/turn-1:1 has dangling initial_contract_ref 'nope' (R8-NAMESPACE-01, review-r2 D-5)` (both origins); Rust `evidence` `dangling initial contract` rc 1; independent Rust predicate now returns a violation instead of `None`. |
| D-6 | root-decided (ROOT-DECISION-R3), unchanged | — |

Regression / new-authority-gap check: `check_case` 0 failures over 175 cases; `check_accounting_references` 476
failures (unchanged inherited set) with **no** `R8-*`/`REQUIRED/UNMET`/`review-r2` tags both with and without
`R11_ROOT_ADMISSIONS` (no corpus bundle declares `admission`, so the full route does not depend on the binding);
`test_r11_review` 10/10 with and without the env, `test_r11_findings` 13/13, `test_r11_evidence` 5/5 (with env),
`coverage_namespace_controls` 43/43, prebuilt `namespace` 7/7, `coverage_r2` 8/8, `structural_r8` 11/11,
`r9_spec_applicability` 7/7, r11 Rust 7/7; root's `root-verify-final-r3` 18/18 rc 0 read. Fixture repairs in
`r9_spec_applicability.rs`/`structural_r8.rs` add an admitted scope grant or split bases; no assertion weakened.
Python `namespace_application` message now says what it checks (`without grant_ref/scope_id`). No new authority
channel was introduced: the only admission source is the caller-bound path, subject-internal paths are refused by
realpath prefix, Rust `evidence` is documented as not being admission. Notes, not defects: a malformed
caller-supplied admissions file (no `entries` key) raises rather than reporting UNMET (fail-closed); the
`admission_contract_matches` default `e.get('raw_input_sha256', digest)` tolerates a record entry lacking that
member (root's record has it in every entry; caller-side).

**Final verdict after remediation: `ACCEPT` (bounded).** D-1..D-5 closed as executed above; R8-NAMESPACE-01 is now
closed_bounded on its stated authority terms for the admission namespace, with the `dollar_ticket` D-SCOPE wiring
and the Rust `evidence` no-admissions-input bound recorded by root as explicit full-S1 limits; all other per-finding
dispositions of the main review stand; `architecture_assessment` remains `bounded_repair_credible`.
