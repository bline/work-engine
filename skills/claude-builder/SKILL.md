---
name: claude-builder
description: Let Codex delegate a bounded engineering task to a retained native Claude Code builder, collect implementation and validation, and coordinate a separate Astra review. Uses existing manual orchestration, context checks, and handoffs. Use when Claude is selected to implement, rather than only investigate or review.
---

# Claude Builder

Codex coordinates one coherent engineering assignment; native Claude Code owns
repository understanding, implementation, tests, and repairs within it. Give
Claude the same relevant skills and engineering capabilities available to a Sol
builder. Its model identity changes the execution adapter, not the objective,
acceptance conditions, or access to the existing orchestration machinery.

The caller owns scope, model selection, review selection, result collection,
integration, and final acceptance. Claude's completion report supplies evidence;
it does not authorize publication or accept its own work.

## Assignment and machinery

Use the repository's [manual-orchestration](../manual-orchestration/SKILL.md)
contract: the Codex caller loads its coordinator or supervisor references; the
Claude builder loads the worker references and
[Anthropic host mapping](../manual-orchestration/references/anthropic-host.md).
If the packet assigns Claude supervision or decomposition, load that role
instead and provide the needed Agent tools. Tool availability alone does not
assign additional work. Keep the caller responsible for collecting the native
process and any descendants before closing the assignment.

Bind the authoritative repository root and instruction paths in the packet.
Read dependencies from that repository, including when this entrypoint is
installed elsewhere: an older user-scope skill copy must not silently replace
the assigned instructions. Include the accepted outcome, authority, exact
workspace/base and dirty baseline, owned write scope, bounded read set, checks,
settled facts, output/continuation paths, and collection owner. Preserve other
actors' work; a shared workspace is not exclusive ownership.

Keep implementation choices with the builder. Existing repo-search, graph
coverage, deterministic validation, context observation, and Wind Walker
contracts remain applicable. Do not impose the reconnaissance/review skill's
read-only tool profile on this implementation role. Read
[native session controls](references/native-session.md) before launching or
resuming Claude; it explains transport, permissions, identity, and process
collection without introducing another runtime or handoff schema.

## Model choice

Start new assignments with native `--model claude-sonnet-5-5 --effort high`
unless the task selects otherwise. This is the user's experimental starting
choice, not a measured quality or cost guarantee; `medium` remains selectable.
The caller may select
`claude-opus-5-5` when unresolved reasoning or task complexity warrants it;
effort is independently selectable. The builder can recommend a change, and
the caller records the choice and reason in the assignment before dispatch.
An explicit task/campaign model constraint takes precedence over this default.

Use the exact configured model identifier. Verify availability through the
installed runtime and preserve requested versus observed identity. A moving
`sonnet` or `opus` alias is not proof that the requested version ran. Use the
[exact-model guard](references/native-session.md#exact-model-guard) on each
builder turn: an automatic substitution would invalidate the selected model's
execution evidence. A refusal or substitution fails that turn; the launcher
does not choose a replacement. A provider classifier flag alone establishes
neither that the task is prohibited nor that a successor is prohibited. Assess
the actual requested work under the applicable rules. For otherwise authorized
work, the caller may select a successor under existing user and model guidance.
The user's `gpt-6-sol`/`medium` choice permits that successor after a Claude
refusal without another per-refusal approval. Reconcile the failed turn and
record the provider/model transition; do not require a separate assurance of
benign intent solely because the previous provider refused. Read
[continuing after a failed turn](references/native-session.md#continuing-after-a-failed-turn)
when another builder may carry the work forward. Capacity,
permission, or transport failures do not establish that a stronger model is
needed or authorize an unrecorded fallback. See the repository's
[model-selection guidance](../../docs/model-selection.md) for decision criteria.

## Retain the builder

Launch with a predeclared UUID and persistence enabled. Bind that UUID to the
observed result and local transcript. Resume that exact session for subsequent
phases and remediation while its context remains useful. A print-mode process
ending is a turn boundary, not retirement of the builder.

Keep session ID, transcript, workspace, transport, requested/observed model,
instruction source, process owner, and continuation reference in the existing
assignment record. Return/output and transport files are evidence pointers,
not substitute workflow state. Verify identity on every turn: a returned ID
mismatch leaves continuity unestablished, even if the text reports success.

Use manual orchestration's Claude context check before substantial retained
dispatch and at the builder's own phase boundaries. Retention is the default,
not a reason to ignore context pressure. A justified replacement uses its
handoff and Wind Walker contracts, preserves unresolved work and prior identity,
and is recorded as a new session. Do not relabel reconstruction as uninterrupted
context. A `handoff_ready` result is unfinished work, not acceptance.

A model change at a completed turn may retain the same transcript using an
explicit `--resume` with the selected model and effort. Verify the returned
session and observed model; record the transition rather than claiming an
unchanged builder configuration. If the runtime cannot support that continuation,
use a truthful handoff. Never run predecessor and successor concurrently over
the same mutable scope.

## Review and completion

Pair this builder with a separate `gpt-6-astra` reviewer, normally `xhigh` under
the repository's review guidance. Read
[Astra review](references/astra-review.md) for the caller's launch, isolation,
retained remediation, and evidence boundaries. The caller owns this Codex tool
boundary; Claude can request review but need not possess Codex's spawn tools.

Send valid findings to the retained builder, or its authorized successor after
a failure, collect its repairs and affected validation, then return the exact
delta to the retained Astra reviewer.
Each role keeps its own context and authority. A reviewer does not become a
builder by writing the fixes itself.

Collect the manual worker result: exact task/subject, attributed changes,
commands and actual check outcomes, evidence references, limitations, unfinished
obligations, continuation, and child/process disposition. Preserve native usage
and unavailable measurements as such; process exit or a successful transport
receipt alone is not engineering success. The caller verifies the final subject
and required review before accepting the result.

This skill is usable under manual orchestration. It does not by itself register
a Claude adapter in a configured Slice Builder campaign. Such integration must
preserve that campaign's configuration, phase acceptance, gate, review-state,
and terminal receipt contracts; a manual result cannot stand in for them.
