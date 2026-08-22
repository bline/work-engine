# Work Engine Studio: Design, Control, and Forensics

## Status

Future implementation possibility / product direction

Implementation is intentionally deferred. On 2026-08-21, a possible
`slice-supervisor` implementation campaign was considered but stopped before a
campaign or builder was launched. This document is the durable owner of that
design context until a human deliberately resumes it.

The current role scheduler is useful implementation evidence for a bounded
coordination plane, not proof that the full Studio control plane exists. In
particular, activation leases, revision-bound subscriptions, claims and
recovery, delivery-health semantics, and authority validation remain future
capabilities unless their executable owners establish them.

## Idea

Work Engine's increasingly structured role, invariant, capability, workflow, and state models create the possibility of a unified visual environment for designing, operating, and investigating agent systems.

The key observation is that the same underlying structural truth can support three product surfaces:

```text
DESIGN
→ what may exist and how it is structured

CONTROL
→ what is happening now and which authorized actions are available

FORENSICS
→ what happened, why it happened, and how the system evolved
```

These should not become three independently maintained interpretations of Work Engine. They should be projections of the same underlying model.

---

## Why this becomes possible

Work Engine is no longer represented only by prose instructions.

The system increasingly has structured representations for:

- roles;
- invariants;
- capabilities;
- observable state;
- mutable state;
- mediated transitions;
- explicit non-authority;
- workflow ownership;
- cross-workflow observation;
- persistent state;
- strategic planning;
- review independence;
- checkpoints;
- receipts;
- scheduling and delivery;
- logical agent identity;
- decision traces;
- transition history.

Because these concepts are explicit and typed, a user interface can manipulate and render them without inventing a second architecture.

The UI can become a view and editor over structural truth.

---

## Core product concept

A future **Work Engine Studio** could let a designer construct an AI role or workflow visually.

The designer may:

- create a role;
- define its objective;
- bind invariant structure;
- assign capabilities;
- expose observable state;
- grant bounded mutation affordances;
- configure mediated transitions;
- define outputs and consumed artifacts;
- attach other roles;
- define cross-workflow state projections;
- establish independence boundaries;
- define context lifetime;
- expose explicit non-authority.

Conceptually:

```text
                  ROLE ENVIRONMENT

             ┌── pinned invariants
             │
             ├── observation affordances
             │
ROLE ────────┼── mutation affordances
             │
             ├── mediated affordances
             │
             ├── owned state / artifacts
             │
             ├── consumed evidence
             │
             └── explicit non-authority
```

The generated environment should remain structurally equivalent to the machine-readable role/environment definition.

---

## Design mode

Design mode answers:

> **What world are we constructing for this agent?**

A role designer may visually pull capabilities into an environment and connect them to roles.

Examples:

```text
repository evidence
       │
       └── MAY_INVOKE ──► Slice Builder

proposal authority projection
       │
       └── MAY_OBSERVE ─► Slice Supervisor

repository mutation
       │
       └── MAY_MUTATE ──► task-owned paths

publication
       │
       └── MEDIATED_BY ─► completion adapter
```

The interface should make the structural distinction visible:

- invariant structure pins the valid boundary;
- machinery enriches the valid interior;
- observation affordances change what a role can know;
- mutation affordances change state within granted authority;
- mediated affordances ask another owner to consider or perform a transition;
- explicit prohibitions define real non-authority.

The designer should not encourage authors to encode routes as commands merely because a drag-and-drop connection is easy to create.

---

## Design-time analysis

The visual builder should continuously analyze the environment being constructed.

Possible diagnostics include:

### Obligation coverage
This role is bound by an invariant but has no capability or observable evidence path capable of establishing it.

### Observation gap
This role is expected to make a judgment about state it cannot observe.

### Authority leak
This role can directly mutate state owned by another workflow.

### Ownership gap
An artifact is emitted but no role or workflow owns its lifecycle.

