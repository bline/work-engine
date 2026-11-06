# Local read-only application server

AH1 composes the pinned `session-query` owner for `list_sessions` and
`read_session`. It owns no transcript meaning or execution authority. It binds a
numeric loopback listener and is intended for one local operator, not browsers,
remote access or multiple installations. MCP stdio and HTTP share the same
handler. The trusted process launching MCP owns access to its stdio.

Pass one absolute path to a regular, owner-owned, mode 0600 JSON configuration:

```json
{"version":1,"listen":"127.0.0.1:49312","mcp":false,
 "query_profile":"/absolute/private/query-profile.json",
 "credential_file":"/absolute/private/operator-token",
 "max_connections":16,"max_queries":4,"request_timeout_seconds":40}
```

The credential file is a private regular file containing 32–4096 printable
ASCII bytes, without a newline. Generate it securely; never put the token in
argv. Query profile format and source pin are defined by `session-query`;
`bridge` must be null and profile paths are trusted startup inputs. The server
checks file ownership and modes, disallows final symlinks, checks the profile
using its owner, and holds exclusive locks on the server config and query profile until drain.
The process clears inherited environment variables before starting threads and uses `/usr/bin/python3` for the existing query helper. Owner absence
or content mismatch leaves liveness intact and readiness false. Readiness checks the owner pin and performs a bounded read-only list probe; it does not guarantee future query success.

`GET /live` is unauthenticated; `GET /ready` requires
`Authorization: Bearer <token>`. The latter returns separate `list_sessions`
and `read_session` booleans. Requests require the exact numeric loopback Host
and actual configured port, reject Origin and do not provide CORS. HTTP supports
`POST /v1/session-queries` with `Content-Type: application/json` and an exact
object:

```json
{"schema_version":1,"operation":"list_sessions","arguments":{},"request_id":"example-1"}
```

The request ID is a nonempty correlation string of at most 128 UTF-8 bytes. It
is not a domain ID. The owner validates operation arguments; extra fields,
duplicate keys, unknown operations and versions are refused. A successful
transport returns HTTP 200 with `schema_version`, `request_id`, `server_identity`
(SHA-256 of exact startup config and query-profile bytes), `server_build`
(SHA-256 of the running executable bytes), and exactly one of `owner_result`,
`owner_error: {"code":...}`, or `error: {"code":...}`. Owner results and
owner-client error codes are preserved. A result timestamp from the owner keeps
its original meaning; this server adds no freshness timestamp. JSON syntax,
HTTP framing and endpoint failures return 400; bad credentials return 401;
request timeouts return 408. Transport errors are distinct from a returned
owner error. Header and request sizes are bounded; chunked transfer is refused.

The `LocalClient` Rust API reads the credential from its protected file, accepts
only loopback endpoints, and returns the complete envelope. MCP uses newline
JSON-RPC and protocol version `2025-06-18`; it implements initialize,
notifications/initialized, tools/list, and tools/call for the two operations.
It emits no events. MCP EOF is a shutdown request. SIGINT/SIGTERM or MCP EOF
stops listener admission and drains accepted HTTP requests and query helpers;
request timeout or disconnect is not evidence that a helper has stopped. The
server waits for outstanding helper permits and reports an unresolved drain
deadline separately; it keeps the config lock while awaiting those helpers.
This covers orderly shutdown, not an abrupt process crash or external effects.
Restart only reopens the same read-only profile and creates no job settlement
or recovery claim.

## Claims-only MCP stdio

```text
work-engine-server claims-stdio /absolute/private/read-profile.json
```

This entry reads the closed `schema_version: 1` repository read profile described
below directly. It opens no HTTP listener and needs no server configuration,
transport credential, session-query profile, command profile or producer grant.
Only the configured repository read tools are advertised. Their MCP annotations
identify local, read-only, nondestructive, idempotent behavior; these hints confer
no authority. Session, command and
publication calls are refused before reaching their owners. Roots, repository
identity and decision scope remain fixed startup inputs.

