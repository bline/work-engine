# Repository layout plan

## Status and lineage

Revision 3, 2026-10-27, written with the user in Claude session
`4b0947de-6afa-41b4-aade-862183380127`. The user set the direction: a top-level
`design/`, move only the Rust campaign records out of `docs/`, a skills catalog
in scope, and the `ARCHITECTURE.md`/`roadmap.md` refresh handed to a separate
session ([brief](architecture-roadmap-refresh-brief.md)). This plan is not
acceptance of any phase. Each phase runs on the user's go-ahead.

Revision 1 (SHA-256 `38ecc141…abb40`) and revision 2 (`94542929…a8f8`) are
preserved with their external reviews and dispositions in
[`reviews/proposals/repository-layout-plan/38ecc141b304/`](../reviews/proposals/repository-layout-plan/38ecc141b304/disposition.md)
and
[`reviews/proposals/repository-layout-plan/94542929c4c2/`](../reviews/proposals/repository-layout-plan/94542929c4c2/disposition.md).
Revision 2 replaced revision 1's partial phase-4 move, added reference classes
with a per-phase resolution report, separated the catalog's dimensions, and
narrowed phase 5 to direction only. Revision 3 makes a historical reference's
resolution basis something established per reference, records unestablished
ones as unresolved, and corrects the justification of phase 4's named
exceptions. The second review approved committing the plan and its review
package after these corrections; that approval covers no move and no phase.

Counts were measured on 2026-10-27, at `415ceda2` for revision 1 and at
`df288c3d` with the working tree as it then stood for the phase-4 simulations.
Re-measure at the start of each phase; other sessions keep changing the tree.

**Correction to the agreed scope.** The session described 65 `docs/rust-*` files
as campaign records. 64 are. The 65th, `docs/rust-development.md`, is the Rust
direction reference that `AGENTS.md` and `README.md` cite, so it stays in
`docs/`. The miscount came from classifying by filename prefix.

## Outcome

A reader or agent can tell from a file's location whether it states current
design, durable operating knowledge, tooling, or history, and new files land in
the right place without a reorganization pass. Reorganizing must not change any
document's authority or historical meaning. Today the root mixes these: 65 of
the 82 files in `docs/` are `rust-*` files, 64 of them campaign records; the
design anchors live inside the superseded `app-server/`; `ARCHITECTURE.md`
(last changed 2026-08-23) and `roadmap.md` (2026-10-11) predate the Rust
direction; and `skills/` holds research data and a non-skill beside the
installable packages.

## Principle

Sort by **lifetime and authority**, not topic. The deciding question for any
file is: *will it be revised to stay true, or kept as written at its date?*

| Category | What it holds | Lifetime | Home |
| --- | --- | --- | --- |
| Doctrine | Binding rules for all work | Changes only with the owner's authority | root: `DESIGN.md`, `PHILOSOPHY.md`, `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md`, `README.md` |
| Current design | What the system is meant to be: architecture views, the ideas pipeline | Revised in place; marked stale when superseded | `design/` |
| Durable references | Operating knowledge consulted while working | Kept current | `docs/` |
| Tooling | Skill packages and campaign configuration | Versioned; skills reach agents by release | `skills/`, `campaigns/` |
| Records | Dated plans, packets, campaign phase documents, results, measurements, reviews, observation notes | Kept as written; corrections are new records | `records/`, `planning/`, `metrics/`, `reviews/`, `proposals/`, `notes/` |
| Implementations | Code and digest-bound experiment units | Owned by their build | `rust/`, `app-server/` (superseded, frozen), `experiments/` |

**Why the split matters.** A dated plan sitting beside current references reads
as current direction, and a current-design document left un-updated reads as
true. Agents follow whatever is nearest: Codex sessions keep adding
`docs/rust-*.md` and `app-server/docs/` files because that is where the
examples are. The categories make "is this still true?" answerable from the
path.

**Placement is not acceptance.** A file's directory says which corpus it belongs
to. It does not accept, authorize or settle anything inside it. Where a document
carries its own status (for example the architecture views governed by
`app-server/docs/architecture/status-grammar.md`, which deliberately mix
accepted, candidate and open material), that status stays authoritative.

`planning/`, `notes/`, `reviews/` and `proposals/` stay where they are by the
user's decision; `records/` starts with the Rust campaign records and is the
home for new campaign phase documents.

