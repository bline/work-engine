---
name: claude-recon-implementation
description: Use native Claude Code for shallow architectural placement alternatives, fresh targeted repository reconnaissance, failure diagnosis, and fresh-entry adversarial review with retained remediation context while preserving Codex context for judgment and implementation. Supports provenance-bearing Anthropic or OpenRouter inference routes without replacing the Claude Code harness. Use for non-trivial repository work, especially when a roadmap objective could plausibly belong in multiple subsystems or cross authoring, persistence, runtime, or presentation boundaries.
---

# Claude Recon Implementation

Use native Claude Code as a forked external process for optional high-volume disposable
repository retrieval, fresh placement falsification, failure diagnosis, and
adversarial review. Codex owns task interpretation, architectural judgment,
implementation, durable understanding, and final acceptance. In Work Engine
config version 2, repository retrieval and independent review are separate
roles; selecting this skill for one role never implicitly selects or satisfies
the other. Direct Codebase Memory through $repo-search is the default retrieval
path.

Claude Code is the harness identity, not merely a convenient client for a
Claude-family model. When Anthropic subscription capacity is unavailable, the
same native `claude` executable may use OpenRouter's Anthropic-compatible
endpoint; do not substitute Codex or a generic OpenRouter agent loop and still
call the result a Claude review. Before invoking either transport, read
[references/native-claude-transport.md](references/native-claude-transport.md)
and use its deterministic launcher. Keep gateway, exact requested model,
Claude Code version, session continuity, and observed-versus-requested upstream
provider distinct in provenance.

OpenRouter Batch is a separate experimental execution mode, not an automatic
quota-failover route. Before using it, read
[references/native-claude-batch-transport.md](references/native-claude-batch-transport.md)
and preserve its harness-configuration and compatibility limitations in the
review receipt.

For research or experimental execution, require the transport reference's
fresh Anthropic-1P key-guardrail attestation. An exact Claude model slug without
an enforced upstream allowlist is insufficient because OpenRouter may route
that model through another provider implementation. Ordinary review may use an
unpinned route only when its receipt and resulting evidence class disclose
that limitation.

Provision Codebase Memory and bounded read-only filesystem tools together for
repository-aware Claude calls. Codebase Memory remains the primary path for
code structure, ownership, and relationships. Filesystem access exists for
claims the index cannot establish: exact literals and non-code content,
reported coverage gaps, generated or unindexed artifacts, and required host
state outside the repository. Its availability is not evidence that it was
used.

Use `Read`, `Glob`, and `Grep` only against the smallest named paths needed for
those claims. Add an external directory with `--add-dir` only when the objective
requires evidence there; name the narrow artifact root rather than a home or
workspace root. Never enable filesystem mutation tools for reconnaissance or
review. Record actual indexed and direct-source evidence modes separately,
including the reason for a transition. Using read-only filesystem evidence
inside the configured Claude process does not change its provider identity and
does not select the distinct `claude-filesystem` repository provider.

This workflow does not invoke Agents API delegation. Do not use `implementation-orchestrator` unless the user separately and explicitly requests agents, subagents, delegation, or multi-agent orchestration.

Treat the diagram below as the high-assurance route, not a universal sequence. Preserve hard invariants—read-only Claude calls, Codex ownership of judgment and mutation, truthful provenance, user-work preservation, and configured validation—while selecting reconnaissance and review stages in proportion to placement ambiguity, consequence, and independence needs.

```text
Claude compact placement alternatives
→ Codex placement decision
→ fresh Claude compact targeted reconnaissance and falsification
→ Codex plan and implementation
→ deterministic test gate
   → Claude failure diagnosis when needed
   → or fresh-entry Claude adversarial review
→ Codex fixes
→ same reviewer evaluates the delta and prior findings
→ repeat as useful until accepted
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

Prefer the native-Claude transport launcher. The command below shows the
underlying Claude invocation; wrap it as specified in the transport reference:

```bash
claude -p \
  --effort medium \
  --model sonnet \
  --no-session-persistence \
  --tools "mcp__codebase-memory-mcp,Read,Glob,Grep" \
  --output-format json \
  --json-schema '<placement-schema>' \
  --dangerously-skip-permissions \
  '<bounded placement prompt>'
```

When the placement premise depends on a known external artifact, add its
smallest stable containing directory to the same invocation:

```bash
  --add-dir '<bounded-external-artifact-root>'
