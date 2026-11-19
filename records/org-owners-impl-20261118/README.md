# O7 owners implementation (2026-11-18)

Operator objective: implement the four O7 mapping owners at fixture standing, per the planning pass in `planning/PLAN.md` (read-only Opus pass; not independently reviewed). Operator decisions 2026-11-18: O-1 owner definitions as crate doc comments plus these records; O-2 the D8 rule moves to a new slice-campaign-owned crate; O-3 slices A1 $16, A2 $30, B $35 (total $81); O-4 A1 and A2 in parallel, B after A1 merges; O-5 `work-engine-review-hosted` stays `owner_pending` and unchanged. Reviews use Opus xhigh by default and Fable only for difficult reviews (operator, 2026-11-18).

- A1 (`a1/`): S-A check S1, `scope.digest` is the exact-subject digest. Builder RESULT and MATRIX, builder gate `runs/gate-1` and root gate on a clean export `root-gate-1` (13/13), mutants (7/7 killed), review `review/REVIEW.md` (Opus xhigh, accept_with_changes; an earlier attempt stopped at the subscription's 5-hour limit). RESULT's "Known contradiction" section records the hosted crate's opposite `scope.digest` rule (review P2).
- Merged into `rescue-detached-20260911` with the operator's standing approval (2026-11-18).
