# Work Engine Control Protocol and Environment Affordances

## Status

Product direction / architecture proposal. No implementation campaign is
authorized or active from this document.

This document preserves the emerging boundary between the Work Engine control
plane, human-facing clients, workflow-owned state, and external environments.
It also records Visual Studio Code as the preferred first UI candidate without
making VS Code the product boundary or a durable state owner.

Related durable context:

- [Work Engine Studio: Design, Control, and Forensics](work-engine-studio-design-control-forensics.md)
- [Role-Aware Agent Scheduler](role-aware-agent-scheduler.md)
- [Persistent Agent State and Runtime Introspection](persistent-agent-state-and-runtime-introspection.md)

## Core direction

The control plane needs a client-facing protocol through which a human or agent
environment can:

- observe authoritative and projected Work Engine state;
- discover currently available controls and environment capabilities;
- submit identity- and revision-bound intent;
- receive, refuse, or mediate requests from workflows;
- observe which authoritative owner accepted or rejected a consequence; and
- reconcile after client, process, or model-context replacement.

The protocol is not the control plane. It is the boundary through which clients
interact with the control plane.

```text
CLIENTS
VS Code · chat · CLI · future Studio · automation
                         │
                         │ Work Engine control protocol
                         ▼
CONTROL-PLANE BOUNDARY
identity · activation · routing · delivery · subscriptions
control packets · runtime bindings · health · projections
                         │
              ┌──────────┴──────────┐
              ▼                     ▼
       DOMAIN OWNERS          RUNTIME ADAPTERS
proposals · slices · reviews  Codex App Server
agent state · receipts · Git  future providers
```

## Control-plane ownership

The control plane is broader than the scheduler.

The scheduler is one control-plane component. It owns temporal obligation,
recipient, presentation, and scheduling-acknowledgement consequences. It does
not own agent state, workflow meaning, runtime identity, or human authority.

Conceptually, the control plane may compose:

```text
control plane
├── logical identity and role routing
├── activation and authority leases
├── scheduler and delivery
├── runtime-binding registry
├── control-packet lifecycle
├── subscriptions and reconciliation
├── delivery/runtime health
└── bounded projections of stronger owners
```

These responsibilities may initially share a process or daemon. Shared process
topology does not merge their semantic ownership.

The control plane owns coordination and delivery semantics. It must not become
the owner of workflow-domain truth or model judgment.

## Domain and runtime boundaries

The following distinctions remain load-bearing:

```text
agent state
    owns recovery-critical operational consequences

scheduler
    owns when an obligation is due and its delivery lifecycle

runtime binding
    owns the relationship between a logical actor and provider runtime

runtime adapter
    owns provider-specific observation and execution machinery

workflow owner
    owns what a runtime observation or human action means

client
    projects state and mediates human/environment interaction
```

A control-plane projection should reference these owners rather than copy their
full state into another general-purpose database.

## Bidirectional protocol

The protocol should support both client-initiated control and workflow-initiated
environment requests.

### Client to Work Engine

A UI or other client may:

- initialize and identify itself;
- bind to a repository, workspace, and human-supervised session;
- reconcile the current projection at a known revision;
- subscribe to later changes;
- inspect an artifact or authoritative-state reference;
- discover controls currently available to the human;
- prepare and submit a control packet; and
- acknowledge presentation or receipt without claiming domain completion.

### Work Engine to client environment

An authorized workflow may request a typed affordance from an active client:

- read the active editor selection;
- read diagnostics or test state;
- reveal a file or source range;
- present a comparison or proposal;
- request a human choice;
- request a workspace edit; or
- surface a due obligation or waiting condition.

The workflow does not call VS Code APIs directly. It asks for a semantic
capability. A client adapter decides whether and how that capability exists in
its environment.

## Client capability advertisement

On connection or reconciliation, a client can advertise its current
affordances and relevant policy state.

```yaml
client_id: vscode-client-7
client_kind: vscode

workspace:
  repository_id: work-engine
  workspace_id: local-window-3
  trusted: true

activation:
  human_session_ref: supervised-session-12

capabilities:
  - name: editor.selection.read
    mode: observation
    requires_focus: true
    approval_policy: ask_if_sensitive
    max_bytes: 50000

  - name: editor.diagnostics.read
    mode: observation

  - name: editor.range.reveal
    mode: mediated_action

  - name: editor.workspace_edit
    mode: mutation
    approval_policy: require_confirmation
```

This is live capability state, not a permanent role guarantee. A capability may
become unavailable when the client disconnects, workspace trust changes, focus
moves, a document closes, or the host cannot establish the requested
observation.

Role contracts should name semantic affordances such as
`editor.selection.read`, not provider-specific methods such as a VS Code API
function.

## Workflow-to-environment request

A workflow request should identify its actor, purpose, recipient environment,
constraints, authority, and expiry.

```yaml
request_id: observation-284
requesting_actor: builder-12
capability: editor.selection.read

recipient:
  repository_id: work-engine
  workspace_id: local-window-3

purpose: Explain the implementation selected by the human

constraints:
  require_nonempty: true
  max_bytes: 20000
  expires_at: 2026-08-22T15:10:00-06:00

authority_ref: active-builder-environment-9
```

A successful observation should be bound to what was actually observed:

```yaml
request_id: observation-284
status: observed

document:
  uri: file:///workspace/src/example.ts
  version: 17
  language: typescript

selection:
  start: {line: 42, character: 4}
  end: {line: 68, character: 1}
  content: "..."
  content_sha256: "..."
  captured_at: 2026-08-22T15:09:12-06:00

provenance:
  client_id: vscode-client-7
  workspace_id: local-window-3
  workspace_trusted: true
```

