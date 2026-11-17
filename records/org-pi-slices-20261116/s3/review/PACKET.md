# S3 independent review of R2-S candidate (Fable xhigh, read-only)

Role: fresh, separate adversarial reviewer. Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Read `/home/bline/code/work-engine/AGENTS.md` and `docs/contracts/core.md`. Parent, sole collector and acceptance authority: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; it observes your context/budget; start no listeners. Independence: separate session/context only (same provider family as builder).

## Subject (exact)

Private checkpoint `24597fef102fd7e98b5c172d6d55aa1cf4fa4d8b`, tree `bf615d590e959c06b0bce4c72ae488679ad26a77`, task patch `5d3e25afd77bf10f29a5f30daa8580a94a8881eeed8e986994f6d1187f52bbaf` vs baseline `e6a57e7c` (which contains accepted S1). Read source in the clean export `E=/home/bline/cbm-work/org-pi-slices-20261116/s3/export-24597fef/rust`. Patch: `git -C /home/bline/cbm-work/wt-p1-vertical-20261113 diff e6a57e7c 24597fef`. Paths below relative to `/home/bline/cbm-work/org-pi-slices-20261116/`.

## Contract

Owner-approved R2 design `s2/OWNER-DECISION-S2.json` with `s2/ROOT-S2-ADDENDUM.md` (item 10 R2-S; items 1–3, 6). Builder packet with the root's invariants 1–6 and assumption-set ruling: `s3/build/PACKET.md`. Builder result and matrix: `s3/build/RESULT.md`, `s3/build/MATRIX.md`, mutants under `s3/build/mutants/`. Root clean-export gate: `s3/ROOT-GATE.json` and `s3/root-gate-1/`. Prior acceptance: `s1/S1-ACCEPTANCE.json`.

## Review

1. Do invariants 1–6 hold at the subject? In particular: no route spawns the SDK child or reaches the consumer without the gate (pre-spawn and pre-consume); discharge is host-derived and total over a closed kind, never caller-supplied; realization check blockers are never assumable; the caller cannot supply or override the realization description; refusal leaves spawn 0 / consumer 0.
2. **Root provisional ruling to challenge (builder deviation 1):** the host description claims `MutationClaim::ReadOnlyClaimed` (labelled assumption) so the assumption world admits, since a mutation check is emitted for every condition. Root's provisional position: acceptable only if it is a claim the host itself backs (empty toolset verified at allocation and request; no tools ⇒ no model-initiated mutation) and is labelled as not confinement. Is that truthful, or is it an assumption smuggled into the "never assumable" realization path? What is the better design inside R2-S scope?
3. Deviations 2–6 in RESULT.md (free-text descriptor values; add-only `BetweenBoundaries` seam; `Blocker.kind` in contract bytes; join tests on frozen world; new `DeliverError` variant): acceptable?
4. Second world: correctly derived by the unchanged compiler, labelled, and the frozen world refused by known realization incompatibility through the real host path? Any S1 coverage lost by rebasing?
5. Matrix/mutant adequacy for the new guards; note `RealizationDiffers`/`PreparationChanged` not separately discriminated.
6. Label truthfulness: assumed discharges are not admissions; production admission still refuses; unclaimed list.

Severity P1/P2/P3 with file:line evidence and confidence; distinguish executed facts (records) from static reasoning. No builds or tests.

## Output

Write `/home/bline/cbm-work/org-pi-slices-20261116/s3/review/REVIEW-1.md` (first line verdict: `accept_private_readiness` or `changes_required`). If writing fails, put the full report in your final message. End with a short final message: verdict and path.

## Constraints

Read-only except the output file. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials/OAuth, configuration contents, `~/.codex`, or other `~/.claude` sessions. No builds, tests, network, Docker, git mutation. Budget: target $8, cap $10 — the objective has little remaining; read the patch and its direct callers, not whole crates.
