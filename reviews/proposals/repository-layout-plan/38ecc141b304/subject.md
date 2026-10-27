# Repository layout plan

## Status and lineage

Draft, 2026-10-27, written with the user in Claude session
`4b0947de-6afa-41b4-aade-862183380127`. The user set the direction: a top-level
`design/`, move only the Rust campaign records out of `docs/`, a skills catalog
in scope, and the `ARCHITECTURE.md`/`roadmap.md` refresh handed to a separate
session ([brief](architecture-roadmap-refresh-brief.md)). This plan is not
acceptance of any phase. Each phase runs on the user's go-ahead.

Counts below were measured on 2026-10-27 at `415ceda2`, with the working tree as
it stood then. Re-measure at the start of each phase; other sessions keep
changing the tree.

**Correction to the agreed scope.** The session described 65 `docs/rust-*` files
as campaign records. 64 are. The 65th, `docs/rust-development.md`, is the Rust
direction reference that `AGENTS.md` and `README.md` cite, so it stays in
`docs/`. The miscount came from classifying by filename prefix; the per-file
kind breakdown (one `development` entry) showed it.

## Outcome

A reader or agent can tell from a file's location whether it states current
design, durable operating knowledge, tooling, or history, and new files land in
the right place without a reorganization pass. Today the root mixes these:
65 of the 82 files in `docs/` are `rust-*` files, 64 of them campaign records;
the design anchors live inside the superseded `app-server/`; `ARCHITECTURE.md`
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
| Records | Dated plans, packets, campaign phase documents, results, measurements, reviews | Kept as written; corrections are new records | `records/`, `planning/`, `metrics/`, `reviews/`, `proposals/`, `notes/` |
| Implementations | Code and digest-bound experiment units | Owned by their build | `rust/`, `app-server/` (superseded, frozen), `experiments/` |

**Why the split matters.** A dated plan sitting beside current references reads
as current direction, and a current-design document left un-updated reads as
true. Agents follow whatever is nearest: Codex sessions keep adding
`docs/rust-*.md` and `app-server/docs/` files because that is where the
examples are. The categories make "is this still true?" answerable from the
path.

`planning/`, `notes/`, `reviews/` and `proposals/` stay where they are by the
user's decision; `records/` starts with the Rust campaign records and is the
home for new campaign phase documents.

## Invariants for every move

Each holds regardless of how a phase is carried out.

1. **Every live reference resolves after the phase.** A broken link or path
   sends an agent to nothing, or to a stale copy left elsewhere. Live
   references are markdown links and path mentions in files that are kept
   current, plus paths read by code. Check: a link checker over tracked
   markdown reports no new broken relative links, and the old path prefixes
   occur only in the allowed set below.
2. **A file whose current digest is cited elsewhere moves byte-identical.**
   Plans and results in this repository cite their parents by SHA-256; changing
   a cited file's bytes makes those citations fail at HEAD. Check: digests of
   the cited set are equal before and after; git shows pure renames.
3. **History stays as written.** Records, frozen baselines, receipts, test
   fixtures and commit-pinned citations (`path@commit`, `path:line@commit`)
   describe the repository at their date and are not rewritten to the new
   layout. A record's links may be repaired only where invariant 2 allows.
4. **A path read by code moves only together with that code**, in the same
   commit, with the code's tests passing. Otherwise a test fails, or a reader
   silently finds nothing.
5. **No phase moves or edits a file with another session's uncommitted
   changes.** Moving it would pull their work out from under them. Check: `git
   status --porcelain` is empty for the move set at commit time; claim the move
   on the coordination board first and announce the result.
6. **Each phase lands as one commit holding the moves and every reference
   rewrite.** Every commit stays a valid checkout for bisecting, for skill
   releases, and for other sessions.

