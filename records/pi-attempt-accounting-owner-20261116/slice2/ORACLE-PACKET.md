# Slice 2a packet: oracle owner — close the oracle-side historical findings

Root: Pi session `01a1e569`. You continue as the independent oracle author (you derived
oracle batch 1). The operator narrowed the campaign to the eleven historical findings; full
S1 (all 175 cases, full-dossier consumption by `check_corpus`) is out of scope and stays OPEN.

## Workspace and ownership

Git worktree, branch `oracle-r2` (base `s1-successor` = `2729107`, which contains the
accepted spec fixes for R8-OWNER-01 and R8-SCHEMA-DOMAIN-01):
`/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/oracle-r2`
Cargo target: `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/oracle-r2-target`.
Commit on the branch per finding (author `pi-accounting-oracle-author`). Evidence and your
result go in `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/oracle-r2-evidence/`.

Exclusive writes: `oracle/**`, `validator/**`, `corpus/cases/**` supplied outputs (never raw
inputs or historical bytes), attributed revision/classification records,
`tooling/spec-check/tests/independent_raw_axes.rs`, `tooling/spec-check/tests/independent_r8_controls.rs`
and new separate financial-qualification test files. Not `spec/**` or
`tooling/spec-check/src/**` (a concurrent spec author owns those; if a fix needs a spec
change, stop that item and report the exact need).

## Findings to close (oracle side)

Required consequences and decisive controls are the matrix rows in
`/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/full-realization-r1/planning/PLAN.md`
("Finding and control matrix"). Original reports are in
`/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/*-REVIEW.md`;
dispositions in `ROOT-FINDING-DISPOSITION.json` there. Locations in those reports refer to an
older subject; establish current behaviour on your branch first (a finding may be partly
addressed by the accepted R10 pilot — say so with evidence rather than re-fixing).

- **R8-FIN-01 (P1)** — retained raw charge cannot be erased by matching supplied outputs
  (C2-R8-01; Python `table_x`/raw checks; P9 700M).
- **R8-IDENTITY-01** — malformed/missing/null/empty/wrong-type identity evidence cannot
  disappear into a complete favourable set (A1-R8-02, A2a-R8-01, C2-R8-02; Rust
  `identities`/`price_range`/`accepts_usage_axis`, Python raw checks; model7 controls).
- **R8-RESERVATION-01** — unique reservation counts admitted distinct evidence, not amount
  cardinality (A1-R8-03; `comparison_applicable: nums.len()==1`).
- **R8-GATE-IDENTITY-01** — objective authority.id, gate authority.id and witness
  authority_id must each be present canonical nonempty strings before equality (B1-R8-01;
  Python `gate_qualification` and its Rust counterpart if one exists).
- **R8-LOW-INVENTORY-01** — candidate usage origins stay in complete U without owned credit
  (B1-R8-02; R08, R12, P133-A04 scopes; LOW-ASSERTIONS/derivations/bundle alternatives).
- **R8-PLACEMENT-01** — raw lifecycle interval fixes placement independently of supplied
  reasons (B1-R8-03; P132-CP supplied placement/reasons are wrong per your batch-1
  derivation; P130-T before-window contradiction keeps).
- **R8-NAMESPACE-01 (oracle side)** — admission-namespace origins (VC08, U06, V03 variants)
  cannot consume a dollar_ticket contract/payload; Python reference checks reject even when
  bundle/refs agree. Correct the supplied VC08 provenance/bundle outputs to truth. A separate
  private conditional admission-namespace positive control (outside the 175) is required;
  its admission must be recorded by root — propose the exact declaration in your result and
  root will add it to `ROOT-ADMISSIONS.json` before acceptance.
- **R8-DOC-01 (P3)** — exact qualification counts and supported-axis standing; remove the
  README overclaim about malformed response identity/cardinality.

## Truth and admissions

- Derive truth for every case a finding's controls depend on, using your bound dossier shape
  (`pi-accounting-oracle-dossier/1`; root decisions in
  `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/ROOT-DECISIONS-R1.json`).
  Reuse your batch-1 dossiers. Derive only the additional cases the controls need; record
  them in `oracle-r2-evidence/dossier/`.
- Root-admitted conditional scopes are in
  `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/ROOT-ADMISSIONS.json`. Consumers
  must bind that file from outside the subject (path/digest given by the caller), never from a
  file the subject supplies. If a newly derived case needs a scope admission, list the exact
  declaration and its raw-consistency claim in your result; do not add it yourself.
- Corrected supplied outputs must follow from your derivations, with an attributed revision
  record naming the finding and the derivation. Never change raw inputs.

## Required evidence

- For each finding: positive and intended-reason negative controls at **both** actual
  consumers (Python validator and independent Rust) where the matrix requires both, including
  a consistent-wrong-output negative (all supplied fields agree with each other but contradict
  raw truth). Each negative must fail for its named predicate, not an unrelated schema/digest
  error. Show the control fails on the base commit (`2729107`) where the defect exists.
- The extended gates still pass on your final commit:
  `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/tools/replay_gates.sh <worktree> <target> <outdir>`.
  If you change a gated expectation deliberately, say exactly why.
- Review P3-4 from slice 1b: `evidence_controls` negatives `VC07-missing-completion`,
  `VC07-relabel-dollar*`, `VC07-unlinked-release` are rejected by schema shape, not their
  named predicates. Repair them so they reach their intended predicates, or record why not.
- `RESULT.md`: per finding — current-state evidence, change, controls with exact command
  outcomes, remaining limits; newly derived cases; proposed admissions; corrected outputs;
  anything blocked on spec.

## Cost discipline

The campaign has about $90 left for everything. Your target for this slice is about $25 of
inference. Avoid re-reading large files you already know; prefer targeted reads and small
scripts. If you judge a finding cannot be closed within reason, stop that item and explain.
No network, no paid APIs, no child agents.
