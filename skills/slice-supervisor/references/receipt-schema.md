# Work-engine receipt schema

Append exactly one `audit_receipt` record when a slice reaches `accepted`, `stopped`, or `failed`. The builder's compact `handoff_receipt` is non-durable context and must never be appended here. Use schema version 4 for new campaigns. Version 4 preserves semantic-path placement evidence from version 3 and adds required evidence-mode, provider-failure, and fallback provenance. The append script continues to accept historical version-1 through version-3 records, but new runs must not emit them.

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

`engine_config` must contain `version`, `source`, `objective`, `work_source`, `builder`, `validation`, `metrics`, `limits`, `approval`, `notifications`, `stop_on`, `explicit_fields`, `defaulted_fields`, and `amendments`. Config version 1 preserves the historical combined evidence/review builder context. Config version 2 uses separate `repository_evidence` and `independent_review` provider/skill objects. The shapes must not be mixed. The last four provenance/condition collections are arrays where appropriate. Preserve explicit and defaulted ownership; do not flatten them into an indistinguishable effective value.

Each validation result is `passed`, `failed`, `blocked`, or `not_applicable`. `not_applicable` requires a concise reason and is never a silent waiver. Every configured requirement must be `passed` for an accepted slice; use another terminal status when a requirement cannot apply.

Any receipt whose plan was accepted requires a nonempty placement certificate, `confirmed` verdict, placement risk, and vertical semantic test. A terminal `accepted` receipt additionally requires `vertical_semantic_test_passed: true`. Preserve a late semantic rejection as a stopped or failed outcome; never rewrite the earlier placement verdict as though it had not occurred.

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

New config-version-2 receipts also contain `repository_evidence_identity` and
`independent_review_identity`, each with the provider and skill actually used.
They also contain `provider_role_metrics`, partitioning provider attempt
outcomes and available measurements between those two roles; role outcome
totals must equal the generic provider outcome totals. Historical
config-version-1 receipts remain valid without these additive fields. If role
identities are present, report both; version-1 identities must both preserve the
same valid combined legacy role. Version-2 receipts include semantic
`evidence_recon_calls`, `evidence_supplemental_calls`, and `review_gate_calls`.
Retrieval stage calls cannot exceed repository-provider attempts, and review
gate calls cannot exceed independent-review-provider attempts.
Actual role identities must match the normalized effective configuration. A
different provider requires a recorded configuration amendment; evidence-mode
fallback inside the configured role does not change provider identity.

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
