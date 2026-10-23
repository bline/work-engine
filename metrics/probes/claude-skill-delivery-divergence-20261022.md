# Skill delivery divergence across the two arms, 2026-10-22

A measurement finding about **how skill text reaches an agent**, not about any
skill's content. Prompted by a user question about symlinking skills into the
agent-discoverable directories.

**R2 is unaffected.** R2's builders are Claude-side, and
`~/.claude/skills/manual-orchestration` has been a symlink to the repository
since before the campaign, which the pre-registration already declares as an
instrument change. Nothing below changes a byte any R2 builder reads.

## Three locations, and what a listing costs

Skill text lives in up to three places: the repository `skills/` (30
directories), `~/.claude/skills` (13), and `~/.codex/skills` (31).

Workers **do** receive a listing of the discoverable skills. Taken from the
fix-b builder's own context, a `<system-reminder>` named **13 skills in about
4 KB**, `manual-orchestration` among them via the Claude-side symlink. So
placing a skill in a discoverable directory puts its name and description into
every worker's context — **177 bytes** for a one-line description, and the full
file if a worker then invokes it. That is why
`skills/coordination-channels` is **not** linked into either directory while R2
runs: it would void amendment 5's zero-delta basis.

## The divergence inventory

Of 38 repository skills present as a **real copy** in an agent directory,
**17 had diverged from the repository. All 17 are on the Codex side.** The
Claude side had no diverged real copy, and its one symlink cannot diverge by
construction.

Two cases were read rather than inferred, and they are not the same kind:

- **`durable-state`** — not a differing script. The Codex copy is **missing**
  `scripts/codex_chatboard.py` and its test. `AGENTS.md` invokes the chatboard
  by repository path, so the script in use is always the repository's. No
  divergence risk in practice.
- **`role-scheduler`** — real divergence. `references/scheduler-contract.md`
  **differs** and `references/historical-inspection.md` is **absent** from the
  Codex copy, while `AGENTS.md` directs Codex agents to follow role-scheduler
  **at startup**.

**Which copy a Codex agent actually loads** — the discovered
`~/.codex/skills` one, or the repository path cited in `AGENTS.md` — is
**`not established`**. The symlink makes the question moot wherever it is
applied; where it is not applied, two texts exist and the answer matters.

## Instrument change: the Codex-side symlink

`~/.codex/skills/manual-orchestration` was an independent real directory, 19
files, 200 K, last modified 2026-10-22 07:46. It is now a symlink to
`/home/bline/code/work-engine/skills/manual-orchestration`, matching the
Claude side.

**Verification, in order.** Identical before the swap (`diff -rq` clean);
identical after, through the symlink (`diff -rq` clean); the four worker-read
digests unchanged — `SKILL.md` 86ec40bc…, `worker.md` 802ac8d8…,
`call-cost.md` af4d3bb9…, `anthropic-host.md` 84cfc80b…, the same digests fix-a
and the R2 planner recorded when they read those files.

**Byte delta inside either arm's worker read set: zero.** The content was
verified identical before the change, so no agent reads a different byte. Only
the delivery path changed.

**What that verification does and does not establish.** The symlinks establish a
**shared source**; the unchanged digests establish **unchanged bytes at that
source**. Neither establishes **what a worker actually received** on any given
dispatch — that is read from a worker's own context, and the only such reading
taken here is the skills listing found in the fix-b builder's transcript.
Amendment 5's per-dispatch verification requirement stands for the rest and is
not discharged by either the symlink or the digests.

**Direction of effect: it removes a confound rather than adding one**, the same
reasoning the pre-registration gives for the Claude-side symlink. Before, the
two arms could silently come to read different treatment text; now they cannot.
The effect on measurements **already taken is nil** because the copies were
identical at the moment of the swap. The effect going forward is the removal of
a divergence risk, not a change in behaviour.

**Method.** `rename(2)` cannot replace a directory with a symlink — errno 21,
`EISDIR`, confirmed by test, so no single-syscall swap exists. The symlink was
pre-staged outside the window and both renames issued from one process, giving a
**40 µs** window containing no content I/O. For comparison, `rm -rf` of a
40-file directory measured 637 µs, and chaining two `mv` processes would put two
~1 ms process spawns inside the window. The displaced directory is retained at
`~/.codex/skills/.mo-old`, so rollback is one unlink plus one rename and the
original bytes are not destroyed. A symlink **can** atomically replace a
symlink, so every future retarget of this path has no window at all.

## Open, and deliberately not acted on

- **The other 16 diverged Codex copies are untouched.** "Diverged" does not
  carry a direction: a Codex copy may be the newer, authoritative text, and
  linking would silently discard it. Each needs a direction decision first.
- **`role-scheduler` specifically**, because `AGENTS.md` has Codex agents read
  it at startup and the contract text differs.
- **Which copy a Codex agent loads** remains `not established`; settling it
  needs a Codex transcript showing the resolved path, which this role did not
  collect.
- This role is excluded from both arms of the prospective correspondence trial
  and changed the Codex arm's delivery path **only** on explicit user
  instruction, with the content verified identical first.

## Correction, 2026-10-23: the extraction regressed its own source

**The wrong claim.** `skills/coordination-channels/SKILL.md` (blob `9aaef9bc`)
was committed as carrying "the objective, the communication responsibilities,
the four separated states, and only capabilities that exist today."

**The error class.** The skill was written by compressing the design document
from its conclusions rather than from its corrected text, so it reinstated four
things the design had already fixed: forwarding to a worker only via an unread
packet or a boundary handoff (the design restored the mid-run correction, and
cites one as the case that contained the collision); overlap checking anchored
to claim time (the design replaced that with an order-independent invariant,
because the known collision had the order a claim-time check misses);
`wait` mandated at startup with no timeout, which blocks indefinitely on an
empty channel; and an unconditional worker exclusion whose scope and reason
were left in a document the reader has not loaded.

**The downstream consequence.** Had a root followed it, the skill would have
forbidden the single action that protected R2's read set, and its own stated
rationale for the worker restriction would have outlived R2. Nothing acted on
it between commit and repair.

**The check that would have caught it.** Diff an extraction against the
corrections list of its source, item by item, rather than against its summary.
Each of the four maps to a numbered finding the design already records.

**Found by** outside review of the extracted skill, not by this role.
