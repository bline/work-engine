# Offline shared delivery foundation v1

Candidate for Root review; fixture qualification only. This is delivery routing
state, not a supervisor, task/grant ledger, execution adapter or acceptance owner.
It refines the Root receiver candidate r2 (SHA256
`d22c1a745ab53d240cd202ad5ed204bf69c5aac2bb7903614960b17d5ed10911`)
with the concrete DTO in `schema-v1.json`. S10 has not qualified these bytes.

## Source and transactions

The configured Python adapter imports the existing `codex_chatboard` owner,
uses its `wait_board(..., timeout_seconds=0, limit=500)` cursor semantics, then
reads the exact returned immutable revision through its existing durable store.
Observation belongs to the foreground service, never to recipient agents.
No new board storage or publication semantics are introduced.

Repository identity is the canonical absolute Git common-directory path plus a
trusted configured `source_id`. It is a local locator binding, not a globally
portable repository UUID. A moved/recreated repository needs owner disposition;
a missing/reset board or sequence gap is unavailable and never advances cursor.
Each observation also carries its previously committed source revision. Before
filtering, the adapter reads that exact retained snapshot through the durable
owner, compares every consumed raw row (including reserved-topic rows), and
verifies reachability through the owner's existing predecessor links. Missing,
changed, detached or incompatible history holds the source. Verification is
bounded to128revisions; exceeding that bound is an explicit hold, not an
automatic new baseline. Non-message revisions and append-only snapshots retain
normal cursor semantics. The store transaction compares both saved cursor and
previous revision, so unchanged-cursor observations cannot overwrite a newer
committed source history. The cursor and local repository path alone never
establish continuity.
The adapter requires the retained board snapshot to contain contiguous native
sequences; reserved local control topics retain the board reader's exclusion.
Only exact `to` address matches enqueue deliveries. No implicit broadcast or
role/session authority follows from an address.

`Event.message_id`, original native `sequence`, `observed_at`, source key,
revision and epoch are retained. `event_id` is a namespaced delivery identity
(SHA256 of the Serde JSON tuple `[source_id,message_id]`); it does not replace
the original board identity. `raw_payload` is the original UTF-8 JSON object
slice from that immutable board revision. Its codec is
`json-object-slice-utf8-v1`: from opening `{` through closing `}`, including all
internal whitespace and string escapes, excluding surrounding array whitespace.
`payload_sha256` hashes those exact bytes, without canonicalizing signed text.
Routing parses a copy; it never rewrites the retained slice.

SQLite FULL synchronous transactions atomically update the source cursor,
inbox and per-recipient deliveries. A conflict or invalid row rolls back the
whole observation. The CLI then records a bounded source hold separately.
Exact replay is idempotent. Source/event bytes or native-sequence conflicts fail.
The one-row persisted state is intentionally bounded: 1000 events, 2000
deliveries, 16 configured targets. There is no garbage collection or silent drop;
a full store remains held for owner disposition. Snapshot8MiB, row64KiB,
observation500rows; source child256MiB address space,10CPU-seconds and15s timeout.
No caller supplies source, executable, store or configuration paths through MCP.

## Receiver and observation DTO

`work-engine.delivery-envelope/v1` contains logical `delivery_id`, unique
journaled `attempt_id`, configured `subscription_id`, exact `recipient` Binding,
binding digest, immutable Event, `fixture_only:true`, and
`authority:"observation_only"`. Binding names recipient/principal/job/lane/
session/workspace/generation, address, activation, expiry and fixture behavior.
These concrete names refine the candidate's `_id` field spellings; the complete
binding digest identifies all fields. Original source cursor information is
represented by `Event.sequence` plus `revision`, `source_key` and `epoch`.
Both receiver and ACK independently check exact binding/digest/generation.

Each attempt ID hashes `[delivery_id,attempt_number]`; it is persisted before
possible send. The immutable envelope digest includes the exact attempt ID.
A bounded journal retains each attempt's independent transport/observation
facts and hold. Failed pre-send attempts may retry at most three times with a
new attempt ID. They cannot accept a later ACK as evidence for another attempt.

Only the explicitly configured **isolated fixture child** exists. The service
executes its own binary with the private fixture config over stdin/stdout,
collects its exact exit, and closes it before continuing. The receiver stores
the exact envelope in its own disposable SQLite observations table before ACK.
There is no native/provider/channel/HTTP/Unix-socket adapter, production
qualification constructor, `approved:true`, deployment or live subscription.
The initial Unix-socket experiment failed because the sandbox denies socket
creation; stdio is the concrete fixture transport in this revision.

`work-engine.delivery-observation/v1` identifies exact delivery, attempt, event,
binding digest, envelope digest, payload digest, generation and one named stage.
The receiver receipt is `isolated-session-receipt-v1`. Transport acceptance,
target observation and application/result are distinct. An observed ACK alone
does not invent transport acceptance. Application/result remains null: this
slice has no operation that establishes completed work or effect settlement.

Missing/invalid ACK after possible send remains unknown. Restart never resends
prepared, unknown or transport-accepted work. Busy remains held/unknown;
fixture offline is proven before send and has a bounded retry limit. The oldest
unobserved delivery blocks later sends for that recipient; other recipients
continue independently. Expiry, revocation and binding changes prevent new send.
The service rereads trusted config at final preparation. Binding mutation cannot
silently move prior deliveries; explicit successor migration is unsupported and
requires a separately owned decision. New evidence from an expired/revoked
binding is rejected; an already-recorded exact ACK remains idempotent.

