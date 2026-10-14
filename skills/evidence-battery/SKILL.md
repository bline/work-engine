---
name: evidence-battery
description: Run a declared set of read-only evidence checks in one process and one model call, and re-verify by digest rather than by restating the evidence.
---

# Evidence Battery

This capability owns the mechanics of collecting read-only evidence cheaply. It
does not decide what counts as evidence, whether a check passed in the sense the
campaign means, or what any digest authorizes. Callers own all of that.

A battery is a plan: one root, optional additional roots for evidence
directories outside the repository, and a list of named checks. Each check
declares exactly one kind — `sha256` over a path list, `aggregate` for a single
digest over a path set, `git` for a read-only subcommand, `json` for a document
with selected keys, or `grep` for a bounded regex sweep. The whole battery runs
in one process and returns one object: each check's result, the ids that failed,
and a `battery_sha256` over all of it.

```
python3 -I skills/evidence-battery/scripts/evidence_battery.py \
  --plan battery.json --save first.json --quiet
python3 -I skills/evidence-battery/scripts/evidence_battery.py \
  --plan battery.json --compare first.json
```

**Why one call.** A model call is charged for the whole context it carries, not
for what it returns. Measured on 2026-10-13, the most expensive session of its
round spent 150 of the round's 338 Plus-units across 1,180 calls, 93% of which
carried a single tool invocation, 60% asking for 2,000 output tokens or fewer
against a mean context of 136K. Of its tool calls, 339 were hash or verify
bookkeeping: 138 separate `sha256sum` invocations, 42 `jq` reads, 21 `git`
reads. The battery exists so a verification round is one charge instead of ten.

**Why compare rather than re-read.** 17% of those hash calls recomputed a path
set an earlier call had already established, one seven-path set fourteen times.
`--compare` answers an unchanged battery in about 34 tokens — a status and a
digest — and otherwise names only the ids that moved, with their before and
after. Re-establishing unchanged evidence is the thing to stop paying for.

**Boundaries.** Nothing here writes to the inspected tree. Paths must resolve
inside a declared root, a final-component symlink is recorded rather than
followed, `git` is restricted to a read-only subcommand list, grep and text
output are bounded, and one failed check never discards the others. The
`sha256` entry shape matches the manifest vocabulary the source-snapshot
evidence already uses, so battery entries and snapshot entries compare directly.
A battery is an observation. It carries no authority, and a digest proves bytes,
not acceptance.
