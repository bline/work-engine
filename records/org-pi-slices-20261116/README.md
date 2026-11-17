# Organizational compilation slices (2026-11-16) — S1 accepted, S2 awaiting owner decision

Committed and merged at the operator's explicit request ("Commit and merge.", 2026-11-16). Owner: Pi root
session `01a1e56e-be12-7424-977b-c7c7983887d0`, which took over Codex session `01a1d364` by operator direction.
Budget: $100 API-equivalent proxy for the objective; about $64 used at this commit (see `cost.py`, `children.txt`).

**S1 (R1 of `/home/bline/cbm-work/org-codex-recovery-plan-20261113/PLAN.md`)** — accepted at bounded private
readiness (`s1/S1-ACCEPTANCE.json`) under a revised acceptance model (`s1/ROOT-S1-DESIGN-DECISION.json`, from the
Fable xhigh recheck `s1/design-recheck/REVIEW.md`). Four root-authored production-path claims of the prior campaign
(complete build-input closure, executable custody joins, private-qualifier re-appraisal, cold/common build
reconciliation) were withdrawn and are **unestablished**; 12 findings that depended only on them are superseded, not
closed. The committed source equals private checkpoint `1cc771146c72ff7d23b371b730aa5e73ce79db61` (tree
`be29bec5a48560682e0a9894ef2ff83a864dbe14`). Evidence: root clean-export gate `s1/ROOT-GATE-3.json`, matrix
`s1/build/MATRIX.md`, Fable xhigh review `s1/review/REVIEW-1.md` and recheck `s1/review/RECHECK-1.md`
(both `accept_private_readiness`). Raw custody records, mutant outputs and exports remain at
`/home/bline/cbm-work/org-pi-slices-20261116/` and are not copied here.

Fixture standing only. No AdmittedOrganization/DispatchAuthorization, live provider, hermetic build, tamper evidence,
confinement or timing guarantee is claimed; see the acceptance record's `unclaimed` list and carried P3 items. The
frozen private qualifier under `records/organizational-compiler-pi-wip-20261114/qualifier-source` is unmaintained and
no longer matches the delivery crate. No tests were rerun after merging into the shared branch beyond what the merge
record states.

**S2 (R2 lawful admission)** — design only: `s2/design/PROPOSAL.md`, independent review `s2/review/REVIEW.md`
(`revise_before_owner`), root addendum `s2/ROOT-S2-ADDENDUM.md` with owner decisions O1–O11. **Not approved; no
implementation.** Committing these files records the proposal; it is not owner approval.

`architectural-risk-history.json` is revision 39 of the campaign's escalation history (predecessor r37 at
`records/organizational-compiler-pi-wip-20261114/campaign-records/` or the original campaign root).
