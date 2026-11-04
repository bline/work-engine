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
