# S1 builder packet: make R1 acceptance evidence runnable and discriminating

Role: one coherent builder (worker). Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md` and load only `references/worker.md` and `references/call-cost.md`. Read the worktree's `AGENTS.md` and `docs/contracts/core.md`. Parent and sole collector/acceptor: Pi root session `01a1e56e-be12-7424-977b-c7c7983887d0`. No children. The parent owns observation of your context and budget from your transcript; do not start chatboard listeners.

## Objective and authority

R1 of `/home/bline/cbm-work/org-codex-recovery-plan-20261113/PLAN.md` (read its R1 section: bullets, **Discriminating checks**, **Exit**). The root has revised the acceptance model: read `/home/bline/cbm-work/org-pi-slices-20261116/s1/ROOT-S1-DESIGN-DECISION.json` and the design review it cites (`s1/design-recheck/REVIEW.md`, especially §3, §4 and "Product-code observations"). The private qualifier (`/home/bline/cbm-work/org-pi-review-continuation-20261114/realization-r9/evidence-gate`) and preparation/bootstrap scripts are frozen history: do not modify, run or maintain them.

Source: worktree `/home/bline/cbm-work/wt-p1-vertical-20261113` (branch `p1-pub-vertical-20261113`, HEAD `e20a9221`; its `rust/` tree equals rejected candidate3 `2617a6c6`). Verify clean status at entry and record HEAD. Main product crate: `rust/crates/work-engine-review-delivery`; neighbours on the R1 path: `work-engine-organization`, `work-engine-review-preparation`, `review-episode-core` (and the adapter/fixtures they use).

## Outcomes required

1. **P-1:** The native vertical tests (`tests/native_vertical.rs`, `tests/common/mod.rs`) run under plain `cargo test` without the `WE_RUN_DIR`/`WE_INVOCATION_ID`/`WE_SOURCE_MANIFEST_SHA256`/`WE_MAP_SHA256` pipeline variables. Remove qualifier bundle production from the acceptance path: tests assert in-process; raw frames/diagnostics may appear in failure output as ordinary diagnostics. Delete bundle-writing code that only fed the qualifier rather than keeping a parallel unused path (no compatibility consumer exists).
2. **P-2:** The selected Pi SDK root is an explicit input (not a hardcoded absolute path), with a clear refusal when missing or mismatched. The selected read-only SDK is `/home/bline/cbm-work/pi-artifact-handoff-20261113/pi` (manifest digest `cf9d096e61d8ffd2c35df238ff74e1f8d7dacfb9bc5813333beef6cc73010985`, SDK entry digest `4569157e6038be9840fa514db4b00a27fd0be9e3622cc7b49a281de6c027d7ff` per prior records). Make the pin say truthfully what it covers; if cheap, verify the handoff manifest too.
3. **P-3 (judge):** `Allocation` and `NativeObservation` decode without `deny_unknown_fields` (`src/lib.rs` ~176/195). If the adapter/producer contract makes extra members invalid (same class as fixed WR7-001), refuse them with a test; otherwise record why not.
4. **Discriminating-check matrix:** write `/home/bline/cbm-work/org-pi-slices-20261116/s1/build/MATRIX.md` mapping each R1 discriminating check (change one requirement → matching instruction/enforcement/refusal; altered result contract or subject refused; removed required evidence refuses acceptance; fake-provider mismatch retained; ordinary-admission refusals preserved) plus the cross-module consumer requirement, and the five product findings (IR-005 trailing frames refuse before consumption; WR7-001 duplicate JSON members; R6A-001 single deadline owner/collection; RLA-002 wait failure doesn't suppress cleanup; RLA-003 elapsed measured correctly) to named tests. For each matrix row, one recorded mutant: in a scratch copy outside the worktree (under `s1/build/mutants/`), remove/neutralize the product guard and show the mapped test fails; record the patch and the failing output. Add a test where a row has none.
5. Negative-control reliability (RL-001, RLA-001, RLA-004, RL-002): keep controls that are reliable; fix or remove intermittent ones; a wall-clock assertion with no owning requirement should be removed rather than tuned. Record each choice in `s1/build/RESULT.md`.
6. Run `cargo fmt --check`, `cargo clippy` for touched crates, and `cargo test --locked` for the four crates with the features the R1 path needs (find them; `fixture` is expected), recording the runs with `/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py run` (see worker.md) into `s1/build/runs/`. Run the native suite three times.

Do not commit, branch, stage or touch the real index; leave changes in the worktree. The root will checkpoint. Keep product semantics and ownership as-is otherwise (organization pure; preparation owns projection; delivery owns byte correspondence; review-episode-core owns result validation; no authority minted, ordinary admission still refuses). Do not broaden scope; report any further product defect you find instead of redesigning.

## Output

`s1/build/RESULT.md` (under `/home/bline/cbm-work/org-pi-slices-20261116/`): entry HEAD/status, files changed with one-line reasons, `git diff --stat`, test commands and outcomes with run record paths, matrix path, P-3 decision, control dispositions, anything unfinished, and explicitly unclaimed items (fixture standing, no hermetic build, no tamper evidence, no confinement). End with a short final message: `map_ready` or `handoff_ready` plus the RESULT path.

## Constraints

- Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials/OAuth, `~/.codex`, other `~/.claude` sessions, or protected configuration files' contents. Ordinary `cargo` invocations are fine; use `--offline` if the registry cache suffices. No network fetches, Docker, live providers, git commit/push/branch, or changes outside the worktree and `s1/build/`.
- `CARGO_TARGET_DIR=/home/bline/cbm-work/org-pi-slices-20261116/s1/target` to avoid the shared target.
- Budget: target about $7, cap $10 of model usage. If you approach the cap, stop and write an honest `handoff_ready` RESULT.
