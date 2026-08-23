# Work Engine ↔ Codex Runtime Adapter Boundary Analysis

## Status

Architecture analysis against the current Work Engine repository and the previously researched Codex App Server runtime.

## Question

Where are the architectural edges for using Codex App Server as a runtime adapter beneath Work Engine without allowing provider runtime semantics to leak into workflow meaning, authority, or durable product state?

---

# Executive conclusion

The current Work Engine architecture is already unusually well prepared for a runtime-adapter layer.

The strongest fit is:

```text
WORK ENGINE SEMANTICS
    logical actors
    workflow state
    authority
    obligations
    decisions
    receipts
          │
          │ provider-neutral runtime port
          ▼
RUNTIME ADAPTER
    bind / resume / activate
    observe runtime status
    deliver input
    normalize lifecycle events
    project runtime topology
          │
          ▼
CODEX APP SERVER
    threads
    turns
    parent/child runtime topology
    execution events
    approvals
    persisted provider context
```

The adapter should **not** be inserted into the durable-state primitive, scheduler store, role contracts, or semantic workflow transition logic.

Instead, Work Engine needs a small provider-neutral runtime boundary between:

1. **semantic logical-agent / workflow ownership**, and
2. **provider execution identity and lifecycle**.

Three current implementation points already reveal this seam:

- `live_slice_state.py` explicitly separates `logical_actor_id` from `provider` and `runtime_session_id`;
- `role_scheduler.py` addresses durable work to `repository_id + logical_role + logical_agent_id`, not provider sessions;
- `harvest_codex_telemetry.py` already treats Codex thread IDs as corroborated runtime provenance rather than semantic identity.

The primary architectural change is therefore not to replace these systems, but to **extract runtime binding into its own owner** and let those systems reference it.

---

# 1. Existing layers that should remain unchanged

## 1.1 Durable-state primitive

Current owner:

```text
skills/durable-state/
```

Its contract is excellent for this architecture:

> opaque key + opaque payload + expected revision → atomic durable publication

The store is intentionally semantically blind.

That means it should **not** gain methods such as:

```text
resume_codex_thread()
start_turn()
find_subagents()
```

or provider-specific schemas.

Runtime bindings may be persisted *using* this primitive, but the primitive must remain unaware of their meaning.

### Boundary

```text
durable-state
    owns: durability mechanics
    does not own: runtime meaning
```

No change required.

---

# 2. The most important current seam: live slice actor binding

Current file:

```text
skills/slice-supervisor/scripts/live_slice_state.py
```

The current state schema contains:

```text
actor_binding:
    logical_actor_id
    provider
    runtime_session_id
```

This was architecturally prescient because logical identity is already separated from provider runtime identity.

The supervisor skill reinforces the same contract:

> Model and provider sessions are runtime bindings, not durable owners of an active slice attempt.

It also says provider session identity is optional provenance.

## Problem

If Codex App Server becomes a real persistent runtime substrate, the current `actor_binding` object becomes too small and too embedded in slice semantic state.

A useful Codex binding may need:

```text
provider
runtime_kind
thread_id
session_id
parent_thread_id
binding_revision
bound_at
last_seen
loaded
runtime_status
active_turn_id
```

Those are not slice semantics.

Putting them directly into `live_slice_state` would make the slice workflow the owner of provider runtime lifecycle.

That would violate the architecture.

## Recommended change

Replace:

```yaml
actor_binding:
  logical_actor_id: builder-1
  provider: codex
  runtime_session_id: abc
```

with something closer to:

```yaml
actor:
  logical_actor_id: builder-1
  runtime_binding_ref: runtime-binding:builder-1:7
```

or, if compatibility requires retaining the existing field:

```yaml
actor_binding:
  logical_actor_id: builder-1
  runtime_binding_ref: runtime-binding:builder-1:7
```

The live slice owns:

```text
which logical actor owns this attempt
```

The runtime layer owns:

```text
how that logical actor is currently embodied
```

### Recommended edge

```text
live_slice_state
      │
      │ logical_actor_id
      ▼
runtime binding registry
      │
      ▼
CodexRuntimeAdapter
```

---

# 3. A new explicit owner is needed: Runtime Binding Registry

The repository currently has the concept but not a dedicated executable owner.

