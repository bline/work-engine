# Engineering builder audit receipt contract

Return this object as `audit_receipt` after a slice is accepted, stopped, or failed. It is the complete terminal record consumed by the supervisor's durable schema validator. Return the separate compact `handoff_receipt` defined in [handoff-receipt.md](handoff-receipt.md) beside it. Do not include prompts, transcripts, chain-of-thought, raw Claude output, source excerpts, diffs, patches, raw test output, debug logs, or secrets.

## Required outcome fields

- `slice_title`
- `slice_goal`
- `status`: `accepted`, `stopped`, or `failed`
- `outcome`
- `stop_reason`: null only when accepted
- `task_owned_files`
- `baseline_overlaps`
- `targeted_checks`
- `gate_checks`
- `review_findings`: counts by severity and blocking status
- `review_fix_iterations`
- `unresolved_concerns`
- `deferred_scope`
- `more_in_scope_work_remains`: true, false, or null
- `placement_certificate`: concise object containing trigger, producer, state owner, consumer, lifecycle, semantic consequence, downstream proof, and insufficient substitute
- `placement_verdict`: `confirmed`, `conflict`, or `unresolved`
- `placement_risk`: `low`, `medium`, or `high`
- `rejected_placement_alternatives`: array of concise candidate/evidence objects
- `vertical_semantic_test`: concise command or structured proof identity
- `vertical_semantic_test_passed`: true, false, or null
- `configured_validation_requirements`: array copied from the effective config
- `validation_requirement_results`: object mapping every requirement to `passed`, `failed`, `blocked`, or `not_applicable`, with a concise reason for `not_applicable`

Checks should contain command identity, pass/fail state, and test totals when available—not transcripts.

## Required builder metrics

Use null for unavailable measurements:

- `workflow_route`: `direct` or `falsified-placement`
- `route_revisions`: array of objects naming the failed premise, stale decisions, preserved evidence, replacement route, and reason
- `validation_breadth`: an object containing nonempty `selected_stages`,
  `omitted_optional_stages` objects with a reason for each omission, and a
  concise nonempty `rationale` grounded in consequence, reversibility,
  uncertainty, changed boundaries, and repository instructions
- `builder_model`
- `initial_reasoning_effort`
- `final_reasoning_effort`
- `reasoning_escalation_count`
- `reasoning_escalations`: array of `{from, to, reason, phase}`
- `builder_input_tokens`
- `builder_output_tokens`
- `builder_context_usage`
- `builder_wall_clock_seconds`
- `builder_turn_count`
- `replacement_builder_count`
- `repository_exploration_outside_evidence_packets`

## Required adapter identity

- `builder_skill`
- `configured_builder_model`
- `configured_reasoning_effort`
- `configured_evidence_skill`
- `validation_profile`

These configured values are not proof of what ran. Keep them beside the actual model, effort, call, and gate measurements above so configuration and observation remain distinguishable.

## Required evidence and review workflow metrics

- `evidence_provider`
- `evidence_recon_calls`
- `evidence_supplemental_calls`
- `review_gate_calls`
- `provider_successful_calls`
- `provider_failed_calls`
- `provider_timed_out_calls`
- `provider_infrastructure_failed_calls`
- `provider_wall_clock_seconds`
- `provider_cost_usd`
- `provider_cache_creation_tokens`
- `provider_cache_read_tokens`
- `provider_output_tokens`
- `provider_thinking_tokens`
- `retrieved_files`
- `retrieved_ranges`
- `retrieved_source_lines`
- `missing_context_blocked_implementation`
- `placement_calls`
- `placement_candidate_count`
- `placement_conflicts`
- `placement_reconsiderations`
- `targeted_reconnaissance_calls`
- `late_semantic_rejections`
- `evidence_mode_metrics`: per-mode attempt outcomes and available token, cost,
  and time measurements using stable capability-class names such as
  `indexed_structure`, `direct_source`, and `builder_direct`
- `provider_failure_reasons`: mutually exclusive primary-cause counts for
  `network`, `timeout`, `permission`, `protocol`, `quota`, and `other`
- `fallback_reason_counts`: counts for `index_unavailable`, `coverage_gap`,
  `graph_ambiguity`, and `provider_failure`
- `fallbacks`: compact route-transition objects containing `from_mode`,
  `to_mode`, `stage`, `reason`, and `failure_kind`

`evidence_recon_calls` is the total of initial non-supplemental evidence calls; report `placement_calls` and `targeted_reconnaissance_calls` as its stage-specific breakdown. Map the configured provider's measurements into these semantic fields when correspondence is direct. Preserve original provider-specific measurements under `additional_metrics` without speculatively renaming them. Measurements must come from tool or platform receipts; never estimate them.

For schema version 4, provider call outcomes are mutually exclusive. Classify
every unsuccessful call under exactly one of `failed`, `timed_out`, or
`infrastructure_failed`, and give it exactly one primary cause in
`provider_failure_reasons`. A timeout therefore contributes to `timed_out` and
the `timeout` reason, not also to the generic failed count.

Each evidence-mode object contains `attempts`, `successful`, `failed`,
`timed_out`, `infrastructure_failed`, and, when available, `input_tokens`,
`cache_creation_tokens`, `cache_read_tokens`, `output_tokens`,
`thinking_tokens`, `cost_usd`, and `wall_clock_seconds`. Counts are nonnegative
integers; unavailable measurements are null. Outcome counts must sum to
`attempts`. Use capability classes rather than provider product names so a new
interface can supply the same evidence mode without changing the durable
schema. Provider and model identity remain separate fields.

Fallback reasons record why the evidence route changed, not merely which tools
ran. A healthy direct-source check caused by incomplete index coverage is
`coverage_gap`; infrastructure overhead is `provider_failure`. For a provider
failure, `failure_kind` is one of the primary-cause names above. It is null for
all other fallback reasons. `fallback_reason_counts` must equal the events in
`fallbacks`; do not increment several primary causes for one attempt.

The principal efficiency comparison is total tokens, cost, and elapsed time to
an accepted decision, partitioned by evidence mode, workflow route, placement
risk, failure cause, and fallback reason. Interpret it beside placement
conflicts, late semantic rejections, regressions, and review repairs; lower cost
alone is not acceptance evidence.

Route fields preserve the real decision process. Do not report an omitted optional stage as passed, and do not treat a route revision as a failure when the revised plan was renewed and accepted. Configured requirements still require explicit passing results.
