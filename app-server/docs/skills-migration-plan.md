# Skills Migration Campaign: Decomposed Sources, Runtime Roles, and Server Services

## Status and authority

This document is an implementation-campaign source for migrating Work Engine
skills into the App Server architecture. It supplies objectives, invariants,
candidate slice boundaries, dependencies, acceptance evidence, and stop
conditions. It does not itself migrate canonical ownership, accept a semantic
classification, authorize a contract change, approve a slice plan, or make a
generated projection authoritative.

Evidence cutoff: repository `HEAD`
`b4d749672990aadb51bab391a2e286b057128ea4` plus the directly inspected working
tree on 2026-08-29. The working tree already contained unrelated and ongoing
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
S8 --> S8P isolated agent worktrees and publication authority
S1 + S3 + S5 + S7 --> S9 generic review contract and episode service
S6 + S7 + S9 --> S10 reviewer role, profile registry, OpenRouter adapter
S1 + S10 --> S11 instruction-review specialist composition
S8 + S9 + S10 + S11 + claim-service evidence --> S12 native review closure
S4 + S5 + S6 + S8P + S12 --> S12E run-scoped extension bundles
S6 + S7 + S8 + S12 + accepted claim-maintenance proposal and implementation grant --> S12M claim-maintenance projection vertical
S12E --> isolated research, analysis, and pilot workloads in S13-S16
S9 through S12 --> S13 reviewer research and admission evidence
S4 + S5 + S6 + S12 --> S14 product-development role imports
S0 + S5 + S12 + service evidence --> S15 remaining service-backed imports
S6 through S15 --> S16 remaining portfolio batches
S16 --> S17 ownership cutover and legacy retirement
```

This is a dependency model, not a compulsory execution sequence. Independent
work may proceed concurrently when immutable subjects and non-overlapping
ownership make that safe.

The approved operational cutover route runs S8P before the first multi-agent
S9 development campaign. S8P does not become a semantic prerequisite of the
provider-neutral review contract: it removes the shared-worktree publication
hazard from the environment in which that contract and later slices are built.

## Approved post-S8 execution direction

This section is the durable continuation owner for the operational direction
approved on 2026-08-30. It records sequencing and cutover boundaries that must
not depend on retained conversation context.

- S8 is accepted and published. Its accepted checkpoint was independently
  reviewed with zero findings, and its 31 migration paths are byte-identical in
  published commit `419d6da3a44e4a2ba1586882f55160af49b07b67`.
- S8P is the immediate next implementation slice. After S8P acceptance, new
  multi-agent migration development moves into App Server using the retained
  supervisor and builder imported by S8.
- The first development slice used to prove that cutover is S9. The existing
  environment may continue to perform proposal adjustment, provider review,
  or other responsibilities that have not yet acquired an App Server owner;
  this compatibility seam remains explicit and fail-closed.
- S9 through S12 migrate provider-neutral review semantics, provider adapters,
  instruction-review specialization, and native review closure. S12 completes
  the native review-loop cutover; it is not a prerequisite for beginning to
  realize the development-economics benefit after S8P.
- S12E follows native review closure and supplies immutable run-scoped
  extension bundles for isolated research, analysis, pilots, and other
  non-core workloads. It does not globally install their skills, admit them as
  production roles, or change the meaning of their owning protocols.
- S13 through S17 remain the broader evidence, portfolio, ownership-cutover,
  and legacy-retirement path. S8P and S12E must not silently absorb those
  semantics.

The S8P coordination boundary is deliberately precise:

- Per-agent local worktrees isolate checkouts, indexes, staging, private
  commits, and `HEAD` without cloning repository history or performing network
  operations.
- The durable chatboard carries messages and advisory claims during the
  transition, but does not confer mutation authority or provide host-enforced
  mutual exclusion. App Server leases, resource-specific fencing tokens, and
  mutation-authority admission are required wherever collision must fail
  closed.
- Worktree isolation does not isolate campaign ordinals, paid-provider budget,
  fixed output or configuration directories, ports, databases, caches, or
  other hidden operational resources. These resources must be operation-
  namespaced when possible and explicitly mediated when genuinely shared.
- Resources that cannot enforce fencing must be isolated or classified as
  advisory-only; a prior token check separated from the mutation is not an
  authoritative fence.

Test and research harnesses use the following compatibility contract rather
than acquiring publication authority merely because they run inside an agent
worktree:

1. Launches receive an explicit repository or assigned-worktree root and an
   immutable subject revision; ambient current-directory selection is not an
   authoritative subject binding.
2. Mutable outputs, provider configuration, and execution scratch space are
   operation-namespaced, while genuinely shared resources are declared for
   mediated admission.
3. Each harness declares whether it is read-only, produces private mutable
   artifacts, or requests canonical publication.
4. Ordinary isolated unit tests may continue to use temporary directories and
   repositories; they do not each require an App Server worktree.
5. Promotion of research results or generated artifacts to a canonical branch
   uses the same accepted-checkpoint, integrated-tree validation, fencing, and
   publisher authority as agent-authored code.

The operational sequence is therefore:

```text
published S8
    -> implement and accept S8P
    -> run the first multi-agent S9 campaign through App Server
    -> complete S9-S12 native review closure
    -> add S12E run-scoped extension bundles for isolated non-core workloads
    -> close the post-S12E production-supervisor inhabitation gap
    -> finish S13-S17 portfolio cutover and legacy retirement
