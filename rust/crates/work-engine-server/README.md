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
