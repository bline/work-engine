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

When comparing OpenRouter realtime and batch reviews, read
[references/paired-openrouter-review-calibration.md](references/paired-openrouter-review-calibration.md).

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

Read [references/placement-alternatives.md](references/placement-alternatives.md) when placement is ambiguous and you map placement alternatives: it holds the placement call, the external-artifact option and the placement JSON schema.

Codex reads the returned placement ranges and records a provisional placement certificate:

```text
When <trigger> occurs, <producer> writes <state> owned by <boundary>.
<consumer> reads it and produces <semantic outcome>.
<downstream test> proves the consequence.
This is not satisfied by <plausible but insufficient substitute>.
```

Record the selected candidate, rejected alternatives and evidence, unresolved preconditions, and confidence. A value matching the first candidate is still Codex's decision, not Claude's. If evidence cannot distinguish the candidates, make at most one narrow request for the single discriminating fact or stop for architectural judgment. Never launch deep reconnaissance across every alternative.

### Claude call failure and quota failover

If Claude fails before returning repository evidence, treat that as infrastructure failure. Inspect the execution conditions before retrying; do not repeatedly issue the same invocation. Prefer correct API/config access outside the sandbox, then retry once. On a direct low-risk route, return the failure to the builder for a recorded route decision: builder-direct observation may replace a defaulted provider only when no explicit independence requirement applies and the same acceptance condition remains provable. Never silently substitute for an explicitly selected provider or high-assurance falsifier.

A recognized quota failure may activate the configured OpenRouter route under
the transport contract. That route is still native Claude Code, but it is not
the same gateway or necessarily the same upstream inference provider. Any
other failure returns without paid failover. A retained review may cross the
gateway only through a verified same-session resume or a truthful authorized
reviewer replacement; disposable replay rules do not establish retained
continuity.

## 3. Run targeted reconnaissance

Read [references/targeted-reconnaissance.md](references/targeted-reconnaissance.md) when you are about to run the targeted reconnaissance call (high-assurance or direct route): it holds the call's inputs and required JSON schema.

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

Read [references/review-episode.md](references/review-episode.md) when the supervisor selected a semantic specialist, when authority-bound review state is configured, when you resume the reviewer with episode-bound state tools, or when you recover a lost reviewer or replaced supervisor context.

Read [references/paired-calibration-campaign.md](references/paired-calibration-campaign.md) during an active paired OpenRouter calibration campaign, before launching either arm.

### Retain the reviewer through remediation

The initial independent-review call is fresh once, not fresh on every repair.
Give it a known UUID with `--session-id`, omit `--no-session-persistence`, and
verify that the returned JSON `session_id` matches. When active-slice recovery
is configured, publish that UUID as
`actor_binding.runtime_session_id` before crossing the provider boundary.
Disposable placement, reconnaissance, supplemental retrieval, and failure
diagnosis calls continue to use `--no-session-persistence`; never reuse one of
those contexts as the reviewer.

After Codex applies a valid finding, invoke the same reviewer with
`--resume <session-id>`. Supply the exact new subject or delta, affected gate
evidence, prior finding identities, and remediation summary. Do not resend the
whole initial research merely to reconstruct context the reviewer already owns.
Describe the pass as a continuation of the original isolated review episode,
not as another fresh or independently established review.

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

Read [references/workflow-metrics.md](references/workflow-metrics.md) when you report the workflow's metrics.

Optimize for keeping high-volume, low-durability evidence in disposable Claude context while preserving Codex context for durable understanding, judgment, and implementation—not merely for the lowest combined token count.
