---
name: slice-builder
description: Launch and control one capable Codex engineering worker that owns an entire configured evidence-driven slice from reconnaissance and planning through implementation, validation, fixes, and final metrics. Use as the default engineering builder adapter for slice-supervisor campaigns, including feature, cleanup, reliability, and performance work.
---

# Slice Builder

Act as the engineering adapter for one coherent slice. Do not decompose the slice among small workers and do not use `implementation-orchestrator`. The builder owns repository understanding, architectural judgment, implementation, targeted checks, the configured evidence/review gate, valid fixes, and the final receipt.

The caller remains the supervisor. It owns the campaign configuration, plan acceptance, limits, durable metrics, continuation, and human escalation; it must not inspect or implement repository work.

Read the effective work-engine configuration and [references/decision-policy.md](references/decision-policy.md) completely. This adapter supports engineering campaigns whose validation requirements can be mapped to repository checks, freshness checks, visual inspection, and adversarial review. Before planning, explicitly reject unsupported validation requirements or a non-engineering objective. Never silently downgrade a gate. Normalize provider metrics by semantic role while preserving provider-native measurements in `additional_metrics`.

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
- configured builder context, including versioned repository-evidence and
  independent-review roles.

Before invoking repository evidence, resolve the config version and builder
context with `scripts/resolve_provider.py`. Config version 2 defaults repository
retrieval to `codex-codebase-memory`, backed by $repo-search, and resolves
independent review separately through $claude-recon-implementation. Config
version 1 retains its historical combined Claude evidence/review meaning; never
mix or silently reinterpret the two shapes.

Use $repo-search for the durable `orient`, `find`, `trace`, `impact`, and `audit`
intents. Treat Codebase Memory as the current primary indexed-structure
capability and use $codebase-memory to learn and apply the interface it currently
exposes; do not copy individual operation names or today's preferred query
sequence into this durable role contract.
Select indexed structural evidence or targeted direct source observation for
each claim from the objective, repository state, available capability classes,
coverage, and observed uncertainty. Prefer indexed evidence for relationships
the current interface can establish. Use direct source observation for literal
content, runtime or presentation details, stale or incomplete coverage,
unresolved graph ambiguity, or claims the current index cannot establish.
Record the actual evidence mode and fallback reason. Current interface limits
are observed constraints, not permanent workflow law.

`claude-codebase-memory` and `claude-filesystem` remain explicitly selectable
retrieval providers backed by $claude-recon-implementation. Provider and skill
identity must resolve consistently. Record repository retrieval and independent
review as separate provenance. Do not silently substitute providers, count
retrieval as review, or weaken configured acceptance requirements. Auto provider
selection is deferred.

Version 2 may instead explicitly configure `adversarial_review` through
$codex-adversarial-review when a human has accepted same-model review. Launch it
with `fork_turns: "none"`, its configured model and reasoning effort, and no
builder-session history. Record it as `accepted_same_model_review`, including
observed isolation and model relationship; never describe it as independent,
cross-model, cross-provider, or statistically independent. Exactly one of
`independent_review` and `adversarial_review` may be configured, and accepted
same-model review cannot satisfy a requirement that explicitly demands
independent review.

Do not confuse provider selection with evidence capability. A configured Claude
role may expose Codebase Memory and bounded read-only filesystem tools in the
same process. Keep indexed structure primary for repository code understanding,
and use direct filesystem observation only for exact non-code content, reported
coverage gaps, generated or unindexed artifacts, or required host state outside
the index. That evidence-mode change does not rewrite provider identity or
require a configuration amendment. Record the actual mode and transition reason;
mere tool availability is not a fallback event. Selecting `claude-filesystem` as
the repository provider remains a distinct explicit configuration choice.

Require the worker to use each resolved skill faithfully for the stages selected by the accepted route. Before planning, confirm that repository retrieval supports the needed evidence and that the independent-review role supports every configured or risk-required independence stage. A `direct` route may use the builder's own read-only repository observation when no explicit configuration or risk condition requires independent evidence. If a defaulted provider is unavailable, record the failed attempt and continue directly only when the same acceptance condition can still be met; an explicitly selected provider or independence requirement remains binding. Retrieval success never satisfies a required independent review. Ordinary test execution belongs to this builder, not either evidence role.

When a placement or review clause predictably depends on an external or
unindexed artifact, require the configured Claude call to provision bounded
read-only filesystem access from its first attempt while retaining indexed
access for repository structure. A reviewer must observe the material clause in
its own fresh process; a retrieval packet alone does not create independence.

Never describe the builder as a subtask worker. State that it owns the whole slice and must return a boundary-change request instead of silently expanding scope.

## Continue one identity through the slice

Retain the builder returned by `spawn_agent`. Use `followup_task` on that identity for every normal phase so its durable architectural context survives.

Apply the corresponding lifetime rule to the configured reviewer: launch one
fresh isolated reviewer at the start of a review obligation, then retain that
reviewer through ordinary remediation while its context remains useful. For a
Claude reviewer, use the configured persistent session ID and resume it. For a
Codex same-model reviewer, retain the spawned reviewer identity and use
`followup_task`. A continuation examines the exact delta and prior findings and
must not be reported as another fresh review. Reset only from a recorded
judgment that the premise, scope, architecture, independence need, or context
fitness materially changed; provider or caller convenience is not a reset
reason.

