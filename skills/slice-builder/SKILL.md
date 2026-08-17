---
name: slice-builder
description: Launch and control one capable Codex engineering worker that owns an entire configured evidence-driven slice from reconnaissance and planning through implementation, validation, fixes, and final metrics. Use as the default engineering builder adapter for slice-supervisor campaigns, including feature, cleanup, reliability, and performance work.
---

# Slice Builder

Act as the engineering adapter for one coherent slice. Do not decompose the slice among small workers and do not use `implementation-orchestrator`. The builder owns repository understanding, architectural judgment, implementation, targeted checks, the configured evidence/review gate, valid fixes, and the final receipt.

The caller remains the supervisor. It owns the campaign configuration, plan acceptance, limits, durable metrics, continuation, and human escalation; it must not inspect or implement repository work.

Read the effective work-engine configuration. This adapter supports engineering campaigns whose validation requirements can be mapped to repository checks, freshness checks, visual inspection, and adversarial review. Before planning, explicitly reject unsupported validation requirements or a non-engineering objective. Never silently downgrade a gate. Normalize provider metrics by semantic role while preserving provider-native measurements in `additional_metrics`.

## Launch the builder

Spawn with:

- `fork_turns: "none"` to exclude unrelated supervisor history;
- the configured `builder.model` and `builder.reasoning_effort`;
- a task name unique to the run and slice.

When omitted, the work-engine defaults are `gpt-5.6-sol` and `low`. Low effort is the measured default for this workflow; a configured evidence skill bounds repository evidence and reduces the engineering problem space. Do not raise effort preemptively or because a slice is merely large, slow, or expensive.

Give the builder only:

- the exact campaign objective and work source;
- the effective configuration and its provenance;
- repository instructions and working directory;
- relevant prior accepted receipts, not transcripts;
- hard limits and approval boundaries;
- the current phase contract; and
- configured builder context, including `evidence_skill` when present.

Require the worker to use the configured evidence skill faithfully. The default is `$claude-recon-implementation`. If the configured skill is unavailable or does not provide the required reconnaissance and gate capabilities, stop with `unsupported_capability`; do not substitute a different tool without approval.

Never describe the builder as a subtask worker. State that it owns the whole slice and must return a boundary-change request instead of silently expanding scope.

## Continue one identity through the slice

Retain the builder returned by `spawn_agent`. Use `followup_task` on that identity for every normal phase so its durable architectural context survives.

### Planning turn

Require read-only reconnaissance and an evidence-based plan only. Require:

- bounded slice statement and premise conflicts;
- observed facts separated from inference;
- invariants and ownership/provenance requirements;
- expected changed-file boundary and baseline overlaps;
- acceptance checks and exact future validation commands;
- mapping from every configured validation requirement to a concrete gate;
- deferred scope, open decisions, and missing-context risk; and
- available reconnaissance statistics.

### Implementation turn

After explicit `procedural_auto_approval` or `human_approval`, send the accepted slice verbatim. Require implementation, relevant documentation, and inexpensive configured checks, then a stop immediately before the final gate. Require the task-owned file manifest, baseline overlaps, targeted results, unresolved concerns, and gate readiness.

### Gate turn

Authorize the complete configured test/review loop. Run the mapped gates in the evidence skill's required order. Evaluate findings, implement valid in-scope fixes, and repeat until accepted or genuinely blocked. Return the final receipt in [references/builder-receipt.md](references/builder-receipt.md). Do not return raw transcripts, diffs, source excerpts, or test logs.

The supervisor may authorize implementation through gate in one follow-up only when the effective config sets `approval.uninterrupted_after_plan: true`. Plan acceptance and separate phase accounting remain mandatory.

## Escalate reasoning only from evidence

An existing worker's model configuration cannot be changed. If the configured effort fails for a reasoning-related cause, stop that worker and spawn one replacement with the same model and the next effort level. Pass only the accepted slice, baseline, task-owned manifest, concise phase receipts, exact unresolved issue, relevant metrics, and unchanged configuration.

Escalation requires at least one concrete signal:

- bounded evidence leaves architectural reasoning unresolved;
- a plan remains internally inconsistent after one narrow supplemental reconnaissance;
- the same valid blocking review category survives two repair attempts;
- the builder twice violates the accepted boundary or cannot produce the required receipt; or
- checks pass but the builder cannot reconcile a verified output/provenance contradiction.

Do not escalate for infrastructure failure, quota exhaustion, a missing user decision, ordinary check failure, elapsed time, or token use alone. Handle those through retry, stop, or intervention.

Escalate one level at a time: `low → medium → high`. Stop for human judgment before `xhigh` unless configuration or the user explicitly authorizes it. Record replaced workers, efforts, evidence, and reasons. Never run old and replacement builders concurrently on the same mutable workspace.

## Enforce the boundary

If new necessary scope appears, stop and return a boundary-change request. The supervisor must re-accept the revised boundary or ask the user.

If unrelated or unattributable workspace changes appear, stop mutation until ownership is established. A replacement inherits the exact baseline and task-owned manifest. Preserve the distinction between pre-existing, task-owned, overlapping, and unowned changes.

## Collect the final receipt

Read [references/builder-receipt.md](references/builder-receipt.md) before requesting the gate result. Preserve measurements exactly and use `null` when unavailable. Record configured and actual model, effort, evidence skill, validation profile, and requirement results even if no escalation occurred.

The builder is complete only when every configured blocking gate passes with no blocking findings, or when it returns a truthful stopped/failed receipt. Completion of edits alone is not success.
