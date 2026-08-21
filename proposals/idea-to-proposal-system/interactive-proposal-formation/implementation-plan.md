# Implementation Plan: Interactive Proposal Formation

This plan assumes the minimal packet contract is available or co-developed.
It describes a credible route, not a mandatory ritual.

## Slice 1: Skill contract

Create `skills/proposal-former/` with explicit inputs, authority boundaries,
packet-writing ownership, completion consequence, and role-scoped output. Give
the model formation affordances and evidence capabilities without a fixed
question sequence.

## Slice 2: Packet operations

Provide deterministic helpers only where inputs fully determine the result:
initialize IDs, validate references, record lineage, and render compact
projections. Leave decomposition, placement, evidence sufficiency, and
completion judgment to the model.

## Slice 3: Scenario fixtures

Exercise at least these cases:

- one idea becomes one proposal;
- one idea splits into independently decidable proposals;
- apparent siblings merge after investigation;
- an idea duplicates existing machinery and yields no new proposal;
- new evidence reopens placement;
- a human decision changes scope or authority.

## Slice 4: Fresh-consumer validation

Give a fresh evaluator only the packet projection intended for evaluation. It
must be able to state the proposal, placement, boundaries, relationships,
uncertainties, and evidence needs without reading the formation transcript.

Give the strategic planner only its planning projection and named on-demand
references. Measure reconstruction requests, irrelevant context loaded, packet
updates, and semantic corrections; do not reduce success to token count alone.

## Slice 5: Integration decision

Use observed sessions to decide whether the capability should integrate with
the strategic planner, roadmap references, persistent agent state, or a richer
UI. Do not introduce those dependencies before evidence establishes their
decision value.
