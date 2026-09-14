# Cross-Cutting Seam Review

## Status

**Accepted 2026-09-14** (explicit user decision after reviewing this reconciliation): the
reconciliation's findings are confirmed accurate. Split authorization: implementation of the
mechanical seam-evidence adapter extensions (implementation-evidence-driven, per the
existing extensible `EvidenceAnchorObserver` taxonomy) is authorized to proceed. The semantic
"truthfully and proportionately" correspondence-judgment owner and the attributed
routing/disposition concept are authorized for **design work only, not implementation yet**
— both require an owner decision this reconciliation deliberately left open.

Reconciled jointly with `ideas/architectural-review.md` against current app-server
implementation and prospective architecture in
`app-server/docs/cross-cutting-seam-review-and-architectural-review-reconciliation.md`
(per the `COUPLED_RECONCILIATION` flag in `app-server/docs/root-ideas-reconciliation.md`).
Deterministic observation/comparison of an already-declared seam dependency is retired to
`app-server/ideas/pending/evidence-anchor-observation-and-impact-nomination.md`'s
`EvidenceAnchorObserver` pattern — narrower than retiring `may_affect` itself, which
requires a known dependent claim/fact and is not the mismatch itself. The remaining open
scope: implementation-evidence-driven seam-evidence adapters and, only where needed,
anchor-kind extensions (not a predeclared list); the owner of the semantic "truthfully and
proportionately" correspondence judgment for non-mechanically-decidable seams; and an
attributed routing/disposition concept (not a fixed local/architectural/documentation/UI/
workflow taxonomy, which would be semantic and multidimensional) that can flag
architectural diagnosis as warranted. See the reconciliation document for the full
disposition.

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
