# Skills Migration Campaign: Decomposed Sources, Runtime Roles, and Server Services

## Status and authority

This document is an implementation-campaign source for migrating Work Engine
skills into the App Server architecture. It supplies objectives, invariants,
candidate slice boundaries, dependencies, acceptance evidence, and stop
conditions. It does not itself migrate canonical ownership, accept a semantic
classification, authorize a contract change, approve a slice plan, or make a
generated projection authoritative.

Evidence cutoff: repository `HEAD`
`41b72898af28d489fd6dbed456593bf20abc5696` plus the directly inspected working
tree on 2026-08-28. The working tree already contained unrelated and ongoing
App Server, product-development, experiment, and instruction-review changes.
Those changes remain user-owned and must not be reverted, absorbed, or treated
as accepted migration state merely because this plan observes them.
Uncommitted observations are provisional and must be pinned to exact bytes or
an immutable checkpoint before a slice relies on them as a migration subject.

The campaign may revise, split, combine, or reorder candidate slices when new
evidence supports a better route. It must preserve every governing invariant,
real dependency, authority boundary, and required acceptance consequence.

## Campaign objective

Migrate the current Work Engine skill estate into an App Server-native system
where:

- each semantic assertion and mechanically enforceable boundary has one
  canonical owner;
- decomposed skill sources generate complete provider-specific model
  projections without fabricating or omitting meaning;
- role-bearing skills can be realized as independently controlled logical role
  instances through the runtime manifest;
- non-role instruction packages remain reusable skills rather than being
  forced into role instances;
- claims, context lifecycle, scheduling, durable state, and other horizontal
  concerns are owned by server services when their semantics are broader than
  one agent;
- deterministic capabilities and projections do not acquire semantic or
  decision authority;
- legacy behavior and artifacts remain available until exact successor
  evidence supports an explicit ownership transition; and
- obsolete agent-local machinery can be retired without losing compatibility,
  auditability, continuation safety, or human authority.

The terminal consequence is not that every legacy directory has been copied.
It is that every retained responsibility has an explicit semantic owner,
runtime realization, model-loading boundary, test owner, migration disposition,
and reconstructable history, with no active consumer depending on an
unclassified legacy path.

## Governing architecture

The target ownership and projection flow is:

```text
canonical authored sources
  structure.yaml + interface.yaml + shared catalogs
                         |
                         v
provider-neutral compiler and structural closure
                         |
          +--------------+----------------+
          |              |                |
          v              v                v
 generated SKILL.md  environment view  runtime requirements
          |                               |
          +---------------+---------------+
                          v
                    runtime manifest
       role template + capabilities + thread realization
                          |
                          v
                logical App Server role instance
                          |
          +---------------+----------------+
          |                                |
          v                                v
 server-owned domain services      hidden context lifecycle
```

The layers have distinct ownership:

| Layer | Owns | Must not own |
|---|---|---|
| Skill structure | Semantic instructions, objectives, authority meaning, causal explanations, optional role profile | Provider deployment choices or server state |
| Skill interface | Closed mechanically enforceable boundaries linked to semantic owners | New domain meaning or widened authority |
| Shared catalogs | Accepted cross-skill invariant, mechanism, capability, state, artifact, and relation truth | Skill-local prose or runtime state |
| Compiler | Deterministic resolution, closure, validation, provenance, and rendering | Semantic invention, contract acceptance, or runtime deployment policy |
| Generated `SKILL.md` | Provider-specific complete model-facing projection | Canonical authored meaning after ownership migration |
| Runtime manifest | Variant role composition, capabilities, thread settings, and provider realization | Role authority, workflow state, or skill meaning |
| Runtime binding | Replaceable logical-role-to-provider-thread association | Canonical role identity or product state |
| Server services | Canonical horizontal service mechanics and durable service state | Role judgment or authority not granted to the service |
| Context lifecycle | Observation, preservation, verification, transition, rehydration, and reconciliation mechanics | Domain truth, human intent, claims, decisions, or workflow authority |

## Current evidence baseline

The plan begins from implemented or directly validated foundations rather than
assuming a blank App Server:

- The runtime manifest already projects named logical role instances, exact
  role-contract and skill inputs, capabilities, thread options, and a
  deterministic role environment revision through a generic role runtime.
- Exact skill resolution is confined to configured roots and exact `SKILL.md`
  files.
- Manifest-role context observation already binds governing role-contract and
  activated-skill bytes into authenticated lifecycle projections.
