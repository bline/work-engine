# ARCHITECTURE.md and roadmap.md refresh — brief

## Status

Brief, 2026-10-27, left by the repository-layout session (Claude session
`4b0947de-6afa-41b4-aade-862183380127`) for a separate session. Not started.
The user chose not to rewrite these files in that session. This brief grants no
authority over doctrine, campaign configuration or file moves.

## Why

Both files describe themselves as current, and both predate the present
direction. Measured 2026-10-27 at `415ceda2`:

- `ARCHITECTURE.md` calls itself "the canonical map of the system's variant
  structure" as it "exists and as it is currently taking shape", and `README.md`
  calls it the "current whole-system map". It was last changed 2026-08-23 and
  mentions Rust once.
- `roadmap.md` was last changed 2026-10-11 and does not mention Rust. Its
  purpose section describes extracting Work Engine from Site2JSON into a
  standalone product.

Changes since then that the two files do not reflect (each to be verified, not
taken from this list):

- Rust is the target for new first-party implementation
  ([`docs/rust-development.md`](../docs/rust-development.md), recorded
  2026-10-07); `rust/Cargo.toml` lists 32 workspace members, and
  `rust/crates/` holds 40 directories.
- `app-server/` is a superseded intermediate path. Its design documents remain
  valid design input, but nothing in `app-server/README.md` says the directory
  is superseded.
- There are two deliberate development levels: outer Codex/Claude skills, scripts
  and CLI workflows (`skills/`, `metrics/`, `campaigns/`), which build the
  inner Rust engine (`rust/`). The outer level has value of its own beyond
  bootstrapping the inner one.
- The context-lifecycle replacement: `README.md` (2026-10-17) says slices S0–S8
  are complete and S9 is in progress; the coordination board on 2026-10-27
  carries S10 traffic. The current slice state is not established here; read it
  from the board and the plan.
- Skills now reach agents through immutable releases (`415ceda2`,
  [`skills/skill-release/SKILL.md`](../skills/skill-release/SKILL.md)).
- The repository layout is being reorganized
  ([layout plan](repository-layout-plan.md)): current design moves to
  `design/`, Rust campaign records to `records/`.

## Consequence wanted

- **`ARCHITECTURE.md`**: a reader gets a true map of what exists now. Each
  subsystem carries a maturity class backed by evidence (code, tests, receipts),
  using the classes the file already uses: implemented, active construction,
  formed direction, exploratory. The map includes both development levels and
  shows `app-server/` as superseded. The file keeps its own division of
  ownership: it does not own doctrine (`DESIGN.md`), rationale
  (`PHILOSOPHY.md`), implementation authority, or roadmap priority.
- **`roadmap.md`**: states the current direction, remaining work and completion
  evidence truthfully. Alternatively the session concludes that the roadmap
  should be retired or replaced, and puts that to the user as a decision.
- **`README.md`**: only where it restates facts from these two files (the "What
  it is becoming" and status paragraphs). Its Structure table belongs to phase 1
  of the layout plan.

## Constraints

- **`roadmap.md` is a campaign input, not only a document.**
  - `campaigns/roadmap.yaml` names it as its `work_source` (`kind: file`), and
    `campaigns/persistent-runtime-state.yaml` and
    `persistent-runtime-state-route-open.yaml` reference it.
  - The strategic-planner record
    `planning/2026-08-20-persistent-runtime-state.yaml` pins its digest
    (`evidence_cutoff.roadmap_revision`).
  - The last receipt in `metrics/roadmap.jsonl` is dated 2026-08-19.

  Before rewriting it, establish from the board and the receipts whether any
  campaign that reads it is active; a rewrite changes that campaign's work
  source. The [strategic-planner skill](../skills/strategic-planner/SKILL.md)
  is the role that reconciles a roadmap with durable evidence and is the
  natural route for the roadmap half.
- **Truth over completeness.** A maturity claim needs evidence. Where none was
  read, write `not established` rather than inferring (see `CLAUDE.md` rules 1,
  2 and 7).
- **No file moves.** The layout plan owns moves. Link to files at their current
  paths; the layout phases rewrite links as files move.
- **`DESIGN.md` is binding** for this work (`AGENTS.md`); read `PHILOSOPHY.md`
  in full, since this is architecture and planning work.
- Other sessions work in the same tree: read the coordination board at start,
  and do not edit files that hold another session's uncommitted changes.

## Sources

- [`app-server/docs/work-engine-planned-architecture.md`](../app-server/docs/work-engine-planned-architecture.md),
  the capstone, and the views under `app-server/docs/architecture/`.
- [`experiments/context-lifecycle-rust/DESIGN.md`](../experiments/context-lifecycle-rust/DESIGN.md)
  and [`IMPLEMENTATION_PLAN.md`](../experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md).
- [`docs/rust-development.md`](../docs/rust-development.md),
  [`docs/rust-migration-plan.md`](../docs/rust-migration-plan.md),
  [`docs/model-selection.md`](../docs/model-selection.md).
- [`rust/FOUNDATION_GATE.md`](../rust/FOUNDATION_GATE.md), `rust/Cargo.toml`
  (workspace members), and
  [`rust/crates/terminal-ui/docs/implementation-roadmap.md`](../rust/crates/terminal-ui/docs/implementation-roadmap.md).
- [`metrics/efficiency-ledger.md`](../metrics/efficiency-ledger.md) and
  [`metrics/experiment-plan-20261026.md`](../metrics/experiment-plan-20261026.md).
- [`HISTORY.md`](../HISTORY.md), the project's self-audit pinned to `5308ae5`,
  for how the current state came about.
- The coordination board (`AGENTS.md` gives the command) for current slice and
  campaign state.

## Done when

- `ARCHITECTURE.md` matches what exists at its revision. Each maturity class
  either cites evidence or says `not established`.
- `roadmap.md` is either refreshed, with the active-campaign question answered
  first, or the user has decided its replacement or retirement.
- `README.md` no longer contradicts either file.
- The changes are committed and announced on the board, with any campaign whose
  work source changed named in the announcement.

## Out of scope

File moves and reference rewrites (the [layout plan](repository-layout-plan.md)),
changes to `DESIGN.md` or `PHILOSOPHY.md`, and campaign configuration changes
beyond what the user decides once the active-campaign question is answered.
