# S3 builder packet: R2-S refusal seam with assumed discharges

Role: one coherent builder (worker). Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Read the worktree `AGENTS.md`. Parent, sole collector/acceptor: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; it observes your context/budget from your transcript; start no listeners; no children.

## Authority and design

Owner-approved R2 design: `/home/bline/cbm-work/org-pi-slices-20261116/s2/OWNER-DECISION-S2.json` (O1–O11 approved, O11 = build this slice). Read `s2/ROOT-S2-ADDENDUM.md` (item 10 "R2-S", items 1–3, 6) and `s2/review/REVIEW.md` §2 F-1, F-2, F-5 and §7 "R2-S". R1 acceptance and its limits: `s1/S1-ACCEPTANCE.json` (`semantic_limit`, F-7, F-8). Paths are under `/home/bline/cbm-work/org-pi-slices-20261116/`.

Source: worktree `/home/bline/cbm-work/wt-p1-vertical-20261113`, branch `p1-pub-vertical-20261113`, HEAD `e6a57e7c` (clean; verify and record). Crates: `work-engine-review-preparation`, `work-engine-review-delivery` (+ `adapter/`), test fixtures; `work-engine-organization` compiler is **unchanged** (use it to compile the second world).

## Outcome (invariants that must hold)

1. **Closed blocker kinds.** Preparation exports a closed `BlockerKind` alongside the existing string ids (one kind per id family: scope-digest-to-subject, problem-scope-to-episode-identity, consumption-evidence-mapping, separation, observer-identity, realization-admission, component-binding, authority-requirement, realization check). `prepare`'s meaning is unchanged; every emitted blocker has exactly one kind.
2. **Host-derived realization.** The delivery host derives the realization description of what it will actually run from its native configuration (provider-disabled Pi SDK child through the adapter; not confined; no guarded sandbox; provider family not the compiled family) and passes it into preparation, so `realization_evaluation` exists on the delivered path. Callers cannot supply or override it. Describe it truthfully; do not invent vocabulary the v1 schema lacks—use the existing descriptor/disposition machinery.
3. **Refusal before spawn and before consumption.** Delivery refuses before any child is spawned when any blocker is a `KnownIncompatibility`, or any blocker kind lacks a discharge. Immediately before consumption the same evaluation runs again and refuses on failure. On refusal: spawn count 0 (pre-spawn) and `consumer_invocations() == 0`. No public route spawns the SDK child without passing this gate (make `run_native` non-public or require the gate's output; update tests accordingly).
4. **Assumed discharges (labelled, host-derived).** Discharge is a total function from `BlockerKind` to an outcome, owned by the delivery host (or a small module beside preparation), derived—never a caller-supplied map. In this slice the only discharges are a labelled fixture **assumption set** with the standing of `FixtureDeclaration` (name it e.g. `AssumedDischarge`, never "admission"). Root ruling for the assumption set: assumable kinds are the three mapping rules, separation, observer-identity, component-binding, authority-requirement and realization-admission; **realization check blockers (`realization:<cond>:<property>`, KnownIncompatibility or Unestablished) are never assumable**. If source shows a kind does not fit this ruling, stop that item and report rather than redesign.
5. **Second world.** A labelled synthetic world: the frozen requirements without `realizationConstraints`, compiled by the unchanged v1 compiler, stored as a test fixture with its derivation documented. The native suite's positive and failure-path tests move to it. The frozen world becomes the negative: refused before spawn by known realization incompatibility through the real host path (spawn 0, consumer 0). Label it an assumption world (the child is not confined; `mutation: none` is assumed, not true).
6. **Discriminating evidence.** Update `/home/bline/cbm-work/org-pi-slices-20261116/s3/build/MATRIX.md` (new) mapping each invariant above to named tests, with one recorded mutant each (neutralize the guard, show the mapped test fails, restore; record patch and failing output under `s3/build/mutants/`): pre-spawn refusal, pre-consume re-check, unknown/undischarged kind refusal, non-assumable realization kinds, host-derived realization (caller cannot bypass). Keep the S1 matrix rows passing on the second world.

Excluded: authority records, Gate O/Gate R, stale/revoked/unknown states, epochs/single-use ledger, storage, any production admission path, organization compiler changes.

## Validation

With `/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py run` into `s3/build/runs/` (env `CARGO_TARGET_DIR=/home/bline/cbm-work/org-pi-slices-20261116/s3/target`, `WE_PI_SDK_ROOT=/home/bline/cbm-work/pi-artifact-handoff-20261113/pi`, `WE_PI_SDK_MANIFEST=/home/bline/cbm-work/pi-artifact-handoff-20261113/pi.manifest.json`; reuse `/home/bline/cbm-work/org-pi-slices-20261116/s1/root-gate-plan-2.json` as a model with cwd = worktree `rust/`): fmt, clippy `-D warnings`, four crates with fixture features, default-feature delivery+preparation, authority-store `--test store`, native suite once (root repeats from a clean export).

## Output

`s3/build/RESULT.md`: entry HEAD/status; files changed with reasons; `git diff --stat`; runs and outcomes; matrix path; deviations; anything unfinished; explicitly unclaimed (fixture standing only; assumed discharges are not admissions; no Gate O/R; no confinement; production admission still refuses). Do not commit, stage or branch. Final message: `map_ready` or `handoff_ready` + path.

## Constraints

Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions, or protected configuration contents. No network, Docker, live provider, git mutation. Keep context lean: use targeted reads (rg, line ranges) instead of rereading large files. Budget: target $9, cap $12; if approaching the cap, stop with an honest `handoff_ready`.
