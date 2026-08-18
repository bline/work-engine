---
name: claude-recon-implementation
description: Use Claude CLI for shallow architectural placement alternatives, fresh targeted repository reconnaissance, test gating, and adversarial review while preserving Codex context for judgment and implementation. Use for non-trivial repository work, especially when a roadmap objective could plausibly belong in multiple subsystems or cross authoring, persistence, runtime, or presentation boundaries.
---

# Claude Recon Implementation

Use Claude CLI as a forked external process for high-volume, disposable evidence gathering. Codex owns task interpretation, architectural judgment, implementation, durable understanding, and final acceptance.

This workflow does not invoke Agents API delegation. Do not use `implementation-orchestrator` unless the user separately and explicitly requests agents, subagents, delegation, or multi-agent orchestration.

```text
Claude placement alternatives
→ Codex placement decision
→ fresh Claude targeted reconnaissance and falsification
→ Codex plan and implementation
→ Claude test gate
   → failure packet and stop
   → or adversarial review
→ Codex fixes
→ repeat Claude gate until accepted
```

## 1. Establish a baseline

Before implementation:

- determine the bounded slice from the request and durable project context;
- read mandatory repository instructions directly;
- record `git status --short` without loading broad diffs;
- note pre-existing changes overlapping likely task files;
- do not broadly search or read the repository yourself.

The baseline distinguishes task changes from the user's existing work. Track task-owned files throughout implementation, including new and already-dirty files.

## 2. Map placement alternatives shallowly

Before implementation reconnaissance, use one read-only Claude process to map architectural placement. This call is a routing pass, not an implementation survey. It must inspect canonical design/roadmap evidence, module ownership boundaries, and only the minimum symbols needed to distinguish plausible homes. Do not ask it to trace every candidate deeply.

Prefer:

```bash
claude -p \
  --effort medium \
  --no-session-persistence \
  --tools "Read,Grep,Bash" \
  --output-format json \
  --json-schema '<placement-schema>' \
  '<bounded placement prompt>'
```

Run Claude outside the Codex sandbox when normal API/config access or subprocess behavior requires it. Claude must not edit files, write patches, run tests or builds, invoke generators, run freshness checks, or mutate repository state. It may identify the exact commands and ranges for those operations.

If Claude fails before returning repository evidence, treat that as infrastructure failure. Inspect the execution conditions before retrying; do not repeatedly issue the same invocation. Prefer correct API/config access outside the sandbox, then retry once.

Require a JSON schema with:

- `semantic_outcome`: the user-visible or runtime consequence the slice must make true, independent of implementation shape.
- `observed_architecture`: directly established ownership, lifecycle, and runtime-boundary facts.
- `premise_conflicts`: conflicts with the request, roadmap, prior receipts, or canonical design documents.
- `placement_candidates`: at most three plausible boundaries. Each contains `boundary`, `state_owner`, `producer`, `consumer`, `lifecycle`, `evidence_for`, `evidence_against`, `required_preconditions`, `downstream_proof`, and `plausible_but_insufficient_substitute`.
- `discriminating_facts`: the smallest facts or questions that distinguish candidates; do not expand each candidate into full reconnaissance.
- `open_decisions`: decisions the evidence does not settle.
- `placement_risk`: `low`, `medium`, or `high`, with evidence.
- `placement_ranges`: only the minimum canonical-document or ownership ranges Codex must read to choose responsibly.
- available call statistics.

Do not request or return a singular recommended slice, complete call paths, exact changed-file lists, or a full implementation plan in this round. Ranking candidates is allowed only when the evidence and uncertainty remain visible.

Codex reads the returned placement ranges and records a provisional placement certificate:

```text
When <trigger> occurs, <producer> writes <state> owned by <boundary>.
<consumer> reads it and produces <semantic outcome>.
<downstream test> proves the consequence.
This is not satisfied by <plausible but insufficient substitute>.
```

Record the selected candidate, rejected alternatives and evidence, unresolved preconditions, and confidence. A value matching the first candidate is still Codex's decision, not Claude's. If evidence cannot distinguish the candidates, make at most one narrow request for the single discriminating fact or stop for architectural judgment. Never launch deep reconnaissance across every alternative.

## 3. Run fresh targeted reconnaissance

After Codex provisionally selects a placement, invoke a new Claude process with no session persistence. Give it the original objective and work source, the placement candidates, the provisional certificate, rejected alternatives and reasons, and applicable repository instructions. Explicitly require it to assume the certificate may be wrong.

This call explores only the selected boundary and the callers or dependencies necessary to prove or falsify it. Require a JSON schema with:

- `observed_state`: facts directly established from repository evidence.
- `premise_conflicts`: conflicts with the request, roadmap, or prior completion summary. Never silently redefine the task.
- `placement_verdict`: `confirmed`, `conflict`, or `unresolved`, with verified evidence. A conflict identifies the failed certificate clause; it does not silently choose another placement.
- `semantic_effect_path`: exact trigger, producer, state, owner, consumer, and observable consequence.
- `selected_boundary`: the smallest coherent implementation boundary supported by the confirmed placement.
- `relevant_ranges`: minimum code, test, and canonical-document ranges, each with `path`, numeric `start_line`, numeric `end_line`, and `reason`.
- `call_path`: relevant symbols, files, data flow, state transitions, and lifecycle.
- `module_and_test_wiring`: registration/loading, exports/globals, test bootstrap, whether browser registration is needed, package/precheck commands, check-only generated-freshness commands, and environmental requirements.
- `invariants`: behavioral and architectural truths; distinguish evidence, inference, user decision, accepted effective state, and durable runtime state where relevant.
- `acceptance_tests`: existing and required correctness checks.
- `vertical_semantic_test`: the smallest test or executable proof that fails unless the intended downstream consumer observes the new state. UI-local or producer-only tests are insufficient when the objective promises a runtime consequence.
- `test_commands`: exact precheck, focused, full-suite, and check-only freshness commands. Do not execute them during reconnaissance.
- `expected_changed_file_boundaries`: files or subsystems the slice may legitimately modify.
- `result_and_error_vocabulary`: existing accepted, rejected, unresolved, stale, failure, and related state language to preserve.
- `deferred_scope`: excluded work.
- `open_decisions`: only decisions repository evidence does not settle.
- `missing_context_risk`: likely omissions and their consequences.

