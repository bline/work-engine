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
- `validation_breadth`: selected deterministic and independent-review stages plus reasons for optional omissions
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

`evidence_recon_calls` is the total of initial non-supplemental evidence calls; report `placement_calls` and `targeted_reconnaissance_calls` as its stage-specific breakdown. Map the configured provider's measurements into these semantic fields when correspondence is direct. Preserve original provider-specific measurements under `additional_metrics` without speculatively renaming them. Measurements must come from tool or platform receipts; never estimate them.

Route fields preserve the real decision process. Do not report an omitted optional stage as passed, and do not treat a route revision as a failure when the revised plan was renewed and accepted. Configured requirements still require explicit passing results.