## Reference classes

A move changes what a path resolves to, so every reference to a moved file
falls into one of three classes. Each class resolves differently, and a check
that passes for one establishes nothing about the others.

| Class | Examples | Resolves | Treatment |
| --- | --- | --- | --- |
| **Live path** | Links and path mentions in files kept current; paths read by code | at HEAD | Rewritten in the phase commit; code paths change with the code |
| **Historical** | Paths inside records (dated notes, plans, results, receipts, baselines); `path@commit` citations; manifests that list paths with digests (`experiments/context-lifecycle-rust/design-sources.json`); fixtures used inside temporary repositories | at a source revision established for that reference, or **unresolved** | Left as written. Reported with its basis, or as unresolved. `records/README.md` maps old paths to new ones for readers at HEAD |
| **Content digest** | SHA-256 or blob ids that identify bytes | by content, anywhere | The cited file moves byte-identical; its digest is recomputed at the new path |

**Establishing a historical basis.** The default candidate is the commit that
wrote the referencing line (`git blame`). It counts as the basis only if the
path exists at that commit, and any digest given with it matches there. A
declared baseline is not automatically the basis for every path a record lists:
`design-sources.json` declares `baseline.repository_head` `7583e209`
(2026-09-27), which predates `docs/rust-generation-integration-plan.md`. That
entry resolves at `9b898790`, the commit that wrote it, where the path exists
and the digest matches. A reference with no established basis is recorded as
unresolved, never as verified.

A referencing file's class is decided per file at the start of the phase, from
its own status text, not from its directory. A file whose class is unclear goes
to the user before the phase runs. Reclassifying a record as a living document
is a recorded decision, not a side effect of a rewrite.

## Invariants for every move

Each holds regardless of how a phase is carried out.

1. **No live-path reference newly fails to resolve**, except named, justified
   exceptions inside byte-locked files; the exception list is part of the
   phase. A broken live reference sends an agent to nothing, or to a stale copy
   left elsewhere.
2. **A file whose current digest is cited elsewhere moves byte-identical.**
   Plans and results here cite their parents by SHA-256; changing a cited
   file's bytes makes those citations fail at HEAD.
3. **History stays as written.** Historical references are not rewritten to the
   new layout. They state what was true at their date.
4. **A path read by code moves only together with that code**, in the same
   commit, with the code's tests passing. Otherwise a test fails, or a reader
   silently finds nothing.
5. **No phase moves or edits a file with another session's uncommitted
   changes.** Moving it pulls their work out from under them. Claim the move on
   the coordination board first and announce the result.
6. **Each phase lands as one commit holding the moves and every rewrite.**
   Every commit stays a valid checkout for bisecting, for skill releases, and
   for other sessions.
7. **A phase is verified against its own move set**, simulated before the
   commit, not against the eventual layout. A partial move breaks references
   between moved and deferred files that an all-moved simulation cannot see.

**The check for every phase** is a before/after resolution report, produced by
simulating the exact move set over every reference to a moved file: markdown
links, path mentions and structured path fields. For each reference it gives
the class, the resolution before and after, and the treatment. A phase is done
when:

- no live-path reference newly fails except the named exceptions;
- every historical reference is listed with its established basis, or as
  unresolved;
- every cited digest matches at its file's new path;
- at commit time the move set holds no changes other than the phase's own:
  none unrelated, and none owned by another session.

Paths that must not move: `docs/ai-workflow-metrics.jsonl` (Rust code checks
the literal path: `historical-evidence-wire/src/lib.rs:423`,
`historical-evidence-read/src/source_read/frozen_metrics.rs:148`), and
`experiments/context-lifecycle-rust/` (a digest-bound unit:
`design-sources.json`, `parent-validated-sources.sha256`).

**Pre-existing condition, independent of this plan.** `docs/rust-development.md`
is untracked. Its only commit is in the slice-checkpoint ref
`refs/work-engine/checkpoints/s9-v28-translation-a-20261013/slice-9/candidate-2`,
with the same bytes. 22 committed files name it, so their links to it are
broken on the branch and on GitHub today. Committing it is its owner's
decision.

## Phases

The order is cheapest first; only phase 1 has to precede the others, because the
moves need the rules they implement.

### Phase 0 — skill delivery (done, `415ceda2`)