Paths that must not move: `docs/ai-workflow-metrics.jsonl` (Rust code checks
the literal path: `historical-evidence-wire/src/lib.rs:423`,
`historical-evidence-read/src/source_read/frozen_metrics.rs:148`), and
`experiments/context-lifecycle-rust/` (a digest-bound unit:
`design-sources.json`, `parent-validated-sources.sha256`).

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

- `docs/repository-layout.md`: the principle and category table above, written
  as categories with reasons, not as a file list (`DESIGN.md` §3.2). It is the
  durable home of these rules; this plan is the record of how they were set.
- `design/README.md` and `records/README.md`: what belongs there and what does
  not, each pointing to `docs/repository-layout.md`.
- `AGENTS.md`: one pointer with its reason — new files go where
  `docs/repository-layout.md` says, because location tells readers whether a
  file is current. `AGENTS.md` had uncommitted changes from another session on
  2026-10-27; apply after those land (invariant 5).
- `README.md` Structure table: add `design/` and `records/`.

**Done when:** the four files exist and a new design document or campaign
record has an unambiguous home.

### Phase 2 — skills catalog

**Outcome:** every directory under `skills/` has a recorded status, and what is
installed for agents follows from that status.

- One file, `skills/catalog.json`, holding each skill's status, a one-line
  reason, and its install targets. It replaces
  `skills/skill-release/install-manifest.json`, so status and installation
  cannot disagree; `skill_release.py` reads it. Descriptions stay in each
  `SKILL.md`.
- Status values: `active`, `stable`, `dormant`, `research`, `other-project`,
  `not-a-skill`.
- Check, in the release tool's `plan`: every `skills/*/` with a `SKILL.md`
  appears exactly once; no entry lacks a directory; an installed skill whose
  status is not `active` or `stable` is flagged. Each installed description
  costs every worker about 177 bytes of context
  (`metrics/probes/claude-skill-delivery-divergence-20261022.md`), so
  installing research or dormant skills needs a reason.
- Resolve the outliers:
  - `skills/planning/` is a 5-line stub of
    `planning/2026-08-20-persistent-runtime-state.yaml` (82 lines) with no
    `SKILL.md`: remove it.
  - `ui-design-principles` is for Site2JSON, another project, and installed on
    the Codex side: uninstall, and move it out of this repository or keep it
    as `other-project`.
  - `linguistic-register-pilot` carries 636 pilot data files inside the skill.
    Every release copies them. Moving the data to `experiments/` is optional;
    its scripts take the data directory as an argument.
  - `orchestrator` (one file, last changed 2026-09-08, not installed): decide
    between `dormant` and removal.

