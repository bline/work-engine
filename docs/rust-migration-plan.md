# Rust migration and consolidation plan

Recorded 2026-10-07 from the app-server assessment and the user's request to
preserve the plan. Rust is the target for new first-party executable components,
as recorded in [Rust development direction](rust-development.md). The proposed
expansion covers Work Engine's app-server control plane and its legacy bridges.
Its main value is clearer ownership, durable transitions, recovery and shared
contracts. Performance gains remain unmeasured.

This is a planning recommendation, not an accepted implementation campaign or
authority to migrate live state, retire an owner, or deploy a replacement. The
[lifecycle design](../experiments/context-lifecycle-rust/DESIGN.md) and
[lifecycle implementation plan](../experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md)
own that replacement's architecture, S0–S10 slices and acceptance boundaries.
The broader [skills migration plan](../app-server/docs/skills-migration-plan.md)
retains its existing authority and per-capability dispositions. This plan does
not resume the paused campaign or replace its acceptance evidence.

## Priorities and complexity

Value rates the expected reliability and maintenance benefit. Complexity includes
behavioral parity, external effects, persisted-state compatibility, recovery and
cutover. These are relative judgments, not delivery estimates. A short adapter
can hide a large legacy implementation; line count alone understates that work.

| Area | Value | Complexity | Proposed treatment |
| --- | --- | --- | --- |
| Context lifecycle, custody, admission and effect accounting | Very high | Very high | Continue the owned replacement; consolidate transition authority in the durable lifecycle owner. |
| Provider transport, process supervision and turn execution | Very high | High | Develop the bounded lifecycle profile alongside the replacement; share proven supervision mechanics with later builders and reviewers. |
| Shared identities, time values, encodings, errors and wire contracts | High | Low to medium | Establish a narrow common foundation with versioned compatibility fixtures. |
| SQLite opening, transactions and migration mechanics | High | Medium | Share infrastructure while retaining domain schemas, validation and transaction ownership. |
| Executable-generation management and dispatch | High | Very high | Migrate as a separately owned deployment and draining mechanism, explicitly coordinated with lifecycle. |
| Workspace coordination, fencing and Git publication | High | High | Migrate after its required common contracts are stable; preserve external-effect reconciliation. |
| Review episodes and review orchestration | High | Medium to high | Use episode persistence as a bounded early domain migration; move broader execution with the provider runtime. |
| Claim-evidence service | High | High | Preserve historical identities, authority, lineage, establishment and reliance semantics. |
| Slice-campaign service and host effects | High | Very high | Separate campaign transitions from execution; migrate against stable dependency contracts. |
| Manifest, skill and extension compilation | Medium | Medium | Move deterministic compilation to Rust while retaining declarative source formats and model-owned judgments. |
| Operator UI and CLI | Medium to high | Medium | Continue Rust UI work in parallel through public schemas, clients and projections. |
| Small delivery adapters and legacy wrappers | Low individually | Variable, often high end to end | Replace with the underlying capability; avoid spending a slice on a syntax-only wrapper port. |

## Consolidation boundaries

### Lifecycle authority and execution

One durable lifecycle owner per logical subject governs admission, transition
ordering, continuation custody, effect accounting and recovery classification.
The executor performs committed work and reports facts; semantic and provider
owners retain their own acceptance and evidence meanings. The selected reducer,
store and executor architecture belongs to the lifecycle design.

