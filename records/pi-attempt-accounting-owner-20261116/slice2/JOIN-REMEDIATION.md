# Join remediation (slice 2 join, branch `join-r2`)

Root: Pi session `01a1e569`. Work directly on the joined branch
`/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/join-r2` (branch `join-r2`,
head `0ccaf02` = oracle-r2 + spec-r2). One writer at a time: the oracle first, then the spec
author. Commit on `join-r2`. Cargo target `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/join-r2-target`.
Same ownership boundaries as your slice-2 packets.

## Root finding at the join

Root's replay of `join-r2` (`slice2/root-verify-join-r2/`) passes 15/16 gates;
`coverage_namespace_controls.py` fails 6 checks with `missing string grant_ref`. Root then ran
the joined Rust `evidence` command directly:

- The oracle-corrected bundles for VC08, U06-unlabelled, V03-labelled-api,
  V03-control-labelled, N03-contradicted-admission and N03-control-unlabelled-admission all
  fail `evidence` with `missing string grant_ref`. They declare `reference_namespace:
  "admission"` without a grant. In the spec contract (`accounting.rs` ~line 103,
  `InitialReferenceContract`), any namespace other than `unestablished` asserts an admitted
  scope and requires `grant_ref` and `scope_id`. Since root did not admit Q-NAMESPACE for these
  historical cases (ROOT-DECISIONS-R1, ROOT-DECISION-R2), the truthful shape is
  `reference_namespace: "unestablished"` with one OriginBase unit per origin. Declaring
  `admission` without a grant overstates the facts.
- The private positive control `oracle/r11-controls/admission-namespace/bundle.json` fails
  `evidence` with `payload applicability: missing grant_payloads`. A positive control must pass
  both actual consumers.
- No gate runs Rust `evidence` over these bundles, which is why your gates passed.

## Step 1: oracle (owner of `oracle/**`, `validator/**`, case outputs, oracle tests)

1. Re-express the six corrected bundles (and their provenance/case outputs if affected) with
   `unestablished` initial contracts and OriginBase units, so each passes the joined Rust
   `evidence` command and the Python actual check path, with money unchanged. Update
   `REVISIONS.json` and the dossier claims if wording changes.
2. Make the R11 private positive control pass joined Rust `evidence` (an admitted
   admission-namespace scope with its grant, scope and payload, matching the root admission
   `R11-CTL-admission-namespace-grouped` in
   `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/ROOT-ADMISSIONS.json`), and keep
   the Python route binding root admissions from outside the subject (`R11_ROOT_ADMISSIONS`).
3. Adjust the Python namespace predicate if it treats `admission` without a grant as the
   corrected form; it must still reject a `dollar_ticket` contract on admission-namespace rows.
4. Add a regression check that runs Rust `evidence` over every bundle your slice changed and
   over the R11 control, expecting acceptance (put it in a `validator/test_r11*.py` file so the
   root gate script runs it).
5. Write a short addendum `oracle-r2-evidence/JOIN-REMEDIATION-RESULT.md`. Target cost about
   $5.

## Step 2: spec author (after the oracle commits)

`tooling/spec-check/tests/coverage_namespace_controls.py` derived its "current wrong shape"
negatives and its positive from the supplied VC08/U06/V03 bundles. After the oracle's
correction the supplied bundles are the corrected (`unestablished`) shape. Rebuild the
controls so the negatives explicitly construct the wrong shape (a `dollar_ticket` contract
with its grant/scope/payload over admission-namespace rows) from the corrected base, and the
positive uses an admitted admission-namespace scope; each still fails for its named
predicate. Confirm the corrected supplied bundles are accepted. Short addendum in
`spec-r2-evidence/JOIN-REMEDIATION-RESULT.md`. Target cost about $3.

Then both: the root gate script must pass in full:
`R11_ROOT_ADMISSIONS=/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/ROOT-ADMISSIONS.json /home/bline/cbm-work/pi-accounting-s1-owner-20261116/tools/replay_gates.sh <join-r2> <target> <outdir>`.
