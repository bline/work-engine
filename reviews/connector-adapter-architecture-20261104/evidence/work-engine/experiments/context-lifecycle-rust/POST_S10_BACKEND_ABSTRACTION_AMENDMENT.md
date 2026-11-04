# Proposed post S10 backend abstraction amendment

**Status:** proposed for evaluation by the context-lifecycle plan owner,
2026-10-21. The user requested this adjacent amendment for that session to
evaluate and link. It is not an accepted amendment or an implementation grant.
The [implementation plan](IMPLEMENTATION_PLAN.md), S10 qualification, and any
separately authorized existing-Codex authentication repair retain their scopes.
The plan owner assigns slice identifiers and integrates accepted changes.

## Intended outcome

Extend the existing Rust context lifecycle so a retained workflow can use either
Codex's managed execution or direct OpenAI Responses inference funded by ChatGPT
plan quota. Work Engine retains authority over context replacement, semantic
preservation, admission, custody and settlement. Backend selection changes how
those contracts are realized and evidenced; it does not create another lifecycle
owner. Future adapters should be possible through the same bounded contracts.

The direct route is a deliberate alternative to Codex. Implementing it is not
conditional on first migrating OAuth through Codex. The user's intended funding
is subscription quota; separately billed API-key inference is not an acceptable
implicit fallback for this route.

This work follows S10. It does not expand the current trial, reinterpret its
evidence, or make direct-API qualification a new prerequisite for S10 acceptance.
The owning session should reconcile the actual S10 exit and outstanding
obligations when admitting the later work; the label S10 alone is not evidence
that every replacement or operational claim has been established.

## Existing design to preserve