### Redundant affordance
Two capabilities expose substantially equivalent transitions without a meaningful distinction.

### Orphan agency
A capability expands the role's action space but serves no declared objective, invariant, or downstream consequence.

### Procedural drift
A current route or tool preference has been promoted into pinned structure without a route-invariant failure condition.

### Independence violation
A reviewer can observe builder reasoning or decision history despite claiming fresh independent evidence.

### Unreachable consequence
The role has a required outcome but no composition of available affordances appears capable of reaching it.

These are candidate design findings. Semantic classification and genuine contract changes remain model-judged and human-authorized.

---

## Generated role pages

Each role can have a generated page derived from the same structured environment.

A role page may show:

```text
Role objective
Context lifetime

At a glance
- pinned invariants
- capabilities
- observable inputs
- mutation surfaces
- owned state
- emitted outputs
- mediated transitions
- prohibitions

Environment graph

Required consequences from other roles
Observation limits
Mutation boundaries
Independence boundaries
Enforcement tiers
Design findings
```

The rendered page should be a deterministic projection, not another editable source of truth.

---

## Control mode

Once the Work Engine control plane and durable workflow state are attached, the same designer can become an operational interface.

Control mode answers:

> **What is happening in this environment right now?**

The static role graph can be overlaid with runtime information such as:

```text
Slice Supervisor
    active: true
    campaign: campaign-42
    current_slice: 7
    state_revision: 118

Slice Builder
    active: true
    phase: review_remediation
    open_findings: 1
    gate_state: passed

Independent Reviewer
    active: true
    status: waiting_on_repair

Strategic Planner
    active: false
```

The same nodes that describe design-time affordances can expose runtime status.

A capability may show `available`, `unavailable`, `waiting`, `blocked`, `quota-limited`, or `healthy`.

A state projection may show current revision, owner, freshness, and last transition.

A mediated transition may show `available`, `pending authority`, `requested`, `completed`, or `blocked`.

---

## Authority-aware controls

The control interface should derive available actions from the same environment graph.

The UI must not present a generic admin panel full of actions disconnected from role authority.

If the environment says:

```text
Slice Supervisor
    MAY_OBSERVE proposal.authority_projection
    MAY_REQUEST strategic_reconciliation
    MAY_NOT_MUTATE proposal.state
```

then the interface can expose:

```text
Inspect proposal authority
Request strategic reconciliation
```

but not:

```text
Approve proposal
Rewrite proposal state
```

This makes the control surface authority-aware by construction.

The UI does not decide what is authorized. It projects the existing authority model.

## The human control surface

A human standing behind Work Engine Studio should be able to exercise complete
control over the environments for which that human has authority. This remains
compatible with the control-plane boundary because **complete control is not
unbounded authority**.

The interface should let the human:

- see every active role, workflow, scheduled obligation, waiting condition,
  pending proposal, review finding, gate, and durable receipt exposed to that
  environment;
- inspect the authoritative source, revision, freshness, and limitations of
  each projection;
- activate, pause, resume, redirect, defer, approve, decline, cancel, or revoke
  consequences when an authoritative domain owner exposes that transition;
- grant narrowly scoped approval, including time-bounded auto-approval, without
  turning scheduling or visibility into authority;
- mediate between independently owned workflows without absorbing their state;
- observe exactly what a control action changed and which owner accepted or
  rejected it.

The control surface therefore gives the human full reach through authorized
affordances. It does not bypass invariant boundaries, invent transitions, or
rewrite domain truth directly. If the human needs a capability that the current
environment does not expose, Studio should make that absence visible and route
the request through the owner of the contract rather than synthesizing an
administrative back door.

## Identity-bound control packets

Chat should be one client of the control plane, not the control plane itself.
An independent UI needs a durable, inspectable command envelope that binds a
human intent to the exact environment and authority under which it may be
considered.

A candidate control packet may carry:

```yaml
packet_id: control-0182
issued_at: 2026-08-21T16:42:00-06:00

actor:
  human_identity: user-or-principal-ref
  session_identity: studio-session-ref
  authority_ref: authority-owner-and-revision

recipient:
  repository_id: work-engine
  role_identity: slice-supervisor
  activation_ref: active-role-lease-or-session-ref

subject:
  domain: slice-campaign
  identity: campaign-42
  expected_revision: 118

intent:
  action: approve_slice_plan
  parameters:
    slice_number: 7

policy:
  approval: explicit
  expires_at: 2026-08-21T17:12:00-06:00
  idempotency_key: campaign-42-slice-7-plan

evidence:
  projection_refs:
    - proposal-packet-or-receipt-ref
  presented_state_digest: sha256:...
```

The exact schema is an implementation choice. The semantic requirements are
not:

- actor, recipient, subject, intended consequence, and authority must not be
  conflated;
- stable logical identity must be distinct from a provider process or terminal
  identity;
- a control must be bound to the state the human actually reviewed, or expose
  that the state has changed before applying it;
- replay, retry, expiry, and duplicate presentation must not duplicate the
  domain consequence;
- the control plane may route, retain, present, and acknowledge the packet, but
  the authoritative workflow owner decides whether its transition is valid;
- the result must identify the accepting owner, resulting revision, and any
  refusal, staleness, or unmet precondition.

This packet boundary allows chat, a graphical Studio, a CLI, or a future
automation client to share the same control semantics without granting any
client special hidden authority.

The bidirectional protocol, client capability advertisement, workflow-to-client
requests, and permission-versus-authority distinctions are developed in
[Work Engine Control Protocol and Environment Affordances](work-engine-control-protocol-and-environment-affordances.md).

## Preferred first client: Visual Studio Code

Visual Studio Code is the preferred first UI candidate. It already places the
human beside repository source, selections, diagnostics, tests, diffs, and
workspace trust, so it can prove both useful projection and mediated control
without requiring the full standalone Studio first.

The extension should be a client of the Work Engine control protocol. It must
not become the control plane, a durable state owner, or a direct reader of
scheduler, agent-state, workflow, or provider-runtime storage.

A narrow first experience could combine native role/workflow/agenda tree views
with a richer environment graph or timeline view, then exercise one typed
environment observation and one identity-bound human control. Extension reload
must reconstruct the view and pending consequences from their authoritative
owners.

VS Code is a first route, not the product boundary. Chat, CLI, a standalone
Studio, and other IDEs should be able to use the same semantic control protocol
and environment-affordance contracts.

## Developer view and visual semantics

A graphical view can do greater justice to Work Engine's structure than a chat
transcript alone. The developer view should make relationships visible at the
scale on which a human must reason about them.

Candidate coordinated views include:

- an environment graph for roles, capabilities, owners, consumers, mediated
  transitions, and explicit non-authority;
- a workflow state view that separates current authoritative state from
  projections, pending requests, and historical evidence;
- a timeline for activation, delivery, decisions, route revisions, findings,
  gates, recovery, and acknowledgements;
- a proposal and control-packet inspector showing the exact semantic subject,
  authority, revision, evidence, and projected consequence before approval;
- an agenda view for overdue, due, and upcoming obligations, including which
  items are informational, auto-approved within scope, or awaiting a human;
- a dependency and consequence graph showing why work is blocked and which
  owner can change that condition;
- a forensic comparison between the environment a role was designed to have
  and the environment it actually observed at a decision point.

Color can accelerate recognition, but color alone must never carry meaning.
Every semantic state should also have a label, shape, icon, pattern, or
position. Visual language should consistently distinguish at least:

```text
authoritative       projected           historical
observed            inferred            unresolved
available           awaiting authority  blocked
accepted            declined            stale / superseded
healthy             degraded            unavailable
```

