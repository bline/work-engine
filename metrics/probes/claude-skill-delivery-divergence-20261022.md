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
