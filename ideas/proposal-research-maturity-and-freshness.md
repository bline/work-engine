# Proposal Research Maturity, Readiness, and Freshness

## Status

Reconciled against current app-server implementation and prospective architecture in
`app-server/docs/proposal-research-maturity-and-freshness-reconciliation.md` — **accepted
and its residue authorized 2026-09-14; see that document for the authorization record.**
This root idea file is retained as historical subject matter; the reconciliation document
is the living artifact. The
freshness/refresh mechanism and its ownership are retired to `claim-evidence-service.md`
(`app-server/src/services/claim-evidence` already implements a real `proposal-research-v1`
vertical profile) — freshness remains a referenced input to readiness, not something this
idea owns. The R0-R5 maturity ladder is downgraded from a canonical lifecycle to an
optional, coarse derived summary: each stage decomposes into an existing or separately
queued owner (proposal formation, placement, `ideas/evidence-backed-proposal-evaluation.md`'s
evaluation dimensions, `ideas/organizational-execution-envelopes.md`'s envelope consumption,
authority/runtime admission). The primary remaining open scope is the decision-specific
readiness contract and attributed assessment semantics: an attributed sufficiency judgment for a named decision
contract (e.g. "sufficient for placement" vs. "blocked for activation, missing X") that
explicitly does not acquire the decision/acceptance/activation authority it informs.
`ideas/evidence-backed-proposal-evaluation.md` is recorded as a likely evidence supplier to
that judgment, not a competing owner. See the reconciliation document for the full
disposition.

Exploratory research architecture. A narrower shared claim-centered evidence-lineage candidate has already been promoted to `proposals/evidence-lineage/`; this idea should consume that candidate or an equivalent primitive rather than redefine it.

## Idea

Proposal research should accumulate in reusable layers and express **which consequential decisions the current evidence is mature enough to support**.

Research maturity is not procedural completion.

## Current evidence

Proposal packets and proposal formation exist. Mechanical packet validation explicitly does not prove evidence sufficiency, freshness, value, placement, or execution readiness.

The evidence-lineage proposal family now explores shared claim identity, provenance, sensitivity, and lineage semantics.

## Cumulative maturity

Useful epistemic states include:

- **R0 Captured** — preserve the observation/idea without pretending it is a coherent proposal.
- **R1 Formed** — coherent objective, consequences, boundaries, alternatives, likely owner, uncertainty.
- **R2 Situated** — repository/architectural placement, competing owners, affected contracts, dependencies, direct evidence.
- **R3 Characterized** — enough evidence for comparison/portfolio judgment: value, complexity, risk, reversibility, maintenance, validation burden, fan-out.
- **R4 Organization-qualified** — enough evidence to propose organizational requirements: capabilities, ownership, independence, information flow, authority, collaboration, context lifetime.
- **R5 Activation-ready** — the durable understanding plus current authority/configuration/runtime facts are sufficient to begin concrete work.

These are labels for supported understanding, not mandatory stages.

## Readiness is decision-specific

A proposal can be:

```text
placement-ready
portfolio-ready
organization-blocked
activation-blocked
```

at the same time.

Consumers should request the readiness profile needed for their decision rather than infer readiness from one universal score.

## Freshness

Research needs an evidence baseline and explicit sensitivity/reopening surfaces.

Relevant change can include:

- changed files or symbols;
- new implementations;
- changed callers/consumers;
- contract or capability changes;
- related proposal completion;
- runtime/provider assumption expiry;
- new alternatives.

Mechanical analysis may mark a claim **candidate-stale**. Attributed judgment determines whether the conclusion remains current, needs revision, is stale, refreshed, or superseded.

## Refresh

Refresh should reuse prior durable conclusions and gather new evidence only for dimensions plausibly affected by change.

## Does not own

This idea does not define:

- the shared claim-lineage schema;
- proposal evaluation;
- organizational compilation;
- portfolio decisions;
- continuous monitoring infrastructure.

It defines research maturity/readiness/freshness semantics that those consumers may use.

## Compact statement

> Research maturity describes what is currently supported; readiness describes which next decision that support can justify; freshness describes whether the supporting evidence still deserves reliance.
