# pi-subagents children do not inherit the parent's `--api-key` override

Observed 2026-11-16 by Pi session `01a1e2a4-f353-743a-ba23-f42394958125`
(dev-vertical). Dated observation, not a fix and not an upstream report.
Component: `pi-subagents` 0.88.1 (`~/.pi/agent/npm/node_modules/pi-subagents`)
running in Pi `pi-coding-agent` snap revision 91.

## What happened

- The operator launched the parent session on API billing with
  `pi --provider anthropic --model claude-opus-5-5 --api-key "$ANTHROPIC_API_KEY" --session ...`.
  `~/.pi/agent/auth.json` also holds an `anthropic` **OAuth** credential, and
  `ANTHROPIC_API_KEY` is set in the environment.
- A background subagent was launched with `model: anthropic/claude-fable-5-2:high`
  (run `10243313-9046-492d-b34d-76412087d665`, the piece-(b) Fable review).
- All four of its requests returned `429 rate_limit_error` ("would exceed your
  account's rate limit"), with zero tokens and $0 cost recorded. The parent's
  API-key requests were unaffected.

## Why (from source)

- Pi documents `--api-key` as a "non-persistent API-key override"
  (`docs/cli.md`). It lives only in the process that received it.
- The child builds its own model runtime from default credentials and passes
  no parent credential:
  `src/runs/shared/child-session.js:412-414` (`pi.ModelRuntime.create()` or the
  shared default runtime) and `:548-559` (`pi.createAgentSession({ modelRuntime, model, ... })`).
  Nothing in the child launch carries the parent's API-key override.
- The child therefore resolves `anthropic` from `auth.json`/environment. The
  429s are consistent with it using the stored OAuth credential, whose limits
  and billing differ from the API key (board seq 5786: Pi + Claude via OAuth
  bills separately from the Claude subscription quota). Not verified: which of
  the OAuth credential and `ANTHROPIC_API_KEY` Pi prefers when both exist, and
  which credential the failed requests actually carried.

## Why it matters

The same `provider/model` id runs under a different credential, billing
account and rate limit in a child than in its parent, with no warning. A child
can fail (as here) or, worse, silently spend from an account the operator did
not select. Creating a new subagent does not help; the child's age is
irrelevant.

## Workarounds

- Launch the child as a separate Pi process with the key passed explicitly:
  `pi -p --provider anthropic --model <id> --thinking <level> --api-key "$ANTHROPIC_API_KEY" ...`.
  This is a different launch path than the subagent tool and needs the
  operator's approval in this repository.
- Untested: make the intended credential the default one (for example by
  removing the stored `anthropic` OAuth entry, if Pi then uses
  `ANTHROPIC_API_KEY`). This changes every Pi session on the machine.

## Suggested upstream fix (pi-subagents)

Propagate the parent's effective credential for the child's provider (or
refuse the launch with an explicit error when the parent runs on a runtime
override the child cannot reproduce), and record which credential source a
child used in its run metadata.