Editor state is volatile. Document version, range, capture time, client, and
workspace identity prevent a stale selection from masquerading as timeless
repository truth. Unsaved contents may require stronger disclosure and
retention policy than committed source.

## Control packets

A control packet carries human or authorized-client intent toward a domain
owner. It should bind at least:

- human/client identity;
- active supervised session or activation lease;
- logical recipient;
- domain subject and expected revision;
- intended consequence;
- authority reference;
- approval and expiry policy;
- idempotency or replay identity; and
- the projection the human actually reviewed.

The control plane may validate coordination-level preconditions and route the
packet. The authoritative workflow owner decides whether the requested
transition is valid.

For example, the control plane does not decide what `approve_slice_plan` means.
It routes a correctly attributed request to the slice workflow, which may
accept, refuse, or report that the reviewed plan is stale.

## Permission, capability, and authority

These concepts must remain distinct.

```text
capability
    the environment can perform or observe something

permission
    the host permits this concrete runtime operation

role authority
    the logical actor may request or judge this class of consequence

human approval
    the human authorizes this action, subject, and state

state precondition
    the target still matches what was reviewed
```

An editor mutation may require all five. Passing one does not imply the others.

This resembles the way Codex operates through tools, sandbox boundaries, and
approval requests: the model requests an affordance, the host mediates it, and
an attributed result returns to model judgment. Work Engine generalizes that
shape across durable roles, workflows, clients, and domain-specific authority.

Provider permission is not Work Engine semantic approval. Permission to write
a file does not imply authority to accept a proposal, complete a slice, publish
a commit, or acknowledge a scheduled consequence.

## Human supervision and auto-approval

The human should be able to grant narrowly bounded standing policy where doing
so is useful. Auto-approval must remain bound to the specific action, subject,
recipient, preconditions, time window, and authority that justified it.

Examples are contextual rather than universal task-type rules:

```text
read active filename
    may be automatic

read unsaved selection
    may require visible disclosure or confirmation

run a known read-only test
    may be pre-authorized for a bounded repository and revision

apply a workspace edit
    normally requires confirmation and document-version preconditions

approve a slice
    requires the owning Work Engine authority path
```

The UI should show which role requested an operation, why it requested it, what
will be disclosed or changed, and which authority or policy applies.

## Preferred first client: Visual Studio Code

VS Code is the preferred first UI candidate because it already contains the
developer's repository, source navigation, selections, diagnostics, tests,
diffs, terminals, and workspace trust decisions.

A first extension could expose:

- an Activity Bar container for Work Engine;
- tree views for active roles, workflows, scheduled obligations, findings, and
  approvals;
- status-bar connection and active-role state;
- an editor view for environment graphs, workflow timelines, proposals, and
  control-packet inspection; and
- commands backed by discovered, authority-aware affordances.

The extension remains a replaceable client. It must not directly own or parse
scheduler storage, agent-state storage, campaign internals, or provider runtime
state. It should speak the Work Engine control protocol.

Extension memory is not durable state. Reload, host restart, remote-workspace
movement, or view disposal must be recoverable through protocol reconciliation.
In an untrusted workspace, the extension should preserve useful read-only
projection where safe and withhold trust-sensitive mutations.

## Runtime adapters and Codex App Server

The control protocol should not expose Codex App Server as its semantic
interface.

Instead:

```text
control protocol
    speaks Work Engine identity, authority, projection, and intent

runtime adapter port
    speaks provider-neutral observation and authorized delivery

Codex App Server adapter
    translates threads, turns, status, topology, and provider approvals
```

Codex App Server can reduce custom machinery for Codex runtime discovery,
thread lifecycle, streaming events, and input delivery. It does not replace
Work Engine logical identity, activation, scheduler authority, agent state,
workflow transitions, or consequence idempotency.

The runtime edge should remain pluggable for other providers and local process
routes. Provider-specific facts enter Work Engine as attributed runtime
observations rather than semantic workflow truth.

## Candidate first vertical consequence

When implementation is deliberately authorized, a useful narrow proof would
establish that:

1. a VS Code client connects and identifies one repository and supervised
   environment;
2. it reconstructs one logical actor from authoritative agent/workflow state;
3. it joins a separately owned runtime-binding projection;
4. it displays scheduler state with owner, revision, freshness, and epistemic
   classification;
5. a workflow requests one typed VS Code observation;
6. the client returns an attributed, version-bound result;
7. the human submits one narrow control packet;
8. the authoritative workflow owner accepts or refuses it; and
9. client reload reconstructs the same durable consequences without relying on
   webview, extension, terminal, or model context as the sole owner.

This is a candidate route, not an implementation command. Framework, transport,
database, rendering library, process topology, and the first selected domain
action remain implementation decisions subject to current evidence.

## Open questions

- What durable owner issues and revokes human-supervised activation leases?
- Which protocol component owns connection identity, repository binding, and
  multi-window routing?
- How are observation revisions and replay handled when a provider stream lacks
  durable cursors?
- Which client capabilities may be auto-approved, and who owns those policies?
- How should unsaved editor content be retained, redacted, or excluded from
  durable evidence?
- How are competing clients for the same logical role represented?
- Which control requests may remain pending when no matching environment is
  active?
- How does the UI show disagreement between semantic state, runtime state, and
  workspace state without silently choosing one as truth?

## Compact statement

> **The control plane coordinates identity, authority, routing, and delivery.
> The control protocol exposes that machinery to replaceable clients.
> Clients advertise typed environment affordances and mediate human intent.
> Workflow owners retain domain meaning, and runtime adapters retain provider
> machinery.**