The shared production path should also serve controlled tests, crash tests and
proof scenarios. Substituting an external peer is useful; a second test-only
lifecycle controller would leave production ordering and teardown untested.
The [legacy failure lessons](../experiments/context-lifecycle-rust/DESIGN_EVIDENCE.md#lessons-from-the-current-effort)
remain inputs to acceptance, rather than a list of callbacks to translate.

### Provider and process supervision

Common machinery can own child handles, task registration, bounded output,
request correlation, cancellation requests, process exit observation and cleanup.
The [review-subject bridge](../app-server/src/services/review-subject/legacy-backend-adapter.mjs)
and [supervisor bridge](../app-server/src/services/slice-campaign/legacy-control-adapter.mjs)
are concrete current consumers of similar invocation mechanics.

Codex and Claude retain distinct adapters for authentication, session continuity,
protocol capabilities and admissible evidence. Local cancellation or process exit
does not establish remote settlement. Generalization follows a second concrete
consumer; a universal provider or tool platform is not a lifecycle prerequisite.

### Shared persistence and encoding

The proposed common libraries cover validated value types, explicit time units,
versioned encodings, database opening/hardening and small transaction utilities.
Review episodes, campaigns, workspace coordination and claims retain their own
schemas, reducers, revision meanings and publication authority. A shared workspace
or persistence library does not require one process or one database.

Encoding compatibility is a migration boundary. Current
[claim-evidence canonical JSON](../app-server/src/services/claim-evidence/identity.mjs)
orders keys by Unicode code point and appends a newline, while
[workspace](../app-server/src/services/workspace-coordination/contract.mjs) and
[campaign](../app-server/src/services/slice-campaign/contract.mjs) encodings use
different rules. Replacing them with one encoder can change persisted identities.
Versioned codecs and cross-language golden vectors preserve the old contracts;
any new identity format needs an explicit migration and reference strategy.

### Public clients and observation

UI, CLI and service consumers share public DTOs, schema versions, negotiation,
cursor resynchronization, bounded waits and error handling. Rendering remains
UI-owned. The UI consumes wire/client libraries without gaining access to a
writable database, reducer, provider handle or internal admission permit.

The [terminal experiment](../app-server/experiments/terminal-ui/README.md) currently
uses fixtures and historical replay. Its archive is not the runtime protocol.
Read-only integration and authorized command integration are separate milestones.

### Distinct lifetimes and semantic owners

Context generation, executable generation, provider session, domain attempt and
OS process remain distinct identities. Ordinary context replacement does not
imply a process restart. Executable upgrades preserve lifecycle custody through
the owning upgrade or handoff contract. The existing
[generation manager](../app-server/src/executable-generation-manager.mjs) combines
build validation, activation, compatibility checks and predecessor retirement;
its replacement needs an explicit coordination boundary with lifecycle.

Campaign acceptance, review findings, claim establishment and workspace authority
also remain distinct. They may share compare-and-swap mechanics without sharing
one universal state machine. An authority reference is data, not a capability
grant. A database commit cannot establish that an external provider or Git effect
was completed; each effect retains its reconciliation and evidence contract.

## Delivery sequence and parallel work

These are priority groups, not a serial dependency chain. Each migration depends
on the specific contracts and evidence it consumes. Unrelated domain work need
not wait for full lifecycle replacement, and the lifecycle effort need not absorb
every later runtime consumer.

| Workstream | Relevant prerequisite | Next useful result |
| --- | --- | --- |
| Lifecycle foundations and bounded service | Lifecycle S0 interfaces and bounded implementation authority | S1 core/store, S2 executor and S3 wire/client converge at S4's actual-service integration. |
| UI schemas and fixtures | S3 U0 handoff | UI builds against the exact schema/client revision and representative fixtures. |
| Controlled read-only UI | S4 U1 handoff | UI connects to the controlled service and exercises observation, reconnection and uncertainty display. |
| Further lifecycle qualification | Dependencies owned by lifecycle S5–S10 | Native operations, semantic preservation, retained workflow, recovery/admin and migration qualification. |
| Review-episode persistence | Its value, encoding and persistence contracts | A bounded domain migration demonstrates reuse without importing campaign orchestration. |
| Workspace and executable-generation work | Each domain's shared contracts and lifecycle coordination where required | Fenced publication and deployment/draining behavior with recovery evidence. |
| Claims and campaign orchestration | Stable contracts for their consumed runtime and domain services | Domain-owned Rust transitions and compatibility-preserving state migration. |
| Legacy retirement | Accepted successor behavior and explicit state/consumer disposition | Remove each transitional bridge when its capability has a qualified successor. |

**The UI does not wait for the entire replacement.** S3 provides U0 schemas,
fixtures and the client API; S4 provides U1 controlled read-only service access.
These are planned handoffs, not already-achieved milestones. The UI owner accepts
and implements the integration. S4 does not establish production connectivity or
authorize UI commands. This preserves the lifecycle session's sequencing
clarification and the existing S0–S10 dependency graph.

Shared libraries start with the [proposed Rust workspace](rust-development.md#proposed-shared-workspace).
The experiments and their evidence remain in place. Package count, broader
extractions and relocation of the UI follow demonstrated consumers and their
owners' plans; there is no additional framework-building phase before lifecycle.

## Compiler and review episode planning lanes

The user is coordinating the existing lifecycle and UI efforts. This session
owns planning for two additional lanes, with a separate planning subagent for
each. Their implementation proposals are:

- [Compiler implementation plan](rust-compiler-implementation-plan.md):
  deterministic skill, manifest and extension compilation, with explicit source
  verification and integration boundaries.
- [Review episode implementation plan](rust-review-episode-implementation-plan.md):
  domain transitions, retained-review continuity and durable episode persistence,
  with a bounded bridge to current consumers.

The user accepted both directions on 2026-10-07 after a read-only plan and targeted
source review that reported no blocking plan or instruction-contract defects.
The exact reviewed compiler plan is SHA-256
`7e64984a4b399578d01c2728d2bb25e62d0e5b352fc9304f1773f45dc4b9a24e`;
the episode plan is
`dd17594b61ca7cc70f853b6c92024ffe1472006e4c8ac2530b63df513297515d`.
Those files remain unchanged so the reviewed subjects stay identifiable.

The first implementation grant was bounded **C0 and R0**. C0 covers
documentation, source characterization, supported corpus specification, the
source-path trust boundary and the producer-identity decision. Executable compiler
fixture capture starts in C1 and is outside this grant. R0 includes its specified
fixtures and characterization tests, plus concrete admission, evidence-scope and
semantic-delta dispositions; parity fixtures alone do not establish its exit.
Production semantic corrections, shared S0 changes, live state,
provider workloads and operational cutover remain outside this lane's grant. The user
continues coordinating lifecycle and UI, whose ownership is unchanged.

The bounded deliverables are now available:

- [Compiler C0 contract](rust-compiler-c0-contract.md): pinned input corpus,
  compatibility cases, byte/path rules, producer identity and the proposed C1
  protocol and S0 registration request. No compiler fixtures or product code
  were executed in C0.
- [Review-episode R0 contract](rust-review-episode-r0-contract.md): frozen
  JavaScript oracles, selected admission and evidence-scope rules, and explicit
  replay, unresolved-question, later-result and correction-join dispositions.
  The `gpt-6-sol` worker added four fixtures and one characterization suite.
  Builder and parent both ran the review-episode test directory with
  `node --test app-server/tests/services/review-episode/*.test.mjs`: all 14 tests passed.
  These are legacy characterization results, not Rust or cutover qualification.

A fresh read-only Astra review found no material C0 defects. Its R0 review
identified an unsupported campaign-interruption claim; the Sol worker added an
actual campaign-selection succession test with linked temporary stores, and the
retained reviewer confirmed the finding closed. The final review reported no
remaining blockers. This is same-provider review, not cross-provider independence.

After accepting these results, the user authorized proceeding with bounded
**C1 and R1** on 2026-10-07. C1 owns the pure unverified skill compiler and
bounded executable; R1 owns the pure episode core and result compatibility.
Implementations and remediation use `gpt-6-sol` at `high` effort. The local
orchestrator runs C1 then R1, retaining one implementation worker per slice.
C2/R2 and later slices are not included in this grant. Source and oracle
preparation initially proceeded while S0 retained shared foundation ownership;
board message 1169 records that earlier integration dependency.

S0 subsequently published commit `84be6695d716f026460fbf11aa703e73631e514e`
on `rescue-detached-20260911`. The user supplied its completed handoff in board
message 1191, transferring serialized three-package member/dependency
registration to this session. Integration resumed under the existing C1/R1
grants: a replacement `gpt-6-sol` compiler worker performed registration and C1
validation, followed by the retained `gpt-6-sol` episode builder for R1.
The published S0 interfaces/toolchain and excluded standalone UI remain the
base; registration does not authorize another lifecycle slice or C2/R2.

Executable results are recorded in the
[C1 result record](rust-compiler-c1-result.md) and
[R1 result record](rust-review-episode-r1-result.md). The parent subsequently
ran the combined ten-member workspace: all 42 tests passed (S0 14, C1 16,
R1 12), as did formatting, Clippy with warnings denied and the release build.
The [combined validation record](rust-c1-r1-validation.md) binds the final
artifacts, review disposition and bounded completion status. These results do
not establish production adapter, store, host or cutover qualification.

The user subsequently accepted one explicit C1 input restriction: reject YAML
array/object mapping keys while retaining scalar-key support. The legacy parser
converts collection keys to strings using presentation details; C1 does not
reproduce that conversion. This decision is recorded with oracle evidence and
rejection tests in the C1 result record, rather than claimed as full parser parity.

The episode lane also retains the affected owners' RC correction and consumption
gates. The compiler corpus records current builder-source drift from the
separate model-adapter repair; immutable source fixtures remain possible without
changing that owner's production pins.

After C1/R1 publication as `8497df02e055c3b93e865cbe75fe4c99eb555eb4`, the
user authorized parallel **C2/R2 planning only**. The resulting bounded plans are
[C2 verified skill closure](rust-compiler-c2-plan.md) and
[R2 durable episode owner](rust-review-episode-r2-plan.md). They propose separate
`gpt-6-sol` implementation owners: C2 covers source verification, the real Python
AEG process and existing compilation/hydration callers; R2 covers episode-owned
SQLite persistence and an offline command executable. R2 also owns the narrow
UTF-16 command-constructor addition in the existing episode core. Their product
paths do not overlap, and neither requires full lifecycle completion.

S1 retains shared workspace ownership until an explicit handoff. One integration
owner can batch the two lanes' dependency requests and R2's two new members;
later focused checks bind the resulting lock, while the combined workspace gate
uses a stable agreed source revision. No common persistence or supervision
framework is a prerequisite. Independent implementation can proceed after its
own authorization and required registration, with shared edits serialized.

The user accepted C2's proposed output-publication compatibility delta: reject
symlink output destinations and preserve existing regular-file permissions.
Source-file symlinks retain their existing behavior. That decision does not
authorize implementation, default backend cutover or later C3/R3 scope.

A separate bounded read-only review found no blocking plan defects in C2
`7390ba1c4490697dfbb62055cf388de42091e8b16102f583f5b542513123d577`
and R2 `edb856b4f14ef70f1005bd8fa172aae4016b1638784751cc2a2878fa464964f9`.
Targeted source checks supported the proposed interfaces and ownership split;
no implementation, tests, builds or database operations ran for that review.
This is same-provider plan review, not implementation qualification or
cross-provider independence. The plans are ready for bounded implementation
approval and the shared integration handoff.

The proposed execution shape is one retained builder per lane, accountable for
implementation, integration, testing and ordinary remediation. Separate worktrees
and non-overlapping domain paths permit independent work; the launch adapter and
accepted scope determine actual concurrency. The user continues coordinating the
cross-session dependencies. Shared workspace registration and lockfile edits
remain serialized through explicit coordination-board handoffs; message 1191
transferred only the three-package C1/R1 registration. Toolchain, common
values/codecs and broader foundation changes retain lifecycle ownership.
Neither lane depends on completion of the lifecycle replacement.

The compiler lane preserves the distinction between pure transformation and
filesystem/Git/source-verification effects. The episode lane preserves the
distinction between domain result admission and provider or claims authority.
Transport changes are explicit integration work: a synchronous caller is not
silently converted to an asynchronous service consumer. Historical encodings
remain named compatibility contracts, with domain-local implementations where
cross-domain sharing has not yet been justified.

The proposed local ownership split makes the later shared call sites explicit:

| Surface | Proposed owner and join |
| --- | --- |
| Compiler crates, compiler bridge, skill/manifest/extension transformations | Compiler lane; retain the current consumer call contracts. |
| Episode crates, episode bridge, native-review host/closure and episode exports | Review-episode lane; compiler requests any required review-host change through that lane. |
| Root workspace and dependency lock | Serialized integration owner; S0 transferred bounded C1/R1 registration in message 1191, with later edits requiring a new handoff. |
| Toolchain, common values/codecs and foundation gates | Lifecycle foundation owner; these lanes submit bounded requests. |
| Executable inventory, binary staging and activation | Existing generation/deployment owner; both bridges need an exact executable identity. |

Core development can overlap. Later edits to the same host or generation inventory
integrate serially, and the joined host receives focused regression coverage when
both replacements are present. The two synchronous bridges also need a combined
responsiveness check; separate launch measurements do not establish the composed
host's latency. This join does not make either pure domain depend on the other's
completion.

Planning uses the repository's current Astra guidance. For these lanes the user
explicitly selected **gpt-6-sol for implementation**, including remediation;
that selection is not an inferred default or permission for model substitution.
The R0 implementation worker was requested as `gpt-6-sol` at `high` effort. C0's
retained Astra agent performs documentation and source characterization only.
Review selection remains with each accepted slice and its owning independence
contract; same-provider review does not establish cross-provider independence.

## Evidence for a migration decision

Each bounded migration proposal can make its acceptance case concrete through:

- The semantic owner and exact old/new boundary, including retained adapters.
- Compatibility for persisted identities, schemas, revisions and external callers.
- Recovery at entry, result publication, caller disconnect and process loss, with
  ambiguous effects retained as unresolved rather than automatically retried.
- Same-path integration evidence, including adverse ordering and teardown where
  the component owns asynchronous work or processes.
- State import, cutover, rollback and consumer disposition appropriate to that
  boundary, before retiring its predecessor.

These are evidence dimensions for forming domain-specific slices, not a new
universal gate or an amendment to existing acceptance contracts. The owning
slice determines the checks needed for its claims. Useful outcome measures
include recovery failures, duplicate execution, stranded work, cross-module
repair effort, operational latency and operator interventions. Compilation and
unit-test counts alone do not establish those outcomes.

## Assessment basis and remaining decisions

The 2026-10-07 assessment used repository HEAD
`7583e2095725957470e28cc11fc69404386fec63` plus current working-tree design inputs.
It inventoried 116 JavaScript files and 26,637 physical lines under
`app-server/src`; tests, generated bindings, scripts and external legacy
implementations are outside that count. Selected source reads and graph traces
support the consolidation candidates. This is a static assessment, not an
exhaustive audit, benchmark or newly executed runtime qualification.

The initial graph coverage generation was `2026-10-07T06:31:37Z`; the later
lifecycle-plan reconciliation used `2026-10-07T06:37:19Z`. Checked paths had no
recorded gaps, a best-effort signal rather than proof of completeness. The Rust
experiment's bounded results and the legacy effort's failures retain the limits
recorded in the lifecycle evidence document.

Lifecycle S0 is published; lifecycle S1 proceeds under its own plan and serialized
workspace handoff. Compiler C2 and episode R2 remain later scope decisions.
Broader scope decisions still include the next additional domain consumer,
which supervision mechanics actually warrant extraction, encoding compatibility
profiles, and each domain's state migration. The priority table informs those
decisions; it does not silently add them to the lifecycle replacement's scope.
