# Review-episode R2: durable owner and offline command executable

Status: proposed bounded implementation plan, 2026-10-07. The current authority
is **planning only**. This document creates no implementation, test execution,
provider, live database, import, deployment or publication authority. It refines
R2 in the [reviewed lane plan](rust-review-episode-implementation-plan.md), keeps
the [R0 contract](rust-review-episode-r0-contract.md) frozen, and consumes the
[accepted R1 result](rust-review-episode-r1-result.md). The user selected
`gpt-6-sol`/high for future implementation and remediation. This planning task
was requested as `gpt-6-astra`/xhigh; independently attested effective runtime
identity is unavailable.

The implementation base is C1/R1 commit
`8497df02e055c3b93e865cbe75fe4c99eb555eb4`, following S0
`84be6695d716f026460fbf11aa703e73631e514e`. The
[combined validation](rust-c1-r1-validation.md) is historical qualification;
its original “uncommitted” wording predates that commit. Concurrent lifecycle
S1 and unrelated staged UI work remain separately owned.

## Outcome and exclusions

R2 supplies an episode-owned SQLite store, an application library and the
one-request `work-engine-review-episode` executable. An offline host fixture
must drive **begin → result → process exit → reopen → exact revision/history
read**, through the actual application, R1 reducer and SQLite store. The same
path must preserve command replay, fence stale writers, survive interrupted
transactions and distinguish a committed command from a missing response.

This is executable store/application qualification. It does not qualify the
production native host, claim establishment, reviewer independence, campaign
acceptance, the three-store correction join or a migrated state root. R3 owns
native integration and real host access/admission evidence; RC owns the frozen
cross-owner corrections; R4/R5 retain data/operational qualification. No edits
to JavaScript owners, native host/closure, Python/Git episode state, lifecycle,
UI, claim stores or deployment configuration belong to this slice. No provider
client or automatic JavaScript writer fallback is added.

## Owned files and shared integration request

These are the proposed implementation paths. A retained Sol builder owns the
whole vertical and ordinary remediation after authorization, preserving other
actors' edits. Adding another path requires an explicit ownership disposition.

| Owner | Exact paths | Responsibility |
| --- | --- | --- |
| R2 store | `rust/crates/review-episode-store/Cargo.toml`; `src/lib.rs`, `src/sqlite.rs`, `src/integrity.rs`, `src/import.rs` under that crate | Domain errors/API, private opening/schema/transactions, row/history validation, copy-only import. SQL constants remain in `sqlite.rs`; no migration framework. |
| R2 store tests | Same crate: `tests/compatibility.rs`, `tests/cas.rs`, `tests/recovery.rs` | Historical bytes/schema, SQLite CAS, integrity/import and transaction failure evidence. |
| R2 application | `rust/crates/review-episode/Cargo.toml`; `src/lib.rs`, `src/main.rs`, `src/protocol.rs`, `src/admission.rs` under that crate | Application composition, bounded executable, wire/error mapping and admission port. |
| R2 executable tests | Same crate: `tests/command.rs`, `tests/crash.rs`, `tests/support/mod.rs` | Actual subprocess driver, fixture admission owner, barriers/fault cuts and bounded child cleanup. |
| Narrow R1 interface completion, assigned to R2 | `rust/crates/review-episode-core/src/command.rs`; `rust/crates/review-episode-core/tests/transitions.rs` | Add `BeginCommand::new_utf16` and `TransitionCommand::new_utf16` accepting `JsString`; keep existing `new(&str, …)` as delegating compatibility constructors. Preserve validation and reducer semantics. Add surrogate command-ID cases. |
| R2 qualification record | `docs/rust-review-episode-r2-result.md` | Exact candidate/build/configuration, checks, cuts, limits and handoff; does not edit the historical R0/R1 records. |
| Shared integration owner only | `rust/Cargo.toml`, `rust/Cargo.lock` | Register the two new members and resolve the agreed dependency delta in a serialized window. These are requests, not R2-builder-owned paths. |

Core remains independent of storage and Serde. Store depends on core,
`rusqlite = "=0.37.0"` with `bundled`, and workspace `thiserror`; `sha2` uses
the existing workspace pin for copy-file reports. Application depends on core,
store and workspace `thiserror`; it uses the core JSON representation for
episode-bearing wire data. Use `tempfile = "=3.27.0"` only in tests. Request
`rustix = "=1.1.5"` with `fs` for safe no-follow opening/metadata mechanics;
coordinate its feature union with C2 rather than editing the lock twice. No
Tokio, Serde conversion of domain payloads, SQLite pool, shared supervisor,
new common crate, public wire/client family or lifecycle dependency is needed.
Dependency pins are proposed inputs; their qualification belongs to the exact
implementation lock, including bundled SQLite and platform features.