```

Tell Claude in the bounded prompt which certificate clause requires that path,
that indexed structure remains primary for repository code understanding, and
that filesystem observation must not broaden beyond the named artifact. Do not
wait for an index-only call to fail before making an already-required external
artifact readable.

If Claude fails before returning repository evidence, treat that as infrastructure failure. Inspect the execution conditions before retrying; do not repeatedly issue the same invocation. Prefer correct API/config access outside the sandbox, then retry once. On a direct low-risk route, return the failure to the builder for a recorded route decision: builder-direct observation may replace a defaulted provider only when no explicit independence requirement applies and the same acceptance condition remains provable. Never silently substitute for an explicitly selected provider or high-assurance falsifier.

A recognized quota failure may activate the configured OpenRouter route under
the transport contract. That route is still native Claude Code, but it is not
the same gateway or necessarily the same upstream inference provider. Any
other failure returns without paid failover. A retained review may cross the
gateway only through a verified same-session resume or a truthful authorized
reviewer replacement; disposable replay rules do not establish retained
continuity.

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
Provision the same read-only evidence capabilities as the placement call. If a
certificate clause depends on an external or unindexed artifact, independently
observe that bounded artifact in this fresh process instead of relying only on
the earlier call's interpretation.

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

After required deterministic commands pass, launch one fresh read-only Claude
reviewer for each perspective selected by the role that owns review selection.
In a slice-supervisor campaign, execute only the supervisor's exact selection
plan; medium/high risk, consequential behavior, cross-boundary state,
persistence/security/ownership effects, and material uncertainty are selection
inputs, not permission for the builder or this adapter to add a reviewer. In
standalone use, the caller retains selection judgment under its governing
contract. This initial review entry must not inherit a retrieval, diagnosis,
placement, or builder session. Pass the accepted slice and placement
certificate, invariants, task-owned manifest and overlaps, concise completion
notes, and compact deterministic result. Claude must not edit files, write
patches, rerun tests, regenerate assets, update snapshots, or otherwise mutate
repository state.

For a supervisor-selected semantic specialist, also pass the exact selection,
immutable subject binding, and complete repository-local specialist contract.
Require the provider result to preserve that contract's applicability,
findings, limitations, and advisory authority rather than collapsing it into
this adapter's generic implementation verdict. A selected
`agent-instruction-review` reads its `SKILL.md` and finding contract, returns a
revision-bound `applicable` or `omitted` result, and remains read-only. Claude is
the independent-review provider in that execution; the specialist still owns
the diagnosis and finding shape.

### Retain the reviewer through remediation

The initial independent-review call is fresh once, not fresh on every repair.
Give it a known UUID with `--session-id`, omit `--no-session-persistence`, and
verify that the returned JSON `session_id` matches. When active-slice recovery
is configured, publish that UUID as
`actor_binding.runtime_session_id` before crossing the provider boundary.
Disposable placement, reconnaissance, supplemental retrieval, and failure
diagnosis calls continue to use `--no-session-persistence`; never reuse one of
those contexts as the reviewer.

When authority-bound independent-review state is available, make it part of the
initial reviewer call rather than adding it after findings exist. Before
provider entry, publish the exact review subject, reviewer UUID, writer
generation, and episode authority manifest. Launch the Work Engine MCP with
that manifest and expose its narrow review-state tools alongside the reviewer's
read-only evidence tools. The reviewer begins its episode, performs the review,
records its attributed initial result, and reads the resulting state before it
returns. The coordinator verifies the returned provider session ID; the
reviewer is not required to self-observe runtime identity through a tool that
does not expose it.

On remediation, resume the same provider session with the same episode-bound
state tools. Give it the new exact subject and bounded delta; it records the
remediation subject, re-evaluates its own findings, and publishes that
re-evaluation before returning. If state publication fails, retain the exact
structured reviewer result, report the operational state as incomplete, and
repair or reconstruct truthfully. Never report an attempted transition as
durable. This standard path applies only to retained review episodes; do not
give mutable episode state to disposable placement, reconnaissance, diagnosis,
or falsification calls.

After Codex applies a valid finding, invoke the same reviewer with
`--resume <session-id>`. Supply the exact new subject or delta, affected gate
evidence, prior finding identities, and remediation summary. Do not resend the
whole initial research merely to reconstruct context the reviewer already owns.
Describe the pass as a continuation of the original isolated review episode,
not as another fresh or independently established review.

If supervisor context is replaced while the provider session remains
available, recover the pending review obligation and runtime session ID through
`resume_active_slice.py`, then resume that session. If the provider session is
lost, reconstruct only from durable subject, findings, and gate evidence and
record a replacement or reconstructed reviewer truthfully; do not claim
retained context.

Reset to a genuinely fresh reviewer only when model judgment finds that the
existing context is no longer fit—for example after a material architectural or
placement change, a changed review premise, degraded or oversized context, or a
renewed independence need. Record the reset reason and preserve still-applicable
findings. A reset is not the default response to an ordinary fix.

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
- attempt outcomes and available token, cost, and time measurements partitioned
  by the evidence capability actually used;
- provider failure primary causes and any transition from indexed structural
  evidence to direct source observation, including whether the reason was index
  availability, coverage, graph ambiguity, or provider failure;
- Claude wall-clock duration, cost, cache creation/read usage, output tokens, and thinking tokens;
- Claude Code version, gateway, exact requested model, transport attempts,
  quota-failover outcome, and whether the upstream provider was observed;
- files, exact ranges, and approximate lines supplied to Codex;
- whether missing context blocked implementation;
- local repository exploration outside Claude-provided ranges;
- selected workflow route, route revisions, preserved evidence, and stale decisions;
- validation and review stages selected or omitted with their risk rationale;
- final test totals and review findings by severity.

Optimize for keeping high-volume, low-durability evidence in disposable Claude context while preserving Codex context for durable understanding, judgment, and implementation—not merely for the lowest combined token count.