When active-slice recovery is configured and this builder launches the
reviewer, assign the provider's resumable session ID before the initial review
call and publish it through the active-slice mechanism as
`actor_binding.runtime_session_id`. After builder or supervisor context
replacement, recover the binding with `resume_active_slice.py` before issuing
the next reviewer call; resume that exact session rather than reconstructing
the review. If the binding cannot be recovered or the provider session is no
longer available, preserve the pending obligation and applicable findings,
record the replacement reason and provenance, and label the replacement as a
new reviewer rather than a continuation.

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

### Gate turn

Authorize the configured validation profile. Build an explicit ordered manifest and run it with `scripts/run_gate.py`. The script is the canonical owner of deterministic execution, fail-fast behavior, and compact gate results; pass command arguments as arrays and never interpolate a shell command.

For `engineering-proportional`, always include the vertical semantic proof, changed-file/workspace integrity, and focused tests. Add freshness checks when generated or derived artifacts may be affected. Add broader regression suites when the change crosses shared/runtime boundaries, has broad fan-out, changes persistence/schema/build behavior, or focused checks cannot bound the risk. Require fresh independent adversarial review for medium/high risk, consequential user-visible or runtime behavior, security/persistence/ownership changes, or material uncertainty. Record why omitted stages were not needed; omission is a scoped judgment, not evidence that a stage passed.

Keep the configured profile identity unchanged. A high-risk proportional run may select the same breadth as `engineering-full`, but record that as `validation_breadth`, not as a profile change.

For `engineering-full`, run vertical proof and changed-file boundary first, then `git diff --check`, applicable prechecks/freshness, focused tests, the full suite, and fresh adversarial review. Explicit configured requirements override profile defaults and must not be waived.

```bash
python3 scripts/run_gate.py --manifest-json \
  '{"checks":[{"requirement":"focused_checks","identity":"focused","command":["python3","-m","unittest"]}]}'
```

If a deterministic check fails, diagnose locally or use the configured evidence
skill's compact failure-diagnosis path when independence or context isolation
adds value, then fix and rerun affected checks. Start adversarial review fresh
once when configured or warranted by the profile. Evaluate findings, implement
valid in-scope fixes, and send each resulting delta back to the same reviewer
identity. Finish with the checks needed to prove the final state; repeat the
full suite only when configured or when fixes changed its risk surface. Launch
a replacement reviewer only after a recorded reset judgment or an unavailable
retained identity, and preserve that provenance in the receipt.

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

Escalate one level at a time: `low → medium → high`. Stop for human judgment before `xhigh` unless configuration or the user explicitly authorizes it. Record replaced workers, efforts, evidence, and reasons. Never run old and replacement builders concurrently on the same mutable workspace.

## Enforce the boundary

If new necessary scope appears, stop mutation and return a boundary-change request. The supervisor returns to planning and re-accepts the revised boundary when the objective, authority, and consequential decisions remain stable; ask the user only when the change requires their judgment or expands authority.

If unrelated or unattributable workspace changes appear, stop mutation until ownership is established. A replacement inherits the exact baseline and task-owned manifest. Preserve the distinction between pre-existing, task-owned, overlapping, and unowned changes.

## Collect the final receipt

When the supervisor returns an accepted checkpoint under an enabled
`engine_config.slice_completion_commit.prompt`, retain this builder while its
context remains useful and ask it to compose one compact schema-version-2
`commit_proposal`. Bind its one-line subject, body, and exact task-owned path
set to the accepted checkpoint commit, tree, and task-patch digest. Include
structured production provenance naming the producer and the durable evidence
that supports the proposal; provider or model identity is descriptive, never
authority. Return the proposal as a separate completion artifact beside the
receipts. It conveys semantic context; it is not part of the audit receipt and
never authorizes a Git mutation.

If this context is lost or no longer trustworthy, a replacement may reconstruct
the proposal only from the accepted checkpoint, attributed manifest, and
durable compact semantic consequences. It records that route truthfully in the
same open provenance structure. When those artifacts do not support an accurate
subject or body, refuse the proposal or surface the uncertainty; do not infer
semantic equivalence or replay a transcript merely to satisfy the schema.

Read [references/builder-receipt.md](references/builder-receipt.md) and [references/handoff-receipt.md](references/handoff-receipt.md) before requesting the gate result. Return `audit_receipt` for durable validation and metrics, plus `handoff_receipt` for the next builder. Preserve audit measurements exactly and use `null` when unavailable. Record configured and actual model, effort, evidence skill, validation profile, requirement results, `workflow_route`, `route_revisions`, and evidence-based validation breadth even if no escalation occurred. Each route revision names the failed premise, stale decisions, preserved evidence, replacement route, and reason. Never copy engine, provider, model, token, cache, route mechanics, or detailed gate bookkeeping into the handoff.

The builder is complete only when every configured blocking gate passes with no blocking findings, or when it returns a truthful stopped/failed receipt. Completion of edits alone is not success.
