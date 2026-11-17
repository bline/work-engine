# S1 independent semantic review of R1 candidate (Fable xhigh, read-only)

Role: fresh, separate adversarial reviewer. Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md` and load only `references/worker.md` (reviewer role) and `references/call-cost.md`. Read `/home/bline/code/work-engine/AGENTS.md` and `docs/contracts/core.md`; load `docs/contracts/evidence-and-claims.md` only where a claim's standing matters. Parent, sole collector and acceptance authority: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`. The parent owns your context/budget observation; start no listeners. Independence claimed: separate session and context only (same provider family as builder).

## Subject (exact)

- Private checkpoint `1ac0bff4190cf757b5d79b5e69f04fbdcbed2124`, tree `a6a51e2e191f2762fd9023590c3f357bda4122b5`, task patch digest `767bd949003604880a07b52fc2ea2d2f1b421c05558facf1c8b98afb49393d2d`, plan_version `sha256:efcdc01d74cc0c2b1d48c043a16040c70d618f3583985fbb4d02920fcf1983d5`, scope `s1-r1-revised-acceptance-model`, gate digest `86296208522483a3189fd4cfa84566103e1b1ea60b1c6c9bc897641a4f951dfb` (builder final-2 summary). Ref `refs/work-engine/checkpoints/org-pi-slices-20261116/slice-1/candidate-1` in repo `/home/bline/cbm-work/wt-p1-vertical-20261113`.
- Clean export of that tree (read source here): `E=/home/bline/cbm-work/org-pi-slices-20261116/s1/export-1ac0bff4/rust`. Task patch vs baseline `e20a9221`: `git -C /home/bline/cbm-work/wt-p1-vertical-20261113 diff e20a9221 1ac0bff4`.
- Root clean-export gate (custody records, read-only): `/home/bline/cbm-work/org-pi-slices-20261116/s1/root-gate-1/` and its summary `s1/ROOT-GATE.json`.

## Acceptance model (root decision; do not relitigate unless you find it untruthful)

`/home/bline/cbm-work/org-pi-slices-20261116/s1/ROOT-S1-DESIGN-DECISION.json` and its design review `s1/design-recheck/REVIEW.md`. R1 is accepted on: exact subject; recorded `cargo test --locked` from clean export; a discriminating-check matrix with mutants; this semantic review. Custody, the frozen private qualifier and build-input closure are **out of scope** and withdrawn as claims; do not raise findings that require them. Owner-approved basis: R1 section of `/home/bline/cbm-work/org-codex-recovery-plan-20261113/PLAN.md` (bullets, Discriminating checks, Exit).

## Review scope

1. Does the product seam meet R1: compiled meaning (OrganizationView) → preparation projection (obligation/subject, result-contract id/version, requirements, authority refs, continuity/independence, evidence refs, permitted effects) → delivery to the genuine provider-disabled Pi 1.5 SDK session → implementation-review/v1 validation by review-episode-core → fixture consumer, through the same owners, with no second route to consumption and no authority minted? Crates: `work-engine-organization`, `work-engine-review-preparation`, `work-engine-review-delivery` (incl. `adapter/review-delivery-adapter.mjs`), `review-episode-core`.
2. Each R1 discriminating check and the cross-module requirement: is it actually exercised by a real cross-module consumer test, and does the mutant evidence in `s1/build/MATRIX.md` show discrimination? Note the builder applied mutants in place and reverted them (see `s1/build/RESULT.md`); judge whether the evidence is adequate, and check the result-contract half of the "altered result contract or subject" row that has tests but no mutant.
3. Confirm or refute the five product findings' repairs at this subject: ORG-PV-IR-005 (trailing frames refuse before consumption), ORG-PV-WR7-001 (duplicate JSON members), ORG-PV-R6A-001 (single deadline owner/collection), ORG-PV-RLA-002 (wait failure does not suppress cleanup), ORG-PV-RLA-003 (elapsed measured after finalization). Prior reports: `/home/bline/cbm-work/org-pi-review-continuation-20261114/architectural-risk-history.json` (events) and reviews under that directory; read selectively.
4. The new changes: strict decode (P-3) and its producer basis; SDK root/manifest input and what the pin truthfully covers; deletion of bundle-writing code; removal of the RL-002 wall-clock assertion; `hang` control bounds; disposition of RL-001/RLA-001/RLA-004 controls. Any lost coverage from deleted tests?
5. P-6: does `ReviewResult::parse` / validation refuse a result for another contract version or subject?
6. Label truthfulness: `FixtureObservationOnly`, `NotEstablished`, provider-disabled basis, refusal of ordinary admission, and the explicit-unclaimed list in RESULT.md.

Severity: P1 / P2 (acceptance-blocking correctness, authority, provenance or evidence defects) / P3. Give file:line evidence, a concrete counterexample or reasoning, and confidence. Distinguish executed facts (from records) from static reasoning. Do not perform builds or tests.

## Output

Write the complete report to `/home/bline/cbm-work/org-pi-slices-20261116/s1/review/REVIEW-1.md` (use the Write tool; this path is permitted). If writing fails, put the whole report in your final message. First line: verdict `accept_private_readiness` or `changes_required`. Include coverage (what you read / did not read). End with a short final message: verdict and path. You may be retained for a recheck after remediation.

## Constraints

Read-only except the output file. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials/OAuth, configuration contents, `~/.codex`, or other `~/.claude` sessions. No builds, tests, network, Docker, git mutation. Budget: target about $14 of Fable usage; read selectively (skip telemetry JSON such as `*-exact-turn.json`, `cost-*.json`, `board-*.json`).
