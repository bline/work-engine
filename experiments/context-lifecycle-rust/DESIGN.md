# Rust context lifecycle replacement design

## Status and decision

Replacement design, reconciled 2026-10-07 UTC for offline Rust development.
S0–S3 are accepted and published; the [implementation plan](IMPLEMENTATION_PLAN.md)
records their exact revisions and bounded S4 continuation authority. The complete
replacement is not production-ready. This document supplies no additional
implementation, provider, legacy-state disposition, migration, publication or
activation authority.

The design preserves Work Engine's semantic and authority contracts while
replacing callback-dependent ownership with a durable state machine and an
explicitly owned execution layer. Rust supplies local ownership, exhaustive
state handling, and restricted construction of capabilities. Transactions and
evidence validation supply the cross-process guarantees that the type system
cannot provide.

The historical JavaScript reference is the schema-14 iterative workspace, including its
uncommitted 17-path patch, not just published main. Its failed gate remains
failed. The separate Rust experiment proves a smaller, text-only case.
[DESIGN_EVIDENCE.md](DESIGN_EVIDENCE.md) records the comparison, failure lessons,
acceptance tests, and evidence limits; [design-sources.json](design-sources.json)
pins the inspected inputs. The existing implementation informed this assessment;
this is not a claim of history-blind or independently reviewed design.

The principal architecture choices are:

1. One durable lifecycle authority per logical subject, in a single-host service.
2. A pure Rust reducer, transactional store, and owned effect executor, with no
   provider I/O performed while a state transaction is open.
3. New native provider threads as the initial replacement mechanism where the
   owning session contract permits them. In-place reset is a separate capability.
4. Separate semantic verification, provider observation, effect settlement,
   lifecycle completion, input disposition, and process teardown.
5. Stable lifecycle storage across ordinary context changes and compatible
   executable upgrades; explicit fenced handoff when storage ownership changes.
6. No automatic retry of uncertain entered effects, and no retirement of actual
   queued work without its own authority and evidence.
7. A versioned read-only projection before UI control integration, with a separate
   authenticated command boundary.
8. A common owned-operation contract for tools: lifecycle owns admission and
   accounting, while execution and domain meaning remain with their declared
   owners. Entry-time and commit-time authorization are distinct guarantees.
9. Rust-native service implementation with small shared value, wire and client
   libraries; UI and other Rust consumers share contracts, not lifecycle internals.

The user's broader direction is recorded in
[Rust development](../../docs/rust-development.md): new first-party implementation
targets Rust, while existing owners remain until their replacements are accepted.
The [multi-slice plan](IMPLEMENTATION_PLAN.md) records delivery, progress and
remaining acceptance boundaries; implementation and operations use their own grants.

Work Engine need not remain operational during development and is not required
to build this replacement. Hosts and consumers can change together offline,
followed by a deliberate qualified cutover. Availability does not justify a
temporary Node/Python bridge, dual running or online rollout machinery. This
premise changes the development route, not ordinary runtime custody, context
continuity, effect accounting or recovery. Saved-state import versus explicitly
authorized archive/clean start remains a human/state-owner decision.

## Purpose and scope

A logical worker must be able to replace model context without losing the meaning
needed to continue, duplicating work, expanding authority, or requiring the caller
that started the operation to remain alive. A retained builder intentionally
waiting for input is healthy; waiting alone does not justify replacement.

The lifecycle service owns admission, transition ordering, continuation custody,
effect identities, recovery classification, and observable outcomes. It does not
own domain truth, campaign acceptance, human approval, the semantic meaning of a
role's work, or a provider's undocumented behavior.

The initial production target is Linux, one trusted host, one selected lifecycle
writer, and a pinned provider adapter. Multiple logical subjects may make
independent progress. Multi-host consensus, arbitrary remote exactly-once effects,
transparent migration of hidden model state, and a general workflow engine are
outside this design. Those omissions do not permit weakening an admission claim:
an unsupported required capability keeps the affected subject fenced.

The Rust service is horizontal infrastructure. A role is one possible subject
binding, not a lifecycle implementation class. A context generation, a domain
attempt, a provider thread, an executable build, and an OS process have different
lifetimes. Normal context replacement does not restart a role or kill a worker.

## Existing work and the replacement boundary

The JavaScript effort already contains valuable semantic compilation, source
binding, custody, signed disposition, migration, and proof contracts. Its current
completion observer and proof runner also address genuine defects. They are
requirements and evidence sources, not a template for Rust module boundaries.

The Rust prototype already has typed ownership, durable entry-before-send,
ordered inputs, conservative restart, read-only observation, exact native
text-turn receipts, and a three-turn live handoff. It still uses a single JSON
aggregate in SQLite, blocking provider I/O, experimental authority, a supplied
continuation package, and a narrow provider profile. Those are experiment choices,
not the production architecture selected below.

| Existing responsibility | Replacement treatment |
| --- | --- |
| Semantic compilation and verification | Preserve semantic obligations and provenance; provide Rust-owned orchestration and independently invocable inference ports. |
| Reservation, lease, tail, admission and queue release | Express as one durable subject transition and its effect plans, not cooperating callback owners. |
| Provider registry and observations | Preserve exact identity/evidence semantics behind a provider port; do not import a second admission authority. |
| Custody and claim dispositions | Preserve stable input lineage, separate acceptance from execution, and retain per-claim authority. |
| Completion observer | Read one durable projection; transport events wake it but do not decide truth. |
| Proof runner | A bounded owner of a proof environment, not a production lifecycle controller. |
| Migration and rollback | Separate offline operations using current custody state and explicit compatibility checks. |
| Source-route scanner | Replace lexical inference with typed operation registration, build inventory, and exercised boundary evidence. |

No runtime command may fall through from a selected Rust owner to the old owner.
Ownership selection is durable and exclusive for each subject. Offline replacement
does not require the existing controller to remain operational for other subjects;
any retained profile still respects that exclusive selection.

## Invariants

The identifiers below are referenced by the evidence and acceptance matrix.

- **I01 Single authority:** one admission owner and context-generation fence per
  subject. All domain and lifecycle effects enter through that owner.
- **I02 Stable meaning:** a successor receives a verified continuation basis;
  a checkpoint preserves references to authority, never creates authority.
- **I03 Entry before dispatch:** exact intent, request digest and entry attempt
  are committed before potentially effectful dispatch to a provider or tool.
- **I04 Truthful ambiguity:** missing evidence is unknown, not non-entry,
  completion, cancellation, or retry permission.
- **I05 Independent execution ownership:** accepted execution survives a lost
  caller, dropped waiter, UI disconnect, and ordinary observation timeout.
- **I06 Ordered custody:** an unresolved earlier input blocks later execution
  unless an explicit owning policy authorizes a particular disposition.
- **I07 Exact binding:** observations, checkpoint, authorization, successor,
  delivery and receipts must refer to the same required identities and revisions.
- **I08 Distinct consequences:** native termination, effect settlement, context
  handoff, delivery success, domain-goal completion and process exit are not aliases.
- **I09 Historical truth:** late settlement does not erase unsafe-close history;
  late contradiction records new evidence and fences dependents.
- **I10 Bounded authority:** deadlines and quotas gate new entry; expiry does not
  discard accounting for effects already entered.
- **I11 Honest recovery:** restart reads durable state before admission; it does
  not recreate expired grants, infer a receipt, or resend an uncertain request.
- **I12 Consumer equivalence:** controlled proofs substitute external providers
  and fault controls, not the lifecycle owner, receipt path, or ordering logic.
- **I13 Offline ownership:** migration, disposition and rollback exclude competing
  launches and preserve one enforceable delivery custodian.
- **I14 Evidence-qualified operation:** every claimed consequence has an admissible
  producer and consumer; constraints and passing compilation are not evidence of
  external settlement.

## Architecture and ownership

