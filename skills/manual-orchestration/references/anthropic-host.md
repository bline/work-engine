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

Model, effort and review-tier choices for this host live in
[model selection](../../../docs/model-selection.md) (the Claude-host rows). Pass the
choice through `Agent` `model` as above. That parameter takes an alias, so record
the observed exact identifier from `message.model`.

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

## Root transfer on Claude Code

The procedure is in [context boundary](context-boundary.md#roots). Claude
specifics:

- The user starts the successor: a new `claude` session in the repository
  directory. The predecessor cannot do it.
- The predecessor's subagents are not reachable from the successor, but their
  transcripts (`~/.claude/projects/<project>/<old session>/subagents/`) and every
  file they wrote stay readable by path, so handoffs and evidence carry over.
- Hand the user a prompt of this shape, filled in:

```
Act as coordinator under $manual-orchestration (read skills/manual-orchestration/SKILL.md
and references/coordinator.md by repository path). Continue from the root handoff
<path>, sha256 <digest>, role <role>: verify the digest and run the board's
handoff-status for this role and digest with your own session ID. If superseded,
already adopted, conflicting, or unregistered, reconcile its returned continuation
references and durable authority before adopting or dispatching. Otherwise read
it and the assignment record it names. Use the board's adopt command with your
own session ID, role <role>, predecessor <predecessor-session-id>, and this path
and digest. If adoption succeeds, acknowledge ownership in the assignment record,
re-claim <resources>, post a transition message, then take the next actions it lists.
```

- Bind the successor's identity by reading its own transcript path
  (`~/.claude/projects/<project>/<new session id>.jsonl`) and record it.

### Finding and retiring native sessions

Claude Code 2.1.310 supports `--name 'Readable task name | continuation ID'`,
`--resume ID_OR_NAME`, and `agents --json` (`--all` includes completed background
sessions; `--cwd` filters by directory). Use the exact verified ID when resuming;
names need not be unique. A reboot does not make the old assignment current.
The board's published handoff lineage supplies the continuation reference.

`stop ID` stops a background session and retains its conversation. The host
adapter uses this for retirement; interactive sessions are not thereby proven
stopped. `/rename <name>` renames the current conversation and is also documented
for `-p`; use an explicit name to avoid generating one from history. External
renaming of an existing session through the adapter has not been probed.
`rm` can remove a worktree as well as a background-session listing, so it is not
mapped to recoverable archival. These controls were checked with installed
CLI help and the [CLI reference](https://code.claude.com/docs/en/cli-reference);
the [commands reference](https://code.claude.com/docs/en/commands) covers `/rename`.
they do not establish model identity, task completion, or transfer authority.

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
