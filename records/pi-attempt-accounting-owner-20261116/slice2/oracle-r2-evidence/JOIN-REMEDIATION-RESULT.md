# Join remediation, step 1 (oracle): result

Branch `join-r2`, commit `3be98f2` (author `pi-accounting-oracle-author`), on top of `0ccaf02`. Root admissions v2: `slice2/ROOT-ADMISSIONS.json`.

## Changes

1. **Six corrected historical bundles** (VC08, U06-unlabelled, V03-control-labelled, V03-labelled-api, N03-contradicted-admission, N03-control-unlabelled-admission).
   - The initial contract changed from `admission`, which had no grant, to `reference_namespace: "unestablished"` (`unestablished-scope:S1/turn-1`, same_origin_only, no grant_ref or scope_id).
   - The origin contract refs and `continuity.preserved_initial_contract_refs` were updated to match.
   - The OriginBase units are unchanged: two per case for VC08/U06/V03, one for N03. Case outputs, provenance withdrawals and money are unchanged.
   - The raw namespace is still `admission` (dossier NS1). Only the *declared* contract no longer asserts a scope that root never admitted (Q-NAMESPACE).
2. **R11 private positive control.**
   - New resolvable payload `oracle/r11-controls/admission-namespace/provenance.json`, containing `grant_payloads` and `initial_scope_contracts` that equal the bundle contract, plus covered extent and origins.
   - The grant now points at that payload, with `authority_ref` = `ROOT-ADMISSIONS.json#ADM-bd1c5011d2a2c4d9`.
   - The Python route still binds root admissions only via `R11_ROOT_ADMISSIONS`. It qualifies against the v2 entry and reports REQUIRED/UNMET without it.
3. **Python namespace predicate.** A `dollar_ticket` or `admission` contract without `grant_ref` and `scope_id` is now rejected ("declares … without an admitted scope grant; use unestablished (R8-NAMESPACE-01)"). A `dollar_ticket` contract on admission-namespace rows is still rejected ("does not apply to").
4. **Regression test** `validator/test_r11_evidence.py`. The root gate script runs it.
   - Joined Rust `evidence` accepts the six bundles and the R11 control. The binary comes from `SPEC_CHECK_BINARY`; a missing binary fails the test.
   - The six contracts are `unestablished` with no grants.
   - Python rejects an `admission` contract without a grant and a `dollar_ticket` contract on these rows.
   - The control qualifies only with the outside-bound admissions.
   - The four R8-LOW-INVENTORY-01 bundles get the same `evidence` verdict before and after my change (see Limits).
5. **Records.** `REVISIONS.json` gained 8 entries: six bundles and the two control files. Slice-2a dossier wording (NS1 positive) and `RESULT.md` were updated, and all 12 dossiers still validate against `dossier-1.schema.json`.

## Evidence

Joined Rust `evidence` returns `structurally_valid` for all six bundles and the control; before this step each of them failed.

`test_r11_evidence` and `test_r11_findings` run 18 tests, all OK. The Rust `r11_` tests pass, 6/6.

Root gate script (`R11_ROOT_ADMISSIONS=… replay_gates.sh join-r2 join-r2-target oracle-r2-evidence/gates-join`): 16 of 17 gates are rc=0, including `test_r11_evidence.py` and `test_r11_findings.py`.

The one failure is `coverage_namespace_controls.py`, which is Step 2 (spec author). It now fails only its 4 "dollar_ticket applied to admission origins" negatives: they still copy the wrong shape from the supplied bundles, which are now correct, so the copied mutant is accepted (rc 0). Its positive (the admitted admission-namespace control) and its "corrected shape with origin bytes changed" check pass. The `missing string grant_ref` failures are gone.

## Limits

The four R8-LOW-INVENTORY-01 bundles (P130-R08, P130-R12, P133-A04 ×2) are rejected by joined Rust `evidence` under Draft2020-12. The cause is that the slice-1b accounting schema requires `primary_memberships` and related fields. The same rejection occurs on their base `2729107` bytes, so it is the pre-existing full-S1 bundle-shape gap and outside this step. The regression test asserts that my change does not alter that verdict.
