---
name: slice-supervisor
description: Run a declaratively configured, evidence-driven work campaign through bounded slices, explicit plan acceptance, configured builder and validation gates, durable receipts, limits, notifications, and truthful stop decisions. Use when Codex must autonomously advance a roadmap, cleanup plan, performance campaign, or other multi-slice objective without absorbing worker context into the parent session.
---

# Slice Supervisor

Supervise the work engine; do not perform campaign work. Keep the parent context
limited to the effective campaign configuration, lifecycle state, concise
receipts, review-selection projections, limits, and continuation decisions.
Delegate repository inspection, implementation reasoning, mutation, validation
execution, and selected-review execution to one persistent configured builder
per slice. The supervisor owns specialist selection and acceptance; it does not
diagnose the review subject.

Before starting, read
[references/work-engine-config.md](references/work-engine-config.md),
[references/review-selection.md](references/review-selection.md), and
[references/receipt-schema.md](references/receipt-schema.md) completely. Then
read the configured builder skill, the decision policy it owns, and its receipt
contract completely. The builder must satisfy the adapter contract in the
configuration reference; stop on an unsupported capability instead of silently
weakening the campaign.

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

Move to `awaiting_gate` only when execution is complete. Authorize the
deterministic portion of the configured validation profile first. Require the
builder to run its ordered deterministic manifest and repair failures before
model review. When those checks are ready, require the builder's bounded
review-selection projection: candidate-binding inputs, integrity-bound artifact
references, task-owned artifact roles, changed contract consequences, the
`present`/`absent`/`uncertain` agent-instruction surface assessment, and material
uncertainty. If that projection is missing or insufficient, return it to the
builder; do not inspect source or diffs in the supervisor.

Create or identify the immutable review candidate, then select specialist
perspectives under
[the review-selection contract](references/review-selection.md). The supervisor
owns this selection. The builder may expose consequences and uncertainty but
must not add, omit, or replace specialists. Give `agent-instruction-review` one
explicit disposition for every candidate: select it for a present or uncertain
agent-instruction consequence, or record why an observed absence makes it
inapplicable to selection. Selection does not declare the specialist itself
applicable.

Send the exact selection plan and subject binding to the same builder. The
builder executes each selected specialist through the configured review
provider, preserves separate provider and specialist provenance, and returns
each attributed result. A selected `agent-instruction-review` instance follows
its own finding contract and may return `applicable` or `omitted`. The builder
evaluates findings and performs authorized remediation; it cannot disposition
the supervisor's selection. Reconsider the panel only when a new immutable
candidate changes the projected consequences or review premise.

For proportional profiles, require the builder to justify deterministic check
breadth while the supervisor selects review independence from consequence,
reversibility, uncertainty, changed boundaries, and repository instructions.
For full profiles, execute every configured deterministic gate before initial
review and preserve its configured fresh-review requirement in the selection
plan. Require valid in-scope findings to be corrected and affected checks
repeated. A repair creates a new immutable candidate; return its exact delta to
the same retained reviewer and reconsider selection only when the projected
consequences or review premise changed. Run a final broad deterministic gate
only when configured or warranted by the resulting risk, then bind the terminal
gate consequence to the final candidate and review outcomes.

When `approval.uninterrupted_after_plan` is true, execution and gate may share one follow-up. Explicit plan acceptance, phase accounting, configured validation, and both receipts remain mandatory.

### Active slice recovery

Model and provider sessions are runtime bindings, not durable owners of an
active slice attempt. Before a configured planning or review obligation crosses
a provider boundary, publish its attempt, phase, and pending obligation with
`skills/slice-supervisor/scripts/manage_active_slice.py begin`. When that capability is temporarily
unavailable, preserve the same obligation with `skills/slice-supervisor/scripts/manage_active_slice.py
wait` and the expected durable revision. Reconstruct that same attempt after
context or session replacement with `skills/slice-supervisor/scripts/resume_active_slice.py`; continue
only the recovered pending phase.

When the pending obligation is conclusively discharged, or the attempt reaches
an authoritative terminal outcome already decided by its owner, record
lifecycle closure with `skills/slice-supervisor/scripts/manage_active_slice.py retire`. Terminal
outcomes include, without being limited to, accepted, stopped, and failed. The
CLI records decisions already made by the model-driven supervisor; it does not
decide discharge, acceptance, stopping, failure, provider choice, retry,
scheduling, or authority. The slice workflow alone authorizes and interprets
transitions; the shared durable-state primitive only publishes opaque revisions
atomically.

