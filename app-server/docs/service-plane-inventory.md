# Service-Plane Inventory

## Status

Evidence-backed baseline. This document records what currently exists under
`app-server/src/services/`. It does not propose a kernel/service/workflow
plane split, a `ServiceOperation` schema, a capability/service-operation
rename, or any implementation change — those remain exploratory and are
tracked separately (see Relationships).

Its purpose is narrow: `incremental-architecture-intake-and-seam-reconciliation.md`
requires every architecture intake session to bind to "one exact implemented
architecture baseline" (§3) before integrating prospective ideas. Root
`ARCHITECTURE.md` cannot supply that baseline for the service plane — it has
zero mentions of `app-server` anywhere in its text and describes the
pre-migration substrate exclusively. This document exists to fill that
specific gap, not to replace or amend `ARCHITECTURE.md`.

## Scope and method

- Evidence cutoff: repository revision `714741ad4b02157b38c2339931d176043d2e846b`
  (2026-09-11).
- Covers all 12 directories under `app-server/src/services/`.
- Classifies each distinct **service-like unit**, not each directory. A
  directory that bundles multiple unrelated concerns is split into multiple
  units; several were. That splitting is itself a finding, not just a
  formatting choice — see Synthesis.
- Each unit is scored against 13 dimensions: semantic owner; state
  relationship; operation identity; determinism; authority; inputs; outputs;
  evidence; recovery; workflow coupling; realization coupling; cross-service
  dependencies; coordinate relevance (what would have to be captured to
  reconstruct this unit's contribution to a historical world, per
  `revisioned-research-and-execution-architecture.md`'s coordinate concept).
- The state-relationship labels used below (state owner / derivation service /
  effect service / realization adapter / workflow coordinator / projection
  service) are audit labels for this pass. They are not a finalized ontology;
  several units resist a single label and are marked as such.
- Claims below were independently spot-checked against source at the file:line
  level where a specific line reference is given. Claims without a line
  reference are carried from the audit pass at directory/file granularity.

---

## Inventory

### `agent-instruction-review`

```text
unit: agent-instruction-review
semantic_owner: code domain
state_relationship: projects + admits; owns no state of its own
operation_identity: projectClosure(...) / admit({result, expectedSubject})
determinism: hybrid — closure projection is deterministic; the admitted
  review content is semantic
authority: none of its own; admit() carries an explicit all-false authority
  object
inputs: owned refs (role projection, subject)
outputs: closure projection + admitted result envelope
evidence: digest-bound closure revision + result revision
recovery: stateless; trivially idempotent
workflow_coupling: reusable operation
realization_coupling: none direct; consumes a runtime-manifest role projection
cross_service_deps: semantic dependency on implementation-review (imports its
  validator and digest function directly)
coordinate_relevance: clean — subject + closureRevision + resultRevision are
  sufficient to reconstruct this unit's contribution
```

### `implementation-review`

```text
unit: implementation-review
semantic_owner: code domain
state_relationship: admits/validates only; owns no state
operation_identity: admit({result, expectedSubject})
determinism: deterministic validation wrapping semantic content
authority: none; explicit all-false authority object, matching
  agent-instruction-review's pattern
inputs: copied result + expected subject
outputs: admitted envelope
evidence: resultRevision digest
recovery: stateless, trivially idempotent
workflow_coupling: reusable — consumed directly by agent-instruction-review
  and by review-episode
realization_coupling: none
cross_service_deps: is itself a real semantic dependency of two other units
  (not infrastructure reuse — both callers depend on its validation semantics)
coordinate_relevance: clean; digest + subject sufficient
```

### `review-episode`

```text
unit: review-episode
semantic_owner: workflow
state_relationship: state owner (durable episode lifecycle)
operation_identity: begin / transition / resumeInitial, over an action enum
determinism: deterministic transitions carrying semantic payload
authority: authority grant + writer-generation match; CAS via expectedRevision
inputs: owned identity refs
outputs: state transitions, full revision history
evidence: complete revision history retained in the store
recovery: idempotent via handledTransitions[transitionId]; writer-generation
  succession (replace_writer) for takeover; CAS revision conflicts fail closed
workflow_coupling: workflow-private review-lifecycle choreography
realization_coupling: none directly; store is injected (in-memory default,
  sqlite-store.mjs present for durability)
cross_service_deps: semantic dependency on implementation-review.admit
coordinate_relevance: strong — identity + revision + handledTransitions are
  already shaped for reconstruction
```

### `review-bench`

```text
unit: review-bench
semantic_owner: other (offline evaluation/tooling, explicitly non-production)
state_relationship: derives/projects over externally supplied artifacts; owns
  no state
operation_identity: method calls over typed artifacts (review_bench_*_v1/v2)
determinism: deterministic scoring/comparison; treats semantic review content
  as opaque input data
authority: explicit production_review_authority: false
inputs: copied/transient (corpus, truth, and result passed in by the caller)
outputs: validated + scored comparison artifacts
evidence: none durable — pure functions, no store
recovery: stateless
workflow_coupling: workflow-private, scoped to an experimental harness
realization_coupling: none
cross_service_deps: none
coordinate_relevance: not applicable by design — explicitly experimental,
  not meant to participate in production history
```

### `reviewer-runtime`

```text
unit: reviewer-runtime
semantic_owner: runtime
state_relationship: realization adapter
operation_identity: adapter dispatch (native-claude-code / openrouter-codex)
determinism: hybrid — adapter mechanics are deterministic; the realized
  judgment is semantic
authority: catalog/capability check before dispatch
inputs: profile + subject digest
outputs: execution receipt (configured/observed/isolation/transport/rawEvidence)
evidence: receipt is durable-evidence-shaped but not itself persisted by this
  unit
recovery: attemptId-scoped; no idempotency/replay logic visible in the
  contract itself
workflow_coupling: reusable — this is the concrete precursor
  `provider-turn-harness-runtime-and-operator-projection.md` §9 already names
  when it says reviewer runtime adapters "demonstrate more than one execution
  source, while the production native-review host still selects a fixed
  native Claude profile"
realization_coupling: direct — two hard-bound adapters (Claude Code,
  OpenRouter/Codex) selected by the caller, not by an admitted realization
cross_service_deps: consumed by the native-review host (see slice-campaign)
coordinate_relevance: receipt fields (attemptId, profileConfigurationDigest,
  subjectDigest) are already coordinate-shaped
```

### `review-subject`

```text
unit: review-subject
semantic_owner: code domain
state_relationship: mediates only — owns no state of its own (confirmed
  directly against contract.mjs, legacy-backend-adapter.mjs, and service.mjs
  earlier this session)
operation_identity: create_candidate / transition_candidate /
  validate_checkpoint / create_physical_profile / validate_physical_profile
determinism: deterministic; delegates to the legacy Python analyzer,
  digest-verified before every invocation
authority: digest-verified backend identity (checkpoint validator source +
  analyzer source) checked before each call
  (app-server/src/services/review-subject/legacy-backend-adapter.mjs:57-106)
inputs: owned subject refs
outputs: candidate/profile envelopes
evidence: full digest chain — backend_sha256, subject digest, profile digest
recovery: stateless per invocation; the operation derives, it never mutates,
  so no CAS is needed
workflow_coupling: reusable — no workflow coupling anywhere in its own
  contract
realization_coupling: adapter boundary (shells to Python; no in-process
  coupling)
cross_service_deps: none
coordinate_relevance: excellent — the cleanest unit in this inventory
```

### `claim-evidence` — ledger core (`service.mjs`, `identity.mjs`, `contract.mjs`)

```text
unit: claim-evidence (ledger core)
semantic_owner: kernel-adjacent (see Synthesis)
state_relationship: state owner
operation_identity: one generic envelope —
  {operation_id, action, profile, expected_state, payload}, action drawn from
  a fixed permission set (app-server/src/services/claim-evidence/service.mjs:51)
determinism: deterministic ledger mechanics; semantic content lives entirely
  inside payload
authority: authority grant checked against PROFILES and decision_scope,
  digest-pinned grant
inputs: owned refs (claim_id, predecessor revision)
outputs: revision/lineage/reliance state transitions
evidence: append-only revisions + lineage + operations log
recovery: idempotent by operation_id + payload digest — replaying the same
  operation_id returns the cached result rather than re-applying
  (app-server/src/services/claim-evidence/service.mjs:57); CAS via
  expected_state / revisionHeads
  (app-server/src/services/claim-evidence/service.mjs:83)
workflow_coupling: already multi-domain — three profiles share one ledger
  mechanism: proposal-research-v1, revision-bound-review-finding-v1,
  production-path-v1 (app-server/src/services/claim-evidence/contract.mjs:4-7)
realization_coupling: none in the core; profile-specific consumers exist
  (production-path-service.mjs, review-finding-bridge.mjs)
cross_service_deps: none in the core itself
coordinate_relevance: the strongest reconstruction shape found in the
  inventory — claim_id / revision_id / operation_id is already exactly a
  service-state reference
```

**This is the load-bearing finding of the whole audit.** A generic,
domain-neutral service-operation envelope is not hypothetical — it already
exists, already runs three unrelated domains through one mechanism, and
already has real idempotency and CAS semantics. It is a stronger existing
precedent than the `slice-campaign` capability-dispatch envelope described
below, precisely because it is already proven across more than one consumer.

### `claim-evidence` — Git-checkpoint observation

```text
unit: claim-evidence/git-checkpoint-observation
semantic_owner: code domain
state_relationship: derivation/observation service over state owned by
  slice-checkpoint
operation_identity: observeGitCheckpoint(...)
determinism: deterministic
authority: none of its own; re-verifies commit/tree identity via Git directly
inputs: owned refs (expected commit/tree)
outputs: normalized observation record
evidence: artifact.digest, event_identity
recovery: stateless
workflow_coupling: reusable
realization_coupling: direct Git subprocess coupling (adapter-shaped, not
  injected)
cross_service_deps: feeds the claim-evidence ledger as an evidence payload
coordinate_relevance: clean — the observation record already carries
  commit/tree/digest
```

### `operational-coordination`

```text
unit: operational-coordination (chatboard adapter)
semantic_owner: kernel-adjacent (domain-neutral coordination)
state_relationship: realization adapter over an external Python "board"
  primitive
operation_identity: execute(operation, input), operation in
  {read, claim, post, release}
determinism: deterministic dispatch; message content is workflow-authored
  (semantic)
authority: self-declared authority: "advisory_coordination_only" — explicitly
  non-binding
inputs: copied input
outputs: board mutation result
evidence: a board revision field
recovery: has its own replay path (recoverReplay re-reads board state after a
  failed call) — notably heuristic, matched by an agent_path regex, not backed
  by a durable CAS index the way review-episode or claim-evidence are
workflow_coupling: reusable — resource/claim/message concepts are not
  code-specific
realization_coupling: adapter boundary (shells to a Python script)
cross_service_deps: none
coordinate_relevance: revision + claim_id/message_id would be sufficient, but
  the recovery mechanism is weaker than review-episode's or claim-evidence's —
  worth strengthening before this unit is relied on for reconstruction
```

### `product-development` — artifact root

```text
unit: product-development/artifact-root
semantic_owner: kernel-adjacent
state_relationship: state owner (filesystem publication primitive)
operation_identity: publishCreateOnly, resolveExistingFile
determinism: deterministic
authority: path confinement (safeRelative / isWithin) plus create-only
  semantics
inputs: owned refs
outputs: published artifact bundle + digest
evidence: per-bundle digest (describeBundle)
recovery: create-only — cannot silently overwrite; full recovery semantics not
  fully exercised in this pass
workflow_coupling: reusable — nothing code-specific here
realization_coupling: filesystem plus execFile for legacy validators
cross_service_deps: wraps legacy idea_intake.py / proposal_packets.py —
  adapter boundary
coordinate_relevance: bundle digest is a clean coordinate reference
```

### `product-development` — delivery adapters

```text
unit: product-development/delivery-adapters (proposal/intake/claim-context)
semantic_owner: code domain (proposal-formation workflow)
state_relationship: realization adapter over legacy Python validators,
  composed with artifact-root
operation_identity: method calls; also exposes a third representation of
  "capability" — proposalCapabilityDefinitions, a dynamic-tool-manifest shape
  of {namespace, name, description, inputSchema}
  (app-server/src/services/product-development/proposal-delivery.mjs:97)
determinism: deterministic dispatch; validated content (proposal authorship)
  is semantic
authority: delegates to a legacy validator subprocess's exit status
inputs: copied file content
outputs: create-only publish via artifact-root
evidence: validator receipt embedded in the publish call
recovery: none beyond artifact-root's digest
workflow_coupling: workflow-private
realization_coupling: direct — hard-shells to python3 and specific script
  paths, no port abstraction
cross_service_deps: depends on artifact-root
coordinate_relevance: clean enough — publish operation_id + packet digest
```

### `skills-migration-integrity`

```text
unit: skills-migration-integrity
semantic_owner: other — migration-tactical, not really a service
state_relationship: pure derivation function; no store, no authority, no
  identity of its own
operation_identity: none — a bare exported function,
  classifySkillsMigrationIntegrity(...), called inline by
  slice-campaign/native-review-host.mjs
determinism: deterministic
authority: none — relies entirely on the caller's Git access
inputs: two Git revisions plus an accepted-paths list
outputs: classification receipt with a receiptDigest
evidence: the receipt is well-shaped (schemaVersion, receiptDigest) but never
  persisted independently — it is embedded inline into a review boundary
recovery: stateless, trivially idempotent
workflow_coupling: workflow-private choreography wearing a "service"
  directory name
realization_coupling: none
cross_service_deps: called directly by slice-campaign/native-review-host.mjs
coordinate_relevance: good digest shape, but no durable existence of its own
  — reconstructing it means re-deriving it, not looking anything up
```

### `workspace-coordination` — core

```text
unit: workspace-coordination (core: contract.mjs + service.mjs)
semantic_owner: kernel — the strongest existing kernel-shaped primitive found
  in this inventory
state_relationship: state owner (resource leases)
operation_identity: acquire / inspect / release / admitMutation /
  savePublication, over typed RESOURCE_TYPES
  (app-server/src/services/workspace-coordination/contract.mjs:4) which
  already include non-Git kinds: directory, git-ref, git-index, port, index,
  review-budget, database
determinism: deterministic
authority: fencing-token possession — admitMutation verifies the lease's
  generation before running the caller's mutation
  (app-server/src/services/workspace-coordination/service.mjs:11-29)
inputs: owned resource key
outputs: lease grant/denial, admission receipt
evidence: lease + admission records are revision-bound
recovery: monotonic fencing-token generations plus CAS-checked mutation
  admission — the cleanest recovery mechanism found in this inventory
workflow_coupling: reusable — resource typing already includes non-Git kinds,
  so this is domain-neutral coordination infrastructure today, not a
  code-domain adapter in disguise
realization_coupling: the mutation callback is caller-supplied (injected),
  not hard-bound to any implementation
cross_service_deps: consumed by slice-campaign
coordinate_relevance: excellent — resource key + fencingToken + leaseId is
  already a clean, generalizable coordinate fragment
```

### `workspace-coordination` — Git realization

```text
unit: workspace-coordination/git-realization (git-publisher.mjs, git-worktree.mjs)
semantic_owner: code domain
state_relationship: derivation/effect service consuming the core unit's leases
operation_identity: not fully read this pass — inferred from imports/usage;
  flagged for follow-up
determinism: deterministic
authority: delegated to the core unit's lease/fencing mechanism
inputs: owned refs
outputs: Git publication effects
evidence: Git commit/tree identities
recovery: relies on the core unit's fencing
workflow_coupling: coupled to the core unit
realization_coupling: direct Git subprocess — a code-domain-specific
  realization of a domain-neutral primitive
cross_service_deps: depends on workspace-coordination core
coordinate_relevance: not fully verified this pass
```

### `slice-campaign` — campaign state

```text
unit: slice-campaign/service.mjs (campaign lifecycle)
semantic_owner: workflow
state_relationship: state owner (campaign lifecycle) — but see Synthesis: it
  also transiently holds a candidate-to-candidate relationship it does not
  durably own
operation_identity: admit / bindCandidate / advance / bindReviewSelection /
  ... (app-server/src/services/slice-campaign/service.mjs)
determinism: deterministic transitions embedding semantic content (review
  results)
authority: CAS via expectedRevision, digest-based publish
inputs: consumes review-subject's candidate/profile refs
outputs: phase transitions
evidence: full campaign-state revision chain
recovery: CAS (publish(..., expectedRevision))
workflow_coupling: workflow-private — this unit *is* the workflow
realization_coupling: orchestrates review-subject, the native-review host,
  and legacy adapters — heavy, central coordinator
cross_service_deps: the heaviest cross-service dependency surface in the
  inventory
coordinate_relevance: known gap, already established this session —
  bindCandidate overwrites the predecessor candidate/physicalProfile on
  replacement (service.mjs:255-278) rather than retaining it. This audit
  re-derives the identical gap from a completely different angle
  (service-ownership analysis rather than history/coordinate analysis); two
  independent paths landing on the same seam is real corroboration, not
  coincidence.
```

### `slice-campaign` — native-review host

```text
unit: slice-campaign/native-review-host.mjs + native-review-closure.mjs
semantic_owner: mixed — code domain and workflow
state_relationship: workflow coordinator for native review execution;
  arguably deserves to be its own top-level unit rather than nested under
  "slice-campaign"
operation_identity: execute / recover / retry / correctResult /
  executeRemediation / ...
determinism: hybrid — boundary construction is deterministic; the review
  itself is semantic, realized through reviewer-runtime
authority: derives from campaign authority
inputs: owned refs (candidate, subject)
outputs: review boundary, obligations, remediation deltas
evidence: claim-evidence-backed findings (opens its own
  openSqliteClaimEvidenceStore)
recovery: obligation-status state machine (awaiting_builder / reported /
  remediation_executing / ...)
workflow_coupling: workflow-private
realization_coupling: consumes reviewer-runtime; calls
  classifySkillsMigrationIntegrity inline
cross_service_deps: depends on reviewer-runtime, review-subject,
  skills-migration-integrity, and claim-evidence
coordinate_relevance: this is exactly where a Candidate Trajectory-style
  remediation delta would plug in — already identified independently earlier
  this session
```

### `slice-campaign` — capability dispatch

```text
unit: slice-campaign/capability-contract.mjs + capability-host-runtime.mjs +
  host-effect-runtime.mjs
semantic_owner: should be kernel; is actually workflow-private — this is the
  proto-service-envelope identified earlier this session
state_relationship: dispatch/infrastructure layer, not a state owner itself
operation_identity: capability.<domain>/<operation> dispatch key, generic
  validate-input / validate-output / handler registration
determinism: not applicable — it is a dispatch layer wrapping both
  deterministic and semantic operations
authority: none beyond registration-time validation
inputs: request envelope
outputs: wrapped {schema_version, generation_id, capability, operation,
  result}
evidence: none of its own — evidence lives in whatever it dispatches to
recovery: none beyond the caller's own idempotency
workflow_coupling: named SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL — confirmed
  scoped and consumed only by this workflow's own executable-generation-*
  files; zero other consumers repository-wide
realization_coupling: not applicable
cross_service_deps: dispatches to the campaign-state unit, the native-review
  host, and legacy adapters
coordinate_relevance: not applicable — this is infrastructure, not itself a
  coordinate contributor
```

Compare this unit against the `claim-evidence` ledger core above: both are
generic dispatch envelopes, but only the ledger core has actually been proven
across more than one domain/consumer. This unit is the weaker of the two
existing precedents, not the only one.

### `slice-campaign` — legacy adapters

```text
unit: slice-campaign/legacy-control-adapter.mjs + legacy-review-adapter.mjs
semantic_owner: code domain
state_relationship: realization adapter
operation_identity: method calls proxying to legacy skill scripts (preflight,
  checkpoint accept/stop, offers, finalize)
determinism: deterministic dispatch to a legacy subprocess
authority: none beyond the subprocess exit status
inputs: copied refs
outputs: legacy receipt
evidence: whatever the legacy script returns
recovery: none visible beyond the subprocess call
workflow_coupling: workflow-private
realization_coupling: direct legacy Python coupling — a concrete, independent
  confirmation of what provider-turn-harness-runtime-and-operator-projection.md
  §9 already flags: "the principal builder path remains statically Codex-bound"
cross_service_deps: depends on legacy skill scripts
coordinate_relevance: not reconstruction-friendly — legacy receipts, not
  durable revisioned records
```

### `slice-campaign` — completion-publication and strategic-reconciliation

```text
unit: slice-campaign/completion-publication.mjs
semantic_owner: workflow
state_relationship: state owner — validates/publishes accepted-checkpoint and
  offer fields
operation_identity: method calls
determinism: deterministic
authority: none beyond field-shape validation
inputs: copied refs
outputs: publication record
evidence: digest-bound
recovery: CAS-ish (savePublication expected-revision check, following
  workspace-coordination's pattern)
workflow_coupling: workflow-private
realization_coupling: none direct
cross_service_deps: see the representation seam below
coordinate_relevance: digest-bound publication record is reconstruction-usable

unit: slice-campaign/strategic-reconciliation.mjs
semantic_owner: workflow
state_relationship: projection — parses a strategic-planning-handoff and
  tracks continuity
operation_identity: method calls
determinism: deterministic
authority: none beyond field-shape validation
inputs: copied refs
outputs: reconciliation state
evidence: digest-bound
recovery: none beyond its own state tracking
workflow_coupling: workflow-private
realization_coupling: none
cross_service_deps: none
coordinate_relevance: its continuity vocabulary — initialized / retained /
  reconstructed (app-server/src/services/slice-campaign/strategic-reconciliation.mjs:5)
  — is a small, independently-arrived-at instance of the coordinate/
  continuation concept central to revisioned-research-and-execution-architecture.md.
  It was not built with that document in mind; it converged on the same idea
  from an unrelated subsystem. That is evidence the concept is real, not just
  a documented aspiration.
```

**Representation seam:** `completion-publication.mjs` independently declares
its own `ACCEPTED_CHECKPOINT_FIELDS` and `CHECKPOINT_ATTRIBUTIONS` constants
(`app-server/src/services/slice-campaign/completion-publication.mjs:9,16`)
rather than importing them from `slice-checkpoint` or `review-subject`'s
contract. A repository-wide search found no other definition of either
constant and no import relationship between this file and either canonical
owner. This is a `REPRESENTATION` seam in
`incremental-architecture-intake-and-seam-reconciliation.md`'s vocabulary
(§7.2): two places independently declare what an accepted checkpoint's shape
is, with no enforced correspondence. It is not currently known to have
drifted — but nothing prevents it from drifting silently if `slice-checkpoint`'s
manifest shape changes.

---

## Synthesis

**`slice-campaign` is not one service — it is at least six**, bundled under
one directory name: a campaign-state owner, a native-review workflow
coordinator, the capability-dispatch envelope itself, a legacy-adapter bundle,
a completion-publication state owner, and an unrelated strategic-reconciliation
projection. This is the largest single finding of this audit. It is not a
plausible-sounding guess — the count above is grounded in each unit's actual,
distinct operation identity, authority mechanism, and consumer set. The
native-review host in particular reads as deserving its own top-level
position, not a nested file group.

**Two genuinely domain-neutral, already-working primitives exist today,
independent of any future design work:**

- `claim-evidence`'s ledger core is a generic service-operation envelope
  already proven across three unrelated domains/profiles
  (`proposal-research-v1`, `revision-bound-review-finding-v1`,
  `production-path-v1`), with real idempotency (`operation_id` + payload
  digest) and CAS (`expected_state`). This is a stronger existing precedent
  for a shared envelope than `slice-campaign`'s capability dispatch, which has
  exactly one consumer.
- `workspace-coordination`'s core is the cleanest kernel-shaped primitive
  found: fencing-token authority, and resource typing that already spans
  non-Git kinds (`port`, `database`, `review-budget`) alongside Git-specific
  ones. Kernel-plane infrastructure is not purely aspirational — a working
  instance already exists and is already domain-neutral.

**Units with no real identity or durable existence:** `skills-migration-integrity`
is a bare function with no store, authority, or identity of its own, called
inline from one file. It is one of the predicted "not really a service" cases
— a workflow-private utility wearing a top-level service directory name.

**Weakest recovery mechanism found:** `operational-coordination`'s replay path
is a heuristic regex match against board state, not a durable CAS index —
notably weaker than `review-episode`'s or `claim-evidence`'s mechanisms for a
unit that otherwise looks reusable and domain-neutral.

**Cross-validation, not new information:** this audit independently re-derived
the `slice-campaign` candidate/profile-overwrite gap
(`service.mjs:255-278`) that this session's Candidate Trajectory work already
targeted — arrived at from a service-ownership angle rather than a
history/coordinate angle. Two unrelated investigative paths landing on the
same seam is real corroboration.

## Capability naming — three representations confirmed, not two

1. **Operation identity (host-internal dispatch key):**
   `slice-campaign/capability-contract.mjs` + `capability-host-runtime.mjs` —
   e.g. `capability.checkpoint_lifecycle/bind_candidate`.
2. **Role/realization-granted permission:**
   `provider-turn-harness-runtime-and-operator-projection.md` §6 —
   `capability.external_information_retrieval`,
   `capability.visual_artifact_observation`; the same sense is used in
   `pre-indexed-capability-resolution-and-frozen-runtime-realization.md`'s
   "discover and index available runtime capabilities."
3. **Model-facing dynamic-tool-manifest schema:**
   `product-development/proposal-delivery.mjs:97`,
   `proposalCapabilityDefinitions` — structurally an instance of sense 1
   (an operation identity), but represented as a tool schema
   (`{namespace, name, description, inputSchema}`) rather than an internal
   dispatch key. A third surface using the word "capability," even though it
   is closer in meaning to sense 1 than sense 2.

Any future formalization of a service-operation concept needs to resolve
this before it can use the word "capability" for anything new.

## Relationships

| Direction | Relationship |
| --- | --- |
| `incremental-architecture-intake-and-seam-reconciliation.md` | This document supplies the service-plane portion of the "one exact implemented architecture baseline" its method requires (§3) before integrating prospective ideas. |
| `provider-turn-harness-runtime-and-operator-projection.md` | `reviewer-runtime` is a concrete, partial precursor to `HarnessRuntimePort`/`ProviderTurnPort`, already named in that document's §9. This audit independently confirms its §9 claim that "the principal builder path remains statically Codex-bound" via the `slice-campaign` legacy adapters. |
| `revisioned-research-and-execution-architecture.md` | `strategic-reconciliation.mjs`'s `continuity` vocabulary and several units' digest/revision shapes are independently-arrived-at coordinate fragments, corroborating that document's coordinate concept from unrelated code. |
| Candidate Trajectory family (`candidate-trajectory-*.md`) | The `slice-campaign` campaign-state gap this audit found independently matches the gap that family already targets — cross-validation, not a new requirement. |
| A prospective service-plane architecture (kernel / service / workflow planes, a `ServiceOperation` schema, a capability/service-operation rename) | Not owned by this document. This inventory is the evidence such a proposal would need to cite; it does not itself make the proposal. |

## Non-goals

- This document does not propose splitting Work Engine into kernel, service,
  and workflow planes.
- It does not propose a `ServiceOperation` schema or any specific envelope
  design.
- It does not propose renaming "capability" or resolving the naming overload
  it documents.
- It does not propose refactoring `slice-campaign` into separate directories,
  even though it identifies six distinct units inside it.
- It does not authorize implementation of anything. It is a baseline for
  future reasoning, not a decision.
- The state-relationship labels used throughout are audit labels for this
  pass. A future pass may find several of them collapse together or need
  different boundaries.
