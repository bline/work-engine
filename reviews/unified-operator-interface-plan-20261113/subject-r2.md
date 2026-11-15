# Unified operator interface: proposed next delivery plan

Draft revision 2, 2026-11-13 UTC. Prepared after the user requested
`$manual-orchestration`, context/budget/cooperation monitoring and a plan before
further development. This pass changes planning and coordination records only.
The proposal is reviewable; its implementation and review roles have not launched.

## The operator outcome

From either the browser or terminal, select the same work and answer:

1. What is happening, what is waiting, and why?
2. What needs my attention: a question, review obligation, failed source,
   configured limit or due scheduled item?
3. Which exact task, session, turn, candidate and owner evidence explain it?

CLI reads provide the same scoped information in human-readable or structured
form. Browser and TUI have equal semantic coverage; their interaction and layout
can differ. Useful operational visibility must arrive before advanced economics
or a complete historical graph.

The [accepted causal-observability design](../app-server/docs/control-plane-causal-observability-ui.md)
continues to own product meaning, evidence classes and authority boundaries.
The [existing UI roadmap](../rust/crates/terminal-ui/docs/implementation-roadmap.md)
continues to map UI-0–UI-4. This plan selects the next shared delivery path; it
does not replace those outcomes or declare them complete.

## Starting point and gaps

The [first implementation result](../records/unified-operator-interface-first-increment-20261113.md)
binds the local shared-session increment and its validation. Its Rust collector,
CLI, TUI and browser share bounded session observations and source failures.
The browser still polls its mailbox every 60 seconds. This is useful development
evidence, not independent design review or live deployment acceptance.

