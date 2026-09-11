# Contract Kit and Lifecycle Template Duplication

## Status

Architecture idea, raw observation captured 2026-09-11 from a codebase-graph
maintainability review of `app-server/` requested by the operator, scoped to
the migration target (root remains legacy Condex workflow). Season 13 had just
finished and PPCE Slice 1 had just landed (`d7fee69`).

This document is exploratory. It does not amend `DESIGN.md`, change the
migration roadmap, authorize implementation, or claim these are the only
duplication seams in the tree — only the ones this pass found evidence for.

## Summary

Three duplication patterns recur across `app-server/src` and
`app-server/src/services/*`, found via the codebase knowledge graph rather
than a manual read-through:

1. a small set of contract primitives (`freeze`, `digest`, `canonicalJson`,
   `requireText`/`requireCondition`, `nonempty`, `exactFields`) reimplemented
   independently in roughly nine services' `contract.mjs` files and a dozen
   more top-level `src/*.mjs` files;
2. a security-relevant SQLite file-open hardening routine reimplemented under
   three different names in three stores, and silently absent from two others
   that persist comparably sensitive state; and
3. a `deliverTurn` → wait-for-completion → snapshot → project-pressure →
   build-episode-identity → hand-to-coordinator template duplicated between
   the live and shadow retained-role lifecycle runtimes.

None of these require a design decision about ports, harnesses, or providers.
They are utility- and template-layer cleanup that would reduce the number of
places a future change (or bug fix) has to be made in lockstep, and in one
case they are also a live inconsistency, not just duplication.

## Finding 1: contract kit reimplemented per service

`freeze`, `digest`, `canonicalJson`, `requireText`, `requireCondition`,
`nonempty`, and `exactFields` (or a per-service-prefixed clone, e.g.
`benchText`/`benchExact` in `review-bench/contract.mjs`) are hand-written
independently rather than imported from one place:

- `app-server/src/services/workspace-coordination/contract.mjs:34-38` and
  `app-server/src/services/slice-campaign/contract.mjs:33-37` have
  byte-identical `freeze()` bodies.
- `app-server/src/services/claim-evidence/identity.mjs:53-55` and
  `app-server/src/services/workspace-coordination/contract.mjs:30-32` have
  near-identical `digest()` implementations — one passes an explicit `"utf8"`
  encoding argument to `.update()`, the other does not.
- The same `freeze()` body appears a further two times, unexported, inside
  `app-server/src/retained-role-live-lifecycle.mjs:6-10` and
  `app-server/src/retained-role-shadow-lifecycle.mjs:6-10`.
- The graph search below is representative, not exhaustive; querying the code
  graph for these six names currently returns matches in around twenty files
  under `app-server/`.

Query used for this evidence:

```
search_graph(name_pattern="^(requireText|requireCondition|nonempty|exactFields|freeze|digest|canonicalJson)$")
```

Consequence: a fix or hardening change to any one primitive (for example,
`digest`'s encoding argument, or a future change to what `freeze` does with
`Map`/`Set` values) has to be found and applied in every copy, and nothing
stops the copies from drifting further apart than they already have.

## Finding 2: SQLite store bootstrap hardening is inconsistent

Three stores each reimplement the same TOCTOU-safe file-open routine under a
different name:

- `app-server/src/services/review-episode/sqlite-store.mjs` — `privateDatabase`
- `app-server/src/services/workspace-coordination/sqlite-store.mjs` —
  `openPrivateDatabase`
- `app-server/src/services/slice-campaign/sqlite-store.mjs` —
  `openPrivateSqliteDatabase`

Each does the same thing: open with
`O_CREAT | O_EXCL | O_RDWR | O_NOFOLLOW`, fall back to an `lstat` check for an
existing file, then re-verify `dev`/`ino` didn't change between that check and
the `DatabaseSync` open.

By contrast, `app-server/src/services/claim-evidence/sqlite-store.mjs:554-558`
and the root `app-server/src/sqlite-app-server-state.mjs:963-964` both just
call `new DatabaseSync(resolvedPath)` directly and `chmod` the file
*afterward* — they do not perform the pre-open existence/symlink check or the
post-open identity re-verification that the other three stores treat as
necessary.