- The claim-evidence service already owns a provider-neutral core, validation,
  storage, read projections, compatibility evidence, and bounded semantic
  shadow machinery.
- The semantic context lifecycle already has implemented observation,
  compilation and verification, publication fences, transition leases,
  rehydration, reconciliation, shadow operation, live operation, and durable
  episode evidence under explicit profiles.
- The Agent Environment Graph currently validates 35 invariants, 43 entities,
  16 mechanisms, and three roles. Its checked-in generated projections are
  current. Its analyzer reports one advisory ownership candidate for
  `artifact.checkpoint_receipt`; that candidate is not a semantic conclusion or
  a migration decision.
- The instruction-review correction concerning causal exposure at the governed
  agent's effective loading boundary exists in the observed working tree but is
  not an immutable bootstrap subject until S1 pins it.

Each slice must revalidate the foundation it materially relies on. This
baseline establishes available evidence, not perpetual freshness or acceptance.

## Campaign invariants

Every slice must preserve these conditions:

1. **Single semantic ownership.** A migration cannot leave both legacy prose and
   decomposed sources independently editable as canonical meaning.
2. **Single boundary ownership.** A mechanical enforcement rule resolves to one
   interface declaration and, where applicable, one semantic boundary.
3. **No authority by migration or projection.** Classification, generated
   output, graph relations, runtime configuration, or successful tests cannot
   grant authority without the owning authority transition.
4. **No fabricated semantics.** Deterministic machinery does not infer, improve,
   shorten, or paraphrase missing role or skill meaning.
5. **Complete effective loading.** When a structural command governs an agent,
   enough of its causal reason and concrete failure mode reaches that agent at
   the effective loading boundary. Reviewer reconstruction or traceability to
   an unloaded source is insufficient.
6. **No runtime expansion.** Runtime realization may narrow declared effects
   but may not exceed a role or skill effect ceiling.
7. **Exact identity and provenance.** Subjects, generated artifacts, reviews,
   tests, runtime environments, and ownership transitions bind exact revisions
   or content digests.
8. **Forward-only steady state.** After ownership migration, canonical sources
   generate provider projections; generated prose is never reverse-parsed into
   canonical meaning.
9. **No premature conditional omission.** Applicability metadata cannot remove
   instructions until an accepted closure contract proves that omission safe.
10. **Service promotion preserves boundaries.** Moving state above agents does
    not transfer semantic judgment, human authority, or role-owned truth to the
    server.
11. **Context is not canonical state.** Provider thread history and compiled
    continuation checkpoints may preserve or reference meaning but do not
    become its semantic owner.
12. **Compatibility is explicit.** A semantic difference from the legacy
    implementation is either authorized and receipted or blocks parity-based
    migration.
13. **Tests prove bounded consequences.** Existing tests are evidence to
    classify, not unquestioned contracts. New tests distinguish invalid states
    rather than freeze an incidental route.
14. **No self-certification.** The instruction-review skill and compiler cannot
    certify their own semantic adequacy. Bootstrap semantic evidence uses a
    separately defined review contract and fresh reasoning context.
15. **User work remains intact.** Slices preserve unrelated worktree changes,
    historical evidence, exact Git subjects, and user-authored sources.

## Migration unit and required record

The migration unit is a responsibility, not a directory. One legacy skill may
decompose into several destinations, and several legacy skills may rely on one
server service.

Each assessed skill receives a revision-bound migration record containing:

- legacy skill identity, source revision, byte digest, references, scripts,
  schemas, tests, generated artifacts, known consumers, and runtime bindings;
- present responsibilities separated into semantic instruction, optional role
  profile, deterministic mechanism, service state, provider adapter,
  projection, experiment, documentation, and compatibility surface;
- candidate semantic owner and authority source for each responsibility;
- candidate destination: canonical structure, interface, shared catalog,
  generated projection, runtime manifest, role instance, server service,
  host capability, compatibility adapter, retained experiment, or retirement;
- effective loading boundary and required instruction closure;
- service, role, and context-lifecycle dependencies;
- test inventory with the protected distinction claimed by each test;
- parity requirement or explicitly authorized semantic-difference contract;
- migration status and exact evidence references;
- unresolved classifications, ownership questions, and reopening conditions;
  and
- legacy retirement prerequisites and rollback or reconstruction expectations.

A record may remain `unresolved`. Classification uncertainty must not be hidden
by assigning a convenient destination.

