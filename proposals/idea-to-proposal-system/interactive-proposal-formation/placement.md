# Placement: Interactive Proposal Formation

## Proposed owner

A new Work Engine planning skill, tentatively `skills/proposal-former/`.

The skill owns formation judgment and packet interaction. The durable packet
contract remains separately owned, so another authorized tool or human can
create a conforming packet without using this skill.

## Boundary relationships

```text
idea sources
    ↓
proposal former — forms and revises candidates
    ↓
proposal packets — canonical candidate state
    ↓
evaluator / strategic planner — evaluates value and portfolio consequence
    ↓
authorized bounded objective
    ↓
slice supervisor — executes and records terminal consequences
```

The arrows describe information consumption. They are not a universal runtime
sequence; proposals may be reopened whenever evidence makes prior conclusions
stale.

## Competing placements

### Strategic planner

Credible neighboring owner, but not preferred. Its current contract asks
whether roadmap direction and ordering remain sound. Proposal formation starts
earlier and may yield no roadmap candidate at all. Keeping formation distinct
lets either context be discarded or retained according to its own value.

### Slice supervisor

Rejected. It owns execution of an already bounded campaign and must not silently
acquire product-selection or proposal-definition authority.

### General-purpose interactive UI

Deferred. A UI may later expose valuable controls, but the semantic capability
and durable contract should be learned from real use before interaction widgets
become product structure.

## Evidence and confidence

Placement confidence: probable.

The repository already models capabilities as skills and separates strategic
planning from campaign supervision. A dedicated formation skill preserves that
boundary while allowing both the strategic planner and humans to consume the
same durable packet.

## Reopening conditions

Reopen placement if formation proves inseparable from strategic portfolio
reconciliation, or if proposal formation becomes a product-neutral capability
with independent consumers that justify extraction from Work Engine.
