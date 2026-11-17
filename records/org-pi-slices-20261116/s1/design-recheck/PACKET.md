# S1 design recheck: is R1's acceptance model credible? (Fable xhigh, read-only)

Role: fresh, separate architectural/design reviewer under the campaign's architecture-escalation guard. Read `skills/manual-orchestration/SKILL.md` (repository path, /home/bline/code/work-engine) and load the worker/reviewer role reference only. Read `AGENTS.md` and `docs/contracts/core.md`; load `docs/contracts/evidence-and-claims.md` §3.9 and `docs/contracts/system-design.md` as needed for the judgment.

Parent/collector: Pi root session `01a1e56e-be12-7424-977b-c7c7983887d0` (supervisor; sole acceptance and design-disposition authority). The operator replaced Astra with Fable xhigh as the reviewer for this campaign.

## Objective (owner-approved)

Rust organizational compilation/execution. Long-term plan: `/home/bline/cbm-work/org-codex-recovery-plan-20261113/PLAN.md`. The current slice is **R1** ("connect one compiled contract to execution and result validation", fixture-only, Pi 1.5 SDK provider-disabled, ending at the existing review consumer). Read the R1 section and its **Exit** and **Discriminating checks** carefully: they are the owner-approved acceptance basis.

## History you must weigh

`N=/home/bline/cbm-work/org-pi-review-continuation-20261114`.

- Three implementation candidates were rejected (candidate1 R7, candidate2 R8, candidate3 `2617a6c6` R9). 21 findings are open (`N/architectural-risk-history.json`, revision 37; gate lists the ids). All are P2.
- Plans R7–R10 (`N/PLAN-R7.md` … `N/PLAN-R10.md`), design decisions `N/ROOT-R*-DESIGN-DECISION.json`, the previous design review `N/architecture-r6/REVIEW-MESSAGE-73.md`, and the realization blocker `N/R10-REALIZATION-ASSESSMENT.md`.
- Latest rejection reports: `N/reviews-r9/C/REVIEW-02-MESSAGE-04.md`, `N/reviews-r9/D/REVIEW-01-MESSAGE-57.md`, `N/reviews-r9/A/ASTRA-REVIEW-02.md`. Earlier: `N/reviews-r8/*/ASTRA-IMPLEMENTATION-REVIEW-R8.md`.
- Campaign guard: `N/ARCHITECTURE-GUARD.md`. Cost so far across windows: roughly $280 API-equivalent with no accepted R1.
- Source: product crate `/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery` (plus related crates in that worktree, branch `p1-pub-vertical-20261113`); private qualifier `N/realization-r9/evidence-gate`; preparation/bootstrap `N/implementation-r6/preparation-03`.

## The question

The root observes that most open findings target the **private evidence-gate qualifier and build-input custody** (e.g. CR9-001: a hand-substituted diagnostic file with a consistently updated manifest still passes; R9D-001: dep-info does not prove complete material build inputs; AR10-004: realizing complete build-input closure requires a Cargo/config route that avoids protected configuration). Hostile-writer, hermeticity and confinement claims are explicitly *excluded* holds, yet the qualifier is repeatedly rejected for not detecting coherent artifact substitution, and each repair grows the parallel appraisal surface. This may be architectural blindness at the level of the acceptance model rather than the product seam.

Judge, with evidence:

1. **Which claims does R1 acceptance actually require**, and who owns each? For each production-path claim the current plans impose (complete material build-input inventory, executable custody joins, qualifier re-appraisal of every emitted artifact, cold/common build reconciliation), apply §3.9: is it required by an authority/semantic owner other than the local constraint, with a nameable consumer failure? Or is it root-authored route ceremony that can be revised by the root without narrowing an owner's claim?
2. **Is the R10 local correction credible** as a converging repair, or does the qualifier-as-parallel-validator structure guarantee further findings? Say what evidence would show convergence.
3. **Minimal truthful acceptance model** for R1: what subject identity, test/discriminating-check evidence, and independent semantic review would establish R1's exit without false claims? What must remain explicitly unclaimed (fixture standing, no hermeticity, etc.)?
4. **Disposition of the 21 findings** under that model: which concern product semantics (must be repaired in product code), which concern only machinery the model would drop (record as superseded with reason, never silently closed), and which remain open questions.
5. **Bounded next scope** for S1 that fits roughly $40–50 total (build + qualification + independent review + remediation), or state that it cannot.

Be concrete about product-code defects you find; do not perform a full implementation review. State confidence and what you did not inspect.

## Constraints

- Read-only except your single output file. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials/OAuth, configuration, or other agents' sessions under `~/.codex` / `~/.claude`. No builds, tests, network, Docker, or git mutation.
- Budget: target about $8 of Fable usage; be economical—read selectively, prefer `rg` and targeted reads over whole large JSON files (`*-exact-turn.json`, `cost-*.json`, `board-*.json` are telemetry; skip them).
- Write your complete report to `/home/bline/cbm-work/org-pi-slices-20261116/s1/design-recheck/REVIEW.md`, then end with a short final message giving the verdict line and the report's path. Verdict line one of: `retain_R10_local_correction`, `revise_acceptance_model`, `reopen_architecture`.