Preserve stable run, slice, attempt, and plan identity, handled consequence IDs,
reference-only links to stronger artifacts, and the logical actor binding.
Provider session identity is optional provenance. Capability waiting is
nonterminal, while retirement prevents stale input from resurrecting the
attempt. This path is distinct from `resume_campaign.py`; terminal-history
semantics remain unchanged.

Implementation and gate completion can cross a builder return or mailbox
boundary before the enclosing slice is terminal. Before that transient channel
can become the only copy, require the builder-side workflow to publish the
compact phase consequence with `skills/slice-supervisor/scripts/manage_active_slice.py publish-phase`.
Bind established consequences to integrity-identified implementation, artifact,
or gate evidence. If interruption prevents completion from being established,
publish an explicit uncertain consequence when recovery can do so safely; never
infer completion from runtime liveness or remembered delivery.

`skills/slice-supervisor/scripts/resume_active_slice.py` returns the supervisor's compact recovery
projection: stable attempt identity, current phase, accepted boundary, pending
obligation, latest phase consequence, authoritative references, lifecycle
state, and durable provenance. Its `runtime_binding` is explicitly nonsemantic.
The projection is not a general interface to model context, transcripts, hidden
reasoning, or worker control-plane state. Continue only from the recovered
semantic consequence and its integrity-bound references.

For a review provider that supports resumable sessions, assign the reviewer
runtime session ID before the initial provider call and publish it in
`actor_binding.runtime_session_id` through `skills/slice-supervisor/scripts/manage_active_slice.py
bind-actor`, with the current durable revision as `--expected-revision` and a
stable unique `--event-id`. The compare-and-swap and replay identity prevent a
stale transition from replacing the intended reviewer binding; skipping this
route can make recovery lose the session or resume the wrong reviewer
continuation. Require the builder to use that exact
binding for the fresh-entry review and every ordinary remediation continuation.
After supervisor-context replacement, recover the binding with
`skills/slice-supervisor/scripts/resume_active_slice.py` and resume the provider session instead of recreating
the review research. The reviewer remains a runtime binding, not the durable
owner: if it is unavailable or model judgment requires a fresh perspective,
record replacement provenance and the reset reason while preserving the same
pending obligation and applicable findings.

Active-slice recovery and publication are owned only by these slice-supervisor
scripts; do not infer a copy under another skill. Every `--identity-json` value
must be an object with exactly `run_id`, `slice_number`, `attempt_id`, and
`plan_version`; camelCase keys and inferred alternate shapes are invalid.

## Accept and record a slice

Require the configured builder's `audit_receipt` plus its compact
`handoff_receipt`. Verify that `worker_metrics.review_selection` reproduces the
supervisor's exact state. A stopped or failed slice that ended before candidate
creation records selection as `not_reached` without inventing a subject or
specialist disposition. If a candidate exists but terminalization occurs before
selection is decided, record `undecided` with that subject and the reason, without
inventing dispositions. A decided selection reproduces the exact subject and
plan, gives `agent-instruction-review` exactly one selected-or-omitted
disposition, and records every selected specialist's execution and applicability
outcome. Accept only after every configured blocking gate passes, every selected
required review completed, blocking findings are resolved, and unresolved
issues are truthfully classified. Accepted work with selection other than
`decided`, a missing selection disposition, pending selected review, or pending
validation is invalid. Retain the handoff only for relevant future builder
context; never use it as the durable record.

When the campaign uses Git-backed slice checkpoints, request a candidate only
after the deterministic gate is ready for review. Give the checkpoint adapter
the declared baseline, exact attributed manifest, plan version, scope revision,
and gate-receipt digest. The adapter owns isolated Git mechanics; the supervisor
owns the lifecycle consequence. Pin review to the returned candidate commit,
tree, task-patch, plan, scope, and gate identities. A material repair requires a
new candidate and preserves the old candidate and findings.

Before acceptance, validate the review result with
`scripts/checkpoint_lifecycle.py`. Only an exact binding may create the private
accepted lifecycle checkpoint. Pass that accepted receipt to terminal
finalization so the durable audit receipt and next-slice recovery name the same
identity. Stopped or failed checkpoint refs preserve recovery state but never
advance acceptance. Checkpoint mechanics may update only private local
`refs/work-engine/checkpoints/`; they never move the user's branch, alter the
real index or working tree, create tags, or publish refs.

