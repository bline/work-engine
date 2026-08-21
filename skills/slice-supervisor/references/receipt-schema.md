# Work-engine receipt schema

Append exactly one `audit_receipt` record when a slice reaches `accepted`, `stopped`, or `failed`. The durable identity is the pair of `run_id` and `slice_number`; while holding the append lock, the append CLI rejects any repeated identity without changing the destination, whether the receipt content is identical or conflicting. The builder's compact `handoff_receipt` is non-durable context and must never be appended here. Use schema version 4 for new campaigns. Version 4 preserves semantic-path placement evidence from version 3 and adds required evidence-mode, provider-failure, and fallback provenance. The compatibility validator continues to accept historical version-1 through version-3 records for explicit read or migration use. The append CLI is the current artifact producer and rejects those historical versions before changing the durable destination.

## Required common fields

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | integer | Use `4` for new configured campaigns. |
| `run_id` | string | Nonempty identifier shared by every slice in one campaign. |
| `slice_number` | integer | Positive, sequential within the run. |
| `timestamp` | string | ISO 8601 timestamp with timezone. |
| `slice_title` | string | Concise human-readable title. |
| `slice_goal` | string | Accepted or proposed bounded slice statement. |
| `status` | string | `accepted`, `stopped`, or `failed`. |
| `outcome` | string | Concise truthful result. |
| `stop_reason` | string or null | Null only for accepted slices. |
| `plan_acceptance` | string | `procedural_auto_approval`, `human_approval`, or `not_reached`. |
| `engine_config` | object | Effective configuration and provenance described below. |
| `builder_skill` | string | Configured builder adapter. |
| `validation_profile` | string | Configured validation profile. |
| `validation_requirement_results` | object | Result for every configured requirement. |
| `more_in_scope_work_remains` | boolean or null | Builder's evidence-based continuation report. |
| `placement_certificate` | object or null | Confirmed producer/owner/consumer contract; null only when planning did not reach placement. |
| `placement_verdict` | string or null | `confirmed`, `conflict`, `unresolved`, or null before placement. |
| `placement_risk` | string or null | `low`, `medium`, `high`, or null before placement. |
| `rejected_placement_alternatives` | array | Concise rejected candidates and evidence. |
| `vertical_semantic_test` | object, string, or null | Downstream proof selected during planning. |
| `vertical_semantic_test_passed` | boolean or null | Observed proof result. Must be true for acceptance. |
| `worker_metrics` | object | Flexible namespaced metrics reported by the worker. |
| `producer_metrics` | object | Flexible namespaced metrics reported by an orchestrator or producer. |

`engine_config` must contain `version`, `source`, `objective`, `work_source`, `builder`, `validation`, `metrics`, `limits`, `approval`, `notifications`, `stop_on`, `explicit_fields`, `defaulted_fields`, and `amendments`. It may contain preflighted `capabilities`. Chrome Vision capability provenance is identity-only: source kind, authored reference or campaign path, canonical resolved path/base, SHA-256, and schema version. Do not persist the expanded runtime config or infer actual use from availability. Config version 1 preserves the historical combined evidence/review builder context. Config version 2 uses `repository_evidence` plus exactly one review role: the provider/skill `independent_review` object or the explicitly authorized provenance-bearing `adversarial_review` object. The shapes must not be mixed. The last four provenance/condition collections are arrays where appropriate. Preserve explicit and defaulted ownership; do not flatten them into an indistinguishable effective value.

Each validation result is `passed`, `failed`, `blocked`, or `not_applicable`. `not_applicable` requires a concise reason and is never a silent waiver. Every configured requirement must be `passed` for an accepted slice; use another terminal status when a requirement cannot apply.

Any receipt whose plan was accepted requires a nonempty placement certificate, `confirmed` verdict, placement risk, and vertical semantic test. A terminal `accepted` receipt additionally requires `vertical_semantic_test_passed: true`. Preserve a late semantic rejection as a stopped or failed outcome; never rewrite the earlier placement verdict as though it had not occurred.

