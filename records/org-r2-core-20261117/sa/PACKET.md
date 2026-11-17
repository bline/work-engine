# S-A builder packet: pure organizational admission core

Role: one coherent builder (worker). Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Read the worktree `AGENTS.md`, `docs/contracts/core.md`, `docs/rust-development.md`. Parent/sole collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; it observes your context/budget; no listeners, no children.

## Authority

Owner-approved R2 design and amendments: `/home/bline/cbm-work/org-pi-slices-20261116/s2/OWNER-DECISION-S2.json`, `s2/ROOT-S2-ADDENDUM.md` (items 3–9), design `s2/design/PROPOSAL.md` §3–§5, review `s2/review/REVIEW.md` F-2..F-5, F-8, F-10; reconciliation `/home/bline/cbm-work/org-r2-reconciliation-20261117/ROOT-SUMMARY.md` and `review/REVIEW.md`; this slice's decision `/home/bline/cbm-work/org-r2-core-20261117/OWNER-DECISION-R2REC.json`. Prior design text reused (read only the cited ranges): `/home/bline/cbm-work/org-rev6-20261112/STAGE0-CONTRACT-r6.md` §3.3 (l.372–424), §4.3 (l.577–728), §18 (l.1538–1635); `/home/bline/cbm-work/org-rev6-20261112/AMENDMENT-6.1.md` §3–§6 (l.144–250).

Worktree: `/home/bline/cbm-work/wt-org-r2-core-20261117`, branch `org-r2-core-20261117`, HEAD `561c6ffc` (clean; verify and record). Do not commit, stage or branch.

## Outcome: new crate `rust/crates/work-engine-organization-authority` (pure)

Dependencies: `work-engine-organization` (read its view/enforce report only; it stays unchanged), serde, sha2, thiserror as needed. No filesystem, SQLite, network, clock reads (time is an input), lifecycle or manual-host dependency. Add to the workspace.

1. **Records** (rev6 §18 with 6.1 F4/F5; property-defined identities per review F-10): `Principal`, `VantageActorBinding`, `ComponentBinding`, `DecisionScope` (no `decisions` field), `RealizationAcceptance`, grant/authority coverage as needed for O4. `IssuanceId` unique per act of issuing, separate from `BodyDigest`; a totally ordered `ObservationRevision` with a state digest. Fixture constructors only under a `fixture` feature (no production constructor; follow `work-engine-authority-store/src/fixture.rs` pattern).
2. **Observation** (addendum item 5): an `AuthorityObservation` trait whose contract states the four source obligations (committed only once durable; revisions never retracted and totally ordered; uncertain revocation keeps fencing; one observation is one state). An in-memory fixture source supporting committed/revoked/superseded records and `Unknown` injection.
3. **Reservation-sealed observation** (6.1 F3, addendum item 3): `reserve(boundary)` → `observe_for(&reservation)` binds the boundary into the observation; gates accept only an observation sealed to the reservation they decide; a later boundary cannot reuse it. Test the same-observation/later-boundary negative (6.1 B12b) in this crate.
4. **Gate O subset** O3, O4, O5, O8, O9 and **Gate R** R1–R4 (addendum item 9), every applicable check evaluated, an outcome vector per check id. Production policy refuses `binding_contract_unqualified` / v1 O5 as the design states; positive results only in the fixture domain.
5. **Admission and states**: an `EffectAdmission` binding principal, actor, subject, operation, boundary, authority revision and realization reference; `admission_state()` → current / stale / revoked / unknown / refused with the consequences in the approved contract. Check outcomes are exposed **by check id** so a host can derive discharges (S-B maps `BlockerKind` → check ids in review-delivery; this crate must not depend on review-preparation).
6. **Oracles**: tests follow 6.1 §6 oracle classes (no observation → no check outcomes; legitimately co-failing checks; production all-refuse), not a universal paired-control rule (review F-3).
7. Excluded (held or later): `GrantProjection`, `realization_digest`, any issuer/installation, storage, Gate O O1/O2/O6/O7, lifecycle integration, review-delivery changes.

## Evidence

- `/home/bline/cbm-work/org-r2-core-20261117/sa/MATRIX.md`: each outcome above → named tests, one recorded mutant per guard (neutralize, show the mapped test fails, restore exactly; record patch + output under `sa/mutants/`), at least: seal mismatch / later-boundary reuse, each Gate O/R check (one representative mutant per check family acceptable), each non-current state, Unknown fail-closed, production refusal.
- Runs via `/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py run` into `sa/runs/` (env `CARGO_TARGET_DIR=/home/bline/cbm-work/org-r2-core-20261117/target`): `cargo fmt --check -p work-engine-organization-authority`, clippy `--all-targets --all-features -D warnings` for the crate, `cargo test --offline --locked -p work-engine-organization-authority --all-features` and default features, plus `cargo test --offline --locked -p work-engine-organization` (unchanged). If `--locked` fails only because the new crate's lock entry is missing, run once without `--locked` to update `Cargo.lock`, record it, then use `--locked`.

## Output

`/home/bline/cbm-work/org-r2-core-20261117/sa/RESULT.md`: entry HEAD/status; files with reasons; `git diff --stat` + new files; runs; matrix; design choices where the specs left room; unfinished; explicitly unclaimed (fixture standing; no storage, issuance or production admission; observation obligations are contract text, not implemented by any real source). Final message: `map_ready` or `handoff_ready` + path.

## Constraints

Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions or protected configuration. No network, Docker, live provider, git mutation. Targeted reads (rg, line ranges). Prefer the simplest code that satisfies the contract; no machinery whose only consumer is its own test. Budget: target $10, cap $13; stop with an honest `handoff_ready` near the cap. The subscription has a 5-hour usage window; if a request fails for quota, stop and write RESULT.md.
