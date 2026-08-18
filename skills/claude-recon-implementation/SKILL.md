---
name: claude-recon-implementation
description: Use Claude CLI for shallow architectural placement alternatives, fresh targeted repository reconnaissance, failure diagnosis, and adversarial review while preserving Codex context for judgment and implementation. Use for non-trivial repository work, especially when a roadmap objective could plausibly belong in multiple subsystems or cross authoring, persistence, runtime, or presentation boundaries.
---

# Claude Recon Implementation

Use Claude CLI as a forked external process for high-volume, disposable evidence gathering. Codex owns task interpretation, architectural judgment, implementation, durable understanding, and final acceptance.

This workflow does not invoke Agents API delegation. Do not use `implementation-orchestrator` unless the user separately and explicitly requests agents, subagents, delegation, or multi-agent orchestration.

Treat the diagram below as the high-assurance route, not a universal sequence. Preserve hard invariants—read-only Claude calls, Codex ownership of judgment and mutation, truthful provenance, user-work preservation, and configured validation—while selecting reconnaissance and review stages in proportion to placement ambiguity, consequence, and independence needs.

```text
Claude compact placement alternatives
→ Codex placement decision
→ fresh Claude compact targeted reconnaissance and falsification
→ Codex plan and implementation
→ deterministic test gate
   → Claude failure diagnosis when needed
   → or fresh Claude adversarial review
→ Codex fixes
→ repeat deterministic gate and review until accepted
```

For an obvious, local, reversible boundary with a known producer, consumer, and focused downstream proof, use a direct route: one compact targeted reconnaissance call, Codex judgment and implementation, then proportionate deterministic validation. Escalate to the high-assurance route when evidence reveals competing ownership, hidden consumers, lifecycle conflict, broad fan-out, consequential behavior, or medium/high placement risk. Record the route and why it fits.

## 1. Establish a baseline

Before implementation:

- determine the bounded slice from the request and durable project context;
- read mandatory repository instructions directly;
- record `git status --short` without loading broad diffs;
- note pre-existing changes overlapping likely task files;
- do not broadly search or read the repository yourself.

The baseline distinguishes task changes from the user's existing work. Track task-owned files throughout implementation, including new and already-dirty files.

## 2. Select the reconnaissance route

Assess placement ambiguity before invoking Claude. Use direct targeted reconnaissance only when one boundary is already supported by canonical context and the consequence can be bounded locally. Otherwise map placement alternatives shallowly as described below.

### Map placement alternatives shallowly

Before implementation reconnaissance, use one read-only Claude process to map architectural placement. This call is a routing pass, not an implementation survey. It must inspect canonical design/roadmap evidence, module ownership boundaries, and only the minimum symbols needed to distinguish plausible homes. Do not ask it to trace every candidate deeply.

Prefer:

```bash
claude -p \
  --effort medium \
  --no-session-persistence \
  --tools "mcp__codebase-memory-mcp" \
  --output-format json \
  --json-schema '<placement-schema>' \
  '<bounded placement prompt>'
```

If Claude fails before returning repository evidence, treat that as infrastructure failure. Inspect the execution conditions before retrying; do not repeatedly issue the same invocation. Prefer correct API/config access outside the sandbox, then retry once. On a direct low-risk route, return the failure to the builder for a recorded route decision: builder-direct observation may replace a defaulted provider only when no explicit independence requirement applies and the same acceptance condition remains provable. Never silently substitute for an explicitly selected provider or high-assurance falsifier.

Require a JSON schema with:

- `semantic_outcome`: the user-visible or runtime consequence the slice must make true, independent of implementation shape.
- `placement_candidates`: at most three compact candidate objects naming the boundary, owner, producer, consumer, lifecycle, evidence for and against, downstream proof, and insufficient substitute.
- `discriminators`: at most five facts or questions that distinguish candidates.
- `ranges`: at most eight minimum canonical-document or ownership ranges Codex must read to choose responsibly.
- `blockers`: at most three unresolved premise conflicts, preconditions, or decisions.
- `placement_risk`: `low`, `medium`, or `high`, with evidence.
- `statistics`: available call statistics.

Do not request or return a singular recommended slice, complete call paths, exact changed-file lists, or a full implementation plan in this round. Ranking candidates is allowed only when the evidence and uncertainty remain visible.

Codex reads the returned placement ranges and records a provisional placement certificate:

```text
When <trigger> occurs, <producer> writes <state> owned by <boundary>.
<consumer> reads it and produces <semantic outcome>.
<downstream test> proves the consequence.
This is not satisfied by <plausible but insufficient substitute>.
```