## Preliminary portfolio hypotheses

The following table is an inventory aid, not an accepted classification. Each
row must be reconciled against its exact current revision during its owning
slice.

| Skill or package | Initial responsibility hypothesis | Candidate primary realization |
|---|---|---|
| `agent-environment-graph` | Structural catalogs, closure, analysis, and projections | Compiler structural core or validation backend plus generated views |
| `agent-instruction-review` | Specialist semantic review contract | Bounded isolated reviewer role plus reusable review projection |
| `chrome-vision` | Browser observation capability and adapter | Reusable skill and host capability |
| `claim-evidence` | Canonical claims mechanics plus agent-facing semantics | Server claim-evidence service plus thin skill/client projection |
| `claude-recon-implementation` | Provider adapter, reconnaissance, and independent review | Split provider adapter, disposable capability, and reviewer role profile |
| `code-change-profile` | Deterministic checkpoint-bound profiling | Host/service capability with thin instruction projection if needed |
| `codex-adversarial-review` | Same-model review contract | Bounded reviewer role profile without an independence claim |
| `comparative-repository-analysis` | Comparison-run judgment and deterministic artifacts | Role or bounded workflow plus host artifact machinery |
| `durable-state` | Opaque durable value mechanics | Server service plus minimal authorized client semantics |
| `idea-intake` | Bounded product-development judgment | Manifest role instance using host-owned read/publication capabilities |
| `independent-review-state` | Review episode state and writer fences | Server service or generic review-runtime component |
| `linguistic-register-pilot` | Experimental protocol and research artifacts | Retained experiment package, not ordinary production role machinery |
| `planning` | Planning support material | Inspect exact package; do not infer role status from name |
| `proposal-former` | Proposal-formation judgment | Manifest role instance using validated intake and publication capabilities |
| `proposal-packets` | Mechanical proposal discovery and validation | Product-development service capability |
| `repo-search` | Repository evidence affordances | Reusable agent skill and configured evidence capability |
| `review-bench` | Review-provider evaluation machinery | Evaluation service or experiment capability, never production approval |
| `role-scheduler` | Durable scheduled-item mechanics and role-facing judgment | Server scheduler service plus role-scoped projection where active |
| `slice-builder` | Retained bounded engineering role and local mechanisms | Manifest role profile plus generic runtime/service capabilities |
| `slice-checkpoint` | Immutable Git checkpoint mechanics | Host capability/service boundary |
| `slice-completion-commit` | Explicitly approved user-visible publication | Host capability with human authority fence |
| `slice-supervisor` | Durable campaign coordination role and workflow state | Manifest role plus server-owned campaign/runtime services |
| `strategic-planner` | Durable advisory planning judgment | Manifest role instance |
| `ui-design-principles` | Domain reasoning guidance | Reusable agent skill |
| `wind-walker` | Context-continuation invariants formerly projected to agents | Hidden context-lifecycle service contract; retain only necessary role-facing causal projection |
| `work-engine-mcp` | External bounded projections over Work Engine state | MCP/API projection over server services, not a parallel owner |

## Test migration policy

Every legacy test is classified before it is copied, rewritten, generated, or
removed:

| Disposition | Meaning |
|---|---|
| `retain` | It proves an unchanged consequence at the same owner boundary. |
| `relocate` | The consequence remains valid but its owner moved to a service, compiler, manifest, or capability. |
| `rewrite` | The old test encoded a replaced mechanism; a new test must prove the preserved consequence. |
| `compatibility` | It remains temporarily to compare legacy and successor behavior. |
| `historical` | It preserves migration evidence but is not a production gate. |
| `remove` | Its claimed requirement was non-contractual or superseded, with the reason recorded. |

The campaign uses four complementary test layers:

1. **Structural tests** validate schemas, stable identities, references,
   closure, provenance, effect ceilings, manifest satisfaction, and drift.
2. **Projection tests** construct the effective loaded instructions and verify
   exact inclusion, ordering or precedence where causal, and declared
   omissions.
3. **Behavioral semantic tests** exercise revision-bound cases whose expected
   consequence requires model judgment, including the corrected
   instruction-review defect.
4. **Runtime consequence tests** exercise App Server roles and services across
   effects, refusals, restarts, concurrent instances, context transitions,
   receipts, and legacy compatibility.

