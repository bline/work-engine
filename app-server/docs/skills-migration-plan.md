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
                  |
                  +--> S4 runtime requirements and manifest satisfaction
                           |
                           +--> S5 lifecycle instruction closure
                           |
                           +--> S6 repo-search non-role import
                           |
                           +--> S7 immutable subject/profile capabilities

S2 through S7 --> S8 builder role and supervisor core
S1 + S3 + S5 + S7 --> S9 generic review contract and episode service
S6 + S7 + S9 --> S10 reviewer role, profile registry, OpenRouter adapter
S1 + S10 --> S11 instruction-review specialist composition
S8 + S9 + S10 + S11 + claim-service evidence --> S12 native review closure
S9 through S12 --> S13 reviewer research and admission evidence
S4 + S5 + S6 + S12 --> S14 product-development role imports
S0 + S5 + S12 + service evidence --> S15 remaining service-backed imports
S6 through S15 --> S16 remaining portfolio batches
S16 --> S17 ownership cutover and legacy retirement
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

## Reviewer-stack priority amendment

The user approved this ordering amendment on 2026-08-28 after S1 acceptance.
Reviewer production and research became near-term consumers of the migration,
but that urgency does not remove the compiler, manifest, or context-lifecycle
dependencies established by S2–S5. The revised route therefore:

- preserves S2–S5 as the common role and skill foundation;
- uses `repo-search` as the first non-role import instead of importing an
  unrelated demonstration package;
- establishes immutable subject and change-profile mechanics before any
  reviewer role consumes them;
- splits builder and supervisor core realization from complete native-review
  integration, avoiding the former dependency cycle in which the engineering
  workflow slice required a reviewer stack that had not yet been migrated;
- gives provider-neutral review semantics and review-episode state owners
  before constructing provider adapters;
- requires a minimal closed reviewer-profile registry before an OpenRouter
  adapter may accept a profile identifier or model slug;
- treats independence as a configured and receipted execution property, never
  as an implication of a role name or provider choice; and
- moves reviewer research immediately behind the production-review vertical
  while keeping benchmark, analysis, adjudication, and routing evidence
  outside production acceptance authority.

This amendment changes priority and slice boundaries, not the campaign
objective, accepted S0/S1 consequences, canonical ownership, or migration
authority. Candidate slices may still split when one boundary cannot be
accepted independently.

### S6 — First non-role import: repository evidence retrieval

**Objective:** Import `repo-search` as a reusable provider-neutral evidence
skill using the canonical package, compiler, projection, provenance, and test
system without creating an App Server role instance.

**Independently valuable outcome:** The compiler proves that role profiles are
optional, and later reviewers receive one bounded evidence-retrieval contract
over configured indexed-structure and direct-source capabilities.

**Required consequences:**

- Repository orientation, discovery, tracing, impact, and bounded audit
  semantics remain provider-neutral.
- Codebase Memory and direct-source access are configured capabilities rather
  than semantic owners or independence evidence.
- Coverage, pagination, fallback provenance, negative-claim limits, and exact
  source verification remain model-facing causal instructions.
- The package contains no role-only placeholder fields and grants no mutation,
  review, or acceptance authority.

**Acceptance evidence:** Deterministic generation and drift detection; byte or
explicitly reviewed semantic parity; configured capability fixtures; effective
loading in one bounded consumer; coverage/fallback cases; and S1 projection
review.

**Excluded:** Creating a repository-search role, hard-coding one evidence
provider, or claiming that retrieval is independent review.

### S7 — Immutable review-subject and change-profile capabilities

**Objective:** Port `slice-checkpoint` and `code-change-profile` as host or
server capabilities that produce an immutable attributed review subject and a
recomputable deterministic physical profile before reviewer selection or
execution.

**Independently valuable outcome:** Every later reviewer can receive the exact
commit, tree, manifest, patch identity, and subject-side metrics it is meant to
review without acquiring checkpoint or profiling authority.

**Required consequences:**

- Checkpoint creation preserves the user's branch, worktree, index, unrelated
  files, attribution, and private-ref boundary.
