---
name: slice-supervisor
description: Run a declaratively configured, evidence-driven work campaign through bounded slices, explicit plan acceptance, configured builder and validation gates, durable receipts, limits, notifications, and truthful stop decisions. Use when Codex must autonomously advance a roadmap, cleanup plan, performance campaign, or other multi-slice objective without absorbing worker context into the parent session.
---

# Slice Supervisor

Supervise the work engine; do not perform campaign work. Keep the parent context limited to the effective campaign configuration, lifecycle state, concise receipts, limits, and continuation decisions. Delegate inspection, reasoning, mutation, and validation to one persistent configured builder per slice.

Before starting, read [references/work-engine-config.md](references/work-engine-config.md) and [references/receipt-schema.md](references/receipt-schema.md) completely. Then read the configured builder skill, the decision policy it owns, and its receipt contract completely. The builder must satisfy the adapter contract in the configuration reference; stop on an unsupported capability instead of silently weakening the campaign.

## Preserve invariants; adapt the route

Keep these categories distinct throughout the campaign:

- **Invariants** are always binding: preserve the objective, evidence provenance, user work, approval boundaries, configured hard requirements, and truthful state.
- **Acceptance conditions** describe what must be demonstrated for this slice. Scale the evidence to the consequence and uncertainty; do not confuse more procedure with stronger proof.
- **Routes** are revisable defaults. Repository retrieval, placement scouting,
  fresh falsification, broad validation, and independent review are distinct
  capabilities selected for a reason, not universal truth conditions. Changing
  retrieval does not change or satisfy review independence.
- **Recovery decisions** record which premise failed, which evidence remains valid, what state became stale, and why the revised route still serves the objective.

At every phase boundary, ask whether the current route remains the simplest credible path to the objective. Revise it when observed evidence invalidates a premise. Never preserve a stale plan merely because it is the next legal transition, and never use adaptability to weaken an invariant or configured requirement.

## Resolve the campaign contract

For a named campaign file, execute preflight before launching a builder:

```bash
node skills/slice-supervisor/scripts/campaign-preflight.mjs <campaign.yaml>
```

Consume the returned `engineConfig` as the durable effective configuration and
retain the separate `campaignSource` identity for terminal receipt assembly.
Never transcribe either value from the campaign by hand. Pass
`resolvedCapabilities` only as transient builder launch context. Preflight
resolves the CLI argument from the invoker's working directory, applies the
documented defaults, records explicit versus defaulted top-level fields, binds
the named campaign to its canonical path and digest, resolves external
capability references from the campaign directory, and resolves Chrome
Vision-owned filesystem paths from the file that authored that configuration.
A capability declaration makes a tool available; it neither proves nor
requires use. Stop on preflight failure rather than manually reinterpreting the
file.

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

Use these primary states:

`idle → planning → awaiting_acceptance → implementing → awaiting_gate → gating → accepted`

Terminal states are `completed`, `stopped`, and `failed`.

Keep `awaiting_acceptance` as an explicit accounting state, although a concise low-risk plan may be procedurally accepted without user interruption. Permit any active phase to return to `planning` when new evidence invalidates the boundary, route, or acceptance evidence; permit `gating` to return to `implementing` for bounded fixes. Record the reason and preserve only evidence that remains applicable. Never begin a new slice unless the previous slice is `accepted`. Transition to `completed` only when the builder establishes that no evidence-supported in-scope slice remains and the configured completion condition is satisfied.

## Start one configured builder

Launch one worker through `builder.skill` for the next coherent slice. Pass the configured model and effort only when the builder supports them. Do not inspect repository artifacts, implementation, diffs, or test details in the supervisor. Do not redo the builder's domain reasoning.

Give the worker only:

- the effective campaign configuration and provenance;
- original objective, work source, and applicable local instructions;
- relevant prior accepted `handoff_receipt` objects, not audit receipts or transcripts;
- hard limits and approval boundaries; and
- the planning phase contract required by the configured builder.