Each visible value should expose its owner, source revision, observed time,
freshness, and confidence or limitation where applicable. A polished graph
with hidden provenance would be less trustworthy than a plain textual view.

The same projection contract should serve both human and agent clients. The UI
may arrange and summarize information, but it must not silently collapse
semantic distinctions that the underlying system preserves.

---

## Relationship to the control plane

The control plane supplies runtime coordination mechanics.

It may own or mediate:

- logical role activation;
- activation leases;
- schedule routing;
- due work;
- delivery;
- listener health;
- acknowledgements;
- role routing;
- subscriptions;
- state-version notifications;
- runtime health.

It should not absorb workflow-domain truth or model judgment.

Conceptually:

```text
                    WORK ENGINE STUDIO

             ┌──────────────────────────┐
             │  shared structural model │
             └─────────────┬────────────┘
                           │
          ┌────────────────┴────────────────┐
          │                                 │
          ▼                                 ▼
     DESIGN VIEW                       CONTROL VIEW
  what may exist                    what is happening now
          │                                 │
          └────────────────┬────────────────┘
                           ▼
                    RUNTIME WORKFLOWS
                           │
                           ▼
                      CONTROL PLANE
                routing / activation / delivery
```

The control plane gives the visual environment live reach.

The structural model tells the interface what that reach means.

---

## Cross-workflow observation

State observation can be represented as an explicit role affordance.

For example:

```text
Slice Supervisor
    MAY_INVOKE → observe_proposal_state
    MAY_READ   → proposal.authority_projection
    MAY_NOT_WRITE → proposal.state
```

In the UI, this may appear as a control attached to the supervisor environment.

Selecting it could show the current bounded projection:

```yaml
proposal_id: proposal-17
revision: 6
status: authorized
blocking_conditions: []
implementation_handoff_ref: handoff-22
```

The observation changes what the supervisor can know. It does not change proposal ownership.

This is one example of how the static environment graph can directly become an operational control surface.

---

## Workflow designer

The same system can expand from individual role design to workflow design.

A workflow designer may connect separately owned state machines:

```text
Proposal Workflow
    owns proposal_state

        ↓ authorized handoff / observation

Slice Workflow
    owns slice_state

        ↓ execution consequences

Strategic Planning Workflow
    owns planner_state
```

The designer can configure workflow identity, semantic state ownership, lifecycle states, authorized projections, role participation, mediated transitions, required upstream consequences, downstream outputs, persistent-state integration, and planner/reviewer attachment points.

Shared durability machinery can support all workflows without owning their semantic state.

---

## Forensics mode

A third projection naturally follows from the same model.

Forensics mode answers:

> **What happened, why did it happen, and what did the agent's environment look like when it happened?**

The interface may replay:

- workflow transitions;
- role activation;
- decision traces;
- route revisions;
- findings;
- evidence acquisition;
- state changes;
- authority decisions;
- compaction/reconstruction events;
- scheduled delivery;
- waiting conditions;
- gate outcomes;
- checkpoints;
- terminal receipts.

Conceptually:

```text
13:04  builder activated
13:08  decision D17: caller B classified non-authoritative
13:12  decision D19: placement A selected
13:19  reviewer finding F3 opened
13:24  evidence E31 falsified D17
13:25  D17 / D19 marked stale
13:28  route revised
13:41  repair completed
13:49  review accepted
13:52  slice accepted
```

Each event can drill down to its owning role, invariant, state revision, evidence reference, decision ancestry, receipt, and raw observable interaction trace.

---

## Design, control, and forensics are one system

The central product insight is:

> **The same structural model should support designing the organization, operating the organization, and investigating the organization.**

Conceptually:

```text
                STRUCTURAL MODEL
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
       DESIGN       CONTROL      FORENSICS
     what may be   what is now   what happened
```

This avoids three common failure modes:

1. design documentation drifting from runtime;
2. control UIs inventing authority not represented in architecture;
3. forensic tools reconstructing semantics from opaque logs after the fact.