- Change profiles bind immutable checkpoint subjects and are recomputable from
  exact repository evidence.
- Profile metrics describe physical change characteristics only; they neither
  select a reviewer nor infer semantic risk, quality, or acceptance.
- Both capabilities remain mediated and unreachable as reviewer judgment.

**Acceptance evidence:** Git-state preservation and conflict fixtures; exact
subject and manifest verification; deterministic profile recomputation;
tamper, stale-subject, and mismatch refusal; and legacy compatibility evidence.

**Excluded:** Reviewer selection, semantic risk inference, review findings,
user-visible commit publication, or moving Git mechanics into a reviewer role.

### S8 — Retained builder role and supervisor core

**Objective:** Import the retained `slice-builder` role and the smallest
`slice-supervisor` campaign core while preserving their distinct authority,
context lifetimes, mutable-workspace admission, mediated transitions, and a
closed compatibility seam for review that has not yet migrated.

**Independently valuable outcome:** App Server can plan, authorize, execute,
gate, checkpoint, recover, and terminalize a bounded slice through compiled
roles and server services without pretending that native review integration is
already complete.

**Required consequences:**

- Supervisor campaign authority remains distinct from builder repository-domain
  judgment and user approval.
- Builder retention spans one accepted slice while useful; supervisor
  continuity spans the campaign without absorbing builder reasoning.
- Plan acceptance, mutation admission, gate execution, checkpoint creation,
  receipt finalization, and completion-commit prompting remain mediated by
  their existing owners.
- Mutable-workspace admission prevents conflicting builders.
- The review boundary is explicit, fail-closed, and compatibility-backed; it
  is not silently implemented by the supervisor or builder.
- Context lifecycle preserves role continuation without becoming campaign
  state.

**Acceptance evidence:** Planning and implementation verticals; deterministic
gate and checkpoint evidence; restart/recovery; negative authority,
concurrency, and publication tests; exact compatibility-boundary evidence; and
legacy outcome comparison excluding native review closure.

**Excluded:** Claiming a complete App Server-native campaign review loop,
automatic user-visible commits, silent scope expansion, or absorbing provider
review semantics into either workflow role.

### S9 — Provider-neutral implementation review and episode service

**Objective:** Extract one canonical provider-neutral implementation-review
contract from the semantics currently mixed into
`claude-recon-implementation`, `codex-adversarial-review`, and experimental
review harnesses, and port `independent-review-state` as the corresponding
provider-neutral review-episode service.

**Independently valuable outcome:** Adversarial code-review judgment, finding
and verdict meaning, evidence sufficiency, read-only authority, remediation
re-evaluation, and retained episode truth have owners independent of any
provider adapter.

**Required consequences:**

- Findings bind exact subjects, stable identities, evidence, observed facts,
  violated expectations, consequences, confidence, and bounded remediation.
- `acceptable_as_is`, `remediation_required`, and incomplete or limited
  outcomes retain explicit evidence requirements.
- Mutation, implementation acceptance, architecture choice, reviewer
  selection, and human authority remain outside the reviewer contract.
- The episode service preserves findings, remediation status, reviewer
  generation, pending actions, resumable provider references, replacement,
  reconstruction, uncertainty, and retirement provenance.
- Review-episode state remains distinct from hidden provider-thread context
  lifecycle and from durable-state storage mechanics.

**Acceptance evidence:** Provider-neutral contract fixtures; exact-subject and
finding validation; fresh-entry, retained-remediation, replacement, recovery,
writer-fence, and uncertainty cases; cross-provider compatibility evidence;
and independent bootstrap review.

**Excluded:** A provider adapter, a production reviewer profile, benchmark
scoring, claim publication, or an independence claim based only on process or
provider identity.

### S10 — Canonical reviewer role, minimal profile registry, and OpenRouter adapter

**Objective:** Realize the provider-neutral review contract as a canonical
read-only App Server reviewer role, establish the minimum closed registry
needed to validate production reviewer profiles, and execute one admitted
profile through a bounded OpenRouter Codex adapter.

