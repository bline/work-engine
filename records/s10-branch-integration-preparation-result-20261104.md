# S10 branch integration preparation result

Recorded 2026-11-04. Standing: the three source lines are consolidated in an
isolated, tested integration candidate. The shared branch, index and working
files have not been changed. Shared publication remains the explicit approval
boundary in the original [integration plan](../planning/s10-branch-integration-plan-20261104.md#execution-boundary).

## Exact candidate and source standing

Worktree: `/home/bline/cbm-work/wt-s10-integration-20261104`.
Branch: `integrate/s10-branches-20261104`.
Source integration commit: `bb70a6f7a4ae4ef4ac354615828693602ee89a21`.
Source tree: `678c32540bb78f0040d5477b8b6cd36f21045f10`.
Expected shared destination: `a7f381ede293a59824189e2645b23f2fcc9a01b8`
on `rescue-detached-20260911`.

| Input | Exact subject | Owner standing |
| --- | --- | --- |
| Pi context observer | `02c2b438aed53dad9de92388958e8a3a4a2191ec`, including source `6f0ecd4cc61eced23893acbf44030f396eac92cf` and explicitly proposed planning records | Accepted observer source; board 4398. Planning inclusion grants no pressure-slice authority. |
| Backend/delivery ancestry | `5c106a281d64cab368c8c54dac7a0f0b57d9f09b` | Final backend owner acceptance and independent recheck; board 4410. |
| Shared query/CLI | `ad1bdb5b4f74c36cc44c0dceb1fd8c43d9f69c00` | Final query owner acceptance and reconstructed read-only Astra recheck; board 4412. |

Observer merge commit `59573e2e422d68aabae78267f10bc9836aff9e65` is
separately available if publication should begin with the observer alone.
The next merge staged the backend ancestry once, followed by the query merge.
No source branches or worktrees were removed.

Owner handoffs remain authoritative for their exact subjects and limits:

- `/home/bline/.local/state/work-engine/s10-pi-pressure-20261104T2034/INTEGRATION-HANDOFF.md`
- `/home/bline/.local/state/work-engine/dot-signed-pi-fixture-20261104T1949/handoff/INTEGRATION-HANDOFF.md`
- `/home/bline/.local/state/work-engine/rust-shared-cli-read-20261104T2036/INTEGRATION-HANDOFF.md`

Their acceptance does not automatically accept this different composition.
The integration owner's direct source review and checks below qualify the
local candidate. No separate independent review of the composition is claimed.

## Conflict resolution

The two content conflicts were `rust/Cargo.toml` and `rust/Cargo.lock`.
The resolution retains `session-delivery`, `session-task-admission` and
`session-query`. The lockfile is the exact union of the backend entries and
the incoming query entry; no registry package version or checksum changed.
Cargo 1.92.0 accepted it offline with `--locked`.

The clean Python merge retains the backend changes and the query owner's
full-entry credential recognition before pagination. The only additional query
source edit is its exact compositor hash pin, also reflected in its README:
`d5d51ab3a870a6efa82f5824a4e3cb9ba5468263724f6068c0d46b5ec586b8a2`.
The query helper remains byte-identical to the accepted input.

The candidate's 97 changed paths equal the union of the three incoming deltas.
Outside the five composition paths (both Cargo files, compositor, query library
and query README), all incoming files match their respective source revisions.
The observer and its pressure dependency bytes retain their original boundaries.

## Verification on the actual composition

| Check | Result |
| --- | --- |
| Cargo 1.92.0 offline locked metadata/build | Passed |
| Affected Rust libraries/tests: query, delivery, task admission and manual host | 164 passed, 12 explicitly ignored |
| Standalone observer | 5 passed |
| Python session-events suite with newly built fixture binaries | 168 run, 164 passed, 4 skipped |
| Actual four-query CLI/MCP/Rust-bridge parity fixture | Passed, including human controls, redaction windows, partial/unavailable observations, authorization negatives and repeated ambiguous reads |
| Incoming path union, preserved source bytes, lockfile union, compositor pin and Git whitespace checks | Passed |

The initial Python attempts lacked fixture binaries and then exceeded Unix
socket pathname limits under a long private fixture root. After building the
required binaries and selecting a short private root, the full configured
suite passed. Failed attempts remain in the evidence directory. The complete
Rust suite passed its long-running two-stage admission test; an unnecessary
30-second diagnostic rerun timed out after spending 28.79 seconds compiling.
It does not negate the original passing suite.

These are local offline checks. No live provider, Docker activation, production
deployment, full S10 acceptance, hermetic build or new recipient acknowledgement
is claimed. Existing component qualification retains its original scope.

## Shared work preservation and publication preparation

The shared checkout still has the original 99 Git status entries: 19 modified
tracked paths and 80 compact untracked entries containing 221 files. A private
snapshot retains their bytes, file modes, index, staged/unstaged binary patches,
HEAD and status. The captured file set, original index bytes, status and shared
HEAD were rechecked unchanged after integration and tests.

Snapshot and evidence root:
`/home/bline/.local/state/work-engine/s10-integration-20261104T2131/`.
The restoration archive is `preservation/dirty-files.tar`; its exact file/hash
inventory is `preservation/manifest.json`. This is preservation evidence, not
authority to commit any unrelated contents.

Seven actual incoming paths overlap the expanded dirty file inventory:

| Paths | Prepared disposition; not applied |
| --- | --- |
| `rust/Cargo.toml`, `rust/Cargo.lock` | Clean three-way working-file overlays retain the local publication/completion packages and dependency declarations together with incoming packages. Saved under `preservation-reconciliation/`. These unrelated local additions are excluded from the committed integration candidate. |
| Manual-host `src/lib.rs`, `src/main.rs`, `examples/pi_docker_fixture.rs` | Existing local bytes already equal incoming bytes; originals remain archived. |
| Manual-host `src/pi_docker.rs`, `docs/pi-docker.md` | Both existing files exactly match retained checkpoint `ae6151fcdc2bd21274a95907d447947f5e1cdb3d`, identified by the existing host-component result record. Apply accepted incoming versions during publication and retain originals in the snapshot/checkpoint; no unique local delta exists against that checkpoint. |

The proposed Cargo working-file union parses without duplicate members or
lockfile packages. It has not been built or accepted as unrelated product work;
the committed clean candidate is the source that passed the checks above.
Publication must recheck the shared head and affected bytes before applying
these dispositions. No stash, reset, clean, shared mass commit or overwrite has
been performed.

## Older branch reconciliation

The three nested predecessors (delivery foundation, delivery pressure and
S10 coding) are ancestors of the backend input and are included once.
`explore/manual-host-docker-20261103`,
`fix/manual-host-large-state-20261103` and
`codex/mcp-catalog-staging-repair-20261103` contain five patch-equivalent
commits already represented at the shared destination; none were replayed.
All branches remain retained. Other historical branches were not retired or
declared redundant by this bounded reconciliation.

## Evidence and next decision

The external evidence directory contains `composition.json`,
`composition-verification.json`, `cargo-resolution.json`, test commands/logs,
`branch-reconciliation.json`, `dirty-overlap.json`,
`manual-host-baseline-reconciliation.json`, `preservation-verification.json`
and an integrity manifest. Graph project `home-bline-code-work-engine`,
generation `ud97738bd647682ddg2`, supplied Tier 2 baseline relationships and
coverage. Private candidate symbols were missing from that watched shared
index, so exact Git and source reads supplied private-source evidence.

The remaining decision is whether to publish this exact integration candidate
to the shared branch using the prepared preservation dispositions. Reconfirm
the expected destination and active claims at execution. No push, branch or
worktree cleanup, AH merge, runtime activation, deployment or admission of the
proposed $15 pressure slice is included. AH and the Web adapter retain their
separate owners, source acceptance and grants.
