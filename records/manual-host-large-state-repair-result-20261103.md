# Manual host large-record repair — 2026-11-03

User requested repair after ordinary use produced a host registry larger than
the executor's 8 MiB external-input limit. The executor could persist that file
and then refuse to reopen it for collection or closure.

Implementation: `831b93c8a110b371958e24ee44a2a563d12bf390`, integrated on
`rescue-detached-20260911` as `dd134a78`.

Host state and raw/summary inspection now read owned records through a buffered
JSON reader without the external-input byte limit. Task/accounting receipt
acknowledgment and existing accounting receipt comparison likewise accept the
host's large records while retaining exact-byte digest/conflict checks. State
writes stream through a buffered writer with the existing pretty JSON encoding,
flush/fsync, atomic rename and parent-directory fsync. External artifacts and
adapter outputs retain their existing bounds. Schema and executor-pin checks
remain in place; this change does not migrate old operations to a new binary.

Validation:

- The new large-record regression first failed on the old code with
  `oversize input .../state/state.json` after the host itself saved the file.
- All 46 lifecycle tests pass, including a registry over 9 MiB, read-only summary
  inspection, large task and accounting acknowledgments, explicit objective
  closure, reopening the closed state, tampered-receipt rejection and continued
  rejection of oversized external artifacts. Existing tests cover binary-pin
  refusal, effect replay/custody, atomic delivery and unknown native effects.
- Clippy with `--all-targets -- -D warnings`, scoped formatting and diff checks
  pass. No new native model turn was commissioned for validation.
- The release binary inspected the actual 8,579,297-byte historical registry and
  reported all 11 operations closed. The prior activated binary refused its
  summary with exit 32. Before/after state SHA-256 was unchanged:
  `73634dd089185367cc1e3c6b9a145d88a4cfef3343f85f7f29ea546265e5a0ca`.
- All 16 unrelated modified tracked files in the shared worktree retained
  their bytes through integration.

Qualified bundle, provisioned from a clean committed worktree:

`/home/bline/.local/state/work-engine/tools/manual-host/c3fb8c6ed3c30af8faa9e42e8714df864ec1bddeb5bad0c0a7f3e9bb5206b5a7`

Binary SHA-256:
`e52f391d0be88c663be7279d151881e9d12c9c31f79c70296ff8cfa856795c30`.
All 44 manifest files verified; bundle files/directories made read-only.
Build custody: `/home/bline/.local/state/work-engine/manual-host-build.ZMCFmjC8/custody`.
Historical-state proof:
`/home/bline/.local/state/work-engine/manual-host-large-state-repair-20261103/historical-state-proof.json`.

The bundle is available for fresh dispatch state. Existing operation executors,
peer assignments and global skill links were preserved. No skill text or
adapter source changed, so this binary repair does not need a skill-text release.
This fixes the byte-limit mismatch; it does not add history pruning or establish
bookkeeping time savings. Remote publication is not part of this result.