The profile must be an absolute, owner-owned private regular file, with no final
symlink and within the existing input bound. Each process retains a shared lock
on that profile through drain, allowing independent clients to use the same
immutable profile concurrently. Existing HTTP configuration and profile locks
remain exclusive. Operators control profile bytes and parent-directory custody;
a profile digest identifies loaded bytes and does not attest authority.
`server_identity` in this mode hashes the exact read-profile bytes, and
`server_build` still hashes the executable.

This entry uses the existing newline JSON-RPC parser, claim projection paging,
errors and shutdown drain. Initialize, tools/list and tools/call accept optional
object `_meta` transport metadata. Metadata is never merged into application
arguments and cannot select a root, scope or authority. Unknown request fields,
nonobject metadata and duplicate keys remain refused.
Initialization negotiates the server-supported
`2025-06-18` protocol even when a client offers another version; the client must
support the returned version to continue. The legacy HTTP/MCP entry retains its
exact-version initialization behavior. EOF, SIGINT and SIGTERM request shutdown.
The nonmutating reader refuses missing, invalid or recovery-requiring roots; a
selected scope cannot discover or resolve facts in another scope. It creates no
root, projection or recovery files.

## Repository facts and captured evidence

An optional absolute `repository_claims_profile` in the trusted server
configuration enables native repository-fact reads. This mode composes
`claim-evidence`'s nonmutating `RepositoryReader`; it requires an existing
`repository-claims-v1` root with bootstrap v3. Existing native review roots and
legacy Python claims roots retain their own adapters. The server does not
initialize or migrate a root. Server identity includes the selected read-profile
bytes as well as the existing query and optional command profiles.

The private read-profile JSON contains `schema_version: 1`, `root`, `config`,
`source`, `expected_config_sha256`, `expected_source_sha256`, `repository_id`
and `decision_scope`. These are trusted startup inputs. The same process-access
boundary governs MCP stdio; HTTP also requires the existing transport credential.
Requests cannot supply roots, authority files, publication grants or actor
selection.
The existing `/ready` probes remain session-query checks; each claim read
verifies its own selected canonical inputs.

| Operation | Read consequence |
| --- | --- |
| `discover_repository_facts` | Bounded query and cursor in the configured repository/scope; candidates remain applicability `not_assessed`. |
| `resolve_repository_fact` | Exact claim ID, revision ID and revision SHA-256; includes admitted authority, evidence status, historical/head status and reliance. |
| `read_repository_evidence` | Integrity-checked UTF-8 byte pages of an immutable CBM capture from the selected artifact root. |
| `check_repository_evidence_currency` | Current equality/change/unavailability of only the capture's source anchors. |

Discovery and resolution return
`{encoding: "claim-evidence-canonical-json-v1", canonical_projection: "..."}`
inside `owner_result`. The string is the complete canonical owner projection,
including its provenance and limitations. It preserves native UTF-16 JSON
strings without coercion through Rust's Unicode scalar string model.
Projections above 32768 UTF-8 bytes return `canonical_projection_page` instead
of `canonical_projection`, plus `projection_sha256`, `offset`, `total_bytes`,
`has_more` and `next_offset`. Both fact operations accept an optional
`projection_page: {offset, max_bytes, sha256}`. Reissuing the same query/cursor
or exact locator with the returned next offset and full projection digest
reconstructs the complete canonical bytes; page sizes are 4–32768 bytes. A
nonzero offset requires the digest. Canonical state changes, including new
reliance, reject stale continuation as `claim_projection_changed`, allowing the
consumer to restart from offset zero. This transport paging also covers large
admitted facts and later reliance growth without removing provenance fields.
Projection currency is relative to canonical claim state; it does not establish
current source truth or consumer applicability.