Create a provider-neutral component conceptually like:

```text
runtime-bindings
```

This can begin very small.

## Minimum record

```yaml
schema_version: 1

logical_actor_id: builder-1

binding:
  provider: codex
  runtime_kind: app_server_thread
  runtime_id: 019f...
  session_id: 019f...

status_projection:
  loaded: true
  status: idle
  active_turn_id: null

provenance:
  bound_at: ...
  last_seen_at: ...

revision: ...
```

## Important distinction

Separate:

```text
binding fact
```

from:

```text
live runtime projection
```

because the first may remain true while the thread is unloaded.

Example:

```text
binding:
  thread_id: 019f...

runtime:
  status: notLoaded
```

That is not a broken binding.

## Ownership

The registry owns only:

- Work Engine logical actor → provider runtime binding;
- current observed runtime status;
- binding history/revision;
- runtime parent/child observations where useful;
- provider capability observations.

It does **not** own:

- workflow phase;
- pending review obligation;
- acceptance;
- role authority;
- proposal state;
- decisions;
- receipts.

---

# 4. The provider-neutral Runtime Adapter port

Do not make Codex App Server the interface.

Define the Work Engine-side interface first.

Conceptually:

```python
class RuntimeAdapter(Protocol):
    def capabilities(...) -> RuntimeCapabilities
    def list_runtime_actors(...) -> ...
    def inspect(runtime_id) -> RuntimeActor
    def bind_or_create(...) -> RuntimeBinding
    def resume(runtime_id) -> RuntimeBinding
    def start_input(runtime_id, input) -> Delivery
    def steer(runtime_id, input) -> Delivery
    def enqueue(runtime_id, input) -> Delivery
    def interrupt(runtime_id) -> ...
    def subscribe(after_revision_or_cursor) -> EventStream
```

The exact API should emerge from consumers rather than this list becoming a premature invariant.

The architectural requirements are simpler:

```text
observe runtime
bind runtime
deliver authorized input
receive runtime events
```

Everything else is provider machinery.

---

# 5. Codex adapter placement

A concrete package could eventually look like:

```text
skills/
  runtime-adapter/
      contract / schemas

runtime/
  codex_app_server.py
  bindings.py
  events.py
```

or remain skill-local initially.

The important issue is ownership, not directory name.

## Adapter responsibilities

The Codex adapter may:

- initialize an App Server connection;
- enumerate stored threads;
- enumerate loaded threads;
- read thread metadata;
- resume a thread;
- start a turn;
- steer an active turn;
- enqueue follow-up input when supported;
- interrupt a turn;
- consume lifecycle notifications;
- expose parent/child runtime topology;
- answer provider capability availability;
- normalize provider events.

## Adapter must not

- accept a slice;
- decide whether a due item is authorized;
- decide whether a review finding blocks;
- mutate workflow phase directly;
- invent Work Engine agent identity;
- promote every Codex child into a Work Engine actor;
- decide whether a runtime event has semantic consequence.

---

# 6. Scheduler integration edge

Current scheduler recipient identity is already correct:

```text
repository_id
logical_role
logical_agent_id
```

That is exactly what a runtime-neutral scheduler should address.

The scheduler should therefore **never add `thread_id` to scheduled items as recipient identity**.

## Current gap

The scheduler documentation already anticipates activation leases, but the prototype does not yet implement the authority/activation layer.

This creates the correct insertion point:

```text
role scheduler
      │
      │ due obligation
      ▼
authority / activation resolver
      │
      │ authorized logical actor
      ▼
runtime binding resolver
      │
      ▼
RuntimeAdapter
      │
      ▼
Codex App Server
```

The scheduler remains unaware of how input reaches Codex.

## Do not do this

```text
scheduler.take_due()
    -> codex.thread_start()
```

That would let time passage directly trigger provider execution.

It collapses the authority boundary.

## Better

```text
due event
    -> Work Engine control decision
    -> activation validated
    -> logical actor resolved
    -> runtime delivery requested
```

The adapter merely executes the already-authorized runtime consequence.

---

# 7. Activation leases belong above the runtime adapter

This is important.

Codex thread availability is not Work Engine authority.

These are different:

```text
Codex:
thread exists and is idle

Work Engine:
logical actor currently holds authority to execute obligation X
```

