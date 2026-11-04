# Bounded inference implementation plan

**Status: proposed, 2026-10-21.** The user authorized this planning document and
its planning agent. This document proposes later implementation units; it does
not authorize provider requests, credential operations, migration or cutover.
It does not modify the [lifecycle implementation plan](IMPLEMENTATION_PLAN.md),
accept the [post-S10 amendment](POST_S10_BACKEND_ABSTRACTION_AMENDMENT.md), or
change separately authorized S10 and existing-Codex authentication work.
Already granted authority remains effective within its own scope; this proposal
adds no approval ceremony for work that is already authorized.

## Target and limits

Work Engine should own bounded operation contracts and their consequences while
direct OpenAI inference, Codex and future Claude CLI adapters supply selected
execution capabilities. The primary boundary is an **operation**, which may
need one inference request, several requests and tools, or a retained harness.
A provider session is one possible realization identity, not the universal unit
of work. Direct inference uses the user's ChatGPT-plan quota requirement; an
API-key billing route is not an implicit fallback.

The first proposed vertical slice is **direct candidate-verification replay**:
submit an immutable continuation candidate and its source basis to a bounded
verifier, then consume its report through the existing semantic assessment
boundary in an isolated, non-authoritative evaluation. This addresses a real
consumer before building a retained tool-using runtime.

The target is selective extraction. It excludes cloning Codex or Claude,
building a universal agent platform, replacing every Node capability, automatic
model routing, deploying the proposed organizational compiler or Plan IR,
replacing operator interfaces, and claiming native-session equivalence for a
direct request. Shell, patching, code mode, hosted tools and arbitrary plugins
are outside the first profile. A later workload can justify a separately bounded
capability without making it a prerequisite for all inference.

[DESIGN.md](../../DESIGN.md) is binding. [PHILOSOPHY.md](../../PHILOSOPHY.md)
supplies non-normative rationale. The selected defaults below are proposals,
revisable when another route preserves the required outcomes and authority.
They do not turn task categories into a permanent routing table.

## Existing ownership and dependency inventory