Deterministic tests must not pretend to establish natural-language semantic
adequacy. Behavioral tests should assert material findings or preserved
distinctions rather than exact stylistic output.

## Slice dependency shape

```text
S0 inventory and baseline
 |
 +--> S1 instruction-review regression contract
 |
 +--> S2 sterile slice-builder compiler experiment
          |
          +--> S3 canonical graph/compiler convergence
          |       |
          |       +--> S4 runtime-requirements and manifest satisfaction
          |                    |
          |                    +--> S5 lifecycle instruction closure
          |
          +--> S6 non-role skill import

S1 + S4 + S5 --> S7 reviewer role import
S0 + service evidence --> S8 service-backed skill import
S4 + S5 + S6 + S8 --> S9 product-development role imports
S3 + S4 + S5 + S7 + S8 --> S10 retained engineering roles
S6 through S10 --> S11 remaining portfolio batches
S11 --> S12 ownership cutover and legacy retirement
```

This is a dependency model, not a compulsory execution sequence. Independent
work may proceed concurrently when immutable subjects and non-overlapping
ownership make that safe.

## Candidate slices

### S0 — Revision-bound portfolio inventory and migration ledger

**Objective:** Make every current skill responsibility, consumer, artifact,
test, and authority question discoverable without changing ownership or
runtime behavior.

**Independently valuable outcome:** Later slices can select coherent work from
one exact inventory instead of rediscovering or silently omitting legacy
responsibilities.

**Required consequences:**

- Every current skill directory has a migration record bound to exact source
  and referenced-artifact digests.
- Role-like behavior is distinguished from agent skill, deterministic
  capability, service state, provider adapter, experiment, and projection.
- Current consumers and generated artifacts are enumerated with evidence.
- Existing tests carry an initial protected-distinction statement and
  candidate disposition.
- Claims and context behavior already promoted into App Server are identified
  as existing target evidence rather than work to copy again.
- Unresolved ownership remains explicit.

**Acceptance evidence:** Closed inventory validation; no unaccounted skill
directory; sampled exact-source verification; recorded graph-index limitations;
and an advisory review of classifications without accepting them as contract
changes.

**Excluded:** Decomposition, generated outputs, manifest additions, service
ports, ownership migration, or legacy deletion.

### S1 — Retrospective instruction-review regression contract

**Objective:** Establish a revision-bound semantic test corpus that detects the
corrected failure where a command's causal reason exists or is reconstructable
but is unavailable to the governed agent at its effective loading boundary.

**Independently valuable outcome:** Every later migration can test assembled
instruction closure, including skills whose prose remains byte-identical.

**Required consequences:**

- Fixtures distinguish source traceability, reviewer reconstruction, actual
  role loading, compact projection, precedence, and conditional omission.
- Positive and negative cases cover structural commands, route preferences,
  causally required sequencing, bundled clauses, and conflicting layers.
- Results bind immutable fixture revisions and assert finding consequences
  rather than preferred wording.
- Applicability, no-finding, limitations, remediation, and retained-reviewer
  semantics remain truthful.
- Bootstrap review of the corpus uses a separately defined contract and fresh
  reasoning context; the corrected skill does not certify itself.

**Acceptance evidence:** Deterministic fixture validation, repeatable behavioral
results with recorded configuration, independent bootstrap assessment, and one
demonstration that the former defect is caught while a valid separately stored
and actually projected causal explanation is accepted.

**Excluded:** Migrating the review skill's canonical ownership or declaring
all legacy instructions correct.

### S2 — Sterile slice-builder decomposition and compiler bootstrap

**Objective:** Prove the smallest provider-neutral forward compiler by
decomposing the pinned slice-builder skill into meaningful experimental sources
and regenerating its current `SKILL.md` byte-for-byte without changing
behavior, authority, environment relations, or tests.

**Independently valuable outcome:** The campaign has one proven compiler,
renderer, intermediate representation, provenance model, and legacy migration
method before generalizing.

**Required consequences:** Use the complete bootstrap contract and acceptance
criteria in `role-compiler-proposal.md`, including meaningful decomposition,
no whole-document escape hatch, exact byte equality, idempotence, source
attribution, resolved authority relations, Agent Environment Graph parity, and
unchanged existing gates.

**Acceptance evidence:** Pinned source receipt; reviewed experimental
`structure.yaml` and `interface.yaml`; byte equality; second-generation no-op;
environment-relation and invariant parity; S1 projection review; and an
explicit decision that does not yet migrate canonical ownership.