Agents read skills from immutable releases instead of the working tree or stale
copies. See [`skills/skill-release/SKILL.md`](../skills/skill-release/SKILL.md);
board notice 3315.

### Phase 1 — placement rules go live

**Outcome:** a new file's home is decidable before any move happens, so the
mess stops growing.

New files:

- `docs/repository-layout.md`: the principle, category table, "placement is not
  acceptance", and reference classes, written as categories with reasons, not
  as a file list (`DESIGN.md` §3.2). It is the durable home of these rules;
  this plan is the record of how they were set.
- `design/README.md`: what belongs there, and that placement identifies the
  design corpus without accepting its contents.
- `records/README.md`: what belongs there. It also serves as the old-path to
  new-path index once phase 4 runs.

Updated entry points:

- `AGENTS.md`: one pointer with its reason — new files go where
  `docs/repository-layout.md` says, because location tells readers whether a
  file is current. `AGENTS.md` had uncommitted changes from another session on
  2026-10-27; apply after those land (invariant 5).
- `README.md` Structure table: add `design/` and `records/`.

**Done when:** the three new files exist with that content, `AGENTS.md` carries
the pointer and its reason, and the `README.md` Structure table lists `design/`
and `records/`.

### Phase 2 — skills catalog

**Outcome:** every skill package has a recorded kind and state, its install
targets follow stated rules, and the release tool checks both against what is
actually installed.

`skills/catalog.json` holds one entry per skill: `kind`, `state`, `install`
targets, and the reasons the rules below require. It replaces
`skills/skill-release/install-manifest.json`, so classification and
installation cannot disagree; `skill_release.py` reads it. Descriptions stay in
each `SKILL.md`.

- `kind` is what the package is: `workflow` (a role or process an agent
  follows), `tool` (a capability with scripts), `research` (an experiment
  instrument), `other-project`.
- `state` is how it stands: `active`, `stable` (unchanged but in use),
  `dormant`, `retired`.

Rules, each with its reason:

1. **Entries and skill directories correspond one to one.** A directory without
   a `SKILL.md` is not a skill and leaves `skills/`. An uncatalogued directory
   would ship in releases with no stated status.
2. **Only `active` or `stable` skills are installed.** Each installed
   description costs every worker about 177 bytes of context
   (`metrics/probes/claude-skill-delivery-divergence-20261022.md`). A dormant
   skill spends that for nothing, and a retired one misleads.
3. **An installed `research` or `other-project` skill carries an
   `install_reason`**, because it adds listing cost for work outside the
   current direction.
4. **An `active` or `stable` skill with no install targets carries a
   `not_installed_reason`**, so leaving it out is visibly deliberate.
   Examples: `coordination-channels`, kept out of worker listings during R2;
   `skill-release`, an operator tool.

Release validation, in `plan`, `status` and the tool's tests:

- **Declared against actual.** Every declared entry is a link into the current
  release. This exists: `status` reports `not_linked`.
- **Orphans.** Links into the release root that are no longer declared are
  removed on deploy, and anything else in an agent directory is reported as
  unmanaged. This exists in `plan` and `deploy`.
- **Idempotency.** Deploying the current release again changes nothing. To
  add as a test.
- **Catalog rules 1–4.** To add, checked by `plan` before a deploy.

Outliers to settle:

- **`skills/planning/`** is a 5-line stub of
  `planning/2026-08-20-persistent-runtime-state.yaml` (82 lines) with no
  `SKILL.md`: remove it (rule 1).
- **`ui-design-principles`** is for Site2JSON, another project, and installed on
  the Codex side: uninstall, or record an `install_reason`.
- **`linguistic-register-pilot`** carries 636 pilot data files inside the skill,
  and every release copies them. Moving the data to `experiments/` is
  optional; its scripts take the data directory as an argument.
- **`orchestrator`** (one file, last changed 2026-09-08, not installed): decide
  between `dormant` and removal.

