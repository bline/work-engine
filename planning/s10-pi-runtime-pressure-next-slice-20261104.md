# Next S10 slice: outside pressure reaches a running Pi lane

Date: 2026-11-04. Status: proposed execution slice, awaiting its own budget approval.
The current user request authorizes the reviewed adapter commit and this planning
update. It does not start implementation, a worker, Docker execution or inference.
The previous $10 observation slice is accepted and closed; it is not reopened.

## Decision

Connect the accepted observation adapter to one actual provider-disabled
Pi/Docker lane and the existing deterministic host boundary. Prove that context
and budget/time pressure supplied from outside the container reaches the bound
Pi session, and that the outside host retains admission, stop and collection
responsibility. Keep local task judgment inside Pi. No standing outside model
supervisor is added.

Do this before attempting context replacement. Replacement remains a subsequent
adapter/configuration seam against the existing Rust lifecycle contract, as
recorded in `records/rust-lifecycle-s10-context-adapter-integration-note-20261104.md`.
There is no current proposal to redesign that lifecycle.

This slice qualifies the S10 runtime recipient and host boundary. Shared A3 event
delivery, remote clients and AH application-server composition remain owned
elsewhere. Direct qualification of an existing runtime input does not establish
MCP service delivery or permission to implement another relay.

## Current evidence and ownership

- Adapter source is committed as `6f0ecd4cc61eced23893acbf44030f396eac92cf`
  on `implement/pi-context-observer-20261104`, based on `85f4c871`. Its exact
  seven-file manifest is `2cead9446c49ffb78ffaecffdf4a699c1b5c7c1f6815165b3d0c588cf1dd3348`.
  Independent read-only review accepted the repaired subject. Cold build, five
  tests, strict Clippy, a constructed-state actual SDK getter probe and a local
  mixed-record execution passed. Final selected-input custody covers 19,045
  entries; the historical first-run gap remains recorded.
- The S10 runtime handoff and evidence are in
  `/home/bline/.local/state/work-engine/s10-pi-pressure-20261104T2034/INTEGRATION-HANDOFF.md`.
  Earlier accepted real-coding and current-Rust source-integration results retain
  their exact source/environment limits; none qualifies the new pressure join.
- Backend session `01a195d4-b7a7-77f2-ba65-277669c0ec97` still owns the signed
  task/MCP/Pi fixture join and its current repair/review. Its candidate is not an
  accepted dependency merely because it has a private commit.
- Query session `01a1a84f-3760-7ce0-b46a-ad5b2f520c20` owns the narrow shared
  read/CLI work. AH session `01a1a862-034f-7750-b86b-de29e11cf327` now owns
  application-server planning, superseding the proposed backend AH allocation.
  Board 4395/4396 records those current boundaries.
- Reviewed connector plan revision 4 is `d86df598`, subject SHA256
  `026fca8e02c317fa60997d20a04c71625f006e8194aad00584a751e634765da9`.
  It explicitly preserves admitted S10, backend and query scopes. AH composes
  the runtime contract; it does not replace its launcher or custody owner.

## One deliverable and its acceptance

Deliver a reviewed, reproducible runtime pressure qualification for one pinned
local profile, using disposable Docker state and a fake/provider-disabled
inference route. Use the actual SDK and runtime receiver, rather than proving
only an adapter function or arbitrary JSON pass-through.

1. Bind the exact accepted host/profile, SDK/image/tool closure, container and
   physical Pi session before admitting the fixture. Use actual SDK session,
   model/window and context estimates. Keep estimated, unavailable, invalid and
   stale evidence distinct. Constructed usage and fake responses are labelled.
2. Feed context evidence and existing-owner budget/time output through the
   selected existing runtime input. Demonstrate the pressure in a subsequent
   actual SDK event/provider request captured by the fake provider. Preserve
   producer identity, units and independent cursors. A transport ACK, queued
   message and observed request input are distinct receipts; none proves model
   comprehension or task completion.
3. Exercise the selected existing host policy: a declared stop/deadline fences
   new work, cancellation uses the existing actuator, and actual wait/exit and
   cleanup are collected. Do not add accounting, infer exact billing settlement,
   or claim that steering immediately interrupts an in-flight request. Keep
   unavailable measurements and delayed/in-flight work visible.
