# Review-r2 remediation (one bounded repair round), branch `join-r2`

Root: Pi session `01a1e569`. Review: `slice2/review-r2/REVIEW.md` (Fable xhigh,
ACCEPT_WITH_CHANGES, sha256 d178267a…). Work on `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/join-r2`
(head `53e7db7`). Oracle first, then spec; commit on `join-r2`. Same ownership as before. Read the
cited defect sections of REVIEW.md for the executed counterexamples.

## Root dispositions

- **D-1 (P2) FIX for the admission namespace.** The Python actual check path must accept a
  non-`unestablished` `admission` initial contract (and any grouping it carries) only when the
  root admission record, bound by the caller from outside the subject, admits that exact
  declaration — reuse `admission_namespace_control`. With no record bound, such a contract is
  `REQUIRED/UNMET` (a failure), never a pass. The reviewer's VC08 self-declared admission
  scope + grant regrouping must fail at the actual path for the named reason. Keep
  `R11_ROOT_ADMISSIONS` (or an equivalent explicit CLI argument) as the only admission source.
  Rust `evidence` has no admission input: record that bound in the README rather than adding an
  admissions channel to the spec CLI.
  The same authority gap for **dollar_ticket** scopes (D-SCOPE) is recorded by root as an
  explicit full-S1 limit; do not wire dollar_ticket admissions in this round.
- **D-2 (P3) FIX.** Under an `unestablished` contract (or no admitted scope), a base grouping more
  than one origin is rejected (BASE §3 OriginBase), at the Python actual path (oracle) and in
  Rust `namespace.rs`/input validation (spec), unless an admitted exact link or identity binding
  joins them under existing rules.
- **D-3 (P3) FIX.** PLAN's R8-IDENTITY-01 row lists null explicitly: a present `null`
  `model`/`response_model`/`authorized_model` is malformed identity evidence at both new
  predicates (Python and independent Rust). An absent member keeps its existing V-1 handling.
- **D-4 (P3) FIX.** Align the raw-namespace classifiers (Python, independent Rust,
  `namespace.rs`) on one rule table, including `usd.rid` settlement rows and rid-less admission
  rows; add a cross-classifier agreement control over all raw rows in the corpus.
- **D-5 (P3) FIX.** Python actual path rejects a dangling `initial_contract_ref`.
- **D-6** stays as root-decided (ROOT-DECISION-R3).

## Evidence

Each fix: the reviewer's counterexample fails for its named predicate at the actual consumer,
positives still pass (VC08 corrected, R11 control with admissions bound, all current cases),
and the root gate script passes in full:
`R11_ROOT_ADMISSIONS=/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/ROOT-ADMISSIONS.json /home/bline/cbm-work/pi-accounting-s1-owner-20261116/tools/replay_gates.sh <join-r2> <target> <outdir>`.
Also show the D-1 counterexample fails when the gate runs without `R11_ROOT_ADMISSIONS`
(no admission source = unmet). Addenda: `oracle-r2-evidence/REVIEW-R2-REMEDIATION-RESULT.md`,
`spec-r2-evidence/REVIEW-R2-REMEDIATION-RESULT.md`. Cost targets: oracle about $6, spec about $3.