The existing TUI also consumes `task.observe.v2`. Its five independently scoped
components and six observation states are owned by
[TO2](../docs/rust-retained-task-host-contract.md#to2-independent-task-observation).
Reusing that work must retain its distinctions between absence, unavailability,
refusal and an established empty result, and its identity/late-reply fencing.
Neither independent component reads nor a transcript form an atomic workflow cut.

The Events server has subscription renewal and bounded event replay. Those
capabilities should wake a qualified read; a notification does not establish
task state. The current Events working-tree source is outside `session-query`'s
accepted source pins. The retained accepted parser used in the first increment
does not qualify that newer revision or the remote connector.

Concurrent research/task-host work is not part of this plan's implementation
scope. Board request 5098 asks its root for supported passive reads and native
identity joins. At drafting, its completion was unaccepted. No title, timestamp,
repository path or transcript phrase can substitute for an owner-backed join.

## Shared boundary and navigation

Keep semantic ownership in the existing task, lifecycle, campaign, review,
scheduler, usage and session owners. A Rust projection composes qualified reads
for an explicitly selected scope. The projection owns presentation joins and
derived indexes, not workflow transitions. New first-party shared behavior is
Rust; existing browser code remains a consumer and interaction layer.

The source map for every displayed fact or edge identifies its owner, exact
subject/revision, read/access route, evidence class, observation time, coverage,
confidentiality and unavailable meaning. Establish the source map before fixing
a broader schema, transport service or storage architecture. Reuse existing
contracts where they satisfy the outcome; do not add a second workflow store.

For each admitted owner, the map also names which changes its notification route
covers, what can change without a session event, how reconciliation detects
missed or unsupported notifications, and the applicable read/cutoff limits.
Time-dependent attention, such as a schedule becoming due or a freshness cue
crossing its threshold, needs an explicit reevaluation route even when no record
changes. Clock reevaluation projects the owner's published time/policy; it does
not establish execution or authorization. Unsupported refresh coverage remains
an unmet condition, rather than inheriting the session stream's health.

Begin with explicit endpoint and task/run selection. Show unlinked sessions as
unlinked. Preserve selected work across refresh, and invalidate affected data
on host, account, source, scope or established identity changes. Drill-down must
return to the same work rather than silently selecting a newer session.

## Delivery sequence

| Increment | Concrete outcome | Evidence needed before its claim |
| --- | --- | --- |
| U1: establish the read boundary | Review the existing increment; qualify the selected current Events source; map one task/run to its independently scoped owner reads, exact native identities and per-owner refresh coverage. | Revision-bound source/refresh map, current pin/read tests, supported and missing joins, access/redaction checks, and separate architecture review. An unavailable join or refresh route remains open. |
| U2: one work item across all clients | CLI, TUI and browser select one task/run and show its objective, observed state, waiting reason, questions and linked session activity. Published candidate/review/limit/schedule observations appear where qualified. | The same owner-produced records reach all three clients. Wrong/cross-task identities, source failure, scope changes, stale and late replies, and account changes cannot fabricate success or retarget inspection. Every missing operational dimension is named. |
| U3: timely updates and recovery | Qualify notification, reconciliation and time-driven refresh coverage for every admitted operational owner. Events wake qualified reads where their coverage is established; reconnect/replay addresses the gaps it actually covers. Both interactive clients expose connection state and source age. | Exercise each admitted owner's independent change without transcript activity, a time-dependent attention transition, and recovery after missed notifications. Also test duplicate/out-of-order events, expired subscriptions, restart, disconnect and replay gaps. Session replay alone cannot qualify task/review/limit/scheduler freshness. Retained polling is an explicit reconciliation path where justified; uncovered sources/transitions remain unmet conditions. |
| U4: prove the operator journey | Follow one actual selected run from attention to task/session to exact owner evidence and back in desktop, phone and terminal. | Bound host/source/binary/schema identities, actual rendered frames, controlled failures, and measured source-to-screen delay. Report controlled and actual production evidence separately. Deployment is its own operational step. |

U1 is the recommended next engineering assignment. U2 may land first with a
controlled installed host, provided its evidence class is visible. It must not
wait for complete cost attribution, recommendations or counterfactuals. A missing
campaign source permits a bounded task result, with campaign visibility still
unestablished; it does not permit a campaign-completion claim.

That early landing may precede unresolved production qualification; it still
depends on settled identity, access and projection contracts for the selected
controlled scope. It does not bypass U1's applicable contract work.

For U3, the proposed acceptance target is at most five seconds without manual
refresh for explicitly named sources and transitions in the selected
qualification environment. U1's refresh map becomes a qualification matrix
identifying those sources, transitions, coverage and measurement endpoints.
For owner changes, measure from the instrumented source publication that makes
the change available through its owner read boundary, before the projection
notices it, to its visible rendering in each interactive client. For time-driven
attention, measure from the published due/expiry/freshness threshold to visible
reevaluation. Record clock alignment, timing uncertainty, source cutoff and any
unqualified cases; an API acknowledgement or healthy session connection is not
the rendering endpoint or coverage proof. This target is a proposal, not current
behavior or an SLA.
Transport selection must follow evidence about the local and remote delivery
routes, auth boundary and measured latency. Do not preselect a new always-on
service or assume mailbox polling can meet that target.

## Proposed organization and review

The root owns product scope, budgets, packets, cooperation, result collection
and integration. It dispatches directly, without another supervisor layer.

| Logical role | Bounded responsibility | Proposed selection and continuity |
| --- | --- | --- |
| Read-boundary builder | U1 source map, Events qualification and shared Rust boundary; no unrelated research/task-host changes. | Fresh `gpt-6.1-sol`, `xhigh` while ownership questions remain. A later implementation packet is selected separately at `medium` once the contract is clear. |
| Client builder | U2 across the CLI/TUI/web consumer path against the settled boundary. One owner for the joined user journey avoids parallel DTO drift. | Fresh `gpt-6.1-sol`, `medium`. Retain for ordinary fixes only after a scoped capacity check. |
| Independent reviewer | Challenge each composed subject and its owner/consumer evidence from a separately established context. The designer does not review its own design. | Separate top-level `gpt-6-astra`, `xhigh` for architecture and `high` for bounded implementation, explicitly standard tier. Retain through bounded remediation when its review premise remains valid. |

These are proposed selections, not observed identities or dispatch authority.
Builder launches would request priority tier through a supported explicit
launch configuration; the current root's observed standard tier is preserved.
Before dispatch, freeze the exact packet, confirm the selected model/tier and
budget, check context, identify file/process ownership and collection route,
and perform applicable task-profile screening. Separate worker completion,
review, bounded acceptance and integration decisions remain visible.

After U1, the root chooses the next coherent outcome and its budget from the
actual gaps. Parallel client assignments become useful only after their shared
contract is settled and their write scopes are independent. U3/U4 are outcomes,
not pre-authorized worker launches.

## Monitoring and cooperation

This planning pass has a supervisor-set $6 speed-adjusted API-equivalent cost
cap and $4 warning; these are a cost proxy, not measured subscription quota.
The [assignment record](/home/bline/cbm-work/unified-interface-plan-20261113/assignment.json)
owns the original items, actual session binding, limit provenance, observation
route and continuation. It does not fund the whole implementation plan.

The root collects a bounded `chatboard wait --follow` combining `all`,
`aspect:operator-interface`, `aspect:research-lifecycle`, its role and direct
session channel with local context and budget observations. Context policy is
200,000 tokens for planning and 235,000 for the empirical band floor; pressure
informs a scoped capacity decision rather than authorizing a reset or transfer.
No automatic ended-turn wake capability is assumed. Listener output, errors,
cursors, collection ownership and availability must have a durable disposition.

Cooperation follows the same ownership map: qualified reads and exact identity
relationships come from their owners, not from UI guesses. A reply acknowledging
a request is not evidence that a read contract has been qualified. Relevant
changes reach affected workers before dependent work proceeds. Other sessions'
unaccepted files and independent source limits remain outside UI acceptance.

## Decisions left visible

- Product direction: this proposes a shared operator monitor first, with deeper
  control and forensics following the existing product design. Existing signed
  approval flows keep their owner; new controls are separately scoped work.
- Scope: one explicitly selected task/run first; repository-wide discovery and
  a complete campaign graph need qualified source boundaries.
- Source: qualify the chosen current Events revision and negotiate the exact
  task/session joins. Neither is assumed settled by the first increment.
- Delivery: select remote transport after auth and latency qualification. Live
  publication, persistent services and deployment are operational decisions.
- Economics: publish owner-backed budget/usage with units, cutoff and coverage.
  Missing cost, context or limits remain unknown. Full attribution and workflow
  recommendations follow later.

The next concrete result should be U1's reviewed source map and a bounded U2
packet with its own cost envelope. Broader implementation should follow the
reviewable plan and settled boundary rather than accumulating UI features ahead
of their evidence.