4. Check wrong session/generation/model, stale or missing readings, duplicate
   or reordered observations, and a recipient unavailable during delivery.
   Reject or hold them according to their existing owners. A generation change
   cannot authorize a new context. No replacement or automatic reset occurs.
5. Freeze the final source/profile and bind source, test input, SDK/dependency,
   toolchain and emitted executables from the first new validation run. Obtain
   a separate read-only review, repair material findings within the same cap,
   preserve failures, and collect every process. Return a small runtime-input
   handoff that the A3/AH owners can consume without adopting another store.

Completion means this local runtime boundary is qualified. It does not complete
the shared service join, full S10, the stronger retained-snapshot profile,
operational replacement or release acceptance.

## Source boundary and first checkpoint

Use an isolated worktree and prefer a new bounded runtime integration module or
standalone package. Reuse `pi-context-observer`, `context-pressure`, the existing
host/receiver and the canonical budget producer. SDK-required JavaScript glue
stays minimal; new reusable first-party policy/control code is Rust.

Do not edit the backend's active source, task admission/current registry, effect
leases, query/CLI, AH server, UI, shared delivery service or installed SDK/tools.
If a small change to an S10 runtime file is necessary, bind its exact source and
coordinate file ownership with active consumers before mutation. A shared API or
new service dependency outside that boundary is a checkpoint result, not an
invitation to implement the adjacent component.

The included checkpoint is **20 elapsed minutes or $3 total**, whichever comes
first. It must demonstrate one actual addressed input reaching a disposable Pi
runtime, identify the reusable host enforcement boundary, and bind the exact
source placement/ownership without relying on the backend's unaccepted repair.
Check actual remaining account and objective capacity before continuing.

Continue only when the complete remaining validation, independent review,
repairs and closure fit the grant. If the runtime input requires an unaccepted
shared-service change or the route is inconclusive at the checkpoint, stop
admission, collect work and return the precise dependency. Do not substitute a
second static fixture or fund another feasibility attempt automatically.

## Proposed budget

Hard ceiling: **$15 total / 90 elapsed minutes**, whichever occurs first.
Target $10–12. These are speed-adjusted API-equivalent accounting dollars,
not measured subscription quota or a provider billing guarantee.

| Included work | USD |
| --- | ---: |
| Root admission, integration and accounting | 3 |
| One bounded builder and focused validation | 4 |
| Independent review and bounded recheck | 3 |
| Repairs | 2 |
| Collection and reporting | 1 |
| Contingency | 2 |
| Total | 15 |

The review allowance reflects the completed observation slice's measured
$2.490772 review cost. Role allowances are forecasts within one objective cap.
Bind actual identities and canonical controls before their inference. Use one
direct builder and a separately launched read-only Astra reviewer; choose the
builder from current model guidance when the packet is admitted.

The clock starts at genuine approval and includes setup, all actors, any context
refresh, tests, review, repairs and closure. Stop admitting productive work at
**60 minutes or $9 total**, protecting 30 minutes/$6 for in-flight work, review,
repair and closure. No retry, reviewer replacement or context refresh resets
the limits. Pre-approval commit/planning work is not silently charged to a
future grant or relabelled as part of the already-closed observation slice.

No implementation budget is activated by this proposal. At the hard ceiling,
report accepted, partial or held standing and collect outstanding work; no
extension is automatic.

## What follows

If this pressure join is accepted, scope one actual context-replacement adapter
qualification under the existing Rust owner: select the Codex native configured
actuator or Pi fresh-session seam, preserve continuation meaning and authority,
and verify replacement/recovery with durable evidence. Summary compaction is
not assumed equivalent. That work needs a separate estimate informed by the
accepted runtime boundary; it is not hidden inside this $15 proposal.

The eventual working-base proof composes accepted task/MCP execution, outside
pressure and one Rust-governed continuity transition. AH and shared delivery
have their own owners and acceptance. This ordering avoids an omnibus S10
assignment while keeping the remaining release consequences explicit. There
is no defensible aggregate release budget or completion date yet.

Revisit this proposal if the backend or AH owners publish incompatible runtime
contracts, the selected source/profile changes, account capacity is inadequate,
or the first checkpoint falsifies the direct runtime-input route. Such evidence
changes the next decision; it does not silently enlarge this assignment.
