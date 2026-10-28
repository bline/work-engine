---
name: slice-builder
description: Launch and control one capable Codex engineering worker that owns an entire configured evidence-driven slice from reconnaissance and planning through implementation, validation, fixes, and final metrics. Use as the default engineering builder adapter for slice-supervisor campaigns, including feature, cleanup, reliability, and performance work.
---

# Slice Builder

Act as the engineering adapter for one coherent slice. Do not decompose the
slice among small workers and do not use `implementation-orchestrator`. The
builder owns repository understanding, architectural judgment, implementation,
targeted and deterministic checks, execution of the supervisor-selected review
plan through the configured provider, valid fixes, and the final receipt. It
does not select or omit specialist reviewers for its own work.

The caller remains the supervisor. It owns the campaign configuration, plan
acceptance, specialist selection, limits, durable metrics, continuation, and
human escalation; it must not inspect or implement repository work.

Read the effective work-engine configuration,
[references/decision-policy.md](references/decision-policy.md), and the
[supervisor-owned review-selection contract](../slice-supervisor/references/review-selection.md)
completely. This adapter supports engineering campaigns whose validation
requirements can be mapped to repository checks, freshness checks, visual
inspection, and adversarial review. Before planning, explicitly reject
unsupported validation requirements or a non-engineering objective. Never
silently downgrade a gate. Normalize provider metrics by semantic role while
preserving provider-native measurements in `additional_metrics`.

Keep invariants, acceptance conditions, routes, and recovery decisions distinct.
Invariants and explicit configuration remain binding. Select an evidence and
deterministic-validation route proportionate to the slice instead of treating
every available stage as mandatory. Review selection belongs to the supervisor.
When evidence invalidates a premise, preserve applicable observations, mark
dependent decisions stale, revise the route, and return for renewed plan
acceptance. Do not turn a recoverable route correction into either silent scope
expansion or automatic failure.

## Launch the builder

Spawn with:

- `fork_turns: "none"` to exclude unrelated supervisor history;
- the configured `builder.model` and `builder.reasoning_effort`;
- a task name unique to the run and slice.

When omitted, the work-engine defaults are the builder default in [model selection](../../docs/model-selection.md): `gpt-6.1-terra` at `medium` as of 2026-10-27. These are current selection defaults, not evidence that a runtime used that model or effort. Preserve requested and observed identities separately, and do not substitute a model after a capacity failure without the owning authority.

Give the builder only:

- the exact campaign objective and work source;
- the effective configuration and its provenance;
- repository instructions and working directory;
- relevant prior accepted handoff receipts, not audit receipts or transcripts;
- hard limits and approval boundaries;
- the current phase contract; and
- configured builder context, including versioned repository-evidence and
  independent-review roles.

Before invoking repository evidence, resolve the config version and builder
context with `scripts/resolve_provider.py`. Config version 2 defaults repository
retrieval to `codex-codebase-memory`, backed by $repo-search, and resolves
independent review separately through $claude-recon-implementation. Config
version 1 retains its historical combined Claude evidence/review meaning; never
mix or silently reinterpret the two shapes.

Read [references/evidence-providers.md](references/evidence-providers.md) when
resolving or launching the repository-retrieval and review roles: Codebase Memory
and `$repo-search` use, `claude-codebase-memory` and `claude-filesystem`
providers, an explicit `adversarial_review`, evidence-mode fallback, and a
placement or review clause that depends on an external or unindexed artifact.

Require the worker to use each resolved skill faithfully for the evidence stages
in the accepted route and every specialist later selected by the supervisor.
Before planning, confirm that repository retrieval supports the needed evidence
and that the configured review role can execute selected semantic specialist
contracts without changing their finding ownership or evidence class. A
`direct` route may use the builder's own read-only repository observation when
no explicit configuration or risk condition requires independent evidence. If
a defaulted provider is unavailable, record the failed attempt and continue
directly only when the same acceptance condition can still be met; an explicitly
selected provider or independence requirement remains binding. Retrieval
success never satisfies a required review. Ordinary test execution belongs to
this builder, not either evidence role.

