# Semantic Context Lifecycle Manager

## Status

Planning document for the Work Engine App Server runtime.

This document proposes a shared code-owned context-lifecycle service for every
independently controlled Work Engine role thread. It is not a claim that the
required provider capabilities are implemented or stable in the currently
pinned Codex release.

The plan preserves the semantic model developed in
[`ideas/01-semantic-context-manager-proposal.md`](../../ideas/01-semantic-context-manager-proposal.md)
while revising its execution architecture. Context management becomes common
runtime infrastructure rather than a durable Wind Walker peer role or a skill
loaded into ordinary task agents.

## Architectural decision

The App Server runtime should provide one horizontal semantic context-lifecycle
plane above provider mechanics and below Work Engine roles:

```text
Work Engine roles and workflows
────────────────────────────────
Semantic Context Lifecycle Manager
  observation
  compilation
  verification
  checkpointing
  retirement
  rehydration
  reconciliation
────────────────────────────────
Provider-neutral runtime adapter
────────────────────────────────
Codex App Server daemon
```

Ordinary roles should perform domain work without monitoring their own token
pressure, inventorying continuation meaning, or deciding how to persist it.
The shared service should apply the same lifecycle contract to every Work
Engine role and expose role-specific extension points only where the role owns
additional typed state.

The first supported subjects should be daemon-owned root threads. App
Server's parent-owned Codex subagents have different direct-control boundaries;
their lifecycle may need to remain parent-mediated rather than being treated as
independently controllable Work Engine roles.

## Ownership boundary

The lifecycle manager owns:

- thread and turn observation;
- token, context-window, timing, and cost telemetry;
- bounded model-visible history projections;
- inspection scheduling;
- semantic-compilation and verification invocations;
- continuation-checkpoint storage and revision fencing;
- transition readiness and actuation coordination;
- transition classification;
- rehydration delivery and reconciliation evidence; and
- the external lifecycle ledger.

It does not own:

- a role's objective, domain truth, or workflow authority;
- user approval, preference, or unresolved human intent;
- canonical claims, decisions, receipts, schedules, or repository state;
- authority grants merely described by a checkpoint;
- the exact hidden model context assembled inside Codex Core; or
- provider transition semantics inferred from an event name alone.

A compiled continuation checkpoint is a runtime-owned rehydration artifact. It
may preserve and reference canonical meaning, but it is not a replacement owner
for that meaning.

## Runtime components

### Telemetry collector

The collector consumes provider events and host measurements, including:

- thread and turn identity and status;
- completed turns and their visible items;
- token-usage totals, last-call usage, and reported model context limit;
- tool calls and results;
- context-compaction items and event ordering;
- lifecycle boundaries and expected next work; and
- compilation, verification, retirement, and rehydration timings.

Cheap mechanical observation may be continuous. Semantic compilation should be
bounded and selected only when it has credible lifecycle value.

### Observed-context projector

App Server can return persisted turns and items, but that history is not the
literal effective prompt sent to the model. The projector must therefore emit
an attributed observation with explicit unknowns rather than claiming an exact
context snapshot.

```yaml
observed_context:
  logical_role_instance: strategic-planner:main
  runtime_binding:
    thread_id: "..."
    binding_revision: 4
  source_revision: "sha256:..."
  last_completed_turn: "..."
  visible_items: []
  governing_sources: []
  activated_skills: []
  token_usage:
    total: 0
    last: 0
    reported_context_limit: null
  unknowns:
    - exact_effective_model_input
    - provider_internal_instructions
    - hidden_reasoning_state
```

The source revision must bind every input on which a retirement decision
depends: visible items, governing instructions, skill revisions, durable state,
role identity, runtime binding, lifecycle position, and expected next work.

The implemented `observed-context-v1` foundation constructs this projection
from attributed content references rather than copied content. It uses closed
trust and instruction-applicability values, requires explicit source-inventory
completeness and omissions, preserves the three provider unknowns above, binds
the current lifecycle snapshot, and rejects a snapshot whose thread differs
from the runtime binding. Its canonical source revision is authenticated with a
domain-separated Ed25519 signature carrying the host component, build revision,
and key identity. Independent verification rejects source, trust, construction,
signature, or key tampering.

This attestation establishes which host construction signed which declared
projection bytes. It does not prove that the declared inventory was complete,
that the model literally saw those items, or that retirement is safe. Durable
ledger anchoring and destination authorization remain separate requirements.

### Transport security and projection trust

The observed-context projection crosses a security boundary. Content does not
acquire governing authority merely because it appears in model-visible context
or describes itself as an instruction. The host-owned projector should
preserve the identity, origin, trust class, instruction applicability,
producer attribution, content reference, and integrity evidence of each item:

```yaml
visible_items:
  - identity: thread-item-123
    origin: human
    trust_class: human_authority_input
    instruction_applicability: contract_defined
    content_ref:
      kind: thread-item
      reference: thread-item-123
      sha256: "<lowercase-sha256>"

  - identity: thread-item-124
    origin: retrieved_content
    trust_class: untrusted_data
    instruction_applicability: none
    content_ref:
      kind: thread-item
      reference: thread-item-124
      sha256: "<lowercase-sha256>"

  - identity: thread-item-125
    origin: tool
    trust_class: attributed_evidence
    instruction_applicability: none
    producer: codebase-memory
    content_ref:
      kind: thread-item
      reference: thread-item-125
      sha256: "<lowercase-sha256>"
```

The compiler and verifier should receive only the least privilege needed for
their bounded inference. Snapshot construction must apply confidentiality and
secret filtering, destination-specific read and write authorization, and
retention limits before content is copied. Compiler output must not write to a
canonical owner merely because it names that owner; publication uses the
destination owner's authorization boundary.

A content digest binds projected bytes but does not authenticate the projector
or prove that required source material was not omitted. The canonical
projection should therefore be constructed by an authenticated host component,
record its construction identity and completeness limitations, and anchor the
revision in an append-only or otherwise tamper-evident lifecycle ledger. The
semantic verifier challenges the projection and its attribution; it does not
replace those transport-security controls.

### Semantic compiler

The compiler is a bounded inference capability, not a durable peer role. It
receives the observed-context revision plus current canonical references and
emits a versioned continuation candidate. Its inference context is disposable;
only its attributed output and evidence references are durable.

The compiler should identify the semantic difference between meaning required
for continuation and meaning already durably owned elsewhere. The common
extraction vocabulary includes:

