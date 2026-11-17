# R2 reconciliation with the finished dev vertical (Opus xhigh, read-only)

Role: one architect/planner (worker). Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Read `/home/bline/code/work-engine/AGENTS.md`, `docs/contracts/core.md`, `docs/contracts/system-design.md`. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; it observes your context/budget; no listeners, no children.

## Why

The operator approved the R2 admission design (`/home/bline/cbm-work/org-pi-slices-20261116/s2/OWNER-DECISION-S2.json`, `s2/ROOT-S2-ADDENDUM.md`, design `s2/design/PROPOSAL.md`, review `s2/review/REVIEW.md` especially F-2, F-6, F-7). R2-S (fixture refusal seam) is accepted and merged (`s3/S3-ACCEPTANCE.json`). Since then the **dev vertical finished**: a production hosted Pi coding path (profile `work-engine.pi-coding-hosted.v1`) with a lifecycle grant/gate that binds funding/route policy and re-verifies currentness before every upstream dispatch, a durable per-attempt journal, live routes, operational entrypoint `coding-job-prepare` → `work-engine-server` → `work-engine-command`. Its open items say grants are self-issued (K-c) and an independent grant issuer is needed before non-funder submission. Root's hypothesis: the planned R2-B host boundary inside the fixture delivery crate would now duplicate an existing owner, and R2-A's admission core should become what the lifecycle grant consumes. Test that hypothesis against source; do not assume it.

## Read (main checkout `/home/bline/code/work-engine`, HEAD `9b289171`, read-only; S1/S3 org crates are merged there)

- `docs/dev-vertical-open-items.md`; `records/dev-vertical-hosted-qualification-20261117/README.md` (selectively); `planning/dev-vertical-completion-20261115.md` (boundary/ownership sections).
- Gate/grant/journal source: `rust/crates/pi-pressure-context-host/src/coding.rs` (CodingEffectGate, CodingContinuity, provider_attempt), `rust/crates/work-engine-manual-host/src/{pi_coding_hosted,pi_coding,pi_coding_claims}.rs`, `rust/crates/work-engine-server` (hosted/live class, job prepare), and the lifecycle grant types they use (`lifecycle-*` as reached). Use `rg` and line ranges; do not read whole large files.
- Org side (for the seam): `rust/crates/work-engine-review-delivery/src/{gate,native,binding}.rs`, `rust/crates/work-engine-review-preparation/src/prepare.rs` (BlockerKind), `rust/crates/work-engine-organization/src/enforce.rs`.

## Answer, with file:line evidence

1. **Owner map.** Which existing owner now holds per-effect runtime admission, single-use/attempt identity, currentness re-check and settlement for Pi jobs? Is the root's hypothesis right that R2-B (as in addendum item 10 / `R2A-ESTIMATE.md` at `/home/bline/cbm-work/org-pi-slices-20261116/R2A-ESTIMATE.md`) would duplicate it? Where would the delivery-boundary single-use (O10/D7) now live?
2. **Seam.** The minimal contract by which an organizational admission (principal, vantage-actor, component binding, subject, operation, authority revision, realization) is consumed by the lifecycle grant/gate: what the grant carries or references, where currentness of the organizational admission is re-checked (per attempt? per effect?), stale/revoked/unknown consequences, and how discharges of the S3 `BlockerKind`s are derived from it. Avoid a second ledger or parallel gate.
3. **Review job on the hosted path.** What a read-only implementation-review job needs from the hosted path (profile, tools, mutation, result contract implementation-review/v1, consumer), and what stays fixture-only. Which parts need live/paid authority from the operator.
4. **Independent grant issuer (K-c).** Can R2-A's organizational authority be the independent issuer, and what does that change in the dev vertical (owners touched, compatibility)?
5. **Rescoped slices and estimate.** Replace R2-A'/R2-B with the smallest coherent sequence (1–3 slices), each with scope, owners touched, tests/qualification, and cost (builder + Fable/Opus review + remediation + root) using actuals: Opus builder $7–10, Fable review $7–12, Opus-xhigh normal review cheaper, remediation $1–3, recheck $2–7, root ~$5/slice. Mark which slice needs live/paid inference.
6. **Owner decisions** newly required (with recommendations), and anything in the approved design (O1–O11) that this reconciliation changes.

State confidence and what you did not read. Do not propose implementation outside the approved R2 design without labelling it a design change needing owner approval.

## Output

Write `/home/bline/cbm-work/org-r2-reconciliation-20261117/design/RECONCILIATION.md`. If writing fails, put it in your final message. Final message: `proposal_ready` or `handoff_ready` + path.

## Constraints

Read-only except the output file. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials/OAuth, configuration contents, `~/.codex`, other `~/.claude` sessions. No builds, tests, network, Docker, git mutation, live jobs. **Budget: target $4, hard cap $5** (the whole pass is capped at $10); stop and write what you have before the cap.