The evidence operations are available only when the read profile also contains
`evidence: {artifacts, repository, project}` with canonical absolute artifact and
checkout paths and the selected CBM project. Evidence reads take a capture
SHA-256, byte offset and bounded page size, with advancing offsets and the full
artifact digest. The currency operation does not modify a fact or decide whether
its proposition still applies.
Historical artifact reads use stable repository identity and selected custody;
the current project label does not exclude an older capture. Each historical
capture retains its own checked project/root/publication binding, and currency
compares its relative source anchors with the selected current checkout.

Publication, manual successor revisions and exact-revision reliance use the
separate out-of-band `work-engine-claims-host`, with externally provisioned
immutable startup/grants and exact-operation admission. They are not MCP tools.
An admitted producer's named-consumer reliance is attributable; it does not
authenticate direct consumer attestation. Source changes and CBM documentation
references supply evidence/review candidates and do not automatically refresh,
invalidate, accept or reopen claims. Context/budget pressure remains separately
owned and may carry these exact references without becoming a claims store.

## Granted commands (offline synthetic profile)

Commands are disabled unless the trusted server configuration supplies both
`"command_capability":"offline-synthetic-v1"` and an absolute
`"command_profile":"/absolute/private/command-profile.json"`. The separate
profile uses the session-query profile schema with a pinned bridge binary and
configuration. The operator must control these inputs, the private bridge store,
and the synthetic native socket. The capability name declares the selected test
profile; it does not verify that an arbitrary socket is synthetic. Live, Pi and
provisioning profiles are not qualified here. The query profile remains
`bridge: null`. Server identity includes the command profile bytes.

The existing HTTP envelope and MCP tools expose `submit_session_request`,
`read_session_operation`, and `read_session_result` when enabled. Submit takes
`request_id`, `grant_credential`, `session_id`, `workspace`, and `input_text`.
Reads take the original `request_id` and `grant_credential`; result also accepts
`text_offset` and `max_chars`. Reads may reconcile owner state. Keep those exact
references across restart; there is no server-side pending-request scan. An
uncertain entry or a transport timeout is not cancellation or settlement. The
bridge owns admission, grants, durable identity, and reconciliation. The server
credential authenticates a local transport caller; it does not issue or confer a
domain grant. An unrelated native client is not fenced by the existing dedicated
session profile.

Rust callers use `LocalClient::command(operation, arguments, correlation_id)`;
`LocalClient::query` retains its read-only API. For example, supply the grant
and input over a protected channel rather than command arguments:

```rust,ignore
let outcome = client.command("read_session_operation", serde_json::json!({
    "request_id": request_id, "grant_credential": grant_credential
}), "local-correlation")?;
```

MCP stdio is trusted process access, and HTTP still requires the separate local
server credential. Helpers receive request material through stdin. The bridge
checks its configured binary and config pins on use; its existing executable
check/use race is not a hostile-mutator guarantee. Treat the referenced files
and their parent directories as privately controlled. Result enrichment is from
the session-events owner; unavailable completion, workers, usage or closure
remain unavailable rather than inferred from an admission receipt.

## Synthetic task-answer owner (tasks-only checkpoint)

A trusted startup config may set `task_profile` (absolute private file) and
`task_capability: "synthetic-owner-v1"`, with no `query_profile`. The profile
selects the database, repository, pinned board program, key policy and actor for
the accepted `session_task_admission::owner_boundary::SyntheticOwner`; callers
can never select any of these. HTTP (`POST /v1/session-queries`), MCP tools and
`LocalClient::task` share one dispatcher with three operations:

- `project_owner_task` `{task_id}` returns the owner projection.
- `consume_owner_answer` `{selection, signed}` durably consumes an exact signed
  answer; exact retry returns the identical owner decision.
- `read_owner_answer` `{selection, signed}` is the immutable reconciliation read;
  absence is `owner_unknown`, never a no-effect proof. Consume is never retried
  automatically.

Owner errors surface as `owner_unavailable|owner_invalid|owner_conflict|owner_held|owner_unknown`.
Synthetic fixed policy and cooperative local Linux only; transport authentication
never creates approval. Evidence: `tests/task_owner.rs`.

### Task capability limits and unsupported combinations

