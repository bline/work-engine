# S10 Pi/Docker host component — 2026-11-03

Status: bounded component accepted and applied to the shared working tree.
The maintainer selected the [host-first revision](../planning/s10-pi-docker-host-first-revision-20261103.md)
at 2026-11-03 23:07:07.749 UTC, within the original attempt beginning
22:54:12.975 UTC. Its 90-minute/$15 total cap was not reset. The complete base,
original S10/M6 and release conditions remain unmet.

## Implemented consequence

[`work-engine-manual-host pi-docker`](../rust/crates/work-engine-manual-host/docs/pi-docker.md)
now admits an explicitly pinned provider-disabled fixture, binds its workspace,
inputs, Pi runtime and Docker realization, correlates RPC replies, and collects
actual Docker daemon exit/removal facts. It provides bounded waits, cancellation,
restart reconciliation without blind launch replay, durable exports and
content-addressed receipts with exact acknowledgment and integrity checks.
Unknown execution/protocol state remains unknown.

The implementation is additive: a
[Pi/Docker realization module](../rust/crates/work-engine-manual-host/src/pi_docker.rs),
CLI/module exposure, a fixture example and component documentation. Its journal
owns container/process evidence, not task, lifecycle, delivery or review meaning.
Five reviewed files were applied after destination baseline and HEAD checks;
all 828 frozen source-file hashes match the resulting Rust workspace. No Cargo
manifest/lock, Git branch/index or global installation change was made by this
integration. No user-visible commit, push or deployment was performed.

Source checkpoint: `ae6151fcdc2bd21274a95907d447947f5e1cdb3d`.
Source manifest SHA-256:
`3522f492f707355553fb2c5c60db48b2c2dcc1ab58421023b869fc341004b2d1`.
Destination HEAD remained `0d51c0b8668d2497c0f7b31d5aac9b169faa60b5`.
Pre-existing workspace changes were preserved.

## Validation and review

The destination package gate passed **59 tests**, with three deliberately ignored
fixture tests. It ran all package targets with the locked dependencies offline;
source inputs were recorded and checked, and its owned process group had no
leftovers. Separate current-source validation ran six focused component
functions, including actual Docker cancellation/orphan recovery, malformed
trailing output, missing final proof, and receipt integrity behavior.

Actual Pi **1.5.0** ran in the pinned Node image, non-root, with read-only root,
no network/credentials/host-root/Docker-socket mount, dropped capabilities and
bounded resources. Daemon inspect/wait/removal and durable exports were
captured. A current positive completed-stream proof survives exact receipt ACK
and fresh CLI reconciliation. Earlier actual Rust-host SIGKILL evidence is
preserved as historical; current interruption regressions construct the relevant
saved states and include actual daemon orphan collection.

The first Astra source review found two P2 defects. Both were fixed and verified:

- Log identity is persisted before file creation; recovery preserves and
  bypasses unjournaled reservations instead of repeatedly colliding before cleanup.
- Success requires final-stream proof bound to the exact completed stdout,
  process and session. A valid prefix, partial tail or old missing-proof journal
  cannot silently become success after restart.

A retained reevaluation accepted the fixes, but its native resume had broadened
the requested filesystem grant. That report remains advisory. A fresh Astra
replacement then verified the same bounded delta under an independently observed
native `readOnly` grant, with no findings. This is a reconstructed review
continuation and same-provider/different-model evidence, with no independence
claim. Final report SHA-256:
`9dbe09d69b19a922e52111f07a7269774f500111b855495c13b3f20ca9fd634b`.

Canonical review-episode publication failed on read-only Git storage. Immutable
native reports and exact source/provenance were retained; no canonical episode
completion was claimed. This operational limitation was not converted into an
additional infrastructure project. Author-only instruction review of the
component documentation found no material issue.

## Supported boundary and next work

This component sends four fixed RPC controls and admits no prompt, inference,
engineering mutation or model tool call. It does not qualify account access,
credential-bearing/networked execution, agent review/repair, actual MCP recipient
delivery, session JSONL persistence, `agent_settled`, canonical Rust context
replacement or domain settlement. A killed host supplies no autonomous daemon-side
deadline; an active host or explicit reconciliation remains required.

The separate delivery effort reports an accepted bounded inbox/stdio-receiver
fixture in chatboard 4317. Its actual Pi receiver and pressure/control timing are
still unqualified. The next integration can consume that owned contribution;
this result creates no follow-on assignment or allocation.

Detailed assignments, authority, custody, initial failures, frozen source/runtime,
review reports, destination gate and accounting are under
`/home/bline/.local/state/work-engine/s10-supervision-20261103/pi-docker-base/`.
Accounting is a speed-adjusted API cost proxy, not measured subscription quota;
the closing observation and reporting-tail qualification are in `root-result.json`.
