# D3 design review packet (read-only, Fable xhigh)

Role: read-only adversarial design reviewer (fresh context). Read `/home/bline/code/work-engine/AGENTS.md`, `docs/contracts/core.md`, `docs/contracts/system-design.md`, `docs/contracts/evidence-and-claims.md` (§3.9 and route-sensitive acceptance); from `skills/manual-orchestration/SKILL.md` load only `references/worker.md` and `references/call-cost.md`. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children. Write only `/home/bline/cbm-work/org-hosted-review-20261117/d3/review/REVIEW.md`.

Subject: `/home/bline/cbm-work/org-hosted-review-20261117/d3/DESIGN.md` (Opus; its packet `d3/PACKET.md` states the objective and the operator authorization). Repository `/home/bline/code/work-engine` (HEAD may have moved; cite what you read).

Check:
1. Is the route truthful and minimal: new `work-engine-review-hosted` crate calling public `prepare` + `FrozenDelivery::freeze`, invoking the unchanged `analysis-qualification` binary by pinned digest, strict `ReviewResult::parse` + subject equality? Any hidden change to dev-vertical crates or owner contracts? Is bypassing `ExperimentBinding` (fixture-only) consistent with the R2 gate's standing, i.e. does the hosted route need *no* org admission legitimately under operator decision D6 (produce + validate, no consumption), and is that stated precisely enough that nobody can read the result as admitted?
2. The realization finding (frozen requirements demand anthropic + guarded_sandbox; hosted path admits only gpt-6-luna) and OQ-1 options A/B/C: correct? Is option A (labelled assumption world) a truthful qualification, or does it risk laundering an unrealizable condition? Any better option (e.g. a requirements revision by its owner)?
3. Claims/unclaimed and the evidence plan: is "contract delivered byte-exact in a provider-bound request" actually checkable from retained bodies; is the no-mutation evidence sufficient for the claim made; anything overclaimed?
4. Cost/slice plan and stop conditions: realistic within $50 total (D3 ≈ $3.3 already spent)?

Output `REVIEW.md` (≤ 100 lines): verdict `proceed` | `proceed_with_changes` | `revise_before_build`; findings P1–P3 with file:line and concrete fixes; any operator decision you would add or reword. Final message: verdict + path.

Constraints: read-only; no builds, no mutating git, no network, no live providers. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions. Targeted reads. Budget: target $3.5, cap $5.