Record the selected candidate, rejected alternatives and evidence, unresolved preconditions, and confidence. A value matching the first candidate is still Codex's decision, not Claude's. If evidence cannot distinguish the candidates, make at most one narrow request for the single discriminating fact or stop for architectural judgment. Never launch deep reconnaissance across every alternative.

## 3. Run targeted reconnaissance

For the high-assurance route, after Codex provisionally selects a placement, invoke a new Claude process with no session persistence. Give it the original objective and work source, the placement candidates, the provisional certificate, rejected alternatives and reasons, and applicable repository instructions. Explicitly require it to assume the certificate may be wrong.

For the direct route, invoke one compact process with the objective, supported boundary, provisional certificate, and applicable instructions. Require observed evidence for every certificate clause and require it to report any competing owner, hidden consumer, lifecycle conflict, or broader consequence. Discovery of any such condition escalates the route before implementation.

This call explores only the selected boundary and the callers or dependencies necessary to prove or falsify it. Require a JSON schema with:

- `verdict`: `confirmed`, `conflict`, or `unresolved`.
- `failed_certificate_clause`: the failed clause for a conflict, otherwise null.
- `facts`: at most eight directly observed facts that prove or falsify the certificate, including premise conflicts rather than silently redefining the task.
- `ranges`: at most twelve minimum code, test, and canonical-document ranges with path, numeric start/end lines, and reason.
- `wiring`: at most five registration, call-path, lifecycle, test-bootstrap, or environmental facts Codex cannot cheaply derive from those ranges.
- `commands`: exact vertical, precheck, focused, full-suite, and check-only freshness commands; do not execute them during reconnaissance.
- `blockers`: at most three unresolved decisions or missing-context risks.
- `statistics`: available call statistics.

Codex owns the selected boundary, semantic-effect path, invariants, acceptance tests, changed-file boundary, vocabulary, deferred scope, and final plan. Derive them from the certificate and compact evidence packet; do not make Claude restate them.

Prefer narrow, cohesive ranges. Codex reads only those ranges initially and does not repeat Claude's exploration merely to confirm it. If the verdict is `conflict` or `unresolved`, stop before implementation and reconsider placement or request judgment. Do not reinterpret the intended outcome to make the selected boundary pass.

A conflict invalidates dependent placement decisions, not the objective or all gathered evidence. Preserve facts that remain applicable, record the failed certificate clause, revise the route or certificate, and rerun only the evidence stage needed for renewed acceptance. Request judgment only when bounded evidence cannot resolve the choice or the revision changes authority or a consequential user decision.

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

## 6. Diagnose gate failures and review passing work

The builder owns ordinary test execution through its deterministic gate. Claude must not run the vertical proof, boundary check, `git diff --check`, prechecks, freshness checks, focused tests, or full suite. The deterministic result is observed gate state; never replace it with model inference.

If that gate fails and Codex needs diagnostic help, invoke a read-only Claude process with only the failing check identity and exit status, bounded error excerpt, isolation result, task-owned files, accepted placement certificate, and likely diagnostic ranges. Require the smallest likely root-failure set, environment-versus-product assessment, whether task-owned files are in the call path, and exact additional ranges. Do not request adversarial review while deterministic checks are failing.

After required deterministic commands pass, invoke a new read-only Claude process for adversarial review when configured or warranted by medium/high risk, consequential behavior, cross-boundary state, persistence/security/ownership effects, or material uncertainty. Pass the accepted slice and placement certificate, invariants, task-owned manifest and overlaps, concise completion notes, and compact deterministic result. Claude must not edit files, write patches, rerun tests, regenerate assets, update snapshots, or otherwise mutate repository state.

Scope review to task-owned changes and only the callers or dependencies necessary to validate them. Treat an unrelated modification as a finding only when evidence shows this task introduced it. Prioritize:

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

Codex decides which findings are valid and in scope, implements fixes, and repeats affected deterministic checks. Repeat broad gates only when configured or when fixes affect their risk surface. If the vertical proof shows there is no valid consumer or owner, return to placement rather than repairing indefinitely inside the wrong boundary. Codex owns final acceptance.

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
- selected workflow route, route revisions, preserved evidence, and stale decisions;
- validation and review stages selected or omitted with their risk rationale;
- final test totals and review findings by severity.

Optimize for keeping high-volume, low-durability evidence in disposable Claude context while preserving Codex context for durable understanding, judgment, and implementation—not merely for the lowest combined token count.
