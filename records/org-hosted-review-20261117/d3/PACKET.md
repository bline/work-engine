# D3 design packet: a real read-only review on the hosted path (read-only)

Role: read-only design worker. Read `/home/bline/code/work-engine/AGENTS.md`, `docs/contracts/core.md`, `docs/contracts/system-design.md`, `docs/contracts/evidence-and-claims.md` (only sections on route-sensitive acceptance), and from `skills/manual-orchestration/SKILL.md` only `references/worker.md` and `references/call-cost.md`. Parent/sole collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children. Do not edit the repository; write only under `/home/bline/cbm-work/org-hosted-review-20261117/d3/`.

## Objective (operator-authorized, 2026-11-17)

The first real end-to-end result of the organizational compiler: a compiled review contract (the `implementation-review/v1` obligation of a frozen organization candidate, as prepared by `work-engine-review-preparation` and frozen by `work-engine-review-delivery`) runs as a **read-only** review job on the dev vertical's **hosted** Pi path with live inference, and produces an `implementation-review/v1` result that `review_episode_core::implementation_review_v1::ReviewResult::parse` (and the subject check) validates. **No consumption** with production standing: results stay `owner_pending`; grants stay self-issued (operator accepted for single-operator use); O7 owners and the organization anchor are **not** needed (operator decision D6). Budget for the whole step: $50 including live inference; the hosted proxy cap per run (700000 µUSD, R-c) applies.

## Starting facts to verify, not trust

- `records/org-r2-core-20261117/reconciliation/design/RECONCILIATION.md` §3 lists what was missing (new hosted review profile with mutation none; mutation evidence; result contract; consumer) as of 2026-11-17 morning.
- Since then the dev vertical landed **H2 Pi route slice 3: hosted analysis profile (Host-served snapshot tools, no shell), analysis grammar and gate, request body retention** (`git show --stat 1051e660`, remediation `22487ec6`, `431ff216`; plan `planning/` files referenced there; slice 4 `dc26d0fd`). This may already provide the read-only hosted profile §3 asked for. Establish exactly what it provides (profile/protocol digest pair, tools, write boundary, result publication, export/no-mutation evidence, how a job's prompt/contract enters, how the result leaves) and what differs for a review job.
- The dev vertical is currently writer for lifecycle-service, lifecycle-store, lifecycle-codex, work-engine-task-host (branch `claude-realization-20261117`) and holds a claim on manual-host / receiver / pi-pressure-context-host / lifecycle-store Pi grammar / server `job.rs` class guard. Prefer a design that adds nothing to those crates, or isolates any needed change to the smallest surface that its owner can implement or approve.

## Output

`/home/bline/cbm-work/org-hosted-review-20261117/d3/DESIGN.md` (≤ ~250 lines):

1. Route: how the frozen review contract becomes a hosted job (who builds the job input from the `ExperimentBinding`/`FrozenContract`, which profile, which snapshot/subject the reviewer reads, how the result is published and retrieved), and where the result is validated (`ReviewResult::parse` + subject equality with the binding's subject). Cite file:line for every existing piece.
2. Per touched crate: the change, its owner (ours vs dev-vertical), size (lines), and whether it can be done outside the claimed crates (e.g. a new crate or a `work-engine-review-delivery` hosted adapter).
3. Evidence plan: offline qualification first (fixture upstream / recorded provider if the hosted path supports it), then the live run(s): what is retained (request bodies, result bytes, no-mutation evidence, cost), what is claimed and what stays unclaimed (no consumption, owner_pending, self-issued grants, fixture org admission or none, D6).
4. Slice plan with cost estimate (builder + Fable review + live) that fits $50 total with the D3 cost already spent; stop conditions; the questions the dev-vertical owner must answer (exact wording ready to post).
5. `not established` items.

Final message `map_ready` + path.

## Constraints

Read-only. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, other `~/.claude` sessions or protected configuration. No network, Docker, cargo builds, live providers, or mutating git. Targeted reads (rg, `git show --stat`, ranges). Budget: target $4, cap $5.5.
