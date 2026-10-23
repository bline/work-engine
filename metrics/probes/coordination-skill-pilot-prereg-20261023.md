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

## Amendment 1, 2026-10-23: five corrections before the first trial

Raised by the outside reviewer of this registration. Two are design errors, not
wording.

### 1. Treatment is baseline **plus** skill (design error)

The arm table could be read as the skill **replacing** the pinned coordination
instructions, which would test a different question. **Both arms receive the
baseline** — `AGENTS.md` `c75cc1f6…` §startup chatboard read and
`docs/codex-chatboard.md` `62b38ee2…`. Treatment adds
`skills/coordination-channels/SKILL.md` `7e604a4a…`. **The measured quantity is
the skill's marginal effect**, and any figure from this pilot must be reported
as marginal, not as skill-versus-nothing.

### 2. Response policies, not identical replies (design error)

The registration said "scripted events identical across arms". Taken literally
that makes the primary metric unmeasurable: if the environment replies the same
regardless of what the coordinator does, a successful correction can have no
effect, so "did the consequence occur" has no variance to measure.

**Corrected:** the environment is a deterministic state machine with an
**identical response policy** per arm that reacts to what the coordinator
actually does. Every fixture has at least one coordinator action that averts the
invalidation and at least one that does not.

**Simulated clock, advanced by emitted actions only** — posts, claims,
dispatches, job starts, corrections. Reads and inspections do not advance it, so
looking before acting is not penalised and **machine speed is never scored.**

### 3. All eight fixtures frozen, with binary criteria

`coordination-skill-pilot-fixtures-20261023.md`, sha256 `f8062bc5…`, frozen
before any trial. "Acts usefully" is replaced by an observable criterion in
every cell; S8's is a schedule meeting both deadlines without a window
exceeding 10 units.

**S3 keeps emission separate from worker action.** Its primary is emission of
the correction before clock 10. The scripted worker's compliance and
acknowledgment are recorded in `scripted_compliance`, which **no primary or
decision rule may read**, because they are properties of the script and not
evidence about real workers.

### 4. S8's premise was wrong

The registration said channels do not apply to a shared quota. **Path
intersection** does not apply; channels can carry quota coordination perfectly
well. The fixture now tests whether the coordinator recognises a dependency with
no file paths, and **any authorized route counts as a solution** — no particular
message or channel is prescribed. The quota is entirely simulated.

### 5. The cost claim was overstated

The registration called the campaign's 1.6–1.7 Mwt agents "a ceiling". They are
**historical observations of different work**, not an established ceiling for
anything. Corrected.

**What the calibration pair yields is a conditional extrapolation, not an upper
bound** for eight scenarios of differing length. Reported as three separate
figures, never as one number:

- **measured trial cost** for S1 treatment and S1 control,
- **harness construction cost**, which is one-time and not per-trial,
- **independent scoring cost**, which scales with trials but is not the
  coordinator's.

**Explicit trial limits are chosen before launch**, not discovered during it: a
per-trial weighted-token ceiling and a maximum emitted-action count per trial,
both registered with the calibration result. A trial that hits a limit is
recorded as `limit_reached`, not as a failure of coordination.