- objective and logical progression;
- current work position;
- completed consequences;
- active commitments and obligations;
- decisions, premises, and invalidation conditions;
- boundaries and authority dependencies;
- evidence and its current interpretation;
- uncertainty and unresolved questions;
- dependencies, identity, and relationships;
- valid, falsified, and superseded routes;
- temporal applicability;
- governing instructions and activated skill state;
- human interactions and their semantic closure; and
- the authorized next action.

Roles may contribute a typed `role_state` extension. A role-specific extension
does not transfer ownership of the shared lifecycle machinery to that role.

The implemented fixture-driven inference foundation places this judgment
behind a provider-neutral compiler capability. Before invocation, the host
verifies the observed-context attestation and admits exactly one digest-matching
content value for every projected visible item, governing source, activated
skill, and expected-next-work reference. Missing, additional, duplicate, or
digest-mismatched material fails before model inference. The model emits only
the semantic body; the host owns and attaches schema identity, logical subject,
source and binding revisions, timestamp, compiler provenance, invocation
identity, and the deterministic candidate revision.

Every semantic record emitted through this boundary must cite supplied source
material. Authority, governing-environment, canonical, and human-interaction
references must also resolve inside the bounded material, and activated-skill
references must exactly match the observed projection. Reload instructions are
limited to projected governing sources and activated skills, so ordinary data
cannot be promoted into governing instructions through compiler output. These
mechanical checks establish attribution and binding constraints, not the truth
or sufficiency of the model's semantic judgment. Raw loaded material is
provided to the bounded inference calls but is not retained in the returned
inspection result.

### Semantic verifier

A separate bounded verification pass should attempt to falsify the candidate's
sufficiency, attribution, authority preservation, interaction closure, and
source binding. The verifier should return blockers and uncertainty rather than
repairing ambiguity into apparent readiness.

Verification reduces semantic omission risk; it cannot establish access to
hidden provider context. Retirement must remain fail-closed when the available
projection is insufficient for a consequential continuation claim.

The implemented verifier foundation receives the exact compilation input and
the host-bound candidate through a separate injected inference capability. It
must challenge `sufficiency`, `attribution`, `authority_preservation`,
`interaction_closure`, and `source_binding`, citing supplied material or the
exact observed-context and candidate revisions for every check and finding.
The model cannot emit a verdict: host code derives `rejected` from any failed
check or blocker, `unresolved` from remaining uncertainty, and `accepted` only
when every check passes without blockers or uncertainty. It then binds the
result to a deterministic verification revision.

Separate capability objects and distinct inference invocation identities
prevent accidental reuse of one call as both judgments. They do not prove a
different provider, model, hidden reasoning context, or organizationally
independent reviewer. Provider-specific disposable-context enforcement and a
live model-quality proof remain future integration work. Even an `accepted`
verification is only the outcome of this bounded challenge; it is not
checkpoint publication, lifecycle readiness, retirement authorization, or
transition evidence.

### Lifecycle ledger

The ledger is the authoritative runtime projection of context lifecycle. It
should record:

- logical role and current runtime binding;
- observed source revision;
- token pressure and inspection reason;
- compiler and verifier identities and versions;
- candidate and accepted checkpoint revisions;
- unresolved blockers;
- transition readiness and authorization;
- model-actuation request and observed tool call;
- prior and successor context-window evidence;
- transition classification;
- rehydration acknowledgement; and
- reconciliation, repair, or uncertainty.

The ledger must not claim a transition, checkpoint publication, or successful
rehydration merely because it was attempted.

The implemented `context-lifecycle-ledger-v1` foundation is a storage-neutral
record contract, not yet the authoritative durable ledger described above. It
assigns a monotonic sequence, binds each entry to its predecessor revision, and
hashes normalized entry content. Whole-chain verification detects content
mutation, reordering, and broken predecessor linkage within the supplied
chain. It cannot detect omission of a valid suffix without an external anchor,
authenticate the writer, prove that a supplied chain is complete, or persist
records. Those properties remain responsibilities of the eventual protected
ledger owner.

The closed event vocabulary currently distinguishes observations, checkpoint
candidates, verification, readiness, publication, actuation requests,
transition observations, reconciliation, and failures. Status constraints
prevent an actuation request from being recorded as more than `attempted`, a
transition observation from being recorded as `accepted`, or a failed action
from being recorded as success. Schema or chain validity never grants
transition authority.

The implemented checkpoint-publication foundation consumes the authenticated
projection, complete continuation candidate, and integrity-valid semantic
verification. Only an `accepted` verifier disposition can proceed. Before
publication, a destination-owned callback must revalidate the exact objective,
authorized-action, canonical authority, durable human consequence, and original
human-source reference set. It returns an authority revision and attributed
evidence; incomplete coverage, invalid authority, or unresolved authority fails
closed.

Publication then submits the complete continuation state, its verification, and
one `checkpoint_published` ledger entry through a single compare-and-swap store
operation. The comparison binds logical role, thread, runtime-binding revision,
observed source revision, authority revision, predecessor checkpoint revision,
and predecessor ledger revision. The reference store independently verifies the
checkpoint digest, ledger linkage, and subject continuity before committing
both records. A stale or conflicting fence, duplicate candidate, unresolved
verification, or invalid authority returns no publication claim.

The publisher accepts an injected atomic store contract. The included in-memory
realization proves the boundary and race outcomes inside one process; it does
not provide authenticated writing, protected durable storage, cross-process
serialization, or recovery. Publication is not retirement readiness or
transition authorization, and this implementation delivers no actuator turn.

## Compiled continuation state

The universal continuation schema should remain compact while preserving exact
source references for authoritative or ambiguous meaning.

The implemented JavaScript-facing `continuation-state-v1` representation uses
the exact property names and reference shapes below:

```yaml
schemaVersion: 1
type: work-engine.continuation-state

subject:
  logicalRoleInstanceId: strategic-planner:main
  runtimeBindingRevision: 4
  sourceRevision: "sha256:<lowercase-sha256>"

compiledAt: "2026-08-25T00:00:00Z"
compiler:
  producer: work-engine.semantic-context-compiler
  model: compiler-model
  version: "1"
  inferenceId: "compiler-inference-id"

objective:
  statement: "Keep the Work Engine roadmap coherent"
  authorityRef:
    reference: "user-message:..."
    sha256: "<lowercase-sha256>"

workPosition:
  phase: implementation
  currentUnit: planner-evidence-delivery-remediation

completedConsequences: []
activeCommitments: []
decisions: []

humanInteractions: []

authorityDependencies:
  canonicalRecords: []
  revalidationRequired: []

unresolved: []

governingEnvironment:
  roleContract:
    reference: "skills/strategic-planner/SKILL.md"
    sha256: "<lowercase-sha256>"
  instructionsToReload: []
  activatedSkills: []

canonicalReferences: []

authorizedNextAction:
  kind: remediate
  objective: "Deliver planner evidence through a supported model-visible input"
  authorityRef:
    reference: "authority-record:..."
    sha256: "<lowercase-sha256>"

roleState:
  schema: strategic-planner-continuation-v1
  value: {}

uncertainty: []
```

Validation adds a deterministic `candidateRevision`; it does not add an
accepted checkpoint revision. The candidate revision binds normalized content
for later verification and ledger evidence, but it does not prove semantic
sufficiency, confer authority, publish a checkpoint, or authorize retirement.
Downstream owners must revalidate referenced authority before accepting the
candidate.

The state is compiled meaning, not a transcript summary. Exploratory reasoning,
closed conversation, and raw tool output should be omitted unless legitimate
continuation depends on them.

## Human interaction compilation

Human-authored interaction carries authority and intent that must not be closed
merely to make retirement convenient. The compiler should classify interaction
state separately from its next-context loading disposition.

```yaml
humanInteractions:
  - id: "user-message:..."
    kind: implementation_authorization
    status: closed_but_active
    authorityEffect: "authorizes changes under app-server/"
    durableConsequenceRef:
      reference: "authority-record:..."
      sha256: "<lowercase-sha256>"
    sourceRef:
      reference: "thread-item:..."
      sha256: "<lowercase-sha256>"
    nextContextDisposition: compiled_consequence

  - id: "user-message:..."
    kind: clarification
    status: open
    authorityEffect: null
    durableConsequenceRef: null
    sourceRef:
      reference: "thread-item:..."
      sha256: "<lowercase-sha256>"
    nextContextDisposition: exact
```

Candidate statuses are:

- `open`: a response, action, choice, clarification, or acknowledgement remains;
- `resolved`: the requested consequence was established;
- `closed_but_active`: the interaction is finished but its decision still governs;
- `superseded`: later human authority explicitly replaced it;
- `historical`: it no longer affects legitimate continuation; and
- `ambiguous`: meaning or closure cannot safely be inferred.

Candidate loading dispositions are:

- `exact`: load the attributed original text;
- `compiled_consequence`: load the durable consequence and its source;
- `reference_only`: retain a resolvable source without loading its text;
- `omit_from_working_context`: retain it in history but not the new window; and
- `escalate`: block retirement or seek human clarification.

Exclusion from the next model context is not deletion, resolution, or loss of
authority. The original interaction remains durably addressable. An assistant
response alone does not prove completion of an implementation request, an
approval decision, or another interaction whose consequence remains pending.

The implemented human-interaction boundary preserves this separation through
both deterministic constraints and model judgment. An interaction source must
resolve to a projected human-authored item. A `compiled_consequence` requires a
durable consequence reference; `ambiguous` meaning must remain `exact` or
`escalate`; `open` meaning cannot be omitted; and `closed_but_active` meaning
cannot become `reference_only` or be omitted. These are fail-closed boundary
conditions, not an exhaustive status/disposition routing table. Other valid
combinations remain subject to the bounded compiler and verifier's semantic
judgment.

For every compiled interaction, the verifier must return exactly one evaluation
bound to the same interaction identity, human source, claimed status, and
loading disposition. It separately assesses closure as `supported`,
`contradicted`, or `uncertain` and loading as `supported`, `insufficient`, or
`uncertain`, with attributed evidence. Host code derives the aggregate
`interaction_closure` check from those evaluations and rejects a contradictory
aggregate claim. Any contradiction or insufficient loading fails the check;
remaining uncertainty, an `ambiguous` status, or `escalate` keeps it uncertain.
The evaluation cannot mutate the candidate, and neither omission nor a
supported classification resolves a human interaction.

## Retirement and `new_context` actuation

Preservation and retirement are independent decisions. Rising pressure may
justify compilation while retention remains preferable. Retirement should occur
only when the accepted checkpoint is sufficient and replacing the context is
semantically safe and economically advantageous.

The currently observed `new_context` capability is model-invoked,
undocumented, and feature-gated. A live probe against the pinned Codex CLI
0.149.1 established that it can replace the model context window while
preserving the App Server thread identity. The host still cannot treat it as a
stable direct App Server request. Therefore the target model remains a narrow
transition actuator even though every other lifecycle responsibility belongs to
the code-owned service.

The production adapter now represents configured support for this route only
through the provider-neutral `model_context_replacement` capability. Its pinned
provider profile fixes the current mechanism to `new_context`, the invocation
owner to `target_model`, the
required provider feature to `token_budget`, and post-request transition
evidence to mandatory. Adapter initialization fails before the App Server
handshake when the capability is requested without an explicit host declaration
that the feature is configured. The capability set cannot be widened after
initialization, and its profile is included in the durable role-thread
environment fingerprint so an incompatible process cannot silently resume the
binding.

This gate authenticates neither provider configuration nor transition success.
The host declaration is configuration evidence, not a capability handshake or
proof that the tool appeared in the model context. The exact live probe binds
the declaration to an App Server process launched with `--enable token_budget`,
while lifecycle observations and successor reconciliation remain necessary to
classify any requested transition. The transition runtime now supplies the
separate preparation, retirement, and reconciliation control turns; capability
negotiation alone still proves none of their outcomes.

The current mechanism hypothesis is:

```text
begin a revision-bound preparation fence
→ ask the target model to attest its exact context-window identity
→ capture complete raw thread snapshot S and semantic projection C
→ compile and verify C
→ atomically publish checkpoint H
→ reread raw thread snapshot S'
→ if S' differs, retain the delta and recompile; repeat until stable
→ promote preparation to the final transition lease
→ start a semantically sterile retirement control turn
→ target model calls new_context as its only action
→ host observes and classifies the transition
→ host supplies H to the fresh window
→ successor reconciles against H and canonical references
```

The target model receives two small, sterile model-facing contracts. The first
attests the exact runtime context-window identity before the host snapshots the
thread. It performs no domain work and invokes no tools. The second is the
retirement actuator contract:

