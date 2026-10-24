# tmux root rotation: mechanism for the existing root handoff, 2026-10-24

The root transfer in
[context-boundary.md](../../skills/manual-orchestration/references/context-boundary.md#roots)
is manual at one step: a root "cannot launch its own replacement", so the user
performs the launch. This probe automates **only that step**, under tmux. The
handoff design, its contents and the quiescence rules are unchanged; nothing
here grants a root new authority.

Role is a **window**, not a pane. A supervisor process validates, gates, posts
the handoff to the chatboard, opens a new window, waits for the successor's own
acknowledgment, and then moves that window into the role's position. No model
call happens before the gate.

- coordinator: [`metrics/scripts/root_rotation_coordinator.sh`](../scripts/root_rotation_coordinator.sh)
- successor launcher (Claude): [`metrics/scripts/launch_claude_successor.sh`](../scripts/launch_claude_successor.sh)
- handoff validator: [`metrics/scripts/handoff_validator.py`](../scripts/handoff_validator.py)
- evidence: [`tmux-root-rotation-20261024.json`](tmux-root-rotation-20261024.json)

All probes ran on private tmux sockets (`-L`); the user's `default` server was
never touched, and every board post went to a scratch repository, never the
production board.

## What the mechanism does

1. **prepare** - cap, validate, establish quiescence (the predecessor's own
   assertion **plus** independent child and claim counts), record the
   validation, then stop at the authority gate. Nothing is launched.
2. **commit** - re-validate, confirm the handoff's bytes are unchanged since
   validation, re-establish quiescence, mint the successor's session id, post
   the handoff reference and digest to `session:<uuid>`, open the successor
   window **in the predecessor's own cwd**, wait for the successor's board
   acknowledgment, then position the window and retire the predecessor.

Authority is separate from readiness: `prepare` reaching the gate means
validated and quiescent, not permitted. The gate is the only place a human
decision enters.

### Retirement modes

| `RETIRE` | who kills the predecessor | positioning primitive |
| --- | --- | --- |
| `keep` (default) | nobody; it stays as `<role>-retired` for questions about its own record | `swap-window` |
| `kill` | the supervisor, after the acknowledgment | `swap-window` |
| `successor` | the successor, as its last action, bounded to the one window id handed to it and conditioned on reading its own acknowledgment back off the board | `move-window` |

`successor` needs `move-window` because the successor's argv prompt is one turn,
so the kill lands before the supervisor positions the role. When the successor
does not retire the predecessor, the supervisor records
`predecessor-retired-by = not_established` and preserves both windows; it does
not substitute for the successor, and it does not record a retirement that did
not happen.

## The validator

The repository had no root-handoff validator; `skills/manual-orchestration/scripts/`
holds `child_context.py`, `integration_readiness.py` and
`provision_context_status.sh` only. This one was written from the doctrine's own
step-3 requirements and calibrated against the real corpus.

**Structure is not checked.** Across 38 real root handoffs the most frequent
heading ("next actions for the successor") appeared in 7, so requiring sections
would reject most real documents. The checks target what actually strands a
successor: referents it cannot follow.

| check | gates by default | what it establishes |
| --- | --- | --- |
| `readable` | yes | the file exists, is non-empty and decodes |
| `path_referents` | yes | every **absolute** path referent exists |
| `digest_claims` | yes | every digest **syntactically bound to a path** matches that file |
| `placeholders` | yes | no `TBD`/`TODO`/`FIXME`/`<fill …>` outside code spans |
| `git_refs` | yes | every **explicitly labelled** commit/HEAD/tag resolves in the repo |
| `doctrine_elements` | advisory | presence of a referent for each step-3 element |

Three outcomes, never two: `0` pass, `21` a defect was found, `32` a gating
check could not run. A check that cannot run is never recorded as a negative
result. `doctrine_elements` is advisory because a keyword scan can establish
that a referent is present but not that it is adequate; it reports
`not_established` rather than `absent`. `--require` names the gating set, so
"unchecked" can never read as "passed".

### Calibration against the real corpus

38 markdown root handoffs under `~/.local/state/work-engine` (enumerated by
path, excluding snapshot copies of skill reference docs; 633 files match
`*handoff*.md` overall, nearly all of them vendored skill docs).

| check | pass | defect | not established |
| --- | --- | --- | --- |
| `readable` | 38 | 0 | 0 |
| `placeholders` | 38 | 0 | 0 |
| `path_referents` | 33 | 5 | 0 |
| `digest_claims` | 21 | 7 | 10 |
| `git_refs` | 13 | 0 | 25 |

**No flag was shown to be a defect, and only one was shown to be correct.**
An earlier version of this section claimed "zero write-time defects" on the
strength of `mtime` ordering. That was wrong: a target modified after the
handoff was written is consistent with drift, but it does not exclude a digest
that was already wrong *and* a file that later changed. Recovering the write-time
state requires version history, which only repo-tracked referents have.

| disposition | n | basis |
| --- | --- | --- |
| proven correct at write time | 1 | claimed digest equals the committed content of `skills/coordination-channels/SKILL.md` in effect at the handoff's `mtime` |
| not provable | 2 | claimed digest matches no committed version of that file; consistent with an uncommitted working tree (82 files are modified-uncommitted in this tree), but not checkable |
| no version history | 4 | assignment records under `~/.local/state/work-engine`; write-time state unrecoverable |

The 5 missing absolute referents are all `/run/user/1000/…` tmpfs paths, valid
when written and since cleaned - an explanation, not a proof, on the same
footing. At rotation time write-time and check-time coincide, which is where
these checks earn their place; retrospectively, age is consistent with every
flag but establishes none of them as benign.

What the corpus *does* establish independently of that question is the
validator's false-positive surface, which was identified by inspection and
removed: bare basenames read as paths, non-adjacent digests attributed to
files, and command templates inside code spans read as unfilled fields.

201 path/digest pairs were accepted and 120 hex tokens were left unpaired and
unchecked. That split is deliberate. Pairing requires **syntactic adjacency**,
not proximity: an earlier "nearest preceding path" rule attributed a
supervisor identity, a retained-review identity and two binding identifiers to
whatever filename happened to appear earlier on the same dense prose line, and
reported four mismatches that were not file claims at all.

### A finding about current handoff style

`handoff-v4.md` of this role's own transfer validates `pass` only when
`digest_claims` is moved out of the gating set. Its four digests are labelled by
noun - "Inventory `9632a25c…`", "Telemetry binary `7a0cbee1…`" - rather than
bound to a path, so none can be mechanically verified. Its self-digest computed
to `6ece2098…`, matching the pointer this role was given at startup. Either the
handoff convention tightens to `<path> SHA256 <digest>`, or that check cannot
gate. This is recorded, not resolved.

## Codex parity

Established with `codex-cli 0.164.0`.

| property | Claude Code 2.1.310 | Codex 0.164.0 |
| --- | --- | --- |
| launch verb with initial prompt | `claude [flags] "<prompt>"` | `codex [OPTIONS] [PROMPT]` |
| argv prompt auto-submits | yes | yes (live, 20:23:29Z) |
| session id bindable **before** launch | `--session-id <uuid>` | **no such flag** |
| session id discoverable after launch | - | `thread_history_1.sqlite` / rollout filename, ~1s |
| post-turn re-entry from outside the session | none established (`--wake` does not start a turn in a session without a listener) | **`codex queue --thread <id> --message` delivers and starts a turn** |
| window survives a failed turn, returns to idle | - | yes |
| per-directory trust modal | yes | yes |

The one real asymmetry is **identity ordering**. The Claude mechanism mints the
successor's UUID first, so the handoff is already waiting on `session:<uuid>`
when the successor starts looking. Codex cannot pre-bind an id, so its ordering
must be launch → discover the thread id → post the handoff → wake with
`codex queue`. Discovery was verified rather than assumed: `thread_turns.thread_id`
`01a16f95-2cb6-7121-ada5-cc7e2ba25d53` has `started_at` 20:23:29Z against a
20:23:28Z launch, a rollout file is named with that id, and the thread's items
contain the launch cwd.

A topic not keyed on session id (for example `rotation:<generation>`) would
remove the asymmetry for both hosts, since the invariant that matters is that
the handoff is on the board before the successor looks and that the successor
proves its identity in its own acknowledgment. Not built.

Re-entry was established rather than assumed, and the exhausted quota is what
made it free to test: a turn that starts and then fails still proves delivery.
`codex queue --thread 01a16fff-3d14-7341-9f88-2bc1285cf965 --message …`, issued
from outside the tmux window into an idle session, put the message in the pane
as a user turn, took that thread's turn count from 1 to 2, failed on the usage
limit, and returned the session to a ready prompt. **This is a capability the
Claude path does not have**, and it makes the Codex ordering - launch, discover
the thread id, post the handoff, wake - viable end to end.

**What remains unestablished for Codex is the adoption turn itself.** The
ChatGPT weekly usage limit is exhausted with a stated retry of 2026-10-30 12:07,
so no Codex model call can complete before then. That residue is model
behaviour - whether the successor performs the five adoption steps and
acknowledges correctly - not mechanism. Whether a window survives a
*successful* turn is also still unestablished, though it survives a failed one
and returns to idle.

## Event-driven wake: one mechanism in both hosts

The coordinator polls the board for the successor's acknowledgment, and a root
hands off only when told. Both become unnecessary if sessions hold a blocking
board wait, because a post wakes them. **The wake primitive below is
established; the replacement is not wired** - the coordinator's loop still
polls, and this section records what to build, not what runs.

**The wait must be one-shot, not `--follow`.** `--follow` rearms inside one
process, so the process never exits and the model never resumes - the
[remediation probe](context-control-remediation-20261024.md) measured a wake
delivered to a running listener followed by **64.91 seconds with zero native
model responses**, with the rows reaching the model only when the listener
ended. What wakes a model is a returning tool call, not an arriving message.

| | Claude Code 2.1.310 | Codex 0.164.0 |
| --- | --- | --- |
| long blocking wait | `BASH_MAX_TIMEOUT_MS` | `background_terminal_max_timeout` |
| measured ceiling before configuring | 600,000 ms | - |
| measured after configuring | **640.2 s foreground, returned in-band** | 3,600,000 ms set |
| inference while blocked | none | none |

`600000` is a **silent clamp**, not a rejection: a requested 700,000 ms was
accepted, applied as 600 s, and the command was then **moved to the background
rather than killed** - it ran on to its own 640 s timeout and exited normally.
So an overlong foreground wait degrades into the background path (a backgrounded
command's exit re-invokes the model) instead of failing. Raising
`BASH_MAX_TIMEOUT_MS` to 3,900,000 in `~/.claude/settings.json` removed the
clamp: a child session reading that file held a 640.2 s foreground wait and the
result returned in-band. The value was read from the settings file, not the
parent's environment. `launch_claude_successor.sh` also exports the ceiling, so
the mechanism does not depend on a host's user settings.

`BASH_DEFAULT_TIMEOUT_MS` was deliberately left at 120,000. Raising the default
alongside the ceiling would make every hung command block for the full hour.

**Pick the wait below the prompt-cache TTL, not at it.** Nothing is spent while
blocked; the cost lands at the wake. This session's main-conversation cache TTL
is 1 hour, so a wait that spans it resumes by rebuilding the prefix instead of
reading it. The two live rotations bound the magnitude: a novel prefix recorded
preamble occupancy 44,402 with `cache_read` 0, while the same prefix inside the
TTL recorded total `cache_creation` 23,266 against `cache_read` 239,169
(**n = 1 pair**, not a controlled comparison). A 60-minute wait sits exactly on
that boundary; roughly 50 minutes is strictly better. Whether Codex's provider
has the same TTL is **not established**.

**A session blocked in a wait is committed, not available.** It is inside a tool
call and cannot answer anything until the wait returns. `RETIRE=keep` exists so
a retired predecessor can answer questions about its own record, so a retired
root must hold no listener.

## Defects this work found in the mechanism

Four, all in code written earlier in the same session:

1. The stall pattern was **case-sensitive** and missed Codex's "Trust this
   folder?". Unfixed, a Codex successor blocked on the trust modal would have
   been misdiagnosed as "no adoption" after the full 180s timeout.
2. The dialog scan ran **to completion before** the adoption poll, so it was
   blind to dialogs appearing after startup - including Codex's rate-limit /
   model-switch modal - and charged 25s to every rotation. Merging the two loops
   took the coordinator's own overhead from ~27,000 ms to 1,443 ms with an
   instantly-acknowledging shell successor; a model successor is still bounded
   by model latency.
3. The validator contract executed `"$VALIDATOR"` as a single program, so a
   validator carrying its own flags failed with exit 127. It now word-splits,
   because a real validator needs `--repo` and `--require`.
4. The validator outcome was binary, collapsing "defect found" with "could not
   check". The coordinator now records `fail_validation_<phase>` and
   `validation_not_established_<phase>` separately, and treats an unusable exit
   code as not-established rather than as a pass.

Defect 3 was caught only because of the fail-closed rule in defect 4: exit 127
degraded to `not_established` and blocked the rotation instead of passing it.

## Observed outcomes

| outcome | exit | predecessor | elapsed |
| --- | --- | --- | --- |
| defect in handoff (bad digest; absent referent) | 21 | untouched, nothing launched | - |
| validity not establishable | 32 | untouched, nothing launched | - |
| validated and quiescent, awaiting authority | 10 | untouched, nothing launched | - |
| successor stalled on a dialog | 27 | preserved at index 0 | 857-1,483 ms |
| no adoption acknowledgment | 34 | preserved at index 0 | 4,855 ms (`ADOPT_TIMEOUT=5`) |
| committed | 0 | per `RETIRE` | 1,443 ms (shell successor) |

Two live Claude rotations committed, at 26.9 s and 26.8 s, both before the
fixed-cost fix; 4 and 5 model calls. The second, in the same cwd within the
cache TTL, recorded cache_creation 23,266 against the first's 53,967 while
cache_read rose from 148,402 to 239,169 - consistent with the preamble being
read rather than rebuilt, **n = 1 pair**, and not a controlled comparison.

## Limits

**The quiescence check is structurally present but has never been exercised
against real sources.** `CHILDREN_CMD` and `CLAIMS_CMD` are independent-check
interfaces, and every run recorded here supplied stubs (`echo 0`, `true`). The
coordinator's own logic around them is tested - an unreadable child count
yields `quiescence_not_established` rather than a pass, a non-zero count yields
`fail_children_live` - but that a real child enumeration and a real
`read --active-claims` for the predecessor session are wired correctly is **not
established**. Until they are, this mechanism does not enforce the
doctrine's step-1 and step-2 preconditions end to end; it enforces the shape of
them.

This is a mechanism probe, not a product. The lease/generation guard that would
close the board-cursor TOCTOU is **not built**: two supervisors racing the same
role is out of scope here. `MAX_ROTATIONS` caps a runaway chain but is not that
guard. The validator's gating set is a policy choice the operator makes, not a
doctrine ruling. Promoting any of this into
`skills/manual-orchestration/` is the skill owner's decision; this role
instruments under measurement and does not modify that skill.