This reads as divergence from copy-paste rather than an intentional relaxation
for those two stores: claim-evidence persists canonical claim state and is the
service PPCE Slice 1 just extended, and the root store persists app-server
runtime state — neither looks like a case where the hardening was deliberately
judged unnecessary. Left alone, this is a decision made by accident of which
file a given store's author happened to start from, not a decision made on
purpose.

## Finding 3: live/shadow lifecycle runtimes share an un-extracted template

`RetainedRoleLiveLifecycleRuntime`
(`app-server/src/retained-role-live-lifecycle.mjs`) and
`RetainedRoleShadowLifecycleRuntime`
(`app-server/src/retained-role-shadow-lifecycle.mjs`) share the same shape:

- validate `roleRuntime` (must have `deliverTurn` and
  `adapter.waitForTurnCompletion`), `lifecycleEvidence` (must have `snapshot`),
  and `pressureProjector` (must have `project`) in the constructor;
- `deliverTurn` calls `startTurn` then awaits its `completion`;
- `startTurn` calls `roleRuntime.deliverTurn(turn)` then hands off to a
  private `#completeTurn`;
- `#completeTurn` awaits `adapter.waitForTurnCompletion`, takes a
  `lifecycleEvidence.snapshot`, bails out with a `not_observed` result if the
  snapshot's `latestTokenUsage.turnId` doesn't match, projects pressure via
  `pressureProjector.project`, bails out again if projection didn't succeed,
  resolves a per-role coordinator, and builds the same
  `` `turn:${logicalRoleInstanceId}:${turnId}:${sequence}` `` episode identity
  before calling into the coordinator.

The two runtimes diverge only in what happens after pressure is projected: the
live runtime also resolves a `pressureController` and calls
`coordinator.run(...)`, while the shadow runtime calls an injected
`projectionForTurn` resolver and calls `coordinator.observe(...)` with a
differently shaped payload. `live-context-lifecycle-coordinator.mjs` (240
lines) and `shadow-context-lifecycle-coordinator.mjs` (469 lines) were not
diffed line-by-line for this pass but are named identically to this live/shadow
pair and are worth checking for the same pattern.

A shared base (or a single parameterized runtime that takes a "finalize"
strategy) would leave only the live/shadow semantic difference visible to a
reader, instead of requiring a line-by-line diff of two ~100-125 line files to
discover it.

## Relationship to existing directions

`provider-turn-harness-runtime-and-operator-projection.md` states that Work
Engine should own context-lifecycle policy explicitly rather than let it live
inside adapters (§1), and separately notes that "existing operation and
context-transition gates provide precursors for safe realization replacement
boundaries" (§9). Finding 3 is a small, concrete step in that direction: making
the live/shadow lifecycle template a single owned thing rather than two
adapters that happen to agree by construction. Findings 1 and 2 are unrelated
to the port split — they are utility-layer hygiene that would be worth doing
regardless of how the ports idea resolves.

## Candidate direction

Introduce narrow, additive extractions without a big-bang rewrite, in roughly
the order least likely to disturb in-flight work:

1. A `contract-kit` module exporting the six primitives, imported by
   `contract.mjs` files one service at a time as they're next touched, rather
   than a repo-wide rename in one pass.
2. A single hardened `openPrivateSqliteDatabase`-style helper used by all five
   stores, closing the claim-evidence and root-store gap explicitly (as a
   reviewed decision, not a silent behavior change) rather than leaving five
   independent judgment calls.
3. A shared turn-completion/pressure-projection/episode-identity base or
   strategy function for the live/shadow lifecycle runtimes, with `run` vs
   `observe` (and their differing inputs) as the only injected difference.

## Non-goals

This idea does not propose:

- a repo-wide mechanical refactor in one commit;
- deciding whether claim-evidence's and the root store's missing hardening was
  ever intentional — only that it should be a decision someone makes on
  purpose;
- resolving the provider/harness/operator port split as a prerequisite to any
  of the above; or
- claiming these three findings are exhaustive — this pass used graph queries
  and targeted reads, not an exhaustive file-by-file review.
