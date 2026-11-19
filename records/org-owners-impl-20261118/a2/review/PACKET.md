# A2 review packet: OrganizationAdmission record at fixture standing (read-only adversarial code review)

Role: fresh-entry adversarial code reviewer. Read the repository `AGENTS.md` and `docs/contracts/core.md`. Do not edit, stage, or commit anything. Do not build into the builder's target directory. Write exactly one file: `/home/bline/cbm-work/org-owners-impl-20261118/a2/review/REVIEW.md`. No chatboard posts and no subagents.

## Subject

- Worktree `/home/bline/cbm-work/wt-org-owners-a2-20261118` (uncommitted diff against base `f772612f`; `git -C <worktree> diff f772612f`). The same bytes are in `/home/bline/cbm-work/org-owners-impl-20261118/a2/a2-1.patch`.
- Builder report: `/home/bline/cbm-work/org-owners-impl-20261118/a2/RESULT.md` and `MATRIX.md`. Builder records: `runs/gate-2` (15/15 passed) and `mutants/runs-1` (10/10 killed). The root is running its own gate on a clean export. Treat the builder's claims as claims and check them.
- A sibling slice A1 (S-A `CheckId::S1`, scope.digest) will merge first; A2 will be rebased onto it later. Judge A2 on its own base; note anything that would make that rebase unsafe beyond mechanical conflicts.

## Authority

- Plan §0, §4, §5 and §8: `/home/bline/cbm-work/org-owners-planning-20261118/PLAN.md` (not reviewed itself).
- Accepted design decisions D4 (organization id is the admitted organization, from the envelope) and D5 (new S-A record `OrganizationAdmission`; remove `organization_id` from `OperationRequest`): `records/org-hardening-owners-20261117/d1/PROPOSAL.md`.
- Operator decision O-1: owner definitions are recorded as crate doc comments plus a records entry (RESULT.md).
- Fixture standing only; production admission must still refuse. The hosted crate stays `owner_pending`.

## What to judge

1. Correctness of `CheckId::OA`: selects exactly one committed admission matching candidate digest, requirements digest and problem scope; refuses on none (`OrganizationUnadmitted`) and on several (`OrganizationHeadsAmbiguous`); the organization then used everywhere comes from that record, never from caller input. Look for any remaining route by which a caller-supplied organization id reaches admission, binding, or delivery.
2. Pins and lifecycle: `AuthorityPins.organization_admissions`; revocation gives `Revoked`, supersession gives `Stale`; at regate time as well as admission time.
3. Vantage bindings naming their admission's issuance; O8/R3 selection by the derived admission; a successor admission needing new bindings. Sound, and no cross-organization binding acceptance?
4. The open item the builder declares: OA does not check the predecessor chain (revision +1, same organization, predecessor superseded). Is trusting the fixture issuer acceptable at fixture standing, or a P2?
5. Delivery: `AdmittedOrganization` in the summary; `CALLER_ASSERTED_LABEL` now covering only `actor_identity`; the inventory labels truthful.
6. Tests and mutants: do the 11 negatives (each with a positive control) and 10 mutants kill the claims they map to? Unkilled routes, such as OA missing from `ALL`, a short-circuit, or a check comparing a value with itself?
7. The owner-definition doc comment on `OrganizationAdmission`: accurate and consistent with D4/D5.
8. Scope: no change to the hosted crate; no production admission path opened.

## Output

REVIEW.md: verdict (`accept`, `accept_with_changes`, or `reject`); findings with severity P1 (blocks), P2 (fix before merge), P3 (optional), each with path:line, evidence and a concrete fix; anything you could not verify. Write a REVIEW.md skeleton early and keep it current. You may run read-only commands and `cargo test` with `CARGO_TARGET_DIR=/home/bline/cbm-work/org-owners-impl-20261118/a2/target-review` if needed, but prefer reading.

Constraints: never read `/etc/work-engine`, `/var/lib/work-engine`, `~/.local/state/work-engine`, credentials, `~/.codex`, `~/.claude` sessions. No network, Docker, live provider, or mutating git. Budget target $3.5, cap $4.5 including the report. Final message: the verdict and the REVIEW.md path.
