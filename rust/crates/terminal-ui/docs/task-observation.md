# One-task supervision: observation contract reconciliation

Prepared 2026-10-13 from UI baseline `d4716fc`. This is the UI consumer's
proposed contract and implementation scope, not an accepted S8 protocol change.
The user authorized this reconciliation. S8 owner acknowledgement, exact source
handoff and backend qualification remain pending; board message 1659 requests
that handoff. No backend or UI runtime behavior changed in this step.

## Outcome and ownership

Inspect one explicitly selected retained task across Overview, Graph and
Conversation. Show its objective, published consequences, outstanding questions,
execution state and result without turning incomplete observation into success.

The task owner owns task meaning, questions, answers and result publication.
Lifecycle owns admission, delivery custody, physical execution and context
replacement. The host composes their observations. The standalone UI consumes
public reads; it opens neither database and imports no writable task, store,
service or runtime owner. Provider-specific identifiers appear in details, not
as the navigation model.

The first slice is explicit endpoint plus task ID, manual refresh/reconnect,
and read-only inspection. Workspace discovery, task creation, question answers,
command sending and campaign mutation are separate later slices.

## Current interface and evidence

[The S8 host contract](../../../../docs/rust-retained-task-host-contract.md)
documents a mode-0600 Unix socket, configured-UID admission, one newline-delimited
request/reply and a 128 KiB bound. The current request is:

```json
{"op":"task.observe.v1","task_id":"task-id"}
```

Optional `delivery_id` and `command_id` select existing read targets. The current
response has `status`, `host_identity`, `owner`, `lifecycle`,
`native_delivery_status`, `command_result` and `control_result`.

| UI need | Current source | Honest interpretation |
| --- | --- | --- |
| Identity and objective | `TaskRecord.task_id`, `subject_id`, `kind`, `objective`, `snapshot_ref` | The selected task and approved source scope, not a discovered workspace inventory. |
| Work position | Task `revision`, `cursor_revision`; lifecycle cursor/context | Separate revision domains. Neither a percentage nor interchangeable command preconditions. |
| Inputs | `TaskRecord.inputs` | Owner-published input intents; publication alone does not establish queued or completed lifecycle delivery. |
| Questions | `questions`, pending/answered state, recipient and scope | Outstanding question or recorded answer. Do not infer that every pending question halts native work. |
| Completed consequences | `completed_effects` with effect/delivery IDs and receipt digest | Recorded owner consequences; no invented operation title, timestamp or full tool log. |
| Result | Optional `AttributedResult`, source references and attribution | A published task result. Keep lifecycle settlement/admission independent. |
| Execution | Lifecycle snapshot and native segment collection | Distinguish logical delivery, queue attempt, native segment, context and thread. Native terminal state alone is not task completion. |
| Runtime identity | `HostIdentity` build/profile/incarnation and pinned identities | Restart and selected implementation identity, not an authority grant. |
| Conversation | Input text, question/answer text and attributed result body | A task conversation projection; not a complete agent transcript or a chronological event log. |

Source verification used Codebase Memory Tier 2 and exact reads. Coverage for
the four paths below reported content-verified/no recorded issue at generation
`2026-10-13T19:08:09Z`; this is best-effort coverage. The dispatch call graph
contains ambiguous cross-crate method resolutions, so the observe arm and its
helpers were checked directly rather than treating those graph edges as proof.

The S8 sources below are working-tree evidence, not publication/acceptance:

| Repository-relative path | SHA-256 |
| --- | --- |
| `docs/rust-retained-task-host-contract.md` | `2cb6f2e7fca9f3a60e8d84d6dde8cf8d5b9f6eccabca39dee15453eaed42c286` |
| `rust/crates/work-engine-task-host/src/lib.rs` | `503214560fdc207219f13e52e1975b3265468da3eb91333b64af98f2f11e1ed3` |
| `rust/crates/retained-task-workflow/src/lib.rs` | `04993b6c6b82c75d1e0802d7bfd462074c71a65a42725e9feee5fc0bad0ab8f7` |
| `rust/crates/lifecycle-store/src/native.rs` | `f61ed252aa494005272870d2c5baef9b0e2e854687ce287e0e73b027bb6a9632` |

## Gaps requiring owner reconciliation

1. **Partial reads lose their meaning.** The observe arm uses `.ok()` for
   lifecycle/native reads and `.ok().flatten()` for command reads. Thus a null
   component can mean failed read, no record, no requested target or no owner.
   `status: observed` does not establish that every component was read.
   Native status can also contain an error object rather than a segment array.
2. **The composite is not atomic.** Task, lifecycle, native and command reads
   happen separately. Owner revision, lifecycle journal cursor and host
   incarnation do not establish one shared commit. A refresh must not create a
   causal edge between mismatched observations.
3. **Native history has a capacity boundary.** The store selects up to 65 rows
   and returns unavailable when more than 64 exist. That is not an empty history.
   Native status currently lacks a cursor identifying its own observation cut.
   The first UI slice can accept explicit capacity refusal; pagination is not a
   prerequisite unless the owner chooses to support larger collections.