Prefer narrow, cohesive ranges. Codex reads only those ranges initially and does not repeat Claude's exploration merely to confirm it. If the verdict is `conflict` or `unresolved`, stop before implementation and reconsider placement or request judgment. Do not reinterpret the intended outcome to make the selected boundary pass.

The targeted packet and placement certificate together support the implementation plan. Neither call independently authorizes implementation.

## 4. Retrieve missing context narrowly

When implementation exposes missing repository knowledge:

1. identify the missing fact, dependency, convention, or call path precisely;
2. issue a supplemental Claude request for only the additional exact ranges and relationships;
3. use a small JSON schema appropriate to that request;
4. read only the returned supplemental context.

Do not repeat the original reconnaissance or substitute broad local exploration. The initial request should include predictable wiring and test mechanics so supplemental calls cover only emergent dependencies.

## 5. Implement in Codex

Codex owns all implementation. Use the confirmed placement certificate and targeted evidence packet to edit code, add focused tests, update documentation, and preserve project boundaries. Establish the vertical semantic test before broad implementation or presentation polish. If the intended consumer cannot observe the new state, stop with a boundary-change request rather than completing a producer-only substitute.

Maintain an exact task-owned changed-file manifest. If a task modifies a file already dirty at baseline, identify the overlap explicitly for final review.

## 6. Run the Claude test gate

After implementation is coherent, invoke a new read-only Claude process outside the sandbox when tests or subprocesses require it. Pass:

- the accepted slice statement;
- the confirmed placement certificate and rejected insufficient substitutes;
- architectural invariants;
- exact task-owned changed-file manifest;
- baseline overlaps and pre-existing dirty state relevant to those files;
- concise completion notes;
- exact commands from the reconnaissance packet.

Claude must not edit files, write patches, regenerate assets, update snapshots, or otherwise mutate repository state beyond ordinary temporary test artifacts. Freshness checks must use check-only modes.

Run in this order, stopping at the first failure:

1. the vertical semantic test and a changed-file-boundary check against the placement certificate;
2. `git diff --check` and required prechecks/check-only freshness checks;
3. focused tests;
4. full suite.

Scope review to task-owned changes and only the callers or dependencies necessary to validate them. Do not review the entire dirty tree. Treat an unrelated modification as a finding only when evidence shows this task introduced it.

Use a JSON schema with mutually exclusive `failure` and `review` outcomes.

### Failure outcome

Stop before adversarial review. Return only:

- failing command and exit status;
- smallest likely root-failure set, not cascading failures;
- failing test names/files;
- concise error excerpts, never raw transcripts;
- whether the failure reproduces in isolation;
- environment-versus-product assessment, including restricted versus normal execution when relevant;
- whether task-owned files are in the failing call path;
- exact code/test ranges needed for diagnosis.

Codex evaluates and fixes the failure, using supplemental Claude reconnaissance if additional context is needed, then repeats the gate.

If the vertical proof fails because the accepted boundary has no valid consumer or owner, classify the plan as invalidated and return to placement instead of repairing indefinitely inside the wrong boundary.

### Passing outcome

Only after every required command passes, perform adversarial review. Prioritize:

- truth or provenance loss;
- broadened user ownership;
- fabricated confidence, success, or fallback states;
- accidental persistence, publication, or mutation;
- runtime-boundary leaks;
- stale or run-local identity misuse;
- unreachable integration paths;
- output contradicting internal decisions;
- weak failure-state explanation;
- missing or misleading tests;
- documentation claiming unreachable behavior;
- regressions and duplicated state vocabulary.

Return actionable findings ordered by severity. Each finding includes severity, files/symbols, verified evidence, violated invariant or expected behavior, and whether it was reproduced or inferred. Also return focused/full totals, duration, finding counts by severity, and whether the implementation is acceptable as-is. Never return raw exploration or test transcripts.

Codex decides which findings are valid and in scope, implements fixes, and repeats the complete Claude gate until tests pass and no blocking findings remain. Codex owns final acceptance.

## 7. Report workflow metrics

When available, report:

- reconnaissance, supplemental, and final-gate call counts;
- placement calls, candidate count, placement conflicts, placement reconsiderations, and targeted-reconnaissance calls;
- placement risk and whether the vertical semantic proof passed;
- successful, failed, timed-out, and infrastructure-failed calls;
- Claude wall-clock duration, cost, cache creation/read usage, output tokens, and thinking tokens;
- files, exact ranges, and approximate lines supplied to Codex;
- whether missing context blocked implementation;
- local repository exploration outside Claude-provided ranges;
- final test totals and review findings by severity.

Optimize for keeping high-volume, low-durability evidence in disposable Claude context while preserving Codex context for durable understanding, judgment, and implementation—not merely for the lowest combined token count.