> When the runtime supplies a `context_retirement_ready` directive and no later
> human input is present, perform no domain work and invoke `new_context` as the
> only action. If later human input is present or domain work has already
> occurred after the directive, do not invoke `new_context`.

The lifecycle service, not the target model, owns validation that the directive
is revision-bound, published, unblocked, and still current before delivery.

This exact mechanism is not permanent doctrine. If a supported host-callable
fresh-context capability becomes available, the adapter may replace the
model-actuated route while preserving the readiness, fencing, authority, and
rehydration invariants.

The retirement control turn must be sterile. Any new domain output, tool effect,
human input, or continuation-relevant meaning invalidates the readiness subject
and requires a new compilation. The lifecycle ledger should retain the accepted
checkpoint and predecessor evidence until successor reconciliation succeeds.

Final revision comparison and actuator delivery must occur under one
revision-bound host transition lease. While that lease is active, new domain
turns and tool effects cannot enter the retiring revision, the runtime binding
cannot change, and only the sterile actuator turn may proceed. Human input must
either revoke the lease before actuation or remain outside the retiring
revision for explicit successor reconciliation; it cannot race silently with
the actuator turn.

> **Transition-lease invariant:** Retirement readiness and actuator delivery
> occur under one revision-bound transition lease. Any competing input, effect,
> or runtime-binding change revokes the lease before actuation.

The implemented transition-lease foundation first opens a preparation fence
and delivers one exact identity-attestation turn to the target model. The host
uses `thread/read` to resolve the rollout path, reads one bounded immutable
JSONL snapshot, and applies the pinned Codex compaction-aware replay-selection
rules to reconstruct the effective context. The raw effective-context revision
is bound separately from the smaller attributed `thread/turns/list` projection
used as semantic compiler material. Telemetry-only rollout additions therefore
change source evidence without falsely changing effective-context identity.

After accepted compilation, verification, and publication, the host rereads
the complete raw snapshot immediately before actuator delivery. A changed
revision invalidates readiness, preserves the newly observed snapshot, and
returns the preparation to recompilation without admitting the delta to the
retiring context. This fixed-point loop prevents meaning that arrived after the
first fence from being discarded or silently bypassing compilation.

The promoted transition lease consumes the complete checkpoint,
its publication-ledger entry, and the exact post-publication source, binding,
authority, checkpoint, and ledger fence. It independently verifies their digest,
linkage, and subject continuity before appending an accepted readiness entry and
issuing an immutable lease revision. The in-process gate then serializes adapter
turn admission, dynamic-tool effects, and role-binding mutation for that logical
role.

Before actuator delivery, a competing domain turn or binding mutation revokes
readiness before it proceeds. A competing dynamic-tool effect is denied and
revokes readiness. Once retirement delivery begins, competing domain turns,
tool effects, and binding mutations remain outside the retiring revision. The
adapter admits only the exact deterministic `context_retirement_ready` text,
with no skill input, request-context item, or newly supplied dynamic-tool
bridge. It requires the negotiated target-model context-replacement capability
and records `actuation_requested` as `attempted` before sending `turn/start`.
Delivery failure records a failed ledger entry rather than success evidence.

This reference gate is in-memory and provides only one-process serialization.
It does not provide a protected durable lease or yet route every provider-native
input path through the controller. The pinned live test does prove one
model-invoked `new_context` transition and successor reconciliation, but that
does not authenticate arbitrary provider signals or make unmediated ingress
safe.

The implemented controller boundary closes durable admission for the logical
role inside preparation admission, before the preparation becomes visible.
Later switchboard messages are stored in arrival order with exact attribution,
binding, transition, content, and client-message identity. Accepted
reconciliation releases them sequentially through the ordinary idempotent role
delivery path, records each delivery receipt, and reopens admission only after
no unreceipted item remains. Failure or restart leaves admission closed and
resumes at the first unreceipted identity; unresolved reconciliation releases
nothing. Pinned Codex CLI 0.149.1 exposes
durable thread-queue requests (`thread/queue/add`, `list`, `update`, `delete`,
`reorder`, and `start`) but no client request that pauses a thread. The Work
Engine controller therefore owns the pause semantics. The operator switchboard
path is mediated, and the executable boundary rejects known direct provider
turn, steer, injected-item, queue, compact, and realtime mutation routes for
role-owned threads. A future provider protocol revision can still introduce a
new ingress method, so the pinned method inventory and the read-only
`canAcceptDirectInput` field are not universal pause proofs.

`thread/compact/start` must not be treated as equivalent to `new_context`
without version-pinned live evidence of its actual semantics. Likewise, an item
or event named `contextCompaction` does not by itself prove whether the provider
summarized, replaced, or otherwise transformed context. The current live proof
therefore combines same-thread identity, a distinct successor context-window
identifier, exact predecessor linkage, post-transition request-context
visibility, and App Server transition notifications.

The implemented event-driven boundary subscribes before retirement delivery,
so the slow model-side round does not require lifecycle polling and a fast
notification cannot race subscription. It accepts only a pinned compaction
signal whose thread and turn match the exact retirement delivery. That signal
records `transition_observed: unresolved` and authorizes one reconciliation
challenge; it does not record successful replacement. The runtime also awaits
the retirement turn's terminal notification before starting rehydration.

## Rehydration

After observing a fresh window, the host should supply the accepted checkpoint
and the references required to reload governing instructions, activated skills,
authority, and role-specific state. The successor should not receive the prior
transcript merely to recreate meaning already compiled durably.

Reconciliation should establish:

- the same logical role instance remains bound as intended;
- the exact checkpoint revision was loaded;
- governing instructions and skills are applicable;
- authority resolves against canonical sources;
- open human interactions and commitments remain live;
- the authorized next action is still valid; and
- no transition uncertainty is being represented as success.

The prior context need not remain concurrently runnable, but its revision,
checkpoint, source references, and transition evidence must remain recoverable
until reconciliation is accepted.

The in-process reference gate now binds the predecessor context-window ID into
the lease before actuation. After the matching transition signal, it injects
the complete published checkpoint and a lease/checkpoint/thread/nonce challenge
through request context while reloading the caller-supplied exact skill set and
denying tools. The successor returns a closed receipt. Host validation requires
the exact immutable predecessor ID, a distinct non-empty current window ID,
the exact challenge bindings, all required continuation claims established, and
no reported uncertainty. Malformed, mismatched, false, or uncertain receipts
remain `unreconciled`; only an accepted receipt releases domain turns, tool
effects, and binding changes. When durable input custody is configured,
acceptance also starts ordered release through the same immutable role
environment and stable client-message identities; a release failure preserves
the remaining queue and keeps admission closed.

