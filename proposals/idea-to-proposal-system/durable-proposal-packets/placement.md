# Placement: Durable Proposal Packets

## Proposed owner

Work Engine's planning layer owns the packet contract.

This is inside the Work Engine product boundary but upstream of campaign
execution. Work Engine already owns objective, authority, information-lifetime,
provenance, and receipt contracts. A proposal packet applies those same product
semantics to the transition from candidate change to bounded objective.

## Repository placement

A top-level `proposals/` surface is the likely human-authored home. A future
schema and validator should live with the planning capability that owns the
contract rather than under `slice-supervisor`, `metrics`, or runtime-state
scripts.

## Neighboring owners and consumers

- `skills/strategic-planner/` consumes proposal meaning, relationships,
  decisions, and observed deltas when reconciling roadmap direction.
- `roadmap.md` may reference accepted or selected proposal IDs, but does not own
  packet content.
- `skills/slice-supervisor/` consumes an authorized bounded objective or an
  implementation projection; it does not form or accept proposals.
- persistent runtime state may carry `proposal_ref` during execution, but it
  does not define proposal identity or semantic content.
- a future evidence-evaluation capability enriches packets under the packet
  contract without becoming their identity owner.

## Competing placements

### Outside Work Engine as a separate proposal product

Credible in the future, but unsupported now. The repository already contains a
strategic planning role and explicitly places proposal formation in its product
roadmap. Extracting a new product boundary before the semantics and consumers
are proven would add ownership and installation complexity without establishing
a distinct external audience.

### Persistent runtime-state subsystem

Rejected as primary owner. Runtime state answers how active work survives and
reconciles execution transitions. Proposal packets answer what candidate change
is being considered and why. They may share identifiers without sharing a
canonical store.

### Roadmap

Rejected as primary owner. A roadmap is a selected portfolio view. A proposal
must survive rejection, deferral, supersession, or refresh without being copied
into or erased from that view.

## Evidence and confidence

Placement confidence: probable.

Direct repository evidence:

- `DESIGN.md` assigns Work Engine ownership of authority, information lifetime,
  provenance, and objective contracts.
- `roadmap.md` explicitly places proposal formation and strategic planning in a
  later Work Engine workstream.
- `skills/strategic-planner/SKILL.md` already names proposal packets as durable
  strategic evidence.
- persistent-state ideas reserve `proposal_ref`/`proposal_id` as references but
  do not define proposal semantics.

## Reopening conditions

Reopen placement if an independently deployable consumer outside engineering
campaign planning becomes primary, or if the packet requires transactional
runtime semantics that cannot be cleanly referenced across the planning/runtime
boundary.
