# Rust review-episode implementation plan

Status: proposed, 2026-10-07. This is implementation planning, not acceptance,
worker launch, runtime execution, provider entry, live-state migration or cutover
authority. No implementation or runtime tests were performed for this document.
The user coordinates lifecycle and UI work separately.

## Outcome and bounded ownership

Move the App Server's provider-neutral review-episode state, deterministic
transitions and SQLite persistence to Rust. Preserve exact historical identities,
retained reviewer continuity, findings, writer replacement, evidence admissions,
CAS and recovery through the existing native-review consumer. An episode records
review truth; it does not select reviewers, establish independence, accept a
campaign, execute a provider or decide whether a finding is valid.

The governing inputs are [DESIGN](../DESIGN.md), [PHILOSOPHY](../PHILOSOPHY.md),
[Rust direction](rust-development.md), [migration priorities](rust-migration-plan.md)
and the existing [S9 episode contract](../app-server/docs/skills-migration-plan.md#s9--provider-neutral-implementation-review-and-episode-service).
The [lifecycle S0 integration owner](../experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md#s0--shared-foundation-and-executable-slice-contracts)
owns root workspace, lockfile, toolchain, common types/encodings and gate inventory.
This lane requests additions there; it creates no second foundation owner.

## Observed boundary and competing placements

| Current owner / evidence | Consequence for this migration |
| --- | --- |
| [Episode contract](../app-server/src/services/review-episode/contract.mjs), lines 48–138; [service](../app-server/src/services/review-episode/service.mjs), lines 21–185 | Six-part identity, authority manifest digest, writer/session, current subject/result, transition digests, v1/v2 state and immutable history are episode-owned. Service calls are synchronous. |
| [SQLite store](../app-server/src/services/review-episode/sqlite-store.mjs), lines 8–85 | Current/history rows contain canonical JSON; history has predecessor revision and sequence. `BEGIN IMMEDIATE` checks CAS and commits both rows. WAL/FULL, private-file opening and read integrity checks are existing behavior. |
| [Native host](../app-server/src/services/slice-campaign/native-review-host.mjs), lines 238–336 | Opens `review-episodes.sqlite3`, constructs the episode owner, derives authority from campaign selection and binds the reviewer session and immutable subject. |
| [Native closure](../app-server/src/services/slice-campaign/native-review-closure.mjs), lines 80–207, 224–380 | Consumes synchronous episode reads/transitions inside initial review, correction, remediation and claim succession. Provider calls themselves are asynchronous. |
| [Implementation-review admission](../app-server/src/services/implementation-review/service.mjs), lines 3–25; [contract](../app-server/src/services/implementation-review/contract.mjs), lines 59–124 | Validates result shape, finding/verdict constraints and exact subject. Explicitly grants no mutation, acceptance, reviewer-selection, human or independence authority. |
| [Finding bridge](../app-server/src/services/claim-evidence/review-finding-bridge.mjs), lines 40–100 | Publishes separate exact claim revisions/reliances from episode results. Claim publication and builder applicability remain outside the episode owner. |
| [Production-path evidence](../app-server/src/services/claim-evidence/production-path-service.mjs), lines 11–118; [campaign succession](../app-server/src/services/slice-campaign/service.mjs), lines 327–371 | Claims evaluate realization/observation and retain succession; campaign owns selection and consumption. Their databases do not share the episode transaction. |
| [App Server exports](../app-server/src/index.mjs), lines 76–83 | Existing service/contract/store exports are compatibility consumers; replacing the native factory does not automatically retire them. |

Placement alternatives:

| Alternative | Assessment |
| --- | --- |
| Rust SQLite wrapper under the existing JS reducer | Smallest edit, but leaves transition authority/stringly state in JS and adds a permanent-looking split. Useful only as a diagnostic prototype; not the intended migration outcome. |
| Domain core/store plus one-command Rust executable and synchronous host adapter | Recommended first complete vertical. Preserves synchronous composition and transaction semantics without a resident service, lifecycle runtime or new public API family. Costs process launch and bounded host blocking. |
| Rust core/store behind a Node native addon | Preserves calls with lower launch overhead; adds ABI/build/distribution and panic/host-process concerns. Reconsider if measured command latency makes the executable boundary unsuitable. |
| Long-lived local service with wire/client packages | Appropriate when multiple independent consumers or sustained load justify it. Now requires async propagation through host, closure, campaign and claims callbacks, plus endpoint authentication, shutdown and reconnect contracts. Defer; do not silently return Promises from synchronous methods. |
| Merge into lifecycle or claim-evidence | Reuses nearby mechanics by conflating authorities. Reject: lifecycle custody and claim establishment are different domains. |

Propose three packages: `review-episode-core`, `review-episode-store`, and
`review-episode` (application library plus `work-engine-review-episode` binary).
DTOs/schema fixtures begin as modules in the application package, not separate
wire/client crates. Core depends only on approved value/encoding dependencies;
store depends on core and SQLite; application composes them. No dependency on
`lifecycle-core`, `lifecycle-store`, `lifecycle-runtime` or lifecycle availability.
Reuse a proven SQLite-opening helper only if S0/S1 exposes it without those
policies; a general persistence or process framework is not a prerequisite.
Otherwise implement the existing opening behavior within the episode store;
agreement on that placement does not require lifecycle to ship a new helper.

## Contract to preserve, with explicit semantic decisions

- Distinct identity, state revision, authority-manifest digest, writer generation,
  provider-session reference, subject digest and result digest remain distinct
  types. The runtime session is resumable data, never an authentication credential.
- `begin` binds generation 1; `resumeInitial` is observational and only valid for
  an active initial episode without a result. Reported episodes may receive later
  exact remediation subjects; reporting is not retirement.
- Every mutation binds expected revision and transition identity/content. Exact
  replay performs no mutation; conflicting reuse fails. Current behavior returns
  the latest state on matching replay, not necessarily the original response.
  Preserve that distinction explicitly rather than inventing an exact-reply claim.
- Result admission requires the current active initial/re-evaluation phase,
  authority/writer match and subject digest match. Preserve all prior finding IDs
  and immutable finding fields; status/remediation evidence may evolve. History
  preserves the writer attributed to each result and each replacement.
- Replacement requires a separately authorized successor, next writer generation,
  exact predecessor revision and current subject. Continuity becomes reconstructed;
  replacement never establishes fresh independence. Uncertain episodes cannot
  publish results; retirement remains terminal and protects its references.
- Decode v1 and v2 without upgrading history. V2 preserves every admission's exact
  claim, establishment, optional observation, consumption, status, boundary and
  consumer. A false/unestablished required admission blocks dependent consumption.
  Succession retains predecessors and the immutable result; the current route
  appends two established successor admissions to the two predecessor admissions.
- Rust enums/private constructors reduce invalid in-process combinations. They
  do not prove caller authority, observation authenticity, provider continuity,
  semantic evidence sufficiency, cross-store settlement or a truthful import.

The current implementation is evidence, not an unconditional correctness oracle.
R0 separates compatibility cases from these source-observed questions; no runtime
failure or exploit is claimed here:

| Observation | Proposed bounded disposition |
| --- | --- |
| Service `transition`, lines 75–85, returns matching replay before checking current revision/writer; `begin` has a similar replay shortcut. | Authenticate the host and authorize observation before returning any state. Decide explicitly whether a superseded writer may receive an authorized replay read; a stricter rejection is a documented semantic delta, not hidden parity. New writes remain fenced. |
| `succeed_evidence`, lines 123–130, chooses reported/remediation from verdict without considering retained unresolved questions; `record_result` does consider them. | Add an adversarial characterization case. Propose preserving unresolved work in phase/pending action, subject to episode-owner acceptance; do not normalize historical states or silently change hashes. |
| A v2 episode may record another result without new admissions while retaining old admissions; correction/remediation routes omit `admitProductionPath`. | Pin which claims are subject/result scoped versus retained-episode scoped with claims/campaign owners. Do not let the Rust bridge certify old evidence as establishing a different result. Preserve history; block affected new consumption if required binding is unestablished. |
| Claims `correct`, lines 93–115, calls `succeedEpisode` before the claim-store correction commit; campaign publication follows afterward. | There is no cross-store transaction. A CLI adds a process-loss boundary to this existing ordering. Characterize each join cut; retain incomplete joins as unresolved. A bounded reconciliation correction may be needed for this operation, not a general claims rewrite. |

If characterization needs a semantic change, record exact old/new behavior,
owning acceptance and affected references in a separately accepted correction
slice. Domain/store work can proceed while that decision is pending; affected
operations cannot be claimed qualified or silently removed from full cutover.
A separately accepted bounded migration profile may leave an affected operation
on its existing owner or visibly unavailable; it must not claim full replacement.

## Trustworthy admission and synchronous integration

Keep the existing trusted native host as the authority/admission adapter during
this migration. It derives grants and immutable subjects from admitted campaign
selection, uses canonical implementation-review validation, and obtains claims
decisions from the existing claim owner. The Rust application revalidates the
deterministic result contract and subject/finding lineage, using a versioned
compatibility module whose normative meaning remains implementation-review v1.
This ports the consumed validator, not reviewer selection/execution or claims.

Propose a private `AdmittedResult`/`AuthorizedCommand` boundary: constructors are
available only after the application validates a host-bound request against its
configured authority/admission port. No public DTO containing `authorized: true`
or a claimed establishment status can construct these values. Bind admission to
episode identity, writer generation/session, expected episode revision, selected
subject, result digest, selection revision, and exact required evidence refs.
Recheck the episode CAS inside the write transaction after any external validation.

The host bridge invokes a pinned binary directly, without a shell, through private
inherited input/output; executable, database root and authority source come from
trusted host configuration, never reviewer/tool request fields. Production access
is the host's existing launcher/OS/capability boundary: same-user hostile processes
are not excluded merely by JSON validation or a private Rust type. Verify the
real review grant cannot reach writable episode files or a privileged invocation.
If that boundary cannot be established, no production cutover is justified;
a public standalone CLI needs its own authentication contract and is out of scope.

The transitional bridge accepts only admissions it constructs from the owning
services. It resolves ordinary evidence against exact owner records, checking
digests, claim subject/consumer/boundary and observation/result association rather
than accepting caller-provided refs/statuses. Succession's not-yet-committed
establishments require the explicit claims-owned correction handoff and recovery
disposition above; they cannot masquerade as already durable records. Missing
verification fails closed for that operation. Rust never writes the claims DB.

Expose the existing synchronous `begin`, `resumeInitial`, `transition`,
`validateResult`, `recover`, `read` and `history` behavior through a small JS
compatibility adapter. Map typed Rust failures back to the existing result-contract
versus transport/store error distinction used by `resultContractFailure`.
Frame/version/size-check requests and responses, bound duration/output, correlate
operation IDs, redact diagnostics and distinguish no-start from uncertain outcome.
Oversize history fails explicitly; never truncate an array and report success.
On timeout, broken pipe or bad response, read the committed transition/history
before deciding whether a retry is valid. No automatic JS writer fallback and no
provider replay. Process exit alone is not proof that SQLite did not commit.

Measure launch cost, large-history cost and event-loop blocking under the controlled
host workload. Keep the synchronous bridge only if its bound meets the host's
accepted responsiveness budget. An async successor is a separate interface change;
it must include claims' synchronous `succeedEpisode` callback and campaign callers,
not just the episode factory. No performance benefit is asserted in advance.

## Encoding, persistence and historical owners

Agree a named legacy codec with S0, provisionally `review-episode-js-json-v1`:
recursive JS UTF-16 key ordering, JS JSON scalar/string rules and no trailing
newline. Its trusted domain/version selects the codec. Preserve identity-key,
authority, transition, subject and state-revision inputs; state revision excludes
the `revision` field, while the stored JSON includes it. Do not substitute the
claim-evidence newline/code-point codec or a new universal digest.
Implement this compatibility codec in the episode core unless another accepted
Rust consumer justifies sharing it. S0 owns common registration/interface changes;
it need not implement this domain's legacy codec before its own lifecycle work.

Golden vectors cover non-ASCII/supplementary keys, escapes and lone surrogates,
safe-integer bounds, missing versus null, arrays, nested fields and v1/v2 states.
Serde defaults are not proof of JS equivalence. R0 determines the compatibility
representation for values Rust strings cannot represent; unsupported historical
values receive explicit import refusal/quarantine, never lossy normalization.

Keep the current two-table JSON/history layout initially. The Rust store owns
private-file checks, chosen durability settings, read validation and one transaction
for current CAS plus history append. Validate identity-key association, revision
hash, predecessor chain and current/history agreement during import; strengthen
integrity checks without rewriting valid rows. No SQLite transaction spans claims,
campaign or external provider effects. Concurrent valid writers resolve by CAS.

The [Python profile](../skills/independent-review-state/SKILL.md) remains a real
legacy owner, not the App Server store adapter. Its [implementation](../skills/independent-review-state/scripts/independent_review_state.py)
uses GitRef durable state in namespace `independent-review-state`, snake_case
schemas, different reference/finding fields, optional authority expiry and Git
revision provenance. The [MCP server](../skills/work-engine-mcp/scripts/server.mjs),
lines 172–242, still invokes that Python owner with an episode-bound authority.
The [legacy parity test](../app-server/tests/services/review-episode/legacy-parity.test.mjs)
checks finding vocabulary conversion only; it is not state/revision equivalence.
Leave this route and Git refs intact. A later accepted import must retain original
Git revisions, byte codec, per-finding attribution and explicit old/new mapping;
do not map its revisions to App Server SHA-256 states by renaming fields.

## Proposed delivery slices and file ownership

One retained `gpt-6-sol` builder at `high` can own this complete lane, including
integration/tests/remediation, after authorization. Consequential review uses a
separate initially isolated `gpt-6-astra` reviewer at `xhigh` where the owning
review contract permits it; this planner is not that reviewer. Follow
[model-selection guidance](model-selection.md), including requested/observed
identity and no inferred cross-provider independence. Core and adapter work uses
disjoint paths from the compiler lane. For this proposed pair, R3 owns edits to
`native-review-host.mjs` and `app-server/src/index.mjs`; compiler C3 requests any
needed review-host adjustment through this lane. Root workspace changes remain
with the S0 integration owner, and generation inventory/activation changes remain
with their deployment owner. Shared integration edits are serialized.

| Slice | Exact proposed ownership and prerequisite | Reviewable exit |
| --- | --- | --- |
| R0 — Freeze contract and evidence cases | This plan; `app-server/tests/fixtures/review-episode-rust/{codec-v1,states-v1-v2,commands,semantic-deltas}.json`; proposed `app-server/tests/services/review-episode/rust-contract-characterization.test.mjs`. Requires bounded implementation/test authority; fixtures may precede S0 agreement. | Exact oracle baseline, trusted admission contract, sync/error/replay behavior, required-claim scope and semantic-delta decisions. Submit dependency/codec requests to S0; do not create a workspace. |
| R1 — Typed core and result compatibility | After accepted S0 value interfaces and codec namespace/placement agreement: `rust/crates/review-episode-core/Cargo.toml`, `src/{lib,identity,codec,state,command,reducer,implementation_review_v1}.rs`, `tests/{contract,codec,transitions}.rs`. Root changes remain S0-owned. | Pure no-I/O transitions, validated inputs and property/golden tests for every action, v1/v2 and finding lineage. This is the first runnable offline Rust increment; not consumer replacement. |
| R2 — Durable owner and command executable | `rust/crates/review-episode-store/Cargo.toml`, `src/{lib,sqlite,integrity,import}.rs`, `tests/{compatibility,cas,recovery}.rs`; `rust/crates/review-episode/Cargo.toml`, `src/{lib,main,protocol,admission}.rs`, `tests/{command,crash}.rs`. Depends on R1; opening mechanics may be domain-owned. | Real CLI begin → result → process exit/reopen → exact read through the production application/store, with CAS and crash cuts. Source-copy importer/validator; no live DB opening. |
| R3 — Native consumer integration | New `app-server/src/services/review-episode/rust-adapter.mjs`; bounded edits to `native-review-host.mjs`, `native-review-closure.mjs`, `app-server/src/index.mjs`; new `app-server/tests/services/review-episode/rust-adapter.test.mjs` and `app-server/tests/review-episode-rust-vertical.test.mjs`. Depends on R2 and accepted admission boundary. | Existing consumer uses the actual Rust executable and store for all episode operations, retaining JS claims/provider owners. No new reducer in adapter. Exact result-contract errors and restart recovery reach campaign/finding consumers. |
| RC — Conditional semantic correction | Only if R0/R3 exposes a required correction: freeze the smallest manifest in episode core and/or existing `slice-campaign/{native-review-closure,service}.mjs`, `claim-evidence/{production-path-service,sqlite-store}.mjs` plus focused tests. Requires the affected owners' accepted behavior. | Characterized replay/question/evidence-scope or correction-join defect is resolved with exact lineage preserved. Claims retain their authority; no broad Rust claims migration or universal transaction framework. |
| R4 — Data and failure qualification | R2/R3 test paths plus `docs/rust-review-episode-migration.md` (future runbook). Depends on full operation coverage and disposition of relevant RC cases. | Copied-database import/read/write-back compatibility, competing processes, downstream failure/restart and binary/codec evidence. No unresolved claim is presented as established; no live data changed. |
| R5 — Owner cutover and predecessor disposition | Accepted deployment configuration/manifest paths selected by deployment owner; documentation/export cleanup only after consumer inventory. Requires separate operational authority and R4 evidence. | One admitted writer implementation per state root, drained host requests, exact readback and tested rollback. Retire old JS writer only for migrated roots; Python/Git owner and historical exports receive explicit independent dispositions. |

Before shared-foundation agreement: refine R0, capture approved source fixtures and
consumer cases, settle authority and compatibility questions. After agreement:
R1/R2 compile against its exact revision. R3/R4 do not wait for lifecycle S4–S10;
only consumed value/codec/opening contracts are dependencies. UI U0 at lifecycle
S3 and U1 at S4 remain unchanged and user-coordinated; this lane edits no UI files.

## Meaningful qualification and future commands

The acceptance matrix covers (a) exact serialized identities/state/history parity,
(b) unauthorized or stale authority and replacement, (c) all legal/illegal phase
transitions, (d) immutable findings and admission lineage, (e) SQLite/file integrity
and simultaneous-process CAS, and (f) real host-to-Rust-to-claims/campaign recovery.
Use generated command sequences to test retired/uncertain fences and replay
invariants; use actual processes for commit/reply/crash and competing-writer cuts.
Do not use an in-memory store to establish SQLite durability or a fake reducer to
establish integration. Fixtures may substitute the external reviewer only.

Extend [native vertical cases](../app-server/tests/slice-campaign-native-review-vertical.test.mjs)
and [host integration cases](../app-server/tests/supervisor-native-review-hosting-integration.test.mjs)
for retained remediation, replacement without freshness claims, post-result finding
publication failure, stale campaign CAS, established/false/unestablished admissions,
exact correction succession and response loss after commit. Observe immutable
episode references consumed by finding publication and campaign terminalization;
an isolated Rust green test is insufficient. Count provider entries in the
controlled peer and prove recovery does not create another entry.

Future commands, after approved implementation and S0 dependency resolution:

```bash
cargo test --manifest-path rust/Cargo.toml --locked -p review-episode-core -p review-episode-store -p review-episode
cargo build --manifest-path rust/Cargo.toml --locked -p review-episode --bin work-engine-review-episode
node --test app-server/tests/services/review-episode/*.test.mjs app-server/tests/review-episode-rust-vertical.test.mjs
node --test app-server/tests/slice-campaign-native-review-vertical.test.mjs app-server/tests/supervisor-native-review-hosting-integration.test.mjs
```

The future Rust vertical selects the built binary explicitly and disables external
provider access. Final test manifest includes admission/claims regression cases
actually touched by RC. Run S0's required formatting/lints/dependency gates too;
this plan does not override them. Any separately required live proof needs its own
authority and exact build/profile/evidence binding, never fabricated independence.

## Import, cutover and rollback

Inventory selected state roots under later read authority; do not presume the
current production data set or that all episodes are closed. Snapshot SQLite
consistently, including WAL state, while fencing writers. On copies, validate all
current/history rows, sequence/predecessors, authority/result/subject digests and
exact external refs. Record source digest, row counts, rejected rows and destination
digest. Preserve raw historical bytes and schema versions; never regenerate old
revisions from Rust-normalized DTOs. Missing external evidence remains unresolved.

Prefer the identical schema and data bytes for the first cutover. Qualify Rust
reading JS rows and JS reading newly produced Rust rows, including restart/replay.
Do not dual-write for parity: compare disposable copies. At cutover, the deployment
owner drains host commands, fences the JS opener, records selected binary/codec and
root identity, then enables the Rust owner with exact readback. A host crash/lost
reply triggers durable reconciliation, not a fallback writer or provider replay.

Rollback after Rust writes is allowed only if the old reader/writer accepts every
new state and the accepted semantics remain valid. Otherwise stop writes and use
an accepted forward repair/export; restoring an earlier backup would discard
review truth. Optional side tables/protocol changes require explicit backwards
compatibility tests and disposition. Keep pre-cutover snapshots as evidence,
never as an excuse to erase later transitions. Python Git-backed episodes are
neither imported nor retired by this cutover.

## Evidence baseline and remaining decisions

Inspected HEAD: `7583e2095725957470e28cc11fc69404386fec63`. All linked tracked
source/test files and DESIGN/PHILOSOPHY matched that HEAD at inspection. Material
working-tree inputs were hashed separately (SHA-256); later coordination additions
may legitimately supersede the migration-plan input without rewriting this record.

| Working-tree input | SHA-256 at inspection |
| --- | --- |
| [AGENTS.md](../AGENTS.md) | `04675bbabbf39576e4590cc8dfa2d681f7a630572ccd5f88bc62ebc422e4d9b8` |
| [Rust direction](rust-development.md) | `aca53736538773d86ca5c0e9381752814fa1f59ff0a23dd12cff249e37c36f37` |
| [Migration plan](rust-migration-plan.md) | `830dd4de26a27d7f28408e44d05f6ea5f8d890f6925103b2364bf5bfdf94e102` |
| [Model guidance](model-selection.md) | `f5c594118b0a151056cea38281b7db8f8b0904de02eea87d586d2697fdfa452c` |
| [Lifecycle implementation plan](../experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md) | `02c8ad62b958708b9d841eacfd29dda9601096f2bca9ba4cfb713d78a4d8baae` |

Evidence tier: Verify (Tier 2), project `home-bline-code-work-engine`, ready index
generation `2026-10-07T06:51:24Z`. Exact scoped symbol searches and relevant
depth-1 traces in both directions were exhausted. A broad claims search was
narrowed to its production-path owner; it was discovery, not an exhaustive claim.
Dynamic injected object calls were verified by direct source because graph edges
did not fully represent them. Coverage checked every relied-on path: no recorded
issue, metadata match. Python scope excluded generated `__pycache__`; the actual
Python source was read. Coverage is best effort, not proof of completeness.
No live data inventory, benchmark, runtime proof or independent review is claimed.

Acceptance still needs: the S0 value interfaces and codec/opening placement agreement;
the host's concrete admission/access boundary and responsiveness budget; explicit
disposition of characterized semantic differences; required-claim scope across
remediation; selected data roots and cutover/rollback authority. None requires
waiting for full lifecycle replacement or expanding this into review orchestration.
