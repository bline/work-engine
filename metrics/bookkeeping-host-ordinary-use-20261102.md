# Bookkeeping host: first ordinary-use observation, 2026-11-02

Scope: the coordinated phase1 workload owned by sessions
`01a195d4-b7a7-77f2-ba65-277669c0ec97` and
`01a19b6d-db5b-7a50-b4d1-7f69e34bb20e`. This is one shared workload, not two
independent trials. Its builder started before host opt-in and retained its
original collaboration route. Two subsequent fresh Astra reviews used the host.
This observation reads their records; it changes no peer-owned state or receipt.

| Host operation | Native execution | Reviewer inference cost | Planned allowance | Result |
| --- | --- | ---: | ---: | --- |
| Initial joined review | 22:11:35.806–22:17:32.338 UTC; 356.532 s | $4.127758 | $4.00 | Five P2 product findings, changes required |
| Fresh repair-delta review | 22:52:52.421–22:54:16.433 UTC; 84.012 s | $1.418312 | $0.75 | Repairs accepted within limited delta scope |

Costs are speed-adjusted API-equivalent estimates for reviewer inference. Their
$5.546070 sum excludes root coordination and is not host overhead. Planning
allowances were exceeded by $0.127758 and $0.668312 respectively; no hard per-role
spend-cap compliance is established. The whole phase reported $28.7216442 known
against $30, with later reporting usage unsettled. The larger product phase
closed incomplete and disabled; accepted review repairs did not become full
product acceptance or a live product deployment.

The host observed exact-turn Astra/high/default settings for both operations,
captured their final JSON results, and recorded 170 helper subprocesses:

| Helper | Initial review | Delta review | Total |
| --- | ---: | ---: | ---: |
| Context reader | 45 | 10 | 55 |
| Usage ledger | 23 | 6 | 29 |
| Scope budget | 24 | 8 | 32 |
| Native role adapter | 45 | 9 | 54 |
| Total | 137 | 33 | 170 |

All those records have terminal status, no timeout and no recorded leftovers.
Both operations are closed with no recorded mechanical failures. That is
evidence that supported dispatch/observation/collection worked on real reviews;
it is not a comparison of root time or operating cost against a manual control.

Three adoption problems and one reporting discrepancy are visible:

1. The first dispatch refused an absent result directory before creating an
   operation or native session. The root inspected source/state, created the
   directory and safely retried the identical request. This is explicit setup
   friction, not evidence of a duplicate launch.
2. Six root-authored Python programs inspected the host's state during the first
   review. They reconstructed identity, settings/realization, context, budget
   and phase views. The root also directly reran the settings reader. Mechanical
   polling moved into the host, but consuming its evidence still took custom
   programs and multiple model calls.
3. Required read-only realization for a retained reviewer remained unqualified,
   so the repair review used a fresh reconstructed context. Its inference cost
   and exceeded allowance must be preserved; there is no matched retained arm
   establishing the size or cause of any cost penalty.
4. The phase closure report says task/accounting receipts were acknowledged.
   The latest delta task receipt `50a654caf45166c08c8417985065cbe9289e1ea23348f7e9323f4d37ba3ca69e`
   is absent from the state's acknowledged list, and
   `final-accounting-ack.stdout` explicitly reports `delivery: pending` for it.
   Older task/accounting receipt acknowledgments exist. The precise gap is the
   latest receipt and the overly broad closure claim; this does not establish
   a duplicate child, lost result or unresolved execution.

There is no measured 50% bookkeeping-turn reduction, isolated root active-time
estimate, or comparable manual baseline for these operations. The six custom
inspections are concrete evidence that the intended removal of root-written
bookkeeping is incomplete. Receipt usability, preflight setup and final
acknowledgment semantics are the clearest next improvement targets. Broader
retained-review support needs its own bounded qualification decision.

Evidence root:
`/home/bline/.local/state/devwits-auth-rollout-20261102/session-management-phase1/`.
Relevant files: `manual-host-review/state.json`, `review-1/result.json`,
`review-2/result.json`, `review-dispatch-preexec-repair.json`,
`final-accounting-ack.stdout`, `RESULT.json`, `RESULT.md`, and `assignment.json`.
The assignment's old `manual_host_optin.pinned_bundle.scope` still says
"nooperationyet"; actual operation state and actor entries establish that this
projection is stale.

Native execution timestamps come from the two reviewers' rollout
`task_started`/`task_complete` events. Root inspection evidence is in
`/home/bline/.codex/sessions/2026/11/01/rollout-2026-11-01T00-38-29-01a195d4-b7a7-77f2-ba65-277669c0ec97.jsonl`
at lines10915,10922,10930,10998,11029 and11051. These six are positively
identified examples, not an exhaustive classification of bookkeeping programs
or all root model calls. Source/CLI discovery and semantic review work are not
silently classified as mechanical overhead.