## Telemetry ingress boundary

[`telemetry-ingress.schema.json`](telemetry-ingress.schema.json) owns the
canonical boundary between authoritative host telemetry and later audit-receipt
assembly. The supervisor invokes the deterministic harvester only after it can
identify the launched builder from the parent's `SubAgentActivity` `started`
event. The parent-side `agent_thread_id` is the primary identifier; the child
rollout must independently corroborate its own ID, parent thread, subagent
source, and canonical agent path before any measurement is accepted.

Each measurement records `observed` or `unavailable`, its value, and exact event
provenance. Missing fields remain unavailable rather than becoming inferred
zeroes. Timestamp proximity, agent path alone, builder self-report, and a child
artifact that is ambiguous, mismatched, malformed, or nonterminal are not valid
telemetry ingress. Tool/provider telemetry remains an explicit ingress array;
merging ingress into a final audit receipt belongs to the later receipt-assembly
lifecycle, not to the harvester.

A corroborated child rollout may contain an earlier started turn with no
completion when a later unique turn completes and is the rollout's final event.
The harvester retains each such turn under
`binding_provenance.interrupted_turns` with its start event and explicitly
unavailable completion provenance; it never synthesizes a completion.
`turn_count` counts every unique started attempt, wall clock spans the first
start through the final latest completion, and token values remain the latest
cumulative values actually emitted rather than inferred per-turn allocations.
Duplicate identifiers, unknown completions, and an incomplete or nonterminal
latest turn remain invalid.

The deterministic receipt assembler accepts one valid schema-version-4
semantic audit receipt, one matching telemetry-ingress artifact, and the
original successful named-campaign preflight result. It requires the run and
slice identities to agree, replaces supported builder-runtime measurements
with authoritative observed values or `null` for authoritative unavailability,
and replaces the semantic receipt's model-authored `engine_config` with
preflight's complete initial effective `engineConfig`. It preserves
builder-runtime identity, artifact, event, and measurement provenance under
`producer_metrics.telemetry_ingress`, while the separate canonical campaign
path and digest are retained under `producer_metrics.campaign_source`. It does
not persist expanded `resolvedCapabilities` or reinterpret arbitrary
`tool_runtime` entries. The assembler fails closed when authoritative ingress
is already present or campaign-source identity is malformed, passes its result
through the existing audit-receipt validator, does not append the receipt, and
does not change the harvester's identity rules. Use:

```bash
python3 skills/slice-supervisor/scripts/assemble_receipt.py \
  --semantic-receipt-json '<schema-v4-audit-receipt>' \
  --telemetry-ingress-json '<telemetry-ingress-v1>' \
  --campaign-preflight-json '<successful-named-campaign-preflight-result>'
```

For production named-campaign terminalization, compose assembly and append in
one process so the authoritative projection cannot be skipped and no mutable
intermediate receipt is introduced:

```bash
python3 skills/slice-supervisor/scripts/finalize_receipt.py \
  --path '<configured-metrics-path>' \
  --semantic-receipt-json '<schema-v4-audit-receipt>' \
  --telemetry-ingress-json '<telemetry-ingress-v1>' \
  --campaign-preflight-json '<original-successful-preflight-result>'
```

