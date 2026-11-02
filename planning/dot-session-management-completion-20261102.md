# Dot session management completion plan — 2026-11-02

Status: joint proposal prepared by the UI/auth and MCP/backend sessions. The
backend owner concurred with the outcomes and ownership and supplied the
refinements and estimates incorporated here. This records a proposed execution
plan; user acceptance, implementation allowances and activation remain separate.

The user requested that the two existing sessions work out one completion plan
for Dot to start and direct Codex and Claude sessions, with the owner present
or away. This plan must preserve model judgment inside explicit authority and
observable consequences; the count of MCP tools is not an acceptance condition.

## Outcome

Dot can start authorized coding work on either host, inspect progress, give
further direction within the task's authority, handle requests for decisions,
interrupt and close owned work, and retrieve a correlated completion result.
The UI shows the actual task, authority, spend, unresolved questions and result.
The same behavior works in present and away modes with an explicit human
approval or delegation boundary. There is no permanently hosted model courier.

## Existing evidence and boundaries

- The auth/approval UI, signed answer protocol, Redis transport and laptop sync
  are deployed. Source and gallery tooling are in `bline/devwits-auth`, `main`,
  through `fffd8f0`. Redis transports decisions; it is not their authority.
- The MCP adapter has six observation tools, three request/operation/result
  tools and four target provisioning tools. Its current qualified execution
  profile is a scoped read-only Codex diagnostic with an exact initial request.
- Source support, local catalog listing, native rehearsal, OAuth verification,
  private staging and actual Dot session control are different evidence levels.
  Backend owner reports live source `b24135b7` in authenticated read-only mode;
  native protected-resource and Stytch discovery returned 200, but ChatGPT
  plugin-form discovery failed and an actual Dot login/read remains unproven.
  Its separately requested urgent OAuth setup reached the owner passkey prompt
  for the exact client callback. Installation, login and an actual Dot read are
  still unproven. That setup has its own allowance and is excluded from this bid.
- Claude transcripts can be observed. Equivalent session execution and control
  through this MCP surface are not established by that observation support.
- The provisioning contract explicitly excludes fleet human approval and away
  mandates. Budget references in its policy are not spend enforcement.
- The backend owner identifies the older v3 fleet editor as a frozen export:
  50 Rust, 6 protocol and 4 hook tests reported, React unbuilt, no human/live
  integration proof. Those are attributed historical results, not checks run
  by this plan. It is a migration/reuse candidate, not a current UI owner.

Evidence: `skills/session-events/scripts/session_events.py:2189`,
`skills/session-events/HTTP-OAUTH.md`, `rust/crates/session-bridge/README.md`,
and the backend owner's deployment and provisioning receipts. Structural
inspection used Tier 2 graph evidence at generation `2026-11-02T16:31:16Z`,
with verified content for those paths and direct source reads of the catalog,
dispatch and HTTP gate. The graph had no recorded gaps for those paths; that
is not a repository-wide completeness claim.

## One authority model, two interaction modes

| Mode | Owner experience | Admission boundary |
|---|---|---|
| Present | Dot discusses work and brings material decisions to the owner. The owner reviews and signs the exact task or change. | Work and subsequent steering must remain covered by current, verified authority. Being present does not require another approval for every internal step. |
| Away | Dot continues and makes permitted decisions within a previously approved mandate. Questions requiring additional authority are held for the owner's return. | The mandate bounds objective, providers/models, workspaces, effects, concurrency, spend, time and escalation. No response, missing evidence or an expired/revoked mandate cannot expand it. |

Presence is not permission. A mode change must define its effect on new
admissions and work already in flight. Cancellation requests are distinct from
observed termination and settlement. Switching back to present must expose any
ongoing delegation and unresolved work rather than silently losing it.

Keep one owner for each semantic fact. Preserve the existing signed decision
and board authority; derive runtime admission from verified, current decisions.
Do not create a second approval authority in the UI, Redis or an MCP argument.
Any proposed migration of the decision owner requires an explicit disposition.
New runtime/domain implementation is Rust; existing React and thin transport
components retain their current boundaries until an accepted replacement.

## Accounting dependency already in progress

A separate user-approved deterministic bookkeeping host core effort is active:
[bookkeeping plan](deterministic-bookkeeping-host-plan-20261102.md), owner
`01a19d59-05c0-7861-b8d5-b227b13a9f1e`, board #4072, with its own
120 USD / 10 hour cap. It is not an allowance for this MCP/UI plan. Its core
scope is Codex; the optional structured-grant extension is not active.

