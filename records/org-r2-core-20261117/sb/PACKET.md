# S-B builder packet: consumption seam and real discharges (fixture)

Role: one coherent builder (worker). Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Read the worktree `AGENTS.md`. Parent/sole collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; it observes your context/budget; no listeners, no children.

## Authority and inputs (paths under `/home/bline/cbm-work/`)

- Operator decision: `org-r2-core-20261117/OWNER-DECISION-R2REC.json`; reconciliation `org-r2-reconciliation-20261117/ROOT-SUMMARY.md` (S-B bullet) and `review/REVIEW.md`.
- S-A (accepted): `org-r2-core-20261117/sa/SA-ACCEPTANCE.json` — read its `sb_obligations` and `p3_carried`; rulings `sa/ROOT-RULINGS-1.md`; crate `rust/crates/work-engine-organization-authority` (public API in `src/gate.rs`, `src/observation.rs`, `src/fixture.rs`).
- R2-S (accepted, merged): `org-pi-slices-20261116/s3/S3-ACCEPTANCE.json` (P3s: ungated `run_attempt*`; publicly mintable, unbound `AssumedDischarges`); crate `rust/crates/work-engine-review-delivery` (`src/gate.rs`, `src/native.rs`, `src/binding.rs`), `rust/crates/work-engine-review-preparation` (`BlockerKind`).

Worktree: `/home/bline/cbm-work/wt-org-r2-core-20261117`, branch `org-r2-core-20261117`; its working tree must equal accepted S-A checkpoint `bfaa455c7a58dbee4ec19fba66ff442d687ef03a` at entry (S-A crate untracked; verify tracked diff and blob hashes against `git ls-tree -r bfaa455c`). Do not commit, stage or branch.

## Outcomes (invariants)

1. **Real discharges.** review-delivery depends on `work-engine-organization-authority` (its `fixture` feature only under review-delivery's `fixture`). The host-owned total `discharge` function maps `BlockerKind` to S-A `CheckId`s: Separation → O8 + R3; ComponentBinding → O4 + R3; AuthorityRequirement → O4; RealizationAdmission → R1 + R2 + R4; ObserverIdentity → R2 + R3. The three mapping kinds stay labelled fixture assumptions (O7). RealizationCheck stays never-dischargeable. Discharges are derived from an `EffectAdmission` the host obtains itself (key on `CheckId`, never on reason subjects), never supplied by a caller.
2. **Admission at delivery.** Before spawn, the host reserves a delivery boundary, takes `observe_for` from a fixture authority source, calls `admit::<Fixture>` and refuses before spawn (spawn 0, consumer 0) on any refusal or undischarged kind. It compares the admission's candidate, requirements, subject and realization digests with the binding (S-A F-7) and refuses on mismatch. The realization digest is computed by the host from its host-derived realization description (labelled fixture definition). No single-use epoch ledger; delivery single use remains the recorded O10 gap.
3. **Regate before consumption.** Immediately before `consume`, the host takes a **fresh** `observe_for` for a consumption boundary (never the admission-time observation) and requires `admission_state` = Current. The first non-current state other than Unknown is final for that admission (S-A F-3); Unknown fails closed. Revocation, supersession or a missing grant between spawn and consumption → consumer 0.
4. **Labels.** Successful `Delivered` carries the admission summary (check outcomes by id, revision, `Domain::Fixture` asserted) and the remaining labelled assumed discharges for mapping kinds. Assumed discharges are never called admissions.
5. **Close R2-S P3s.** `run_attempt` / `run_attempt_observed` stop being ungated public spawn routes (crate-private with a test-support export under `fixture`, or gated). `AssumedDischarges` becomes bound to the binding/preparation it was derived from and not publicly mintable.
6. **Fixture authority world.** Test support builds a fixture authority world (principals, vantage-actor bindings, component bindings, grants, decision scope, realization acceptance) matching the S3 assumption world; the frozen world still refuses before spawn.

Excluded: storage, issuance, lifecycle/hosted integration, S-C, organization or S-A crate semantic changes (S-A API additions only if strictly needed — report them).

## Evidence

- `/home/bline/cbm-work/org-r2-core-20261117/sb/MATRIX.md`: each invariant → named tests; one recorded mutant each (neutralize, run mapped test, restore exactly; records under `sb/mutants/`): discharge mapping per kind family, admission refusal before spawn, digest comparison, fresh-observation regate (replaying the admission observation must not satisfy it), revocation between boundaries, Unknown fail-closed, finality of non-current state, gated `run_attempt*`, unmintable `AssumedDischarges`.
- Keep S1/S3 matrix tests passing.
- Runs via `/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py run` into `sb/runs/` (env `CARGO_TARGET_DIR=/home/bline/cbm-work/org-r2-core-20261117/target`, `WE_PI_SDK_ROOT=/home/bline/cbm-work/pi-artifact-handoff-20261113/pi`, `WE_PI_SDK_MANIFEST=/home/bline/cbm-work/pi-artifact-handoff-20261113/pi.manifest.json`; model the plan on `/home/bline/cbm-work/org-pi-slices-20261116/s3/root-gate-plan-2.json`, cwd worktree `rust/`, and add the S-A crate's tests): fmt, clippy `-D warnings`, four crates + S-A crate with fixture features, default features, authority-store `--test store`, native suite once.

## Output

`/home/bline/cbm-work/org-r2-core-20261117/sb/RESULT.md`: entry check; files with reasons; diff stat vs `bfaa455c`; runs; matrix; deviations; unfinished; explicitly unclaimed (fixture standing; assumed mapping discharges; O10 gap; no hosted path; production admission refuses). Final message `map_ready`/`handoff_ready` + path.

## Constraints

Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions or protected configuration. No network, Docker, live provider, mutating git. Bash: simple commands (git -C, cargo, python3, rg, ls, head, cd). Targeted reads. The delivery-gate subsystem already has one rejection warning — prefer the smallest change that meets the invariants. Budget: target $9, cap $12; stop with honest `handoff_ready` near the cap or on a quota error.
