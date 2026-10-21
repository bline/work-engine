# Running on Claude Code (Anthropic models)

The role references are written in Codex terms. On Claude Code the roles,
packets, evidence rules and the retained-or-fresh decision are unchanged; the
mechanics map as below. Load this with your role reference when the session
is Claude Code.

| Codex | Claude Code | difference that matters |
| --- | --- | --- |
| `spawn_agent`, `fork_turns="none"` | `Agent` tool with a non-fork `subagent_type` (`general-purpose`, or a narrower type when it fits) | A `fork` inherits your whole context; it is a retained context, not a fresh one. |
| `followup_task` | `SendMessage` to the agent's ID | **Any** message to a finished agent resumes it with its full transcript. Every `SendMessage` that carries work is a retained dispatch and needs the pre-dispatch check. |
| `wait_agent` | nothing: the harness notifies on completion | Do not read or tail a running agent's output file, sleep, or list agents to check progress. Do independent work or end the turn's tool use until notified. |
| requested model and effort | `Agent` `model` (`opus`, `sonnet`, `haiku`, `fable`) | Set `effort` only when the user or an owning instruction asks. Observed model is `message.model` in the transcript. |
| rollout, `session_meta` lineage | `~/.claude/projects/<project>/<session>/subagents/agent-<id>.jsonl` and `agent-<id>.meta.json` | The ID is returned by the spawn call; `meta.json` records description and tool-use ID. |
| shared worktree | `Agent` `isolation: "worktree"` | Use it when parallel builders could touch the same files; the worktree is removed if unchanged. |

## Context check

```
python3 -I skills/manual-orchestration/scripts/child_context.py \
  claude --transcript <subagents dir>/agent-<id>.jsonl \
  [--next-task-tokens <estimate>] --out <assignment dir>/context-check-N.json
```

Context is input + cache-creation + cache-read tokens of the latest assistant
call. Transcripts record no window, so the levels are absolute and their basis
differs from Codex:

- **Trigger 150,000** carries over on a cost basis only: every call re-sends
  the whole context, so a retained 160K child pays about six times a fresh
  child's ~25K opening on each call (Codex measurement; the Claude opening is
  `not established`). H1's status is unchanged: a pilot hypothesis.
- **Band floor 202,533** is the lowest of 20 Claude Code auto-compactions on
  this machine as of 2026-10-21 (median 302,526, max 968,704, mixed windows).
  It is a lower bound, not the window.

On 2026-10-21 a coordinator sent a retained re-review to a Claude reviewer
reading 163,367 without running this check, because the skill had no Claude
reader yet. The re-review completed; by H1 it should have been a fresh reviewer
or a recorded overrule (continuity of the reviewer's own findings is a valid
reason).

## What does not carry over

- `work-engine-context-status` and `provision_context_status.sh` read Codex
  rollouts only; use the `claude` subcommand instead.
- Whether a subagent can itself launch agents depends on the tools its type
  is granted; a supervisor layer needs a type that includes `Agent`. Confirm
  before assigning a supervisor role; otherwise the coordinator supervises.
- The identity binding is the agent ID and transcript path from the spawn
  result, sent after launch by `SendMessage` as in [SKILL.md](../SKILL.md#launching-a-child).
- Repository instructions still apply: Claude subagents read `CLAUDE.md` and
  `AGENTS.md`, and the coordination board is shared with Codex sessions.

## Cost

`metrics/scripts/claude_transcript_usage.py` sums weighted tokens (uncached +
cached/10 + 5 x output, the baseline's unit) for one transcript. Account every
subagent transcript in the task, including the coordinator's own, before a
total; [measurement](measurement.md) holds the rules.