- `task_profile` with `command_profile`/`command_capability` or
  `coding_job_profile`/`coding_job_capability` is rejected at startup
  (`invalid_config`). These capabilities carry native-session or coding-job submission
  authority; the task capability is a synthetic, fixed-policy owner. Composing
  them in one process has no accepted authority model, so it is refused rather
  than silently combined. Existing command configurations (without a task
  profile) and session-query configurations are unchanged; a query profile may
  be combined with a task profile and was tested (`tests/task_owner.rs`).
- Without a task profile the task operations are disabled: HTTP returns
  `error.code = invalid_request`, MCP `tools/list` does not advertise them.
- Task readiness (`GET /ready`, authenticated; `/live` stays liveness only). The
  accepted owner has no nonmutating availability probe (`project` and `consume`
  write; `read_exact` needs a real signed selection), and this transport does
  not invent one. A constructed owner therefore proves only that the profile
  parsed and the owner opened, so `/ready` never reports the task capability as
  ready: it returns `task_owner: false` plus
  `task_owner_readiness: {state, reason, capability}`. `state` is `unavailable`
  with `reason: board_executable_missing` when the profile's `board.python.path`
  is not a file, otherwise `unestablished` with
  `reason: no_nonmutating_owner_probe`. The file-presence check refutes only; it
  does not verify the pin, script, dependencies, database or repository, so
  `unestablished` also covers other possible failures. Real operations report
  their own attributed `owner_*` errors (e.g. `owner_unavailable`). With a query
  profile, `/ready` returns the query booleans (`list_sessions`, `read_session`)
  and the task fields together; a healthy query probe does not stand in for the
  task owner. Configurations without a task profile keep their prior `/ready`
  bodies. No live readiness claim is made. Evidence: `tests/task_owner.rs`.
- Task requests share the existing connection, body, output and
  `max_queries` limits, duplicate-key rejection and envelope identity. A single
  mutex serialises owner calls inside the process; concurrent exact retries
  return one decision, only one of which is `historical:false`.
- `consume_owner_answer` is never retried by the server. A consume that ends
  `owner_unknown`/`owner_unavailable`, or whose connection dies, is reconciled
  with `read_owner_answer` on the identical input.
- Synthetic fixed policy and cooperative local Linux only. Transport
  authentication never creates approval: the owner re-verifies the signature,
  deadlines, revocation and replay on every consume.

## Coding jobs (offline fixture profile)

With `"coding_job_profile":"/absolute/private/job-profile.json"` and
`"coding_job_capability":"coding-job-offline-fixture-v1"` the server also serves
`submit_coding_job`, `read_coding_job_operation`, `read_coding_job_result` and
`cancel_coding_job` (HTTP envelope, MCP tools, `LocalClient::job`, and
`work-engine-command PROFILE OPERATION --json|--human ARGS|-`). This capability is
distinct from the session-message `offline-synthetic-v1` command profile: separate
config fields, a separate job credential pinned by hash in the operator profile,
and no session grant or bearer authorizes a job. The Pi Host remains the owner of
admission, effects, custody and receipts. See the
[coding-job operating guide](../../../docs/coding-job-operating-guide.md) and
[schema](schemas/coding-job-v1.json). Deterministic qualification uses fixtures.

A separate opt-in `coding-job-local-live-v1` capability requires a version 2
`local-live` job profile and the pinned `coding-qualification` producing binary.
It uses the existing lifecycle-live Host profile and canonical public Job DTO;
version 1 offline profiles retain provider refusal. The config capability must
match the profile. See the operating guide for exact pins, limits, the producer
facade and the separate live admission/acceptance obligations.

The Rust `coding-job-prepare` operator entry prepares fresh exact bundles from
explicit task/workspace/validator/limits and separately trusted authority inputs.
Its [operating reference](docs/coding-preparation.md) covers provider-disabled
submission, separately versioned Sol/high preparation, compatibility and the
limits of collector, workspace and container evidence. Preparation executes no
inference and grants no provider permission.