The [lifecycle design](DESIGN.md#context-replacement-protocol) already separates
context replacement semantics from a selected provider capability. Its
[provider and tool contracts](DESIGN.md#provider-and-tool-contracts) explicitly
allow a harness to own execution, require tested capability profiles, and forbid
a second tool loop for work already owned by that harness. This amendment extends
those boundaries rather than proposing a parallel session manager.

The implementation inspected on 2026-10-21 provides these starting points:

| Existing owner | Reusable responsibility | Current integration boundary |
| --- | --- | --- |
| [Lifecycle transition core](../../rust/crates/lifecycle-core/src/state.rs) | Quiesce, capture, verify, checkpoint, switch, rehydrate and reconcile; retain recovery states | Successor readiness currently requires a provider-thread identity |
| [Canonical material port](../../rust/crates/lifecycle-service/src/material_port.rs) | Source projection, dependency observation and currentness fencing | A direct transcript source needs an attributed projection into this existing material boundary |
| [Semantic continuation](../../rust/crates/lifecycle-semantic/src/continuation.rs) | Objectives, completed consequences, decisions, commitments, interactions, authority and next action | These meanings are independent of the inference transport |
| [Semantic service](../../rust/crates/lifecycle-service/src/semantic.rs) | Checkpoint establishment and successor-use assessment | Compiler/verifier execution and rehydration evidence use native runtime bindings |
| [Domain execution](../../rust/crates/lifecycle-service/src/operations/mod.rs) | Selected task tools and turn integration | Codex thread creation, tool loop, history and event normalization |
| [Native store evidence](../../rust/crates/lifecycle-store/src/native.rs) | Ingress lineage, effect settlement and usage accounting | Settlement interprets Codex event shapes; usage selects cumulative native counters |

These are source observations, not proof that a direct backend already works or
that S10's live semantic acceptance is complete. Rebind the source baseline when
planning implementation; concurrent authentication work can change the adapter.

## Proposed boundary changes

### Context and execution identity

Preserve distinct logical subject, context generation, delivery, effect, attempt,
runtime incarnation and provider identities. One Work Engine turn may contain
several direct inference requests and tool effects. Each request needs its own
attempt and provider-response lineage without becoming a new domain delivery.

A backend execution context may have a remote thread handle, a connection-scoped
continuation handle, or locally retained request history. A local identity must
not be presented as evidence that the provider allocated persistent state.
Evaluate the current mandatory successor-thread field and its consumers before
choosing a type or schema change; preserve historical identity meaning and codec
lineage through any migration.

Backend switching, context replacement and token refresh are distinct operations.
An authorized switch can establish a new continuation relationship, but cannot
claim that hidden state transferred or that a required native session remained
the same. Existing same-session and reviewer-exposure contracts determine which
routes are admissible.

### Execution capabilities and tool ownership

Expose the operations and evidence needed by domain work, compilation,
verification and successor qualification. Capabilities describe what a selected
backend/profile can establish: history availability, continuation, interruption,
usage, context limits, tool mediation and observed execution identity. Unsupported
or unestablished capabilities remain visible to admission and recovery.

Codex can retain its thread history and tool loop where the admitted profile
satisfies the owning contracts. Direct Responses needs a Work Engine-owned loop
that submits requests, accepts complete tool calls, routes them through existing
operation admission, and returns attributed results. Exactly one owner executes
each loop and tool effect; otherwise retries or overlapping harnesses could
duplicate consequences.

The current narrow domain tool set is the proposed initial direct profile.
Namespaced function calls are a starting implementation choice. Rebuilding the
JavaScript code-mode runtime is not an acceptance requirement unless the selected
workload demonstrates a need. Parallel tool calls require operation-specific
concurrency and authority handling, not unconditional parallel execution.

The existing `TextTurnPort` lacks tool exchange. Evaluate extending or composing
ports at its owning boundary; this proposal does not prescribe one large trait,
a crate count, or a new generic agent framework.

### Transcript and request context

Distinguish two retained products:

- The inference transcript supports requests within one context generation:
  messages, provider output items, tool calls and outputs, and supported opaque
  reasoning state with exact request/response associations.
- The verified continuation preserves required meaning across context
  replacement. It remains owned by the existing semantic lifecycle rather than
  becoming an automatic transcript truncation policy.

The direct implementation needs durable request assembly and recovery boundaries
for both. Canonical source projection should expose relevant transcript material
with source identity, trust, availability and revisions. Recording text does not
make it governing instruction or authorize an action. Opaque reasoning items are
provider artifacts, not decoded reasoning or portable semantic evidence.

Under the current HTTP plan-use interface, the client sends the context required
for each request. A successor can therefore be realized as a newly assembled
context under a new local generation, initialized from the admitted continuation.
It still needs actual successor-use evidence before ordinary admission resumes.
The compiler and verifier also use the selected inference adapter, with separately
bound input histories and the exposure evidence required by their policy.

### Pressure, completion and recovery evidence

Map backend observations into the existing lifecycle predicates while retaining
their source. Codex history/readback and direct response streams provide different
evidence; a direct adapter must not manufacture Codex events or independent
readback claims to satisfy old validators.

Retain exact request/profile bindings, provider response identifiers, stream
events, tool receipts and terminal observations as required for settlement.
Request hashes identify submitted material; they do not independently prove what
the remote service executed. A completed model response does not settle pending
tool effects. Closing a stream establishes local disconnection, not definitive
remote cancellation, final usage or permission to retry.

Aggregate direct usage across distinct requests in a logical turn without
double-counting replayed observations. Missing cache, reasoning or final usage
counters remain unavailable rather than zero. Quota observations and funding
provenance remain separate from token counts and hypothetical API-price estimates.

Context occupancy, model limits and pressure inputs need explicit measured,
configured or estimated provenance. Cumulative tokens consumed are not current
context occupancy. Preserve the existing policy boundary: pressure informs a
replacement decision and cannot waive semantic preservation or effect settlement.

### Authentication and subscription funding

The direct adapter needs the documented Sign in with ChatGPT registration,
account selection, granted-scope validation, protected credential storage,
serialized refresh and sign-out lifecycle. Credentials remain outside model
context and evidence payloads. Record the selected account/registration binding
without leaking tokens.

On missing authorization, exhausted quota or revoked access, preserve the actual
failure and stop affected new entry. Silently selecting an API key would violate
the user's funding requirement. Refreshing credentials does not create a new
Work Engine context or justify replaying an uncertain request.

## API premises and unresolved compatibility

As documented on 2026-10-21, the plan-use interface supports OAuth-authenticated
`POST /v1/responses` and account-specific `GET /v1/models`. HTTP requests require
`store: false`, `stream: true` and client-supplied context. HTTP
`previous_response_id` and persistent conversations are unavailable; WebSocket
continuation is limited to the authenticated connection. Function/custom tools
are supported through namespaces or `additional_tools`.

The current restrictions include system-role message items, hosted MCP, Responses
`tool_search`, top-level `programmatic_tool_calling`, and parameters such as
`max_output_tokens` and `temperature`. Existing Codex metadata does not establish
direct-route entitlement or capability. A stream must reach `response.completed`
for provider completion; incomplete, failed and disconnected streams have
different recovery consequences.

The initial qualification needs to resolve:

- Actual access to the selected model and required reasoning controls.
- Acceptance and replay behavior of encrypted reasoning items.
- Exact tool-call, usage and error shapes, including failures after streaming
  starts and missing terminal usage.
- A defensible source for context capacity and request-size observations.
- How disconnects, token renewal and restart interact with entered requests and
  completed tool effects.
- Task quality and quota consumption after changing Codex's default prompt and
  code-mode exposure to explicit Work Engine instructions and tools.

These questions are qualification targets, not established limitations or reasons
to substitute another backend silently. The documented flow is for open-source
and locally hosted applications; paid or remotely hosted offerings have a
separate integration path. Account and deployment eligibility remain prerequisites
of any live proof.

Primary documentation: [models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference),
[preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations),
[registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in),
[accounts and sessions](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions),
[errors and recovery](https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery),
and [deployment scope](https://developers.openai.com/siwc/token-sharing-open-source).
Recheck these evolving premises at implementation admission.

## Suggested post S10 work packages

The plan owner can split or combine these by coherent ownership and evidence.
Their outputs, rather than this suggested decomposition, define the proposal.

| Package | Proposed result | Evidence needed |
| --- | --- | --- |
| Boundary and identity design | Accepted mapping of lifecycle, execution-context, request and tool-loop ownership; capability and evidence contracts; historical compatibility decisions | Trace existing consumers and identify exact contract changes, including mandatory native-session requirements |
| Codex integration through the revised boundary | Existing admitted behavior remains reachable through the abstraction | Actual service composition preserves context cycling, custody, recovery and supported tool semantics |
| Direct Responses implementation | OAuth and direct transport joined to durable transcript assembly, the selected tool loop and all semantic-control roles | Controlled protocol/fault cases establish mechanics and truthful evidence handling |
| Direct lifecycle qualification | Retained work survives context replacement and restart using subscription-backed direct inference | Separately authorized live semantic evidence plus controlled failure/recovery evidence for the exact selected profile |

No package introduces a default model change. Resolve model capability against the
selected account and the owning evaluation contract. A provider or prompt change
requires explicit comparison provenance; historical efficiency results remain
historical rather than being relabeled as the new backend's baseline.

## Proposed acceptance consequences

The admitted implementation should establish the following at the real retained
workflow boundary:

1. Both Codex and direct Responses use the same lifecycle authority and semantic
   contracts. Backend choice and capability limitations are observable.
2. An API tool call executes through the existing admission owner, its result
   reaches the next inference request, and the logical turn settles only with
   the evidence required for its entered effects.
3. Two context replacements preserve completed-result meaning, pending human
   interaction, superseded decisions, governing constraints and queued-input
   order. The successor uses the retained result without repeating its effect.
4. Restart and faults around request entry, tool execution, result persistence
   and successor activation preserve identity, uncertainty and custody. An
   uncertain external effect cannot become a safe retry merely through restart.
5. Stale source or authority revisions prevent dependent activation or entry;
   compiler/verifier separation retains its selected exposure guarantees.
6. Quota refusal, refresh failure, malformed streams and cancellation produce
   truthful outcomes without API-key fallback, fabricated success or invented
   usage. Controlled evidence and actual subscription usage remain distinct.
7. Historical Codex receipts retain their meaning; direct evidence is attributed
   to its real source. Any schema migration has explicit compatibility and
   recovery evidence rather than retroactive conversion of observations.

Acceptance is scoped to the selected tool and backend profiles. It does not imply
replacement of every older Node app-server capability, arbitrary backend switching,
provider-independent hidden reasoning, or hosted tools absent from this route.
Live semantic adequacy cannot be inferred from scripted responses alone.

## Evaluation requested from the plan owner

Evaluate the proposed outcome against the completed S10 boundary; identify which
existing ports should be extended, any competing ownership, and the minimum
contract/schema changes. Reconcile the remaining direct-route questions into
bounded qualification work and decide the final post-S10 slice structure.

If accepted, link this amendment or integrate its accepted content into the plan
with a recorded disposition. Implementation, live inference, migration and
operational cutover retain their separate admission and authority boundaries.
