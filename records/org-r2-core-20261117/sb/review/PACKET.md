# S-B independent code review (Opus xhigh, read-only)

Role: fresh, separate adversarial reviewer. Read `/home/bline/code/work-engine/skills/manual-orchestration/SKILL.md`; load only `references/worker.md` and `references/call-cost.md`. Read `/home/bline/code/work-engine/AGENTS.md` and `docs/contracts/core.md`. Parent, collector and acceptance authority: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners. Independence: separate session/context; same provider family as the builder.

## Subject

Private checkpoint `dd68df6c7a7f526d934bc16354fe632c985620ff`, tree `59eb0a36b7a05a2ece155dd586ae1170d639a303`, task patch `ac517a6bd83a9b7373a6b7439bf5e665a78bd62c5e479ba37942006ba5571c7d` vs baseline `561c6ffc`, parent checkpoint `bfaa455c` (accepted S-A). S-B diff: `git -C /home/bline/cbm-work/wt-org-r2-core-20261117 diff bfaa455c dd68df6c` (review-delivery crate + lockfile). Clean export: `/home/bline/cbm-work/org-r2-core-20261117/sb/export/rust`.

## Contract and evidence (paths under `/home/bline/cbm-work/org-r2-core-20261117/`)

Builder packet with invariants 1–6: `sb/PACKET.md`. S-A acceptance with the S-B obligations: `sa/SA-ACCEPTANCE.json` (`sb_obligations`), `sa/ROOT-RULINGS-1.md`. Builder result (note design decisions 1–7): `sb/RESULT.md`; matrix `sb/MATRIX.md`; mutants `sb/mutants/`; root gate `sb/ROOT-GATE.json` + `sb/root-gate/` (written when the gate finishes). Prior acceptance of the subsystem: `/home/bline/cbm-work/org-pi-slices-20261116/s3/S3-ACCEPTANCE.json`.

## Review

1. Invariants 1–6 at the subject: discharges keyed on `CheckId` from a host-obtained `EffectAdmission`, never caller-supplied; mapping kinds assumed and labelled; RealizationCheck never dischargeable; admission and F-7 digest comparison before spawn (spawn 0 / consumer 0 on refusal); regate before consumption with a fresh observation (replaying the admission observation cannot satisfy it); first non-current non-Unknown state final; Unknown fail-closed; `Domain::Fixture` asserted; labels on success; `run_attempt*` no longer an ungated public route; `AssumedDischarges` bound and not publicly mintable.
2. Builder decisions 1–7, especially (1) consumption admission obtained before spawn so supersession shows as Stale, (2) no point check of the delivery admission, (3) F-7 compares four fields only, (4) `test_support` still exposes a generic runner under `fixture`, (5) `HeldAdmission` public under `fixture`. Acceptable, or a defect?
3. Any route from a binding to the consumer that bypasses the gate; any way a stale/revoked admission reaches consumption.
4. Truthfulness of labels and the unclaimed list (fixture standing, O10 gap, realization digest definition over the description).
5. Matrix/mutant adequacy; vacuous tests; S1/S3 coverage kept.

Severity P1/P2/P3 with file:line evidence and confidence; distinguish executed facts from static reasoning. No builds or tests.

## Output

Write `/home/bline/cbm-work/org-r2-core-20261117/sb/review/REVIEW-1.md` (first line verdict: `accept_private_readiness` or `changes_required`). If writing fails, put the report in your final message. Final message: verdict + path.

## Constraints

Read-only except the output. Never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, configuration contents, `~/.codex`, other `~/.claude` sessions. No builds/tests/git mutation. Budget: target $5, cap $7.
