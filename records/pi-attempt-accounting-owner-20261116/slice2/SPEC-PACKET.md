# Slice 2b packet: spec author — coverage, namespace application and review follow-ups

Root: Pi session `01a1e569`. You continue as the spec/structural author (you made slice 1b,
accepted bounded as `2729107`). The campaign is narrowed to the eleven historical findings.

## Workspace and ownership

Git worktree, branch `spec-r2` (base `s1-successor` = `2729107`):
`/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/spec-r2`
Cargo target: `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/spec-r2-target`.
Commit per item (author `pi-accounting-spec-author`). Evidence/result:
`/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/spec-r2-evidence/`.
Same exclusive writes as slice 1b (`spec/**`, `tooling/spec-check/src/**`, structural/schema/
wire tests, crate files, regenerated views). A concurrent oracle author owns `oracle/**`,
`validator/**`, `corpus/cases/**`, `independent_raw_axes.rs`, `independent_r8_controls.rs`;
do not edit them. New spec-side test fixtures go under `tooling/spec-check/tests/`.

## Required consequences

Matrix rows in
`/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/full-realization-r1/planning/PLAN.md`
and reports in `/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/`.
Establish current behaviour on your branch first.

1. **R8-COVERAGE-01 (spec coherence side)** — C-validity and DefiniteLink apply to the exact
   covered obligation/anchor origins (A1-R8-01 `accounting.rs` `validate_result`; C1-R8-02:
   coverage that retires an anchor must identify the applicable affirmative link to its
   covered request — setting `definite_link_refs` to `[]` in
   `oracle/r8-controls/VC07-conditional.bundle.json` `$.documents[1].coverage_decisions[0]`
   currently still passes `evidence`; C2-R8-03 in C1/C2 reviews). Controls from the matrix:
   witness swap/redirect, joint class omission, self/uncited/other-request links, endpoint
   bytes changed under the same ID, wrong valuation usage, partial/overlapping coverage —
   each rejected for its intended predicate; exact valid budgeted and unbudgeted positives
   survive; unlinked VC07 retains AtLeast0. Financial truth stays with the oracle; you enforce
   structural/typed coherence.
2. **R8-NAMESPACE-01 (Rust side)** — Rust input/evidence checks reject applying a
   `dollar_ticket` contract/payload to admission-namespace origins, even when bundle and refs
   agree with each other (A2b-R8-01, C1-R8-04: VC08 two rows `subscription_proxy_admitted.rid=1`
   and `subscription_proxy.rid=1` share `base:adm:S1/turn-1:rid=1` under a dollar_ticket
   declaration). A correctly declared separate conditional admission-namespace scope is a
   positive (test fixture of yours; not a live guarantee). Note the oracle will correct VC08's
   supplied outputs on its branch; your check must reject the current (wrong) VC08 shape and
   accept the corrected shape class.
3. **Slice 1b review follow-ups** (`/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/review-spec-r1/REVIEW.md`):
   P3-1 sentinel collapse observable through `uniqueItems` at the free-form
   `Assertion.expected_fields_or_relations` location — fix (reviewer option (b) or better) or
   precisely document; P3-3 documentation inaccuracies in README/RESULT.

## Evidence

Discriminating negatives (fail for the named predicate; show they pass on `2729107` where the
defect exists) and positives; extended gates pass on your final commit via
`/home/bline/cbm-work/pi-accounting-s1-owner-20261116/tools/replay_gates.sh <worktree> <target> <outdir>`;
`RESULT.md` per item with exact command outcomes and remaining limits.

## Cost discipline

About $90 remains for the whole campaign; target about $8 for this slice. Targeted reads,
small scripts. No network, no paid APIs, no child agents. Stop and report if an item needs a
new financial policy or oracle-owned edits.