**Excluded:** Improving slice-builder prose, generalizing every skill kind,
conditional loading, a reverse parser, or a second migrated skill.

### S3 — Shared structural core and canonical ownership convergence

**Objective:** Give the compiler and Agent Environment Graph one structural
vocabulary and closure implementation without creating a second semantic graph
owner.

**Independently valuable outcome:** Skill-local role profiles can generate the
same lossless role views, invariant closure, source hashes, analysis inputs, and
drift evidence currently produced from central environment sources.

**Required consequences:**

- Schema and intermediate-representation ownership are explicit.
- Existing graph validation remains fail-closed for duplicate keys, unknown
  references, invalid relations, and unauthorized contract judgments.
- Compiler output reproduces the slice-builder role projection without an
  unexplained semantic delta.
- Candidate analysis remains advisory and cannot mutate canonical sources.
- A documented transition distinguishes the pre-migration central canonical
  inputs from any future generated aggregate.
- No renderer or graph view becomes canonical doctrine.

**Acceptance evidence:** Cross-implementation parity or one shared structural
core; transitive-closure tests; deterministic projection and drift tests;
judgment-artifact validation; and an explicit owner for every schema concept.

**Excluded:** Migrating the central environment catalog merely because parity
was achieved.

### S4 — Runtime-requirements projection and manifest satisfaction

**Objective:** Connect canonical skill semantics to App Server deployment by
generating closed runtime requirements and validating that a runtime-manifest
role realization satisfies them without exceeding authority.

**Independently valuable outcome:** A role cannot be started with missing
capabilities, incompatible lifetime, omitted contract inputs, or effects above
its declared ceiling.

**Required consequences:**

- The compiler emits only mechanically decidable runtime requirements.
- Manifest deployment choices remain variant structure.
- Required capabilities, prohibited effects, effect ceilings, bindings,
  schemas, and intrinsic continuity needs have closed identities.
- Runtime grants may narrow but cannot widen the declared environment.
- The role contract is present in the exact activated skill inputs.
- Semantic instructions currently authored directly as manifest developer
  instructions receive a single-owner disposition: generated semantic
  projection or deliberately deployment-local fragment.
- Any semantic skill change has an explicit environment-revision and binding
  migration consequence; exact skill-byte and executable-generation
  fingerprints cannot disagree silently about compatibility.

**Acceptance evidence:** Positive and negative manifest fixtures; environment
revision tests; exact skill-resolution tests; capability ceiling tests;
incompatible-binding refusal; and a generated requirements receipt bound to
the manifest and compiled skill closure.

**Excluded:** Encoding domain judgment in the manifest validator or using one
handwritten runtime gateway per role.

### S5 — Hidden context-lifecycle integration with compiled roles

**Objective:** Ensure server-owned context preservation and replacement consume
the exact compiled role environment while leaving domain meaning and authority
with their canonical owners.

**Independently valuable outcome:** A compiled role can survive context-window
replacement without carrying agent-local context-management procedure or
losing instruction closure.

**Required consequences:**

- Observed context binds the role instance, binding revision, environment
  revision, exact governing contract, activated skill digests, visible
  materials, expected next work, completeness, omissions, and lifecycle state.
- Intrinsic role continuity requirements remain distinguishable from pressure
  thresholds and lifecycle implementation policy.
- Compiler and verifier cannot create authority or silently close unresolved
  human interaction.
- Rehydration restores necessary model-facing causal instructions, not opaque
  invariant IDs alone.
- Role packages no longer require ordinary agents to monitor token pressure or
  operate context replacement.
- `wind-walker` receives an explicit retirement or residual-projection plan;
  no duplicate lifecycle owner remains active.

**Acceptance evidence:** Shadow and live fixtures as appropriate; exact-source
projection verification; semantic-checkpoint refusal cases; same-role
replacement and reconciliation evidence; and an S1 review of pre- and
post-transition effective instructions.

**Excluded:** Moving role objectives, claims, decisions, schedules, or human
authority into lifecycle checkpoints.

### S6 — First non-role instruction-package import

**Objective:** Prove that a reusable agent skill without a role profile can use
the same canonical package, compiler, projection, provenance, and test system
without becoming an App Server role instance.

**Candidate selection:** Choose a bounded skill with limited state and clear
consumers after S0 evidence; `ui-design-principles` or another instruction-only
package may be suitable, but this plan does not preselect it authoritatively.

