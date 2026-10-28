# Records

Dated documents produced by campaigns and slices: phase plans, contracts,
validations, results and coordination notes. See
[repository layout](../docs/repository-layout.md) for how this directory relates
to the others.

## Kept as written

A record states what was true at its date, and other files cite records by path
and by SHA-256. Editing one changes what it asserts and breaks the citations to
it. Corrections are new records that name what they correct.

Names follow `<area>-<workstream>-<phase>-<kind>.md`, as in
`rust-compiler-c2-plan.md`. Other kinds of record keep their existing homes:
`planning/`, `metrics/`, `reviews/`, `proposals/` and `notes/`.

## Rust campaign records

The Rust campaign records moved here from `docs/rust-*.md` in phase 4 of the
[layout plan](../planning/repository-layout-plan.md): 62 files, flat, with their
names unchanged. 56 are byte-identical to the old path, including the 10 records
whose SHA-256 is cited elsewhere (`rust-c1-r1-validation`,
`rust-campaign-sc3-remediation-plan`, `rust-compiler-c0-contract`,
`rust-compiler-c1-result`, `rust-compiler-c3-plan`,
`rust-generation-integration-plan`, `rust-host-hp2-contract`,
`rust-review-episode-r0-contract`, `rust-review-episode-r1-result`,
`rust-review-provider-rp2-implementation-plan`). The other 6 changed only in
links that pointed at siblings left in `docs/`, now prefixed `../docs/`:
`rust-compiler-c2-plan`, `rust-compiler-implementation-plan`,
`rust-migration-plan`, `rust-review-episode-implementation-plan`,
`rust-review-provider-rp1-implementation-plan` and `rust-workspace-w0-contract`.
Three Rust files stay in `docs/`:

- `docs/rust-development.md`, a durable reference;
- `docs/rust-retained-task-host-contract.md` and
  `docs/rust-context-lifecycle-s8-implementation-plan.md`, which tests under
  `rust/crates/lifecycle-proof/tests/` read from the live tree. They move with
  that code.

New campaign records go here.

## Moved files

Each row gives the old path, the new path, the commit, and any reference inside the
file that is recorded as unresolved. Every reference *to* a moved file that phase 4
found (202 in 67 files) is listed with its class, its resolution before and after,
its treatment and, for historical references, its resolution basis or
`unresolved`, in
[`repository-layout-phase4-resolution-report.json`](repository-layout-phase4-resolution-report.json).

