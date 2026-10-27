# Rust development direction

Recorded 2026-10-07 from the user's direction to use Rust for Work Engine going
forward. Rust is the target language for new first-party executable components:
services, UI, CLIs, adapters, test infrastructure and reusable libraries. This
records an implementation direction, not acceptance of a particular replacement
design or authorization to rewrite existing systems.

## Evolve the system, preserve the contracts

The user clarified that Work Engine need not remain operational during this
development and is not the infrastructure used to build its own replacement.
Hosts and consumers may be replaced together offline, followed by a deliberately
qualified cutover. Continuous availability, dual running and temporary bridges
are not prerequisites. Runtime custody, context continuity and recovery remain
product contracts; development downtime does not retire a semantic owner, waive
a gate, settle unknown effects, dispose of saved state or authorize deployment.
Import versus an explicitly authorized archive/clean start remains a decision for
the human/state owner. Temporary interoperability needs a concrete consumer and need,
bounded cost/scope and a successor/removal condition; it cannot create a second
permanent owner.
Third-party provider executables need not be written in Rust to satisfy their
adapter contract. JSON, SQL, configuration and documentation remain appropriate
contract or data formats.

Design around Rust's ownership, exhaustive enums, private construction and
compile-time dependency boundaries. Do not mechanically reproduce callback
controllers, stringly typed internal state or shared mutable managers from the
old implementation. Durable transactions and external evidence still establish
cross-process truth: compilation alone cannot prove authorization, settlement,
semantic adequacy, or correct recovery.

The lifecycle replacement is the first substantial application of this direction;
it is not a prerequisite for every future Rust service. Its
[design](../experiments/context-lifecycle-rust/DESIGN.md) and
[multi-slice plan](../experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md)
retain their own acceptance boundaries. Broader domain migration needs its own
scope and owners; this document does not schedule a repository-wide rewrite.

## Proposed shared workspace

Use a production-intended workspace under `rust/`, with packages in `rust/crates/`
and small binaries assembled from them. Lifecycle S0 established this workspace
at `84be6695d716f026460fbf11aa703e73631e514e`; S1–S3 are also published as recorded
in the lifecycle plan. That does not require everything to become one process.
A workspace can share dependency declarations, lints and a lockfile while its packages remain
separate executables and semantic owners.
[Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html).

Keep the two current experiments and their saved evidence in place. The first
implementation slice selected and pinned the foundation toolchain and dependencies;
later slices bind their exact supported versions/features and build profiles,
with licensing/security evidence in their own qualification scope.
The UI experiment's declared Rust 1.88 minimum and lifecycle experiment's 1.92
minimum are historical experiment observations, not authority to override the
published workspace toolchain. Integrating or relocating
the UI remains work owned by its own implementation effort.

The shared dependency direction is:

```text
small common values/encodings
    ├── domain-owned core and persistence/runtime implementations
    └── domain-owned public wire contracts
            └── clients ── UI / CLI / other services
```

The UI consumes public protocol/client packages, not a service's reducer,
writable database, provider handle or private authorization permits. Sharing a
workspace does not grant a client internal capabilities. Cargo dependency checks
and compile-fail tests make these boundaries observable; runtime authorization
is still required for commands.

## Share semantics that are genuinely common

Start with a narrow `work-engine-types` package for common identity/value and
named, versioned encoding contracts that already have more than one consumer.
Domain identifiers stay distinct and namespaced; two IDs are not interchangeable
merely because both are strings or integers. Authority references are data, not grants.
Other components retain their own state machines, persistence and meaning.

The first domain packages are `lifecycle-wire` and `lifecycle-client`. Other
domains can later add their own wire/client packages using the same foundation.
Do not generalize a lifecycle state into a universal workflow state, or require
a universal tool platform before implementing a bounded owned-operation profile.
Extract further shared behavior when its semantics and consumers justify it,
not because two pieces of code look similar. Avoid an unowned `utils` package.

Public DTOs generate versioned schemas and compatibility fixtures. Rust clients
may share DTO definitions; existing non-Rust consumers can use the same schemas
and encoding vectors. Shared source is not a substitute for negotiation and
compatibility checks between separately deployed binary versions. Schema shape
does not establish authority, freshness or semantic validity.

New canonical encoding is not a repository-wide replacement for historical
serialization. Claim-evidence and workspace coordination currently use different
ordering/newline rules, so applying one new `digest()` to both would change durable
identities. Select the codec from the trusted domain/kind/version contract, preserve
existing identity bytes and verify cross-language golden vectors. Unknown codecs
fail explicitly; callers cannot choose a weaker codec to validate their own input.
Implement legacy codecs when an accepted migration needs them, not all at once in
the shared foundation.

## Reuse mechanics, preserve decision owners

Process/task supervision can share child ownership, bounded framing/output,
request correlation, cancellation requests and exact exit observation. Lifecycle
still owns admission and effect accounting; provider adapters own their session
and evidence contracts. A generic supervisor reports observations, not domain
success or remote settlement. Keep those mechanics separable in the first runtime;
extract a cross-domain library when actual consumers justify it, without making
builders or reviewers depend on lifecycle internals.

SQLite opening, transaction helpers and migration bookkeeping are similar reuse
candidates. Domain owners still choose schemas, durability requirements, migration
compatibility, validation and transaction/CAS boundaries. A helper cannot turn
commits in separate stores into one transaction or settle external effects. Start
with the lifecycle store's needs rather than a speculative persistence framework.

Context generation, executable generation, provider session and OS process are
distinct identities and lifetimes. Executable deployment owns build selection,
launch/drain coordination and compatibility; lifecycle owns context transitions
and custody.
The executor drains its owned work and reports the outcome; a deployer cannot
infer remote settlement from child exit.
Their startup/fencing/handoff contract must compose, but normal context replacement
does not require executable redeployment. The lifecycle plan proves the deployment
boundary it needs with a selected permanent Rust startup owner. The earlier
Node generation-integration proposal is an alternative, not a required bridge
or online coexistence route. Broader generation-manager, review, workspace, claims and
campaign migrations remain separately scoped work, not additions to its first
integrated milestone.

## Dependency starting points, not a universal stack

| Need | Starting choice | Boundary |
| --- | --- | --- |
| Async service I/O | Tokio; selected tokio-util facilities | Not a dependency of pure domain reducers; cancellation signals do not settle remote effects. |
| Local transactional persistence | rusqlite | Lifecycle's dedicated SQLite owner; other services choose storage for their own contracts. |
| Public JSON contracts | Serde/serde_json, Schemars; jsonschema for conformance | Pin schema dialect/generator and test generated output; no deserializable internal permits. |
| Typed library errors | thiserror | Stable domain error variants; general diagnostic wrapping at executable boundaries only. |
| Structured diagnostics | tracing | Correlation and redaction without making logs the authoritative event journal. |
| Adversarial validation | Proptest, selective Loom, actual-process tests | Different evidence scopes; none proves the entire distributed/external world. |

These choices are detailed with primary references in the lifecycle design.
Rust does not imply a single database, actor framework, transport, UI framework
or universal runtime for every service. Public packages should expose owned
types rather than leaking incidental dependency types across durable contracts.

## Implementation and coordination

Model assignments follow the revisable [model-selection guidance](model-selection.md)
and each task's explicit configuration. Parallel builders need agreed interfaces,
non-overlapping file ownership and one named integration owner; module completion
does not establish the behavior of their composition. Interface or authority
changes go back to their owner rather than being redefined independently in
each branch. The lifecycle plan records accepted progress and current bounded
authority; publication and operational changes retain their own authority.