**Independently valuable outcome:** The schema demonstrates that roles are
optional and avoids turning the whole skill estate into thread templates.

**Required consequences:** Byte or explicitly reviewed semantic parity;
complete causal projection; no role-only placeholder fields; exact runtime
activation through configured consumers; retained references; and classified
tests.

**Acceptance evidence:** Deterministic generation, drift detection, effective
loading test in at least one consumer role, and S1 semantic review.

**Excluded:** Adding a role instance merely to fit the compiler schema.

### S7 — Instruction-review specialist role import

**Objective:** Realize `agent-instruction-review` as a bounded App Server review
role using compiled instructions, immutable subjects, and explicit reviewer
lifetime without granting mutation or acceptance authority.

**Independently valuable outcome:** The campaign has a production-like
specialist capable of reviewing later migrated projections under the corrected
contract.

**Required consequences:**

- Fresh entry is available when the evidence claim requires it.
- The same isolated reviewer may continue through bounded remediation without
  falsely claiming renewed independence.
- Subjects and findings bind exact revisions and stable finding identities.
- The reviewer receives the actual effective instruction closure being
  evaluated, including manifest fragments and conditional references.
- Mutation, architecture choice, proposal acceptance, panel selection, and
  implementation approval remain unreachable.
- Runtime retention choices satisfy rather than redefine review semantics.

**Acceptance evidence:** Manifest satisfaction; read-only effect tests;
fresh-entry and retained-remediation fixtures; corrected-defect corpus; and an
independent bootstrap review of the migrated role projection.

**Excluded:** Treating every review as independent or allowing the role to
certify its own implementation.

### S8 — First service-backed skill import

**Objective:** Prove the decomposition of a legacy skill whose durable and
mechanical responsibilities have been promoted to a server service while
retaining only the agent-facing semantics and authorized client surface that
agents actually need.

**Preferred evidence-bearing candidate:** `claim-evidence`, because its App
Server service, compatibility tests, read projections, and migration design
already exist. Final selection follows S0 reconciliation.

**Independently valuable outcome:** The migration can remove agent-local state
ownership instead of copying it behind a new path.

**Required consequences:**

- Canonical claim identity, revision, lineage, reliance, authority, and store
  mechanics remain server-owned.
- Model judgment remains outside deterministic publication mechanics.
- The thin skill explains semantic use and limitations without teaching the
  agent to emulate the registry.
- Runtime capabilities expose only authorized operations.
- Legacy fixtures establish exact parity or an authorized semantic-difference
  receipt.
- No second local store, authority ledger, or lifecycle owner remains active.

**Acceptance evidence:** Service and compatibility suites; concurrent-instance
and restart tests; capability refusal tests; exact projection review; and one
bounded real consumer using the service rather than legacy local machinery.

**Excluded:** Turning the claims service into a universal claims role.

### S9 — Initial product-development role imports

**Objective:** Import bounded role-bearing skills that already have App Server
vertical evidence, beginning with roles whose inputs, outputs, authority, and
host capabilities are narrow enough to verify independently.

**Candidate set:** `idea-intake`, `proposal-former`, and `strategic-planner`.
Each remains its own slice if their evidence or ownership differs materially.

**Independently valuable outcome:** Product-development judgment runs through
generic compiled role packages and manifest composition rather than additional
handcrafted gateways.

**Required consequences:**

- Exact request, source, evidence-cutoff, and output bindings remain enforced.
- Host publication proves mechanical integrity and persistence only, not
  semantic quality, acceptance, priority, or implementation authority.
- Logical instances bind to replaceable threads and server lifecycle services.
- Existing role-specific code is classified into generic boundary machinery,
  named deterministic integration, compatibility adapter, or retirement.
- Zero-output and unresolved outcomes remain truthful where the current host
  lacks a closed durable disposition.

**Acceptance evidence:** One exact end-to-end vertical per role; restart and
binding evidence; negative authority tests; projection review; deterministic
output validation where applicable; and compatibility with current artifact
owners.

**Excluded:** Proposal acceptance, roadmap mutation, artifact-root migration,
or implementation authorization.

### S10 — Retained engineering workflow roles and mediated capabilities

**Objective:** Import the retained builder and supervisor environments while
preserving their distinct authority, context lifetimes, mediated transitions,
review boundaries, checkpoint ownership, and recovery behavior.

