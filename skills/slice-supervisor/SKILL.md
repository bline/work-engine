---
name: slice-supervisor
description: Run a declaratively configured, evidence-driven work campaign through bounded slices, explicit plan acceptance, configured builder and validation gates, durable receipts, limits, notifications, and truthful stop decisions. Use when Codex must autonomously advance a roadmap, cleanup plan, performance campaign, or other multi-slice objective without absorbing worker context into the parent session.
---

# Slice Supervisor

Supervise the work engine; do not perform campaign work. Keep the parent context limited to the effective campaign configuration, lifecycle state, concise receipts, limits, and continuation decisions. Delegate inspection, reasoning, mutation, and validation to one persistent configured builder per slice.

Before starting, read [references/work-engine-config.md](references/work-engine-config.md) and [references/receipt-schema.md](references/receipt-schema.md) completely. Then read the configured builder skill and its receipt contract completely. The builder must satisfy the adapter contract in the configuration reference; stop on an unsupported capability instead of silently weakening the campaign.

## Resolve the campaign contract

Accept configuration from an inline user block, a user-named file, or the request plus documented defaults. Configuration describes the run; this skill remains the machine.

Resolve and record:

- the user's exact objective and work source;
- every explicit and defaulted configuration field, kept distinguishable;
- a nonempty `run_id` stable across the campaign;
- configured builder skill, model, effort, and builder-specific context;
- validation requirements and approval mode;
- metrics destination, hard limits, notifications, and stop conditions;
- current slice number and state.

Precedence is explicit user instruction, then the named config document, then documented defaults. Stop on meaningful conflicts or unknown fields. Never invent a plan, limit, validation waiver, approval, or builder capability. Treat the resolved configuration as immutable for the run; a material change starts a new run or is recorded as a human-approved amendment.

Treat the original objective as authoritative. A builder may reorganize work but must not silently narrow, broaden, reinterpret, or omit it. A work source supplies evidence and boundaries; it does not replace the objective.

Use these states only:

`idle → planning → awaiting_acceptance → implementing → awaiting_gate → gating → accepted`

Terminal states are `completed`, `stopped`, and `failed`.

Never skip `awaiting_acceptance`. Never begin a new slice unless the previous slice is `accepted`. Transition to `completed` only when the builder establishes that no evidence-supported in-scope slice remains and the configured completion condition is satisfied.

## Start one configured builder

Launch one worker through `builder.skill` for the next coherent slice. Pass the configured model and effort only when the builder supports them. Do not inspect repository artifacts, implementation, diffs, or test details in the supervisor. Do not redo the builder's domain reasoning.

Give the worker only:

- the effective campaign configuration and provenance;
- original objective, work source, and applicable local instructions;
- relevant prior accepted receipts, not transcripts;
- hard limits and approval boundaries; and
- the planning phase contract required by the configured builder.

Require read-only evidence gathering and an evidence-based plan only. The builder must first map shallow placement alternatives, decide provisionally, and use a fresh targeted-reconnaissance call to prove or falsify only the selected boundary. The plan must include the bounded slice, premise conflicts, observed evidence versus inference, placement candidates and risk, a confirmed placement certificate, rejected alternatives, invariants, expected output/change boundary, a vertical semantic test, acceptance checks, deferred scope, open decisions, baseline overlaps where applicable, and a concise recommendation. Do not allow implementation during planning.

Retain the builder identity through planning, implementation, and gate phases so its bounded understanding survives. Start a fresh builder for the next slice. Apply the configured builder's evidence-based replacement or escalation protocol; never improvise one merely to sustain momentum.

## Accept or escalate the plan

Evaluate procedure, not domain design. Auto-accept only when `approval.plan` permits it and all are true:

- evidence supports a bounded coherent slice;
- the plan preserves the objective and work-source boundary;
- targeted reconnaissance confirms a placement certificate that names the actual producer, state owner, consumer, lifecycle, semantic consequence, and downstream proof;
- rejected alternatives and plausible-but-insufficient substitutes are explicit, and no equally supported placement remains unresolved;
- invariants, output boundary, acceptance checks, deferred scope, and overlaps are explicit;
- no consequential product, architecture, aesthetics, ownership, policy, destructive, publication, migration, or other configured human decision remains open;
- the builder declares support for every configured validation requirement; and
- no hard limit or ownership conflict is active.

Otherwise stop before execution, notify when configured, and ask for the smallest decision needed. Record acceptance as `procedural_auto_approval` or `human_approval`; use `not_reached` only when the slice terminates before acceptance. Acceptance fixes the boundary. Newly necessary scope must return to `awaiting_acceptance` or stop for judgment.

## Execute in controlled phases

Send the accepted slice and placement certificate verbatim to the same builder. Require it to prove the smallest vertical semantic path before broad implementation, execute only the accepted boundary, perform inexpensive configured checks, and stop immediately before the final gate with a concise implementation receipt: outputs or changed files, baseline overlaps, vertical and targeted checks, unresolved concerns, and gate readiness. A missing owner or consumer invalidates the boundary and returns to plan acceptance; it is not an ordinary implementation repair.

Move to `awaiting_gate` only when execution is complete. Then authorize the builder's complete configured validation and adversarial-review loop. Require valid in-scope findings to be corrected and the gate repeated until accepted or a stop condition occurs.

When `approval.uninterrupted_after_plan` is true, execution and gate may share one follow-up. Explicit plan acceptance, phase accounting, configured validation, and both receipts remain mandatory.

## Accept and record a slice

Require the configured builder's terminal receipt plus the common fields in its adapter contract. Accept only after every configured blocking gate passes, blocking findings are resolved, and unresolved issues are truthfully classified. Completed work with pending validation is not accepted.

Do not retain raw tool output, exploration, debugging, test logs, diffs, or copied source. Ask the builder to compress an overlong receipt rather than summarizing its evidence yourself.

When `metrics.path` is non-null, append exactly one terminal receipt per slice using:

```bash
python3 work-engine/skills/slice-supervisor/scripts/append_metrics.py \
  --path <configured-metrics-path> --record-json '<receipt>'
```

The script validates and locks the append. Record the effective engine configuration, placement proof, and provenance as required by schema version 3. Preserve unavailable values as `null` and flexible builder metrics inside their namespaced objects. Correct rejected receipts from actual evidence; never pad them with guesses. If the user explicitly configured a null metrics path, retain the receipt in supervisor state for the final report and state that no durable record was written.

## Decide whether to continue

After every accepted receipt:

1. Check configured limits, stop conditions, and unresolved decisions.
2. Compare comparable measurements with recent accepted records. Flag material outliers such as more than twice a recent median as attention evidence, not automatic failure unless a configured limit is crossed.
3. Continue only if the builder reports meaningful in-scope work remains and acceptance is clean.
4. Start a new configured builder for the next slice.

Stop and preserve state when a configured stop condition occurs, evidence is insufficient, the objective conflicts with its work source, ownership cannot be established, validation cannot distinguish work failure from environment failure, repairs do not converge, a required capability or quota is unavailable, a hard limit is reached, or human judgment is required.

On a stop, append a truthful `stopped` or `failed` receipt with the exact triggering condition. Never represent termination as success or a builder's inability to find work as objective completion without evidence.

## Notify and finish

Use notifications only when both the effective configuration and applicable repository instructions allow them; obey any repository-required intervention notification even when the config omits it. Do not notify for progress or ordinary slice acceptance. Include the slice, state, exact reason, and whether state is stable.

At the end, report the effective campaign, slices attempted/accepted/stopped, objective status, aggregate available metrics, review findings, anomalies, supported workflow improvements, and exact stop or completion reason. Missing metrics remain `null`; incomplete history must not imply precision.
