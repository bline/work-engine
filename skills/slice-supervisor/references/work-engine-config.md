# Work engine configuration

Configuration supplies campaign context to the stable supervisor state machine. Use YAML or JSON with these exact fields. Reject unknown fields so misspellings do not silently change behavior.

## Version 2 shape

```yaml
version: 2
objective: "Advance the Site2JSON roadmap"

work_source:
  kind: file
  value: "docs/roadmap.md"

builder:
  skill: "slice-builder"
  model: "gpt-5.6-sol"
  reasoning_effort: "low"
  context:
    repository_evidence:
      skill: "repo-search"
      provider: "codex-codebase-memory"
    independent_review:
      skill: "claude-recon-implementation"
      provider: "claude"

validation:
  profile: "engineering-proportional"
  requirements:
    - semantic_proof
    - risk_proportional_checks
    - workspace_integrity

metrics:
  path: "docs/ai-workflow-metrics.jsonl"

limits: {}

approval:
  plan: "procedural_when_safe"
  uninterrupted_after_plan: false

notifications:
  intervention: true
  completion: false

slice_completion_commit:
  prompt: enabled

stop_on:
  - objective_complete
  - human_judgment
  - unresolved_architecture
  - quota_exhaustion
  - validation_nonconvergence

capabilities:
  chrome_vision:
    config: ../config/chrome-vision.yaml
```

## Field contract

- `version`: Required integer; use `2` for new campaigns. Version 1 remains a
  compatibility shape with its historical combined evidence/review meaning.
- `objective`: Required nonempty statement. Preserve the user's wording.
- `work_source`: Optional evidence/boundary source. `kind` is `file`, `inline`, or `repository_evidence`; `value` is required for `file` and `inline` and omitted for `repository_evidence`. A missing work source means the objective plus applicable repository evidence, not an inferred roadmap file.
- `builder`: Optional only when all documented defaults apply. `skill` identifies an adapter satisfying the contract below. `model` and `reasoning_effort` are passed only if supported. `context` is a namespaced object owned by that builder; retain it verbatim in provenance but never place secrets in configuration. In version 2, `context.repository_evidence` and `context.independent_review` each contain a `provider` and `skill` and resolve independently. A human-authorized campaign may configure `context.adversarial_review` instead, with provider, skill, model, reasoning effort, evidence class, and isolation. Exactly one review-execution role is allowed; accepted same-model review cannot satisfy an explicit independent-review requirement. The role supplies provider/session mechanics for supervisor-selected specialists and does not give the builder review-selection authority. Do not mix version-1 and version-2 context fields.
- `validation`: `profile` names a builder-supported evidence policy and `requirements` lists observable outcomes. The builder must confirm support before plan acceptance. Under `engineering-proportional`, the builder selects deterministic check breadth while the supervisor selects review perspectives and required independence from the slice's projected consequence and risk. `engineering-full` requires focused checks, applicable freshness checks, the full suite, and supervisor selection of fresh adversarial review. Explicit requirements remain binding under either profile. Do not reinterpret an unknown requirement.
- `metrics.path`: Durable JSONL destination. An explicit `null` disables durable metrics only when the user says so; do not confuse it with a missing value.
- `limits`: Hard limits explicitly supplied by the user, such as `slices`, `time_seconds`, `cost_usd`, `tokens`, or `repair_attempts`. An empty object means no configured hard limits. Never invent them.
- `approval.plan`: `procedural_when_safe` or `human_required`. `approval.uninterrupted_after_plan` controls whether execution and gate may share a follow-up; it never removes plan acceptance.
- `notifications`: Requested Boolean policy for intervention and overall completion. Applicable repository instructions determine the actual command, may require intervention notification, and may prohibit completion notification; they take precedence over this request-level policy.
- `slice_completion_commit.prompt`: `enabled` offers an explicitly authorized
  ordinary commit after each accepted slice; `disabled` suppresses the offer.
  It never enables automatic commits or disables private checkpoints and
  checkpoint-based continuation.