```text
Role host or authorized operator                 Read-only UI
          |                                         |
   versioned commands                        snapshots and cursor
          v                                         ^
   validation and authorization                     |
          v                                         |
   Store actor: reducer + transaction + durable projection
          |                     ^
   committed effect plans       | bound facts and resolutions
          v                     |
   Owned executor and process supervisor
          |
   Provider, semantic inference, tool and artifact ports
```

The **store actor** is a dedicated thread owning the writable SQLite connection.
It serializes short state transactions across subjects. The pure reducer handles
one subject aggregate at a time. External work runs concurrently under bounded
executor supervision and returns messages; it cannot mutate lifecycle tables.
Read-only connections may serve consistent snapshots without holding a write
transaction open. This is a selected single-host architecture, not a distributed
leader protocol. SQLite itself permits one writer at a time and supports read
snapshots in WAL mode. [SQLite isolation](https://sqlite.org/isolation.html).

The **executor** owns every submitted task and exact child handle. It schedules
only committed plans and reports every task's result, panic or cancellation. It
does not decide that a delivery is successful; the reducer validates returned
evidence against the effect's contract. A provider adapter translates transport
facts and enforces its capability profile, but cannot reopen domain admission.

The **semantic service** uses the same effect infrastructure for compiler and
verifier invocations. It owns bounded material gathering and semantic judgments,
not transition ordering. Canonical domain stores remain outside lifecycle storage.

The **proof harness** owns disposable proof processes and authority consumption.
It uses the service's ordinary commands and projections. Capture used only to
prove an experiment never becomes a prerequisite for ordinary lifecycle progress.

### Proposed package boundaries

| Package or module | Owns | Excludes |
| --- | --- | --- |
| `work-engine-types` | Narrow shared identity/value and named/versioned digest-encoding contracts, with domain namespaces | Lifecycle state, grant issuance, database or async runtime access; universal replacement of legacy codecs |
| `lifecycle-core` | IDs, validated values, reducer, invariant checks, effect plans | I/O, clock reads, provider SDKs, UI |
| `lifecycle-store` | SQLite migrations, atomic commits, journal, queries, recovery load | Model judgment, network dispatch |
| `lifecycle-runtime` | Lifecycle execution binding around separable task/process supervision, scheduling, clock service and shutdown | A second reducer or writable state cache; generic process exit as domain settlement |
| `lifecycle-provider` | Provider port and evidence-domain contracts | Domain admission decisions |
| `lifecycle-codex` | Pinned native protocol, history, capability checks, process adapter | Universal tool-settlement claims |
| `lifecycle-semantic` | Material projection, compiler/verifier contracts, continuation validation | Grant creation or workflow acceptance |
| `lifecycle-wire` | Versioned public DTOs and generated schema artifacts | Deserializable internal permits |
| `lifecycle-client` | Version negotiation, read-only snapshots/cursor resync, bounded waits; distinct command client | Lifecycle advancement, implicit mutation retry, database/provider handles |
| `lifecycle-service` | Authenticated local endpoints and host composition | Role-specific lifecycle algorithms |
| `lifecycle-admin` | Offline import, disposition, upgrades, backup and export | Ordinary domain execution |
| `lifecycle-proof` / `lifecycle-test-support` | Same-path scenarios; development-only peers, compatibility fixtures and deterministic barriers | Runtime authority, a second lifecycle controller or production default enablement |

These may begin as modules in a few crates rather than ten independently
published packages. Dependency direction and capability isolation matter more
than package count. The core is safe Rust with no provider dependency. Any needed
OS-specific unsafe code stays in a small reviewed platform adapter. Dependency
and compiler versions are pinned when an implementation slice is accepted.

### Rust-first sharing and dependency direction

The implementation home is the S0-established `rust/` workspace, leaving the saved
experiment and its evidence in place. New host, CLI, proof and administrative code
targets Rust. A temporary JS/Python consumer requires a concrete retained need,
bounded scope and removal condition; availability alone supplies no such need.
It cannot own a second lifecycle controller. The UI effort can consume
the new libraries without moving its experiment or sharing process lifetime.

`work-engine-types` is deliberately small. Common public identity encodings and
versioned digests have UI/service/test consumers; private entry permits and
mutable lifecycle state do not. Lifecycle-specific identifiers remain distinct
in their owning namespace. Core validates domain values and owns the reducer;
wire conversion validates untrusted DTOs without teaching clients how to grant
admission. Shared parsing or cryptography does not confer semantic authority.

Shared encoding does not mean one historical serialization contract. New wire
kinds use the selected versioned codec; existing domain identities retain their
original codec and exact digest bytes. Codec selection comes from the trusted
domain/kind/version contract, not an untrusted selector or whichever algorithm
happens to match. Legacy codecs stay with the domain/migration owner until genuine
reuse warrants extraction; the common package need not implement every old format.

The core and wire packages may depend on common values, but neither depends on
the other. Store and runtime use core contracts; client uses wire; service
composition converts between them. UI depends on client/wire, not core, store,
runtime or provider adapters. Dev-only peers are excluded from production
composition. This is a dependency/ownership boundary, not a requirement to create
a separate package for every small module.

`lifecycle-client` owns framing, protocol negotiation and observation recovery,
not accepted execution. Dropping it cannot abandon a service-owned operation.
Read-only APIs expose no mutation methods. A separate command client supplies
requests for service-side authorization; possession of that client is not a grant.
Client reconnect never automatically resends an effectful command unless its
explicit idempotency contract and current authority permit that exact replay.

The shared library owner maintains compatible fixtures and versioned schemas.
Generated output and old-client/new-service combinations are tested rather than
assuming that sharing Rust source makes deployed versions compatible. Known
unsupported variants are reported honestly; clients cannot turn an unknown
settlement variant into success. UI rendering stays UI-owned.

### Reusable mechanics beneath lifecycle ownership

The runtime separates process/task machinery from lifecycle execution policy.
Child handles/incarnations, bounded transport framing and output, request
correlation, cancellation requests, task joins and exit observation can serve
other domains. Their reusable surface reports facts and errors, never lifecycle
admission, delivery success, semantic acceptance or remote settlement. The
lifecycle execution binding claims committed effects, attaches exact identities
and submits observations to the store. Provider adapters retain distinct native
session, authentication and admissible-evidence contracts; shared process code
does not make Codex and Claude transport outcomes interchangeable.

S2 starts with separable modules behind the accepted ports, not a mandatory new
cross-domain framework or additional package count. If a shared supervision crate
is extracted, its dependency direction stays below lifecycle policy: it cannot
import the lifecycle reducer/store. A builder or reviewer later uses those
mechanics through its own owner binding, not by becoming a lifecycle role class.
This does not create a second executor for work already owned by a harness.

Likewise, connection opening, transaction helpers and migration bookkeeping may
be reusable SQLite mechanics. Lifecycle owns its schema, durability configuration,
migration compatibility and order, writer fence, transition validation and exact
transaction/CAS boundaries. Helpers cannot leak writable connections around that
owner, silently weaken its configuration, or claim atomicity across external
stores/effects. S1 implements only the lifecycle needs; a general persistence
framework and unrelated domain migrations are not prerequisites for M1.

### Dependency baseline and selection work

The following starting choices informed the selected architecture. S0–S3 now
bind their accepted toolchain/dependency sets in the published workspace and
handoffs; this rationale is not a claim that the composed service is implemented
or that latest versions have been audited.

| Concern | Starting choice and responsibility | Important limit |
| --- | --- | --- |
| Async I/O and task accounting | Tokio; selected tokio-util cancellation/framing utilities | Register and join owned work explicitly; cancellation is only a request, not a settlement receipt. |
| Store | rusqlite on the dedicated writer thread | No pooled writable connection escapes the actor and no provider await holds a transaction. |
| Public serialization/schema | Serde/serde_json plus Schemars; jsonschema for conformance | Select JSON Schema 2020-12 explicitly; generation is not semantic validation. |
| Errors | thiserror for typed module errors; existing anyhow-style wrapping only at executable boundaries | Project stable public errors; never make clients parse debug strings or dependency error text. |
| Diagnostics | tracing with structured identity fields and a configured subscriber | Logs and span IDs are not durable operation IDs or authority; redact sensitive payloads. |
| Identity/digest encoding | Existing SHA-256 primitive behind domain/version/codec-bound digest types; qualify RFC 8785 for new wire kinds | Preserve historical codec bytes; generic JSON serialization, shape-valid digests and map parsing that loses duplicate keys are insufficient. |
| Testing | Proptest, selective Loom, temporary roots and actual child-process tests | Generated event cases, local synchronization exploration and process evidence are separate scopes. |

Primary library references:
[Tokio JoinSet](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html),
[CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html),
[rusqlite](https://docs.rs/rusqlite/latest/rusqlite/),
[Schemars](https://docs.rs/schemars/latest/schemars/),
[jsonschema](https://docs.rs/jsonschema/latest/jsonschema/),
[thiserror](https://docs.rs/thiserror/latest/thiserror/),
[tracing](https://docs.rs/tracing/latest/tracing/span/index.html).
Schemars may change generated structure without a semver-major release, so pin
the generator/settings and compare artifacts explicitly. Schema generation and
conformance tooling need not become production runtime capabilities.

`serde_json_canonicalizer` is a candidate RFC 8785 encoder for new wire contracts,
not a replacement for historical codecs or yet a selected trust component.
Qualify it against cross-language golden vectors, Unicode,
number boundaries and strict duplicate-key rejection before using its bytes in
signed or digest-bound evidence. Large exact values use specified strings.
[Candidate documentation](https://docs.rs/serde_json_canonicalizer/latest/serde_json_canonicalizer/).
Reject unsupported inputs before a lossy representation hides their invalidity.

The accepted slices bind compiler, dependencies/features, build profiles and
their licensing/advisory evidence; each later slice preserves a reproducible
resolved graph and records new qualification needs.
Cargo features are additive configuration, not an authorization mechanism: a
production profile cannot expose fixture providers or unsafe administrative
entry points merely because another consumer enabled a feature. Shared values
and wire packages stay free of Tokio, SQLite and provider dependencies.

Do not add an actor framework, ORM, universal workflow engine or automatic retry
middleware by default. The store actor and executor need small explicit ownership
contracts, not another implicit controller. Library choice can change within its
accepted boundary while those contracts remain intact.

### Clock and evidence seams owned by this service

The runtime owns an injectable clock boundary: monotonic wait budgets and wall
authority expiry are separate typed values with explicit units. The pure reducer
receives sampled time as input rather than reading a clock. Restart revalidates
absolute authority without persisting a process-local monotonic instant as if it
were portable. Tests can control these samples and acknowledge real process
boundaries; a fake clock does not simulate OS/provider settlement.

The runtime attaches subject/transition/delivery/effect/attempt/incarnation IDs to
structured diagnostic events. The store still owns committed lifecycle history.
Usage counters include provider provenance, cached/non-cached distinctions and
explicit measurement availability; unavailable is never zero. A telemetry backend
or future prediction model is not required for lifecycle correctness.

### Command boundary

Every mutation envelope includes a protocol version, principal, immutable command
ID, subject, expected relevant revisions, grant reference and canonical request
digest. The service resolves grants against its pinned trust configuration, not a
key supplied by the same request. In-process callers use the same validation and
transaction path as IPC callers; passing a Rust type is not an authorization bypass.

| Command family | Durable result | Additional restriction |
| --- | --- | --- |
| Enqueue input | Custody acknowledgment with input and delivery IDs | Does not start delivery. |
| Request replacement or yield | Accepted transition ticket or exact rejection | Captures policy/authority basis and predecessor identity. |
| Submit adapter or semantic fact | Applied, duplicate, conflicting or rejected observation | Restricted producer port; public clients cannot forge provider evidence. |
| Request interruption | Bound control-effect ticket | Targets an exact entered attempt; does not claim cancellation succeeded. |
| Authorize recovery or claim disposition | Recovery ticket or committed disposition | Separate grant, exact fenced revision and evidence freshness. |
| Prepare or commit ownership handoff | Inactive acceptance or exact handoff revision | Offline administrative boundary, never a domain command. |

An accepted ticket is stable even if its response is lost. Polling that command ID
returns the committed result. Reusing it with different bytes is a conflict.
Malformed or unauthenticated messages are bounded transport rejections, not an
unlimited unauthenticated durable journal. Snapshot and wait endpoints have no
mutation capability.

## Rust representation and its limits

Use distinct newtypes for `SubjectId`, `ContextGeneration`, `TransitionId`,
`InputId`, `DeliveryId`, `EffectId`, `AttemptId`, `CheckpointId`, `GrantId`,
`RuntimeIncarnation`, `ProviderThreadId`, `ProviderTurnId`, `BuildId` and `StoreId`.
Proof-run IDs and launch labels are additional identities, not synonyms for any
of these. Digests have a domain and encoding version; a 64-character string alone
is not an admissible digest claim.

Untrusted wire DTOs are parsed into request values, then converted through
`TryFrom`-style semantic validation. Private constructors create `AuthorizedEntry`
and `CapturePermit`; these are not `Clone`, `Serialize` or `Deserialize` and have
`#[must_use]`. They are short-lived local conveniences over a durable grant and
revision check, not cryptographic capabilities. Every transaction revalidates
the durable scope; copied bytes never reconstruct a live permit directly.

Use exhaustive enums for persisted stages, outcomes, errors and effect kinds.
Do not use a collection of booleans to encode mutually exclusive authority
states. Use typestate only for short local ownership transfers, such as a claimed
proof environment becoming runner-owned. Durable state cannot rely on a Rust
stack surviving restart and therefore uses validated data plus an exhaustive
reducer. Rust exhaustiveness helps expose missing cases; it does not establish
that each case has the right semantics.
[Rust match](https://doc.rust-lang.org/book/ch06-02-match.html).

`Send` and `Sync` constrain how Rust values cross threads; neither makes an
external effect atomic. Share immutable configuration and projections with
`Arc`, not a broadly mutable controller behind `Arc<Mutex<_>>`. Avoid public
escape hatches returning the raw provider or writable connection.
[Rust concurrency traits](https://doc.rust-lang.org/book/ch16-04-extensible-concurrency-sync-and-send.html).

The conceptual state model is:

```rust
// Design notation, not a compiled API or an already implemented schema.
enum AdmissionOwner {
    Domain { generation: ContextGeneration },
    Transition { id: TransitionId },
    Recovery { id: RecoveryId },
    Fenced { reason: FenceId },
}
enum TransitionStage {
    Quiescing, Capturing, Verifying, Checkpointed,
    Switching, Rehydrating, ReadyToCommit,
    Reconciled, RecoveryRequired, AbortedBeforeEntry,
}
enum DispatchState { Prepared, Entered, DefinitivelyNotEntered }
enum ExecutionOutcome { Pending, Completed, Failed, Interrupted }
enum Settlement { Unresolved, Established(SettlementEvidence), Conflict }
enum DeliveryResult {
    Pending, Succeeded, FailedBlocked, Unknown,
    Yielded { successor: TransitionId }, RetiredNoRetry,
}
```

`Failed` execution with unresolved side effects remains unsafe to advance. A
known domain failure is not mislabeled as pending merely because cleanup remains.
The aggregate stores these dimensions separately and defines their permitted
combinations. Compatibility decoders reject unknown authority-bearing variants;
UI readers can display an unknown observational variant as unsupported.

## Durable storage and transaction contract

Replace the experiment's rewritten aggregate JSON with versioned, normalized
tables and an append-only event journal. The selected store is SQLite on a
supported local filesystem, with foreign keys, WAL, explicit durability settings,
integrity checks, bounded busy handling, and a lifetime writer lock. No network
filesystem or power-loss guarantee is implied without platform validation.
The service opens a canonical store root, checks underlying file identity and
refuses hard-linked database aliases. Its lock identity remains stable for the
writer lifetime; moving/replacing a live store root is unsupported. Path strings
alone cannot establish exclusive ownership.

| Durable records | Required relationship |
| --- | --- |
| Subjects and admission owners | Unique active owner and generation per subject; separate semantic and queue revisions. |
| Commands and command results | Unique principal/command key; same key plus changed canonical request is a conflict, including rejected commands. |
| Transitions and recovery operations | At most one selected admission owner; exact predecessor, basis and terminal history. |
| Inputs and custody claims | Immutable payload digest/lineage and sequence; unique active delivery claim for an input. |
| Effect intents and attempts | Exact subject, purpose, request, grant, deadline, operation contract/implementation, selected executor, build/profile and generation. |
| Raw observations and applications | Stable source identity distinct from event classification and CAS application. |
| Outcomes and settlements | Native outcome, side-effect evidence and delivery consequence separately bound. |
| Continuation candidates and checkpoints | Immutable content, source and verifier basis, governing environment and publication revision. |
| Runtime incarnations and close records | Exact owned processes, launch epochs, local termination and unsafe-at-time history. |
| Grants, dispositions and handoffs | Scope, expiry, signer/trust basis, expected revisions and replay identity. |

One command transaction loads the affected subject, validates current authority,
reduces the command/event, and commits state changes, journal entries, resulting
effect plans, and its command result together. A stale expected revision returns
a concurrency conflict; it does not manufacture a semantic failure. Replaying
an immutable observation reloads state and reclassifies that same observation.

The journal and query tables are changed in the same transaction. Notifications
are sent only after commit and carry a revision/cursor. Lost notifications are
recovered by querying from the durable cursor. There is no asynchronously updated
second database that can accidentally become the authoritative completion view.

Each subject has its own revision dependencies. An unrelated subject's update
does not invalidate a checkpoint. Queue arrival alone need not invalidate a
semantic package if that payload remains separately durable and no included
semantic assertion depends on its old queue state. Activation still rechecks
current queue custody and all relevant authority/source revisions.

Artifacts are content-addressed immutable blobs. For filesystem publication,
fully write and sync owned staging bytes, publish without overwrite, sync the
directory as required by the durability claim, then reference the digest from a
transaction. Recovery can collect unreferenced blobs but cannot turn a partial
file into a committed artifact. Lifecycle ordering uses database facts, not file
watchers. Snapshots use a coherent SQLite backup or explicitly coherent WAL image,
not the main-file hash alone. [SQLite backup](https://sqlite.org/backup.html).

Retention never discards unresolved effects, outstanding custody, accepted grant
consumption, or deduplication fences. Payload retention can be shorter than ID and
digest retention after an authorized policy, but expired observations cannot be
used to claim evidence that no longer exists. Migrations have declared reader and
writer version ranges; an older executable refuses a newer incompatible store.
No automatic downgrade is permitted.

## Admission and effect execution

Admission is an atomic decision against current subject ownership, the earliest
eligible input, context generation, authority, capability profile, policy and
quota. Enqueueing acknowledges durable custody, not permission to execute.
Lifecycle control operations have distinct narrowed grants and cannot admit
ordinary domain work while the transition owns the subject.
Admission records which authorization guarantee the operation requires. A
pre-dispatch authority check does not establish that authority stayed valid until
a later mutation committed; the operation boundary described below must enforce
that stronger guarantee where required.

The effect protocol has five boundaries:

1. **Prepare:** commit the exact effect plan and command result. No external entry.
2. **Claim entry:** atomically revalidate the plan's fences and allocate an attempt.
   Record `Entered` before any potentially effectful write. Debit the applicable
   provider-turn allowance here, including attempts that become uncertain.
3. **Execute:** transfer a non-clone local entry permit to the owned task. Adapter
   writes are serialized per provider session where that protocol requires it.
4. **Record:** persist native acknowledgments, raw observations and attributable
   result evidence. Early notifications have an owner before the caller gets a
   ticket; they await exact association rather than being guessed into a turn.
5. **Apply:** validate evidence, update settlement/delivery/transition, and enqueue
   the next justified plan in the same transaction. The tail never depends on
   resuming an original caller's stack or on a proof consumer calling advance.

There is an unavoidable crash window after durable entry and before send. Recovery
treats it as uncertain unless a provider-specific protocol establishes definitive
non-entry. A missing registry row alone is insufficient. An idempotency key is
not assumed to be honored merely because a provider accepts it as a request field.

Prepared, unentered work may be issued only if its original grant and current
fences still permit it. Entered unknown work is observed/reconciled, never blindly
retried. A fresh attempt requires an explicit operation-specific retry policy,
proof that its predecessor cannot still execute, and any required authority.
The initial implementation has no generic automatic retry for effectful calls.

Native event identity belongs to the ingress boundary. Fan-out preserves the same
source coordinate; consumers do not mint a new identity each time they see the
same ingress. Do not deduplicate all identical raw bytes: two real emissions may
be distinct. Host command replay and provider event replay are separate mechanisms.

## Execution ownership and cancellation

Use a supervised Tokio runtime for external I/O and bounded task sets. A task is
registered under its effect and runtime incarnation before it can produce a
result. Store completion as data, including errors, and join every owned task.
Ordinary failures must not escape as unobserved task panics or dropped error paths.
Admission reserves result-accounting capacity as well as execution capacity;
producer saturation cannot deadlock the store while it waits for a task whose
result is blocked behind that same saturated queue. Task registration and its
result sink exist before scheduling the task body.

Dropping a Tokio `JoinHandle` detaches its task; dropping a `JoinSet` aborts its
tasks. Neither is a durable settlement protocol. The selected executor explicitly
drains join results and records cancellations/panics; it does not use a convenience
shutdown that discards task errors as proof of success.
[JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html),
[JoinSet](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html).

Cancellation has three different targets:

- Cancel an **observation**: remove that waiter, leaving accepted execution owned.
- Request **operation interruption**: admit a separate bound provider-control
  effect. Acknowledgment is not terminality or absence of outstanding effects.
- Stop an **executor/process**: preserve unresolved entry and local-close facts;
  restart reconciles them rather than treating task drop as successful cancellation.

Long-lived transport-reader tasks own framing state across awaits. Do not put a
fresh multi-step read or write into a `select!` loop whose losing branch discards
partial progress. Handle cancellation at explicit protocol boundaries; classify
any uncertain outbound write conservatively. Tokio documents cancellation-safety
differences between operations. [select cancellation safety](https://docs.rs/tokio/latest/tokio/macro.select.html).

The database thread is dedicated rather than an unbounded collection of
`spawn_blocking` calls. Shutdown must join it or report it unresolved; aborting an
already-running blocking task does not stop it.
[Tokio task cancellation](https://docs.rs/tokio/latest/tokio/task/index.html).

Bound mailbox capacity, parser frames, pending associations, task concurrency and
retained diagnostics independently. Backpressure delays new admissions. Overflow
of required provider evidence closes the affected admission and records an
observation gap; silently dropping evidence must never produce a successful
receipt. Storage failure fences new entry globally for that store while owned
tasks and available evidence remain accounted for.

## Context replacement protocol

The default route uses a fresh native thread for the successor, under the same
logical subject and service. A subject whose contract requires the same native
session cannot use that route silently. For example, a reviewer episode requiring
native same-session retention needs either a verified in-place capability or an
explicitly authorized reviewer replacement, truthfully recorded as replacement.

| Stage | Committed prerequisite and consequence |
| --- | --- |
| Quiescing | Reserve exact predecessor and close new domain entry; allow already-entered work only within its existing authority. |
| Capturing | Establish safe predecessor boundary or an explicit complete unresolved-effect inventory; freeze the semantic source basis. |
| Verifying | Run bounded compiler and separate verifier effects; reject uncertainty or attribution gaps. |
| Checkpointed | Publish accepted immutable checkpoint and bind current source, authority, environment and subject revisions. |
| Switching | Commit exact switch authorization and successor allocation intent; perform only the selected provider capability. |
| Rehydrating | Deliver exact checkpoint, governing material and challenge to successor under control-only permissions. |
| ReadyToCommit | Validate exact successor response, package adequacy, provider identity and current dependency fences. |
| Reconciled | Atomically select successor generation, retire predecessor admission, and transfer queued-input readiness to the scheduler. |
| RecoveryRequired | Keep affected admission fenced with exact unresolved obligations; no fabricated completion or automatic replay. |

The transition may reconcile while an unrelated domain goal remains unfinished.
Ordinary delivery is a separate scheduler transaction after successor selection,
so custody transfer itself cannot execute Q4-like work or re-enter pressure logic.

When active work must yield, reserve the transition before interrupting its exact
turn. The terminal observation advances the durable owner even if the original
caller never wakes. A natural terminal that wins the race suppresses an unnecessary
interrupt. A yielded delivery records a link to its successor transition, not
ordinary success; the original input identity and completed consequences survive.

Yield does not create a second delivery of the same input. Its durable work cursor,
completed effects and unresolved obligations become part of the continuation basis.
Resuming a yielded task is a newly admitted, distinctly identified attempt under
the same delivery lineage, with explicit continuation authority and no replay of
earlier effects. A new context alone grants neither that attempt nor permission
for a later input to overtake it.

Selecting the successor and granting domain admission are separate predicates.
Unresolved remote work may be carried only if an independent durable owner can
continue accounting for it without the old model context. Otherwise capture is
unsafe. In either case, unresolved conflicting work still blocks domain entry;
`Reconciled` cannot turn an unknown effect into a settled one.

An in-place provider reset, if supported later, is another switch implementation
with stronger evidence requirements: exact lease/switch identity, old context,
control turn and new context. Unleased reset is not deferred until a future lease
can launder it into success. Unexpected compaction or lost provider lineage fences
the subject according to the supported adapter contract. New-thread isolation
does not prove access to hidden provider context or universal suppression of
undocumented resets.

Abort is safe only before entered irreversible work, with an explicit rule for
already-owned queue items. After uncertain entry, failure becomes recovery, not a
return to the old context based on hope. A context successfully switched is never
claimed to be restored byte-for-byte by rollback.

## Semantic continuation

The semantic contract preserves consequences of reasoning, not an exhaustive
transcript or hidden chain of thought. Its required fields include objective and
work position, completed consequences, commitments, decisions and invalidation
conditions, uncertainty, unresolved human interactions, governing instructions,
activated skills, canonical references, and the authorized next action. Optional
role-owned extensions carry a named schema without moving lifecycle ownership
into the role.

The source projection binds visible provider history, governing sources, relevant
canonical records, authority revisions and their content digests. Missing material
is missing, not an invitation for the compiler to invent a reference. Model-output
references resolve only inside the bounded input/material set. Data cannot promote
itself into an instruction or grant; trusted loaders decide governing provenance.

Compilation and verification are separate entered effects with distinct invocation
identities. The host supplies subject, source, invocation and schema metadata;
the model supplies attributed semantic content. A verifier challenges sufficiency,
attribution, authority preservation, interaction closure and source binding. Host
code derives accepted/rejected/unresolved from those findings. Different invocation
IDs alone do not establish independent judgment; any required independence profile
is separately selected and evidenced by the owning contract.

A checkpoint is admissible only when both mechanical binding and semantic
verification pass. Before successor activation, current authoritative references
are revalidated. A stale human grant blocks the dependent action even if the
checkpoint correctly quotes its earlier revision. Pending questions retain their
question ID, recipient, scope and resolution status. Late answers are routed by
those durable identities, not by whichever thread is currently visible.

Successor readiness must demonstrate use of required continuation obligations,
not just echo a digest. Deterministic checks establish exact package and identity;
model judgment evaluates semantic adequacy where mechanical checks cannot. If
projection is insufficient to preserve correct continuation, the service stays
fenced or requests authorized recovery. It does not relabel a smaller fact-carry
test as proof of full semantic preservation.

The first semantic acceptance scenario uses realistic but synthetic work with an
unresolved approval, a superseded decision, a canonical reference, a governing
instruction, and pending input. The successor must distinguish all five without
receiving their answers again in the next domain prompt.

## Provider and tool contracts

Adapters advertise versioned, tested capabilities: exact identity observation,
history/readback, interrupt, new-thread allocation, optional in-place reset,
usage counters, and the evidence they can supply for each effect kind. Selection
requires the claims of the intended operation; provider/model names alone do not
satisfy a capability contract.

The current Codex text domain is retained as a narrow capability. It binds exact
request/profile/thread/turn/item/output/full-history digests and rejects unknown
effectful activity. Native completion alone remains insufficient. Observation-only
records are not upgraded into text settlement after the fact. Pin and verify the
binary, configuration and actual supported profile in each acceptance run.

### Owned operations rather than a universal tool platform

Lifecycle consumes an **owned-operation contract**, not an arbitrary tool name
or a promise that every tool shares the same semantics. This follows the
[Service Plane idea](../../app-server/ideas/pending/service-plane-and-kernel-domain-boundary.md):
generalize the surrounding contract, not the domain payload. The idea's proposed
grammar is design input, not an accepted global schema or an implemented service
registry. This design needs only the bounded contracts for selected operations.

| Contract dimension | Required meaning |
| --- | --- |
| Operation identity and revisions | Distinct operation, contract and implementation identities; a compatible signature alone does not establish equivalent behavior. |
| Semantic owner and executor | The domain owner defines result meaning; one selected executor/tool-loop owner performs it. Lifecycle owns neither domain meaning nor a duplicate executor. |
| Input and authority | Typed payload or owned references, required scope, effect class and authorization guarantee. Capability availability is not permission. |
| Outcome and evidence | Separate execution outcome, committed mutation evidence where applicable, and settlement of descendants or remote effects. |
| Recovery and idempotency | Exact replay identity, reconciliation route, interruption semantics and conditions for a safe new attempt. No assumed retry safety. |

Each invocation binds that contract to its input digest, subject, effect/attempt,
grant revisions, selected executor and runtime incarnation. Trusted composition
loads the descriptor and validates the implementation's evidence capabilities;
model-supplied metadata cannot declare an operation safe. Domain-specific Rust
payload/result types remain behind typed ports. Lifecycle validates the exact
binding and required accounting evidence without interpreting, for example, what
a code-review finding means.

Execution can be mediated by Work Engine or owned by an admitted harness. Reuse
existing services through adapters where their guarantees suffice. A harness may
continue to own its native tools and tool loop, but it must establish the selected
admission, identity, outcome, settlement and recovery contract. If that contract
requires a per-tool pre-entry gate and the harness cannot enforce one, the route
is unsupported. Observing a tool afterward is not equivalent enforcement.

The [runtime-ports idea](../../app-server/ideas/pending/provider-turn-harness-runtime-and-operator-projection.md)
separates provider-turn, harness/tool-loop and operator-projection ownership.
The selected composition records exactly one owner for each execution responsibility.
Work Engine must not start a second provider turn or tool loop for an operation
already owned by its harness. Hidden direct-tool paths or unaccounted descendants
make the affected profile ineligible; wrapping the visible calls is insufficient.

### Entry authorization and commit authorization

The operation's canonical owner defines when authority must hold and what
revocation means. The profile records one of the following obligations explicitly;
the lifecycle adapter cannot choose a weaker obligation to fit its implementation.
Those definitions remain within every governing grant and authority ceiling.
Declaring an operation entry-bound cannot override an upstream requirement for
commit-time currency or cancellation; incompatible contracts reject admission.

- **Entry-bound authority:** the owner explicitly permits the bounded entered
  operation to finish under its admitted authority, even if later grants narrow.
  Narrowing blocks new admissions. Interruption and already-entered consequences
  follow the owner's stated rules; this is not a universal permission to finish.
- **Commit-bound authority:** every relevant authority/validity dependency must
  still authorize the mutation at its commit boundary. Enforcement must serialize
  the invalidating writers with that commit, or establish an equivalent target-side
  fence. Checking a revision and then performing unrelated I/O does not close the
  race. Missing enforcement makes the operation unsupported.

No default is inferred when those semantics are unspecified. Reads and derivations
also require scoped authority and a declared evidence/recovery contract; their
classification does not grant an exemption from confidentiality or cancellation
requirements. A committed result is distinct from remote-effect settlement and
from lifecycle completion, regardless of authorization mode.

The [Authority Commit Gate draft](../../app-server/ideas/pending/authority-commit-gate-and-effect-admission.md)
addresses the stronger commit-bound case across multiple authority owners. Its
upstream invalidation semantics and complete dependency-to-writer registry remain
blocking prerequisites, and its proposed restructure has open review findings.
This lifecycle design neither adopts that mechanism nor claims those prerequisites
are resolved. It also cannot replace the draft's no-stale-commit invariant with
weaker drain/reconcile semantics and call that equivalent.

For an entirely lifecycle-owned mutation, one store transaction may establish the
required check-and-commit boundary only if **all** relevant invalidations share it.
Authority spread across external stores does not become atomic through the Rust
type system or a lifecycle-local transaction. A cross-owner operation stays disabled
until its own adequate enforcement route is selected and proved. The broad commit
gate is not a prerequisite for every lifecycle milestone, but any operation that
needs its unresolved guarantees remains unavailable.

### First supported tool profile

Select a small set of owned operations and exercise them end to end; do not first
build a universal service plane or rewrite every tool. Scoped workspace reads and
controlled local file changes are candidate operations, not automatic guarantees:
each must declare and prove its authorization and reconciliation boundary. A file
rename being atomic does not itself close authority revocation races.

Foreground build/test commands can follow when their process-tree ownership,
containment and effect accounting are demonstrated. Unrestricted shell execution,
detached children and opaque remote mutations do not become supported by receiving
an operation ID. Harness-owned alternatives are equally eligible when they satisfy
the same required contract. The initial concrete profile remains an acceptance
decision; no tool-enabled production capability is claimed by this amendment.

No generic `provider_done => all_effects_settled` conversion exists. A denied
capability attempt before entry differs from an accepted async user question,
running shell command, or unknown custom tool. Accepted human interaction must be
durably accounted for or reject eligibility; it cannot be hidden by a later answer.
Tool permissions during capture/rehydration are limited to explicitly required
read-only retrieval, with its own evidence and authority. Domain tools stay fenced.

Adapters persist allocation intent before native thread creation, native identity
before turn dispatch when available, and exact correlation for terminal-before-
response ordering. Lost creation/start responses remain uncertain. An exact
history read may establish a result without issuing another turn; absence from
partial/paginated/unsupported history is not definitive non-entry.

## Custody and operator recovery

An input has immutable bytes/digest, producer lineage, subject and stable delivery
identity. Its logical custody owner is independent of the current provider thread.
Transfer acceptance is durable before the source releases ownership. Delivery
requires a later admission transaction; a selected successor is not yet an
accepting successor, and an accepting successor is not permission to execute.

An entered delivery may succeed, fail, yield into another transition, remain
unknown, or conflict. Newer input cannot overtake it. A failed delivery is promptly
visible as failed while unsettled effects remain explicit. No retry or terminal
retirement follows just from a terminal native error.

Recovery is a separately identified owner admitted by CAS from an exact fenced
revision. Its grant lists allowed actions and conditions; it inherits no automatic
reset, replay or queue-release permission. An operator may observe, supply exact
settlement evidence, authorize a safe new attempt where proven, or terminally
retire one exact input under the established owning policy.

Terminal retirement records `RetiredNoRetry`, not a fake successful provider
receipt. It requires an independently pinned trust policy, the exact claim and
observation revisions, unexpired scope, and evidence accounting for the provider
turn/effects. Local process shutdown alone is insufficient. Commit rechecks
freshness and competing-launch exclusion. A late conflicting fact preserves the
retirement history and re-fences dependent admission instead of silently changing
the record. Already-performed downstream effects cannot be undone by that fence;
they require explicit incident accounting. The initial release has no autonomous
retirement policy.

## Observation and UI contracts

Read-only APIs expose snapshots and a durable event cursor scoped to a subject.
An observation request cannot drive lifecycle progress. The server performs one
consistent read for admission, transition, custody, effect and result dimensions;
provider and OS observations include their provenance/time rather than pretending
to be part of that same database transaction.

The proposed `LifecycleSnapshotV1` contains schema version, store/subject identity,
commit sequence, context generation, transition disposition, current admission,
delivery outcome, effect settlement, runtime availability, blocking reason,
authority references, evidence references and measurement availability. It has
explicit unknown/not-applicable/unavailable values rather than ambiguous nulls.
Historical transition completion is shown separately from current admission: a
later transition may already have closed it again.

`wait(target, deadline)` returns `Observed(snapshot)`, `DeadlineExpired(last_seen)`
or `ObservationUnavailable(error)`. Targets name the exact transition/delivery and
required consequence. A composite proof predicate is not the universal definition
of lifecycle completion. Events are hints to reread; cursor gaps trigger a snapshot
resync. UI backpressure may coalesce presentation updates but cannot erase durable
results. Projection authorization and redaction prevent prompt, credential or
approval material leaking merely because it exists in storage.

Serde DTOs generate versioned JSON Schemas for the public wire shape. Schemas
validate structure, not authority, freshness, semantic sufficiency or settlement.
Command envelopes reject unknown authority-bearing fields; read-only readers have
a documented forward-compatibility policy. New structured signed/hashed payload
kinds use RFC 8785 JSON canonicalization, with a versioned digest envelope binding
domain, kind, schema/codec version and payload. Duplicate keys are rejected before
normal map parsing; invalid Unicode, non-finite values and out-of-range numbers
are rejected. Strings are preserved without Unicode normalization. Large counters/IDs and exact decimal
costs use specified string representations. Omitted and null remain distinct.
Binary artifacts use a separate exact-byte digest kind. Cross-language golden
vectors test both encodings; a generic serializer is not assumed canonical.
Internal permit types never appear in generated wire schemas.
[JSON canonicalization](https://www.rfc-editor.org/rfc/rfc8785).
[JSON Schema 2020-12](https://json-schema.org/draft/2020-12).

Historical references are not recanonicalized under that new envelope. For example,
the current claim-evidence codec sorts object keys by Unicode code point and adds
a trailing newline, while workspace coordination sorts keys by JavaScript's
default string order and adds no newline. S24/S25 in [DESIGN_EVIDENCE.md](DESIGN_EVIDENCE.md)
pin those inspected implementations. Compatibility vectors distinguish Unicode
ordering, newline bytes, numeric behavior and domain/version selection. Import
verifies each old reference using its original contract, preserves its identity,
and binds any new identity through an explicit migration record. Adding a codec
label to the Rust representation does not retroactively change old hashed bytes.
Unknown/ambiguous formats fail; trying several codecs until one passes is not
negotiation. S0 defines this boundary; S7/S9 implement only the legacy contracts
required by the accepted lifecycle import, not a claim/workspace rewrite.

Command access uses an authenticated local boundary, initially a permissioned Unix
socket and an explicit trusted-principal/grant resolver. Read and command clients
are distinct types and endpoints. Same-user hostile code is not defeated by Rust
types or socket permissions; stronger isolation needs a separately tested sandbox
or service-user boundary. A localhost address alone is not authentication.

The UI may first consume historical snapshots and read-only live projections.
Context-replace, recovery and disposition controls remain unavailable until their
command/authority/error contracts have their own acceptance evidence.

The implementation plan exposes two distinct UI handoffs: U0 is accepted DTOs,
generated schemas and representative compatibility fixtures; U1 adds the tested
read-only client and projection from the actual controlled Rust service. U0 is
not live integration, and U1 is not activation of the production proxy. These
observation deliverables can precede completion of the entire M2 provider/tool
profile. UI command capability remains a later, separately accepted handoff.

## Time and efficiency policy

Represent wall timestamps as validated `UnixMillis`, wait budgets as durations,
and in-process deadlines with a monotonic clock. Never pass an untyped integer
between these domains. Persist operational authority expiry; derive a bounded
monotonic budget at admission without extending the expiry on restart. Clock
rollback or inconsistent time observations fence new expiry-sensitive entry until
resolved. Late exact completion evidence is still retained and reconciled.

A caller's timer winning or losing a race is not sufficient evidence of timeliness.
Recheck the authoritative deadline after awaited boundaries and before returning a
time-bound proof success. A physically completed close stays completed even when
late; the proof is late/failed, not retroactively unclosed.

Pressure and efficiency are policies over evidence, not transition authority.
Initially support explicit replacement requests and revision-bound pressure
policy. Normal waiting is not a fault or a universal reset trigger. A critical
context limit cannot waive continuation verification; retain periodic durable
meaning early enough to avoid relying on a final impossible preservation turn.

Deliver cost accounting before prediction. Preserve raw provider usage and map it
to disjoint uncached input, cache reads, cache writes by class, and output. Record
provider/model/pricing revision and whether values are measured or unavailable.
Input age is not cache state; context occupancy is not billable input volume.

For equivalent useful continuation, report conditional payback:

```text
request_cost = U*pU + R*pR + sum(Wk*pWk) + O*pO
net_saving(N) = sum(saving_per_request[t], t=1..N) - replacement_overhead
break_even = ceil(replacement_overhead / constant_positive_saving)
```

Equality repays overhead; positive savings require exceeding it. No positive
per-request saving means no finite payback under that scenario. Rehydration and
cache-warming cost appear once, not twice. Unknown future request count stays an
assumption, not a fabricated forecast. Collect later continuation outcomes before
considering predictive policy or ML. Optimization cannot weaken semantic safety,
authority or effect accounting. This follows the existing
[efficiency proposal](../../ideas/01-semantic-durability-and-context-lifetime-judgment.md).

## Shutdown and process ownership

Runtime/process shutdown is distinct from context replacement. The supervisor
owns process handles, birth identity/incarnation, task sets and a monotonic launch
registry. All provider-capable launches register before they can execute; shutdown
first closes the launch/admission gate so it can drain a stable owned set. It does
not repeatedly sample a mutable `activeWorker` accessor and hope no replacement
appears after the last await.

The explicit protocol is: refuse new entry and respawn; snapshot owned attempts;
request supported cancellation where authorized; drain local tasks/results;
account for remote effects; close transport; observe/reap exact children; persist
the shutdown report. A child that cannot be identified or reaped remains unresolved.
PID reuse is handled with birth identity or a platform process handle.
The store writer and result-ingress channels remain alive until task outcomes and
the shutdown report are committed; then the writer checkpoints as appropriate and
closes. Closing storage before its outstanding result producers is not clean
shutdown. If storage is unavailable, retain an explicit unresolved shutdown result
rather than claiming a persisted report.

`CloseReport` separately records attempted/not-attempted, resolved/rejected/timed-out,
completion time/late status, each process exit, local tasks, remote settlement,
and unsafe-at-close history. A later zero active count does not erase an unsafe
close. Diagnostic cleanup may have a separately bounded resource-cleanup budget,
but it cannot extend proof authority or convert an expired proof into success.

RAII releases local resources; it is not an async shutdown acknowledgment. Tokio
child handles normally allow a process to continue after handle drop. Use explicit
owned shutdown and wait/reap, with kill-on-drop only as a fallback containment aid.
Neither process exit nor task cancellation proves remote settlement.
[Tokio Child](https://docs.rs/tokio/latest/tokio/process/struct.Child.html).

## Proof orchestration and diagnostics

One proof program drives the real service composition for controlled and live
modes. Mode changes provider transport/profile and permitted authority, not the
transition sequence. The controlled mode is non-authoritative for live activation.
Live proof consumption fixes identities, budget and deadline before bootstrap.

A setup owner registers resources before launching them and owns cleanup through
setup failure. It explicitly transfers an immutable resource scope to the runner.
The runner enters before observation/capture waits and receipt construction. It
cannot accept an options map that overrides fixed authority, identity or deadline.
There is no alternate exported unbound runner entry point.

Proof identity binds consumed run ID, executable build, source/profile digests,
state root, provider boundary and runtime incarnation. Launch label and build ID
remain distinct. Replacing the incarnation invalidates a same-execution proof even
if normal service recovery could succeed. Restart-recovery proofs deliberately
span a separately declared lineage; they are not mislabeled same-incarnation runs.

Receipt input comes from that run's committed evidence. Validate the candidate
receipt before teardown. Return a successful proof result only after required
settlement and teardown succeed within its contract. Offline cutover preparation
then runs as a separate stage; it never depends on a still-live proof process.
Failures retain the execution error and cleanup report together, without masking
the original error or pretending missing files are complete observations.

Deterministic barriers acknowledge the exact effect/attempt/incarnation and
ordering under test. Prefer a typed test-control channel over file watchers. If a
file artifact is required, publish complete bytes atomically without overwrite
and reject malformed/wrong-identity content. Diagnostic envelope limits are
documented separately from execution limits. Production defaults disable test
controls; no environment flag silently enables provider entry.

## Migration and executable upgrades

Executable deployment and context lifecycle remain coordinated owners, not one
state machine. Deployment selects validated builds and owns launch/drain
coordination and compatibility; lifecycle controls context transitions, admission
and custody. Its executor performs owned-task draining and supplies the truthful
shutdown report; deployment cannot substitute process exit for effect settlement.
Context generation, executable generation, native provider session and OS process
identity are bound explicitly rather than assumed equal or one-to-one. A context
cycle may leave the executable/process alive, and an executable restart need not
authorize a new context or delivery attempt.

Context cycles do not create a new lifecycle database. A compatible executable
upgrade reopens the stable store under its exclusive writer/startup fence and
reconciles state before exposing domain admission. This removes cross-database
custody transfer from the ordinary context path. It is an explicit departure from
copying every legacy generation-root mechanism.

The replacement proves its real startup, fencing, draining and handoff boundary
with a selected permanent Rust startup/deployment owner. The exact owner,
consumer and contract are an S9 handoff, not an assumption that the earlier Node
generation-integration alternative must be implemented. Artifact/runtime identity,
captured configuration, saved path bases and persisted restart selection remain
required; ambient build output cannot select a different executable. Offline
exclusive transfer does not require online coexistence or hot rollover.
The entire executable-generation manager and other Work Engine domains need not
move into lifecycle. A necessary temporary bridge identifies its actual consumer,
why direct replacement cannot provide the needed result, bounded scope/owner and
removal condition; it cannot supply a second lifecycle controller. Separate
deployment ownership cannot defer the required S9 cutover/rollback evidence.

Saved-state disposition is unresolved: the human/state owner chooses import or
explicit archive/clean start against the actual inventory and obligations. No
choice, deletion or retirement follows from the offline development premise.
The import/T25/current-custody and rollback requirements below remain in force
until that owner explicitly accepts a revised compatibility/disposition contract.
An archive decision must account for outstanding custody, unresolved effects and
historical references; archival alone does not discharge them.

An initial JavaScript-to-Rust import requires an offline, authority-bound
handoff. Produce a read-only inventory from coherent SQLite/WAL and registry
images, classify history versus current obligations, and bind exact canonical
paths, file identities, schema and source revisions. Do not count historical
`recovery_required` rows as live obligations solely by label. Owner gaps and
unfinished delivery claims are blockers unless independently resolved.

For cross-store transfer use the following crash-safe protocol:

1. Pin source/destination identities and reserve a handoff ID. Bootstrap starts
   from an actually absent destination root, imports the exact current capsule
   into an inactive store, and records acceptance without delivery permission.
   No fixture or operator prepopulates the destination's custody rows.
2. Under the shared launch/admission fence, revalidate the source snapshot and
   atomically mark its custody owner fenced to that handoff. Existing provider
   entries must already be settled or explicitly represented as unresolved.
3. CAS the launch manifest from the exact source owner to the accepted destination
   and handoff revision. All launch paths, including activation and rollback, use
   this gate before spawning provider-capable children.
4. Destination verifies source fence, manifest selection and capsule acceptance,
   then commits ownership before admitting anything. Crash retries use the same
   identities; conflicts and partial adoption remain inactive.

There is no instant in which both sides possess delivery authority. Failure after
source fencing may leave neither side active; recovery restores availability only
through the handoff protocol, not by bypassing the fence. Old executables must
actually reject the upgraded/selected state or be enforceably excluded by the
launcher. A document saying they should reject is not sufficient.

Migration preserves input bytes, stable delivery IDs, successor acceptance,
terminal-retirement fences, pending/entered/completed claims, receipt lineage and
the current owner. Q3/Q4-like obligations require their own fresh reconciliation;
this design does not authorize or assume their present disposition.

Rollback uses the stopped candidate's **current** custody snapshot, not an initial
pre-cutover capsule. It cannot undo external effects, resurrect retired inputs,
or replay completed/uncertain deliveries. Code rollback is allowed only to a
binary compatible with the current store/evidence contract. Otherwise use an
explicit forward recovery or separately proven cross-store export/import. Never
claim safety by restoring an old database backup after new external work occurred.

Filesystem and manifest locks use stable lock identity; a contender must not unlink
another owner's lock during cleanup. Fresh paths, canonical path equivalence,
symlinks, process-start identity, paused launches and spawn-before-registration
crashes are acceptance cases. Path validation occurs before migration/mutation and
is revalidated through the commit boundary.

## Acceptance and implementation sequence

The complete requirement and regression matrix is in
[DESIGN_EVIDENCE.md](DESIGN_EVIDENCE.md). Existing passing tests are evidence for
their stated scope only; proposed tests below are not claimed to have run.

| Milestone | Deliverable and exit condition |
| --- | --- |
| M0 Design acceptance | Review the ownership, provider/session constraints, semantic contract, migration choices and requirement matrix. No implementation authority is implied. |
| M1 Durable Rust service | Replace prototype persistence/transport with the reducer/store/executor boundary; prove public-path crash cuts, exclusive ownership, cancellation and bounded waits. Preserve existing text behavior. |
| M2 Provider and observation contract | Versioned read-only wire projection, exact native adapter contract, adversarial transport/history tests, and a small owned-operation profile with explicit executor and authorization guarantees. UI can connect read-only. |
| M3 Semantic preservation | Bounded source projection, compiler/verifier effects, authority/reference reload, realistic semantic continuation and insufficient-material negatives. |
| M4 One retained workflow | Full controlled host path with active work, pressure/yield, verified checkpoint, successor, two queued inputs, supported tools and restart. No test-side advancement. |
| M5 Recovery and migration | Per-claim recovery/disposition, upgrade admission, fresh-root custody handoff and current-state rollback; old executable refusal and launch-race tests. |
| M6 Replacement acceptance | Independent review, provider-disabled production-composition gate, separately authorized bounded live proof, staged selection and operational rollback readiness. |

The milestones describe acceptance consequences, not a requirement that every
file be written serially. [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) splits
delivery into a foundation, bounded parallel lanes and integrated joins. Core/
storage and execution share the accepted command/effect handoff; wire/client work
shares only the public contract. One builder owns complete service integration.
Separate component passes never satisfy the M1 service exit condition. Semantic
and recovery work may overlap only when their state/operation contracts are
stable and shared core/migration changes remain serialized through that owner.

At M4 use an agreed actual retained-workflow consumer in a permanent Rust host,
with a non-production subject, realistic semantics and supported tool profile.
The lifecycle and host owners bind its entry path and exact revision before S8;
a proof-only host or mock role cannot substitute for actual host admission.
Native-review ownership is not implicitly merged with lifecycle, and HP1 or its
independent domain work does not wait for S4. Both controlled and
live acceptance must use the same installed service build and scenario driver.
Live proof remains separately authorized and budgeted; the experiment's three
turn allowance is consumed and cannot be reused.

Model-based tests explore event permutations and assert invariants after every
step. Property tests shrink failures; Loom is useful for small in-process
synchronization components, not as proof of SQLite, OS, provider or semantic
behavior. Actual-process tests kill/restart at acknowledged boundaries and
preserve request counts, committed snapshots, receipt identities and exact exits.
[Proptest](https://proptest-rs.github.io/proptest/intro.html),
[Loom](https://docs.rs/loom/latest/loom/).

The gate includes formatting, Clippy with warnings denied, unit/property and
compile-fail tests, SQL migration/compatibility tests, adapter contract tests,
actual-process failure orderings, full host composition, wire compatibility and
packaged-executable startup. The fixture provider and offline admin paths are not
included in production executable capabilities by accident. Diagnostics are
tested both off and on with the same argument/payload envelope as the full gate.

Unexpected failures stop advancement and preserve their evidence. Do not repeat
until green, weaken assertions, or treat a later nonrecurrence as an explanation.
Separate unrelated gate exceptions remain exact-subject decisions; they are not
inherited automatically. Compiler success is necessary, not completion evidence.

## Design review questions and remaining evidence

These are explicit acceptance decisions and experiments, not omissions to be
silently decided by an implementation worker:

1. Accept the fresh-thread default and identify subjects requiring native
   same-session continuity; those routes remain disabled until supported.
2. Select the first owned-operation profile, its execution owners and entry/commit
   authority semantics; establish the evidence for outcomes and settlement. Do not
   promise a universal external-effect seal or adopt the unresolved broad commit
   gate implicitly.
3. Select the allowed rollback compatibility range and the human/state-owner
   import versus archive/clean-start disposition. Preserve T25/current-custody
   evidence until an exact owning-contract amendment is accepted; actual legacy
   migration/disposition remains a separately authorized operation.
4. Select the semantic verifier's quality/independence requirement and realistic
   continuation corpus. Mechanical digests cannot set that quality bar.
5. Select deployment trust boundaries, data retention/redaction policy and bounded
   operating limits before production activation. The initial single-host trust
   assumptions are not multi-tenant security guarantees.
6. Agree S8's actual permanent Rust host consumer and S9's permanent Rust
   startup/deployment boundary with their owners; the native-review and earlier
   Node generation plans do not select them by implication.

No full Rust replacement, tool-enabled live lifecycle, independent production
review, performance advantage, or semantic-quality guarantee has yet been
established. The multi-slice plan retains the end-to-end replacement target;
each executable slice still binds its exact scope, contract revision, dependency
set and discriminating tests before implementation authority is requested.