## MCP seam and credential boundary

Existing tool and callback payloads remain compatible. The opt-in tools are
`delivery_subscribe`, `delivery_inspect`, `delivery_ack`. Inspection pages at50
records and returns `next_after` for the next page. It excludes raw payloads,
source paths and configuration. Subscribe chooses a target from trusted startup
configuration and backfills retained matching inbox events atomically.

Set `SESSION_DELIVERY_FIXTURE_ENABLED=1` and all four pins:
`SESSION_DELIVERY_BINARY`, `SESSION_DELIVERY_BINARY_SHA256`,
`SESSION_DELIVERY_CONFIG`, `SESSION_DELIVERY_CONFIG_SHA256`. Absent config leaves
the tools unlisted/disabled; partial pins fail closed. The facade injects its
existing trusted server principal; arguments cannot assert another principal.
It uses the existing bounded helper I/O, with a distinct protocol discriminator.
The new helper resolves and hashes regular owner-owned non-writable files on
open descriptors and executes/passes those same descriptors through Linux
`/proc/self/fd`, avoiding pathname replacement after the pin check. The Rust
`--config-fd` route is RPC-only. Existing Bridge path validation is unchanged.
The inherited OAuth/role middleware still owns authentication. A local fixture
credential or storage access is not human approval or production authority.

## Foundation context compatibility (before the optional producer extension)

The board raw slice retains any nested context data byte-for-byte, including
source session, epoch, sequence, freshness/unavailable quality and model/window.
There is no cross-session sequence flattening or lifecycle order assignment.
The test vector exercises an unavailable context envelope nested in board data.
This is **envelope preservation**, not enrollment of the existing pressure
producer. At foundation acceptance, no concrete producer adapter, active Pi control route
or timeliness proof was demonstrated. The optional producer extension below
adds only its explicitly stated disposable qualification. Actual pressure enrollment and live Pi/Claude/Dot
qualification remain unmet and owned by Root plus the relevant source/runtime
owners. No budget/completion source migration is claimed.

## Optional existing pressure producer

The trusted fixture config optionally pins one `pressure` producer: actual reader
executable SHA-256/path, explicit incarnation, native session/agent/host/rollout,
existing reader levels/step/lead-calls, and exact configured recipient names.
No MCP caller supplies any of these. Existing configs omit this field and retain
board behavior. Service command `observe-pressure` invokes the pinned reader's
`control` command; `run` observes both sources and dispatches independently of
either source's hold. This remains bounded foreground fixture qualification,
not production enrollment. The policy stays entirely with the existing reader;
this crate does not use lifecycle sequences or request compaction.

Pressure reuses the existing Event and envelope v1 DTOs. The variant discriminator
is `Event.schema:manual-context-control-v1`, `kind:context_pressure`, with codec
`reader-output-utf8-v1`. `raw_payload` retains the complete exact UTF-8 reader
stdout, including its nullable event, opaque cursor and final newline. The
existing producer does not supply global event IDs: `message_id` and `event_id`
are delivery-owner identities hashing `[incarnation, binding_digest, cursor]`.
They are not board IDs. `revision` hashes the verified source prefix;
`source_key` identifies the producer. `epoch` is the JSON representation of its
native epoch, including `null`. Native position/observation time, model/window,
freshness, status, unavailable detail, levels and original event bytes remain
in raw_payload. Legacy `sequence` projects source_line (0 only when absent);
legacy `observed_at` projects its string (empty only when absent). Those
sentinels never imply measured counters, timestamp or lifecycle order. Consumers
must branch on Event.schema and use the nullable native fields. Source sequence
is not the delivery scheduler: a durable local insertion order schedules mixed
source events and preserves board ordering on migration. No fabricated board
row, topic, address or sequence is published or inserted.

SQLite's same existing state transaction binds producer config, opaque cursor,
source-prefix length/digest, last raw output, inbox and recipient deliveries.
A quiet initial normal result commits a baseline without generating an alert;
this is observation evidence, not proof of enrollment or timely notification.
The source prefix must remain unchanged and must be stable across the reader
invocation; concurrent growth yields a visible retry-required hold without
advancing the cursor. Truncation/rewrite never adopts a new baseline. Native
appended compaction epochs retain distinct producer epochs and the existing
reader's changed-state event. Binding/policy/incarnation changes cannot reuse
the saved baseline; explicit owner disposition/new store is required. This
bounded profile offers no in-place successor migration. Reader observation
errors/counter gaps remain unavailable exactly as emitted; missing source,
pin, history or process errors retain a durable hold without substituting zero.

The source reader is executed from the same open descriptor whose bytes were
hashed. Environment is cleared; stdout is bounded to128KiB, runtime15seconds,
source history256MiB and executable64MiB. A producer call and any read thread
are collected before return. Filesystem/stdlib/dynamic-library isolation is
not hermetic. Current prefix hashing is linear in source size; this bounded
adapter does not promise lossless recovery of transient pressure between
observations. Recipient delivery uses the common backlog, attempt/unknown and
principal/generation/digest ACK contracts. Pressure status in existing opt-in
MCP inspect exposes binding digest, cursor-committed and hold only, with no raw
rollout/config paths. Real Pi remains blocked as described in
[receiver-candidate.md](receiver-candidate.md); no runtime profile is authorized
by the optional pressure config or fixture_only flag.