At planning time S1 owns the shared lock (board 1212), has an incomplete,
unreviewed implementation, and explicitly promises no reusable persistence
owner. Its manifest already proposes rusqlite 0.37.0/bundled and tempfile
3.27.0; those are useful pin observations, not an accepted shared API. R2
therefore owns narrow private-file opening and transaction mechanics. Reuse
could later cover regular-file/no-follow checks or connection policy plumbing
if an accepted owner and both actual consumers agree a small interface. Schema,
durability, validation and transaction/CAS policy remain domain-owned. Extracting
that helper is neither R2 work nor a prerequisite.

After S1's handoff, the integration owner records the new starting root hashes,
adds exactly `review-episode-store` and `review-episode`, and resolves only the
agreed dependency delta. Preserve the ten existing members, UI exclusion,
toolchain, profiles and historical foundation gate. No speculative C3/R3
registration is included. Package validation can then use the frozen lock;
whole-workspace qualification uses a serialized stable candidate with S1/C2
changes explicitly included or excluded by their owners. Do not report a
workspace gate while its inputs are changing underneath it.

## Domain encoding and the small core interface addition

R1's actual representation is `JsValue` with UTF-16 `JsString` keys/values,
its own parser, `ryu-js` number formatting and domain-local
`review-episode-js-json-v1`. It is not a Serde JSON model. Carry this AST from
wire parsing through admission comparison, command construction, reducer,
storage and response. Use checked Rust-string conversion only for bounded
ASCII protocol controls, filesystem configuration and already-ASCII digests.
Do not stringify/reparse domain values through `serde_json::Value`, replace
surrogates, change missing/null behavior, reorder using Unicode scalar values,
append a digest newline, or substitute the shared JCS codec.

The complete current `command.rs` exposes only `&str` constructors despite
keeping `transition_id: JsString` internally. The additive constructors above
close that representational gap: existing callers retain their API, while
the executable can supply an unpaired-surrogate transition ID without loss.
Tests cover both constructors producing identical valid scalar commands,
nonempty/whitespace validation, and distinct surrogate IDs/digests followed
by restart and exact replay. No accepted R0 semantic choice is reopened.

Historical `state_json` remains its exact canonical UTF-8 byte string, including
escaped lone surrogates. Use `ReviewEpisodeState::from_stored_json`, compare
the row revision and identity key to validated state, and retain original bytes.
The R1 nesting limit of 256 is retained: an unsupported row is refused with
source bytes preserved, never normalized or silently skipped. State revision
continues to exclude the `revision` field; stored state includes it.

## SQLite ownership, schema and integrity

Use the existing two STRICT tables unchanged:

```sql
CREATE TABLE review_episode_current (
  identity_key TEXT PRIMARY KEY,
  revision TEXT NOT NULL UNIQUE CHECK(length(revision)=64),
  state_json TEXT NOT NULL
) STRICT;
CREATE TABLE review_episode_history (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  identity_key TEXT NOT NULL,
  revision TEXT NOT NULL UNIQUE CHECK(length(revision)=64),
  predecessor_revision TEXT,
  state_json TEXT NOT NULL
) STRICT;
```

Keep legacy `user_version` and `sqlite_sequence` behavior; no R2 side tables,
request log, schema upgrade or rewrite of valid historical rows. Validate table
shape, strictness, uniqueness and absence of unexpected executable schema
objects before use. Unknown/incompatible schema fails explicitly. Initialization
creates both tables in one transaction only in a newly admitted offline root;
opening an existing store does not repair missing tables or run migrations.

The store owns WAL, FULL synchronous mode, `trusted_schema=OFF`, bound SQL
parameters and a 5,000 ms default busy timeout. Startup options may lower the
timeout for controlled tests; request payloads cannot change it. Check the
effective settings. The R2 supported security/filesystem profile is local Unix
storage in an owner-controlled private directory. Create directories as 0700
and database/sidecars as 0600; reject symlinks, nonregular files, unexpected
ownership and multiply linked database files. Use no-follow creation/opening,
keep the initial file handle and compare device/inode before and after the
SQLite open. Validate existing WAL/SHM entries as well. Never chmod a path
before verifying the object being changed. Unsupported platforms/filesystems
fail with an explicit profile error rather than claiming these guarantees.