The first-pass statuses are in the [appendix](#appendix-first-pass-skill-statuses).
They come from last-change dates and install state only; whether a skill is in
use is not established.

**Done when:** the catalog exists, `plan` reports no inconsistencies, and the
install lists match the statuses the user chose.

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

**Done when:** each item is moved or removed and invariant 1 holds.

### Phase 4 — Rust campaign records to `records/`

**Outcome:** `docs/` holds durable references only.

- Destination: top-level `records/`, flat, file names unchanged. At the same
  depth as `docs/`, a byte-identical move keeps 320 of the 336 relative links
  in these records that resolve today; a deeper folder would break their
  `../rust/...` links.
- Move set: the 64 records, less those with uncommitted changes and those read
  by code.
  - **53 move now.**
  - **9 wait for their owners** (invariant 5): 8 untracked
    (`rust-campaign-sc3-remediation-plan`, `rust-compiler-c5-plan`,
    `rust-j2-hp3-parallel-coordination`, `rust-review-provider-rp{1,2,3}-implementation-plan`,
    `rust-workspace-w0-contract`, `rust-workspace-w1-result`) and 1 modified
    (`rust-compiler-implementation-plan`). New records are created in
    `records/` from phase 1 on.
  - **2 move with their code** (invariant 4):
    `rust-retained-task-host-contract.md` and
    `rust-context-lifecycle-s8-implementation-plan.md` are read from the live
    tree by `rust/crates/lifecycle-proof/tests/` (`common/mod.rs`,
    `retained_workflow.rs`, `task_question_answer.rs`,
    `task_result_publish.rs`). `retained_workflow.rs` had uncommitted changes
    on 2026-10-27.
- Byte-identical (invariant 2): 8 tracked records whose current digest is cited
  — `rust-c1-r1-validation`, `rust-compiler-c0-contract`,
  `rust-compiler-c1-result`, `rust-compiler-c3-plan`,
  `rust-generation-integration-plan` (cited by
  `experiments/context-lifecycle-rust/design-sources.json`),
  `rust-host-hp2-contract` (cited by a slice-campaign test fixture),
  `rust-review-episode-r0-contract`, `rust-review-episode-r1-result`. This
  leaves 3 links broken: 1 in `rust-compiler-c3-plan` and 2 in
  `rust-generation-integration-plan`, all pointing back into `docs/`. They
  resolve at the records' own commits.
- Link repair: of the 16 links that a move would break, all pointing back into
  `docs/` (10 to `rust-development.md`, 5 to `model-selection.md`, 1 to
  `workflow-invariants.md`), 13 are in 6 uncited records. Rewrite those, except
  in the 3 of the 6 that wait for their owners.
- Inbound rewrites in live files: 13 relative links in 6 files
  (`app-server/README.md`,
  `experiments/context-lifecycle-rust/DESIGN_EVIDENCE.md`,
  `experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md`,
  `rust/crates/terminal-ui/docs/implementation-roadmap.md`,
  `rust/crates/terminal-ui/docs/task-observation.md`,
  `notes/rust-context-lifecycle-slice-state-20261014.md`); none of these files'
  digests is cited. Plus `docs/rust-` path mentions in kept-current files.
- Leave as written (invariant 3): the slice-campaign fixture
  (`rust/crates/slice-campaign/tests/fixtures/subject-handoff.json`; its test
  runs in a temporary repository, and 2 of its 3 digests already differ from
  the live files), the frozen 2026-10-09 baseline under
  `metrics/baselines/pure-codex-20261009/`, which cites the set as the pattern
  `docs/rust-*-validation.md`, and other dated records.

**Done when:** `docs/` contains no campaign records except the 11 waiting ones,
the link check shows no new breakage beyond the 3 recorded above, and the cited
digests verify at their new paths.

### Phase 5 — current design to `design/`

**Outcome:** current design has one home outside the superseded
implementation, and `app-server/` holds its code and its own records.

- Inventory first, from these measured sets:
  - `app-server/docs`: 79 tracked files — 23 under `architecture/`, 21 under
    `research/`, 16 top-level `*-reconciliation.md`, 18 other top-level
    documents, and one receipt file.
  - `app-server/ideas`: 51 tracked — 47 `pending/`, 3 `history/`, 1 `retired/`.
  - `ideas/`: 80 tracked — 28 top level, 25 `intake/`, 27 `history/`.
  - References: 114 files mention `app-server/docs/`, 50 `app-server/ideas/`,
    and about 93 match a root `ideas/` pattern (a rough count).
- Classify each file by the principle:
  - **Current design** moves to `design/`: the capstone
    `work-engine-planned-architecture.md`, `architecture/`, and design-bearing
    research.
  - **Records** of reconciliation waves go to `records/` or stay with their
    subject.
  - **App-server specifics** stay in `app-server/`: `codex-agents.md`,
    `skills-migration-plan.md`, `service-plane-inventory.md` and the receipt
    ledger `ai-workflow-metrics.jsonl`.
- Ideas: merging `ideas/` and `app-server/ideas/` into `design/ideas/` means
  reconciling two conventions (`ideas/README.md` with `reconciliation-map.json`
  versus `app-server/ideas/pending/idea-status-grammar.md`). That
  reconciliation is a design decision for this phase, not a mechanical move.
  Skill tests (`idea-intake`, `proposal-former`, `proposal-packets`) use
  `ideas/` paths only inside temporary repositories and fixtures, so they do
  not read the live directory.
- The depth change (`app-server/docs/architecture/` is three levels deep,
  `design/architecture/` two) breaks relative links to code, so links are
  rewritten, except in digest-cited files (invariant 2). Measure the cited
  set first.
- No script or source file names `ideas/` or `app-server/docs/` paths in the
  patterns searched on 2026-10-27. Rust test fixtures and
  `rust/crates/terminal-ui/docs/` do. Re-check at phase start.
- After the move, `app-server/README.md` says the directory is superseded and
  points to `design/`.

**Done when:** `design/README.md` indexes the moved documents, the link check
passes under invariants 1–3, and no current-design document remains under
`app-server/` without a pointer from `design/`.

## Out of scope

- Rewriting `ARCHITECTURE.md` and `roadmap.md`: handed to another session via
  [the refresh brief](architecture-roadmap-refresh-brief.md).
- Merging `planning/`, `notes/` and `reviews/` into `records/`.
- Moving `docs/` records other than the Rust campaign set
  (`manual-decomposition-*`, `causal-continuity-case-study.md`,
  `agent-experiment-*` and others). They are candidates for a later pass under
  the same rules.

## Appendix: first-pass skill statuses

Last change is the last commit before `415ceda2`, which touched many skills
mechanically. Installed: C = Claude, X = Codex. Usage is not established;
statuses marked `?` need the user's judgment.

| Skill | Last change | Installed | First-pass status |
| --- | --- | --- | --- |
| `slice-supervisor` | 2026-10-27 | CX | active |
| `slice-builder` | 2026-10-27 | CX | active |
| `slice-completion-commit` | 2026-10-27 | CX | active |
| `manual-orchestration` | 2026-10-27 | CX | active |
| `durable-state` | 2026-10-27 | X | active (AGENTS.md runs its chatboard) |
| `claude-builder` | 2026-10-27 | X | active |
| `claude-recon-implementation` | 2026-10-27 | X | active |
| `codex-adversarial-review` | 2026-10-27 | X | active |
| `coordination-channels` | 2026-10-25 | – | active, deliberately not installed |
| `independent-review-state` | 2026-10-17 | X | active |
| `role-scheduler` | 2026-10-16 | X | active (AGENTS.md startup) |
| `slice-checkpoint` | 2026-10-16 | CX | active |
| `evidence-battery` | 2026-10-14 | C | active |
| `proposal-former` | 2026-10-10 | X | active? |
| `proposal-packets` | 2026-10-10 | X | active (CI runs its validator) |
| `skill-release` | 2026-10-27 | – | active, operator tool, not installed |
| `claim-evidence` | 2026-08-31 | CX | stable? |
| `code-change-profile` | 2026-08-30 | CX | stable? (its tests failed before 415ceda2: analyzer-version binding) |
| `work-engine-mcp` | 2026-08-29 | X | stable (`.mcp.json` launches it) |
| `agent-environment-graph` | 2026-08-28 | X | stable? |
| `agent-instruction-review` | 2026-08-28 | X | stable? |
| `wind-walker` | 2026-08-24 | CX | stable (AGENTS.md requires it) |
| `idea-intake` | 2026-08-24 | X | stable? |
| `strategic-planner` | 2026-08-21 | CX | stable? |
| `chrome-vision` | 2026-08-20 | X | stable? |
| `repo-search` | 2026-08-20 | CX | stable? |
| `orchestrator` | 2026-09-08 | – | dormant? |
| `review-bench` | 2026-09-08 | X | research |
| `comparative-repository-analysis` | 2026-08-31 | X | research |
| `linguistic-register-pilot` | 2026-08-30 | X | research |
| `ui-design-principles` | 2026-08-22 | X | other-project (Site2JSON) |
| `planning` | 2026-08-21 | – | not-a-skill (remove) |
