# Design

Current design: what Work Engine is meant to be. Files here are revised in place
to stay true, and are marked stale when superseded. See
[repository layout](../docs/repository-layout.md) for how this directory relates
to the others.

## Placement is not acceptance

Being in this directory identifies a document as part of the design corpus. It
does not accept, authorize or settle anything inside it. Each document's own
status stays authoritative; the architecture views, for example, deliberately
mix accepted design, reconciled-but-unaccepted candidates and open hypotheses
under [`status-grammar.md`](../app-server/docs/architecture/status-grammar.md).

## What belongs here

System-level design: architecture views, ideas being reconciled into design,
and design-bearing research.

Not here:

- binding doctrine (`DESIGN.md`, `PHILOSOPHY.md` at the root);
- operating references (`docs/`);
- a component's own documentation (with the component);
- records of design work, such as plans, reviews and reconciliation waves
  (`records/`, `planning/`, `reviews/`).

## Where the design corpus is today

As of 2026-10-27, nothing has moved here yet. Moving it is phase 5 of the
[layout plan](../planning/repository-layout-plan.md), which needs its own
reviewed inventory first. Until then the corpus is:

- the capstone, [`work-engine-planned-architecture.md`](../app-server/docs/work-engine-planned-architecture.md);
- the architecture views, [`app-server/docs/architecture/`](../app-server/docs/architecture/);
- research foundations, [`app-server/docs/research/`](../app-server/docs/research/);
- ideas: [`ideas/`](../ideas/README.md), a reconciled projection, and
  [`app-server/ideas/`](../app-server/ideas/), with pending, historical and
  retired material under its own status grammar;
- the context-lifecycle design,
  [`experiments/context-lifecycle-rust/DESIGN.md`](../experiments/context-lifecycle-rust/DESIGN.md),
  which stays where it is because the experiment is a digest-bound unit.

New system-level design documents go in this directory now, and may link to
those locations.
