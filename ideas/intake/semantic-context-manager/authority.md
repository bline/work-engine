# Intake Authority

## Revision 1 — initial intake direction

On 2026-08-24, the user adopted the recommendation to prioritize a Semantic
Context Manager vertical over a complete Codex App Server migration, with the
smallest hooks-based runtime observation seam required for a trustworthy
prototype. The user asked to begin intake of the context-manager idea unless a
pre-intake revision was needed.

Before intake, the primary agent judged that one bounded source revision was
needed because intake binds exact raw-source bytes. The revision adds:

- host-owned agent discovery through a provider-neutral runtime adapter;
- Codex lifecycle hooks as the present prototype realization;
- Codex App Server events as the richer target realization;
- snapshot binding and a retirement fence; and
- two-phase rehydration that retains the old context until the fresh context
  acknowledges the exact handoff.

This authority permits assessment of that exact revised raw source and
publication of an intake record and derived proposal-formation projection. It
does not accept a proposal, authorize proposal formation beyond the projection,
authorize implementation or cleanup, settle permanent placement, or change
roadmap priority.

## Revision 2 — App Server and Wind Walker direction

On 2026-08-24, after reviewing a live App Server context-lifecycle experiment,
the user replaced the hooks-first prototype direction with a parallel Codex App
Server scaffold and asked that the complete recovered context be written to the
planned idea and intake locations. The user adopted the following decisions:

- App Server is the primary first runtime scaffold; hooks are no longer the
  planned prototype route.
- Durable logical role instances bind to threads, and threads may contain
  successive model context windows.
- Wind Walker becomes a centralized durable observer role rather than a skill
  requiring every task agent to manage its own context.
- Observation and lifecycle coordination are centralized while semantic
  preservation remains owned and boundedly validated by the target role.
- Rising pressure triggers preservation; optimality and semantic safety govern
  clearing.
- Native fresh-context replacement is permitted after continuation readiness.
- Opaque summarizing compaction is prohibited as a continuation mechanism.
- An App Server event named `contextCompaction` does not by itself establish
  whether a context was summarized or replaced.
- Snapshot binding, a write fence, transition classification, an external
  lifecycle ledger, and post-transition reconciliation remain required.
- The App Server scaffold and role-port direction belongs in a neighboring raw
  idea rather than being absorbed into the Context Manager candidate.

The user also asked for conflicting neighboring proposals to be examined. This
authority permits revision of the raw Context Manager source, a revision-2
intake assessment and record, exact runtime evidence, relationship nominations,
and the neighboring App Server scaffold raw idea. It does not authorize
rewriting or accepting formed proposals, implementation, cleanup, skill
relocation, permanent placement, or roadmap changes. Apparent proposal
conflicts must remain attributed reopening conditions or relationship
nominations unless their owner supplies separate authority.

## Revision 3 — manual fresh-context confirmation

On 2026-08-24, the user confirmed that with `token_budget` enabled, manual
`thread/compact/start` triggers a fresh context-window transition on the same
durable App Server thread and does not exhibit legacy summarizing-compaction
behavior.

This direct human runtime attestation resolves the revision-2 uncertainty about
the observed semantics of manual compaction in that runtime. It authorizes an
exact-source intake revision that records manual `thread/compact/start` as the
currently observed host initiation surface for native fresh-context
replacement. It does not establish identical behavior across untested Codex
versions, supply the missing raw protocol transcript, prove a separate direct
`new_context` host API, authorize implementation, or change any proposal's
acceptance or roadmap state.