Never describe the builder as a subtask worker. State that it owns the whole slice and must return a boundary-change request instead of silently expanding scope.

## Continue one identity through the slice

Retain the builder returned by `spawn_agent`. Use `followup_task` on that identity for every normal phase so its durable architectural context survives.

Apply the corresponding lifetime rule to the configured reviewer: launch one
fresh isolated reviewer at the start of a review obligation, then retain that
reviewer through ordinary remediation while its context remains useful. For a
Claude reviewer, use the configured persistent session ID and resume it. For a
Codex reviewer, retain the spawned reviewer identity and use
`followup_task`. A continuation examines the exact delta and prior findings and
must not be reported as another fresh review. Reset only from a recorded
judgment that the premise, scope, architecture, independence need, or context
fitness materially changed; provider or caller convenience is not a reset
reason.

Read [references/retained-reviewer.md](references/retained-reviewer.md) when this
builder launches, binds, resumes or replaces the configured reviewer, or when
active-slice recovery is configured. It holds the review-episode state, the
bounded observational state, the `bind-actor` route and the usage lines for
`manage_active_slice.py` and `resume_active_slice.py`.

### Planning turn

Require read-only evidence and an evidence-based plan only. First assess consequence, reversibility, placement ambiguity, repository familiarity, and the number of ownership/runtime boundaries involved. Select and record a route identity. The named defaults are:

- `direct`: for an obvious, local, reversible boundary with a known producer, consumer, and focused downstream proof. Gather only targeted evidence needed to confirm the semantic path, using builder-direct observation unless independent evidence is explicitly configured or risk-justified.
- `falsified-placement`: for competing homes, cross-boundary state, consequential behavior, weak repository familiarity, or medium/high placement risk. Obtain shallow alternatives, make an explicit Codex placement decision, then use a fresh provider call that assumes the selected boundary may be wrong.

These defaults are not an exhaustive route taxonomy. A different nonempty
route identity may be used when it describes an authorized evidence and
validation route more truthfully; record the route's rationale and preserve its
authored identity in the audit receipt.

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

Also return the candidate-ready review-selection projection defined by the
supervisor's contract. Supply the exact attributed manifest and integrity
identities needed for the supervisor to bind an immutable candidate, then
identify artifact roles, changed semantic contracts, and whether materially
normative agent-facing text is `present`, `absent`, or `uncertain`. This is
subject evidence for the supervisor; do not select reviewers, declare a
specialist applicable, anticipate findings, or inspect unrelated files merely
to fill the projection.

Before returning that implementation result through the builder mailbox, publish
its compact phase consequence through the supervisor-owned active-slice
mechanism. The durable consequence names the accepted boundary and uses
integrity-bound references for implementation artifacts; it does not copy the
transcript, hidden reasoning, or raw test output. A successful transient return
is not a substitute for publication. If interruption makes completion
uncertain, preserve or publish that uncertainty and require recovery from the
durable projection instead of inferring success from runtime state.

Read [references/phase-publication.md](references/phase-publication.md) when
publishing an implementation or gate phase consequence: the identity object, the
revision recovery and the `publish-phase` command.

### Gate turn

Authorize the configured validation profile. Build an explicit ordered manifest and run it with `scripts/run_gate.py`. The script is the canonical owner of deterministic execution, fail-fast behavior, and compact gate results; pass command arguments as arrays and never interpolate a shell command.

After the gate result and any required review consequences are final but before
returning them through the builder mailbox, publish the compact gate-phase
consequence to the same active slice attempt. Bind established completion to
the gate/review artifacts by integrity identity. `run_gate.py` remains the
deterministic execution owner; its transient output alone is not durable
workflow publication.

