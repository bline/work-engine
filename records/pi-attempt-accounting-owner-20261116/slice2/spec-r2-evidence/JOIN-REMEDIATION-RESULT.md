# Join remediation Step 2 (spec author) — addendum

Branch `join-r2`, commit `53e7db7` (only `tooling/spec-check/tests/coverage_namespace_controls.py` changed; no `src/**`, `spec/**` or oracle-owned file touched). Worktree clean after restoring gate-dirtied `validator/__pycache__`.

## What changed
The supplied VC08/U06/V03×2/N03×2 bundles are now the corrected shape (`unestablished`, no grant, OriginBase units), so the old negatives (which relied on the supplied bundles being wrong) and the positive (which text-replaced `dollar_ticket`→`admission`) no longer applied. Rebuilt:
- **Corrected supplied bundles accepted** (6 positives, real root): each passes the joined Rust `evidence`.
- **Wrong shape constructed explicitly** (`declared_scope`): from each corrected bundle, a private root (spec copy + the case's corpus subject) re-declares every source with a `dollar_ticket` contract + `initial_reference_scope` grant + matching provenance payload (`grant_payloads`, `initial_scope_contracts`, covered extent/origin inventory, scope id), so bundle, contract, grant and payload all agree with one another. All six are rejected by the named predicate: `namespace application: origin … raw row is admission-namespace but its initial contract … declares dollar_ticket`. Earlier payload predicates pass first, so the namespace predicate is the one that fires (not a schema/grant failure).
- **Positive**: the same construction with `admission` (an admitted admission-namespace scope in a private root; not a live guarantee) is accepted. Same fixture with an origin's bytes changed under the same id is rejected: `raw bytes differ from its bound raw row`.

## Results
- `coverage_namespace_controls.py`: PASS 39/39 on `join-r2`. On the 2729107 binary the same script fails 11 (the six wrong-shape negatives, the bytes-changed check, and the four coverage-inventory/hold-release checks), confirming the negatives are discriminating (`/tmp`-level run; predecessor binary `slice2/spec-r2-target-pred`).
- Full root gate: `R11_ROOT_ADMISSIONS=…/slice2/ROOT-ADMISSIONS.json tools/replay_gates.sh join-r2 join-r2-target spec-r2-evidence/join-gates` → **17/17 rc=0** (python-controls, rust-positive, rust-r10-controls, python-main-pilot, generated, impact, schema_agreement, wire_corpus, evidence_controls, cargo-test, c_owner_cli, nano_domain, numeric_domains, no_panic_mutations, coverage_namespace_controls, test_r11_evidence, test_r11_findings). `join-gates/results.jsonl`.

## Limits
Unchanged from RESULT.md: the namespace check judges only origins bound by digest to a readable raw row; the constructed scopes are private test fixtures, not admissions.