**Independently valuable outcome:** App Server can run a revision-bound
implementation review through a selected provider without allowing provider
configuration or adapter behavior to redefine review semantics.

**Required consequences:**

- The role derives from the verified independent-reviewer environment while
  replacing its provider-specific legacy contract with the generic review
  contract and `repo-search` composition.
- Independence is established only by configuration and receipts proving fresh
  entry outside builder reasoning and mutation authority.
- The minimal registry owns approved profile identifiers, exact permitted
  model slugs, capability/schema bindings, configuration digests, enabled
  state, limitations, and the accepting authority.
- The OpenRouter adapter accepts only an admitted profile identifier and owns
  isolated `CODEX_HOME` construction, pinned provider/model configuration,
  transport provenance, failure classification, and raw event capture.
- Arbitrary or untested model slugs, undeclared capabilities, subject drift,
  and malformed output fail closed.
- MCP is used only if the isolated process cannot receive the same bounded
  meaning through internal dynamic tools; transport never becomes a semantic
  owner.

**Acceptance evidence:** Manifest and runtime-requirement satisfaction;
read-only and effect-ceiling tests; admitted/rejected profile fixtures;
isolated-home and exact-model verification; raw-event and failure receipts;
fresh-entry and retained-remediation verticals; immutable-subject evidence;
and independent bootstrap review.

**Excluded:** General reviewer admission research, routing policy, arbitrary
OpenRouter model selection, benchmark-based production approval, or calling a
role independent merely because of its name.

### S11 — Agent-instruction-review specialist composition

**Objective:** Import `agent-instruction-review` as a specialist contract that
composes with the canonical reviewer role when an immutable subject contains
materially normative agent-facing instructions.

**Independently valuable outcome:** Later role and skill migrations can receive
production-like specialist review under the corrected causal-exposure contract
without confusing instruction review with generic implementation review.

**Required consequences:**

- Applicability is explicit and does not imply selection or acceptance.
- The reviewer receives the actual effective instruction closure, including
  manifest fragments, precedence, omissions, and conditional references.
- Fresh entry and bounded retained remediation preserve truthful independence
  and reviewer-generation claims.
- Subjects and findings bind exact revisions and stable identities.
- Mutation, architecture choice, proposal acceptance, panel selection, and
  implementation approval remain unreachable.

**Acceptance evidence:** Corrected S1 regression corpus; manifest satisfaction;
read-only effect tests; applicability, no-finding, limitation, remediation,
fresh-entry, and retained-remediation fixtures; and independent bootstrap
review of the generated projection.

**Excluded:** Making the specialist the generic code-review contract, treating
every review as independent, or allowing it to certify its own implementation.

### S12 — Claim-backed findings and native supervisor review closure

**Objective:** Port the thin reviewer-facing `claim-evidence` projection over
the existing App Server claim service, publish revision-bound review findings
without transferring verdict authority, and close the supervisor's native
review-selection, delivery, remediation, checkpoint, and receipt loop.

**Independently valuable outcome:** One App Server-native campaign can execute
a bounded slice from planning through immutable, retained, claim-backed review
and terminal receipt without relying on the legacy agent-local review harness
as semantic owner.

**Required consequences:**

- Canonical claim identity, revision, lineage, reliance, authority, and store
  mechanics remain server-owned.
- Review materiality, applicability, verdict, episode status, synthesis, and
  acceptance remain with their existing owners rather than the claims service.
- The thin projection explains authorized reviewer use and limitations without
  teaching the reviewer to emulate the registry.
- The supervisor selects configured specialists; the builder evaluates and
  remediates findings; neither selects or certifies itself.
- Exact checkpoint, profile, episode, finding, claim revision, remediation,
  and terminal receipt identities remain reconstructable.
- No second local claim store, review-state owner, or context-lifecycle owner
  remains active.

**Acceptance evidence:** Claim service and compatibility suites;
concurrent-instance, restart, and capability-refusal tests; exact projection
review; one revision-bound review finding publication and reliance vertical;
one end-to-end accepted fixture slice with remediation and recovery; negative
authority tests; and legacy outcome comparison.