For `engineering-proportional`, always include the vertical semantic proof,
changed-file/workspace integrity, and focused tests. Add freshness checks when
generated or derived artifacts may be affected. Add broader regression suites
when the change crosses shared/runtime boundaries, has broad fan-out, changes
persistence/schema/build behavior, or focused checks cannot bound the risk.
Project medium/high risk, consequential user-visible or runtime behavior,
security/persistence/ownership changes, and material uncertainty for supervisor
review selection; do not turn those observations into a builder-owned panel.
Record why omitted deterministic stages were not needed; omission is a scoped
judgment, not evidence that a stage passed.

Keep the configured profile identity unchanged. A high-risk proportional run may select the same breadth as `engineering-full`, but record that as `validation_breadth`, not as a profile change.

For `engineering-full`, run vertical proof and changed-file boundary first, then
`git diff --check`, applicable prechecks/freshness, focused tests, and the full
suite. Its configured fresh adversarial-review requirement is a binding input to
the supervisor's selection plan and must be executed after selection. Explicit
configured requirements override profile defaults and must not be waived.

```bash
python3 scripts/run_gate.py --manifest-json \
  '{"checks":[{"requirement":"focused_checks","identity":"focused","command":["python3","-m","unittest"]}]}'
```

If a deterministic check fails, diagnose locally or use the configured evidence
skill's compact failure-diagnosis path when independence or context isolation
adds value, then fix and rerun affected checks. After deterministic readiness,
stop until the supervisor returns an exact selection plan. Execute every
selected specialist through the configured review provider without adding,
omitting, replacing, or merging perspectives. A selected
`agent-instruction-review` uses its repository-local skill contract and finding
contract; provider identity does not absorb specialist identity. Evaluate
findings, implement valid in-scope fixes, and send each resulting delta back to
the same reviewer identity. Finish with the checks needed to prove the final
state; repeat the full suite only when configured or when fixes changed its risk
surface. Launch a replacement reviewer only after a recorded reset judgment or
an unavailable retained identity, and preserve that provenance in the receipt.

Return both receipt views defined in [references/builder-receipt.md](references/builder-receipt.md) and [references/handoff-receipt.md](references/handoff-receipt.md). Partition provider effort by evidence mode and preserve compact failure and fallback provenance as required by the audit contract. Do not return raw transcripts, diffs, source excerpts, or test logs.

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

Escalate effort and model as [model selection](../../docs/model-selection.md#escalation) describes. Record replaced workers, efforts, evidence, and reasons. Never run old and replacement builders concurrently on the same mutable workspace.

## Enforce the boundary

If new necessary scope appears, stop mutation and return a boundary-change request. The supervisor returns to planning and re-accepts the revised boundary when the objective, authority, and consequential decisions remain stable; ask the user only when the change requires their judgment or expands authority.

If unrelated or unattributable workspace changes appear, stop mutation until ownership is established. A replacement inherits the exact baseline and task-owned manifest. Preserve the distinction between pre-existing, task-owned, overlapping, and unowned changes.

## Collect the final receipt

Read [references/completion-proposal.md](references/completion-proposal.md) when
the supervisor returns an accepted checkpoint under an enabled
`engine_config.slice_completion_commit.prompt`, or when a replacement must
reconstruct the `commit_proposal`.

Read [references/builder-receipt.md](references/builder-receipt.md) and [references/handoff-receipt.md](references/handoff-receipt.md) before requesting the gate result. Return `audit_receipt` for durable validation and metrics, plus `handoff_receipt` for the next builder. Preserve audit measurements exactly and use `null` when unavailable. Record configured and actual model, effort, evidence skill, validation profile, requirement results, `workflow_route`, `route_revisions`, and evidence-based validation breadth even if no escalation occurred. Each route revision names the failed premise, stale decisions, preserved evidence, replacement route, and reason. Never copy engine, provider, model, token, cache, route mechanics, or detailed gate bookkeeping into the handoff.

The builder is complete only when every configured blocking gate passes with no blocking findings, or when it returns a truthful stopped/failed receipt. Completion of edits alone is not success.
