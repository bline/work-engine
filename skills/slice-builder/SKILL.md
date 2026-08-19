---
name: slice-builder
description: Launch and control one capable Codex engineering worker that owns an entire configured evidence-driven slice from reconnaissance and planning through implementation, validation, fixes, and final metrics. Use as the default engineering builder adapter for slice-supervisor campaigns, including feature, cleanup, reliability, and performance work.
---

# Slice Builder

Act as the engineering adapter for one coherent slice. Do not decompose the slice among small workers and do not use `implementation-orchestrator`. The builder owns repository understanding, architectural judgment, implementation, targeted checks, the configured evidence/review gate, valid fixes, and the final receipt.

The caller remains the supervisor. It owns the campaign configuration, plan acceptance, limits, durable metrics, continuation, and human escalation; it must not inspect or implement repository work.

Read the effective work-engine configuration. This adapter supports engineering campaigns whose validation requirements can be mapped to repository checks, freshness checks, visual inspection, and adversarial review. Before planning, explicitly reject unsupported validation requirements or a non-engineering objective. Never silently downgrade a gate. Normalize provider metrics by semantic role while preserving provider-native measurements in `additional_metrics`.

Keep invariants, acceptance conditions, routes, and recovery decisions distinct. Invariants and explicit configuration remain binding. Select an evidence and validation route proportionate to the slice instead of treating every available stage as mandatory. When evidence invalidates a premise, preserve applicable observations, mark dependent decisions stale, revise the route, and return for renewed plan acceptance. Do not turn a recoverable route correction into either silent scope expansion or automatic failure.

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
- relevant prior accepted handoff receipts, not audit receipts or transcripts;
- hard limits and approval boundaries;
- the current phase contract; and
- configured builder context, including `evidence_skill` and
  `reconnaissance.provider` when present.

Before invoking repository evidence, resolve the builder context with
`scripts/resolve_provider.py`. The default provider is claude-codebase-memory,
backed by $claude-recon-implementation. Treat Codebase Memory as the primary
evidence capability and use $codebase-memory to understand and apply the
capabilities it currently exposes. Select the evidence route dynamically from
the objective, repository state, available graph capabilities, coverage, and
observed uncertainty. Supplement graph evidence with targeted repository
evidence when needed to establish trustworthy claims; do not treat current tool
limitations or today's preferred query sequence as permanent workflow law.

claude-filesystem remains an explicitly selectable filesystem-first provider.
Provider identity and evidence-skill identity remain separate provenance and
must resolve consistently; record the resolved provider as `evidence_provider`.
Do not silently substitute providers or weaken configured acceptance
requirements. Other recognized providers remain unavailable until supported;
auto provider selection is deferred.

Require the worker to use the resolved evidence skill faithfully for stages selected by the accepted route. Before planning, confirm that it supports every stage the configured validation profile or current risk requires. A `direct` route may use the builder's own read-only repository observation when no explicit configuration or risk condition requires an independent provider. If a defaulted provider is unavailable, record the failed attempt and continue directly only when the same acceptance condition can still be met; an explicitly selected provider or independence requirement remains binding. Otherwise attempt a route revision only when it does not weaken configuration, independence, or provenance, or stop with `unsupported_capability`. Ordinary test execution belongs to this builder, not the evidence skill.

Never describe the builder as a subtask worker. State that it owns the whole slice and must return a boundary-change request instead of silently expanding scope.

## Continue one identity through the slice

Retain the builder returned by `spawn_agent`. Use `followup_task` on that identity for every normal phase so its durable architectural context survives.

### Planning turn

Require read-only evidence and an evidence-based plan only. First assess consequence, reversibility, placement ambiguity, repository familiarity, and the number of ownership/runtime boundaries involved. Select and record one route:

- `direct`: for an obvious, local, reversible boundary with a known producer, consumer, and focused downstream proof. Gather only targeted evidence needed to confirm the semantic path, using builder-direct observation unless independent evidence is explicitly configured or risk-justified.
- `falsified-placement`: for competing homes, cross-boundary state, consequential behavior, weak repository familiarity, or medium/high placement risk. Obtain shallow alternatives, make an explicit Codex placement decision, then use a fresh provider call that assumes the selected boundary may be wrong.

Escalate from `direct` to `falsified-placement` as soon as evidence reveals a plausible competing owner, hidden consumer, lifecycle conflict, or broader consequence. Do not deeply explore every alternative.

Require:

- bounded slice statement and premise conflicts;
- observed facts separated from inference;
- semantic outcome independent of implementation shape;
- placement risk, selected route, and supporting evidence;
- a provisional placement certificate naming trigger, producer, state owner, consumer, lifecycle, observable consequence, and downstream proof;
- candidates, discriminating evidence, and rejected alternatives when plausible alternatives exist;
- a placement verdict of `confirmed`; on `conflict`, revise the premise and route while the objective and authority remain stable; on `unresolved`, make at most one narrow discriminating request before stopping for a real decision or unavailable evidence;
- invariants and ownership/provenance requirements;
- expected changed-file boundary and baseline overlaps;
- acceptance checks, a vertical semantic test, and exact future validation commands proportionate to the claims;
- mapping from every configured validation requirement to a concrete gate;
- deferred scope, open decisions, and missing-context risk; and
- available reconnaissance statistics.