These checks constrain accidental path substitution and other-user access.
They do not exclude hostile processes under the same UID: the trusted parent
directory and launcher boundary remain prerequisites. R2 does not claim a
race-proof sandbox from pathname checks or 0600 permissions.

Reads use a consistent SQLite read transaction. For the selected episode,
validate every history row's exact canonical bytes, revision and identity;
require one root with null predecessor, ascending sequence, each later
predecessor equal to the preceding episode revision, and current equal to the
last history row in both revision and raw JSON. Global sequence gaps and
interleaving between episodes are valid. Reject orphan current/history rows,
forked/missing predecessors and a mismatch between lookup key and embedded
identity. A malformed row is an integrity failure, not `not_found`.

Across adjacent snapshots require retained transition IDs to retain their
digests and exactly one new handled transition for an appended revision;
the root contains its begin transition. This supports response reconciliation
without adding a request ledger. Do not replay the new reducer to “prove” old
history: old question/succession semantics remain valid historical bytes.
Because command payloads are not stored, digest-chain integrity is not an
independent proof of historical caller authority or lawful execution.

`recover`, current `read` and exact-revision `read` agree on one validated
history. `history` returns the complete ordered array or an explicit bound
error. Validation can stream rows; no implicit limit or silently truncated
success is permitted. The offline full-store validator additionally checks
all identities, SQLite integrity, global sequence uniqueness and the
autoincrement high-water mark.

## Transaction, admission and replay composition

The application consumes a configured store handle and a trusted
`HostAdmissionPort`; neither comes from command JSON. Its internal
`AuthorizedCommand` and `AdmittedResult` have private construction, no
deserialization implementation and no boolean bypass. A port implementation
is trusted executable code, not something a request selects. Public DTOs and
R1 `Authority` values are data; possession of them is not a host grant.

The port receives the exact request and observed episode revision and resolves
the configured authority/admission source. The application checks a complete
binding: operation and request digest, episode identity, selection revision,
authority manifest/source digest, writer actor/provider/generation/session,
requested expected revision, selected subject digest, transition ID/content
digest, and result digest/evidence references when applicable. A separately
authorized read grant can inspect immutable history independently of writer
generation. Admission checks apply before disclosure, including replay reads.

External/host validation occurs before taking the SQLite write lock. Its
private permit is bound to the observed current revision (or observed absence
for begin). Inside `BEGIN IMMEDIATE`, load and validate the current/history
again and compare that observation; if it changed, return revision conflict
and require fresh admission. No external claim lookup or provider call occurs
while holding the SQLite transaction.

Invoke R1 `begin` or `transition` on that transaction's current state. R1 owns
the requested expected-revision and current-writer rules, including replay
ordering. Do not reject a current writer's exact replay merely because its
original expected revision is old. `Replay` returns the latest validated
state, marks `replayed=true` and writes no current/history row. Conflicting
reuse fails; a replaced writer's command replay fails even when bytes match.
Read authorization is a separate route, not a way to execute a fenced write.

For `Applied`, precompute and validate canonical state and bounded response,
insert history with the actual predecessor revision, and perform an explicit
conditional current update using that same revision (begin uses absence).
Require exactly the expected row effect; then commit once. All store writes
remain within this transaction. A failed reducer, admission, CAS or statement
rolls back without a new state. Distinguish SQLite lock contention from domain
revision conflict. A commit/rollback I/O failure that cannot establish the
outcome is reported as outcome-unknown and closes the connection.

`resumeInitial` and `validateResult` use admitted consistent snapshots and the
R1 validators, without publication. Validation results are observational and
cannot be reused to bypass a later result command's admission or transaction
CAS. The six transition actions and all R1 selected deltas remain unchanged.

### R2 offline port versus R3 production trust

R2 implements a fixture admission port for disposable offline roots. A trusted
test/operator launch supplies a read-only fixture registry outside request
JSON; the registry maps a grant ID to the exact request/authority/evidence
binding. Missing grant, changed field, wrong root, mismatched digest or a
caller-invented status/ref fails. The fixture may intentionally describe
established/false/unestablished evidence to exercise deterministic behavior;
responses and qualification artifacts label the source `offline_fixture`.
That label is not stored into or used to change historical episode hashes.