---

## Potential implementation stages

These stages are a route-open planning aid, not an authorized campaign or a
prescribed subsystem decomposition. Before implementation begins, current
repository evidence should select owners and consumers for each consequence.

### Stage 1 — Role designer

- create and edit role environment definitions;
- assign capabilities;
- bind invariants;
- define owned/observable/mutable state;
- define mediated transitions;
- define explicit prohibitions;
- preview generated role page;
- run structural diagnostics.

### Stage 2 — Workflow designer

- define workflow-owned state machines;
- connect roles;
- configure cross-workflow projections;
- define handoffs;
- connect planner/reviewer roles;
- define persistent-state interfaces;
- generate structured configuration.

### Stage 3 — Runtime control overlay

- connect control-plane identity and activation;
- show live workflow/role state;
- show waiting/blocked conditions;
- expose authorized observation controls;
- expose authorized mediated actions;
- surface health and delivery state.

### Stage 4 — Forensic replay

- replay semantic transition history;
- inspect decision traces;
- inspect route revisions;
- inspect evidence;
- inspect compaction/recovery;
- compare design intent with runtime behavior.

### Stage 5 — Simulation and design analysis

- simulate reachable transitions;
- detect underpowered roles;
- detect overbroad authority;
- compare alternative environment designs;
- estimate capability redundancy;
- test independence boundaries;
- evaluate proposed workflow changes before activation.

### Candidate control-surface entry sequence

When this direction is resumed, a small vertical path is preferable to a broad
dashboard shell:

1. establish the identity, revision, authority, refusal, replay, and result
   consequences for one control packet;
2. project one existing authoritative workflow and one active logical role into
   a read-only developer view with freshness and provenance;
3. let a human issue one mediated action through that packet and show the
   authoritative owner's response;
4. add activation and subscription machinery only when their durable owners and
   recovery semantics are demonstrated;
5. expand to scheduling, proposals, reviews, gates, receipts, graphs, and
   forensics through adapters that reference rather than duplicate domain truth;
6. validate that a non-chat client can reconstruct the same view and control
   consequences after context loss or process replacement.

This sequence deliberately leaves framework, database, transport, rendering
library, and deployment topology open. Those choices should follow from the
selected ownership, recovery, latency, accessibility, and operator
consequences—not become doctrine because they are convenient for the first
prototype.

---

## Important boundaries

### Structural truth remains owned outside the UI

The visual tool must not become a second source of truth.

Edits should write back to the structured owners or produce reviewed change proposals.

### Runtime control does not create authority

A visible button must correspond to an already-authorized affordance.

The interface cannot grant itself authority by rendering an action.

### Control plane does not become domain owner

The control plane coordinates activation, routing, delivery, and runtime mechanics.

Workflow state remains owned by the workflow.

### Visual simplicity must not erase semantic distinctions

Observation, mutation, mediation, ownership, independence, and prohibition are different relationships and should remain distinguishable.

### Generated analysis is candidate evidence

Static analysis can identify likely structural problems.

A model and human authority remain responsible for semantic classification and contract changes.

---

## Product consequence

Work Engine may eventually become more than an agent workflow runtime.

Because its roles, invariants, authority, state, and machinery are explicitly modeled, it can become a **design and operations platform for constructing AI engineering organizations**.

The designer is not authoring prompts directly.

It is authoring environments:

```text
What must remain true?
What can this role observe?
What can it change?
What must it request from another owner?
What machinery does it possess?
What consequences does it owe?
What can it never do?
```

The control plane then makes those environments operational.

The forensic layer makes their behavior inspectable.

---

## Compact statement

> **Design the role environment.  
> Operate that same environment through the control plane.  
> Investigate its history through the same structural model.**

A sufficiently explicit architecture turns the Work Engine design tool into its control interface—and eventually into its forensic console as well.