```

## Post-S12E operational inhabitation correction

This section is the durable continuation owner for a migration correction
accepted on 2026-09-01 after the first manual attempt to inhabit the migrated
environment. It preserves the scoped acceptance of S12 and S12E while closing
an operational inference that their original acceptance evidence did not
establish.

S12 completed its scoped native review-loop closure, and S12E completed its
scoped sealed run-extension machinery. Neither slice owned or tested the full
production route from the pinned Codex TUI, through the proxy and executable
generation, into a normal `slice-supervisor` domain turn with every capability
required by that role backed by a real stable-host owner. Their accepted
component evidence therefore remains valid, but it was insufficient evidence
that the migrated environment was ready to host ordinary supervisor-led
development or an isolated pilot. Do not reinterpret component acceptance as
operational inhabitation evidence.

The attempted linguistic-register pilot exposed the missing route before real
pilot execution. A newer ambient Codex CLI was initially selected instead of
the repository-pinned `0.149.1` client; generation-worker failures lost useful
bounded provenance; and, after the client mismatch was removed, operator
commands could enumerate roles while a normal turn still failed in generation
dispatch. The production `slice-supervisor` manifest requires eleven host
capabilities, but the executable generation registered none of their real host
owners. Continuing S13 without closing that boundary would make successful
fixtures stand in for the production route and could strand campaign state or
authority behind tools that the role cannot invoke.

The immediate proxy and worker reliability repair is published as
`4ddb44664697843439874ffc43560894247f7a39`. The remaining correction is a
bounded A1-through-A4 compatibility closure:

1. **A1 — stable supervisor host-effect seam.** Define a closed, validated,
   generation-bound protocol from replaceable executable-generation workers to
   a stable host-owned runtime. Reject malformed lookalikes without falling
   through to another effect route, preserve bounded diagnostics, and dispose
   the worker before its stable runtime. A1 intentionally registers zero of
   the eleven supervisor capabilities. Its independently reviewed accepted
   checkpoint is
   `refs/work-engine/checkpoints/proxy-supervisor-repair-20260901/slice-2/accepted`,
   tree `cdecce3e4968a1b2c8241e81330a61db20d19e01`.
2. **A2 — six campaign and control capabilities.** Implement real stable-host
   owners for `preflight`, `lifecycle_control`, `receipt_finalization`,
   `checkpoint_lifecycle`, `completion_offer`, and `resume`, with closed
   operation envelopes and thin generation-pinned clients. Human completion
   decisions remain exact durable inputs: `completion_offer` may record a
   human `create` decision as `create_authorized`, but it cannot publish Git;
   `completion_publication` owns that later mutation. A2 does not emulate the
   other five capabilities or claim first-turn readiness.
3. **A3 — workspace and publication capabilities.** Implement
   `workspace_coordination`, `worktree_lifecycle`, `canonical_publication`, and
   `completion_publication` through isolated worktrees and authoritative
   mutation admission. This slice must close the observed transitional gap in
   which the legacy completion adapter refuses a dirty human checkout while
   the canonical publisher cannot move its currently checked-out branch.
   Accepted work must be publishable without staging, absorbing, moving, or
   deleting unrelated human files, and stale parent or fencing state must fail
   closed.
4. **A4 — strategic composition and inhabitation proof.** Implement
   `strategic_reconciliation`, compose all eleven real capabilities into the
   production supervisor environment, and run the pinned proxy/TUI vertical.
   This is the first slice allowed to claim that the migrated supervisor is
   operationally inhabitable.

At this amendment's 2026-09-01 evidence cutoff, A1 is privately accepted and
A2 is implementation-ready but not checkpointed, independently reviewed, or
accepted. Its implementation and deterministic-gate receipts are
`.git/work-engine/app-server-development/proxy-supervisor-repair-20260901/a2-implementation-ready-v1.json`
and `a2-deterministic-gate-v1.json`, with SHA-256 digests
`8ce62bad981b835f6e237ae81d459552d7fa16b75618aa0926f25515ce573a8d`
and `9a34d3c2bbae877397cde98258596dccde1bee3f4976783165cbbedb6b621240`.
A3 and A4 have not started. Later evidence may advance this status but must not
rewrite these recorded facts.

The A1-A4 labels describe this corrective compatibility closure; they do not
renumber the canonical migration slices, widen S12/S12E ownership, or absorb
S13 research semantics. Each accepted sub-slice remains an immutable handoff
to the next. Missing capability registrations, no-op adapters, generic command
execution, caller-selected backend paths, or fixture-only composition cannot
satisfy the closure.

The operational-inhabitation gate requires evidence that the repository-pinned
Codex `0.149.1` client can initialize through the production proxy, attach the
production `slice-supervisor`, and complete its first normal domain turn under
the active executable generation; all eleven required capabilities resolve to
their named real owners; generation reload and proxy restart preserve or
truthfully recover stable state; malformed, stale, unauthorized, and
unavailable operations fail closed with bounded diagnostics; and the user's
ordinary checkout, index, unrelated files, and network authority remain
untouched. Operator-only commands such as role enumeration, isolated service
tests, or a turn that never reaches supervisor capability use are insufficient
substitutes.

S13 remains paused until that gate passes. Its first implementation
responsibility is preserved at immutable candidate
`refs/work-engine/checkpoints/skills-migration-app-server-s9-20260830/slice-6/candidate-1`,
commit `1711a951805b26b492a69192ef2f29a9d9f5cb02`; it resumes at review and
acceptance rather than being rebuilt. Real linguistic-register pilot execution
remains separately governed by its pilot protocol and human approval after the
sealed bundle and end-to-end dry run succeed. Neither the corrective closure
nor successful inhabitation grants pilot execution or production authority.

After A1 through A4 were published and the pinned client completed a normal
attached supervisor turn, recovery of the preserved S13 candidate exposed a
second, narrower inhabitation gap. The read-only supervisor could recover the
review-ready campaign and its selected obligations, but the stable host did not
compose S12's accepted native-review service owners or expose a mediated review
capability. The role therefore could not create the authority-bound reviewer
session, enter the configured provider, or durably read the resulting episode,
finding, and reliance state without an outer-session shell actor.

The corrective `supervisor-native-review-hosting` slice closes only that host
composition boundary. It adds a stable-host capability for an exact already
selected immutable obligation; the host derives the subject, authority,
provider profile, native Claude Code session, command, tools, and private
SQLite locations. Provider-entry admission precedes launch, ambiguous admitted
outcomes cannot replay automatically, and remediation resumes only the exact
recorded session. The supervisor receives no shell, filesystem, credential,
model-routing, reviewer-selection, finding-evaluation, review-acceptance, or
campaign-acceptance authority. This production path uses direct Anthropic
through the native Claude Code harness; OpenRouter, batch, paired calibration,
and Codex/OpenRouter reviewer execution remain distinct routes and are excluded
from this closure. S13's candidate and ten deterministic checks remain
unchanged and resume at their existing review/acceptance boundary after this
correction is accepted.

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

## Staged development cutover amendment

The user approved this bootstrap and cutover boundary on 2026-08-29. The
migration does not need to reproduce proposal intake before App Server can
become the development environment. Existing accepted proposal and roadmap
revisions may enter through their exact durable owners. When proposal meaning
must change, formation, evaluation, and approval may continue in the legacy
environment and supply a new immutable authorized revision; that bridge does
not transfer proposal authority to App Server or make legacy conversation a
runtime dependency.

The cutover is staged:

- S5 establishes hidden context-lifecycle participation for compiled roles,
  and the existing App Server claim service remains the canonical claim owner.
- S7 remains a prerequisite for development orchestration because supervisor
  and reviewer consumers require immutable attributed subjects and
  deterministic physical change profiles.
- Before S8, verify that the existing claim service is reachable through the
  environment boundary required by one real development consumer. Add an
  earlier thin claim projection only when that observed consumer cannot use the
  current service boundary; do not preemptively move S12 review semantics or
  claim authority into the builder.
- S8 may move retained builder and supervisor execution into App Server while
  preserving the explicit compatibility-backed legacy review seam.
- The user approved S8P on 2026-08-30 as the first post-S8 operational cutover
  slice. Before multi-agent development proceeds, App Server gives each agent
  or slice a local Git worktree and index, then confines canonical branch
  publication to an isolated integration worktree and server-owned publisher.
  A single-agent development pilot may exercise this slice itself; the broader
  coordination substrate does not become a prerequisite for S9.
- S12's accepted end-to-end claim-backed review vertical is the retirement
  gate for the legacy development workflow. S9 alone does not establish native
  reviewer execution, specialist composition, or claim-backed closure.

Development of the remaining workflow may therefore occur inside the App
Server environment while the legacy workflow bootstraps planning, review, and
acceptance boundaries that are not yet native. This is an explicit temporary
architecture, not evidence that those boundaries have already migrated.

Expected cost and latency savings are hypotheses to measure rather than
acceptance assumptions. Preserve comparable legacy baselines for model/provider
tokens, charged cost, wall time, context recovery, review repair iterations,
and user interventions, then compare like-for-like App Server slices as native
boundaries become active. Lower cost or latency never substitutes for semantic,
authority, or validation evidence.

### Coordination substrate successor note

The current working-tree cross-session chatboard and fail-closed
active-campaign registry form the first operational coordination slice, not the
complete App Server coordination architecture. This observation remains
provisional until that infrastructure has its own exact checkpoint or commit
and acceptance evidence. The slice provides durable messages, expiring
advisory resource claims, compare-and-swap conflict handling, and guarded
campaign registration. It addresses immediate shared-tool collisions when
every caller follows the repository entry contract, but does not establish
automatic interception, fencing, observation of external effects, or
model-context delivery.

The App Server and context-lifecycle successor must coordinate both semantic
scope and typed physical operational resources. Disjoint source ownership is
insufficient when agents can still collide on fixed directories, generated
indexes or caches, ports and sockets, database files, Git index or branch
state, tool-global configuration, rate limits, or named remote campaigns. The
resource model must distinguish independently governable identities and access
modes, such as an exclusive Git index or branch ref, an exclusive canonical
output directory or port, reserved provider capacity, and advisory or
shared/exclusive source scopes; it must not collapse ordinary disjoint edits
into a global workspace lock.

Prefer session- and operation-namespaced resources where possible. Genuinely
singular resources require expiring leases whose grants carry resource-specific
monotonically increasing fencing tokens. Validation by an adapter immediately
before mutation is insufficient because lease expiry or supersession can occur
between validation and use. The mutation authority must atomically admit the
operation with its token and reject absent, expired, superseded, or mismatched
admissions. Once admitted, completion must occur within an explicitly defined
transactional boundary. Resources that cannot enforce fencing, especially
arbitrary filesystem writes, must be isolated or namespaced, mediated through
an enforcing adapter, or explicitly classified as advisory-only.

Git development uses local worktrees created from the existing repository, not
per-slice clones or implicit network fetches. Agent worktrees share the local
object database and history while retaining independent files, indexes, and
`HEAD` state. Builders may stage and create private commits only inside their
assigned worktrees. Accepted checkpoints remain immutable publication inputs;
they do not authorize movement of a canonical branch ref.

Canonical publication uses a separate operation-namespaced integration
worktree. A server-owned publisher applies one accepted checkpoint to the
observed current branch tip, records or returns semantic conflicts to the
responsible owner, reruns validation affected by reconciliation, and admits the
branch-ref mutation atomically with its fencing token and expected parent.
Unrelated branch advancement may require reconciliation and new validation; it
does not invalidate another agent's private workspace or permit the publisher
to absorb unmanifested files. The user's ordinary checkout and index remain a
human workspace rather than shared agent staging or publication machinery.

Keep intent, resource authority, observation, and judgment-bearing messages
distinct. Intent is advisory; an acquired fenced lease supplies a candidate
admission rather than mutation authority by itself; filesystem, Git, service,
and provider receipts establish what actually happened; and only messages that
can invalidate or redirect an agent's reasoning should enter model context.
Completion must bind the intent, exact resource set, lease and fencing tokens,
starting repository generation, and observed resulting generation, while
append-only typed events support rebuildable projections for current leases,
active agents, repository generation, and relevant pending messages. Messages,
acknowledgements, and advisory claims never confer mutation authority.

Host adapters should eventually register intents and resource claims before
tool entry, submit admissions to the authoritative mutation boundary, reject or
delay conflicts, observe completion and release, and project only causally
relevant changes at safe turn or context-lifecycle boundaries. The app server
owns coordination authority; mediated tools enforce it; agent-facing skills
teach declaration and conflict interpretation without becoming authority.
Until that successor is implemented and accepted, the lightweight board
remains an explicit compatibility and safety mechanism; its existence does not
prove that hidden resources are declared, that all callers are guarded, or
that multi-agent coordination has migrated into App Server.

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
- Any later semantic characterization of authority, persistence, concurrency,
  security, topology, or other change meaning is a separately attributed and
  versioned projection. It may consume the physical profile but cannot acquire
  its deterministic identity or silently revise its observations.
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
- The accepted plan can reference an integrity-bound expected semantic-impact
  artifact, while the immutable candidate and accepted checkpoint preserve the
  actual attributed before/after subject, changed-file manifest, and physical
  change-profile identity. Expected and observed scope remain distinct and
  lossless even when no claim-maintenance consumer is active.
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

### S8P — Isolated agent worktrees and publication authority

**Objective:** Remove the shared-worktree and shared-index coupling from
App Server development while preserving immutable checkpoint, validation,
human-approval, and canonical branch-publication boundaries.

**Independently valuable outcome:** Multiple agents can edit, stage, test, and
produce private checkpoint commits concurrently without another agent's stage,
commit, or branch advancement corrupting their workspace or publication
preconditions.

**Required consequences:**

- App Server creates local, operation-namespaced Git worktrees from explicit
  repository baselines without cloning history or performing implicit network
  operations.
- Each agent or slice receives an independent checkout, index, and `HEAD`;
  generated outputs, caches, and other mutable physical resources are also
  namespaced or explicitly mediated when sharing is safe.
- Agent commits remain private development or checkpoint artifacts. Only the
  publisher owns admission to a configured canonical branch ref, and neither a
  worktree lease nor an accepted checkpoint grants publication authority.
- Publication begins from an accepted checkpoint and the observed current
  branch tip in a separate integration worktree. It preserves the exact
  attributed manifest, refuses unmanifested content, and distinguishes clean
  mechanical reconciliation from conflicts requiring builder or human
  judgment.
- Validation is bound to the integrated result rather than a stale pre-merge
  tree. Branch admission checks the fencing token and expected parent at the
  mutation authority, then records the starting tip, accepted checkpoint,
  resulting commit and tree, validation receipt, and observed branch
  generation.
- Cleanup removes disposable worktrees and per-operation mutable state without
  deleting accepted checkpoints, private evidence refs, shared Git objects, or
  user-authored work.
- Explicit fetch and push remain separately authorized network operations;
  local worktree creation, use, integration, and cleanup require no network.

**Acceptance evidence:** Two concurrent agent worktrees that stage and commit
without index or file interference; publication after unrelated branch
advancement; clean automatic reconciliation and semantic-conflict refusal;
stale, expired, superseded, wrong-resource, and wrong-parent fencing cases;
unmanifested-file and dirty-human-index preservation; validation bound to the
integrated tree; restart recovery; cleanup retention; and proof that worktree
creation performs no network operation.

**Excluded:** General-purpose merge policy, automatic semantic conflict
resolution, implicit fetch or push, publication without explicit human or
workflow authority, treating advisory source-scope claims as mutation
authority, or completing the broader host-adapter and selective-context
coordination substrate.

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
- A profile records requested model, provider constraints, reasoning, effective
  instructions, tools, output schema, and isolated-home configuration. The run
  receipt records observed model, resolved provider, serving variant or
  explicit unknowns separately; configured identity never substitutes for
  execution evidence.
- The OpenRouter adapter accepts only an admitted profile identifier and owns
  isolated `CODEX_HOME` construction, pinned provider/model configuration,
  transport provenance, failure classification, and raw event capture.
- Provider admission uses and records a freshness-bound canonical projection
  of the live catalog evidence relevant to the admitted profile. A known
  missing required capability or violated routing constraint fails closed;
  catalog presence does not guarantee inference admission or replace the
  actual execution receipt.
- Raw event capture follows an explicit data classification, access, retention,
  redaction or omission, and tamper-evidence contract. When exact retention is
  not authorized, the adapter emits an authenticated bounded projection and
  records what was omitted rather than silently discarding evidence.
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
- Claim reads and relevant-revision projections remain provider-neutral. They
  may later be joined to a replaceable structural projection without making a
  graph provider, retrieval result, or cache the canonical claims owner.
- No second local claim store, review-state owner, or context-lifecycle owner
  remains active.

**Acceptance evidence:** Claim service and compatibility suites;
concurrent-instance, restart, and capability-refusal tests; exact projection
review; one revision-bound review finding publication and reliance vertical;
one end-to-end accepted fixture slice with remediation and recovery; negative
authority tests; and legacy outcome comparison.

**Excluded:** Letting claims accept a review, automatic completion commits,
silent campaign expansion, or treating the claims service as a reviewer role.

### S12E — Run-scoped extension bundles for isolated non-core workloads

**Objective:** Allow one admitted App Server run to attach an immutable,
compiled extension bundle containing the skills and bounded capabilities
needed by a research, repository-analysis, pilot, or other non-core workload
without mutating the core runtime manifest or global skill catalog.

**Independently valuable outcome:** The stable App Server core can host
specialized work such as the linguistic-register pilot while keeping
experimental protocols, dependencies, artifacts, and failure states isolated
from production role machinery.

**Required consequences:**

- A bundle binds exact source revisions and digests for every skill,
  instruction closure, role overlay, capability request, schema, fixture, and
  adapter it carries. Ambient skill directories and conversational references
  are not installation evidence.
- Attachment stages the bundle into one private executable generation with an
  explicit repository subject, checkout, artifact root, scratch root,
  retention policy, credential policy, and namespaced operational resources.
  Activation occurs only at the existing fenced generation boundary.
- Admission validates compiler/runtime requirements, instruction precedence,
  capability and effect ceilings, version conflicts, provider requirements,
  and every host adapter requested by the bundle. A bundle may narrow an
  admitted run but cannot grant itself filesystem, network, publication,
  claims, review, human, or production-admission authority.
- Bundle capabilities are resolved through a run-scoped server registry rather
  than added to the hard-coded core catalog. Missing, stale, conflicting, or
  unmediated capabilities fail closed.
- The lifecycle is explicit and recoverable: requested, validated, admitted,
  activated, executed, artifact-sealed, detached, and cleaned or retained by
  policy. Every terminal receipt distinguishes semantic workload outcome from
  transport success, artifact publication, and later production admission.
- Dependencies use sealed local projections or declared caches when possible;
  network installation is a separately authorized, receipted effect rather
  than an implicit consequence of starting a run.
- Research or pilot artifacts remain owned by their experiment package.
  Promotion into canonical repository or product state still requires the
  ordinary proposal, accepted-checkpoint, review, fencing, and publication
  authorities.
- `linguistic-register-pilot` is the first intended cross-domain proof, but
  S12E only supplies its isolated execution environment. The pilot's own
  protocol, profile-separability claims, stopping rules, and production
  exclusions remain authoritative and must be admitted separately.

**Acceptance evidence:** One sealed fixture bundle and one rejected bundle;
compiler and instruction-closure verification; run-local capability admission
and refusal tests; effect-ceiling and credential-isolation tests; fenced
activation and restart recovery; dependency-cache and no-network fixtures;
artifact sealing, detachment, cleanup, and retained-artifact receipts; proof
that the core manifest and global catalog are unchanged; and one dry-run
linguistic-pilot fixture that cannot publish or acquire production authority.

**Excluded:** Global installation as a side effect of one run; arbitrary
package-manager or network execution; loading unsealed local skills; allowing
an extension to replace core semantic owners; treating successful execution as
profile, reviewer, claim, or production admission; or making experimental
packages prerequisites of ordinary coding roles.

### S12M — Conditional claim-maintenance projection vertical

**Status and authority gate:** This is a conditional companion slice, not an
authorized consequence of the skills-migration campaign. It may enter campaign
planning only after an exact revision of
[`claim-maintenance-and-reliance-propagation`](../../proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/proposal.md)
is evaluated, accepted, and granted implementation authority by its named
decision owner. Merely recording this dependency does not accept that proposal,
authorize mutation of Codebase Memory, or make S12M a prerequisite for S13
through S17 or for campaign completion.

**Objective:** After S12 proves the production claim-backed review vertical,
exercise one provider-neutral, generation-bound claim-maintenance projection
from exact planned and observed slice evidence through bounded impact retrieval,
canonical `may_affect` nomination, authorized refresh, and exact-revision
reliance consequences.

**Independently valuable outcome:** Work Engine can revalidate one affected
claim neighborhood after a repository or documentation change without making
builders maintain a graph, making structural reachability semantic authority,
or introducing a second canonical claim registry.

**Required consequences:**

- Canonical claims, authored relationships, nominations, judgments, reliance,
  and obligations remain owned by the production claim-evidence boundary and
  their authorized domain workflows.
- A deterministic compiler may derive a cacheable projection specification
  from exact canonical records. Losing that specification requires
  recompilation, not semantic recovery or human reconstruction.
- The current structural provider is replaceable. If Codebase Memory is used,
  synchronization protects projection-owner namespaces, binds the exact
  repository subject and structural index generation, evaluates projections
  before atomic generation publication, and retains unresolved endpoints
  truthfully.
- `repo-search` remains the role-facing evidence contract. Results include
  projection revision, structural generation, repository subject, freshness,
  coverage, exclusions, failures, truncation, unresolved endpoints, and
  unmapped candidates within the declared bounds.
- The code-change profile owns immutable physical and structural before/after
  observations and identity reconciliation. It does not publish semantic
  nominations or refresh judgments.
- Retrieved candidate impact, a canonical `may_affect` nomination, an
  authorized refresh judgment, any resulting claim revision, reliance
  reconsideration, delivery, acknowledgement, and semantic completion remain
  separately addressable consequences.
- At least one code-to-documentation and one documentation-to-code path exercise
  planned-versus-actual comparison, stable identity across ordinary movement,
  explicit unresolved matching, and one newly unmapped candidate.

**Acceptance evidence:** Exact proposal decision and implementation grant;
immutable subject and change-profile fixtures; projection-owner collision and
parser-structure protection tests; stale-generation refusal; full-rebuild
atomic-publication and unresolved-endpoint recovery; deterministic projection
recompilation; claim-aware `repo-search` receipts; retained-unchanged and
changed refresh paths; exact reliance and selective-reopening evidence; and one
restart- and context-replacement-safe end-to-end vertical across both change
directions.

**Excluded:** Making Codebase Memory canonical semantic state or a permanent
provider doctrine; automatic semantic invalidation, causality, applicability,
reopening, role activation, or scheduling from graph reachability; builder-owned
projection maintenance; silent newest-revision selection; or widening the
active migration campaign without its owning amendment and authorization.

### S13 — Reviewer research and admission evidence

**Objective:** Build the provider-neutral experimental machinery needed to
compare, understand, adjudicate, and eventually admit reviewer profiles without
turning research output into production review approval.

**Candidate responsibility sequence:**

1. Port `review-bench` as experimental evaluation machinery.
2. Define an immutable provider-neutral reviewer-turn event schema and
   normalizers for Codex, OpenRouter, Claude, Gemini, and later providers while
   preserving provider payloads or explicit authorized omissions.
3. Recover the historical AGY experiment evidence from Codex rollout telemetry
   and AGY conversation stores into immutable, source-bound attempt artifacts
   before either source is pruned or reinterpreted.
4. Add a versioned reviewer-turn behavior analyzer over raw traces, keeping
   mechanical observations separate from semantic phase or evidence-use
   interpretations and binding every derived interpretation to its evaluator,
   inputs, confidence, and failure state.
5. Define a review-adjudication contract or role bound to sealed truth,
   executable adjudication, or explicitly incomplete historical truth, with
   truth completeness and later adjudication revisions visible.
6. Extend the minimal profile registry with evidence-bearing admission,
   limitations, last-tested revisions, and authorized acceptance.
7. Add review-routing recommendations only after adjudicated evidence can
   support them; keep economic ordering such as free-first and the configured
   high-assurance escalation provider as replaceable policy rather than review
   doctrine.
8. Build a versioned reviewer-coverage map as a rebuildable projection over
   exact change profiles, reviewer configurations, truth, results, scoring,
   and defect-class opportunities. Use production frequency, consequence,
   current review cost, uncertainty, and expected information value to propose
   active benchmark candidates, and estimate reviewer combinations only from
   observed joint evidence with failure correlation visible.
9. Pilot runtime supervision only under a distinct protocol after descriptive
   behavior evidence exists. A warning, forced pass, challenge, or escalation
   changes the reviewer configuration and cannot be pooled with an
   observational baseline.

Each responsibility remains its own slice when its owner, acceptance evidence,
or experimental status differs materially.

**Historical AGY evidence recovery:** The existing repository cases, Codex
rollout records, and AGY conversation databases jointly retain the earlier
Gemini attempts, admitted receipts, tool traces, latency and token usage, and
failure evidence, but the runner did not publish repository-owned Review Bench
result or scoring artifacts. Recovery must bind every extracted artifact to the
exact source path, source digest, extraction implementation and version,
reviewer configuration, immutable subject, and observed limitations. Preserve
raw evidence or an explicitly authorized bounded projection independently from
derived metrics. A completed and admitted review may become a reconstructed
result with reconstruction confidence visible; rejected output, timeout,
quota exhaustion, capability violation, and other incomplete attempts remain
attempt artifacts and cannot be promoted retroactively into reviewer results.
Later adjudication and scoring are separate artifacts and must not overwrite
the recovered observation. At minimum, recover both admitted Flash runs, the
Flash false-acceptance comparison, the Pro High non-termination evidence, the
rejected Pro Low attempt, and the later native-agent and quota-failure attempts
before launching another paid provider comparison.

**Deferred Gemini 2.5 comparison:** After the S10 execution boundary and the
minimum S13 experiment machinery run inside App Server, evaluate the currently
catalogued OpenRouter candidates `google/gemini-2.5-flash` and
`google/gemini-2.5-pro` against one sharp immutable, independently adjudicated
case before expanding either configuration. This is candidate research, not
profile admission. The experiment has these constraints:

- Preserve two distinct reviewer configurations and never pool their results:
  Codex/App Server through the bounded OpenRouter adapter measures the intended
  execution substrate, while AGY through an inspected compatibility mechanism
  measures comparability with the earlier AGY Gemini experiments.
- Treat the AGY compatibility route as disposable experimental infrastructure.
  Its exact source revision, installation mutations, request and response
  translations, credential access, provider routing, and observed model must be
  inspected and receipted before use. It cannot become a production adapter or
  establish harness equivalence merely because a request completes.
- Bind each run to the same immutable subject and truth revision used for its
  comparator. Distinguish defect localization, causal interpretation, decision
  polarity, false findings, false exoneration, non-termination, and result
  admission; transport success is not reviewer evidence.
- Begin with Flash as the low-cost capability gate, then run Pro only when the
  preceding result can answer a comparison question. Use provider-default
  reasoning first and vary the reasoning budget only to test a named
  hypothesis. Replicate a promising configuration before broadening the case
  set, and stop a configuration after a reproduced false-exoneration or other
  disqualifying failure unless a subsequent run tests a specific correction.
- Apply a hard USD 8 experiment ceiling. After every paid run, record actual
  input, reasoning, output, latency, provider, serving identity or explicit
  unknowns, and charged cost before admitting another run. Price estimates and
  catalog capabilities are freshness-bound planning evidence, not durable
  properties of a model slug.
- Run a translation control through the AGY compatibility route only when an
  admitted native AGY result exists for a meaningful comparison and the
  additional spend has decision value. An unavailable historical result or an
  unadmitted prior attempt cannot establish proxy equivalence.

Execution remains intentionally deferred until the App Server role, isolated
provider adapter, immutable event capture, and minimum adjudication path exist.
No compatibility installation or paid inference is part of the migration
slice that records this plan.

**Independently valuable outcome:** Reviewer research can improve provider and
profile choices while production review retains exact authority, provenance,
and failure boundaries.

**Required consequences:** Raw events or authorized bounded projections remain
immutable and confidentiality-classified; normalizers and mechanical metrics
remain reproducible; semantic behavior metrics are versioned and attributed;
physical and semantic change characterizations remain distinct; inference,
transport, result admission, and later adjudication are orthogonal states;
sealed-defective, sealed-clean, incomplete historical, and operational cases
retain their different truth ceilings; incomplete truth stays incomplete;
requested reviewer configuration remains distinct from observed execution
identity; coverage is declared only for a named reviewer configuration, region
definition, defect-class opportunity set, consequence ceiling, protocol, and
evidence cutoff using conservative lower bounds for desirable outcomes and
upper bounds for harmful outcomes; repeated attempts do not inflate effective
sample count or case diversity; harnesses propose admission but an authorized
owner accepts it; and coverage, routing, or runtime-behavior scores cannot
accept implementation.

**Acceptance evidence:** Cross-provider raw-event and authorized-redaction
fixtures; normalization, derivation, and drift tests; orthogonal execution,
admission, and adjudication transitions; sealed-clean, sealed-defective,
incomplete-historical, and operational adjudication cases; requested-versus-
observed identity fixtures; benchmark provenance; profile-admission receipts;
routing refusal and escalation cases; sparse-region, stale-evidence,
truth-ceiling, repeated-case, defect-opportunity, known-failure, and
joint-reviewer overlap fixtures; active benchmark selection receipts;
observational-versus-supervised protocol separation; and explicit
experimental-versus-production projections.

**Excluded:** Silent ground-truth manufacture, benchmark-based production
approval, reviewer self-selection, or runtime behavior scores that accept a
slice; a universal change-difficulty score; a universal inspection-coverage
threshold; or permanent doctrine naming one price tier or provider as the
required first or escalation route; raw attempt count treated as independent
sample size; or reviewer-combination coverage inferred by unioning marginal
scores without joint evidence.

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
- [`../../proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/proposal.md`](../../proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/proposal.md)
  owns the formed but not yet accepted phase-two claim-maintenance consequence;
  its repository-integration evidence informs S12M without authorizing it.
- [`../../docs/workflow-invariants.md`](../../docs/workflow-invariants.md) and
  [`../../docs/agent-environments.yaml`](../../docs/agent-environments.yaml)
  remain the current canonical graph inputs until an explicit ownership
  migration says otherwise.
- [`../../skills/agent-environment-graph/SKILL.md`](../../skills/agent-environment-graph/SKILL.md)
  governs validation, projection, drift, and semantic-judgment handling for the
  current environment graph.
- [`../../skills/agent-instruction-review/SKILL.md`](../../skills/agent-instruction-review/SKILL.md)
  governs revision-bound review of effective agent-facing instruction text.
