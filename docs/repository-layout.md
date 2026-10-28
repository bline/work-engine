# Repository layout

Where a file belongs in this repository, and why. Recorded 2026-10-27 from
[the layout plan](../planning/repository-layout-plan.md) (revision 3,
`6ec3fd3e`), which records how these rules were set and tracks the migration
still in progress.

## Why location matters

Readers and agents judge a file's standing from where it sits, and new files
get written where the existing examples are. A dated plan beside current
references reads as current direction; a design document nobody updates reads
as true. Location should answer one question before anyone opens the file: is
this still true?

## The deciding question

> Will this file be revised to stay true, or kept as written at its date?

| Category | What it holds | Lifetime | Home |
| --- | --- | --- | --- |
| Doctrine | Binding rules for all work | Changes only with the owner's authority | root: `DESIGN.md`, `PHILOSOPHY.md`, `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md`, `README.md` |
| Current design | What the system is meant to be | Revised in place; marked stale when superseded | [`design/`](../design/README.md) |
| Durable references | Operating knowledge consulted while working | Kept current | `docs/` |
| Tooling | Skill packages and campaign configuration | Versioned; skills reach agents only by [release](../skills/skill-release/SKILL.md) | `skills/`, `campaigns/` |
| Records | Dated plans, packets, phase documents, results, measurements, reviews, observations | Kept as written; corrections are new records | [`records/`](../records/README.md), `planning/`, `metrics/`, `reviews/`, `proposals/`, `notes/` |
| Implementations | Code and digest-bound experiment units | Owned by their build | `rust/`, `app-server/` (superseded), `experiments/` |

## Current homes for common files

These follow from the categories; the categories decide when a case is not
listed.

| Writing | Goes in |
| --- | --- |
| System-level design: architecture, an idea being reconciled into design | `design/` |
| Operating knowledge others need while working, kept current | `docs/` |
| A campaign or slice phase document: plan, contract, validation, result, coordination | `records/`, named `<area>-<workstream>-<phase>-<kind>.md` (for example `rust-compiler-c2-plan.md`) |
| A plan, decision packet or review request prepared outside a campaign | `planning/` |
| A review of an exact revision | `reviews/`, beside a copy of the reviewed subject |
| A formed candidate change | `proposals/` |
| A receipt, probe, baseline or measurement | `metrics/`; give a probe a `-YYYYMMDD` suffix |
| A dated observation | `notes/`; give it a `-YYYYMMDD` suffix |
| A component's own documentation | with the component, such as `rust/crates/<crate>/docs/` |
| An experiment and its evidence | `experiments/<name>/` |

## Placement is not acceptance

A directory says which corpus a file belongs to. It does not accept, authorize
or settle anything inside the file. Where a document carries its own status —
the architecture views mix accepted, candidate and open material under
[`status-grammar.md`](../app-server/docs/architecture/status-grammar.md) — that
status stays authoritative wherever the file sits.

## Moving or citing a file

Other files cite paths and digests, so a move can silently break a reference or
change what it asserts. Each reference to a moved file is in one of three
classes, and a check that passes for one says nothing about the others:

| Class | Resolves | Treatment on a move |
| --- | --- | --- |
| **Live path**: links and mentions in files kept current; paths read by code | at HEAD | rewritten in the same commit; code paths change with the code |
| **Historical**: paths inside records, `path@commit` citations, manifests, fixtures in temporary repositories | at a revision established for that reference — by default the commit that wrote the referencing line, if the path exists there and any digest matches — or **unresolved** | left as written; reported with its basis, or as unresolved |
| **Content digest**: SHA-256 or blob ids | by content, anywhere | the cited file moves byte-identical |

The rules that follow from these, with their reasons:

- **A file whose current digest is cited elsewhere moves byte-identical.**
  Plans and results cite their parents by SHA-256; changed bytes make those
  citations fail.
- **History is not rewritten to a new layout.** Records state what was true at
  their date.
- **A path read by code moves only with that code**, in one commit, with its
  tests passing. Otherwise a test fails or a reader silently finds nothing.
- **Never move a file that holds another session's uncommitted changes.** It
  would pull their work out from under them. Claim moves on the coordination
  board and announce them.
- **Verify a move against its exact move set.** Simulate it and produce a
  before/after resolution report. A partial move breaks references between moved
  and unmoved files that a simulation of the final layout cannot see.

Two paths must not move: `docs/ai-workflow-metrics.jsonl`, whose literal path
Rust code checks, and `experiments/context-lifecycle-rust/`, a digest-bound
unit.

## Migration in progress

As of 2026-10-27, `design/` exists but most of its content has not moved:
current design is still under `app-server/docs/`, `app-server/ideas/` and
`ideas/`. The Rust campaign records moved to `records/` in phase 4, except
`docs/rust-development.md` (a durable reference) and two records that tests read
from the live tree, `docs/rust-retained-task-host-contract.md` and
`docs/rust-context-lifecycle-s8-implementation-plan.md`; those two move with
that code. New files go to the new homes now and may link to the old locations.
The [layout plan](../planning/repository-layout-plan.md) owns the remaining
moves; each needs the user's go-ahead.
