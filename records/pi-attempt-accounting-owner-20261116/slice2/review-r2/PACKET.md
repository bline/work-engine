# Independent review packet: slice 2 (historical findings), joined branch `join-r2`

You are a fresh, read-only adversarial reviewer (Fable xhigh via `claude-sdk/`, standing in
for Astra xhigh by operator direction; same provider as root and builders: record this as
same-provider review). You authored none of this. Root: Pi session `01a1e569`. Write only your
report.

## Subject

- Repo `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/successor/subject`. Base
  `s1-successor` (`2729107`, accepted bounded: R8-OWNER-01, R8-SCHEMA-DOMAIN-01). Subject: the
  final head of branch `join-r2` (worktree
  `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/join-r2`), which joins
  `oracle-r2` (oracle author) and `spec-r2` (spec author) plus join remediation. Review
  `git diff s1-successor join-r2` and the resulting behaviour as a whole.
- Builder claims to challenge (do not trust):
  `slice2/oracle-r2-evidence/RESULT.md`, `slice2/oracle-r2-evidence/JOIN-REMEDIATION-RESULT.md`,
  `slice2/spec-r2-evidence/RESULT.md`, `slice2/spec-r2-evidence/JOIN-REMEDIATION-RESULT.md`
  (all under `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/`).
- Root's records: `slice2/ROOT-DECISION-R2-NAMESPACE-UNITS.json`, `slice2/ROOT-DECISION-R3.json`,
  `slice2/ROOT-ADMISSIONS.json`, `slice1/ROOT-DECISIONS-R1.json`, root gate results under
  `slice2/root-verify-join-r2*`.

## Scope

The operator narrowed the campaign to the eleven historical findings. Full S1 (all 175
cases' complete dossiers consumed by `check_corpus`, 149 stale R8 bundles, complete A/B/C
review) is out of scope and stays open; do not report its absence as a defect, but do report
anything this change claims beyond its scope.

Findings under review (required consequences and decisive controls are the matrix rows in
`/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/full-realization-r1/planning/PLAN.md`;
original reports in `/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/*-REVIEW.md`):
R8-FIN-01 (P1), R8-IDENTITY-01, R8-COVERAGE-01, R8-RESERVATION-01, R8-DOC-01, R8-NAMESPACE-01,
R8-GATE-IDENTITY-01, R8-LOW-INVENTORY-01, R8-PLACEMENT-01. Also check that R8-OWNER-01 and
R8-SCHEMA-DOMAIN-01 did not regress.

## Properties to examine (not suspected defects)

- For each finding: does the actual consumer path (Python `validate_corpus.py` check route and
  the independent Rust test/CLI) now enforce the required consequence on current subjects, not
  only on a constructed fixture? Would a consistent-wrong-output subject (all supplied fields
  agreeing with each other, contradicting raw truth) still pass anywhere?
- Do negatives fail for their named predicate (not an unrelated schema/digest error), and do
  positives prevent blanket rejection? Do the new controls actually run in the gate script?
- Truth: are corrected supplied outputs (P132-CP placement/reasons, VC08/U06/V03/N03 unit
  split and `unestablished` contracts, R08/R12/P133-A04 U inventories, P130-R02 charge path)
  derived from raw inputs and policy, with raw inputs unchanged and revisions attributed?
- Authority: are root admissions bound only from outside the subject; can the subject supply
  or extend its own admission, expectations or registry? Does any check treat an
  `admission`/`dollar_ticket` declaration without an admitted grant as established?
- Independence: does the Rust financial test import spec financial decisions or Python
  results, or the Python check depend on generator outputs for truth?
- Regressions in pilot controls, structural checks or schema/number domains.

## Execution

You may run commands. Use a scratch `CARGO_TARGET_DIR` under `/tmp` and scratch copies under
`/tmp` for mutants; never modify the subject or evidence directories. The joined binary is
`/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/join-r2-target/debug/pi-accounting-spec-check`.
Spend effort where evidence is decisive; this review has a cost target of about $20.

## Report

Write `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/review-r2/REVIEW.md`:
verdict `ACCEPT` | `ACCEPT_WITH_CHANGES` | `CHANGES_REQUIRED`; per finding a disposition
(`closed_bounded` | `open` with reason); each defect with severity, location, executed
counterexample or precise reasoning, expected behaviour, introduced vs inherited;
`architecture_assessment` (`bounded_repair_credible` | `local_design_correction` |
`design_review_required`); what you read, executed and did not cover; same-provider standing.
