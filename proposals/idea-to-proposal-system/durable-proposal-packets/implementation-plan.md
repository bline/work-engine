# Implementation Plan: Durable Proposal Packets

This is a proposed delivery shape, not an acceptance requirement. Steps may be
combined or revised while preserving the stated consequences.

## Slice 1: Contract and fixtures

Define the minimal manifest and Markdown ownership boundaries, lifecycle and
authority semantics, relationship vocabulary, lineage rules, and role
projections. Create representative fixtures for formed, placement-uncertain,
split, superseded, and decided proposals.

Evidence of completion:

- stable IDs survive path/title changes;
- family origin is referenced rather than copied;
- required and optional artifacts are mechanically distinguishable;
- fixtures expose uncertainty without pretending field completion is certainty.

## Slice 2: Deterministic validation

Add a validator for closed, mechanically decidable properties only: schema
version, required identity fields, unique IDs, resolvable local references,
valid relationship shapes, lineage consistency, and authority metadata on
decision transitions.

Semantic quality, placement correctness, and proposal value remain model/human
judgments and must not be reduced to field-presence validation.

## Slice 3: Selective projections

Define and demonstrate compact views for proposal formation, evidence
evaluation, strategic planning, campaign supervision, and implementation. A
projection may summarize canonical state but cannot become its sole durable
owner.

## Slice 4: Dogfood and revise

Represent this family and at least one unrelated repository idea using the new
contract. Record friction, redundant fields, missing identity transitions, and
context-loading cost. Revise the contract before integrating it into roadmap or
runtime consumers.

## Validation

- schema and reference tests;
- split/merge/supersession lifecycle tests;
- round-trip projection tests proving canonical state is not lost;
- manual semantic review against `DESIGN.md` command and authority rules;
- a fresh consumer exercise in which the strategic planner uses only its
  projection and named on-demand evidence.