Require read-only evidence gathering and an evidence-based plan only. The builder first classifies placement risk and selects a justified route. An obvious, local, reversible boundary may use direct targeted evidence. Ambiguous, cross-boundary, consequential, or high-risk placement uses shallow alternatives followed by a fresh attempt to falsify the selected boundary. The plan must include the bounded slice, premise conflicts, observed evidence versus inference, placement risk and route, a confirmed semantic-path certificate, invariants, expected output/change boundary, a vertical semantic proof, proportionate acceptance checks, deferred scope, open decisions, baseline overlaps where applicable, and a concise recommendation. Require candidates and rejected alternatives only when plausible alternatives exist. Do not allow implementation during planning.

Retain the builder identity through planning, implementation, and gate phases so its bounded understanding survives. Start a fresh builder for the next slice. Apply the configured builder's evidence-based replacement or escalation protocol; never improvise one merely to sustain momentum.

## Accept or escalate the plan

Evaluate procedure, not domain design. Auto-accept only when `approval.plan` permits it and all are true:

- evidence supports a bounded coherent slice;
- the plan preserves the objective and work-source boundary;
- the selected route is justified by placement risk and evidence independence needs;
- evidence confirms a semantic-path certificate that names the actual producer, state owner, consumer, lifecycle, semantic consequence, and downstream proof;
- plausible alternatives and insufficient substitutes are resolved when they materially affect placement;
- invariants, output boundary, acceptance checks, deferred scope, and overlaps are explicit;
- no consequential product, architecture, aesthetics, ownership, policy, destructive, publication, migration, or other configured human decision remains open;
- the builder declares support for every configured validation requirement; and
- no hard limit or ownership conflict is active.

Otherwise keep planning when another bounded evidence step can resolve the issue; stop and ask only when progress requires human judgment, new authority, or unavailable capability. Record acceptance as `procedural_auto_approval` or `human_approval`; use `not_reached` only when the slice terminates before acceptance. Acceptance establishes the current boundary, not an irreversible fiction. Invalidating evidence reopens planning and requires renewed acceptance while preserving compatible evidence and recording stale decisions.

## Execute in controlled phases

Send the accepted slice and semantic-path certificate verbatim to the same builder. Require it to prove the smallest vertical semantic path before broad implementation, execute only the accepted boundary, perform inexpensive configured checks, and stop immediately before the final gate with a concise implementation receipt: outputs or changed files, baseline overlaps, vertical and targeted checks, unresolved concerns, and gate readiness. A missing owner or consumer invalidates the boundary and returns to planning with a route-revision record; it is not an ordinary implementation repair or an automatic terminal stop.

Move to `awaiting_gate` only when execution is complete. Then authorize the builder's configured validation profile. For proportional profiles, require the builder to justify check breadth and review independence from consequence, reversibility, uncertainty, changed boundaries, and repository instructions. For full profiles, execute every configured gate. Require valid in-scope findings to be corrected and affected checks repeated; run a final broad gate only when configured or warranted by the resulting risk.

When `approval.uninterrupted_after_plan` is true, execution and gate may share one follow-up. Explicit plan acceptance, phase accounting, configured validation, and both receipts remain mandatory.

## Accept and record a slice

Require the configured builder's `audit_receipt` plus its compact `handoff_receipt`. Accept only after every configured blocking gate passes, blocking findings are resolved, and unresolved issues are truthfully classified. Completed work with pending validation is not accepted. Retain the handoff only for relevant future builder context; never use it as the durable record.

Do not retain raw tool output, exploration, debugging, test logs, diffs, or copied source. Ask the builder to compress an overlong receipt rather than summarizing its evidence yourself.

For a named campaign, assemble the terminal semantic receipt with its matching
telemetry ingress and the original successful campaign-preflight result before
append. The assembler replaces model-authored `engine_config` with the
preflight-owned value and records campaign-source provenance separately from
telemetry provenance. Finalize the named-campaign receipt through the composed
production command so authoritative assembly cannot be omitted:

```bash
python3 skills/slice-supervisor/scripts/finalize_receipt.py \
  --path <configured-metrics-path> \
  --semantic-receipt-json '<schema-v4-audit-receipt>' \
  --telemetry-ingress-json '<telemetry-ingress-v1>' \
  --campaign-preflight-json '<original-successful-preflight-result>' \
  --handoff-receipt-json '<compact-handoff-receipt>'
```

The finalizer passes the assembled in-memory result directly to the existing
append boundary. It does not use an intermediate receipt file or reread the
campaign. The assembler remains the authoritative projection owner and append
remains the schema-v4 write, terminal-identity, locking, and durability owner.
For an accepted slice with remaining work, the finalizer validates the handoff
against the assembled receipt and derives its four nonduplicated semantic
collections into `continuation_context`. It omits that projection for
accepted-complete, stopped, and failed terminals.

For terminal receipts not sourced from a named campaign, when `metrics.path` is
non-null, append exactly one already-authoritative terminal audit receipt per
slice using:

```bash
python3 skills/slice-supervisor/scripts/append_metrics.py \
  --path <configured-metrics-path> --record-json '<audit-receipt>'
```

The script compatibility-validates the receipt, requires schema version 4 for
the new durable write, and atomically rejects an already-durable `run_id` and
`slice_number` identity while holding the audit append lock. Historical schema
versions remain readable by the compatibility validator but are not valid
inputs to this production command. Record the effective engine configuration, placement proof,
and evidence-routing provenance as required by schema version 4. Preserve
unavailable measurements as `null`, zero counts as zero, and flexible
provider-native metrics inside their namespaced objects. Correct rejected audit
receipts from actual evidence; never pad them with guesses. Never append the
compact handoff. If the user explicitly configured a null metrics path, retain
both receipt views in supervisor state for the final report and state that no
durable record was written.

## Decide whether to continue

After an interruption between terminal slices, recover one named run with:

```bash
python3 skills/slice-supervisor/scripts/resume_campaign.py \
  --path <configured-metrics-path> \
  --campaign-preflight-json '<fresh-successful-preflight-result>' \
  --run-id '<stable-run-id>'
```

Resume only when the command reports `resumable: true`. It binds both the
effective engine configuration and campaign-source identity, reconstructs the
compact handoff, and identifies the next sequential slice without writing or
reserving it. Structured stopped, failed, accepted-complete, and historical
continuation-unavailable results are stable non-resumable states. Command
failure means malformed or incompatible state requiring intervention. This
path does not recover mid-slice or partial artifacts and does not authorize an
amendment.

After every accepted receipt:

1. Check configured limits, stop conditions, and unresolved decisions.
2. Compare comparable measurements with recent accepted records. Flag material outliers such as more than twice a recent median as attention evidence, not automatic failure unless a configured limit is crossed.
3. Continue only if the builder reports meaningful in-scope work remains and acceptance is clean.
4. Start a new configured builder for the next slice.

Stop and preserve state when a configured stop condition occurs, a bounded replan cannot resolve insufficient evidence, the objective conflicts with its work source, ownership cannot be established, validation cannot distinguish work failure from environment failure, repairs do not converge, a required capability or quota is unavailable, a hard limit is reached, or human judgment is required. A route change, corrected premise, or recoverable provider failure is not by itself a stop condition.

On a stop, append a truthful `stopped` or `failed` receipt with the exact triggering condition. Never represent termination as success or a builder's inability to find work as objective completion without evidence.

## Notify and finish

Use notifications only when both the effective configuration and applicable repository instructions allow them; obey any repository-required intervention notification even when the config omits it. Do not notify for progress or ordinary slice acceptance. Include the slice, state, exact reason, and whether state is stable.

At the end, report the effective campaign, slices attempted/accepted/stopped, objective status, aggregate available metrics, review findings, anomalies, supported workflow improvements, and exact stop or completion reason. Missing metrics remain `null`; incomplete history must not imply precision.