This receipt is a model assertion checked against host-owned bindings, not an
authenticated provider context-window attestation. A gated live test now proves
that pinned Codex CLI 0.149.1 exposes the required predecessor/successor lineage
for one strategic-planner transition and that the successor reconciles the
published checkpoint before the gate reopens.

Until the provider supports a concurrently testable successor or a reversible
context transition, this is a recoverable semantic handoff rather than atomic
two-phase commit. Reconciliation failure can reconstruct continuation from the
checkpoint and predecessor evidence, but it cannot restore the exact retired
model context. The ledger must represent that limitation and any resulting
repair or uncertainty instead of claiming rollback.

## Optimization objective and telemetry

The shared service creates one optimization surface across all Work Engine
roles:

> Minimize total lifecycle cost while preserving correct continuation,
> authority, uncertainty, governing environment, and unresolved human meaning.

Measurements should include:

- retained-context input tokens and latency;
- context growth rate and reported utilization;
- compiler and verifier input/output tokens, latency, and cost;
- checkpoint size and durable-write cost;
- retirement attempts, cancellations, and primary causes;
- rehydration tokens and latency;
- time until productive work resumes;
- rediscovery, repair, and canonical reread cost;
- missed obligations, distorted authority, or reopened decisions;
- semantic-verifier findings and target-role corrections when sampled; and
- final task correctness and review outcomes.

Thresholds and inspection timing are optimization knobs, not product doctrine.
The safety and authority invariants bound the optimization space.

The implemented shadow foundation now records one integrity-bound lifecycle
episode for every supplied pressure observation, including observations for
which semantic inspection is not scheduled. An explicit, revision-bound shadow
schedule selects the pressure dispositions that invoke the existing compiler
and verifier and whether an accepted result is offered to the checkpoint
publisher. The coordinator contains no transition lease, retirement directive,
or context-replacement capability, and every receipt records
`retirementAttempted: false`.

Each episode binds the logical role and runtime binding, pressure observation
and policy, shadow schedule, measurement source, observed-context revision when
present, semantic and checkpoint outcomes, failure stage, and available cost
measurements. Inference providers may report input, cached-input, output-token,
and cost observations; the host independently measures inference and
publication duration and checkpoint bytes. Retained-context and avoided-token
estimates remain nullable and require an identified estimation method and
revision. Missing telemetry remains missing rather than being interpreted as
zero.

The reference episode store remains available in-memory, and the same injected
boundary now has a restart-safe SQLite implementation. The SQLite adapter uses
a versioned schema and one explicit caller-owned database path. It stores
immutable episode payloads under unique episode and content revisions, making
exact replay idempotent across connections while rejecting changed evidence.
Deterministic summary projection refuses to combine different pressure-policy
or schedule revisions and reports both measurement totals and observed/missing
counts. This creates the evidence surface for a future feedback loop without
granting the statistics authority to change policy:

```text
revision-bound lifecycle episodes
→ coverage-qualified aggregate outcomes
→ compare explicit policy revisions
→ propose threshold or scheduling changes
→ shadow-test the new revision
→ authorized configuration adoption
```

This loop may optimize when lifecycle work begins and whether its expected cost
is justified. It cannot weaken semantic verification, authority preservation,
checkpoint freshness, quarantine, or reconciliation because those are outside
the optimization surface.

The same SQLite transaction boundary stores a checkpoint publication and its
ledger evidence, advances the lifecycle-ledger head, and conditionally advances
the complete source, binding, authority, checkpoint, and ledger fence. A
separate compare-and-swap operation can advance source, binding, or authority
state while preserving the existing checkpoint and ledger heads, allowing a
later checkpoint to name its real predecessor. Restart reads revalidate stored
episode, checkpoint, and ledger revisions before returning them.

This is local process-independent durability, not a protected distributed
ledger. WAL mode, foreign keys, `FULL` synchronous writes, a busy timeout, and
file mode `0600` provide a practical single-host base. They do not authenticate
writers, encrypt the database, fence another host, prove backup completeness,
or make SQLite the owner of canonical claims. Node 22 also labels its built-in
SQLite API experimental; the storage contract isolates that driver choice.

### Configurable pressure scheduling and hysteresis

The host may project measured context pressure into four scheduling
dispositions:

```text
comfortable
  lifecycle inference has no pressure-derived scheduling value

approaching
  bounded replaceability and compilation-cost estimation may be worthwhile

replacement_candidate
  checkpoint compilation and verification have credible scheduling value

critical
  preservation and an already-safe transition take scheduling priority
```

These dispositions describe runtime scheduling pressure. They do not prescribe
model behavior, determine what meaning must be preserved, authorize retirement,
or make a checkpoint valid. Pressure remains evidence alongside semantic value,
reconstructability, authority, uncertainty, and cost. The lifecycle service
must not ask ordinary roles to monitor or classify their own pressure.

Each boundary has separately configurable entry and exit values. A wider exit
band provides hysteresis, preventing repeated assessment when noisy utilization
hovers near one boundary. Implementations may also configure measurement
sources, minimum observation intervals, estimation cooldowns, and retry timing.
Initial values are experimental profiles to be tuned through real workloads,
not defaults promoted into doctrine.

Pressure scheduling, checkpoint validity, and transition progress remain
orthogonal state:

```text
pressure disposition
  comfortable | approaching | replacement_candidate | critical

checkpoint disposition
  absent | compiling | rejected | accepted | stale

transition disposition
  idle | ready | actuating | unreconciled | reconciled
```

Hysteresis applies only to pressure-derived scheduling. A semantic, authority,
human-interaction, source, or runtime-binding revision that overtakes a
checkpoint invalidates readiness immediately. Falling pressure cannot make a
stale checkpoint current, and critical pressure cannot convert unresolved
continuation into accepted continuation.

The pressure controller should therefore be deterministic and policy-driven.
It consumes a host-supplied utilization observation and returns a pressure
disposition plus transition evidence. It does not invoke inference, publish a
checkpoint, authorize retirement, or claim that one provider telemetry field
is an exact active-context measurement. Provider adapters or configured
measurement profiles own that interpretation.

