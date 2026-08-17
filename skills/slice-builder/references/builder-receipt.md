# Engineering builder receipt contract

Return one concise object after a slice is accepted, stopped, or failed. Do not include prompts, transcripts, chain-of-thought, raw Claude output, source excerpts, diffs, patches, raw test output, debug logs, or secrets.

## Required outcome fields

- `slice_statement`
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
- `configured_validation_requirements`: array copied from the effective config
- `validation_requirement_results`: object mapping every requirement to `passed`, `failed`, `blocked`, or `not_applicable`, with a concise reason for `not_applicable`

Checks should contain command identity, pass/fail state, and test totals when available—not transcripts.

## Required builder metrics

Use null for unavailable measurements:

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

Map the configured provider's measurements into these semantic fields when correspondence is direct. Preserve original provider-specific measurements under `additional_metrics` without speculatively renaming them. Measurements must come from tool or platform receipts; never estimate them.
