# Proposal-Backed Portfolio and Roadmap Selection

## Status

Reconciled against current app-server implementation and prospective architecture in
`app-server/docs/proposal-backed-portfolio-selection-reconciliation.md` — **accepted and
its residue authorized 2026-09-14; see that document for the authorization record.** This
root idea file is retained as historical subject matter; the reconciliation document is the
living artifact. Proposal
identity, relationships (`depends_on`/`enables`/`informs`/`related_to`/`split_from`),
and proposal decisions are already real. "A strategic planner" refers to a
campaign-scoped mechanism (`strategic-planning-handoff.mjs`) whose structured handoff
schema (see `post-migration-strategic-plan.md`'s "Strategic planning handoff" block) is
real precedent for strategic-assumption content, not evidence it is missing.
Confirmed `SUPPLIES` relationships from `ideas/evidence-backed-proposal-evaluation.md`
(item 6, evaluated value/risk) and `ideas/proposal-research-maturity-and-freshness.md`
(item 5, readiness). The primary remaining open scope is a `PortfolioDecision` record:
a revision-bound basis (exact strategic assumptions, proposal/evaluation/readiness
revisions) plus attributed cross-proposal analysis (portfolio-relative unlock/enablement;
portfolio-relative mutual exclusion under this basis, kept separate from any durable
`conflicts_with` proposal relationship) and a priority/sequencing/exclusion disposition
that references proposal packet identity without mutating the proposal's own lifecycle
(portfolio exclusion is not proposal rejection). See the reconciliation document for the
full disposition.

Exploratory planning/portfolio idea.

## Idea

Make accepted or decision-ready proposal packets the durable subjects of roadmap and portfolio judgment rather than copying their meaning into ad hoc roadmap prose.

The roadmap becomes an index and prioritization surface over proposal identity, relationships, decisions, and current strategic consequences.

## Current evidence

Work Engine already has:

- typed proposal relationships;
- proposal decisions;
- a strategic planner;
- a hand-authored roadmap with current ownership.

The roadmap does not yet operate as a proposal-backed portfolio projection.

## Required consequence

Portfolio judgment can reason over:

- accepted proposal identity and revision;
- dependency and enablement relationships;
- blocking or conflicting proposals;
- unlock value;
- evaluated value/risk evidence;
- current strategic assumptions;
- implementation/activation readiness;
- explicit human priority decisions.

A roadmap entry should reference proposal truth rather than become a competing copy of it.

## Authority

The portfolio/roadmap owner decides:

- priority;
- sequencing;
- deferral;
- rejection/removal from active portfolio;
- campaign selection.

Proposal relationships and evaluator findings inform those decisions but do not mechanically schedule work.

## Does not own

This idea does not own:

- proposal formation;
- proposal evaluation;
- research freshness;
- execution envelopes;
- campaign supervision;
- closed-loop calibration.

## Compact statement

> The roadmap should organize and prioritize proposal identities, not become another place where proposal meaning is rewritten.
