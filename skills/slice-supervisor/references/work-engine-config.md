# Work engine configuration

Configuration supplies campaign context to the stable supervisor state machine. Use YAML or JSON with these exact fields. Reject unknown fields so misspellings do not silently change behavior.

## Version 1 shape

```yaml
version: 1
objective: "Advance the Site2JSON roadmap"

work_source:
  kind: file
  value: "docs/roadmap.md"

builder:
  skill: "slice-builder"
  model: "gpt-5.6-sol"
  reasoning_effort: "low"
  context:
    evidence_skill: "claude-recon-implementation"

validation:
  profile: "engineering-full"
  requirements:
    - focused_checks
    - full_suite
    - freshness_checks
    - adversarial_review

metrics:
  path: "docs/ai-workflow-metrics.jsonl"

limits: {}

approval:
  plan: "procedural_when_safe"
  uninterrupted_after_plan: false

notifications:
  intervention: true
  completion: false

stop_on:
  - objective_complete
  - human_judgment
  - unresolved_architecture
  - quota_exhaustion
  - validation_nonconvergence
```

## Field contract

- `version`: Required integer; use `1`.
- `objective`: Required nonempty statement. Preserve the user's wording.
- `work_source`: Optional evidence/boundary source. `kind` is `file`, `inline`, or `repository_evidence`; `value` is required for `file` and `inline` and omitted for `repository_evidence`. A missing work source means the objective plus applicable repository evidence, not an inferred roadmap file.
- `builder`: Optional only when all documented defaults apply. `skill` identifies an adapter satisfying the contract below. `model` and `reasoning_effort` are passed only if supported. `context` is a namespaced object owned by that builder; retain it verbatim in provenance but never place secrets in configuration.
- `validation`: `profile` names a builder-supported gate and `requirements` lists observable outcomes. The builder must confirm support before plan acceptance. Do not reinterpret an unknown requirement.
- `metrics.path`: Durable JSONL destination. An explicit `null` disables durable metrics only when the user says so; do not confuse it with a missing value.
- `limits`: Hard limits explicitly supplied by the user, such as `slices`, `time_seconds`, `cost_usd`, `tokens`, or `repair_attempts`. An empty object means no configured hard limits. Never invent them.
- `approval.plan`: `procedural_when_safe` or `human_required`. `approval.uninterrupted_after_plan` controls whether execution and gate may share a follow-up; it never removes plan acceptance.
- `notifications`: Requested Boolean policy for intervention and overall completion. Applicable repository instructions determine the actual command, may require intervention notification, and may prohibit completion notification; they take precedence over this request-level policy.
- `stop_on`: Named terminal conditions. The supervisor always stops for safety, unsupported capabilities, ownership ambiguity, or required human authority even if omitted. Unknown conditions require clarification.

## Documented defaults

Defaults preserve the existing engineering workflow:

- `builder.skill`: `slice-builder`
- `builder.model`: `gpt-5.6-sol`
- `builder.reasoning_effort`: `low`
- `builder.context.evidence_skill`: `claude-recon-implementation`
- `validation.profile`: `engineering-full`
- `validation.requirements`: `focused_checks`, `full_suite`, `freshness_checks`, `adversarial_review`
- `metrics.path`: `docs/ai-workflow-metrics.jsonl`
- `limits`: `{}`
- `approval.plan`: `procedural_when_safe`
- `approval.uninterrupted_after_plan`: `false`
- `notifications.intervention`: `true`
- `notifications.completion`: `false`
- `stop_on`: the list in the example above

Record which values were explicit and which came from these defaults. A user explicitly selecting the default value remains an explicit decision; classify provenance by source, not by value equality. Defaults are not user decisions.

## Builder adapter contract

A configured builder must declare before plan acceptance that it can:

1. keep planning read-only and return observed evidence separately from inference;
2. accept a fixed bounded slice and report any necessary boundary change;
3. execute through controlled implementation and gate phases using one persistent identity;
4. satisfy or truthfully reject each validation requirement;
5. preserve baseline and unrelated work where the medium has mutable state;
6. return the common terminal receipt fields plus namespaced adapter metrics; and
7. distinguish objective completion, remaining work, stops, and failures.

The supervisor must not emulate a missing capability. A builder-specific evidence or reviewer skill belongs in `builder.context`; the supervisor passes it through and records it but does not invoke it directly.

## Provenance and amendments

Store a compact `engine_config` in every schema-version-3 receipt:

```json
{
  "version": 1,
  "source": "inline_user_config",
  "objective": "Advance the Site2JSON roadmap",
  "work_source": {"kind": "file", "value": "docs/roadmap.md"},
  "builder": {"skill": "slice-builder", "model": "gpt-5.6-sol", "reasoning_effort": "low", "context": {"evidence_skill": "claude-recon-implementation"}},
  "validation": {"profile": "engineering-full", "requirements": ["focused_checks", "full_suite", "freshness_checks", "adversarial_review"]},
  "metrics": {"path": "docs/ai-workflow-metrics.jsonl"},
  "limits": {},
  "approval": {"plan": "procedural_when_safe", "uninterrupted_after_plan": false},
  "notifications": {"intervention": true, "completion": false},
  "stop_on": ["objective_complete", "human_judgment", "unresolved_architecture", "quota_exhaustion", "validation_nonconvergence"],
  "explicit_fields": ["objective", "work_source"],
  "defaulted_fields": ["builder", "validation", "metrics", "limits", "approval", "notifications", "stop_on"],
  "amendments": []
}
```

Use `source` values such as `inline_user_config`, a named config path, or `request_plus_defaults`. For an approved amendment, append a concise object containing the timestamp, changed fields, prior values, new values, reason, and `human_approval`; do not rewrite provenance as though the new value had always applied.

Keep `run_id`, slice number, lifecycle state, timestamps, and acceptance results outside `engine_config`. They are observed run state, not campaign configuration.

## Campaign examples

Cleanup changes context, not machinery:

```yaml
version: 1
objective: "Restore structural coherence"
work_source: {kind: file, value: "docs/cleanup.md"}
builder: {skill: slice-builder, model: gpt-5.6-sol, reasoning_effort: low, context: {evidence_skill: claude-recon-implementation}}
validation: {profile: engineering-full, requirements: [focused_checks, full_suite, adversarial_review]}
```

An objective without a plan is explicit:

```yaml
version: 1
objective: "Identify and remove measurable extraction bottlenecks without changing output semantics"
work_source: {kind: repository_evidence}
```

The second form authorizes evidence-based slice discovery within the objective. It does not authorize the supervisor or builder to invent product priorities.
