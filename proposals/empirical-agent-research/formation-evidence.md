# Formation Evidence: Empirical Agent Research Foundations

## Evidence cutoff and classes

- Repository: `/home/bline/code/work-engine`
- Repository observation cutoff: 2026-08-23
- Installed Codex observed: `codex-cli 0.149.0`
- Evidence classes used below:
  - **User direction** records authority exercised in the formation session.
  - **Direct runtime observation** records inspected local behavior.
  - **Repository observation** records current artifacts and contracts.
  - **Inference** remains revisable interpretation.

## User direction

The user directed Work Engine to preserve discoveries before implementation,
including:

- future ability to reconstruct environment, capability exposure, procedure,
  authority, evidence, context, temporal behavior, outcomes, efficiency,
  stability, harness effects, composition, comparisons, and provenance;
- use of retained Codex and Claude evidence for retrospective research;
- runtime preservation of future provider sessions or traces;
- lossless compression when retaining transcripts or large trace payloads;
- code-change characterization as a control variable for Review Bench; and
- eventual analysis of which reviewer configurations perform well on which
  kinds of code changes.

This direction authorizes proposal formation and planning. It does not by
itself authorize sensitive trace collection, implementation, retention-policy
changes, benchmark admission, reviewer selection, or roadmap priority.

## Direct Codex observations

An isolated, ephemeral test outside Work Engine established that the installed
Codex build honors `CODEX_ROLLOUT_TRACE_ROOT` and supports
`codex debug trace-reduce`.

The trace bundle contained:

```text
manifest.json
trace.jsonl
payloads/*.json
```

The reducer successfully produced `state.json`. The raw trace recorded rollout,
thread, turn, inference, code-cell, response, token, and protocol events with
payload identities and timestamps.

The first captured inference request represented the effective input as ordered
`input` items rather than top-level `instructions` and `tools` keys. It included
seven developer/user message items and one `additional_tools` item. The directly
exposed tool namespaces were:

- `functions`: `exec`, `wait`, and `request_user_input`;
- `collaboration`: six multi-agent controls.

Their descriptions and parameter schemas were present in the inference payload.
The directly exposed `functions.exec` description explained nested-tool
discovery and contained schemas for ordinary nested tools. A second isolated
probe returned 176 runtime `ALL_TOOLS` entries without invoking a nested tool.
The trace preserved the code-cell call and result and the next inference input
in which the discovered inventory became visible.

This directly distinguishes:

```text
registered nested capability
        !=
directly model-visible capability
        !=
capability made visible through discovery
        !=
invoked capability
```

The second provider request used `previous_response_id` and an incremental
tool-result input rather than duplicating the first request. Exact logical
reconstruction therefore requires joining the traced inference chain. Raw
payloads remain authoritative; reduced state is derived.

## Existing local session evidence

The inspected host contained:

- 206 ordinary Codex rollout JSONL files, approximately 2.9 GB in total;
- 90 ordinary Codex rollouts associated with the Work Engine working directory;
- 14 Claude Work Engine session JSONL files, approximately 8.5 MB in total.

The 90 Work Engine Codex rollouts all contained session metadata, turn context,
world state, token records, custom tool calls, and developer messages. They did
not contain explicit serialized `input_schema` or `tool_schema` fields. Eight
contained explicit compaction records.

The inspected Claude files contained messages, model identity, session and
working-directory metadata, tool-use blocks, and tool results. No distinct
system-instruction field or supported exact per-inference tool manifest was
observed. Claude transcript format and retention remain provider implementation
details rather than a Work Engine contract.

An identifier match alone does not prove that a session performed a historical
run; later sessions may quote earlier run IDs. Historical reconstruction
therefore needs confidence-qualified correlation against timestamps, Git
subjects, checkpoints, prompts, findings, and provider identities.

## Repository observations

- Slice receipts intentionally exclude prompts, transcripts, raw tool output,
  source excerpts, raw validation output, and debug logs.
- Current telemetry ingress can bind ordinary Codex parent and child rollout
  paths and derive token, turn, timing, and terminal-state evidence.
- Current metrics distinguish configured from observed execution, but observed
  model, token, session, accepted-plan, and continuation fields are not
  consistently populated.
- Role-owned durable operational state explicitly excludes full transcript,
  hidden-reasoning, raw source, diff, and test-log persistence. It owns recovery
  consequences, not forensic execution evidence.
- Revision-bound review artifacts own exact subjects, episodes, findings,
  outcomes, conflicts, synthesis, and applicability, not raw provider traces.
- Claim-lineage dogfood preserves bounded claim identity and provenance but does
  not yet provide automatic discovery, transitive propagation, continuous
  monitoring, or universal workflow linkage.
- Slice checkpoints already provide strong immutable subject identities through
  commit, tree, patch, manifest, gate, plan, scope, and attempt bindings.

## Formation inferences

- For future Codex runs, first-party rollout traces can own model-facing runtime
  evidence; Work Engine needs only durable capture, security, integrity, and
  semantic lineage bindings rather than a duplicate inference telemetry system.
- For Claude, append-safe snapshots of ordinary session evidence remain useful
  until equivalent model-facing trace evidence exists.
- Lossless chunk compression belongs at the archive boundary. Summaries and
  reduced state are derived projections and cannot replace raw bytes.
- Code-change characterization is necessary but insufficient to explain review
  performance. Task/problem evidence and reviewer evidence visibility remain
  separate explanatory objects.
- Review Bench should use change dimensions for stratification and conditional
  capability estimates, not as benchmark truth or a universal difficulty score.

## Limitations and reopening evidence

- The isolated Codex probes did not exercise compaction, nested MCP invocation,
  browser evidence, or a child-agent trace inside the new bundle format.
- No supported exact Claude inference-request capture was demonstrated.
- Storage cost, compression ratio, privacy requirements, encryption boundary,
  and retention policy have not been exercised.
- Historical correlation quality has not been measured over all candidate runs.
- Static before/after graph analysis at immutable historical revisions has not
  been proven through the current Codebase Memory adapter.
