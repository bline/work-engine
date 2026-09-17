# Incremental terminal accounting projection

Status: pending post-migration proposal

```yaml
idea_status:
  architectural_supersession: none
  architectural_supersession_note: "Corrected 2026-09-17, per review: checked on content grounds, not citation absence. 'Ownership shape' names ProviderTurnPort/HarnessRuntimePort/OperatorProjection consistent with runtime-realization.md's own port fields (those specific concepts ARE now stated there, an argument for partial supersession of that narrow slice) -- but grep-confirmed zero occurrences of 'accounting projector' or 'terminal accounting' anywhere in app-server/docs/architecture, so this document's actual proposal (an incrementally-maintained accounting projection) is not stated by canonical architecture. Kept at none rather than partial because the port vocabulary alone is not this document's own architectural contribution -- see persistent-provider-runtime-managers-for-codex-and-claude.md for the case where a document's own core claim, not just shared vocabulary, is now stated elsewhere."
  residue: none
  backlog: present
  backlog_ledger: "'Migration sketch' (5 numbered steps) tagged [PLAN: OPEN] on direct evidence -- zero real code or campaigns found."
  audit_scope:
    - staged-plan-section
    - keyword-scan: full_document
    - close-read: "full document (71 lines)"
  audit_scope_completeness: complete
  status_as_of: 2026-09-16
```

```yaml
idea_provenance:
  origin: direct_capture
```

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

**[PLAN: OPEN — 2026-09-16. No durable execution evidence found within the surfaces checked: `app-server/src` (grepped for "accounting-event store", "accounting projector", "terminal accounting" — zero hits), and Work Engine campaign/worktree state under `/home/bline/.local/state/work-engine` (no workstream named for this plan).]**

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