First-pass values are in the [appendix](#appendix-first-pass-catalog-values).
They come from last-change dates and install state only; whether a skill is in
use is not established.

**Done when:** the catalog exists, `plan` reports no rule violations, the
idempotency test passes, and the install lists match the user's choices.

### Phase 3 — root clutter

**Outcome:** the root holds only doctrine, entry points and top-level
directories.

| Item | State | Proposed home |
| --- | --- | --- |
| `review.md` | 2026-08-19 pasted review transcript | `reviews/` (check inbound references at move time; the hits found so far point at `app-server/docs/architecture/review.md`, not this file) |
| `comparative-repository-analysis-workflow-design.md` | 1,269-line design for the skill of that name, last changed 2026-08-20, no literal references | that skill's `references/`, or `design/` |
| `config/chrome-vision.yaml` | the only file in `config/`, no literal references | confirm whether Chrome Vision loads it by convention before moving |
| `evidence/` | empty, untracked | remove |
| `--help` | untracked, 233 bytes, a token-usage summary JSON written 2026-10-27 11:01 | remove |

**Done when:** each item is moved or removed and its resolution report is clean.

### Phase 4 — Rust campaign records to `records/`

**Outcome:** the Rust campaign records live in `records/`. `docs/` keeps its
durable references and, until a later pass, the other historical records listed
under out of scope.

**Destination:** top-level `records/`, flat, file names unchanged. At the same
depth as `docs/`, byte-identical moves keep the records' `../rust/...` links
resolving.

**One wave, after the owners commit.** Simulated on 2026-10-27 at `df288c3d`,
counting links that resolve today and would not after the move:

| Move set | Newly broken | Where |
| --- | --- | --- |
| 53 now (revision 1) | 39 | 6 in digest-locked records, 13 in rewritable moved records, 12 in owner-held deferred records, 8 in other files |
| **62**: all but the 2 code-read records | 25 | 3 in digest-locked records, 13 in rewritable records, 9 in 5 other files |
| **64**: all, together with the code change | 32 | 3 in digest-locked records, 13 in rewritable records, 16 in 8 other files |

Revision 1's 53-file move is withdrawn. It breaks 12 links inside owner-held
records that this plan may not edit, and 3 inside digest-locked records that
point at a record left behind. The phase runs as one wave of 62 or 64.

**Preconditions:**

- The 9 owner-held records are committed by their owners, or the owners agree
  to the move: 8 untracked (`rust-campaign-sc3-remediation-plan`,
  `rust-compiler-c5-plan`, `rust-j2-hp3-parallel-coordination`,
  `rust-review-provider-rp{1,2,3}-implementation-plan`,
  `rust-workspace-w0-contract`, `rust-workspace-w1-result`) and 1 modified
  (`rust-compiler-implementation-plan`).
- For 64: `rust-retained-task-host-contract.md` and
  `rust-context-lifecycle-s8-implementation-plan.md` are read from the live
  tree by `rust/crates/lifecycle-proof/tests/` (`common/mod.rs`,
  `retained_workflow.rs`, `task_question_answer.rs`, `task_result_publish.rs`).
  They move only with that crate's test change (invariant 4), after
  `retained_workflow.rs`'s uncommitted changes land. Otherwise run 62 now and
  move these 2 with their code later.

**Named exceptions (invariant 1).** Three links stay broken because their
sources are digest-locked:

- `rust-compiler-c3-plan.md:16` → `rust-development.md`;
- `rust-generation-integration-plan.md:45` → `rust-development.md`;
- `rust-generation-integration-plan.md:45` → `model-selection.md`.

They cannot be rewritten: `rust-compiler-c3-plan`'s digest is cited by
`rust-generation-integration-plan` and `rust-migration-plan`, and
`rust-generation-integration-plan`'s by `design-sources.json`. None of the three
resolves at the commit that wrote it (`df9ebb6e`, 2026-10-07):
`docs/model-selection.md` first appears on the branch at `0b0c72d0`
(2026-10-21), and `docs/rust-development.md` has never been on the branch.
Today they resolve only at HEAD, and the two to `rust-development.md` only in a
working tree that holds the untracked file. After the move they are recorded as
unresolved, and `records/README.md` names them with their intended targets.

**Digest-locked set (invariant 2):** the 8 tracked records whose current digest
is cited:

- `rust-c1-r1-validation`
- `rust-compiler-c0-contract`
- `rust-compiler-c1-result`
- `rust-compiler-c3-plan`
- `rust-generation-integration-plan`
- `rust-host-hp2-contract` (cited by the slice-campaign fixture)
- `rust-review-episode-r0-contract`
- `rust-review-episode-r1-result`

Recompute the cited set at phase start; the 9 owner-held records may add to it.

**Rewrites:** the 13 links in uncited moved records that point back into
`docs/`. Rewriting them breaks no citation.

**Inbound files.** Classify each at phase start (reference classes):

| File | Links | First-pass class | Treatment |
| --- | --- | --- | --- |
| `app-server/README.md` | 1 | live | rewrite |
| `experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md` | 2 | live (reconciled plan) | rewrite |
| `experiments/context-lifecycle-rust/DESIGN_EVIDENCE.md` | 3 | unclear: a dated assessment, not digest-cited | user decides |
| `experiments/context-lifecycle-rust/BOUNDED_INFERENCE_IMPLEMENTATION_PLAN.md` | 2 | live, but untracked and owner-held | its owner rewrites, or the phase waits |
| `planning/architecture-roadmap-refresh-brief.md` | 1 | live until the refresh session finishes | rewrite |
| `notes/rust-context-lifecycle-slice-state-20261014.md` (64 only) | 3 | historical: an observation note "written at HEAD `cfafee4`" that "carries no authority" | leave; index in `records/README.md` |
| `rust/crates/terminal-ui/docs/implementation-roadmap.md` (64 only) | 2 | live | rewrite |
| `rust/crates/terminal-ui/docs/task-observation.md` (64 only) | 2 | mixed: preserves a 2026-10-13 proposal under a current-disposition section | user decides |

**Left as written (historical):**

- The slice-campaign fixture
  (`rust/crates/slice-campaign/tests/fixtures/subject-handoff.json`). Its test
  runs in a temporary repository, and 2 of its 3 digests already differ from the
  live files.
- The frozen 2026-10-09 baseline under `metrics/baselines/pure-codex-20261009/`,
  which cites the set as the pattern `docs/rust-*-validation.md`.
- `design-sources.json`. Its entry for `docs/rust-generation-integration-plan.md`
  resolves at `9b898790`, the commit that wrote it, not at its declared
  `baseline.repository_head` `7583e209`, which predates the file.

**Done when:** the resolution report shows no live-path reference newly
failing beyond the three named exceptions, every historical reference is listed
with its established basis or as unresolved, the cited digests match at the new
paths, and
`records/README.md` maps every moved file's old path.

### Phase 5 — current design to `design/` (direction only)

**Outcome:** current design has one home outside the superseded
implementation, `app-server/` holds its code and its own records, and no claim
changes its status by moving. This revision settles direction only. Execution
needs its own inventory, reviewed before any move.

The inventory starts from these sets, measured 2026-10-27:

- `app-server/docs`: 79 tracked files — 23 under `architecture/`, 21 under
  `research/`, 16 top-level `*-reconciliation.md`, 18 other top-level
  documents, and one receipt file.
- `app-server/ideas`: 51 tracked — 47 `pending/`, 3 `history/`, 1 `retired/`.
- `ideas/`: 80 tracked — 28 top level, 25 `intake/`, 27 `history/`.
- References: 114 files mention `app-server/docs/`, 50 `app-server/ideas/`,
  and about 93 match a root `ideas/` pattern (a rough count).

It must cover:

- **Classification of each file by the principle.**
  - **Current design** moves to `design/`: the capstone
    `work-engine-planned-architecture.md`, `architecture/`, and design-bearing
    research.
  - **Records** of reconciliation waves go to `records/` or stay with their
    subject.
  - **App-server specifics** stay in `app-server/`: `codex-agents.md`,
    `skills-migration-plan.md`, `service-plane-inventory.md` and the receipt
    ledger `ai-workflow-metrics.jsonl`.
- **Structured path fields, not just links.** The architecture views hold 26
  `owner:`/`source:` lines in 10 files, for example `owner:
  app-server/ideas/pending/deterministic-authority-projection-and-adaptive-organizational-topology.md`.
  Some name current ownership, others historical provenance. Each gets a class
  before anything is rewritten, since rewriting one changes what it asserts.
- **Status preservation.** `design/README.md` states that placement does not
  accept contents. Moved views keep their per-claim status under
  `status-grammar.md`.
- **The ideas merge as a design decision.** `ideas/README.md` describes an
  active reconciled projection with `reconciliation-map.json`, while
  `app-server/ideas/` holds pending, historical and retired material under
  `pending/idea-status-grammar.md`. Merging them must keep those statuses
  distinct, not flatten them into one current corpus. Skill tests
  (`idea-intake`, `proposal-former`, `proposal-packets`) use `ideas/` paths
  only inside temporary repositories and fixtures, so they do not read the live
  directory.
- **Depth.** `app-server/docs/architecture/` is three levels deep and
  `design/architecture/` two, so links to code are rewritten except in
  digest-cited files. Measure the cited set first.
- **Code coupling.** No script or source file names `ideas/` or
  `app-server/docs/` paths in the patterns searched on 2026-10-27. Rust test
  fixtures and `rust/crates/terminal-ui/docs/` do. Re-check at the start.

**Done when:** the reviewed inventory exists, the move's resolution report
passes under every invariant, `design/README.md` indexes the moved documents,
and `app-server/README.md` says the directory is superseded and points to
`design/`.

## Out of scope

- Rewriting `ARCHITECTURE.md` and `roadmap.md`: handed to another session via
  [the refresh brief](architecture-roadmap-refresh-brief.md).
- Merging `planning/`, `notes/` and `reviews/` into `records/`.
- Moving `docs/` records other than the Rust campaign set
  (`manual-decomposition-*`, `causal-continuity-case-study.md`,
  `agent-experiment-*` and others). They are candidates for a later pass under
  the same rules.

## Appendix: first-pass catalog values

Last change is the last commit before `415ceda2`, which touched many skills
mechanically. Installed: C = Claude, X = Codex. Usage is not established;
values marked `?` need the user's judgment.

| Skill | Last change | Installed | Kind | State |
| --- | --- | --- | --- | --- |
| `slice-supervisor` | 2026-10-27 | CX | workflow | active |
| `slice-builder` | 2026-10-27 | CX | workflow | active |
| `slice-completion-commit` | 2026-10-27 | CX | workflow | active |
| `manual-orchestration` | 2026-10-27 | CX | workflow | active |
| `durable-state` | 2026-10-27 | X | tool | active (AGENTS.md runs its chatboard) |
| `claude-builder` | 2026-10-27 | X | workflow | active |
| `claude-recon-implementation` | 2026-10-27 | X | tool | active |
| `codex-adversarial-review` | 2026-10-27 | X | workflow | active |
| `coordination-channels` | 2026-10-25 | – | workflow | active; `not_installed_reason` needed |
| `independent-review-state` | 2026-10-17 | X | tool | active |
| `role-scheduler` | 2026-10-16 | X | workflow | active (AGENTS.md startup) |
| `slice-checkpoint` | 2026-10-16 | CX | tool | active |
| `evidence-battery` | 2026-10-14 | C | tool | active |
| `proposal-former` | 2026-10-10 | X | workflow | active? |
| `proposal-packets` | 2026-10-10 | X | tool | active (CI runs its validator) |
| `skill-release` | 2026-10-27 | – | tool | active; `not_installed_reason`: operator tool |
| `claim-evidence` | 2026-08-31 | CX | tool | stable? |
| `code-change-profile` | 2026-08-30 | CX | tool | stable? (its tests failed before `415ceda2`: analyzer-version binding) |
| `work-engine-mcp` | 2026-08-29 | X | tool | stable (`.mcp.json` launches it) |
| `agent-environment-graph` | 2026-08-28 | X | tool | stable? |
| `agent-instruction-review` | 2026-08-28 | X | workflow | stable? |
| `wind-walker` | 2026-08-24 | CX | workflow | stable (AGENTS.md requires it) |
| `idea-intake` | 2026-08-24 | X | workflow | stable? |
| `strategic-planner` | 2026-08-21 | CX | workflow | stable? |
| `chrome-vision` | 2026-08-20 | X | tool | stable? |
| `repo-search` | 2026-08-20 | CX | tool | stable? |
| `orchestrator` | 2026-09-08 | – | workflow | dormant? |
| `review-bench` | 2026-09-08 | X | research | stable?; `install_reason` needed |
| `comparative-repository-analysis` | 2026-08-31 | X | research | dormant?; `install_reason` needed |
| `linguistic-register-pilot` | 2026-08-30 | X | research | dormant?; `install_reason` needed |
| `ui-design-principles` | 2026-08-22 | X | other-project | stable? (Site2JSON); `install_reason` needed |
| `planning` | 2026-08-21 | – | not a skill | remove (rule 1) |