Therefore the future activation lease should reference both:

```yaml
logical_actor_id: builder-1
lease_epoch: 8
authority_ref: ...
runtime_binding_ref: runtime-binding:builder-1:7
```

but the lease owner should remain Work Engine control semantics.

Codex does not mint Work Engine leases.

The runtime adapter may report:

```text
thread unavailable
thread active
thread unloaded
```

but it cannot report:

```text
builder authorized
```

---

# 8. Telemetry integration edge

Current file:

```text
skills/slice-supervisor/scripts/harvest_codex_telemetry.py
```

This is highly relevant.

Today it reconstructs:

- parent thread;
- child thread;
- launch relationship;
- agent path;
- terminal turn;
- interrupted turns;
- token events;
- runtime timing;

from rollout JSONL.

This proves the existing architecture already treats provider runtime identity as **evidence ingress**.

App Server can improve the source of that evidence.

## Recommended evolution

Keep:

```text
telemetry ingress schema
assembly / validation
semantic receipt boundary
```

Replace or supplement:

```text
rollout-file discovery
```

with:

```text
App Server runtime/event observations
```

Conceptually:

```text
CURRENT

rollout JSONL
    ↓
harvest_codex_telemetry
    ↓
telemetry ingress

FUTURE

Codex App Server events
    ↓
Codex runtime adapter / event normalizer
    ↓
telemetry ingress
```

This is one of the lowest-risk first integrations because the semantic consumer already exists.

---

# 9. Provider resolution is NOT the runtime adapter boundary

Current file:

```text
skills/slice-builder/scripts/resolve_provider.py
```

This resolves things such as:

```text
repository_evidence:
    provider
    skill

independent_review:
    provider
    skill
```

That is a **capability/provider selection contract**.

It should not become responsible for runtime threads.

Do not evolve it into:

```text
resolve_provider(...)
    -> thread_id
    -> app_server_socket
    -> activation
```

Instead:

```text
provider resolution
    decides which capability/provider fulfills a role

runtime binding
    decides which provider runtime actor embodies a logical actor
```

These may intersect, but they are separate axes.

---

# 10. Work Engine role identity vs runtime topology

The planned architecture correctly distinguishes durable logical actors from provider subagents.

Codex's `parent_thread_id` should therefore enter Work Engine as:

```text
runtime topology observation
```

not:

```text
semantic parent_agent_id
```

## Example

```text
WE-builder-12
    ↕ binding
Codex thread A
    ├── Codex scout B
    └── Codex explorer C
```

B and C can appear in the UI as runtime descendants.

They become durable Work Engine actors only if a semantic owner deliberately promotes them.

A useful promotion criterion is:

> Does this actor independently own durable state, authority, obligations, decisions, or continuity that must survive runtime replacement?

If not, leave it provider-local.

---

# 11. UI edge

The Studio architecture becomes particularly clean with this adapter.

A role page should query two independent projections.

## Semantic projection

From Work Engine:

```text
role
objective
invariants
capabilities
authority
workflow state
owned artifacts
decisions
findings
obligations
receipts
```

## Runtime projection

From runtime bindings / adapter:

```text
provider
thread
loaded status
active turn
last seen
runtime children
provider requests
execution state
```

Then join by:

```text
logical_actor_id
```

not `thread_id`.

## Result

```text
Slice Builder 12

SEMANTIC
status: waiting_on_capability
open finding: F-4
authority: mutate task-owned files

RUNTIME
provider: codex
thread: 019f...
loaded: false
last seen: 22:17
```

This is exactly the distinction the UI needs to remain truthful.

---

# 12. Runtime event ingestion

A small normalized provider-event layer would be useful.

Example normalized events:

```yaml
kind: runtime.actor.status_changed
logical_actor_id: builder-1
binding_ref: runtime-binding:builder-1:7
provider_event_ref: ...
observed:
  status: active
```

```yaml
kind: runtime.turn.completed
logical_actor_id: builder-1
binding_ref: ...
runtime_turn_id: ...
```

```yaml
kind: runtime.child.started
parent_binding_ref: ...
runtime_child_id: ...
```

These are **observations**.

Workflow owners decide whether an event implies:

```text
resume capability
record telemetry
open waiting state
close waiting state
request approval
do nothing
```

This preserves the rule:

> provider events do not directly become workflow transitions.

---

# 13. Capability waiting edge

The current persistent live-state implementation has:

```text
active
waiting_on_capability
retired
```

This is an excellent consumer of runtime information.

Example:

```text
semantic state:
    waiting_on_capability(independent_review)

runtime/control observation:
    provider capability restored

workflow decision:
    resume_capability(...)
```

The runtime adapter should not call `resume_capability` itself.

Instead it emits enough evidence for the workflow/control owner to make that transition.

This is particularly important for Claude quota-reset scheduling.

---

# 14. Claude and non-Codex providers

Do not make the new runtime layer synonymous with Codex.

The Work Engine contract should support:

```text
RuntimeAdapter
    ├── CodexAppServerAdapter
    ├── future ClaudeRuntimeAdapter
    └── process/CLI adapter
```

Even if only Codex is initially implemented.

This protects:

- provider diversity;
- independent-review semantics;
- future replacement;
- cross-provider role configuration.

The current provider-selection code already demonstrates that Work Engine needs multiple providers.

---

# 15. Suggested component map

A clean future architecture is:

```text
                         WORK ENGINE

┌─────────────────────────────────────────────────────────┐
│ Workflow semantics                                      │
│                                                         │
│ slice state · proposal state · review state · planner    │
└───────────────────────┬─────────────────────────────────┘
                        │ logical_actor_id / refs
                        ▼
┌─────────────────────────────────────────────────────────┐
│ Logical actor + authority/control layer                 │
│                                                         │
│ logical identity · activation lease · authorized action │
└───────────────────────┬─────────────────────────────────┘
                        │
          ┌─────────────┴─────────────┐
          ▼                           ▼
┌──────────────────────┐   ┌──────────────────────────────┐
│ Scheduler            │   │ Runtime Binding Registry     │
│                      │   │                              │
│ durable obligations  │   │ logical actor → runtime      │
│ delivery state       │   │ runtime status projection    │
│ acknowledgements     │   │ binding history              │
└──────────┬───────────┘   └──────────────┬───────────────┘
           │                               │
           └──────── authorized delivery ──┘
                                           │
                                           ▼
                              ┌──────────────────────────┐
                              │ Runtime Adapter Port     │
                              └────────────┬─────────────┘
                                           │
                         ┌─────────────────┴──────────────┐
                         ▼                                ▼
                Codex App Server                 future provider
```

---

# 16. What should be current truth vs cached projection

## Work Engine authoritative

- logical actor identity;
- semantic workflow state;
- authority;
- activation lease;
- pending obligation;
- handled consequence;
- decision trace;
- acceptance;
- scheduler item state;
- receipts.

## Runtime adapter observed

- provider thread exists;
- provider thread loaded;
- active/idle/error state;
- runtime parent;
- runtime children;
- active turn;
- provider-side queue;
- approval/input requests;
- provider availability.

## Derived UI projection

- "Builder 12 is active in Codex";
- "Builder 12 is semantically waiting although its thread still exists";
- "Reviewer runtime disappeared but review obligation remains pending";
- "Two transient Codex descendants currently support Builder 12".

The derived projection is not itself authoritative state.

---

# 17. First implementation slice I would choose

Do not begin by wiring scheduling.

Begin with **read-only runtime binding and observation**.

## Slice A — Codex runtime observation adapter

Implement:

```text
connect to App Server
initialize
list persisted threads
list loaded threads
read one thread
subscribe to lifecycle/status events
normalize runtime actor status
```

No workflow mutation.

## Slice B — explicit logical actor binding

Add a durable binding record:

```text
logical_actor_id → Codex thread_id
```

Display/recover it across client restart.

Still no scheduler execution.

## Slice C — UI/runtime projection

Join:

```text
live_slice_state logical_actor_id
+
runtime binding
+
Codex runtime status
```

Render it.

This proves the semantic/runtime split.

## Slice D — controlled input

Add:

```text
start input on an already-authorized logical actor
```

with Work Engine authority supplied externally.

## Slice E — scheduler bridge

Only after activation/authority semantics exist:

```text
due obligation
→ validated activation
→ binding resolution
→ runtime delivery
→ independent acknowledgement
```