The executable requires an explicit offline launch profile and an initialized
offline root; it has no enabled production admission implementation in R2.
`init-offline` accepts only a new/empty private directory and creates its
offline marker and database. Normal requests cannot supply a database path,
fixture registry path, executable selector or admission mode. An offline
marker/fixture file is an administrative guard against accidental use, not
authentication against someone able to alter same-UID files. R2 never reports
the synthetic port as real claims-owner verification or reviewer isolation.

R3 must supply the native host-bound port, establish the real launcher/file
capability boundary, resolve exact claims-owner records, and handle prepared
correction successors without presenting them as committed evidence. A
serialized `authorized: true`, status string, private Rust type or arbitrary
CLI invocation cannot substitute for that contract. No production activation
is reachable merely by removing the offline flag.

## Executable protocol and recovery

Use one length-prefixed UTF-8 JSON request and one response per process: a
four-byte unsigned big-endian byte length, followed by exactly that many
bytes, then input EOF. Version 1 identifies domain `review-episode`; a bounded
ASCII `requestId` correlates transport only. Operations are `begin`,
`resumeInitial`, `transition`, `validateResult`, `recover`, `read` and `history`.
Arguments retain the legacy operation shapes; transition IDs remain domain
UTF-16 values. Reject unsupported versions/operations, extra envelope fields,
trailing frames/bytes and incomplete frames before mutation. No shell launch.

Default request/response limits are 8 MiB/32 MiB, checked as byte counts;
startup configuration can tighten them. Reject malformed/oversized input
before database mutation, and known oversize success responses before commit.
For history, report an explicit `ResponseTooLarge` rather than truncating.
The CLI closes after one response; stdout contains only the frame, stderr
contains bounded redacted diagnostics. Test driver timeouts bound the entire
child and drain/kill/reap it. These limits are an offline operating profile,
not an accepted production host responsiveness budget; R3 measures and binds
its own compatible limits before integration.

Successful replies identify observed current revision and `applied`, `replay`
or `observed`, with the exact state/result requested. Typed errors retain R1
`ResultContract`, `Authority`, `RevisionConflict`, `InvalidCommand` and
`StateIntegrity`, augmented with framing, admission, path/schema, busy,
resource-bound, I/O and outcome-unknown categories. Keep domain rejection
separate from process/transport failure for R3's result-contract mapping.
Do not derive certainty from exit code: a killed process, broken pipe,
partial/malformed reply or timeout after launch can follow a commit.

Recovery is an authorized read through a new process/connection. Given the
identity, transition ID and expected content digest, validate complete history
and locate the first row that introduced that ID. If its digest matches,
report the committed revision plus latest state; this is not a promise to
reproduce the original response. Different content is an ID conflict. Absence
in a successfully validated store means no such durable transition at that
read point; retry still needs current admission/CAS. A replaced writer may
need the independently authorized reader, because command replay stays fenced.
If storage/read authorization is unavailable or corrupt, outcome stays unknown.
Never infer provider non-entry or repair this condition by replaying a provider,
switching writers, rolling back committed state or falling back to JavaScript.

## Copy import boundary

R2 provides a source-copy validator/importer, exercised only on generated
offline databases and immutable supplied copy fixtures. It does not discover
state roots, snapshot a running database or open live stores. The source is a
self-contained, closed SQLite copy with no WAL/SHM sidecars and an explicit
copy manifest identifying path, file digest and declared provenance. Reject
symlink/nonregular sources, source/destination aliases, unresolved sidecars,
changed source digest and a destination that already contains an episode DB.
The source is opened read-only; its digest is checked before and after.
Manifest declarations do not independently establish source custody.

Validate the entire source before publication using the schema, row, chain,
current/history and SQLite integrity rules above. Create the destination in a
new offline root and import all rows in one transaction, preserving raw
`state_json`, identity keys, revisions, predecessors, explicit sequence values
and `sqlite_sequence` high-water mark. Never recompute old revisions, collapse
sequence gaps or partially accept an episode. Failed import leaves no usable
published destination; retain the source and diagnostic rejection report.
After commit, close/checkpoint, reopen and compare every source/destination
row and counter before declaring the copy accepted. Publication/response loss
is reconciled by destination validation rather than blindly importing again.