**Candidate responsibility set:** `slice-builder`, `slice-supervisor`,
`slice-checkpoint`, `slice-completion-commit`, `independent-review-state`,
provider review adapters, and campaign receipt machinery. The supervisor and
builder should not be forced into one slice if mutable-workspace or authority
risks warrant separation.

**Independently valuable outcome:** An authorized campaign can execute one
bounded slice through App Server-native roles and services without relying on
the legacy agent-local harness as semantic owner.

**Required consequences:**

- Supervisor campaign authority remains distinct from builder repository-domain
  judgment and user approval.
- Builder retention spans the accepted slice while useful; supervisor
  continuity spans the campaign without absorbing builder reasoning.
- Plan acceptance, checkpoint creation, review selection, receipt finalization,
  and user-visible commit publication remain mediated by their owners.
- Independent reviewers begin outside builder reasoning context and cannot
  mutate implementation.
- Mutable workspace admission prevents conflicting replacement builders.
- Checkpoint and completion-commit capabilities preserve exact Git and human
  authority fences.
- Context lifecycle preserves role-specific continuation without becoming
  campaign state.

**Acceptance evidence:** One end-to-end accepted test slice in an isolated
fixture repository; deterministic gates; immutable review candidate;
remediation loop; exact checkpoint and receipt evidence; restart/recovery;
negative authority and concurrency tests; and legacy outcome comparison.

**Excluded:** Automatic user-visible commit publication, silent campaign scope
expansion, or a broad rewrite of all campaign machinery in one slice.

### S11 — Evidence-driven remaining portfolio batches

**Objective:** Migrate all remaining retained responsibilities in coherent
batches selected from the S0 ledger, using the proven non-role, reviewer,
service-backed, product-role, and retained-workflow patterns.

**Independently valuable outcome:** Each batch removes a bounded class of
legacy dependency while preserving a functioning predecessor until acceptance.

**Selection constraints:**

- A batch shares an owner or runtime pattern, not merely a directory prefix.
- Provider adapters remain separate from semantic review profiles.
- Experiments remain experiments unless separately promoted.
- MCP surfaces remain projections over canonical services.
- Scheduler activation remains distinct from durable scheduling mechanics.
- A skill with unresolved ownership receives its own decision slice rather than
  being silently grouped.

**Acceptance evidence:** Complete migration records for the batch; pattern-
specific parity and runtime evidence; no unaccounted consumer; explicit
ownership decision; and rollback or reconstruction evidence.

**Excluded:** A final bulk copy or deletion justified only by inventory
completion pressure.

### S12 — Canonical ownership cutover and legacy retirement

**Objective:** Complete explicit ownership transitions and remove legacy
execution paths only after every active consumer uses accepted generated
projections, manifest roles, services, or capabilities.

**Independently valuable outcome:** The repository has one active semantic and
runtime architecture, with historical migration evidence but no ambiguous
dual ownership.

**Required consequences:**

- Each migrated package has an accepted ownership record naming old and new
  canonical sources, generated artifacts, compiler version, source digests,
  authority, and reconstruction expectations.
- Direct edits to generated projections fail drift checks.
- Central environment inputs are retained as canonical or replaced by an
  explicitly accepted generated aggregate; they never drift into an informal
  second owner.
- Runtime manifests and bindings reference accepted compiled environments.
- Server-promoted responsibilities have no active agent-local state owner.
- Legacy discovery and compatibility paths are removed only after exact
  consumer and historical-reference checks.
- Historical subjects, receipts, and Git identities remain resolvable.

**Acceptance evidence:** Repository-wide consumer inventory; clean generation
and drift gates; App Server integration suite; service migration receipts;
context replacement evidence for retained roles; historical reconstruction
test; and separately authorized retirement changes.

**Excluded:** Deleting historical evidence, rewriting immutable subjects, or
inferring authority from technical completion.

## Slice entry contract

Before implementation begins, every slice plan identifies:

- the exact objective and independently valuable terminal state;
- immutable work source and repository evidence cutoff;
- accepted authority and mutation boundary;
- semantic owners affected and whether any contract change is proposed;
- candidate files or components without converting them into an exhaustive
  route unless scope confinement requires it;
- predecessor compatibility and rollback expectations;
- required deterministic, behavioral, runtime, and review evidence;
- effective instruction-loading boundary;
- server-service and context-lifecycle interactions;
- unresolved human decisions; and
- conditions that stop the slice without claiming success.

