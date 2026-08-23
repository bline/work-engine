# Review Applicability and Mutation-Scope Coordination

## Status

Exploratory coordination idea. Review-artifact semantics are separately represented by the formed adaptive-specialized-review proposal family; Git-backed checkpoints are already implemented.

## Idea

Preserve the usefulness of independent review when repository state continues to evolve, review providers are intermittent, or later work may overlap the reviewed subject.

The unique problem here is **coordination between review applicability and mutable scope**, not review artifact identity itself.

## Current evidence

Work Engine already has:

- immutable private checkpoints;
- identity-bound review/gate evidence;
- retained reviewer sessions through remediation;
- nonterminal waiting for some capability failures;
- a scheduler prototype;
- formed proposals for revision-bound review artifacts.

What remains unresolved is how overlapping work should coordinate with an outstanding or completed review obligation.

## Required consequence

A review-dependent workflow can determine:

- the exact immutable subject being reviewed;
- the scope whose mutation may invalidate or complicate later applicability;
- whether planned work mechanically overlaps that scope;
- whether a semantic overlap exists despite no path overlap;
- whether work may continue, must wait, needs a new review subject, or needs explicit adjudication;
- whether a later result applies directly, composes with newer evidence, requires refresh, or is superseded.

## Coordination concepts

Possible machinery includes:

- durable review obligations;
- protected mutation scopes;
- overlap detection;
- reservations or claims;
- explicit supersession;
- applicability judgments;
- bounded conflict dispositions.

These are mechanisms, not mandatory universal procedure.

## Boundary from scheduling

The role scheduler owns time/delivery state for future obligations.

This idea owns domain-specific coordination between **review truth and evolving code state**. A review request may use scheduling machinery, but review applicability is not a scheduling fact.

## Boundary from review artifacts

A review artifact owns the original finding, subject, assumptions, limitations, and lineage.

This idea governs whether later mutable work may rely on or must refresh that artifact.

## Compact statement

> Preserve the original truth of a review while making later reliance on that review explicitly dependent on the code and assumptions it still applies to.