4. **Ordering and scope are limited.** Segment sequence is supplied, but there
   is no common event order across inputs, questions, effects and results. A
   targeted lifecycle delivery and subject-wide native segments have different
   scopes. No complete event history, graph of agents, transcript or qualified
   usage percentage follows from these fields.

## Proposed read contract

Prefer a separately versioned task observation response, for example
`task.observe.v2`, with v1 behavior retained. Exact naming and shared Rust wire
package placement belong to the host/lifecycle owners. Do not place task
semantics into the generic lifecycle snapshot or create a universal backend
protocol for this single consumer.

The response should correlate the requested task/targets and identify its
protocol version and host incarnation. Each independent component should
preserve the following distinctions (names are proposals):

| Component state | Meaning |
| --- | --- |
| `present` | Validated value plus its owner-provided identity/revision and scope. An empty collection is meaningful only inside this state. |
| `absent` | A successful lookup established that the requested record does not exist. |
| `not_requested` | No target was selected for this optional lookup. |
| `not_applicable` | The owner establishes that the component does not apply. |
| `unavailable` | The read could not establish a value; bounded reason such as timeout, capacity, unsupported or owner failure. |

Unknown legacy nulls remain **unknown**, not retroactively assigned one of these
states by the client. Details should retain a safe diagnostic code where the
owner supplies one; raw internal errors and secrets are not UI payloads.

Composition should explicitly remain non-atomic unless the owner provides a
stronger guarantee. Component bindings should include task/subject and relevant
delivery, owner revision, lifecycle store/stream/cursor, and native observation
revision where available. A request ID only correlates a response; it is not a
cross-store consistency token. Do not fabricate a native revision from the
separately read lifecycle snapshot.

The UI records local receipt time as **last read**, never as source event time.
Last-known data stays tied to endpoint, task, host incarnation and component
identity. On partial refresh it remains visibly stale per component. A late
reply cannot replace a newly selected task. Identity changes clear inspection
anchors; unchanged reads preserve selection and scroll. The client supports
bounded explicit reads and cancellation of stale results, never automatic
effectful retries.

Owner response needed: accepted source revision; versioning/compatibility
choice; component error and capacity semantics; available native observation
identity; public client ownership; and a provider-disabled fixture/installed
host route that qualifies this exact boundary. These are outstanding handoff
items, not authority inferred from publication of this document.

## First UI slice

The proposed landing view is **Overview**, with Graph a separate view rather
than the former Dashboard label. A persistent header shows:

```text
Inspecting: <task title>  ·  <task ID>  ·  Read-only  ·  Last read <age>
Overview   Graph   Conversation   Lifecycle              Ctrl-P Commands
```

Overview presents the objective, published result if any, outstanding questions,
recorded inputs/effects, and separate execution/admission state. Empty and
unavailable sections have different copy. There is no synthesized overall
progress percentage or universal green success state.

Graph initially shows only relationships established by identifiers: task to
inputs/questions/result and delivery to native segments/context transitions.
Missing bindings remain unlinked. It does not invent agents, branches or review
relationships. Node selection changes inspection; double-click also opens
Conversation. Single click followed by the Conversation tab has the same scope.

Conversation groups attributable task inputs, questions/answers and result text.
Without an event order, use labeled groups and owner-supplied order within each
group; do not interleave them into a fabricated chronology. A selected node with
no attributable text gets an explicit empty state. Source Markdown copying and
configurable detail remain available. The context pane shows selected identity,
evidence and execution detail; on narrow terminals existing panel switching
continues to work. The retained composer stays disabled in read-only observation
and keeps drafts. Inspection never changes its recipient.

## Implementation sequence and acceptance

1. Reconcile the owner handoff above; bind fixtures to the chosen version and
   exact source. UI work may proceed against clearly labeled draft fixtures,
   but fixture success does not establish a live host contract.
2. Add a bounded task read client/adapter without writable-owner dependencies;
   connect explicit endpoint and task ID and preserve per-component freshness.
3. Add the shared inspected identity and Overview/Graph/Conversation projections.
   Retain lifecycle-only connection and existing fixture/replay modes.
4. Join the accepted installed host in a disposable provider-disabled scenario.
   Demonstrate task/question/result and native handoff, partial read failure,
   capacity refusal, restart and stale-reply behavior through actual UI frames.

Acceptance includes absent versus unavailable versus known-empty, pending
question with and without active execution, published result with unsettled
delivery, multiple native segments for one delivery, mismatched component
identity, no invented chronology, graph-to-conversation scope, responsive reads,
draft preservation and source-text copying. Record any case the owner contract
cannot establish as a gap, not a passing UI inference.

No transcript streaming, full history, task-list discovery, answers, send,
replacement, grant changes, live provider call or deployment is part of this
first read-only slice. Those remain separately scoped follow-ups.