The finalizer calls the existing assembler and passes its exact in-memory
result to the existing append boundary. Assembly failure writes nothing;
append retains current-write validation, exclusive terminal identity under
lock, and file plus parent-directory durability. The command does not reread
the campaign file. It accepts the compact handoff separately and may accept an
authorized accepted/stopped slice-checkpoint receipt. The finalizer projects
that receipt under `producer_metrics.slice_checkpoint`; the append validator
binds its run, slice, lifecycle kind, object identities, private ref, plan,
scope, gate, and task-patch identity to the terminal receipt. For an
accepted slice, the finalizer may also project a schema-version-1
`producer_metrics.slice_completion_commit` lifecycle receipt. A new proposal
uses schema version 2 with structured, open production provenance containing a
producer description and durable evidence references; historical proposal
schema version 1 remains readable with its original completing-builder origin.
Neither producer nor provider/model identity grants authority. Every proposal
binds the accepted checkpoint commit, tree, and task-patch digest. `created`
records the verified ordinary commit;
Historical `pending` projections remain readable, but new pending interaction
is supervisor-owned live state and cannot enter a terminal write. `declined`
and `refused` preserve truthful resolved non-commit outcomes.
Before append, the finalizer obtains the adapter's read-only verification of a
`created` receipt's repository, proposal, commit, parent, tree, message,
publication target, and resulting state. Shape-valid input alone is not
authoritative evidence.
None replaces the private checkpoint as resume authority. For an accepted
slice with `more_in_scope_work_remains: true`, it derives this top-level
projection before the single append:

```json
{"continuation_context":{"schema_version":1,"durable_decisions":[],"affected_boundaries":[],"unresolved_concerns":[],"deferred_scope":[]}}
```

The projection is not the durable handoff artifact. Current named-campaign
resumable writes require the exact projection. Accepted records with no
remaining work and stopped or failed records omit it. Historical compatibility
reads permit its absence.

Inter-slice recovery uses:

```bash
python3 skills/slice-supervisor/scripts/resume_campaign.py \
  --path '<configured-metrics-path>' \
  --campaign-preflight-json '<fresh-successful-preflight-result>' \
  --run-id '<stable-run-id>'
```

The reader validates a unique, ordered, contiguous selected-run history and
deep-compares both authoritative engine configuration and campaign-source
identity. Trusted terminal outcomes return structured resumable or
non-resumable results. A stopped or failed result includes an additive
`terminal_state` projection containing the receipt-owned `status` and exact
`stop_reason`; its stable coarse `reason` remains `stopped` or `failed` and the
result remains non-resumable. Accepted-complete and continuation-unavailable
results omit `terminal_state` because accepted receipts have no stop reason.
When an accepted record contains `producer_metrics.slice_checkpoint`, the
reader verifies that its private ref still names the recorded commit and that
the commit names the recorded tree. A resumable result returns that projection
as `baseline_checkpoint`; current branch HEAD is not a substitute.
Malformed history, gaps, duplicates, missing runs, or binding mismatches fail
the command. It writes nothing. This contract does not define mid-slice
recovery, partial-artifact recovery, amendments, migration, or slice
reservation.

When the accepted checkpoint repository contains the supervisor-owned private
completion-offer ref for the last slice, resume validates and returns it
additively as `completion_offer`. Open or resolved offer state never changes
`resumable`, `next_slice_number`, or `baseline_checkpoint`.

## Recommended normalized engineering fields

Include when available, using `null` otherwise:

- `changed_file_count`
- `test_totals`
- `review_findings` and `review_fix_iterations`
- evidence/review provider call, failure, time, cost, token, and retrieval measurements
- placement calls, candidate counts, conflicts, reconsiderations, targeted-reconnaissance calls, placement risk, vertical-proof status, and late semantic rejections
- engineering input/output/context measurements
- selected workflow route, route revisions, preserved evidence, stale decisions, and validation-breadth rationale
- configured and actual builder model and reasoning effort
- reasoning escalation and replacement counts
- repository exploration outside configured evidence packets
- slice wall-clock time
- `anomalies`, `deferred_scope`, and `unresolved_concerns`

## Required version-4 evidence provenance

`worker_metrics` must contain:

- `workflow_route`, `route_revisions`, and `validation_breadth`, preserving the
  route decision, recovery history, selected validation stages, explicitly
  omitted optional stages and reasons, and a concise risk-based rationale;
