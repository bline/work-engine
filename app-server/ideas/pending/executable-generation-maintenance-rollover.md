# Idea: Executable-Generation Maintenance Rollover

**Status:** Post-migration proposal input with an emergency helper implemented

Repeated migration repairs change the executable environment fingerprint and
therefore require a fresh generation-state root. The semantic context should
survive that maintenance boundary, while the predecessor generation identity,
role bindings, attachments, and delivery state must not be copied forward.

The immediate deterministic helper is:

```bash
npm run app-server:prepare-generation -- \
  --from "$WE_MIGRATION_GENERATION_STATE" \
  --to "$HOME/.local/state/work-engine/app-server-migration-generation-live-vNEXT" \
  --socket /tmp/work-engine-migration.sock
```

It refuses a live proxy socket or an existing target, creates a private target,
copies only `semantic-context.sqlite3`, verifies the source stayed stable and
the destination digest matches, and emits a machine-readable receipt. It does
not activate the generation, rewrite shell variables, copy bindings, or start
the proxy.

Post-migration proposal work should make this a first-class maintenance
operation owned by executable-generation lifecycle rather than a shell-level
deployment convention. That operation should stage, attest, activate, and
surface rollback/continuation evidence while preserving ProviderTurnPort,
HarnessRuntimePort, and OperatorProjection as architectural seams.

## Additional reload evidence: 2026-09-08

An in-process reload reached `active_unexercised` and retired its predecessor,
but the successor's first `turn/start` failed with `App Server adapter is not
initialized`. The stable client had initialized the predecessor before the
candidate existed. Candidate validation and activation did not carry that
negotiation state into the fresh internal adapter, so durable generation state
claimed activation while the role environment was not exercisable.

The emergency repair makes the stable bootstrap retain the bounded App Server
initialization exchange and project it into each later role-generation worker.
The successor adopts the already-negotiated response before activation; it does
not initialize the provider process a second time. A regression test now runs a
role turn, replaces the generation in-process, and runs another role turn
through the successor using the single original client initialization.

The first-class maintenance operation should generalize this requirement:
`active_unexercised` must mean that every stable-session prerequisite needed by
the candidate has been projected and validated, not merely that its worker
process passed static generation validation. First exercise should remain a
separate receipt, but it must not be the first point at which missing bootstrap
state can be discovered.

## Real-code findings (2026-09-16 investigation)

Both status claims are confirmed directly, not assumed from this document's
own text.

**The deterministic helper is real and matches this description exactly.**
`prepareGenerationRollover` (`app-server/scripts/prepare-generation-
rollover.mjs:36-83`) refuses a live target socket (`socketIsLive`, checked
before anything else), refuses an existing target root, copies only
`semantic-context.sqlite3` with `COPYFILE_EXCL`, re-hashes source-before,
source-after, and destination to catch a source mutated mid-copy, and returns
a frozen receipt whose `copiedGenerationIdentity: false` and
`copiedRoleBindings: false` fields make the non-copied scope machine-readable,
not just documented. Covered by `app-server/tests/prepare-generation-
rollover.test.mjs`.

**The `active_unexercised` repair is real, tested, and already generalizes
past the single incident.** `CodexAppServerAdapter.adoptInitialization`
(`app-server/src/codex-app-server-adapter.mjs:520-539`) lets a successor
adopt an already-negotiated `initialize` response instead of re-initializing
the provider process. `executable-generation-role-environment.mjs` calls it
in exactly the two places this repair requires: once when a role environment
is constructed from a carried-forward `appServerInitialization` (line 309),
and once live, whenever an `initialize` request actually crosses the
successor's own effect boundary (lines 396-404, inside `handleRequest`). The
regression test named in this document is `"a successor role generation
adopts the stable App Server initialization"`
(`app-server/tests/executable-generation-worker.test.mjs:1967`) — it runs a
role turn to completion, then a second role turn through a successor
generation, using the same single upstream `initialize` exchange throughout.
A second test, `"stable transport keeps in-flight work on its predecessor and
routes later work to successor"` (same file, line 303), independently
confirms the safe-boundary half: in-flight work finishes under the
predecessor generation while new work is already routed to the successor.