### Implementation turn

After explicit `procedural_auto_approval` or `human_approval`, send the accepted slice and placement certificate verbatim. Require the vertical semantic proof before broad implementation or presentation polish. If the selected owner or consumer cannot support it, return a boundary-change request instead of implementing a locally coherent substitute. Then require implementation, relevant documentation, and inexpensive configured checks, followed by a stop immediately before the final gate. Require the task-owned file manifest, baseline overlaps, vertical and targeted results, unresolved concerns, and gate readiness.

### Gate turn

Authorize the configured validation profile. Build an explicit ordered manifest and run it with `scripts/run_gate.py`. The script is the canonical owner of deterministic execution, fail-fast behavior, and compact gate results; pass command arguments as arrays and never interpolate a shell command.

For `engineering-proportional`, always include the vertical semantic proof, changed-file/workspace integrity, and focused tests. Add freshness checks when generated or derived artifacts may be affected. Add broader regression suites when the change crosses shared/runtime boundaries, has broad fan-out, changes persistence/schema/build behavior, or focused checks cannot bound the risk. Require fresh independent adversarial review for medium/high risk, consequential user-visible or runtime behavior, security/persistence/ownership changes, or material uncertainty. Record why omitted stages were not needed; omission is a scoped judgment, not evidence that a stage passed.

Keep the configured profile identity unchanged. A high-risk proportional run may select the same breadth as `engineering-full`, but record that as `validation_breadth`, not as a profile change.

For `engineering-full`, run vertical proof and changed-file boundary first, then `git diff --check`, applicable prechecks/freshness, focused tests, the full suite, and fresh adversarial review. Explicit configured requirements override profile defaults and must not be waived.

```bash
python3 scripts/run_gate.py --manifest-json \
  '{"checks":[{"requirement":"focused_checks","identity":"focused","command":["python3","-m","unittest"]}]}'
```

If a deterministic check fails, diagnose locally or use the configured evidence skill's compact failure-diagnosis path when independence or context isolation adds value, then fix and rerun affected checks. Perform fresh adversarial review when configured or warranted by the profile, evaluate findings, and implement valid in-scope fixes. Finish with the checks needed to prove the final state; repeat the full suite only when configured or when fixes changed its risk surface.

Return both receipt views defined in [references/builder-receipt.md](references/builder-receipt.md) and [references/handoff-receipt.md](references/handoff-receipt.md). Do not return raw transcripts, diffs, source excerpts, or test logs.

The supervisor may authorize implementation through gate in one follow-up only when the effective config sets `approval.uninterrupted_after_plan: true`. Plan acceptance and separate phase accounting remain mandatory.

## Escalate reasoning only from evidence

An existing worker's model configuration cannot be changed. If the configured effort fails for a reasoning-related cause, stop that worker and spawn one replacement with the same model and the next effort level. Pass only the accepted slice, baseline, task-owned manifest, concise phase receipts, exact unresolved issue, relevant metrics, and unchanged configuration.

Escalation requires at least one concrete signal:

- bounded evidence leaves architectural reasoning unresolved;
- placement alternatives remain tied after one narrow discriminating request;
- a revised route still cannot reconcile targeted evidence with the placement certificate;
- a plan remains internally inconsistent after one narrow discriminating request;
- the same valid blocking review category survives two repair attempts;
- the builder twice violates the accepted boundary or cannot produce the required receipt; or
- checks pass but the builder cannot reconcile a verified output/provenance contradiction.

Do not escalate for infrastructure failure, quota exhaustion, a missing user decision, ordinary check failure, elapsed time, or token use alone. Handle those through retry, stop, or intervention.

Escalate one level at a time: `low → medium → high`. Stop for human judgment before `xhigh` unless configuration or the user explicitly authorizes it. Record replaced workers, efforts, evidence, and reasons. Never run old and replacement builders concurrently on the same mutable workspace.

## Enforce the boundary

If new necessary scope appears, stop mutation and return a boundary-change request. The supervisor returns to planning and re-accepts the revised boundary when the objective, authority, and consequential decisions remain stable; ask the user only when the change requires their judgment or expands authority.

If unrelated or unattributable workspace changes appear, stop mutation until ownership is established. A replacement inherits the exact baseline and task-owned manifest. Preserve the distinction between pre-existing, task-owned, overlapping, and unowned changes.

## Collect the final receipt

Read [references/builder-receipt.md](references/builder-receipt.md) and [references/handoff-receipt.md](references/handoff-receipt.md) before requesting the gate result. Return `audit_receipt` for durable validation and metrics, plus `handoff_receipt` for the next builder. Preserve audit measurements exactly and use `null` when unavailable. Record configured and actual model, effort, evidence skill, validation profile, requirement results, `workflow_route`, `route_revisions`, and evidence-based validation breadth even if no escalation occurred. Each route revision names the failed premise, stale decisions, preserved evidence, replacement route, and reason. Never copy engine, provider, model, token, cache, route mechanics, or detailed gate bookkeeping into the handoff.

The builder is complete only when every configured blocking gate passes with no blocking findings, or when it returns a truthful stopped/failed receipt. Completion of edits alone is not success.