Report source/destination file digests, schema profile, row/episode counts,
rejected locations/reasons and byte-comparison outcome. Whole SQLite files
may differ physically despite identical rows; do not demand equal file hashes
or infer semantic equivalence from counts. External reference existence,
claim authenticity/applicability and lawful historical authorization remain
unestablished by import. R4 still owns selected real-copy qualification and
R5 live cutover/rollback. No new live-import authority follows from this CLI.

## Executable validation matrix

Use the real binary and temporary local SQLite files. The test driver is
Rust. Where cross-language parity is required, it invokes Node's existing JS
episode service/store on disposable copies as a transitional oracle; do not
add a JavaScript reducer or permanent test framework. Frozen R0 vectors remain
unchanged and are inputs, including v1/v2 and selected-delta cases.

| Cases / owned test | Required observable result |
| --- | --- |
| `command.rs`: begin/result/reopen, then every supported operation/action | Real subprocesses produce exact R1 canonical states/revisions; exact read, recover and history agree; invalid result is `ResultContract`, never a store/transport error. |
| Core `transitions.rs` plus executable `command.rs`: supplementary and lone-surrogate IDs/strings/keys, null versus omission, safe-number/nesting edges | Exact UTF-16 bytes and hashes survive command construction, persistence and reply. Unsupported depth is explicit refusal with source preserved. |
| Store `compatibility.rs`: JS-produced v1/v2 rows read/imported by Rust; Rust-produced rows read by legacy JS | Raw state bytes/revisions/history agree in compatibility cases. R0 deliberate semantic deltas use the accepted target oracle and are not falsely reported as old behavior parity. |
| `cas.rs`/`crash.rs`: two separately spawned processes begin the same identity; different commands race on one expected revision | One applied write, one classified conflict; one current row, one appended successor/root. Barriers ensure both processes reached the intended contention point. |
| Same command raced by two processes; replay after further transitions; conflicting same-ID payload | Exactly one append; the other may first receive admission-snapshot conflict/busy and, after fresh admission, returns replay. Replay returns latest state without append; conflicting content fails. |
| Replacement race versus old writer; old-writer replay; independent read | Winning replacement fences old writes/replay; an admitted reader can still inspect history. No inferred fresh independence. |
| `recovery.rs`: wrong row key/revision, invalid JSON/canonical bytes, missing/forked predecessor, orphan rows, tampered current, changed handled digest, unknown schema/triggers | Explicit integrity/schema failure with no write or automatic repair. Valid global sequence gaps/interleaving remain accepted. |
| Busy holder, malformed/oversized/truncated frame, extra frame, response bound, wrong/missing grant and binding mutations | Bounded classified failure, no mutation; malformed transport never constructs a permit. Compile-fail doctests in `admission.rs` prove private permit/result construction. |
| `crash.rs`: terminate after admission, after BEGIN, after history insert, after current update, immediately before COMMIT | Reopen through a new process: either unchanged/absent state as appropriate, no partial current/history; retry with fresh admission appends once. |
| `crash.rs`: terminate immediately after COMMIT, before reply, or after partial reply; close output pipe | Reopen locates exactly one matching committed transition; lost response remains ambiguous until readback, and exact retry adds no row. |
| `recovery.rs`/`crash.rs`: statement failure after history insert, commit failure with unavailable readback | Ordinary rollback preserves prior state; ambiguous I/O reports unknown, not “not committed.” Injected fault scope is stated; it is not a physical-media proof. |
| `compatibility.rs`/`recovery.rs`: all-or-nothing import, preserved high-water mark, crash before/after import commit, invalid source, source alias/sidecars/digest change, second import | Valid copy is byte-preserving and restart-readable; rejected copy never becomes a usable partial destination; retry reconciles committed destination before acting. |
| `command.rs`: established/false/unestablished fixture evidence, later result without new applicable evidence, question-preserving succession | R1 deterministic statuses and lineage survive storage. Artifact explicitly says fixture evidence; no production claim or cross-store settlement is asserted. |
| Default build/configuration checks | Offline profile is explicit, production admission unavailable, no provider capability, no fallback writer, no fault controls in the default/release executable. |

Crash tests use named checkpoint barriers and a parent-observed ready signal,
then kill the actual child; sleeps alone do not locate a cut. Internal fault
controls are behind an explicit nondefault `test-faults` feature in the two
new crates, propagated by the application. They can observe/block transaction
cuts without replacing store/reducer logic. The default binary rejects fault
configuration. Test both the default executable and fault build; record their
different artifact identities. OS process-kill recovery proves process-loss
behavior, not power-loss durability on arbitrary storage.

