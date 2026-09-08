# Incremental terminal accounting projection

Status: pending post-migration proposal

## Problem

Terminal receipt assembly currently reconstructs provider, evidence, fallback,
and review accounting after a slice has finished. Long-lived or recovered
campaigns can contain valid durable events without a durable rule that maps
every historical event into the current receipt taxonomy. Reconstructing those
counts at terminalization invites either fabricated zeroes or repeated forensic
work by expensive agents.

The schema-v5 `unavailable_metrics` compatibility path is an emergency escape
hatch for preserved legacy executions. It is not the target architecture.

## Direction

Make terminal accounting an incrementally maintained projection of admitted
runtime events. Each semantic owner records the accounting consequence when it
admits an event, including pre-provider failure, provider entry, transport
outcome, correction, retained-session retry, fallback, repository-evidence
stage, and review-gate use. A deterministic projector folds those events into a
revision-bound receipt projection throughout the campaign.

Terminalization should then bind and validate the current projection rather
than rediscover history. Missing instrumentation remains an explicit
availability state with provenance; it never becomes an inferred zero.

## Ownership shape

- ProviderTurnPort owns provider-entry and provider-outcome observations.
- HarnessRuntimePort owns process, transport, retry, and recovery observations.
- Campaign and review services own the semantic classification of admitted
  events and their exact subject or operation identity.
- The accounting projector owns deterministic aggregation and internal
  consistency, but no workflow decision or acceptance authority.
- OperatorProjection exposes availability, gaps, and terminal readiness without
  becoming a second state owner.

Preserve ProviderTurnPort, HarnessRuntimePort, and OperatorProjection as
architectural seams. Do not encode receipt accounting directly into one
provider adapter or UI transport.

## Required properties

- Event identities and projection revisions are idempotent and CAS-bound.
- Counts distinguish not-entered, entered, transport-terminal, contract-rejected,
  corrected, and recovered outcomes before aggregation.
- Provider-role, evidence-mode, failure-reason, and fallback projections derive
  from the same admitted events and cannot drift independently.
- Partial or unavailable measurements carry durable provenance and a named
  instrumentation gap.
- Recovery and continuation packets carry the projection identity, not a
  model-authored summary of accounting history.
- Historical receipts remain readable; compatibility declarations cannot be
  used for newly instrumented work.

## Migration sketch

1. Define the provider/harness event vocabulary and semantic classification
   owner for every currently ambiguous S13 event class.
2. Add an append-only accounting-event store and deterministic projector.
3. Emit events from the existing provider, reviewer-runtime, repository-evidence,
   retry, and fallback boundaries.
4. Compare incremental projections with existing terminal receipts in shadow
   mode.
5. Make projection completeness a terminal-readiness condition, retaining the
   legacy unavailable declaration only for executions that predate coverage.

This proposal does not authorize implementation before migration completion.