The first Codex measurement profile uses the latest completed turn's
`last.totalTokens / modelContextWindow`, floors the ratio to basis points, and
clamps it at 10000. Its profile and source observation are digest-bound in the
pressure evidence. This is an intentionally replaceable scheduling proxy, not
an assertion that `last.totalTokens` is exact active-context occupancy. A
missing context-window value produces unavailable pressure evidence.

The retained-role shadow wrapper returns the admitted delivery immediately,
then its completion path joins only a token-usage observation carrying that
exact turn ID with the durable role binding and role-scoped coordinator. This
keeps the operator's UI turn live while lifecycle work follows provider turn
completion. Stale usage is not applied to the new turn. Comfortable
observations can be persisted without invoking semantic inference; when the
schedule requires semantic inspection, the host must supply the authenticated
projection and exact bounded source materials rather than asking the wrapper
to infer a source inventory.

Process restart must restore pressure scheduling and evidence ordering
together. The runtime derives the global pressure-observation sequence floor
from all integrity-checked durable episodes and starts the notification collector above
it. For each role, it restores the latest disposition only when the durable
episode's pressure-policy revision matches the configured policy. Policy
change resets the disposition to `comfortable` while retaining the sequence
floor. This avoids both loss of hysteresis and silent reuse of an old sequence
as new evidence.

Host assembly must preserve that ordering: open and validate durable state,
derive the sequence floor, construct the collector, subscribe to provider
notifications, and only then admit retained-role turns. Per-role coordinators
are constructed lazily from the configured policy and the latest compatible
durable disposition. In the App Server Host, the lifecycle implementation and
profile are generation-bound, while the SQLite path is stable controller state
outside immutable generation snapshots. Candidate workers cannot observe or
write role turns before activation; predecessor disposal closes its connection
after admitted work drains. The host owns notification detachment and database
closure.

At a semantic-inspection threshold, manifest-role projection assembly consumes
explicitly classified visible materials rather than inferring trust from
content. It reads the exact activated skill bytes and derives every digest,
binds the completed turn and runtime role projection, and emits the signed
observed-context projection together with the exact source-material closure.
The caller still owns completeness and omission declarations; the assembler
rejects caller-supplied digests and undeclared material fields.

### Critical preservation failure and human recovery

With the configured `token_budget` behavior, mandatory provider compaction may
replace the working window rather than preserve it through an opaque summary.
If pressure becomes critical while safe semantic continuation is unavailable,
another target-role turn can therefore destroy continuation-relevant meaning.
The service must not treat mandatory compaction as a fallback handoff.

At that boundary, the host quarantines the affected logical role: it stops new
model turns and effects from entering that context, preserves the unresolved
failure evidence, and surfaces the recovery decision outside the pressured
role. Critical scheduling may prioritize compilation or an already-ready
transition, but it never weakens compilation, verification, publication,
authority, or reconciliation requirements.

Notification is a required consequence but need not be bound to a channel. An
ephemeral notification inference may receive the bounded failure evidence and
a notification skill that describes the need to reach the user without naming
SMS, email, webhooks, or another implementation. Codex capability discovery can
then expose whichever authorized notification interfaces the user already
configured. The lifecycle service does not acquire the user's notification
credentials or duplicate that configuration.

The notification outcome must be observable. A successful delivery retains its
delivery receipt. If no notification interface is available, an authorized
local capability may preserve a pending message as an artifact whose path and
digest are returned to the host. Platform-local notifications are optional
affordances rather than lifecycle doctrine. A tool attempt without a delivery
receipt or retrievable pending artifact is not a successful notification.

The user may recover conservatively by exporting the visible model history to a
temporary artifact before authorizing context clearing. The host binds that
artifact by reference and digest, retains its confidentiality and lifetime
constraints, and supplies it to the successor alongside the accepted checkpoint
and canonical references. Reconciliation must distinguish recovered transcript
material from canonical state and unresolved meaning. The export neither
captures hidden reasoning or provider-only instructions nor grants its content
new authority. It is human-supplied recovery evidence, not restoration of the
exact retired model state.

The notification channel, pressure values, cooldowns, retry timing, recovery
artifact location, and retention period are configurable. Quarantine on unsafe
critical exhaustion, truthful failure evidence, user authority over recovery,
and fail-closed reconciliation are not.

## Invariants

1. Correct continuation must not depend on meaning represented only in the
   retiring context.
2. A compiled checkpoint cannot create, widen, transfer, extend, or reactivate
   authority.
3. Human ambiguity, preference, approval, or obligation cannot be guessed into
   closure.
4. Governing instructions and activated skill state are continuation state when
   later work depends on them.
5. Retirement must bind the exact observed source and fail when a later semantic
   delta overtakes it.
6. Preservation pressure and retirement optimality remain separate judgments.
7. Provider event names and requested operations do not establish transition
   semantics without observed evidence.
8. Opaque provider summaries cannot become the sole owner of required
   continuation meaning.
9. The target model may actuate `new_context`, but it does not thereby acquire
   lifecycle-state ownership or retirement authority.
10. Excluding an interaction from a new context does not delete its history,
    resolve its meaning, or remove its authority.
11. A fresh window must reconcile the exact checkpoint before productive domain
    work resumes.
12. Failed compilation, verification, publication, actuation, transition, or
    reconciliation must remain visibly failed or unresolved.
13. Model-visible content does not gain instruction or authority status merely
    through inclusion in an observed-context projection.
14. Projection construction, destination writes, and retained copies remain
    inside authenticated, least-privilege, confidentiality-aware host
    boundaries.
15. Final readiness comparison and actuator delivery share one revision-bound
    transition lease that competing input, effects, or binding changes revoke.
16. Recovery claims semantic continuation from durable evidence, not restoration
    of the exact retired model context.
17. Pressure thresholds may schedule lifecycle work but cannot establish
    semantic readiness or relax a failed preservation boundary.
18. When another target-role turn risks destructive clearing and safe
    continuation is unavailable, the affected role remains quarantined until an
    authorized recovery path is established.
19. Notification delivery must be evidenced by a delivery receipt or an
    observable pending-message artifact; an attempted route is not delivery.

## App Server architecture manifest projection

The future App Server architecture manifest can describe this as a service
rather than a role:

```yaml
services:
  semantic-context-lifecycle-manager:
    implementation: services/semantic-context-lifecycle-manager.mjs
    applies_to: work-engine-root-roles

    observes:
      - thread-events
      - turn-items
      - token-usage
      - context-transition-items

    semantic_compiler:
      kind: ephemeral-inference
      output_schema: continuation-state-v1
      retains_context: false

    semantic_verifier:
      kind: ephemeral-inference
      retains_context: false

    ledger:
      schema: context-lifecycle-ledger-v1
      storage: external-durable-state

    transition:
      capability: new_context
      invocation: target-model
      feature_gate: required
      expected_revision_fence: required
      fail_closed: true
```

This manifest describes runtime composition. Role contracts continue to own
role behavior, and the provider adapter continues to own version-specific App
Server mechanics.

## Current evidence gaps

The current scaffold does not yet prove the complete lifecycle:

- the pinned protocol exposes persisted turns and token telemetry but not an
  exact effective-context read;
- `new_context` is not a pinned direct App Server client request;
- the adapter now fail-closed negotiates a provider-neutral
  `model_context_replacement` capability only for the pinned target-model
  mechanism with explicit `token_budget` configuration, prevents capability
  widening after initialization, and binds the selected provider profile to the
  retained role thread; the configuration declaration is not a provider
  handshake or transition-success claim;
- an opt-in live integration probe against pinned Codex CLI 0.149.1 enabled the
  `token_budget` feature, invoked model-side `new_context`, preserved the exact
  App Server thread id, observed a distinct successor context-window id with
  exact predecessor linkage, delivered a fresh request-context marker, and
  observed context-compaction plus token-usage notifications;
- those combined observations establish the bounded same-thread replacement
  route for this pinned version, but they do not authenticate a production
  checkpoint or classify every other compaction path;
- the adapter boundary now normalizes pinned token-usage and compaction
  notifications into immutable, bounded, provider-neutral lifecycle
  observations; compaction observations remain explicitly unclassified and the
  run-local collector is not a durable lifecycle ledger;
- the host can now construct and independently verify a signed
  `observed-context-v1` projection binding attributed item references, trust and
  instruction applicability, governing sources, exact skill digests, runtime
  binding, lifecycle evidence, expected next work, omissions, and mandatory
  provider unknowns; key custody and durable ledger anchoring remain unbuilt;
- the host can now validate and revision-bind a closed
  `continuation-state-v1` compiler candidate, including exact authority and
  source references and explicit human-interaction status and loading
  disposition;
- the host can now run a provider-neutral compiler and distinct verifier
  against recorded bounded fixtures, rejecting unprojected or digest-mismatched
  source material, binding host-owned provenance and revisions, requiring five
  cited verification checks, and deriving the verification disposition in
  code;
- a gated live executable-host test now routes one strategic-planner turn
  through the generation-bound pressure controller, semantic compiler, and
  distinct verifier, persists one non-failed replacement-candidate episode,
  closes every generation admission, and proves that checkpoint publication
  and transition remain unrequested; this establishes the live inference path,
  not provider/context independence or representative live-model quality;
- the host now preserves human-interaction status independently from loading
  disposition, applies only the irreducible fail-closed combinations above,
  requires one cited verifier evaluation per interaction, and derives the
  aggregate interaction check without treating omission, assistant output, or
  escalation as resolution;
- the host can now construct and verify storage-neutral lifecycle-ledger
  records with closed event/status vocabulary and a tamper-evident predecessor
  chain; authenticated writing, protected durable storage, completeness, and
  authoritative ledger recovery remain unbuilt;
- the host can now publish a complete, accepted checkpoint and its publication
  ledger evidence through one revision-fenced compare-and-swap boundary after
  exact authority revalidation; the included store is in-memory and does not
  provide protected durable storage, authenticated writing, or cross-process
  serialization;
- the host can now acquire an in-process revision-bound transition lease and
  deliver one exact sterile retirement control turn while gating adapter domain
  turns, dynamic-tool effects, and role-binding mutations; preparation now
  precedes compilation, binds a target-model identity receipt and a complete
  persisted-thread snapshot, and requires a final identical snapshot before
  actuation;
- the same in-process gate now subscribes before retirement, uses an exact
  pinned compaction notification as an event-driven wake-up, records the signal
  as unresolved, injects one exact checkpoint-bound reconciliation challenge,
  validates predecessor/successor window lineage and continuation claims, and
  releases domain work only after acceptance; a gated live strategic-planner
  test has completed this full publication, model-requested clearing, and
  reconciliation path; protected durable leasing, controller-owned durable
  queuing for every ingress route, and authenticated window identity remain
  unbuilt;
- the SQLite state adapter now owns revision-bound input admission and an
  ordered durable custody queue. Preparation closes admission atomically with
  its in-process role lock, the operator switchboard acknowledges post-fence
  input without delivering it to the predecessor, accepted reconciliation
  releases each stable client-message identity through normal binding checks,
  and restart resumes from the first unreceipted item. Known direct provider
  mutation requests for role-owned threads are rejected before forwarding;
  protocol-version inventory remains necessary to detect future ingress;
- the host can now combine an explicit pressure policy and shadow schedule with
  the existing semantic inference and checkpoint-publication boundaries, emit
  integrity-bound per-observation episode receipts, and summarize only
  revision-homogeneous measurements with missing-data coverage; the lightweight
  reference store remains in-memory and no policy is tuned or adopted
  automatically;
- the host now provides a schema-migrated SQLite adapter that preserves episode
  identity across restarts and connections, atomically stores checkpoint plus
  ledger evidence while advancing their revision fence, restores and verifies
  current heads, and rejects stale competing writers; role bindings, transition
  leases, outcome annotations, backup/restore, authenticated writers,
  encryption, and multi-host coordination remain unbuilt;
- the adapter now compiles request-bound evidence into a deterministic text
  item inside the pinned `TurnStartParams.input` field; a live temporary
  manifest role returned a runtime-random context value, and the production
  strategic-planner role returned a version-1 handoff that passed exact
  objective, evidence-cutoff, continuity, schema, and terminal validation; and
- these live turns establish request transport, the first planning handoff, and
  one bounded same-thread checkpoint publication, context-window transition,
  and reconciliation. The host now reconstructs a pinned compaction-aware
  effective-context inventory from the rollout, but does not claim an
  authenticated provider prompt, cross-process transition serialization, or a
  production policy for retirement.

These are implementation premises to test, not reasons to weaken the semantic
contract.

## Implementation sequence

The sequence below is a planning hypothesis. Evidence may justify revising it
while every invariant remains preserved.

1. **Completed foundation evidence:** prove one live request-bound planning
   handoff using the pinned model-visible request-context input implemented by
   the adapter.
