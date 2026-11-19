# Packet: O7 owners planning pass (read-only)

Repository: /home/bline/code/work-engine (branch rescue-detached-20260911 @ 432657cc). Read `AGENTS.md` and `docs/contracts/core.md` first.

## Assignment

Produce a planning document, not code. Write exactly one file:
`/home/bline/cbm-work/org-owners-planning-20261118/PLAN.md`. Do not edit, create, or delete any other file. Do not run builds or tests, do not use git commands that write anything, do not post to the chatboard, and do not launch subagents.

Your budget is about $2.50. Write PLAN.md early as a skeleton, and update it as you go, so it holds your current best answer if you are stopped.

## Inputs

- D1 owners design: `records/org-hardening-owners-20261117/d1/PROPOSAL.md`. The operator accepted decisions D1-D8 as recommended on 2026-11-17. Treat them as decided:
  - D1 `scope.digest` = (A) exact-subject digest;
  - D2 slice-campaign owns items 2 and 3;
  - D3 planLineage equality (i);
  - D4 organization id = the admitted organization (envelope);
  - D5 new S-A record `OrganizationAdmission`, remove `organization_id` from `OperationRequest`;
  - D6 step 3 needs none of the four (now merged);
  - D7 deferred;
  - D8 dependency placement is the owner's call.
  The proposal was not independently reviewed. Verify its code citations before relying on them.
- `records/org-hardening-owners-20261117/OWNER-DECISIONS.json`, `README.md`.
- The step-3 crate `rust/crates/work-engine-review-hosted` (merged). Its results stay `owner_pending` because the four items below have no owner implementation.

## The four items

1. `scope.digest` as the exact-subject digest (S-A; D1 sized S).
2. Episode-within-scope predicate, including the planLineage equality (slice-campaign; D1 sized M).
3. Consumption-evidence mapping (slice-campaign; D1 sized S; private code reportedly exists — find it and say where).
4. `OrganizationAdmission` record at fixture standing (S-A; D1 sized M).

D1 order: items 1 and 4 in parallel inside S-A, then one slice-campaign slice for items 2 and 3.

## What PLAN.md must contain

For each item:
- the exact files, types, and functions to change, from the current tree (path:line);
- which crate owns it, and what sign-off or owner decision implementation needs;
- the acceptance evidence (tests, mutants, negative controls) that would show it is done;
- risks and open questions that could change the scope.

Then:
- a slice plan: how many slices, what is in each, dependencies, and which can run in parallel (state each actual dependency; list order is not a dependency);
- a dollar estimate for each slice, in API-equivalent USD on the Claude subscription. Use this calibration from the comparable S1 hardening slice (one medium slice touching review-delivery and review-preparation):
  - builder Opus high: $8.98 (stopped at cap before its report);
  - review Opus xhigh: $3.27;
  - remediation-1 Sonnet high: $1.77;
  - Fable recheck: $4.46;
  - remediation-2 Sonnet: $1.23;
  - Fable recheck-2: $2.91;
  - root coordination: about $4;
  - total about $26.
  Give a low/expected/high range per slice and a total, with a recommended cap per slice and the reasoning;
- a short list of decisions the operator must make before implementation.

Be concrete and cite the code. Mark anything unverified as unverified.

## Final message

`handoff_ready` and the PLAN.md path.