Plan acceptance authorizes only that slice's bounded implementation. It does
not accept a semantic classification, migrate canonical ownership, authorize a
later slice, or grant cleanup authority unless those effects are explicit in
the accepted plan.

## Common slice acceptance gate

A slice is accepted only when all applicable evidence is available:

1. The exact implemented subject and task-owned change set are immutable or
   otherwise revision-bound for review.
2. Deterministic tests establish mechanically decidable contracts.
3. Behavioral tests establish the requested semantic consequence with truthful
   configuration and limitations.
4. Effective model-facing instructions pass the corrected instruction-review
   contract.
5. Runtime grants do not exceed declared effect ceilings.
6. Server services retain canonical state and reject unauthorized or
   conflicting operations.
7. Context-lifecycle integration preserves continuation dependencies and
   unresolved human interaction where applicable.
8. Legacy parity is exact or every semantic difference has an authorized
   receipt.
9. Generated projections are deterministic, attributed, and drift-free.
10. No unrelated user work was reverted, absorbed, or silently made part of the
    migration.
11. Findings are evaluated by the authorized builder or author; advisory review
    does not accept the slice.
12. The named decision owner performs any required ownership or contract
    transition separately from technical validation.

## Campaign stop and replan conditions

Stop the affected slice and return to its decision owner when:

- canonical semantic ownership cannot be identified;
- decomposition exposes a genuine contract change without authority;
- byte parity and semantic fidelity materially conflict;
- a runtime requirement cannot be expressed without embedding domain judgment
  in deterministic code;
- the manifest must exceed an effect ceiling to realize the role;
- selective loading cannot prove complete causal closure;
- context replacement depends on meaning represented only in retiring context;
- legacy compatibility requires rewriting immutable evidence or historical
  identity;
- a service promotion would transfer role or human authority to the server;
- a dirty-worktree overlap cannot be isolated safely; or
- a candidate batch contains materially different owners or acceptance risks
  that require separate slices.

A stopped route is evidence about the migration; it is not automatic failure of
the campaign objective. Preserve the exact premise, observation, and dependent
conclusion made stale, then select a replacement route within existing
authority.

## Campaign completion condition

The skills migration campaign is complete when:

- every legacy responsibility has an accepted destination or explicit retained
  status;
- every active model-facing instruction is generated from or deliberately
  retained under one canonical semantic owner;
- every role realization satisfies compiled requirements through the runtime
  manifest;
- every server-promoted responsibility has one canonical service owner and no
  active agent-local duplicate;
- every retained role participates safely in hidden context lifecycle without
  owning its mechanics;
- every active consumer uses accepted successor paths;
- generated artifacts and environment views are reproducible and drift-free;
- compatibility paths scheduled for removal have been retired with receipts;
- unresolved ownership, authority, and semantic questions remain explicitly
  owned rather than hidden; and
- the authorized campaign owner accepts the terminal evidence.

## Neighboring authoritative and planning material

- [`../../DESIGN.md`](../../DESIGN.md) owns Work Engine product structure and
  the distinction between invariants, consequences, affordances, procedures,
  and judgment.
- [`../../PHILOSOPHY.md`](../../PHILOSOPHY.md) is non-normative reasoning.
- [`role-compiler-proposal.md`](role-compiler-proposal.md) defines the current
  compiler candidate and sterile slice-builder bootstrap.
- [`../README.md`](../README.md) documents the implemented runtime-manifest and
  App Server boundary.
- [`semantic-context-lifecycle-manager.md`](semantic-context-lifecycle-manager.md)
  defines the horizontal context-lifecycle service boundary.
- [`claim-evidence-service.md`](claim-evidence-service.md) defines the promoted
  claim-evidence service architecture.
- [`../claims-service-migration.md`](../claims-service-migration.md) records the
  claims and product-development migration inventory and existing slices.
- [`../../docs/workflow-invariants.md`](../../docs/workflow-invariants.md) and
  [`../../docs/agent-environments.yaml`](../../docs/agent-environments.yaml)
  remain the current canonical graph inputs until an explicit ownership
  migration says otherwise.
- [`../../skills/agent-environment-graph/SKILL.md`](../../skills/agent-environment-graph/SKILL.md)
  governs validation, projection, drift, and semantic-judgment handling for the
  current environment graph.
- [`../../skills/agent-instruction-review/SKILL.md`](../../skills/agent-instruction-review/SKILL.md)
  governs revision-bound review of effective agent-facing instruction text.