## Relationship to the architecture views (2026-09-16)

**`runtime-realization.md` — this is real, additional evidence for that
dimension's own not-yet-real machinery, not merely adjacent to it.**
Re-read directly: §8 (Rematerialization) describes exactly this shape —
"stale realization → current role requirements ... → Candidate Resolution
and Admission, run again → new immutable realization" — and §9 (Safe
Execution Boundaries) describes exactly the in-flight-drains-under-A,
new-work-routes-to-B sequence the worker test at line 303 already
demonstrates. §10 (Realization Identity Reuses Revision/CAS Lineage) expects
"the successor names its predecessor and the reason for transition" —
`executable-generation-store.mjs`'s `beginReload({reloadId,
requestedByTurnId, predecessorGenerationId})` (line 204) does precisely
this, and CAS-checks the active generation still matches the expected
predecessor before transitioning (line 213), the same optimistic-concurrency
shape `mechanisms/revision-cas-and-publication.md` names generally. This
matters because `runtime-realization.md`'s own status text currently credits
only vague "precursor pieces" ("the runtime manifest and compiled role
environments") as real, and explicitly states "none of these is the complete
materialized-realization architecture this page describes." That may
understate the truth: the executable-generation lifecycle (`ready` →
`active_unexercised` → `active_exercised`, `bootstrap_restart_required` as an
explicit stale-transition outcome) looks like a working, tested,
independently-arrived-at instance of §8/§9/§10 together, applied to
rematerializing the `harness_runtime` implementation itself (a concrete
`codex_app_server` process) rather than to a role's provider selection.

**Resolved 2026-09-16, following operator review that sharpened the initial
finding.** `runtime-realization.md` now cites this real code directly at §9
(safe execution boundaries) and §10 (revision/CAS lineage) as confirmed,
implemented instances at the executable-generation substrate layer — while
explicitly stating this is **not** an implementation of `RoleRealization`'s
own rematerialization through Candidate Resolution and Admission (§8, §4),
which remains unbuilt. The operator's own correction mattered: the initial
finding here risked crediting §8-§10 wholesale, when only §9 and §10 are
strongly evidenced and §8 is a nested, narrower analogue rather than the
dimension's own artifact. `mechanisms/transition-fencing-and-leases.md` was
separately pressure-tested and now names this as a third confirmed,
implemented instance — protecting a third kind of thing (which executable
substrate generation may currently realize a role's `harness_runtime`)
distinct from both named fence classes. `mechanisms/revision-cas-and-
publication.md`'s "Runtime Realization" row was upgraded in place (proposed
→ partially implemented via a nested instance) without adding an eighth
instance to its confirmed count of seven.

**`context-lifecycle.md` — the rollover already respects, and never
duplicates, that dimension's ownership.** `semantic-context.sqlite3` is
Context Lifecycle's own owned artifact (§1: "continuation-checkpoint
storage... the external lifecycle ledger"). The rollover script treats it as
an opaque, bit-exact blob — copy, then verify identical digests before and
after — never opening, reinterpreting, or regenerating its contents. This is
the correct boundary: executable-generation rollover operates one layer
below context replacement (which process root the ledger lives in), never
on the ledger's own semantic content (what the ledger says). The two
"generation" concepts — context-lifecycle's context generation and
executable-generation's process generation — are namesakes, not the same
mechanism; worth noting explicitly so a future reader does not conflate
them, the same discipline used to separate Context Observer's "reasoning
environment" from process environment variables elsewhere in this ideas
review.

**No mechanism ownership conflict.** Nothing here is Transition Fencing,
Resource Lease and Fencing, or Authority-Preserving Intent Projection
territory — there is no operator intent being bounded, no mutual-exclusion
resource being fenced beyond the single-writer socket check already
enforced deterministically, and no in-progress semantic judgment being
protected. If accepted as a first-class maintenance operation, this remains
`runtime-realization.md` domain detail (or its own residue, depending on the
citation-gap question above) — not a new dimension, mechanism, or
substrate.
