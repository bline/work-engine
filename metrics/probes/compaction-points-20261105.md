# Codex compaction points and the 250K limit, 2026-11-05

At 2026-11-05T04:58:28Z (file modification time) `~/.codex/config.toml` gained
`model_auto_compact_token_limit = 250000`. This probe records where Codex
sessions compacted between 2026-10-26 and 2026-11-05T07:40Z, and which sessions
followed the new limit. It supports the context levels in
[context boundary](../../skills/manual-orchestration/references/context-boundary.md#measured-basis).

This is a read-only analysis of local rollouts. No agent was launched.

## Result

**A session follows the limit only if it started after the change, and a
spawned child follows its parent.**

| Model, window | Kind | Sessions | n (threads) | Input before compaction: min / median / max |
|---|---|---|---:|---|
| `gpt-6.1-sol`, 475,000 | automatic | started before, compacted before | 69 (17) | 437,522 / 448,224 / 456,118 |
| `gpt-6.1-sol`, 475,000 | automatic | started before, compacted after | 4 (2) | 448,797 / 449,608 / 449,676 |
| `gpt-6.1-sol`, 475,000 | automatic | started after | 1 (1) | 248,987 |
| `gpt-6-astra`, 258,400 | automatic | started before | 10 (9) | 239,130 / 243,607 / 245,418 |
| `gpt-6.1-sol`, 475,000 | requested | started before | 52 (19) | 22,547 / 305,838 / 449,662 |

- The one session started after the change that grew past 250K, VS Code thread
  `01a1aa1e`, compacted at 248,987 and restarted at 30,980 (n = 1).
- Two roots started before it, `01a195d4` and `01a1a3dc`, compacted four times
  after it at 448,797-449,676 (n = 2).
- Builder `01a1aa15`, spawned at 05:01Z by root `01a1a2c0` (started
  2026-11-03), reached 338,682 with no compaction (n = 1). It took its parent's
  limit, not the file's.
- Without the limit, automatic compaction sat near 94% of the recorded window
  for both models: about 448K of 475,000 and 244K of 258,400.

The band the guidance used until now, 171,267-239,002 on 258,400-token windows
([2026-10-21](compaction-band-20261021.json)), predates both models. It counted
every `compacted` record, so it may include requested compactions (not
established).

## Levels chosen from this

With the limit in force, Astra (from 239,130) and new Sol 6.1 sessions (at
248,987) both compact a little under 250K, so one pair of token levels fits
both:

- **band floor 235,000**, just under the lowest automatic compaction observed;
- **trigger 200,000**, which leaves 35,000 before the band floor for the next
  piece of work. That margin is chosen, not measured: one remediation estimated
  at 40K grew about 61K on 2026-10-22 (n = 1), so the next-task estimate still
  decides.

Percent-of-window levels of 58% and 66% put these warnings at 275,500 and
313,500 on a 475,000 window, above the limit, so they never fired before an
automatic compaction.

These levels replace hypothesis H1 for Codex as hypothesis H2. They are not a
calibrated optimum.

## Cost basis

A replay of 13 Sol 6.1 root threads (8,015 calls since 2026-11-02T07:00Z)
under a 250K limit estimated 13-21% lower root spend for restart sizes of
30-70K, with about twice as many compactions (56 to 117). The estimate leaves
out the API's long-context surcharge, which subscription usage reportedly does
not pay. It models the same work at a smaller context, so it does not measure
what is lost or re-read after a compaction. The replay script is in the
analysis workspace (`root_compact_replay.py`) and is not part of this probe.

## Caveats

- **Automatic versus requested is a heuristic.** A compaction preceded by model
  or tool items of the same turn counts as automatic; one that opens its own
  turn counts as requested. Most requested compactions are in S10 lineage
  threads and continuation fixtures.
- **Small n after the change**: 1 session that followed the limit, 2 roots that
  did not, 1 child that did not.
- **Scope**: rollouts under `~/.codex/sessions` and `~/.codex/archived_sessions`
  modified since 2026-10-26 (605 files). Isolated qualification homes hold
  copies and are excluded.
- **Who set the limit** is not recorded in these files.

## Inputs and reproduction

```text
python3 -I metrics/experiments/compaction-points-20261105/compaction_points.py \
  --since 2026-10-26T00:00:00Z --change-at 2026-11-05T04:58:28Z \
  --out compaction-points.json ~/.codex/sessions ~/.codex/archived_sessions
```

- `compaction_points.py` sha256
  `3c6bfa7c262705e7b43985dc130672172a10cd973045eef545bc5f8c66f78caa`.
- [Output](compaction-points-20261105.json), run 2026-11-05T07:38Z, sha256
  `a9b61fc6e28f5d8de1a9d2c217a074aa26d11feedc894d2462ead4e29cf6ef58`. Later
  runs include later sessions.
