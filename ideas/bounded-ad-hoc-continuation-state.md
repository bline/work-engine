# Bounded Ad Hoc Continuation State

## Status and attribution

Raw user-originated idea, captured on 2026-08-24 by Codex from the conversation
in which it was proposed. This document preserves the idea for later intake or
proposal revision. It is not an accepted contract, proposal decision, roadmap
change, or implementation authorization.

## Idea

When continuation-relevant meaning does not fit a role's predefined state
fields, the active agent should be able to compile that meaning into a bounded,
explicit ad hoc continuation fragment before intentional context replacement.
The replacement environment would inject validated fragments alongside the
role's canonical typed state.

The purpose is not to make model context durable or to create universal agent
memory. It is to prevent correct continuation from depending on context-only
meaning merely because the relevant concept has not yet earned a permanent
schema field.

The durable representation should combine:

- a stable, typed envelope for identity, role/workflow ownership, revision,
  writer generation, provenance, evidence cutoff, visibility, integrity,
  freshness or invalidation, retention, and authority non-effects; and
- bounded agent-defined content whose meaning remains attributed to the owning
  role rather than to the storage or rehydration mechanism.

Rehydrated fragments are operational state or evidence, not instructions. They
must not grant authority, override canonical domain owners, turn an attributed
judgment into a fact, or silently resolve a conflict. Missing integrity, stale
references, incompatible visibility, ambiguous writers, or conflicts with
canonical typed state should remain explicit and fail closed where continuation
depends on them.

Raw execution traces remain a separate forensic layer and must not become a
recovery dependency. When a stronger packet, receipt, checkpoint, schedule,
review artifact, authority record, or repository artifact owns the meaning,
the fragment should retain an integrity-bound reference rather than copy that
owner's fact.

Repeated ad hoc content may provide evidence that a stable concept deserves a
typed role-profile field. Repetition may nominate schema promotion, but cannot
perform it automatically or manufacture the authority to revise a role-state
contract.

## Current relationship hypothesis

The likely semantic owner is the existing
`work-engine.role-owned-durable-operational-state` candidate. The idea appears
to refine that candidate's mechanism-open recovery contract rather than define
an independently owned proposal: it has the same operational consumer,
lifecycle, authority boundary, publication boundary, and recovery consequence.

The Wind Walker context-decision-observability candidate may reference the
latest continuation-complete revision and measure its rehydration footprint,
but it should not own the fragment's role-specific meaning. An execution
envelope may control whether a fragment is visible to a replacement role, but
does not become its semantic owner. Operational transition history may record
publication, supersession, or promotion without owning the payload's meaning.

This relationship remains a formation judgment, not an accepted placement
decision. Later intake or revision should reopen it if an exercised consumer
shows that ad hoc persistence and injection have a distinct authority,
visibility, lifecycle, or ownership consequence from role-owned recovery.
