# Native Claude session controls

Use native Claude Code with its normal skills, project instructions, tools,
MCP connections, compaction, and persisted transcript. The existing
`skills/claude-recon-implementation/scripts/claude_transport.py` can launch a
writable command; this adapter borrows its transport mechanics, not the
reconnaissance skill's read-only role. Read its
[transport reference](../../claude-recon-implementation/references/native-claude-transport.md)
for the selected gateway. Resolve paths from the authoritative repository root.

## Entry and continuation

Run from the assigned workspace. Prepare the prompt as an exact UTF-8 file,
hash it, and allocate a UUID and distinct evidence paths before entry. The
following is a launch shape; substitute the packet's paths, configured model,
effort, and authorized permission/tool settings:

```bash
python3 <repository>/skills/claude-recon-implementation/scripts/claude_transport.py \
  --transport anthropic --continuity retained --exact-model-guard \
  --receipt <assignment>/turn-1.transport.json \
  --stdin-file <assignment>/turn-1.prompt.txt \
  --stdin-sha256 <prompt-sha256> \
  -- claude -p --session-id <builder-uuid> \
  --model claude-sonnet-5-5 --effort high \
  --tools default --permission-mode acceptEdits \
  --allowedTools Bash --output-format stream-json --verbose \
  --include-partial-messages \
  > <assignment>/turn-1.stream.jsonl
```

The example grants shell execution as well as edits; use it only when the
assignment authorizes those capabilities. Shell access is not path confinement.
An isolated worktree separates changes but is not a security sandbox. Keep the
accepted mutation boundary in the packet and select actual enforcement when
the task requires confinement. Do not bypass permissions to resolve a failed
test or missing tool. Permission denials are execution limitations to reconcile,
not evidence that the code is defective.

The output is JSONL: the native `result` is one event in the stream. Read the
transport receipt's `model_guard` and launcher exit as well as that event; a
native success event cannot override a guard failure. Partial messages expose
the serving model at message start, before the final assistant message.

For the next turn use new prompt/stream/transport paths and replace
`--session-id <builder-uuid>` with `--resume <verified-builder-uuid>`.
Supply the configured model, effort, and capabilities again. Preserve the
working directory and transcript store. Explicitly send changed instructions
in the resumed task and require them to be read: the installed runtime may
snapshot its system prompt, so changed launch text alone need not refresh it.

These flags have materially different meanings:

| Control | Consequence |
| --- | --- |
| `--session-id UUID` | Start a persisted identity, then verify it from the result/transcript. |
| `--resume UUID` | Continue the specifically bound builder. |
| `--continue` | Picks the directory's latest session; unsafe for exact identity among concurrent sessions. |
| `--fork-session` | Creates another identity; this is replacement/fork provenance, not same-session continuation. |
| `--no-session-persistence` | Removes resumability; incompatible with this retained builder. |

Use exact resume for retained work; omit the latter three controls. Keep one
active invocation per session. Concurrent resumes can race both transcript and
workspace ownership.

## Capabilities and identity

Keep native built-in tools available for implementation and validation, and
load the task's configured MCP tools and relevant skills. Avoid `--bare`,
`--safe-mode`, `--disable-slash-commands`, or an inherited reviewer allowlist
unless the task deliberately supplies equivalent required capabilities. If
using strict MCP configuration, include every assigned server and verify its
availability; a configuration file is not evidence of a working connection.

Observe Claude's returned `session_id`, actual assistant model records (or
model-usage metadata), tool availability and permission denials. Verify the
local transcript path and its session identity; do not infer identity from a
filename or inherited `CODEX_THREAD_ID`. A native external process is not a
Codex `spawn_agent` child, so Codex rollout lineage does not bind it.

Deliver the verified binding, parent/collection owner, continuation path, and
context observer to the builder at the next boundary. Use the existing Claude
reader against that actual transcript:

```bash
python3 -I <repository>/skills/manual-orchestration/scripts/child_context.py \
  claude --transcript <verified-transcript> \
  --out <assignment>/context-check-2.json
```

This route needs no new Rust build or Codex rollout reader. Record observation,
retain/replace decision, and reason in the existing assignment record. Unknown
telemetry remains unknown. Claude uses the same reader for its phase checks.
Its native subagents follow the Anthropic host mapping and handoff contract;
the caller still collects their disposition through the builder.

## Exact-model guard

