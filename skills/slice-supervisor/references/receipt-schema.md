# Work-engine receipt schema

Append exactly one record when a slice reaches `accepted`, `stopped`, or `failed`. Use schema version 2 for configured campaigns. The append script continues to accept historical version-1 records but new runs must not emit them.

## Required common fields

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | integer | Use `2` for configured campaigns. |
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
| `worker_metrics` | object | Flexible namespaced metrics reported by the worker. |
| `producer_metrics` | object | Flexible namespaced metrics reported by an orchestrator or producer. |

`engine_config` must contain `version`, `source`, `objective`, `work_source`, `builder`, `validation`, `metrics`, `limits`, `approval`, `notifications`, `stop_on`, `explicit_fields`, `defaulted_fields`, and `amendments`. The last four provenance/condition collections are arrays where appropriate. Preserve explicit and defaulted ownership; do not flatten them into an indistinguishable effective value.

Each validation result is `passed`, `failed`, `blocked`, or `not_applicable`. `not_applicable` requires a concise reason and is never a silent waiver. Every configured requirement must be `passed` for an accepted slice; use another terminal status when a requirement cannot apply.

## Recommended normalized engineering fields

Include when available, using `null` otherwise:

- `changed_file_count`
- `test_totals`
- `review_findings` and `review_fix_iterations`
- evidence/review provider call, failure, time, cost, token, and retrieval measurements
- engineering input/output/context measurements
- configured and actual builder model and reasoning effort
- reasoning escalation and replacement counts
- repository exploration outside configured evidence packets
- slice wall-clock time
- `anomalies`, `deferred_scope`, and `unresolved_concerns`

Non-engineering builders may use different normalized fields inside `worker_metrics`. Preserve unknown producer keys there instead of promoting them speculatively.

## Truth and safety rules

Measurements are numbers or null, never estimates disguised as facts. Preserve configured values separately from observed execution values. A builder report that no work remains is evidence, not by itself proof that the objective is complete.

Do not include raw transcripts, prompts, chain-of-thought, source excerpts, diffs, patches, raw validation output, detailed debug logs, or secrets. Receipts are operational records, not execution archives.

Compare anomalies only against recent comparable accepted records. A value above roughly twice a recent median is a review signal, not proof of failure. Do not manufacture a baseline when history is sparse.
