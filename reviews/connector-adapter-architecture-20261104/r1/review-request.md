# Fresh architecture-plan review

The user asked for a connector-adapter plan reviewed by Astra high. Your bounded
assignment is to review the exact candidate below, challenge its assumptions,
and return findings. Requested model: gpt-6-astra; effort: high; tier: default.
The root owns final collection and any plan changes. This is planning only.

Read and follow `skills/manual-orchestration/SKILL.md` for the reviewer role and
only its role-relevant references. Repository AGENTS, DESIGN.md and PHILOSOPHY.md
apply. Do not inherit the designer conversation or use its conclusions as proof.
No implementation, file mutations, live connectors, provider requests, service
changes, credentials, board publication, or child launches are assigned. Return
your report in the final response; the root persists it. You are not alone in
the repository: do not change or revert another session's work.

## Exact subject and read set

- Subject: `/home/bline/code/work-engine/reviews/connector-adapter-architecture-20261104/r1/subject.md`.
- SHA-256: `799eea2d51b4b68c7d40dd4537294f6ff0b98848e4ecd550d7e20591e40dffaa`.
- Evidence manifest: `../evidence/manifest.json` from this directory, SHA-256
  `3e9d1ad411a14f2043bd563925f9ba02caaf28882b0eb0e3d572f1551a34ebda`.
- Read the complete subject and binding doctrine/rationale. Then use only the
  relevant frozen evidence ranges needed to verify material claims. Begin with
  `evidence/work-engine/skills/session-events/scripts/session_events.py`
  lines 2500-2607, 2936-3107 and 3431-3482; bridge `src/lib.rs` lines 180-214,
  288-440 and `src/native.rs` lines 528-560; `mcp-auth/src/main.rs` lines 63-197.
- In `evidence/devwits-auth/`, relevant ranges are `src/task-contract.js` 1-45,
  `laptop/approval_sync.py` 127-289, `laptop/owner_answer.py` 122-170,
  `docs/owner-answer-v1.md`, `api/_lib.js` 91-121 and `src/work-dashboard.jsx`
  39-70. Inspect other snapshotted files only as a material review question needs.
- Read `evidence/private-candidate/rust/crates/session-task-admission/README.md`
  and relevant `src/service.rs` 54-115, `src/native_authority.rs` 1-110,
  `src/projection.rs` 1-65. This is private partial candidate source, not deployed
  authority. Handback result snapshots establish attributed standing only.
- Rust direction and post-S10 alternative are in the frozen evidence. They
  constrain placement and competing ownership without accepting that amendment.

Evidence prefix above is the review directory
`/home/bline/code/work-engine/reviews/connector-adapter-architecture-20261104/`.
The manifest maps snapshots to original paths and exact source identity.

Parent repository evidence: Tier 2; graph project home-bline-code-work-engine.
Initial generation 2026-11-04T18:04:20Z / uf26ca0b8169e9032g2; refreshed coverage
2026-11-04T19:35:17Z / ua0d1dfc6a4d4a38ag2. Targeted Server/Bridge/Daemon/Native
snippets and bridge inbound/outbound callers were checked; targeted symbol
pages exhausted. Broader candidate searches were not used for exhaustive claims.
Selected main paths have no recorded gaps. devwits-auth and the exact private
checkpoint are outside the graph project; direct-source fallback was performed.
Clean coverage is best effort only. You may use the supplied exact source if
MCP is unavailable; do not claim graph access you do not have. New structural
claims need the repository's usual coverage-aware verification.

## Review questions

Assess whether the plan is implementable and sufficiently bounded, whether the
chosen seams preserve current semantic owners and useful provider differences,
and whether the stages genuinely demonstrate another connector. Challenge both
unnecessary universal abstraction and hidden transport/provider coupling.

Check identity versus approval, per-client visibility/replay isolation, legacy
wire/signature/cursor compatibility, uncertain execution/retry, current-source
and Pi qualification dependencies, authoritative versus projected state,
Rust/non-Rust sharing, and cutover/recovery. Identify omitted acceptance
consequences, vague dependencies or premature architectural commitments.

Do not require unrelated projects to finish, a universal framework, additional
paid reviews, or a preferred procedure unless a concrete required outcome fails.
This is a design review, not implementation validation or deployment approval.

## Binding and output

The root collection owner is `01a1a7ed-45f5-7041-9db5-a6c9aa1cd3f3`.
After launch the root will publish your verified session/rollout identity in
`/home/bline/.local/state/work-engine/connector-adapter-plan-20261104/assignment.json`.
Read that binding at the next boundary and acknowledge it in the report.
The pinned context binary is
`/home/bline/.local/state/work-engine/tools/context-status/7a0cbee1b4f8bfbf3cb0a0c1f2c7e6b7a34454ff2ea63b14ca79f97c83345c18/work-engine-context-status`,
SHA-256 `7a0cbee1b4f8bfbf3cb0a0c1f2c7e6b7a34454ff2ea63b14ca79f97c83345c18`.
The root maintains your durable continuation/binding; you have no write scope.

Return one JSON report (no code fence), roughly 1,500 words or fewer:
subject_sha256; verdict (accept, changes_required, blocked_evidence or
blocked_scope); findings with stable ID, severity, subject line/section,
concrete failure scenario, required consequence and suggested repair;
nonblocking observations; evidence limits; and identity-binding acknowledgement.
P1 blocks acceptance; P2 must be fixed before acceptance; P3 is an improvement.
Accept permits P3 only. A changes_required verdict needs a P1/P2.
Do not assume requested model/effort/tier prove the observed runtime identity;
the root will independently bind those records.