The launch shape above enables `--exact-model-guard` with native
`--output-format stream-json --verbose --include-partial-messages`. Keep these
controls on resumed turns with the selected raw model identifier. The launcher
merges scoped `switchModelsOnFlag: false` and singleton `availableModels` into the one
`--settings` file it passes, keeps any settings the assignment already passes,
watches assistant message models and refusal/fallback signals while the child
runs, and stops the owned process tree on a violation. The settings are a
request to Claude Code; the launcher does not observe it enforcing them, and the
receipt's settings digests bind only that file, not the merged policy Claude Code
applied. Details and limits are in the
[transport reference](../../claude-recon-implementation/references/native-claude-transport.md#exact-model-guard-opt-in).

Read the result from `model_guard` in the receipt, not from the exit code alone:

- `verified_exact` plus exit 0 means every observed assistant model was exact and
  the native result succeeded. It does not prove no unobserved activity.
- `violation` (exit 41) covers a different model, a refusal or a fallback. It is
  terminal for this invocation; there is no automatic retry or substitution.
  The turn's acceptance is invalid, and tool effects before detection are
  `not_established`. Preserve the failure and use the continuation boundary below
  before assigning further work.
- `incomplete_evidence` (42/124/128+n) and `unavailable` (43) are not successes.
  This includes a cancellation or timeout received at any point before the final
  receipt is written (even after the native process exited), and a receipt the
  launcher could not write (exit 42; an unwritable in-flight receipt also stops
  the child). Resume only after reconciliation, as an ordinary recovery decision.
- A `<synthetic>` assistant notice (such as an API connection error) is a harness
  message, neither a model switch nor an observation. Alone, with no exact
  assistant message, the run is `incomplete_evidence`. After exact messages and
  with a successful result it does not change the verdict: `verified_exact`,
  exit 0, with `observations.synthetic_assistant_messages` and
  `synthetic_error_categories` recording it. The guard speaks to model identity,
  not to whether the turn's work is complete, so read those counts when deciding
  whether to accept or resume.
- On `--resume`, `modelUsage` and `init` models are history and metadata; only
  assistant messages decide.

A resumed turn needs the same flags and a fresh receipt path. The exit-0
`verified_exact` receipt is evidence for that turn only.

## Continuing after a failed turn

A failed builder invocation does not by itself fail the slice objective. The
caller can select another builder, including a Codex worker through
[manual orchestration](../../manual-orchestration/SKILL.md), within the owning
assignment's authority. A fixed model qualification or campaign provider
constraint still belongs to its acceptance owner: replacement cannot satisfy
the original model-specific claim or silently amend it. Use existing selection
authority when granted; obtain a changed decision only when the owning contract
requires one. For otherwise authorized builder work, the user's selection of
`gpt-6-sol` at `medium` effort permits it as a successor after Claude refusal;
no per-refusal approval is needed. Record the selected model and effort without
attributing its work to the failed model qualification.

A provider refusal records that provider's intervention. Its category alone
does not establish the triggering operation, illicit intent, or whether the
requested work is prohibited. Assess the actual task and available evidence
under the applicable rules; neither a confirmed false-positive diagnosis nor a
fresh declaration of benign intent is a prerequisite for continuing otherwise
permitted work with the selected builder. If that assessment identifies a
concrete prohibited action or unresolved authorization boundary, explain that
specific limitation. Do not infer it merely from the classifier label. The
failed invocation stays stopped and attributable, and the successor retains its
own safety and permission controls.

Before a successor mutates the same scope, establish that the predecessor and
its owned processes have stopped and reconcile the workspace and any uncertain
tool effects. Preserve partial changes and other actors' work. Carry the failed
receipt/stream/transcript references, requested and observed identities,
reconciled subject, unresolved findings, required checks, successor selection
and reason, and collection owner in the existing assignment and handoff. Give
the successor distinct execution evidence paths so it cannot overwrite the
failed run or present reconstruction as uninterrupted context.

The successor repairs the remaining implementation under its own attribution.
Changing builders does not resolve correctness defects or waive validation and
review. Preserve the retained reviewer when its independence and useful context
remain valid. Under a configured slice campaign, use its provider/configuration
and receipt contracts; this manual adapter does not register a campaign builder
or turn a partial manual result into an accepted slice.

## Interruptions and collection

The unguarded transport launcher buffers process output until completion; the
exact-model guard forwards it live. Either way a host exec handle means
the caller still owns collection; wait on it without repeatedly reopening the
transcript. Use bounded tool waits that permit user steering.
Do not end the coordinating turn while required results remain unless a
verified resumption mechanism owns collection.

After interruption or transport failure, reconcile the native process and any
descendants, persisted transcript, current files, and continuation before
dispatching again. A timeout or missing result is not proof that no edits or
commands occurred. Stop owned processes or establish their completed state
before resuming; do not kill unrelated Claude sessions. Resume with a recovery
instruction grounded in the observed state instead of replaying the original
mutating task blindly.

Use the explicitly selected gateway. The default example has no paid fallback.
Do not enable automatic transport/model replay for a mutating builder: even a
quota failure can follow successful tool effects. A gateway change requires
the governing authorization and reconciled session/workspace state; preserve
gateway and model provenance separately from the local session identity.

Transport success establishes process delivery, not session continuity, tool
success, tests, or acceptance. Inspect the Claude result/error and permission
denials, then verify the required artifacts and actual checks. Keep the prompt,
result, transport receipt and relevant transcript references attributable to
each turn; retain secrets only in their existing credential owners.