| Old path | New path | Commit | Unresolved references |
| --- | --- | --- | --- |
| `docs/rust-c1-r1-validation.md` | `records/rust-c1-r1-validation.md` | layout phase 4 | none |
| `docs/rust-c2-r2-integration.md` | `records/rust-c2-r2-integration.md` | layout phase 4 | none |
| `docs/rust-c2-r2-validation.md` | `records/rust-c2-r2-validation.md` | layout phase 4 | none |
| `docs/rust-campaign-history-performance-plan.md` | `records/rust-campaign-history-performance-plan.md` | layout phase 4 | none |
| `docs/rust-campaign-hp2-plan.md` | `records/rust-campaign-hp2-plan.md` | layout phase 4 | none |
| `docs/rust-campaign-sc2-implementation-plan.md` | `records/rust-campaign-sc2-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-campaign-sc3-remediation-plan.md` | `records/rust-campaign-sc3-remediation-plan.md` | layout phase 4 | none |
| `docs/rust-ce3-sc2-core-validation.md` | `records/rust-ce3-sc2-core-validation.md` | layout phase 4 | none |
| `docs/rust-ce3-sc2-integration-plan.md` | `records/rust-ce3-sc2-integration-plan.md` | layout phase 4 | none |
| `docs/rust-claims-ce3-implementation-plan.md` | `records/rust-claims-ce3-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-claims-hp2-plan.md` | `records/rust-claims-hp2-plan.md` | layout phase 4 | none |
| `docs/rust-compiler-c0-contract.md` | `records/rust-compiler-c0-contract.md` | layout phase 4 | none |
| `docs/rust-compiler-c1-result.md` | `records/rust-compiler-c1-result.md` | layout phase 4 | none |
| `docs/rust-compiler-c2-plan.md` | `records/rust-compiler-c2-plan.md` | layout phase 4 | none |
| `docs/rust-compiler-c2-result.md` | `records/rust-compiler-c2-result.md` | layout phase 4 | none |
| `docs/rust-compiler-c3-plan.md` | `records/rust-compiler-c3-plan.md` | layout phase 4 | line 16 -> `rust-development.md` (intended `../docs/rust-development.md`) |
| `docs/rust-compiler-c4-result.md` | `records/rust-compiler-c4-result.md` | layout phase 4 | none |
| `docs/rust-compiler-c5-plan.md` | `records/rust-compiler-c5-plan.md` | layout phase 4 | none |
| `docs/rust-compiler-implementation-plan.md` | `records/rust-compiler-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-episode-join-performance-coordination.md` | `records/rust-episode-join-performance-coordination.md` | layout phase 4 | none |
| `docs/rust-episode-owner-join-implementation-plan.md` | `records/rust-episode-owner-join-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-er1-p1-validation.md` | `records/rust-er1-p1-validation.md` | layout phase 4 | none |
| `docs/rust-generation-integration-plan.md` | `records/rust-generation-integration-plan.md` | layout phase 4 | line 45 -> `rust-development.md` (intended `../docs/rust-development.md`) and `model-selection.md` (intended `../docs/model-selection.md`) |
| `docs/rust-h0-implementation-validation.md` | `records/rust-h0-implementation-validation.md` | layout phase 4 | none |
| `docs/rust-h1-r1-remediation-validation.md` | `records/rust-h1-r1-remediation-validation.md` | layout phase 4 | none |
| `docs/rust-host-hp1-result.md` | `records/rust-host-hp1-result.md` | layout phase 4 | none |
| `docs/rust-host-hp1-validation.md` | `records/rust-host-hp1-validation.md` | layout phase 4 | none |
| `docs/rust-host-hp2-contract.md` | `records/rust-host-hp2-contract.md` | layout phase 4 | none |
| `docs/rust-host-replacement-plan.md` | `records/rust-host-replacement-plan.md` | layout phase 4 | none |
| `docs/rust-hp2-ce1-sc0-validation.md` | `records/rust-hp2-ce1-sc0-validation.md` | layout phase 4 | none |
| `docs/rust-hp2-sc1-ce2-validation.md` | `records/rust-hp2-sc1-ce2-validation.md` | layout phase 4 | none |
| `docs/rust-hp3-controlled-host-implementation-plan.md` | `records/rust-hp3-controlled-host-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-hp3-controlled-host-validation.md` | `records/rust-hp3-controlled-host-validation.md` | layout phase 4 | none |
| `docs/rust-hp3-execution-evidence-implementation-plan.md` | `records/rust-hp3-execution-evidence-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-j1-e1-validation.md` | `records/rust-j1-e1-validation.md` | layout phase 4 | none |
| `docs/rust-j1-hp3-parallel-coordination.md` | `records/rust-j1-hp3-parallel-coordination.md` | layout phase 4 | none |
| `docs/rust-j1-owner-join-implementation-plan.md` | `records/rust-j1-owner-join-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-j2-completion-implementation-plan.md` | `records/rust-j2-completion-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-j2-h1-integration-validation.md` | `records/rust-j2-h1-integration-validation.md` | layout phase 4 | none |
| `docs/rust-j2-hp3-parallel-coordination.md` | `records/rust-j2-hp3-parallel-coordination.md` | layout phase 4 | none |
| `docs/rust-j2-implementation-validation.md` | `records/rust-j2-implementation-validation.md` | layout phase 4 | none |
| `docs/rust-j2-r2-remediation-validation.md` | `records/rust-j2-r2-remediation-validation.md` | layout phase 4 | none |
| `docs/rust-migration-plan.md` | `records/rust-migration-plan.md` | layout phase 4 | none |
| `docs/rust-p1-replay-regression-validation.md` | `records/rust-p1-replay-regression-validation.md` | layout phase 4 | none |
| `docs/rust-review-episode-implementation-plan.md` | `records/rust-review-episode-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-review-episode-r0-contract.md` | `records/rust-review-episode-r0-contract.md` | layout phase 4 | none |
| `docs/rust-review-episode-r1-result.md` | `records/rust-review-episode-r1-result.md` | layout phase 4 | none |
| `docs/rust-review-episode-r2-plan.md` | `records/rust-review-episode-r2-plan.md` | layout phase 4 | none |
| `docs/rust-review-episode-r2-result.md` | `records/rust-review-episode-r2-result.md` | layout phase 4 | none |
| `docs/rust-review-episode-r3-plan.md` | `records/rust-review-episode-r3-plan.md` | layout phase 4 | none |
| `docs/rust-review-provider-execution-plan.md` | `records/rust-review-provider-execution-plan.md` | layout phase 4 | none |
| `docs/rust-review-provider-execution-validation.md` | `records/rust-review-provider-execution-validation.md` | layout phase 4 | none |
| `docs/rust-review-provider-rp0-coverage-boundary-validation.md` | `records/rust-review-provider-rp0-coverage-boundary-validation.md` | layout phase 4 | none |
| `docs/rust-review-provider-rp1-implementation-plan.md` | `records/rust-review-provider-rp1-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-review-provider-rp1-validation.md` | `records/rust-review-provider-rp1-validation.md` | layout phase 4 | none |
| `docs/rust-review-provider-rp2-implementation-plan.md` | `records/rust-review-provider-rp2-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-review-provider-rp2-validation.md` | `records/rust-review-provider-rp2-validation.md` | layout phase 4 | none |
| `docs/rust-review-provider-rp3-implementation-plan.md` | `records/rust-review-provider-rp3-implementation-plan.md` | layout phase 4 | none |
| `docs/rust-review-provider-rp3-validation.md` | `records/rust-review-provider-rp3-validation.md` | layout phase 4 | none |
| `docs/rust-sc3d-implementation-validation.md` | `records/rust-sc3d-implementation-validation.md` | layout phase 4 | none |
| `docs/rust-workspace-w0-contract.md` | `records/rust-workspace-w0-contract.md` | layout phase 4 | none |
| `docs/rust-workspace-w1-result.md` | `records/rust-workspace-w1-result.md` | layout phase 4 | none |

## Unresolved references

Three relative links stay broken because their sources are digest-locked: other
files cite the SHA-256 of `rust-compiler-c3-plan.md` and of
`rust-generation-integration-plan.md`, so neither can be edited. Each was written
as a sibling link in `docs/`, and none resolved at the commit that wrote it
(`df9ebb6e`, 2026-10-07): `docs/model-selection.md` first appears on this branch
at `0b0c72d0` (2026-10-21), and `docs/rust-development.md` was first committed at
`0089f947` (2026-10-27). From `records/` the intended targets are:

| Source | Line | Written as | Intended target |
| --- | --- | --- | --- |
| `records/rust-compiler-c3-plan.md` | 16 | `rust-development.md` | `docs/rust-development.md` |
| `records/rust-generation-integration-plan.md` | 45 | `rust-development.md` | `docs/rust-development.md` |
| `records/rust-generation-integration-plan.md` | 45 | `model-selection.md` | `docs/model-selection.md` |

Records outside this directory that cite the old `docs/rust-*.md` paths are left as
written. Read each old path as the `records/` path in the table above.