Phase 0 must agree the accounting/admission boundary with that owner before
adding a parallel spend controller. Reuse qualified core mechanics where they
fit. MCP/provider and mandate semantics still need their own owner; a funded
core plan is not evidence that those capabilities already exist. The bookkeeping
lane also owns a mechanical session-events SKILL synopsis fix in its private
worktree (#4074); preserve it during later overlapping integration.

## Proposed completion phases

Qualify real Dot authentication/discovery and a permitted read against the
exact endpoint before a live writable pilot. Offline phase 0 contract/reuse
work can proceed while the existing owner passkey handoff is pending. The
current connection issue is not proof that writable operation is ready. Freeze the supported provider/client access
policy as well as the separate effect-authority checks; do not assume that a
custom OAuth scope exists or that an identity scope approves a task.


| Phase | Coherent deliverable | Principal acceptance |
|---|---|---|
| 0. Reconcile contracts and reuse | Bind current source, runtime and UI identities; locate existing task, decision, mandate and host owners; freeze versioned public inputs/results and proposed authority boundaries. | Every intended outcome has an owner and a test; reuse claims have current evidence; no competing source of authority. |
| 1. Present-mode Codex vertical slice | A real approved coding task travels UI → verified decision/admission → Dot → owned Codex session → correlated result → UI. Add the minimum safe workspace-write profile and budget controller required by that task. | The owner approves a disposable-repository edit; Dot starts it without a courier model; the exact result, cost coverage and session/process disposition are returned. Changed/replayed/expired decisions are rejected. |
| 2. Complete Codex control | Support further task-bound direction, interruption, bounded concurrency, reconnection and truthful recovery. | Dot sends a follow-up, observes completion, interrupts another run and closes it; duplicate delivery and a lost acknowledgment do not start duplicate work. Unknown custody holds further admission. |
| 3. Claude parity | Bind the native Claude host/retained-session mechanics to the same public outcomes and authority checks. Qualify provider-specific interruption and closure. | The same launch → direction → result → stop tests pass on Claude. Reading a transcript or killing a wrapper is not proof of native settlement. |
| 4. Away mandates | Implement durable approval/delegation, reservations/accounting, expiry/revocation and present/away transitions across both hosts. | Both hosts complete bounded delegated work while the owner is absent; outside-scope/budget/deadline actions hold; revocation and restart preserve the decision boundary. |
| 5. Integrated operator experience | Extend the existing UI or qualify reusable fleet editor components for task/session state, approvals, mandates, mode selection, spend and unresolved questions. Publish Dot's actual capabilities and escalation behavior. | The UI follows authoritative server state, survives reload and shows honest running/pending/unknown/closed states; sign-in alone cannot authorize execution. |
| 6. Joined qualification and rollout | Separate implementation review from activation. Exercise both hosts and modes with real Dot, tokens, decisions, events and cleanup; stage and activate with rollback disposition. | Four real paths (Codex/Claude × present/away), transition/revocation/restart/outage tests, independent review of execution/authority changes, and complete release/accounting receipts pass. |

Phases describe outcomes and useful milestones, not a fixed route through the
code. UI work can overlap backend work once their public boundary is bound.
Each phase can expose a better placement or sequencing choice without waiving
its acceptance conditions. The joined surface must also carry Dot-created
requests for owner decisions and the owner response, not only UI-originated
commands. The older fleet export is at
`/home/bline/Documents/Codex/2026-10-31/task/fleet-v2-prototype`; inspect its
`RESULT.md` and `artifacts/handoff-v3.json` and reconcile unresolved C1 findings
before reuse. Do not add tools merely to reach a target count;
retain compatible controls and add only missing meaningful transitions.

## Proposed source ownership

- Backend session: `01a195d4-b7a7-77f2-ba65-277669c0ec97` owns Rust runtime,
  provider adapters, admission/mandates, MCP composition, events and deployment
  preparation in work-engine on `rescue-detached-20260911`.
- UI/auth session: `01a19b6d-db5b-7a50-b4d1-7f69e34bb20e` owns auth/approval
  presentation and the operator interaction surfaces in devwits-auth `main`,
  consuming the backend's public versioned contracts.
- Both remain directly responsible to the user. Neither becomes a supervisor
  above the other. This session edits the planning artifact; backend ownership
  and collection of its existing work remain with that session.
- Before implementation, name one integration conductor per joined phase and
  assign an independent reviewer outside implementation context. No reviewer,
  builder or model selection is launched by this planning document.

## Cross-boundary requirements to settle in phase 0

Basic task admission, revocation, aggregate budget reservations and first-funded
clock enforcement are prerequisites of the first writable Codex slice. Away
mode adds delegated choice; it must not be the point where basic controls first
appear. Reuse the bookkeeping lane's qualified machinery rather than a second
implementation.

Discussion, draft proposal, authenticated submission, signed per-item decision,
admission, delivery, native execution observation and terminal result remain
distinct. Commit accepted state and its outbox atomically where one domain
owns them; preserve deduplication, conflicting-key rejection, retries and dead
letters. Events wake Dot, with correlation to the exact task/turn/result.
Give structured results to the worker already doing the work rather than
introducing a permanent summarizer or courier.

Use the reserved/delegated material-decision distinction in DESIGN section 11
when deciding what returns to the owner. Not every completion requires a human
question; urgent decisions bypass batching, not authority. Mode changes are
authenticated and race-defined against queued admission, expiry and revocation.

Longer away intervals also need qualified token/session and JWKS refresh, or
an explicit hold requiring reauthentication. Refreshing identity must not renew
a mandate or expand its effects. Native passkey interaction cannot be faked.


Task identity/revision and exact signed decision; provider/profile/workspace
and effects; per-task and aggregate cost/time/concurrency bounds; event and
operation correlation; native versus process/session identities; stale-result
handling; grant/mandate revocation; retry eligibility; and action-time consumer
verification of the approval. Reuse the existing answer format if its signed
bytes cover the required meaning; do not silently reinterpret an old signature.

A deterministic admission controller must account for all actors of an
objective, including coordination and review, before further funded work.
Provider telemetry can lag: distinguish observed spend, reservations, estimates
and unknown settlement. A stale meter or process exit cannot establish safe
headroom or completed remote work. Preserve a closure reserve and stop new
admissions while unresolved authority or budget evidence affects them.

## Estimates and next decision

These are provisional speed-adjusted API-equivalent cost ranges, not an actual
bill, subscription quota entitlement or implementation grant. They include the
actors, review and closure for the named workstream. Rebid after phase 0;
prototype fit, Claude control, the bookkeeping boundary and approval semantics
can change the remaining scope materially.

| Workstream | Estimated allowance | Engineering time | Coverage |
|---|---:|---:|---|
| Backend phase 0 | $8–12 | 1–1.5 h | Contract/reuse/owner map and first vertical-slice bid |
| Backend phase 1 | $18–28 | 2–3 h | Present-mode Codex coding task with authority, spend and closure proof |
| Backend phase 2 | $12–20 | 1.5–2.5 h | Codex direction, interruption and recovery |
| Backend phase 3 | $16–28 | 2–3 h | Claude parity and native settlement |
| Backend phase 4 | $20–34 | 2.5–4 h | Away mandates, routing and transition races |
| Backend phase 6 | $12–20 | 1.5–2.5 h | Joined backend qualification and release preparation |
| UI/auth work across phases 0–6 | $20–35 | 3–6 h | Contract participation, operator controls, signed task/mandate presentation, state/results and frontend validation/review |
| **Combined incremental MCP/UI envelope** | **$106–177 (round to $110–180 for planning)** | **Backend 10.5–16.5 h plus UI 3–6 h, partly parallel** | Includes each stream's coordination, review, remediation allowance and closure once |

The backend estimates were supplied by its owner; the UI/auth estimate is this
session's bid. Both are planning ranges, not completion guarantees. The existing
OAuth prerequisite has its own allowance (backend estimates another $3–6 /
0.5–1 h only if newly required; do not fund it again while already covered).
The already approved bookkeeping core is separate and is not added to or
borrowed by this envelope. The current estimate must be reduced or revised if
phase 0 establishes overlapping work; it does not authorize rebuilding that core.

Reserve at least 20% of every admitted phase for review, reconciliation and
closure; it is included in the estimates, not added afterward. One phase-level
ledger must include every participating session and all relevant provider
calls. Keep other simultaneous objectives separate, with unresolved attribution
shown explicitly rather than counted twice or called zero.

Recommended next authorization: **phase 0 only, $10 / 75 minutes across the two
sessions**, productive boundary $8 / 60 minutes and protected $2 / 15 minute
closure reserve. This is a proposed first cap; if the contract inventory and
UI check cannot fit, return a bounded rebid. It does not admit phases 1–6.
Phase 0 returns exact source placement, reuse and dependency disposition,
versioned inputs/results, provider qualifications, acceptance fixtures and a
bounded bid for the first real Codex vertical slice. The user can accept or
revise each next coherent slice with its concrete review/activation boundary.

## Planning provenance and disposition

UI/auth planner and editor: `01a19b6d-db5b-7a50-b4d1-7f69e34bb20e`.
Backend contributor: `01a195d4-b7a7-77f2-ba65-277669c0ec97`.
Its durable contribution is
`/home/bline/.local/state/devwits-auth-rollout-20261102/session-management-plan/backend-contribution-20261102.md`.
This is peer planning and source-owner concurrence, not independent review of
an implementation or transfer of either root's authority.

The planning pass uses a supervisor-set $4 / 30 minute guardrail across the two
sessions; it is not a numeric user grant or old implementation money. The raw
backend interval from 17:49 overlaps its separately authorized OAuth setup. Raw
combined interval figures are conservative upper bounds, not settled additive
planning charges. The contribution follows the backend's 18:03:15 passkey-wait
boundary. Exact measurement sources and this limitation are retained in
`/home/bline/.local/state/devwits-auth-rollout-20261102/session-management-plan/assignment.json`.

No new worker/reviewer, runtime implementation, native coding task or deployment
was launched by this planning pass. The backend keeps its existing human OAuth
handoff; the bookkeeping owner keeps its separately authorized work. The only
work-engine source write from this planner is this dated proposal.