After private acceptance, apply `engine_config.slice_completion_commit.prompt`.
When disabled, suppress the offer and continue from the private checkpoint.
When enabled, prefer sending the accepted checkpoint and exact manifest back to
the completing builder before releasing useful context. Require its compact
schema-version-2 proposal to bind both and truthfully name its producer and
durable supporting evidence. If that context is unavailable, a replacement may
reconstruct only from the accepted checkpoint plus durable attributed manifest
and compact terminal semantic consequences. Insufficient evidence must produce
refusal or explicit uncertainty, never an unsupported equivalence claim. The
producer route and provider/model identity do not authorize the proposal; its
content and immutable bindings remain decisive. Then ask the user to create or
decline the ordinary commit. Only an explicit per-slice create decision authorizes
`$slice-completion-commit`; an open, declined, unavailable, or refused offer
never blocks checkpoint-based continuation. The supervisor owns one durable
live offer outside model context, terminal audit history, and the ordinary
worktree. The current implementation uses a private Work Engine ref as storage;
that mechanism is an affordance rather than invariant product structure. The
sibling adapter owns real Git preflight, mutation, and read-only reconciliation
after uncertain publication. Because resolving a stale open offer could
misreport an already-published commit, every terminal transition reconciles Git
first and fails closed when publication state is ambiguous. Persist terminal
offer transitions through the live owner, and retain the accepted private checkpoint—not current branch
HEAD—as the authoritative next-slice baseline.

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
  --semantic-receipt-json '<schema-v5-audit-receipt>' \
  --telemetry-ingress-json '<telemetry-ingress-v1>' \
  --campaign-preflight-json '<original-successful-preflight-result>' \
  --handoff-receipt-json '<compact-handoff-receipt>' \
  --checkpoint-receipt-json '<accepted-checkpoint-receipt>' \
  --completion-commit-receipt-json '<resolved-created-declined-or-refused-receipt>'
```

The finalizer passes the assembled in-memory result directly to the existing
append boundary. It does not use an intermediate receipt file or reread the
campaign. Before a `created` completion-commit projection can enter that result,
the finalizer requires the completion adapter to re-establish its Git claims
read-only; otherwise fabricated adapter-shaped input could become durable
history. The assembler remains the authoritative projection owner and append
remains the schema-v5 write, terminal-identity, locking, and durability owner.
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

The script compatibility-validates historical receipts and requires schema
version 5 for
the new durable write, and atomically rejects an already-durable `run_id` and
`slice_number` identity while holding the audit append lock. Historical schema
versions remain readable by the compatibility validator but are not valid
inputs to this production command. Record the effective engine configuration, placement proof,
and evidence-routing provenance as required by schema version 5. Preserve
unavailable measurements as `null`, zero counts as zero, and flexible
provider-native metrics inside their namespaced objects. Correct rejected audit
receipts from actual evidence; never pad them with guesses. Never append the
compact handoff. If the user explicitly configured a null metrics path, retain
both receipt views in supervisor state for the final report and state that no
durable record was written.

## Reconcile strategy when execution changes the map

Strategic reconciliation is a separate capability, not another builder phase.
Use `$strategic-planner` when durable execution evidence could materially
change roadmap priority, dependency order, an architectural assumption,
expected value, release readiness, or the wisdom of continuing the current
campaign unchanged. Do not invoke it on a fixed slice cadence or from a numeric
signal alone, and do not make the supervisor continuously re-plan the product.

Pass the bounded strategic objective, governing roadmap and doctrine, current
repository revision, compact terminal consequences, the prior planning handoff
when one exists, and relevant proposal expectations. Do not pass raw builder
transcripts, logs, source dumps, or audit bookkeeping that has no strategic
consumer.

The planner returns a compact planning handoff; it does not accept slices or
amend the campaign. The supervisor decides the execution consequence within
its authority. A recommendation that changes the objective, work-source
boundary, approval, or another immutable configuration field requires the
owning amendment or a new campaign before another builder starts. If strategic
review is materially necessary but unavailable, stop rather than continuing
on a roadmap known to be unreliable.

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
reserving it. When the terminal receipt carries a slice checkpoint, recovery
also verifies the private ref and commit tree and returns it as
`baseline_checkpoint`; that immutable identity, not current branch HEAD, is the
next slice's repository baseline. Structured stopped, failed, accepted-complete, and historical
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
