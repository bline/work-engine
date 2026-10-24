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

**2. The 66-second discovery exposure: worker nonexposure established.**
Answering the ask in board messages 2552/2553, addressed to this role by name.
`bounded-inference-readiness-root` (01a1653b) created
`~/.codex/skills/coordination-channels` at 03:53:12.384520Z and removed it at
03:54:18.266750Z, and asked owners to inspect worker launch and turn listings
during and after the window before relying on a zero worker-context delta.

**Disposition: no worker arm received the skill in its listing.** Exactly one
session's skill catalog ever named `coordination-channels` — 01a1653b itself,
the root that made the link — in one developer `<skills_instructions>` message
at 03:53:39.179Z and its `state.host_skills` snapshot a millisecond later. That
root's self-report is confirmed; its caution about child dispatch is also
confirmed, independently: no other session's catalog named the skill.

**The retention worry does not hold, checked rather than assumed.** 01a1653b
compacted at 04:12:48.822Z, 18 minutes after the unlink. That record's two
retained `<skills_instructions>` blocks do **not** name the skill, so the
listing did not survive the boundary. Its five remaining occurrences are
ordinary conversation text — two user questions, an agent answer, and board
message bodies — which is not skill exposure, and under prereg amendment 5 the
treatment-relevant artifact is the listing a session receives.

Completeness basis, as rule 7 requires. Authoritative domain:
`/home/bline/.codex/sessions/**/rollout-*.jsonl`, **799 files enumerated and all
799 scanned** (131 modified since 2026-10-20). Artifact classes in which a Codex
session's skill catalog can appear: the `developer` message carrying
`<skills_instructions>`, the `state.host_skills` body, and a compaction record's
retained copy of either; the scan matched the first two across the domain and
the third was inspected in the only hit session. The Claude-side tree is covered
separately by its own listing — 13 entries, no coordination entry. **This covers
Codex rollout records and the Claude skills tree; it does not cover a context
assembled by any other means.**

Not established: whether any already-loaded context elsewhere held the listing
and was never written to a rollout. Nothing in the searched domain suggests one.

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

## Re-pin, 2026-10-24: v3-v5, and the v2 treatment text was superseded

Owed since v2 and closed here. The exact commit was supplied by the skill's
owner at board **2729** (`context-remediation-root`, session
`01a16bc6-7afb-7b61-8d84-836521fccb92`), which also confirms this measurement
artifact was not edited by them. **Three** boundaries have passed since v2, not
one, so observations must be attributed per version and never pooled across
them.

| version | commit | at | blob | lines | file sha256 |
| --- | --- | --- | --- | --- | --- |
| v1 | registration | — | `7e604a4a…` | 116 | `5eda9d07…` |
| v2 | `13ad4acd` (this role) | 2026-10-23 | `7ee894fa…` | 159 | `d247611f…` |
| v3 | `a37d3149` | 07:51Z | `634f1ba8…` | 170 | `77d9807d…` |
| v4 | `b4db89b8` | 17:10Z | `b8532b6c…` | 190 | `14c92a41…` |
| **v5, in use** | **`5e163a37`** | **21:00Z** | **`b676a898…`** | **212** | **`0c418e6e…`** |

Verified: the blob at `5e163a37` is identical to the blob at HEAD, and
`5e163a37` is an ancestor of HEAD.

### v2's rearm guidance was wrong and v5 corrects it

v2 (mine) told a listener to "wake on `0` and `2` alike and distinguish them in
the wakeup text; it costs one turn per quiet period", offering the in-cell loop
only as the preferred alternative. v5 removes that option and names its cost:
*"a 55-second timeout that wakes the model to rearm can resend its full context
about once a minute, even when the input is cached."* It requires host-side
quiet retry inside an owned process/cell, with the model woken only for
messages, control events, errors, or the availability bound.

That is a correction to the treatment text this probe measures, made by the
skill's owner, not by this role. Recording it because a rubric that pins a text
must say when the pinned text was wrong.

### v5 also closes the self-wake gap this role did not close

v5 adds `--exclude-session-id` (opt-in suppression of the listener's own
ordinary posts, with the shared-session-ID qualification stated) and
`post --wake` to preserve required signals through it. That answers the outside
audit's CU-04 — a session woken by its own posts — which v2 did not address and
which this role flagged at board 2708 as a gap in its own artifact.

### Consequence for the measurement

**The 34 Kwt-per-wake figure is now a measurement of a superseded treatment.**
It was taken under v1/v2 wake economics, which v5 changes by design. It must not
be carried into any v5 comparison or used to price a coordination pilot; a
current figure has to be re-measured against a v5 listener. Stated here because
this role quoted that number in design discussion after v4 had already landed.

n for every v1/v2 observation in this probe is unchanged, and none of it
transfers to v5.
