# S-B escalation design review (Fable xhigh, read-only, focused)

Role: fresh, separate architectural/design reviewer under the campaign architecture guard (`/home/bline/cbm-work/org-pi-review-continuation-20261114/ARCHITECTURE-GUARD.md`; Fable replaces Astra by operator direction). Read `/home/bline/code/work-engine/AGENTS.md` and `docs/contracts/core.md` only (budget). Parent: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`, which decides.

## Why you are here

The review-delivery host gate has now had a second substantive rejection, and the root judges the defect a recurrence of the same invariant:

- S3 REVIEW-1 F-2 (subject `24597fef`, repaired and accepted): the host's realization description was a constant while caller-controlled `NativeConfig` fields selected what actually ran. `/home/bline/cbm-work/org-pi-slices-20261116/s3/review/REVIEW-1.md` §2 F-2.
- S-B REVIEW-1 P2-1 (subject `dd68df6c`): `FixtureAuthority.request` is a caller input; its vantage (and organization id) selects which Gate R checks run, and only four fields are compared with the binding. `/home/bline/cbm-work/org-r2-core-20261117/sb/review/REVIEW-1.md` (read all of it; P3-1 to P3-7 are related).

Common class: caller-supplied inputs choose what the gate evaluates without being derived from, or bound to, facts the host already owns. Fixing fields one by one has happened twice.

## Inputs

S-B packet `/home/bline/cbm-work/org-r2-core-20261117/sb/PACKET.md`; builder result `sb/RESULT.md`; S-A acceptance `sa/SA-ACCEPTANCE.json` (sb_obligations) and `sa/ROOT-RULINGS-1.md`. Source in the clean export `/home/bline/cbm-work/org-r2-core-20261117/sb/export/rust/crates/` — mainly `work-engine-review-delivery/src/{native,gate,binding}.rs` (`FixtureAuthority`, `compare_with_binding`, `deliver_and_consume_observed`, `HeldAdmission`, `test_support`) and `work-engine-organization-authority/src/gate.rs` (`OperationRequest`, `admit`, `evaluate`).

## Question

Is a bounded implementation repair still credible, is a local design correction needed, or must the plan be reopened? Specifically judge the root's candidate local correction: **the host constructs the whole `OperationRequest` from facts it owns** (binding's preparation: vantage, subject, candidate and requirements digests; host-derived realization digest; boundary from the host), so the caller supplies only the authority source, the organization id (explicitly unanchored and labelled) and time — instead of comparing caller fields after the fact. Does that remove the class? What else in the gate inputs remains caller-chosen and how should each be handled (P3-1 delivery-admission regate/revision equality, P3-2 single time input, P3-3 `test_support` + `consume_fixture_result` route)? Give the minimal correction scope, its tests/mutants, and anything that must be carried or labelled instead.

Output: `/home/bline/cbm-work/org-r2-core-20261117/sb/design-review/REVIEW.md`, first line one of `retain_with_bounded_repair`, `local_design_correction`, `reopen_plan`. If writing fails, put it in your final message. Final message: verdict + path.

Constraints: read-only except the output; never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, configuration contents, `~/.codex`, other `~/.claude` sessions; no builds/tests/git mutation. **Hard budget $5**: targeted reads only, then write.
