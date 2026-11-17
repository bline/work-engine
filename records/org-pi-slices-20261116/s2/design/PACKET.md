# S2: R2 lawful organizational and realization admission — design proposal (read-only)

Role: one architect/planner (worker). Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md` and load only `references/worker.md` and `references/call-cost.md`. Read `/home/bline/code/work-engine/AGENTS.md`, `docs/contracts/core.md` and `docs/contracts/system-design.md`; load `docs/contracts/evidence-and-claims.md` (§3.9) when a claim's standing matters. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; it observes your context/budget; start no listeners. No children.

## Objective

Produce the bounded design decision that R2 of `/home/bline/cbm-work/org-codex-recovery-plan-20261113/PLAN.md` requires before any implementation ("Establish lawful organizational and realization admission"; read R2, the contract-to-owner matrix, and the R1/R3 sections for context). The output is a **proposal for owner approval**, not authority. PLAN.md: "Any new executable profile, mechanism-table revision or relaxation of a standing hold requires an explicit owning decision"; "Exit: a separately approved admission contract and its discriminating qualification."

Starting state: R1 is accepted at bounded private readiness — `/home/bline/cbm-work/org-pi-slices-20261116/s1/S1-ACCEPTANCE.json` (read its `semantic_limit`, carried P3 items F-7 and F-8, and `unclaimed`). Source at private checkpoint `1cc771146c72ff7d23b371b730aa5e73ce79db61` in worktree `/home/bline/cbm-work/wt-p1-vertical-20261113` (the working tree currently equals it). Relevant crates include `work-engine-organization` (pure candidate/view/enforce), `work-engine-review-preparation` (projection; blockers for authority/component bindings), `work-engine-authority-store` (fixture store; ordinary `admit` always refuses), `review-episode-core`, `work-engine-review-delivery`, and the existing lifecycle/admission/publication owners (`lifecycle-*`, `session-task-admission`, etc.). Canonical intent: `/home/bline/code/work-engine/design/architecture/{organizational-compilation,authority-and-ownership,runtime-realization,role-and-contract-structure}.md`. Stage 0 Gate O / Gate R proposals: locate them (start with `/home/bline/cbm-work/org-rev7-handoff-20261113/handoff-org-compilation-root-rev7.md`, `/home/bline/cbm-work/org-p1f-e2e-20261113/`, `design/ideas/pending/deterministic-authority-projection-and-adaptive-organizational-topology.md`, and `rg -i "gate o|gate r"` in the worktree and main repo); state what you found and did not.

## The proposal must answer

1. **Owners.** For organizational standing/grants, principal-to-vantage separation, component bindings, and per-effect concrete runtime admission: which existing owner (crate/service/contract) owns each, and the exact missing consumer seam. Do not create a parallel grant or scheduling owner; reconcile Gate O/Gate R with existing admission/publication/lifecycle owners.
2. **Separation mechanism.** Options for an owner-approved principal-to-vantage separation and component-binding mechanism, with tradeoffs; recommend one. Keep candidate derivation (pure) separate from authority publication and from per-effect runtime admission.
3. **Admission contract.** The minimal contract text: what an admission binds (actual principal, actor, subject, operation, authority revision, concrete realization), its states (current, stale, revoked, unknown, refused) and their consequences, and how R1's fixture path changes (e.g. whether blockers/realization mismatch must now refuse delivery or consumption — S1's semantic limit).
4. **Discriminating qualification.** Checks that would show the contract works: changed principal/revision/realization → refusal; stale/revoked/unknown → no advance; fixture identity cannot satisfy a production binding; ordinary-admission refusals preserved until the contract's success path is approved.
5. **Holds.** What remains blocked (PAS-4, L1, first-instruction fencing, S1/A1-x/ptrace/OCI-wrapper, live provider, Docker) and what the proposal needs from the owner (each decision listed explicitly as a question with your recommendation).
6. **Implementation slicing and estimate.** One or two bounded implementation slices after approval, each with scope, owners touched, tests, and a rough API-equivalent cost (builder + independent review + remediation), keeping in mind that recent slices cost about $10 builder / $12–19 review.

Be concrete with file:line evidence for current seams. Prefer the simplest design that satisfies the contracts (pre-release: no compatibility guarantee unless a named consumer needs it). State confidence and what you did not read.

## Output

Write `/home/bline/cbm-work/org-pi-slices-20261116/s2/design/PROPOSAL.md`. If writing fails, put the whole proposal in your final message. End with a short final message: `proposal_ready` or `handoff_ready` and the path.

## Constraints

Read-only except the output file. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials/OAuth, configuration contents, `~/.codex`, or other `~/.claude` sessions. No builds, tests, network, Docker, git mutation. Budget: target $7, cap $10; read selectively; skip telemetry JSON (`*-exact-turn.json`, `cost-*.json`, `board-*.json`).
