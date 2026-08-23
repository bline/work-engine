# Work Engine Studio

## Status

Exploratory product-surface direction.

## Idea

Provide a human-facing environment for **design, observation, control, and forensics** over Work Engine's existing semantic and runtime structures.

The likely first host is VS Code, but VS Code is not the product boundary.

## Current evidence

Work Engine already has:

- an Agent Environment Graph with generated per-role views;
- structured role/invariant/capability relationships;
- proposal packets and formation;
- durable scheduler/control-plane-shaped machinery;
- checkpoints, receipts, and live-state projections;
- emerging runtime-adapter and execution-envelope ideas.

The full Studio does not exist.

## Views

### Contract / design view

Show stable role and capability contracts, invariants, ownership, observations, mutations, mediation, and explicit non-authority.

Where future organizational-envelope machinery exists, editable controls should operate on authorized environment/configuration dimensions rather than rewriting skill implementations.

### Organization view

Show the problem-level execution envelope and each role projection:

- instantiated roles;
- ownership/delegation;
- capability selections;
- information flow;
- mutation boundaries;
- configuration provenance;
- baseline-versus-effective differences.

### Runtime view

Show provider/runtime realization:

- binding;
- loaded/active state;
- current turn;
- runtime descendants;
- capability availability;
- waiting conditions.

### Control view

Expose only controls authorized by the current semantic/control-plane projection.

### Forensics view

Replay:

- envelope revisions;
- workflow transitions;
- decisions;
- findings;
- evidence changes;
- runtime events;
- checkpoints and receipts.

## Design-time diagnostics

Possible candidate findings include:

- missing obligation coverage;
- authority leaks;
- observation gaps;
- ownership gaps;
- independence conflicts;
- redundant/orphan machinery;
- unreachable required consequences;
- declared-vs-realized enforcement mismatch.

Mechanical diagnostics remain candidate evidence where semantic judgment is required.

## Boundary

Studio is never an alternate source of truth.

It renders and edits through canonical owners and authority-controlled transitions.

## Does not own

Studio does not own:

- role contracts;
- execution-envelope semantics;
- control-plane authority;
- runtime execution;
- workflow state;
- proposal decisions.

## Compact statement

> Use one structural model to understand what may exist, what is active now, what the user may control, and what happened historically.
