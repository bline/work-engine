# Manual host ordinary-use repairs — 2026-11-03

Implemented and activated the four user-approved adjustments from the UI and
backend ordinary-use observations. Source commit:
`a8f9c3059599480fa2947e55040acb2fff10b8ca`, integrated by fast-forward on
`rescue-detached-20260911` with all 16 unrelated modified tracked files preserved.

## Resulting behavior

- Binary artifacts stream through 64 KiB buffers for hashing, snapshot copying
  and later source/snapshot revalidation. Complete matching bytes are published
  by hard link after fsync; interrupted copies remain separate stages. Reads
  cannot follow a file's growth indefinitely. The structured-input bounds on
  packets, results, custody JSON and adapter output remain in place.
- Retained-turn configuration can use an applied-settings event in the idle
  interval immediately preceding the exact task-start and matching turn context.
  Source line hashes and both attribution records are retained. Earlier-turn,
  missing-context, conflicting or later-turn evidence remains insufficient.
- Exact completed/failed/interrupted history is readable while a native session
  is unloaded. Collection uses only thread/read and paged thread/turns/list;
  no thread/resume, grant change or new model turn is needed. Unloaded
  nonterminal history remains unestablished.
- CLI --compact projects the response, and --out reserves a new absolute log
  path before effects and syncs the full response or execution error there.
  Existing files refuse before effects. Optional preflight_files binds and
  checks declared gate/input files before launch; it does not infer paths or
  authorization from packet prose. Draft path strings are hashed by the host.

## Validation

All 50 Rust lifecycle tests and all 193 Python orchestration tests pass. Clippy
with all targets and warnings denied, scoped Rust formatting, diff checks,
skill validation and the skill-release catalog/synopsis check pass. The new
artifact, pre-start settings and unloaded-turn regressions failed against the
previous implementation before repair.

The artifact regression captures/revalidates a 32 MiB binary and rejects later
mutation. Other coverage checks large owned state/receipts, strict configuration
failures, missing/changed gate files, exact ACK-only behavior, output-log
conflicts, logged execution errors and no paid/native effect on refusal.

Read-only replays now attribute both previously unestablished retained turns:
UI harness 01a1a0ac-3fb2-7ed0-9ad5-78f607ee81df (Terra/medium/priority) and
backend repair 01a1a13d-6910-7eb0-a14c-d2b6dc958391 (Sol/high/default).
The completed unloaded review 01a1a0cd-5107-7840-be9f-3dcc189dafa0 was read using
only two domain read calls. Peer rollout bytes and historical dispositions were
preserved; these observations do not retroactively accept old operations.

The release binary's bind-only replay hashes the three original 29–31 MB builder
binaries to their original report digests, without creating dispatch state or
result directories. Compact inspection of an actual historical host state also
leaves its bytes unchanged. No new paid model turn was commissioned.

## Activated delivery

Skill release a8f9c305 is active and verifies identical, with 22 Codex and 11
Claude managed links intact. Only manual-orchestration's adapter, reference and
tests changed; names and install lists are unchanged. Automatic release notice:
coordination-board sequence 4194.

Qualified, read-only bundle for fresh host state:

`/home/bline/.local/state/work-engine/tools/manual-host/b03ad3a899b7323ed911a828d83bc87b5a34d95a579822776e2dbb488fd74569`

Binary SHA-256:
`b25c9bdff5593b4e08ce892a3f87f0924cb519210ed2659d9b326ba78a15b329`.
All 44 manifest files and the bundle identity verify against the clean committed
source. Build custody:
`/home/bline/.local/state/work-engine/manual-host-build.oSMrX2Rr/custody`.
Read and bundle proofs:
`/home/bline/.local/state/work-engine/manual-host-ordinary-use-repairs-20261103/ordinary-use-read-replays.json`
and `qualified-bundle-proof.json` in the same directory.

Existing operations retain their original executors and bundles; no peer state
or assignment was migrated. Running agents can reread the updated manual-host
reference and select the new bundle for new dispatches. This result does not
establish a time-savings percentage. Remote publication was not performed.
