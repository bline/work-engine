# Cross-Cutting Seam Review

## Status

Exploratory review capability.

## Idea

Review whether independently reasonable components remain coherent **at their seams**.

Local correctness does not establish that boundaries between architecture, implementation, documentation, UI, state, provenance, and design principles still compose well.

## Typical seams

Examples include:

- principle ↔ implementation;
- documentation ↔ implementation;
- state ownership ↔ UI representation;
- authority ↔ exposed control;
- provenance ↔ displayed certainty;
- capability contract ↔ provider realization;
- proposal expectation ↔ implementation consequence.

## Required consequence

A seam review can identify:

- which boundary is under review;
- which independent contracts/principles meet there;
- evidence from each side;
- mismatch or disproportion;
- consequence of the mismatch;
- whether the issue is local, architectural, documentation, UI, or workflow scope;
- confidence and limitations.

## Distinction from architectural review

Architectural review asks whether the system model or ownership is wrong.

Seam review asks whether **two otherwise valid parts correspond truthfully and proportionately**.

A seam finding may trigger architectural review, but the capabilities are not identical.

## Distinction from UI review

UI review focuses specifically on whether the human interface is a good representation/control surface for underlying machinery.

Seam review is broader and may have no human-facing surface at all.

## Invocation

Seam review should be consequence-selected. It is not a mandatory fixed gate for every slice.

## Compact statement

> Components establish local correctness; seam review asks whether the relationships between them still preserve the design system as a whole.