**Excluded:** Letting claims accept a review, automatic completion commits,
silent campaign expansion, or treating the claims service as a reviewer role.

### S13 — Reviewer research and admission evidence

**Objective:** Build the provider-neutral experimental machinery needed to
compare, understand, adjudicate, and eventually admit reviewer profiles without
turning research output into production review approval.

**Candidate responsibility sequence:**

1. Port `review-bench` as experimental evaluation machinery.
2. Define an immutable provider-neutral reviewer-turn event schema and
   normalizers for Codex, OpenRouter, Claude, Gemini, and later providers.
3. Add a versioned reviewer-turn behavior analyzer over raw traces.
4. Define a review-adjudication contract or role bound to sealed truth,
   executable adjudication, or explicitly incomplete historical truth.
5. Extend the minimal profile registry with evidence-bearing admission,
   limitations, last-tested revisions, and authorized acceptance.
6. Add review-routing recommendations only after adjudicated evidence can
   support them.

Each responsibility remains its own slice when its owner, acceptance evidence,
or experimental status differs materially.

**Independently valuable outcome:** Reviewer research can improve provider and
profile choices while production review retains exact authority, provenance,
and failure boundaries.

**Required consequences:** Raw events remain immutable; normalizers and
mechanical metrics remain reproducible; semantic behavior metrics are
versioned and attributed; incomplete truth stays incomplete; harnesses propose
admission but an authorized owner accepts it; and routing scores cannot accept
implementation.

**Acceptance evidence:** Cross-provider event fixtures; normalization and drift
tests; sealed and incomplete adjudication cases; benchmark provenance;
profile-admission receipts; routing refusal and escalation cases; and explicit
experimental-versus-production projections.

**Excluded:** Silent ground-truth manufacture, benchmark-based production
approval, reviewer self-selection, or runtime behavior scores that accept a
slice.

### S14 — Initial product-development role imports

**Objective:** Import bounded role-bearing skills that already have App Server
vertical evidence, beginning with roles whose inputs, outputs, authority, and
host capabilities are narrow enough to verify independently.

**Candidate set:** `idea-intake`, `proposal-former`, and `strategic-planner`.
Each remains its own slice if their evidence or ownership differs materially.

**Independently valuable outcome:** Product-development judgment runs through
generic compiled role packages and manifest composition rather than additional
handcrafted gateways.

**Required consequences:** Exact request, source, evidence-cutoff, and output
bindings remain enforced; host publication proves mechanics rather than
semantic quality or acceptance; logical instances use server lifecycle
services; existing role-specific code receives an explicit disposition; and
zero-output or unresolved outcomes remain truthful.

**Acceptance evidence:** One exact end-to-end vertical per role; restart and
binding evidence; negative authority tests; projection review; deterministic
output validation where applicable; and compatibility with current artifact
owners.

**Excluded:** Proposal acceptance, roadmap mutation, artifact-root migration,
or implementation authorization.

### S15 — Remaining service-backed skill imports

**Objective:** Apply the accepted service-backed pattern to retained skills
whose durable or mechanical responsibilities belong above agents, while
retaining only necessary agent-facing semantics and authorized clients.

**Candidate set:** `durable-state`, `role-scheduler`, and other S0 ledger items
whose server-service destination is accepted. Context lifecycle remains hidden
service infrastructure rather than an ordinary reviewer or agent skill.

**Independently valuable outcome:** Each accepted import removes one
agent-local state or mechanics owner without copying it behind a new path.

**Required consequences:** Service state and mechanics remain canonical at the
server boundary; model and human judgment do not move with them; authorized
clients expose only bounded operations; legacy parity or authorized difference
is explicit; and no duplicate local owner remains active.

**Acceptance evidence:** Service and compatibility suites; restart,
concurrency, and refusal tests; exact thin-projection review; bounded real
consumers; and ownership-transition evidence.

**Excluded:** Turning services into universal roles or treating durable
mechanics as cognitive work.

### S16 — Evidence-driven remaining portfolio batches

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

### S17 — Canonical ownership cutover and legacy retirement

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
