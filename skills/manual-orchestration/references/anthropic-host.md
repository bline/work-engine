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
| shared worktree | `Agent` `isolation: "worktree"` | Use it when parallel builders could touch the same files; the worktree is removed if unchanged. **Check the base at entry:** two harness worktrees on 2026-10-22 started at `a69efc44`, 278 commits behind HEAD. The packet names the required base; the worker runs `git rev-parse HEAD` and `git status --short`, resets its own clean branch to the base if they differ, records it, and stops if the tree is dirty. |

## Models and review tiers

Defaults for this repository, each with its evidence:

| role | model, effort | basis |
| --- | --- | --- |
| builder with a fully specified packet | `sonnet`, high | Matched Opus on gates and defect incidence at 38% of the cost and half the time (`metrics/probes/claude-s2-builder-20261020-protocol.md:467-560`, n=1). Both carried the same defect class past every gate, so review is still required. |
| planner, verifier, tier-1 reviewer, assessor | `opus`, high | Where errors propagate; no matched Sonnet review comparison yet. |
| tier-2 reviewer | `fable`, fresh context | Scarce: one contract review cost 2% of the weekly Fable quota (~362K weighted, n=1). Use only for a shared contract, final acceptance of a workstream, or an unresolved P1 between builder and tier 1. Give it a small input (contract, diff, tier-1 findings, with paths) and never a retained follow-up. |

Search `metrics/probes` for newer evidence before overriding these.

## Context check

```
python3 -I skills/manual-orchestration/scripts/child_context.py \
  claude --transcript <subagents dir>/agent-<id>.jsonl \
  [--next-task-tokens <estimate>] --out <assignment dir>/context-check-N.json
```

The pinned CLI gives the same reading plus accounting:
`<pinned binary> claude --transcript <path> [--json]` for one transcript, and
`<pinned binary> claude-agents --session <id> --with-usage [--json]` for every
subagent of a session (counts each `message.id` once with its maximum output,
and excludes a fork's inherited parent call).

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

## Worktree mechanics

- Shells inside an isolated worktree have refused compound commands that mix
  `git` with other programs or `export` variables; run `git` and `cargo` as
  separate plain commands with inline variables (`CARGO_TARGET_DIR=... cargo ...`).
- A successor launched to continue in an existing worktree is not isolated
  itself: give it the worktree path, and have it use absolute paths,
  `git -C <worktree>` and `--manifest-path`, never the main checkout.
- `SendMessage` to a running agent queues until its next tool round; to a
  finished agent it resumes the full transcript. Use it for bindings and scope
  additions, and count it as a retained dispatch when it carries work.

## What does not carry over

- The Codex `--session` and `agents` modes read rollouts only; use the `claude`
  and `claude-agents` modes, or `child_context.py claude`, for transcripts.
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