- `stop_on`: Named terminal conditions. The supervisor always stops for safety, unsupported capabilities, ownership ambiguity, or required human authority even if omitted. Unknown conditions require clarification.
- `capabilities.chrome_vision.config`: Optional Chrome Vision availability declaration. It is either an inline config-v1 mapping or a YAML/JSON path resolved relative to the declaring campaign file. Run `node skills/slice-supervisor/scripts/campaign-preflight.mjs <campaign.yaml>` before builder launch. For an external config, `artifactDirectory` is relative to that config file; for inline config, it is relative to the campaign file. Preflight returns the complete initial effective `engineConfig`, a separate digest-backed `campaignSource` identity for the named campaign, and the full capability configuration only in transient `resolvedCapabilities`. The assembler projects `engineConfig` into the terminal receipt and records `campaignSource` separately under producer provenance. Chrome Vision durable provenance retains source kind, authoring/reference path, canonical base/path, SHA-256, and schema version—not the expanded runtime configuration and not a claim that the capability was used.

## Documented defaults

Defaults provide an adaptive engineering workflow while explicit full-gate campaigns remain unchanged:

- `builder.skill`: `slice-builder`
- `builder.model`: `gpt-5.6-sol`
- `builder.reasoning_effort`: `low`
- `builder.context.repository_evidence.skill`: `repo-search`
- `builder.context.repository_evidence.provider`: `codex-codebase-memory`
- `builder.context.independent_review.skill`: `claude-recon-implementation`
- `builder.context.independent_review.provider`: `claude`
- `validation.profile`: `engineering-proportional`
- `validation.requirements`: `semantic_proof`, `risk_proportional_checks`, `workspace_integrity`
- `metrics.path`: `docs/ai-workflow-metrics.jsonl`
- `limits`: `{}`
- `approval.plan`: `procedural_when_safe`
- `approval.uninterrupted_after_plan`: `false`
- `notifications.intervention`: `true`
- `notifications.completion`: `false`
- `slice_completion_commit.prompt`: `enabled`
- `stop_on`: the list in the example above

Record which values were explicit and which came from these defaults. A user explicitly selecting the default value remains an explicit decision; classify provenance by source, not by value equality. Defaults are not user decisions.

## Builder adapter contract

A configured builder must declare before plan acceptance that it can:

1. keep planning read-only and return observed evidence separately from inference;
2. accept a bounded slice, preserve its invariants, and return for renewed acceptance when evidence requires a route or boundary revision;
3. execute through controlled implementation and gate phases using one persistent identity;
4. satisfy or truthfully reject each validation requirement;
5. preserve baseline and unrelated work where the medium has mutable state;
6. project the exact candidate-binding inputs, accept the supervisor's immutable
   review subject and selection, execute the selected specialists without
   changing that selection, and return their attributed outcomes;
7. return a complete terminal `audit_receipt` with common fields and namespaced adapter metrics, plus a compact semantic `handoff_receipt` for the next builder; and
8. distinguish objective completion, remaining work, stops, and failures.

The supervisor must not emulate a missing capability. Builder-specific
repository-evidence and review-execution roles belong in `builder.context`; the
supervisor passes the selected review plan to the builder and records the
outcomes but does not invoke the provider directly. The supervisor owns
specialist selection, while the builder owns provider execution and remediation.
Changing retrieval must not implicitly change or satisfy independent review.

## Provenance and amendments

For a named campaign, preflight deterministically produces the compact initial
`engine_config` below by applying documented defaults and recording authored
top-level fields separately from defaulted ones. The terminal assembler uses
that value authoritatively; the model does not transcribe it.

