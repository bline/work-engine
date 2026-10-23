# Dogfooding the coordination skill: observation rubric

**Frozen before the cycle it observes.** Operational evidence, **not** a
controlled estimate of the skill's benefit — there is no control arm, so no
figure from this may be attributed to the skill.

Treatment in use: `skills/coordination-channels/SKILL.md`, loaded from the
repository by participating roots. Two identifiers per version, both recorded so
no document contradicts another; the sha256 is what `sha256sum` returns and what
sessions are asked to record.

**v1**, frozen at registration, 116 lines:
- git blob `7e604a4a8618d53624c8ccd7b55d9f2769bf8ae3`
- file sha256 `5eda9d070c741a0dc8085b21936e8605152441bf9c3557aa71de9ba5a6fc914f`

**v2**, in use from this commit, 159 lines:
- git blob `7ee894fa16d0fa06593386a29c4a33aeb25f67c6`
- file sha256 `d247611ffa9ebff03470a264009c32d63bf3b44c33ad68396c320cd6ee5b6f6e`

**The treatment changed mid-cycle, and that is a version boundary, not a
re-pin.** Observations recorded before this commit are of v1 and must not be
pooled with later ones. What changed: v1's CB-02 monitoring revision, which was
written but left uncommitted and so was never the text any session loaded; plus
the two corrections from the first observation below. A root that recorded
`5eda9d07…` was loading HEAD; a root that read the working tree between
registration and this commit was loading neither pinned version, which is itself
a defect of this setup and is why both identities are now tabled.

The prompt handed to participating sessions is
`/home/bline/cbm-work/coordination-dogfood-prompt.md`. Scope: **root and coordinator only.** The scripted pilot
(`coordination-skill-pilot-prereg-20261023.md`) stays registered, deferred and
unrun.

## The R2 restriction still applies

The board's `proposal-evaluation-r2` claim is **active**, expiring
2026-10-23T13:07:17Z — verified by reading active claims, not inferred.
**Sessions going quiet does not establish that R2 has ended.** So the
worker-exposure restriction in prereg amendment 5 remains in force: this skill
stays out of every worker packet, shared instruction, inherited context and
discovery directory, and that is verified **per dispatch**, not once.

## Recorded during the cycle

Each entry needs the message reference and evidence of what happened — **not a
separate report per exchange**, which would cost more than the coordination.

- **Missed recipients.** A party affected by a change who was not reached, found
  afterwards. The one known prior instance is the 2026-10-22 scope collision,
  caught by a hand-directed read of three packets.
- **Late corrections.** A correction that arrived after the dependent work was
  already invalid.
- **Unnecessary interruptions.** A message or correction that cost context and
  changed nothing, with its cost.
- **Recovery problems.** Traffic lost or unreachable across a boundary, a
  compaction or a filter change.
- **Communication overhead.** Weighted tokens and calls spent on coordination,
  recorded as a cost **to be weighed against what it averted** — never as a
  score to minimise.
- **Unsupported consequence claims.** Any report that something was received,
  narrowed or handled without evidence of the consequence.

## Observed

**1. A subscription that ends on its own timeout, with no signal.**
*Recovery problem and unnecessary interruptions.* Codex session `01a1653b`
(B2 coordinator) armed eleven `wait` listeners at `--timeout-seconds 3600`,
each cell waking the session by messaging its own thread **only on exit 0**.
`wait` exits 0 with messages and **2** on timeout, so a timeout would have
ended the subscription silently. Basis: the eleven arms and their exit guards
read from the rollout, cursor advancing 2561→2571 (one wake per message, so not
a stale-cursor loop); exit codes **run**, not inferred — 0 with messages, 2 on
timeout, 1 on a bad argument. Not observed to have fired: at the last reading
the process was still blocked, deadline 06:10Z, so the lapse is **established as
reachable, not as having occurred.**

Cost measured from the rollout's `last_token_usage`: about **34 Kwt per wake**
across three requests, **378 Kwt** for eleven messages over 41 minutes, with all
fourteen exec cells in that window doing board bookkeeping and no B2
implementation. **Six of the eleven wakes were `aspect:native-runtime`**, a
workstream that session was not working on — roughly 206 Kwt of interruption
that changed nothing. Weighed against what it averted: nothing yet, since no
message in the window altered B2's work. n = 1 session.

Both corrected in treatment v2, and the resumption gap recorded in
`docs/codex-chatboard-channels.md` Part 1 with a Part 2 item 8 (`--follow`).
This is the focused regression case the rule below anticipates: **a quiet
listener and a lapsed listener are indistinguishable from inside the session**,
which no scripted trial would have thought to script.

## The rule that keeps a quiet cycle honest

**If nothing during the cycle required coordination, the result is
`uninformative`, not `successful`.** A cycle with no missed recipients and no
coordination demand is evidence of nothing. This is registered now so a quiet
cycle cannot later be read as a pass.

Equally: a failure found here becomes a focused regression case for the deferred
pilot. That is the main thing this cycle can produce that the pilot cannot —
cases nobody thought to script.

## This role's declared dependencies, as the skill requires

Declared with the four fields the design requires: dependent work, responsible
actor, source identity, lifetime.

| dependent work | responsible actor | source identity | lifetime |
|---|---|---|---|
| remediation cohort score (integration status, cost per finding) | `claude-coordinator` c1d31e88 | fix-a `00a77fe3`, fix-b `2161afd7`, and whether they integrate and onto what base | until the cohort score is final |
| prereg amendment 4's R2-M-002 overlap figures | `claude-coordinator` c1d31e88 | `r2-plan/freeze-check.json` as written 20:02 | until R2 is scored |
| prereg amendment 5's zero-delta basis | every R2 dispatcher | the four worker-read digests `86ec40bc`, `802ac8d8`, `af4d3bb9`, `84cfc80b` | until R2 ends |

**Overlap check, both directions, with its basis.** The only active claim is
`proposal-evaluation-r2`, write scope `rust/crates/proposal-evaluation` and
`rust/crates/portfolio-decision`. Nothing this role depends on or writes
(`metrics/**`, `docs/codex-chatboard-channels.md`,
`skills/coordination-channels/**`) lies inside it, and nothing in those paths is
in any R2 slice's read set. Basis: the active-claims read above and the read-set
paths in `scout2/result-v1.json`. **This does not close the audience** — an
undeclared reader of `metrics/**` would not be found this way.