The owning [provider and tool contracts](DESIGN.md#provider-and-tool-contracts)
already define bounded owned operations and exactly one execution/tool-loop
owner. Extend that boundary rather than creating a competing operation registry.
The following dispositions concern this proposed scope; none retires an owner
merely because a replacement can return similar text.

| Dependency and current evidence | Owner to preserve | Proposed disposition |
| --- | --- | --- |
| [Lifecycle design](DESIGN.md), [main plan](IMPLEMENTATION_PLAN.md): context transition, admission, custody, recovery and qualification have an existing owner; S10 remains separately active. | Lifecycle authority and its implementation owner. | **Retain.** Direct retained execution eventually supplies different observations to the same owner. First replay has no checkpoint/publication/activation authority. |
| [Continuation functions](../../rust/crates/lifecycle-semantic/src/continuation.rs): `compile_request` and `verify_candidate` construct bounded semantic requests; bindings currently include `NativeInvocationPurpose`, session and thread fields. | Lifecycle semantic contract. | **Retain** source, obligation and result meanings; **adapt** execution attribution only after the owner settles native/direct identity compatibility. No second continuation schema. |
| [Assessment](../../rust/crates/lifecycle-semantic/src/assessment.rs): `VerifierReport` is model analysis; `assess_candidate` derives checked status using exact bindings and observed exposure. | Host semantic assessment, separate from model judgment. | **Retain.** A direct report has no authority to accept itself. Generalize only the attribution seam needed by the actual verifier consumer. |
| [Semantic service](../../rust/crates/lifecycle-service/src/semantic.rs): `establish_checkpoint` composes source projection, compilation, separate verification, host assessment and dependency revalidation before publication. | Lifecycle service composition and canonical material/currentness owners. | **Retain** production ordering and owners; **adapt** the selected control-inference dependency later. The replay driver exercises the consumer without entering this publication path. |
| [Control turn](../../rust/crates/lifecycle-service/src/semantic_control.rs): `run_control_turn` launches Codex, creates a thread without dynamic tools, binds entry, and obtains terminal/history evidence. | Native adapter owns native observations; lifecycle owns admission and settlement. | **Retain** as the Codex implementation; **replace the mandatory native dependency for an admitted direct profile**, not its historical receipt meaning. A direct stream does not fabricate native readback. |
| Instruction environment: the native control path sends the operation prompt through Codex; the direct path will not inherit the same harness instruction environment. | Semantic owner defines the question and required output; runtime instruction projection preserves applicable doctrine and authority. | **Adapt explicitly.** Bind the direct instruction/input revision, project only the operation's relevant contracts, and qualify changed behavior. Copying a hidden harness prompt or creating one universal agent prompt is not a prerequisite. |
| [TextTurnPort](../../rust/crates/lifecycle-provider/src/lib.rs): text request/observation, profile, execution outcome and settlement; observation requires provider thread/turn IDs. | Existing provider boundary. | **Adapt or compose** a smaller request/observation seam. This interface is not already a general tool-exchange or direct-context contract. Avoid adding every harness feature to one trait. |
| [Rust domain tools](../../rust/crates/lifecycle-service/src/operations/mod.rs): selected profiles expose `workspace.snapshot_read`, and task-profile `question_open`/`result_publish`, through native execution. | Snapshot/task owners define tool meaning and authority. | **Retain** their contracts; later **adapt** transport and dispatch to a direct loop. The first profile exposes no tools. Rebuilding code mode is not required by these function contracts. |
| Native ingress, effect settlement, cumulative usage and successor-thread coupling described in the [existing amendment](POST_S10_BACKEND_ABSTRACTION_AMENDMENT.md#existing-design-to-preserve). | Lifecycle store and context identity owners. | **Retain** historical evidence; later **adapt** request-level accounting and non-native context identity. Rebind these source observations before implementation; this plan does not certify the current auth candidate. |
| [Rust migration plan](../../docs/rust-migration-plan.md), [deterministic compiler plan](../../docs/rust-compiler-implementation-plan.md): common mechanics and deterministic skill/manifest compilation have separate consumers and histories. | Their respective domain and workspace owners. | **Retain.** Do not move deterministic compilation into inference, or introduce a shared database/process manager merely for this adapter. |
| Older Node harness capabilities and port precursors are described in the [ports proposal](../../app-server/ideas/pending/provider-turn-harness-runtime-and-operator-projection.md#9-current-app-server-precursors). | Existing harness/domain owners. | **Defer replacement.** Their current production use and parity obligations were not audited here. **Retire only** obsolete bindings after consumer qualification and explicit saved-state disposition; no blanket Node retirement follows. |
| Direct SIWC authorization, streaming transport and request observations are new for this selected profile. | Adapter owns authentication/transport; operation owner owns permission and result meaning. | **Add narrowly.** Native Codex credentials are not presumed to be a direct SIWC grant. Future Claude CLI is a separately qualified harness implementation, not a required first-slice dependency. |

## Why bounded operations fit the wider design

[Organizational compilation](../../app-server/docs/architecture/organizational-compilation.md)
derives execution requirements from accepted meaning, with event-scoped judgment
for the semantic remainder. Context, vantage and role boundaries differ. That
supports a small inference surface when the actual question is bounded; it does
not prove every organizational operation can be inference-free or one-shot.

[Plan IR](../../app-server/ideas/pending/structural-plan-ir-for-capability-aware-multi-model-execution.md)
proposes preserving semantic structure so executors need not reconstruct it.
Its delegated implementation judgment remains meaningful. The
[implementation compiler](../../app-server/docs/architecture/implementation-contract-compilation.md)
is initially a model role despite the phrase “deterministic derivation.” Neither
phrase authorizes replacing unresolved semantic judgment with a hard-coded rule.

The [service-plane idea](../../app-server/ideas/pending/service-plane-and-kernel-domain-boundary.md)
distinguishes deterministic, semantic and hybrid operations. A mechanically
knowable fact with no admitted owner is an ownership/implementation gap, not
automatically a reason to invoke a model. The
[refinement idea](../../app-server/ideas/pending/resolution-layered-semantic-refinement.md)
similarly separates mechanical correspondence, semantic judgment and admission.
These proposals inform the boundary without becoming prerequisites or accepted
global schemas.

## Minimal conceptual contracts

Use the dimensions already owned by the lifecycle design: operation/contract/
implementation identity, semantic owner and executor, typed input and authority,
outcome/evidence, recovery and idempotency. Add a selected realization binding
only where needed to attribute semantic execution. These are semantic dimensions,
not a mandated Rust field list or a new service.

An operation invocation binds its exact input and source revisions, authority,
selected capability profile, implementation/model/configuration identity and
required result evidence. Keep the logical operation, each provider request
attempt, each tool effect, local execution/context identity and remote identifiers
distinct. One logical operation may contain several requests without becoming
several domain deliveries. Credential refresh is neither a context replacement
nor a new attempt at the operation.

The [ports proposal](../../app-server/ideas/pending/provider-turn-harness-runtime-and-operator-projection.md)
provides useful separation:

| Boundary | Minimum responsibility in this scope | Excluded authority |
| --- | --- | --- |
| Provider turn | Prepare/send one bound request; attribute text/tool/usage/error/terminal observations; identify uncertainty and unsupported behavior. | Domain acceptance, context replacement, funding permission, tool execution by implication. |
| Harness/runtime | Own the selected loop, context assembly and any tools; expose interruption, continuity and recovery evidence actually supported. A one-shot driver can be very small. | Redefining operation meaning, granting tools, or settling remote effects from process exit. |
| Operator projection | Present operation state, evidence and permitted controls through existing owners. | Becoming the canonical execution/session identity or directly changing an admitted adapter. |

Capabilities describe **what can be established**, including bounded text,
tool mediation, context/history availability, continuity, exposure isolation,
cancellation observations, completion, usage and funding evidence. Bind their
scope and tested revision. “Unknown,” “unsupported,” and “observed” are different
states. Do not normalize away material native/direct differences.

Three useful capability shapes are direct one-shot inference, a Work Engine
controlled tool loop, and a sustained external harness. They are not mandatory
classes assigned to semantic task names. Selection can consider context value,
tool needs, evidence strength, quality, latency and quota inside authorized
constraints. An already admitted Codex route remains useful where its capabilities
fit. No new adaptive router is needed to select an explicit first profile.

The matrix below separates current dependencies from proposed capability work.
It is not a claim of behavioral equivalence or a future adapter qualification.

| Required capability | Current Codex integration | Proposed direct implementation | Future Claude CLI adapter |
| --- | --- | --- | --- |
| Bounded semantic text | Existing native control-turn path; its accepted scope remains with the lifecycle owner. | B1/B2 verifier profile, then compilation if qualified. | Unqualified; evaluate against the same consumer contract. |
| History and continuity | Native thread/history evidence used by the current path. | Exact request evidence first; local history and continuation mapping only for later retained work. | Preserve and qualify actual native semantics; do not presume Codex-compatible sessions. |
| Tool loop and code mode | Native loop with selected Work Engine tools; code mode belongs to the task profile. | B4 controlled function-call loop; JavaScript execution only if a consumer justifies it. | Inventory actual tool mediation and pre-entry enforcement before admission. |
| Context replacement | Existing lifecycle owner composes native observations and successor execution. | Later context-generation, projection and successor-use integration with that same owner. | Qualify transition and recovery evidence against lifecycle requirements. |
| Shell, MCP, web, images, skills and delegation | Broader harness needs remain outside the narrow first profile; older Node production obligations need an inventory. | Retain external harnesses or add individual capabilities when their consumers require them. | Discover and qualify each required capability; the product name is not evidence. |
| Completion, accounting and funding | Native evidence contracts plus separately owned session-auth repair/qualification. | Attributed stream/attempt evidence, usage accounting and independently qualified SIWC grant. | Separate provider/account/funding qualification; no inference from OpenAI entitlement. |

## First vertical slice: candidate verification

The real consumer is `verify_candidate` → verifier report → `assess_candidate`,
currently composed by `establish_checkpoint`. Recommend this before direct
compilation because it exercises a consequential semantic judgment with exact
source/candidate inputs and an existing host-derived result boundary, while
avoiding tool execution and retained-context replacement. Compilation is the
next nearby consumer, not a speculative new service. Existing Codex output is
comparison evidence, never the semantic oracle merely because it came first.

The slice takes a frozen, authorized source basis, a continuation candidate and
policy; produces the existing bounded verifier prompt; submits a tool-free
direct request; retains attributed observations; and passes a completed result
through the host assessment contract. Output is an isolated evaluation artifact
marked non-authoritative. It cannot publish a checkpoint, resume a subject,
resolve human interaction or count as an accepted S10 transition.
Use a separately bound evaluation corpus and disposable state. S10's frozen
trial state and held-out material remain under their existing custody and
exposure contracts; this comparison does not implicitly acquire them.

There is a concrete compatibility issue before implementation: current semantic
bindings require native purpose/session/thread fields. The proposed solution is
a small explicit native/direct execution-identity distinction at the semantic
owner's boundary, with native receipt meaning preserved. Direct context identity
means locally assembled exposure; provider response identity remains separate.
Do not put invented provider-thread handles in native fields to make validators
pass. Whether that distinction is a versioned binding type or a composed adapter
is an implementation choice after the owner accepts the semantic mapping.

Verifier exposure needs host evidence of the exact request, selected model,
enabled capabilities and separation from compiler reasoning/hidden oracles.
A different ID or a request hash alone does not establish that history. Preserve
compiler/verifier separation even when they share transport code. If direct
evidence cannot establish a required exposure claim, return it as unestablished;
do not weaken the claim or call the model's report an accepted assessment.

## Proposed implementation units and dependencies

The units are a default decomposition with exit consequences, not a required
ritual. They can be combined or reordered where their causal dependencies permit.
Implementation begins under a bounded later grant; this planning grant covers
the present document only.

| Unit | Proposed work and dependency | Exit consequence |
| --- | --- | --- |
| B0: close the consumer boundary | Resolve the decisions needed by the first profile against a frozen source baseline, current S10 ownership and the intended account/deployment. Specify native/direct identity and evidence compatibility; leave later retained-work decisions with their affected units. | A bounded first implementation contract names its exact consumer, files, state, capabilities, funding and test consequences. Unresolved decisions are not silently delegated by omission. |
| B1: offline request and replay foundation | After implementation authority and B0: Rust request assembly, redacted durable attempt observations, stream decoder, fake peer/fault cases and verifier replay integration. No real credentials or provider call required. | The real semantic consumer is exercised through the selected seam; incomplete streams and invalid/exposure-unestablished results cannot become success. Native fixture meaning remains intact. |
| B2: direct verifier vertical proof | B1 plus explicit authority for the exact session-quota qualification and established account/funding prerequisites. Implement/use independent SIWC auth; run a bounded source/candidate corpus through the direct verifier in disposable evaluation state. | Actual direct inference, truthful terminal/usage/funding evidence and semantic results exist for the selected profile. Report accepted, rejected and unresolved host outcomes and quality failures. No production lifecycle acceptance follows. |
| B3: compiler and admitted control-inference use | B2 evidence plus lifecycle owner's integration decision, current S10 obligations and compatible schema/state handling. Add direct compilation with separate exposure; compose existing host assessment/currentness/publication owners. Retain Codex as an explicit alternative implementation. | The selected production control-inference profile can establish its required claims through real composition. Candidate success alone still does not establish successor adequacy or full retained-work acceptance. |
| B4: bounded direct tool loop | Only when an accepted consumer requires it; depends on per-operation admission, request/tool accounting and the relevant S10-owned contracts. Start with the actual snapshot/task profile. | Complete tool calls enter once under current grants; attributed results feed the next request; restart and uncertainty do not duplicate effects. Required per-tool gates are enforced before entry. |
| B5: retained direct execution and selective retirement | B3/B4 as needed plus reconciliation with the post-S10 amendment, lifecycle identity/currentness/recovery integration and separately scoped live qualification. | Retained work, replacement, successor use and restart satisfy the selected lifecycle claims. Only superseded bindings/capabilities with completed consumer and saved-state dispositions become retirement candidates. |

**Parallel-safe lane:** document/fixture specification and, once authorized,
isolated transport, authentication and replay implementation can proceed without
changing S10 runtime files, frozen evidence, auth-repair candidates or live state.
The direct SIWC route does not depend on first migrating authentication through
Codex. Shared Cargo manifests, dependency locks and semantic type changes still
need coordination with their current owner; “isolated” does not confer ownership.

**S10-dependent lane:** changes to shared lifecycle bindings, effect accounting,
canonical material projection, production semantic control, successor activation
and retained-task tools depend on that owner's accepted boundary and outstanding
obligations. S10 is not a prerequisite for all direct transport research, and
this proposal is not a new S10 gate. A shadow pass cannot discharge live claims.

The existing amendment starts with a retained workflow and full selected tool
profile. This plan proposes a smaller first operation to expose authentication,
protocol, attribution and semantic-evidence problems earlier. Its later retained
work can consume the amendment's accepted content if the owner accepts it; this
document neither silently supersedes that proposal nor implies it was accepted.

## Rust placement and extraction

Follow the [Rust direction](../../docs/rust-development.md). Keep semantic request
construction and host assessment with `lifecycle-semantic`; lifecycle integration
stays with `lifecycle-service`. A small provider-request module/package can own
direct SIWC/Responses mechanics, without depending on lifecycle authority or
tool-domain payloads. Final package naming and public types remain a B0 decision.

Start from the actual verifier consumer and the existing provider seam. Extract
a reusable provider library when compilation, a review consumer or another
domain demonstrates the shared semantics; do not require a daemon, generic store,
huge trait or permanent Node bridge. Third-party Codex/Claude executables remain
valid capabilities behind Rust adapters. An offline coordinated replacement is
permitted; dual running is not required. Saved-state compatibility and authorized
archive/clean-start choices remain explicit even without current users.

## Completion, recovery and accounting

Persist enough attempt identity and send intent to recover truthfully across
the local/remote boundary. There is no atomic transaction spanning local storage
and the provider. Distinguish prepared, possibly entered, observed terminal and
unknown outcomes; a crash between send and acknowledgement cannot become “never
sent.” A retry is a new attributed attempt under the operation's recovery
contract, not replay of an observation. Unknown remote completion or quota use
remains visible even for a tool-free request.

Only a qualifying terminal response can support provider completion. EOF,
transport success, emitted text, a tool-call fragment, deadline expiry, local
cancellation and process exit are not substitutes. Provider completion, valid
structured output, host semantic acceptance and tool-effect settlement are
separate facts. For later tool loops, persist complete call identity, authorized
entry and result custody; reconcile uncertain effects before any retry whose
safety depends on them. One executor owns each loop and effect.

Aggregate usage once per distinct provider request/attempt observation, then
across the logical operation. Deduplicate replayed events; distinguish increments
from cumulative counters. Missing final, cached or reasoning counts remain
unavailable, including after failures. A second failed attempt may consume quota.
Token use is not current context occupancy, subscription quota balance or proof
of funding. Record configured limits, measured values and estimates separately.

Keep request-context history distinct from semantic continuation. Tool messages
and supported opaque provider items can be needed to form a later request; they
do not become portable reasoning evidence or accepted continuation by retention.
Full transcript persistence is unnecessary for the first one-shot profile beyond
its exact input/output and required evidence. Retained execution later needs
durable request assembly and the existing lifecycle's preservation guarantees.

## API, authentication and funding qualification

Official documentation rechecked by the coordinating session on 2026-10-21
describes OAuth-authenticated Responses and account-specific model discovery.
HTTP uses `store: false`, `stream: true` and supplied context; persistent
conversations and HTTP `previous_response_id` are unavailable. Use supported
instructions/developer inputs, not system message items. Namespaced tools or
`additional_tools` are relevant only to later profiles. Provider completion
requires `response.completed`; a quota failure can occur after streaming starts.
Parameter/model support and opaque-item replay need qualification against the
chosen account and revision. See [models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference),
[preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations)
and [errors](https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery).

The direct route needs its own supported SIWC registration and stable host,
verified account/session binding, PKCE and state/nonce/ID-token validation,
protected credential storage, serialized refresh and atomic credential-set
replacement. Credentials stay outside model inputs and evidence payloads;
receipts use non-secret binding references. Refresh failure or revoked access
blocks affected new requests without changing provider or billing route.
Codex login is not presumed transferable authorization. See
[sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in),
[profiles and sessions](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions)
and [deployment scope](https://developers.openai.com/siwc/token-sharing-open-source).

Session-funded inference and strictly plan-quota-funded inference are distinct
claims: official account documentation also mentions optional credits. Establish
what account controls and observations can support the user's quota requirement
before a live proof. An OAuth token, request count or absent API key alone cannot
prove credits were excluded. If strict quota-only enforcement/evidence is not
available, surface that exact unmet condition to the funding owner; do not silently
consume credits, switch to API billing or claim quota proof from token telemetry.

## Validation and evidence consequences

| Boundary | Required evidence for the proposed claim |
| --- | --- |
| Protocol and completion | Fragmented/interleaved events, duplicate observations, malformed data, output bounds, missing terminal, failed/incomplete terminal, after-start quota refusal and disconnects preserve truthful outcomes. Complete tool arguments are not executed from partial deltas. |
| Attempts and recovery | Failures before send, after possible entry, after provider completion and before persistence retain exact uncertainty. Replaying captured observations does not send requests. Later tool cases include crash after effect but before result custody. |
| Semantic consumer | Source/candidate/prompt/result bindings and citations validate; corrupted, incomplete, authority-widening and supersession-reviving candidates are challenged. Mechanical shape does not substitute for real-model semantic adequacy. Hidden evaluation answers stay outside verifier exposure. |
| Exposure and authority | Realized tools, model/configuration and input custody are evidenced by the host at the claimed boundary; compiler reasoning is not supplied to the verifier. Unsupported evidence leaves the dependent claim unestablished. Shadow output never enters production acceptance. |
| Usage and funding | Multi-request totals, repeated events, unavailable counters, partial failure and retries retain their actual meanings. Qualification distinguishes documented entitlement, selected session route, observed usage and evidence excluding disallowed funding. |
| Native compatibility | Existing native fixtures and receipt identities preserve meaning; direct evidence cannot pass as native thread/history evidence. Any wire/store migration has named compatibility and recovery cases. |
| Later retained profile | Live continuation, actual successor use, pending interaction, completed-effect preservation, stale-dependency fencing, queue order and restart are established under the owning lifecycle acceptance; scripted success and the B2 replay are insufficient. |

Compare selected profiles by semantic adequacy, unresolved/repair rate, accepted
outcomes, actual token/quota observations, latency and maintenance cost. Keep
model, instructions, tools and input revisions attributable. No savings claim or
new default model follows from this plan, and a changed profile cannot inherit
historical qualification by renaming its results.

## Decisions to close before the affected implementation

1. **First consumer and authority:** accept or revise verifier replay as the
   first unit; name its corpus/state owner and the exact later implementation
   scope. Live qualification needs a concrete bounded grant, not this proposal.
2. **Identity and exposure:** semantic/lifecycle owners settle native/direct
   binding representation, historical compatibility and the evidence required
   for isolation/model identity. Preserve the current host acceptance meaning.
3. **Funding and eligibility:** determine registration, account/deployment scope,
   requested model availability and strict plan-quota controls/evidence. No
   direct-provider grant is inferred from existing Codex access.
4. **Placement and persistence:** choose the smallest actual consumer boundary,
   package ownership, attempt journal/evidence custody and redaction/retention
   requirements. A generic service registry remains outside scope.
5. **Protocol profile and bounds:** bind supported reasoning/output controls,
   byte/deadline/request limits, model identity observations, retry semantics and
   unacceptable unknowns. Local bounds do not imply remote settlement or cost caps.
6. **Integration and retirement:** before B3–B5, reconcile current S10/auth work,
   amendment disposition, required live claims, schema migration and saved-state
   handling. Decide the required later tool/harness profile from its consumer.

## Source status and evidence limits

The cited organizational-compilation and implementation-contract-compilation
pages describe proposed architecture with local maturity exceptions; Plan IR,
service-plane, runtime ports and resolution-layered refinement remain proposed
or exploratory. [Runtime realization](../../app-server/docs/architecture/runtime-realization.md)
has partial precursors and local accepted boundaries, not a fully implemented
resolver. [Execution strategy selection](../../app-server/docs/architecture/execution-strategy-selection.md)
is accepted design only. [Context lifecycle](../../app-server/docs/architecture/context-lifecycle.md)
and the Rust plan provide related temporal ownership; neither establishes the
proposed organizational machinery merely by appearing in architecture docs.

Repository evidence is Tier 2, task-directed verification on 2026-10-21, project
`home-bline-code-work-engine`, graph store generation `u97205c3b8e628138g2`, indexed
at `2026-10-21T15:47:07Z`. Relevant symbol searches were fully paginated;
`verify_candidate` inbound/outbound depth-one results were consumed, and the
coordinator supplied the complete `establish_checkpoint` depth-one/source check.
Exact source confirmed the native identity constraint, verifier host boundary,
control-turn composition and selected tools. Coverage checked all 24 relied-on
repository paths with `content_verified`, no recorded gaps and no remaining page.
This is best-effort evidence, not an exhaustive dependency or runtime audit.

Observed Git HEAD was `65c00f00e5d614becfa8649ce3d0651814a5cf34`; concurrent source
and uncommitted work mean HEAD alone does not bind every inspected byte. Rebind
the relevant sources at implementation admission. The amendment's broader
store/core inventory is attributed secondary evidence here, and older Node
production use remains unresolved. No inference, auth operation, implementation
test or S10 acceptance was performed for this plan. Official protocol premises
are time-sensitive documentation evidence, not account-specific qualification.
The requested planning configuration was Astra/xhigh; effective runtime identity
was not independently exposed to this author.

The coordinating session integrated instruction-environment ownership and the
capability matrix after collecting the planning draft. It re-read native control
request construction and checked both control/domain source paths at graph
generation `u91e30c6eda77e7a6g2` (`2026-10-21T16:32:08Z`), with content verification
and no recorded coverage gaps. This supplements the planning evidence above; it
does not upgrade the broader inventory into an exhaustive feature audit.