- `workflow_route` and every route revision's `replacement_route` are nonempty
  string identities. `direct` and `falsified-placement` remain named defaults,
  but validators do not treat them as an exhaustive taxonomy or canonicalize
  an authored identity;
- `evidence_mode_metrics`, partitioning attempt outcomes and available token,
  cost, and wall-clock measurements by capability class;
- mutually exclusive `provider_successful_calls`, `provider_failed_calls`,
  `provider_timed_out_calls`, and `provider_infrastructure_failed_calls` counts;
- `provider_failure_reasons`, containing nonnegative counts for `network`,
  `timeout`, `permission`, `protocol`, `quota`, and `other`;
- `fallback_reason_counts`, containing nonnegative counts for
  `index_unavailable`, `coverage_gap`, `graph_ambiguity`, and
  `provider_failure`; and
- `fallbacks`, the compact transition events from which fallback reason counts
  are derived.

New config-version-2 receipts contain `repository_evidence_identity` and the
identity matching the configured review role. Legacy `independent_review_identity`
contains provider and skill. `adversarial_review_identity` contains provider,
skill, model, reasoning effort, evidence class, isolation, observed builder
context inheritance, model relationship, and whether independence was claimed.
They also contain `provider_role_metrics`, partitioning provider attempt
outcomes and available measurements between those two roles; role outcome
totals must equal the generic provider outcome totals. Historical
config-version-1 receipts remain valid without these additive fields. If role
identities are present, report both; version-1 identities must both preserve the
same valid combined legacy role. Version-2 receipts include semantic
`evidence_recon_calls`, `evidence_supplemental_calls`, and `review_gate_calls`.
Retrieval stage calls cannot exceed repository-provider attempts, and review
gate calls cannot exceed configured-review-provider attempts.
Actual role identities must match the normalized effective configuration. A
different provider requires a recorded configuration amendment; evidence-mode
fallback inside the configured role does not change provider identity.

An `accepted_same_model_review` identity requires fresh-process isolation,
`builder_context_inherited: false`, `model_relationship: same_model`, and
`independence_claimed: false`. It must not be represented as cross-provider,
cross-model, statistically independent, or independent reasoning, and cannot
satisfy a configured requirement explicitly demanding independent review.

Capability classes describe how evidence was obtained, not which product
supplied it. Provider and model identity remain separate provenance. A fallback
event records `from_mode`, `to_mode`, `stage`, `reason`, and `failure_kind`.
`failure_kind` is required only for `provider_failure` and must otherwise be
null. Counts must remain internally consistent. Use null for unavailable
measurements; never use null when the observed count is zero.

Version 4 treats successful, failed, timed-out, and infrastructure-failed
provider-call counts as mutually exclusive outcomes. Every non-successful call
has exactly one primary cause in `provider_failure_reasons`; the cause counts
must equal the failed, timed-out, and infrastructure-failed call total.

The validator checks that route and validation-breadth records are present and
internally consistent. It does not replace the supervisor's procedural judgment
about whether the selected breadth is adequate for the recorded consequence,
uncertainty, reversibility, and placement risk.

Non-engineering builders may add different normalized fields inside
`worker_metrics`, but version-4 receipts still provide the common evidence
provenance fields above, using empty objects, zero counts, and an empty fallback
list when no repository-evidence attempt occurred. Preserve unknown producer
keys instead of promoting them speculatively.

## Truth and safety rules

Measurements are numbers or null, never estimates disguised as facts. Preserve configured values separately from observed execution values. A builder report that no work remains is evidence, not by itself proof that the objective is complete.

Do not include raw transcripts, prompts, chain-of-thought, source excerpts, diffs, patches, raw validation output, detailed debug logs, or secrets. Receipts are operational records, not execution archives.

Compare anomalies only against recent comparable accepted records. A value above roughly twice a recent median is a review signal, not proof of failure. Do not manufacture a baseline when history is sparse.
