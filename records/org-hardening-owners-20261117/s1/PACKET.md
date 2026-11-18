# S1 builder packet: post-R2 gate hardening (fixture)

Role: one coherent builder (worker). Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Read the worktree `AGENTS.md` and `docs/contracts/core.md`. Parent/sole collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; it observes your context/budget; no listeners, no children.

## Authority and inputs (paths under `/home/bline/cbm-work/` unless absolute)

- Operator 2026-11-17 approved this hardening slice (fixture only) under the objective `org-hardening-owners-20261117/assignment.json`.
- Accepted S-B: `org-r2-core-20261117/sb/SB-ACCEPTANCE.json` (`p3_carried`, `carried_open`), `sb/MATRIX.md` (rows 1–6 + T1–T7; the "Status of rows 1-6 after remediation 1" paragraph), `sb/mutants/mutate.py` + `mutant-plan-1.json`/`-2.json` (M1–M11 specs), `sb/remediation-1/RESULT.md` (§4 inventory as implemented, "not done"), `sb/design-review/REVIEW.md` §4–§8 (P3-3 explains the two routes around the gate), `sb/review/REVIEW-1.md` P3-3.
- Crates: `rust/crates/work-engine-review-delivery` (`src/gate.rs`, `src/native.rs`, `src/binding.rs`, `src/lib.rs`, tests), `rust/crates/work-engine-review-preparation` (`src/fixture.rs` `consume_fixture_result`, `tests/fixture_consumer.rs`), `rust/crates/work-engine-organization-authority` (S-A; public API `src/gate.rs`, `src/observation.rs`, `src/fixture.rs`).

Worktree: `/home/bline/cbm-work/wt-org-hardening-20261117`, branch `org-hardening-20261117`, clean at `0fe9dc74` (contains merged R2 `a765298e`). Verify clean at entry. Do not commit, stage, branch or run mutating git.

## Outcomes (invariants)

1. **No consumption without a gate-issued proof.** In every build (default, `fixture`, any feature combination you introduce other than one explicitly named test-only feature), the preparation crate's fixture consumer cannot advance an episode unless the caller presents a proof that the consumption-boundary gate found the admission current for *this* candidate/subject/binding. The proof must be unforgeable outside its issuing owner (not publicly constructible, not `Clone`-mintable from parts, not satisfiable by a proof issued for another candidate, subject or boundary) and checked by the consumer against its own inputs before the episode transition. You choose the route; state it in a short design note in RESULT with the alternatives you rejected. A plausible route: the preparation crate depends on `work-engine-organization-authority` (no cycle: S-A depends only on `work-engine-organization`) and the consumer requires an S-A-issued value (S-A types are constructible only by S-A) whose request identity (subject, candidate digest, vantage) and boundary kind the consumer compares with its candidate/declaration; any "current at consumption" proof would then need an S-A-issued regate result. S-A API **additions** are allowed only if strictly needed and must be reported; no S-A semantic change. If no route meets this without changing S-A semantics or the dependency graph beyond one added edge, stop and report (`handoff_ready`) with the options — do not narrow the claim silently. Review-delivery's gated path must keep working and pass the proof through.
2. **`test_support` behind a separate non-default feature** not implied by `fixture` (e.g. `test-support`), so in a `fixture` build the gated delivery path is the only spawn route by `cfg`. Update the absence sentences (`native.rs` module doc ~14–24, ~593/616, ~1463–1468/1509–1514, `lib.rs`) to what is now true; say that a feature is a statement of build intent, not access control. Tests needing the generic runner enable the feature via `required-features`.
3. **Delivery regate revision carried.** `Delivered`'s delivery-boundary `AdmissionSummary.regate_revision` is `Some(revision of the fresh delivery regate before run_native)`; consumption stays `Some`. Positive test asserts both and that each differs from (follows) its admission revision where the scripted source makes that observable.
4. **S-B mutants re-established on corrected code.** Re-specify and re-run M1–M4 and M6–M11 against the current text (M5 stays retired as a replaced claim). Each must be killed by its mapped test; if a mutant's claim was dissolved by remediation 1, record it as a replaced claim with the reason rather than forcing it. Add one mutant per new claim in 1–3 (at least: consumer accepts a missing/other-subject/other-candidate proof; consumer skips the proof check; `test_support` visible under `fixture` alone — a compile_fail doctest or `cfg` check is acceptable; delivery regate revision dropped).
5. Optional only if cheap (≤ $0.5 together): SB `p3_carried` "T5 asserts [O4] not AuthorityExpired reason" (assert the reason). Otherwise leave carried.

Acceptance frame (from S-B design review §4): for the consumer's new proof input, and any input you add, give one inventory row: host-derived / digest-bound / labelled caller assertion. No input may be an unlabelled caller assertion.

Excluded: storage, issuance, single-use ledger (O10 gap stays), hosted/lifecycle integration, S-C, O7 mapping owners, organization anchor, production admission.

## Evidence

- `org-hardening-owners-20261117/s1/MATRIX.md`: invariant → named tests → mutant → result; plus the re-established M-rows (old id, new spec, result).
- Mutants: copy and adapt `org-r2-core-20261117/sb/mutants/mutate.py` to `org-hardening-owners-20261117/s1/mutants/` (exact-once replacement, original bytes + sha256 saved, exact restore; pre/post hash steps). Run via `/home/bline/code/work-engine/skills/manual-orchestration/scripts/custody.py run` into `s1/mutants/runs-N/`.
- Gate via custody into `s1/runs/`: model the plan on `org-r2-core-20261117/sb/root-gate-plan-2.json` with cwd `/home/bline/cbm-work/wt-org-hardening-20261117/rust`, env `CARGO_TARGET_DIR=/home/bline/cbm-work/org-r2-core-20261117/target` (reused to save build time), same `WE_PI_SDK_*`; steps fmt, clippy (fixture; default; and with the new test-support feature), four crates + S-A with fixture features, default features, authority-store `--test store`, native suite once. Add the preparation crate under any feature combination your change introduces.

## Output

`org-hardening-owners-20261117/s1/RESULT.md`: entry check; design note (route, rejected alternatives, dependency edge added if any, S-A additions if any); files with reasons; diff stat vs `0fe9dc74`; runs; matrix summary; inventory rows; deviations; unfinished; explicitly unclaimed (fixture standing; assumed mapping discharges; O10 gap; caller-asserted organization id; no hosted path; production admission refuses; a feature is build intent, not access control). Final message `map_ready`/`handoff_ready` + path.

## Constraints

Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions or protected configuration. No network, Docker, live provider, mutating git. Bash: simple commands (git -C, cargo, python3, rg, ls, head, cd). Targeted reads; do not read whole large files when a range suffices. Prefer the smallest change that meets the invariants. Budget: target $6, cap $8; stop with honest `handoff_ready` near the cap or on a quota error.
