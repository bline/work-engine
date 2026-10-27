# Records

Dated documents produced by campaigns and slices: phase plans, contracts,
validations, results and coordination notes. See
[repository layout](../docs/repository-layout.md) for how this directory relates
to the others.

## Kept as written

A record states what was true at its date, and other files cite records by path
and by SHA-256. Editing one changes what it asserts and breaks the citations to
it. Corrections are new records that name what they correct.

Names follow `<area>-<workstream>-<phase>-<kind>.md`, as in
`rust-compiler-c2-plan.md`. Other kinds of record keep their existing homes:
`planning/`, `metrics/`, `reviews/`, `proposals/` and `notes/`.

## Not yet moved

As of 2026-10-27 the Rust campaign records are still at `docs/rust-*.md`. The
exception is `docs/rust-development.md`, a durable reference that stays in
`docs/`. They move in one wave under phase 4 of the
[layout plan](../planning/repository-layout-plan.md), once their owners have
committed them. New campaign records go here now.

## Moved files

Filled in when files move. Each row gives the old path, the new path, the
commit, and any reference to the file that is recorded as unresolved.

| Old path | New path | Commit | Unresolved references |
| --- | --- | --- | --- |