```json
{
  "version": 2,
  "source": "inline_user_config",
  "objective": "Advance the Site2JSON roadmap",
  "work_source": {"kind": "file", "value": "docs/roadmap.md"},
  "builder": {"skill": "slice-builder", "model": "gpt-5.6-sol", "reasoning_effort": "low", "context": {"repository_evidence": {"skill": "repo-search", "provider": "codex-codebase-memory"}, "independent_review": {"skill": "claude-recon-implementation", "provider": "claude"}}},
  "validation": {"profile": "engineering-proportional", "requirements": ["semantic_proof", "risk_proportional_checks", "workspace_integrity"]},
  "metrics": {"path": "docs/ai-workflow-metrics.jsonl"},
  "limits": {},
  "approval": {"plan": "procedural_when_safe", "uninterrupted_after_plan": false},
  "notifications": {"intervention": true, "completion": false},
  "stop_on": ["objective_complete", "human_judgment", "unresolved_architecture", "quota_exhaustion", "validation_nonconvergence"],
  "capabilities": {"chrome_vision": {"source": "file", "authoredReference": "../config/chrome-vision.yaml", "resolvedPath": "/repo/work-engine/config/chrome-vision.yaml", "pathBase": "/repo/work-engine/config", "sha256": "<lowercase sha256>", "schemaVersion": 1}},
  "explicit_fields": ["objective", "work_source"],
  "defaulted_fields": ["builder", "validation", "metrics", "limits", "approval", "notifications", "stop_on"],
  "amendments": []
}
```

Use `source` values such as `inline_user_config`, a named config path, or `request_plus_defaults`. Named-file preflight also returns `campaignSource` with `source`, `authoredReference`, canonical `resolvedPath`, canonical `pathBase`, lowercase content `sha256`, and `schemaVersion`. This source identity is producer provenance, not an `engine_config` field and not telemetry binding provenance. For an approved amendment, append a concise object containing the timestamp, changed fields, prior values, new values, reason, and `human_approval`; named campaign preflight validates and preserves authored amendment objects. Do not rewrite provenance as though the new value had always applied.

Keep `run_id`, slice number, lifecycle state, timestamps, and acceptance results outside `engine_config`. They are observed run state, not campaign configuration.

## Campaign examples

Cleanup changes context, not machinery:

```yaml
version: 2
objective: "Restore structural coherence"
work_source: {kind: file, value: "docs/cleanup.md"}
builder: {skill: slice-builder, model: gpt-5.6-sol, reasoning_effort: low}
validation: {profile: engineering-full, requirements: [focused_checks, full_suite, adversarial_review]}
```

## Version 1 compatibility

Version 1 retains the exact historical builder context:

```yaml
version: 1
builder:
  context:
    evidence_skill: claude-recon-implementation
    reconnaissance:
      provider: claude-codebase-memory
```

The resolver normalizes that pair as one combined legacy retrieval/review role
and records `legacy-combined-evidence-and-review` provenance. It also preserves
`claude-filesystem`. Version 1 rejects split-role fields; version 2 rejects the
legacy fields. Migrate by authoring version 2 explicitly, never by silently
reinterpreting a stored version-1 context.

Use `engineering-full` when the user wants the same broad gate on every slice or when the campaign's consequences make that consistency part of the acceptance contract. Under the default `engineering-proportional` profile:

- `semantic_proof` requires an observed vertical consequence, not merely compiling edits;
- `risk_proportional_checks` requires focused tests and adds broader suites plus independent review when consequence, fan-out, uncertainty, or changed boundaries warrant them; and
- `workspace_integrity` requires changed-file/baseline checks and applicable generated-artifact freshness checks.

The audit receipt records selected and omitted stages with reasons. An omitted optional stage is not reported as passed; the profile passes only when its selection policy and executed evidence both satisfy the slice's risk.

Do not rewrite `engineering-proportional` as `engineering-full` merely because a high-risk slice selected full-suite and independent-review breadth. Configuration is authored policy; `validation_breadth` is observed execution.

An objective without a plan is explicit:

```yaml
version: 2
objective: "Identify and remove measurable extraction bottlenecks without changing output semantics"
work_source: {kind: repository_evidence}
```

The second form authorizes evidence-based slice discovery within the objective. It does not authorize the supervisor or builder to invent product priorities.