This sequence keeps the most dangerous edge until the ownership contracts are proven.

---

# 18. Existing code most likely to change

## `skills/slice-supervisor/scripts/live_slice_state.py`

Change:

```text
actor_binding contains provider runtime fields
```

toward:

```text
logical actor + runtime binding reference
```

Do not add App Server calls.

## `skills/slice-supervisor/scripts/resume_active_slice.py`

Potentially resolve/display runtime binding after semantic recovery, but semantic resume must succeed independently of provider runtime availability.

## `skills/slice-supervisor/scripts/harvest_codex_telemetry.py`

Introduce an alternate App Server event/provenance ingress while retaining the existing semantic telemetry output.

## `skills/role-scheduler/*`

Eventually consume a provider-neutral activation/delivery service.

Do not embed Codex RPC.

## Agent environment model

Add runtime-observation capabilities later, for example:

```text
MAY_OBSERVE runtime.binding
MAY_OBSERVE runtime.status
MAY_REQUEST runtime.delivery
```

Be careful not to encode provider-specific Codex methods as invariant role capabilities.

## Studio UI

Join static role environment + workflow semantic state + runtime projection.

---

# 19. New code I would add

Conceptually:

```text
runtime/
    contract.py
    bindings.py
    events.py
    codex_app_server.py
```

or equivalent skill-packaged machinery.

### `contract.py`

Provider-neutral types and adapter protocol.

### `bindings.py`

Logical actor ↔ runtime identity ownership and persistence.

### `events.py`

Normalize provider events into runtime observations.

### `codex_app_server.py`

JSON-RPC / socket implementation for Codex.

No workflow semantics should live here.

---

# 20. Anti-corruption boundary

Treat the runtime adapter as an **anti-corruption layer** between provider concepts and Work Engine concepts.

Translate:

```text
Codex thread
→ runtime actor

Codex turn
→ runtime execution

thread/status/changed
→ runtime status observation

parent_thread_id
→ runtime-parent observation

thread/resume
→ provider resume affordance
```

Do not translate:

```text
thread completed
→ slice accepted

thread exists
→ role authorized

subagent spawned
→ Work Engine agent created

queue item executed
→ scheduler obligation acknowledged
```

Those latter mappings require Work Engine semantics.

---

# 21. Architectural edge summary

The main edges are:

## Edge A — Semantic workflow ↔ logical actor

Already exists implicitly in `live_slice_state`.

Strengthen it.

```text
slice attempt → logical_actor_id
```

## Edge B — Logical actor ↔ runtime binding

**New explicit owner required.**

```text
logical_actor_id → provider runtime binding
```

## Edge C — Runtime binding ↔ Codex App Server

**Runtime adapter.**

```text
binding → thread / turn / events
```

## Edge D — Runtime observations ↔ workflow semantics

Must remain mediated.

```text
runtime event → evidence
workflow owner → semantic transition
```

## Edge E — Scheduler ↔ authority/control ↔ runtime delivery

Do not connect scheduler directly to Codex.

```text
due obligation
→ authority
→ logical actor
→ runtime binding
→ adapter delivery
```

## Edge F — Codex runtime events ↔ telemetry ingress

Excellent early integration.

```text
App Server event stream
→ runtime normalizer
→ existing telemetry ingress
```

## Edge G — UI ↔ semantic + runtime projections

Join on Work Engine logical identity.

```text
role page
= semantic environment
+ live workflow state
+ runtime projection
```

---

# Final recommendation

The current architecture should **not be redesigned around Codex App Server**.

That would be the wrong lesson.

Instead, Codex App Server should become one replaceable implementation of a runtime port that Work Engine was already implicitly designing.

The cleanest architectural statement is:

> **Workflow state owns what the actor means and what consequence is valid.  
> Runtime binding owns where that actor currently executes.  
> The runtime adapter owns how to observe and drive that provider runtime.**

The most important near-term refactor is therefore:

> **extract provider runtime identity out of workflow semantic state into a first-class runtime-binding projection, while preserving `logical_actor_id` as the join key.**

Once that exists, the scheduler, telemetry ingress, UI, recovery, and future control plane all get a clean place to attach without making any of them the owner of Codex itself.
