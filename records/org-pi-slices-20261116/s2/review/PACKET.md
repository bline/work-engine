# S2 independent design review of the R2 admission proposal (Fable xhigh, read-only)

Role: fresh, separate adversarial architecture/design reviewer. Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md` and load only `references/worker.md` and `references/call-cost.md`. Read `/home/bline/code/work-engine/AGENTS.md`, `docs/contracts/core.md`, `docs/contracts/system-design.md`; load `docs/contracts/evidence-and-claims.md` §3.9 only if needed. Parent/collector and design-disposition authority: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; the operator decides the owner questions. The parent observes your context/budget; start no listeners. Independence: separate session/context; same provider family as the author (Opus).

## Subject

`/home/bline/cbm-work/org-pi-slices-20261116/s2/design/PROPOSAL.md` (sha256 recorded by root at launch). Its governing plan is R2 of `/home/bline/cbm-work/org-codex-recovery-plan-20261113/PLAN.md`; starting state is R1 accepted at `/home/bline/cbm-work/org-pi-slices-20261116/s1/S1-ACCEPTANCE.json`; source at private checkpoint `1cc77114` in `/home/bline/cbm-work/wt-p1-vertical-20261113` (working tree equals it). Prior design material the proposal reuses: `/home/bline/cbm-work/org-rev6-20261112/STAGE0-CONTRACT-r6.md` (§3.3, §4.3, §8–9, §18), `AMENDMENT-6.1.md`, and its review log `assignment-log.md` (rev6 verdict `do_not_authorize`, F1–F7; storage blocker C5-B1).

## Questions

1. Does the proposal satisfy R2 as written (owners reconciled with existing admission/publication/lifecycle owners, no parallel grant or scheduling owner, candidate derivation separate from publication and per-effect admission, qualification binding actual principal/actor/subject/operation/authority revision/realization with stale/revoked/unknown handled)? Check its code-seam claims against source with file:line.
2. Does reusing rev6 §18 + Gate O/Gate R logic carry forward any unresolved rev6 review defect (F1–F7) or C5-B1 that the proposal claims to avoid? Is the "observation contract, storage undecided" boundary sound or does it hide the storage problem?
3. Is it the simplest design meeting the contracts (pre-release, no compatibility consumers)? Flag unnecessary machinery, especially anything resembling the S1 lesson (root-authored production-path claims whose only consumer is their own gate).
4. Owner decisions D1–D8: are they the right questions, correctly framed, with sound recommendations? Anything missing that the owner must decide?
5. Is D3 (making R1's fixture path refuse on undischarged blockers) consistent with R1's accepted exit and the S1 acceptance record, and safe to implement inside R2-B?
6. Are the R2-A/R2-B slices coherent, correctly bounded and credibly estimated? Would a smaller first slice deliver a meaningful, reviewable outcome within about $25–30 total (build + review + remediation)?

Severity P1/P2/P3 with evidence and confidence; distinguish source-checked facts from reasoning.

## Output

Write `/home/bline/cbm-work/org-pi-slices-20261116/s2/review/REVIEW.md` (first line verdict: `approve_for_owner_decision`, `revise_before_owner`, or `reject`). If writing fails, put the whole report in your final message. End with a short final message: verdict and path.

## Constraints

Read-only except the output file. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials/OAuth, configuration contents, `~/.codex`, or other `~/.claude` sessions. No builds, tests, network, Docker, git mutation. Budget: target $8, cap $11; read selectively; skip telemetry JSON.
