# Same-session context continuity and human interaction

This candidate separates context transformation from root ownership transfer
and defines the interaction boundary needed for automatic same-session
resumption. The existing lifecycle service remains the proposed semantic
owner; this is an amendment to that contract, not a second lifecycle system.
The manifest in [packet.json](packet.json) owns candidate metadata and authority.

## Why this delta

The observed S10 root composed an existing compaction API, durable continuation
and background wakeup. It announced the refresh and resumed the same session.
Its wakeup identified its runtime origin in prose, but entered history as a
user message. Input checks were separate from actuation and delivery, and a
declared one-attempt limit was not enforced across controller restart.
[Formation evidence](formation-evidence.md) separates those observations from
the judgment that the route is worth supporting.

The [semantic lifecycle design](../../../app-server/docs/semantic-context-lifecycle-manager.md)
already protects durable meaning, authority, input fencing and reconciliation.
The [transition protocol](../../../app-server/docs/context-lifecycle-transition-protocol.md)
already owns durable transition progress and effect recovery. The missing
clarification is how an active human-supervised root may use those boundaries
without treating either an unchanged session ID or Wind Walker as permission.

The neighboring
[semantic-durability proposal](../semantic-durability-context-lifetime/proposal.md#human-facing-interaction-boundary)
explicitly leaves automatic replacement in human interaction to a separate
contract. This candidate addresses that boundary; it does not accept or revise
the neighboring proposal.

## Proposed contract insertion

The following text is proposed for the existing semantic lifecycle contract,
after its Ownership boundary. The transition protocol supplies admission,
custody, lease, effect and recovery contracts. A realization must bind its
chosen compaction mechanism to those contracts, including attribution, the
irreversible actuation boundary, interruption and abort behavior. Reuse is an
implementation obligation, not an established mapping from a compaction API to
the protocol's existing `new_context` control-turn identities.
Acceptance does not require the current app-server paths, a Python controller,
a particular provider API, or a new universal receipt schema.

### Context change, ownership and authority

Context transformation and root ownership transfer are independent operations.
Same-session compaction does not itself transfer a role, establish fresh
independent reasoning, or eliminate the need for durable continuation meaning.
Root transfer retains its owning authorization and adoption boundary.

An active assignment's existing authority may cover ordinary same-session
lifecycle administration and continuation within that assignment where no
owning contract reserves the operation. A supporting host contract must also
establish the ordering, custody and recovery claims below. This does not require
a new affirmative grant for every operation or new approval solely because
context changes. A reserved human
decision, explicit route restriction, pause or cancellation remains binding.
Access to a socket, a safe checkpoint, Wind Walker compliance, and unchanged
thread identity do not independently grant actuation or resumption authority.

Because a checkpoint records authority rather than issuing it, actuation and
productive resumption must each be covered by current governing authority. A
change of authority invalidates stale admission. Missing or conflicting
authority leaves the dependent operation unestablished. Rehydration permitted
for recovery cannot reactivate the paused assignment or resolve a pending
human decision.

A role may judge the value of, or request, a transition within its authority.
The host retains readiness and admission; a model may actuate only within that
host-owned transition operation. A lifecycle effect dispatched outside that
operation leaves this contract's ordering, custody and no-duplicate claims
unestablished, even if the role's task authority otherwise covers its objective.

### Human input and runtime authorship

Separate input checks and dispatch can miss a human restriction just before an
irreversible operation. The host must establish an authoritative ordering
between received human input, context actuation and productive resumption.
A realization binds the human-usable input paths covered by this claim. An
accessible path whose received input cannot participate in that ordering leaves
the claim unestablished; a mediated-path test cannot establish broader coverage.
Before actuation, competing input
invalidates readiness as required by the existing transition lease. After
actuation begins, newly received input remains durably attributable outside
the retiring revision and participates in successor reconciliation. A newer
pause, cancellation or scope change must govern admission before any affected
domain work resumes. Safe custody must not become indefinite invisibility;
receipt, pending disposition and the responsible owner remain observable.

Revoking readiness is not sufficient when new input restricts the lifecycle
operation itself. The owning human/role authority determines its effect on the
pending operation. Input ordered before actuation that restricts, or may
restrict, that operation leaves re-actuation unestablished until current
authority resolves its scope. Merely compiling it as an open interaction does
not authorize another attempt. Ambiguous scope likewise leaves the affected
actuation or productive admission unestablished rather than narrowing the pause.

Automatic continuation has its own stable admission identity and a position in
the same authoritative order. Its productive admission must respect every human
input ordered before it. Queued human domain input remains in durable custody
until accepted reconciliation and ordinary ordered release, as the transition
protocol requires. This amendment adds an ordering obligation for the wakeup:
current authority effects needed for reconciliation/admission must be available
through their owning control projections, without prematurely delivering queued
domain input. A realization unable to supply that evidence remains unresolved.

A runtime wakeup retains its actual author, delivering actor and authority reference
through delivery, history, compaction and recovery. Its own text cannot
authenticate that origin or create human approval. If a provider transport uses
a user-message field, the host must preserve the actual origin through an
attributable envelope and a model-visible projection that does not promote it
to human authority. A transport unable to preserve that distinction cannot
establish this automatic-resumption contract.

Attribution establishes origin, not model compliance. Reconciliation evidence
and domain admission must establish that wakeup content did not close a human
decision or expand authority; the envelope alone cannot establish that outcome.

### Recovery and effect custody

For each transitioning logical role instance, the host owns the durable
transition operation and lifecycle effects it dispatches. Its existing lease
and effect records must bind the exact role,
runtime, predecessor, checkpoint and authorized continuation. Replaying an
event or restarting a process must not create a second compaction or wakeup
for an already committed effect. An effect with uncertain provider entry
requires reconciliation against retained evidence before another attempt;
neither a missing completion record nor an expired process proves non-entry.

Compaction completion and semantic recovery are separate claims. Before
dependent domain work, recovery must establish the applicable role binding,
exact checkpoint, current instructions and authority, unresolved interactions,
collection obligations and valid next scope. Missing, contradicted or
uncertain evidence remains visible under the existing recovery contract.
Opaque provider state may help continuation but cannot be the sole durable
owner of meaning whose loss would invalidate it.

### Operator visibility

The operator must be able to distinguish an intended transition, an observed
context event, accepted recovery, cancellation, failure and unresolved outcome.
The existing lifecycle record and user-facing projection must identify the
initiator, authority source, checkpoint, session and context lineage where
observable, realized mechanism, resumption disposition and remaining owner.
Unknown provider semantics stay unknown. Lifecycle work that ends the visible
turn also needs an owned, evidenced resumption or notification route; a result
file alone is not delivery to the user.

Because the resumption route itself can fail, failed or unresolved resumption
must reach the operator through an owned, evidenced route, or the announcement
must identify where its terminal outcome is observable and explain that absence
means unresolved rather than progress. Operator visibility applies to denied,
interrupted and crash outcomes as well as successful continuation.

Visibility need not imply a new approval step or a fixed channel. A supported
operation can announce its intent and outcome in the current thread. The host
must not describe it as a root transfer when ownership remains unchanged, or
as a verified fresh context merely because a compaction item appeared.

## Ownership and loading

Domain and role owners retain objectives, claims, decisions and commitments.
The lifecycle host owns ordering, transition custody, attribution and recovery
evidence; it consumes shared fencing rather than redefining that mechanism.
Provider adapters describe and evidence the transformations they can realize.
Wind Walker retains its general continuation-independence contract.

Here, ordering means admission relative to lifecycle actuation and resumption.
The existing
[intent-projection mechanism](../../../design/architecture/mechanisms/authority-preserving-intent-projection.md)
describes bounded submission and operator feedback; the
[writable-ingress candidate](../../../design/ideas/pending/context-replacement-emergency-and-writable-operator-ingress.md#candidate-b-writable-atomic-operator-ingress)
describes adjacent transport gaps. This contract consumes those distinctions
without taking ownership of general human intent or feedback semantics. Their
proposed material does not create authority or a new acceptance gate here.

Manual coordinators receive the compact distinction in
[context-boundary.md](../../../skills/manual-orchestration/references/context-boundary.md#same-session-replacement-and-root-transfer).
That clarification restates existing authority and recovery boundaries; it
does not activate this candidate host contract or authorize an improvised
controller. The detailed host contract belongs at host design and admission
boundaries, not in every worker prompt. The proposed host requirements expose
their causal reasons above when they govern an implementing agent.

The present normative review subjects are the proposed contract insertion and
the changed context-boundary paragraphs. They govern lifecycle authority,
input authorship, effect custody and loading reach. Pilot sample size and case
ordering are bounded research choices, not production doctrine.

## Decision and evidence still needed

The user or authorized architecture owner decides the interaction contract.
A later implementation owner must bind a supported host/provider realization;
this packet does not endorse the incident controller, release instructions,
run a live transition or authorize a pilot. The existing owner-held lifecycle
documents are left unchanged for review of this additive delta.

[Pilot criteria](pilot.md) test whether the proposed distinctions survive
normal operation, concurrent human input and controller failure, and whether
the resulting route is useful compared with a successor-root handoff. A
favorable result supports bounded adoption, not universal compaction triggers.