2. **Completed foundation evidence:** prove the pinned model-side `new_context`
   route replaces a context window without replacing the retained App Server
   thread and that a new request-context value is visible afterward.
3. **Completed foundation implementation:** normalize pinned token telemetry and
   provider transition signals into bounded provider-neutral lifecycle
   observations without treating signal names as semantic proof.
4. **Completed foundation implementation:** construct and independently verify
   an authenticated `observed-context-v1` projection while preserving explicit
   completeness limits and provider unknowns.
5. **Completed foundation implementation:** define and validate
   `continuation-state-v1` candidates and storage-neutral, tamper-evident
   lifecycle-ledger records without confusing validation with publication or
   durable authority.
6. **Completed foundation implementation:** run a hidden provider-neutral
   semantic compiler and distinct verifier against recorded bounded fixtures,
   with host-owned source admission, provenance, revision binding, and
   disposition derivation. This does not enable retirement.
7. **Completed foundation implementation:** evaluate human-interaction closure
   and loading disposition as separate, source-bound dimensions, with
   host-derived aggregate status and fail-closed ambiguity or escalation.
8. **Completed foundation implementation:** capability-gate the proven
   model-side `new_context` route in the production adapter through a pinned,
   explicitly configured, target-model provider profile without treating
   configuration as transition evidence.
9. **Completed foundation implementation:** publish an accepted checkpoint and
   its ledger evidence through an atomic source-, binding-, authority-,
   checkpoint-, and ledger-revision fence without enabling retirement.
10. **Completed foundation implementation:** acquire an in-process
    revision-bound transition lease and deliver an exact sterile retirement
    control turn without claiming model actuation or transition success.
11. **Completed foundation implementation:** subscribe before retirement, use
   the pinned compaction notification as a no-polling wake-up without treating
   it as semantic proof, inject one exact checkpoint, and mechanically gate an
   exact predecessor-linked reconciliation receipt before releasing domain
   work.
12. **Completed foundation evidence:** prove live checkpoint publication,
   model-requested clearing, injection, and semantic reconciliation on one
   strategic-planner thread under a stable raw snapshot.
13. **Completed foundation implementation:** provide a deterministic
   configurable pressure-disposition controller with hysteresis, without
   binding telemetry interpretation or retirement policy.
14. **Completed foundation implementation:** compose pressure scheduling,
   semantic inspection, optional accepted-checkpoint publication, and
   integrity-bound episode telemetry in a shadow coordinator from which
   retirement is mechanically absent.
15. **Completed foundation implementation:** persist shadow episodes,
   checkpoint publications, lifecycle-ledger heads, and revision fences through
   a schema-migrated transactional SQLite adapter with restart and competing
   writer evidence.
16. **Completed foundation implementation:** route completed hosted
   manifest-role turns through the configured Codex pressure profile and
   durable shadow coordinator. The App Server Host returns role delivery before
   lifecycle completion, joins exact-turn token telemetry, stores episodes in
   stable SQLite state across executable-generation replacement, avoids
   inference for comfortable turns, and keeps checkpoint publication and
   retirement mechanically unavailable. Authenticated manifest-role projection
   assembly and separate ephemeral Codex compiler and verifier capabilities
   have deterministic coverage.
17. **Completed foundation evidence:** execute a gated live strategic-planner
   shadow inspection through the executable host without clearing context. The
   persisted episode binds a non-failed compiler/verifier result to the
   replacement-candidate observation while checkpoint publication and
   transition remain unrequested. Revision-homogeneous outcome comparison
   remains future tuning work.
18. **Completed foundation implementation:** bind preparation to durable input
   admission, queue post-fence switchboard messages outside the predecessor
   revision, and release them in order through idempotent role delivery only
   after accepted reconciliation. Restart recovery and delivery receipts are
   covered. Known provider-native mutation routes to role-owned threads are
   rejected at the executable boundary while emergency interruption remains
   available.
19. **Completed foundation implementation:** compose a live coordinator and
   retained-role host that order preparation, target-window attestation,
   conservative full thread-item projection, distinct semantic inspection,
   checkpoint publication, lease promotion, model-requested clearing, and
   reconciliation. Visible turn completion remains pending through the
   lifecycle, and concurrent execution of one episode is deduplicated. This
   generic host is not yet selected by the shadow-only executable profile.
20. Add an explicit live runtime profile, initialize and advance its durable
   publication fence under canonical authority revalidation, select the live
   host in the executable proxy, and execute one gated proxy transition before
   enabling ordinary development inside it.

## Relationship to neighboring documents

- [`ideas/01-semantic-context-manager-proposal.md`](../../ideas/01-semantic-context-manager-proposal.md)
  owns the broader semantic-context reasoning and historical proposal lineage.
- [`codex-agents.md`](codex-agents.md) records the observed shared-daemon and
  operator-console topology that makes independently controlled root roles
  practical.
- [`../README.md`](../README.md) describes the current implemented App Server
  boundary rather than this future lifecycle plan.
- [`ideas/priority-one-new-runtime-direction.md`](../../ideas/priority-one-new-runtime-direction.md)
  remains the user-authored migration direction and is referenced, not modified,
  by this plan.

## Open decisions

- Which component owns final retirement authorization once compiler and verifier
  agree?
- Which model/provider and freshness contract should compile and verify state?
- Which subset of the proven transition signals must a production lifecycle
  ledger retain, and how should their absence or disagreement be classified?
- How should pending post-fence input custody be exposed to the operator while
  preserving its exact ordering and delivery state?
- Which interaction classes require exact source loading rather than a compiled
  consequence?
- How are role-specific continuation schemas registered and versioned?
- Which host-owned telemetry projection best estimates active-context pressure
  for each provider profile?
- Which initial pressure bands, cooldowns, and retry timing should shadow mode
  test before real-world tuning?
- How should later correctness, repair, and productive-resumption outcomes be
  appended to immutable SQLite lifecycle episodes without rewriting them?
- What evidence and authority boundary promotes a shadow comparison into a new
  active pressure-policy or scheduling revision?
- After unresolved preservation reaches critical pressure, when should the
  supervisor retry bounded inference versus immediately request human recovery?
- What closed receipt distinguishes delivered notification from a retrievable
  pending-message artifact, and where should the fallback artifact live?
- How long should a human-exported recovery transcript remain available after
  reconciliation?
- How long must predecessor checkpoints and transition evidence be retained?