## Delivery sequence and bounded exit

1. Integrate the two manifests through the shared owner, add the UTF-16
   constructor completion, and build the smallest production-code vertical:
   offline grant → begin/result transaction → exit/reopen → exact read. Use
   the actual executable immediately; avoid finishing an abstract store before
   demonstrating its composition.
2. Complete all command/read routes, history validation and byte-compatible
   copy import. Keep source/target semantic-delta expectations separate.
3. Add the actual-process CAS/crash matrix and response reconciliation,
   admission-negative and resource-bound cases. Qualify the composed result
   against the exact lock/build/profile; retain builder ownership for fixes.

These are delivery defaults inside the bounded outcome, not a second semantic
contract. Future focused commands, after implementation authorization and the
serialized dependency handoff:

```sh
cargo +1.92.0 test --manifest-path rust/Cargo.toml --locked -p review-episode-core -p review-episode-store -p review-episode
cargo +1.92.0 test --manifest-path rust/Cargo.toml --locked -p review-episode --features test-faults --test crash
cargo +1.92.0 fmt --manifest-path rust/Cargo.toml -p review-episode-core -p review-episode-store -p review-episode -- --check
cargo +1.92.0 clippy --manifest-path rust/Cargo.toml --locked -p review-episode-core -p review-episode-store -p review-episode --all-targets --all-features -- -D warnings
cargo +1.92.0 build --manifest-path rust/Cargo.toml --locked --release -p review-episode --bin work-engine-review-episode
```

The integration owner then applies the established workspace formatting,
Clippy/tests/release, dependency/feature/license/advisory and UI-isolation
checks to a stable exact candidate. Record Node/toolchain/SQLite versions,
source and binary hashes, default versus fault features, executed test/cut
counts, requested/observed model identity and unavailable measurements.
Separate initially isolated review, if selected under the owning contract,
cannot be supplied by relabeling this planner; same-provider review makes no
cross-provider independence claim.

R2 exits when the real offline vertical and the full bounded matrix pass,
no unresolved material store/application finding remains, the exact shared
integration gate is recorded, and the result states the remaining R3/RC/R4/R5
boundaries. No production-host qualification, latency budget, three-store
settlement or cutover is needed to declare this bounded exit; none may be
claimed from it. Root handoff timing is the only current coordination
dependency. Production admission and real-copy/cutover authority are already
assigned later-owner obligations, not unanswered R0 design choices.

## Planning evidence and limits

Required DESIGN/PHILOSOPHY and current Rust/model guidance were read. Repository
retrieval used Verify tier, project `home-bline-code-work-engine`, ready status;
the coverage generation was `2026-10-07T17:05:54Z`, recorded `17:05:55Z`.
Scoped service/store/core searches and relevant depth-one traces in both
directions completed without remaining pages. Dynamic JS store calls and
ambiguous Rust name-based edges were resolved by direct source, not treated
as complete call-graph evidence. In particular, spurious cross-language
`clone`/`Replay` matches establish no Rust dependency.

Exact service/store/core files, manifests and governing documents received
coverage checks: no recorded gaps, metadata match except the changing root
lock's `not_tracked` freshness, whose relevant dependency entries were read
directly. Full `command.rs` was read for the missing UTF-16 constructor claim.
Coverage is best effort, not proof of completeness. No S1 implementation source
or SQL is relied on as an accepted API; board 1212 and its manifest suffice
for the coordination/dependency observation. No code, test, provider, database,
import, benchmark or live runtime execution was performed for this plan.

| Frozen input | SHA-256 |
| --- | --- |
| Reviewed episode lane plan | `dd17594b61ca7cc70f853b6c92024ffe1472006e4c8ac2530b63df513297515d` |
| R0 contract | `a6a8fe6cc6f714b9b2aaaa6db50d13888cac0cc8088e882f169e50a484d09862` |
| R1 result | `a405df31a83ac7d8b9d6fbff39a1ae127290c4dbc4350d4002797889b08c76dc` |
| C1/R1 combined validation | `4c03bf943e0e9d06fce4ae716aec42444dd1868b27a8081f571b17370df5a7f3` |
| Ten-member root manifest at planning | `6c593393b67b61814f3a47b152fc20191dc974b1200559752e20f2719cf43365` |
| R1 `src/command.rs` before proposed API completion | `704fd1eaedc83a527f1189a1877cbcd66cbf49f4e977d61cd5237073ce1c323e` |
