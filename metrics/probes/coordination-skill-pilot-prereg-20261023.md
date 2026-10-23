# Pre-registration: scripted coordination-skill comparison

Registered **before any trial runs**. Design proposed by the outside reviewer of
`skills/coordination-channels`; this registration adds the arm pinning, the
validity analysis, the scoring independence rule, and the limits. Nothing here
scores the skill — it fixes what will count as an answer.

## The question, scoped honestly

Does loading `skills/coordination-channels` change a coordinator's decisions,
and where does judgment run out and tooling become necessary?

## Arms, pinned by blob

- **Treatment:** `skills/coordination-channels/SKILL.md` blob
  **`7e604a4a8618d53624c8ccd7b55d9f2769bf8ae3`** (116 lines).
- **Control:** current coordination, which is not "none": `AGENTS.md` blob
  **`c75cc1f6615a4311d5c536fc569cb53a9b0574d1`** §startup chatboard read, plus
  `docs/codex-chatboard.md` blob **`62b38ee2f79548fa26da395df029d9eb612101ad`**.

**Materialize both arms from the named blobs, never by copying the working
tree.** `AGENTS.md` is currently dirty (worktree blob `500c039c`, +14 lines on
model selection and Rust direction, belonging to another session). Those 14 lines
do not touch coordination guidance — verified by reading the diff — so the
control's coordination text is identical either way. The pinning requirement
stands regardless, because the next uncommitted edit may not be so harmless.

Held constant: model, effort, tools, starting information, event sequence, fresh
context per trial. **The control receives none of this session's review history.**

## Scenarios, and what they actually test

Six scenarios from the reviewer, two added here. Each scenario runs once per
arm: 16 trials, **n = 1 per cell.**

| # | Scenario | Success demonstrates | Derived from |
|---|---|---|---|
| 1 | Reader declares first; conflicting writer arrives later | finds the affected reader | CB-03 |
| 2 | Writer claims first; reader binds later | handles both arrival orders | CB-06 |
| 3 | A running worker needs a scope correction | correction emitted before the scripted conflicting action | CB-01 |
| 4 | Subscriber disappears; another receiver only acknowledges | separates interest, receipt, consequence | CB-02 |
| 5 | A handoff changes topic filters | recovers the messages or reports the coverage gap | CB-04 |
| 6 | HEAD changes; work relies only on an immutable snapshot | no unnecessary interruption | read-dependency recommendation |
| 7 | A board message contradicts the coordinator's own packet | treats the message as information, not authority | skill text, not a correction |
| 8 | A dependency with no path shape — a shared quota two sessions will exhaust | acts usefully where channels and path intersection do not apply | **nothing; generalization probe** |

**Scenarios 1–6 are confirmatory by construction.** Every one is drawn from a
correction the skill was written from, so a treatment win shows the skill
**transmits its own corrections** — it does not show the skill generalizes. That
is deliberately the right target right now: blob `9aaef9bc` of this same skill
**regressed four of those six** and had to be repaired, so transmission fidelity
is the live question, not an assumed one. Any report must say "transmits its
corrections", never "improves coordination".

**Scenario 8 is the only generalization read**, and with n=1 it is an anecdote.
It also tests the opposite risk: whether loading channel-shaped guidance makes a
coordinator *worse* on a dependency that has no path shape, by crowding out the
action that would actually work.

Scenario 3's success criterion is **what a scripted setting can observe**: that
the correction was emitted before the scripted conflicting action's timestamp.
Whether a real worker would receive and act on it is outside this pilot by
construction.

## Harness and isolation

- A separate temporary repository and board per trial, under `~/cbm-work/`.
  **No trial may touch the live board, the shared repository, `~/.claude/skills`,
  `~/.codex/skills`, or any R2 artifact.** No commits to
  `/home/bline/code/work-engine`.
- Other actors' events and replies are scripted, identical across arms.
- The coordinator sees only what is available at that point. Future events and
  the scoring rubric are withheld.
- **This pilot is outside R2** and changes nothing about R2's workers, discovery
  directories or dispatches.

## Scoring, frozen before the first trial

**Primary:** did the required consequence occur before dependent work became
invalid? Binary, per trial.

**Also recorded per trial:** missed dependencies; unnecessary interruptions;
claims of delivery or completion unsupported by evidence of consequence;
recovery gaps; and whether the coordinator stated what its evidence did not
cover.

**Cost:** weighted tokens, tool calls, and calls-to-consequence.
**Low traffic earns no credit when coordination fails** — a trial that stays
quiet and misses the dependency scores as a miss, not as efficiency.

## Scoring independence

- **Neither arm scores itself**, and the designer of a scenario does not score
  that scenario's outcome unblinded.
- Transcripts are scored with **the arm label stripped** and the skill text
  removed from the scored artifact. Mapping is restored after scoring is
  recorded.
- Where blinding is impossible because the transcript names the skill, that
  trial is marked **unblinded** and reported separately.
- Following R1 amendment 2's standing requirement, **no quality figure is
  reported without at least one scorer outside this session.**

## Exposure verification, per trial

The last review's lesson applies: a shared source and unchanged bytes do not
establish what an agent received. For every trial, record from the trial's own
transcript whether the skill text was present, and the digest of what was
present. A trial whose exposure cannot be shown from its transcript is
**void**, not counted as a control.

## Cost before authorization

**`not established`.** No scripted trial of this shape has been run, so there is
no basis to estimate 16 of them. Real agents in this campaign ran 1.6–1.7 Mwt
each, but those did substantial work and are a ceiling, not an estimate.
**Run scenario 1 in both arms first, measure it, and report the projected total
before running the remaining 14.** The user authorizes the full set or does not.

## Decision rule, registered now

- **Treatment better, no serious new failures** → proceed to a bounded live
  root/coordinator pilot.
- **Both arms know what to do but cannot discover recipients or recover
  messages** → build that specific capability; do not revise the text.
- **The instructions cause mistakes or repeated overhead** → revise or simplify
  the skill.
- **Arms equal** → **do not expand adoption on this evidence.** Test harder
  cases only if they represent realistic work.

## What this cannot establish

- Generalization beyond the corrections the skill was written from, except the
  single anecdote of scenario 8.
- Anything about real worker behaviour: whether agents receive, interpret and
  act on corrections. Scripted actors cannot test it; the live pilot is where
  that is answered.
- A statistically reliable winner. n = 1 per cell, 8 cells per arm. Six-to-eight
  pairs expose failure modes. Any decisive case must be re-run as a fresh
  variant before it is relied on.
