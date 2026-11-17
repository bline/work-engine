# S-A independent review: pure organizational admission core (Fable xhigh, read-only)

Role: fresh, separate adversarial reviewer. Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Read `/home/bline/code/work-engine/AGENTS.md` and `docs/contracts/core.md`. Parent, collector and acceptance authority: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners. Independence: separate session/context; same provider family as builder.

## Subject

Private checkpoint `1b8bf3015a14d5465ba4d73beeb9d1db45b7fcde`, tree `5e6390fcb6423b1a657e8f6c4c95c68e680f0b8b`, task patch `08d0ca2989db144feb351335f1e0b780ada39b38b98df9db6af14a4a83dad7a2` vs baseline `561c6ffc` (repo/worktree `/home/bline/cbm-work/wt-org-r2-core-20261117`). Read source in the clean export `/home/bline/cbm-work/org-r2-core-20261117/sa/export/rust/crates/work-engine-organization-authority`. Patch: `git -C /home/bline/cbm-work/wt-org-r2-core-20261117 diff 561c6ffc 1b8bf301` (new crate + workspace member + lock entry).

## Contract and evidence (paths under `/home/bline/cbm-work/`)

- Builder packet with required outcomes: `org-r2-core-20261117/sa/PACKET.md`; owner decisions `org-r2-core-20261117/OWNER-DECISION-R2REC.json`, `org-pi-slices-20261116/s2/OWNER-DECISION-S2.json`, `org-pi-slices-20261116/s2/ROOT-S2-ADDENDUM.md`; design `org-pi-slices-20261116/s2/design/PROPOSAL.md` §4 (contract text) and §5; prior design `org-rev6-20261112/STAGE0-CONTRACT-r6.md` §4.3 (l.577–728), §18 (l.1538–1635); `org-rev6-20261112/AMENDMENT-6.1.md` §3–§6 (l.144–250).
- Builder result `org-r2-core-20261117/sa/RESULT.md` (note its 15 design choices), matrix `sa/MATRIX.md`, mutants `sa/mutants/records-20261117T111045Z/` and `sa/mutants/summary.json`, root gate `sa/ROOT-GATE.json` + `sa/root-gate/`.

## Review

1. Does the crate meet packet outcomes 1–7: records (6.1 F4/F5, property-defined identities, no production constructor), observation trait with the four obligations, reservation-sealed observation with later-boundary refusal, Gate O subset O3/O4/O5/O8/O9 and Gate R R1–R4 with per-check outcomes, five states with the approved consequences, 6.1 §6 oracle classes, exclusions respected? Is it pure (no I/O, clock input only)?
2. Correctness of each check against rev6 §4.3/§18 and the proposal's §4 contract; fail-closed behaviour (Unknown, missing candidate, source-obligation violations); production all-refuse truly unreachable-positive.
3. Builder design choices 1–15: flag any that change approved meaning (e.g. choice 1 puts Gate R and `EffectAdmission` in this crate while O1 placed them with the consuming host; choice 11 extends the §18.5 schema; choice 6 conservative grant pinning; choice 8 regate on every state query). Say which need a root/owner ruling.
4. Is anything over-built (machinery whose only consumer is its own test — the S1 lesson) or missing that S-B needs (check outcomes by id, usable from review-delivery without depending on review-preparation)?
5. Matrix/mutant adequacy; tests that pass vacuously.

Severity P1/P2/P3 with file:line evidence and confidence; distinguish executed facts (records) from static reasoning. No builds or tests.

## Output

Write `/home/bline/cbm-work/org-r2-core-20261117/sa/review/REVIEW-1.md` (first line: `accept_private_readiness` or `changes_required`). If writing fails, put the report in your final message. Final message: verdict + path.

## Constraints

Read-only except the output. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, configuration contents, `~/.codex`, other `~/.claude` sessions. No builds/tests/git mutation. Budget: target $10, cap $13; the gate.rs file is ~1.3k lines — read it, but read rev6/6.1 only at the cited ranges.
