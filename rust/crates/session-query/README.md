# Session query client

This temporary local consumer calls the existing session-events MCP `Server.call_tool`. MCP remains a sibling consumer; the owner retains transcript composition, bridge authorization and read reconciliation. This crate does not own an operation journal or submit commands. Native session IDs are session identities, not lifecycle or task IDs.

Create a private mode `0600` JSON profile owned by the invoking user. All paths are absolute and designate the local operator's scope:

```json
{
  "version": 1,
  "owner": "/absolute/path/to/session_events.py",
  "owner_sha256": "470a625b76d17a62a68659709d99d2767c944441be4cd2ddbff96bc6be12a12d",
  "codex_home": "/absolute/path/to/local/codex",
  "claude_home": "/absolute/path/to/local/claude",
  "state_dir": "/absolute/path/to/private/session-events-state",
  "bridge": null,
  "context": null
}
```

Granted reads require `bridge` to be an object containing absolute `binary` and `config` paths and their lowercase SHA-256 pins `binary_sha256` and `config_sha256`. An optional `context` object accepts absolute `budget_root`, `role_ledger`, and paired `ledger_path`/`ledger_sha256` fields (use null for omitted fields), passed to the owner’s existing context builder. Without it, those observations remain unavailable. The existing owner validates the bridge and its private configuration. The profile supplies no remote grant. The client embeds its helper and executes a single bounded, digest-verified snapshot of the compositor; it does not reopen that snapshot by pathname.

Run `work-engine-query profile.json list_sessions --json -` and provide `{}` on stdin. The four supported names are `list_sessions`, `read_session`, `read_session_operation`, and `read_session_result`. Arguments are a JSON object on stdin (`-`) or in an owner-only regular file; granted reads require `request_id` and `grant_credential`. Never place credential material in argv. Success and failure use the `work-engine.session-query.v1` JSON envelope; `--human` prints a concise view. Errors are sanitized and unsuccessful queries exit nonzero. The raw enriched owner result remains in the JSON success envelope; unavailable observations and ambiguous native state are not converted into successful or zero values.

The test fixture seeds disposable existing bridge records directly; it does not submit a request or contact a provider. Its protocol-responder tests exercise transport behavior but do not stand in for bridge authorization checks. No real user credentials are needed.

Remove or replace this adapter when the accepted A3 session-owned Rust query implementation takes over the compositor and preserves the granted operation and result semantics; this adapter does not authorize that replacement. The bridge's existing own path validation and reconciliation remain its responsibility, including its existing check/use properties.

For `read_session`, `text_offset` and `max_chars` count characters in the original entry. When a window boundary falls inside a recognized credential, the owner expands that boundary to the whole credential and returns a redaction marker. Adjacent windows may therefore repeat the marker; advance using original-entry offsets. `entryChars` is the original length, `textTruncated` reports whether original text remains after the expanded end, and `redactions` counts markers in the returned window. Beyond the end, the result is empty and not truncated. Human output escapes terminal control characters; JSON preserves the owner’s string representation.

The executable is built with `cargo build -p work-engine-query` from `rust/`.
For an explicitly local service, use an owner-only profile file such as
`{"version":1,"kind":"local-service","endpoint":"127.0.0.1:12345","credential_file":"/absolute/private/token","timeout_seconds":10}`.
The numeric endpoint must be loopback, the timeout is 1–60 seconds, and the four reads are available. Granted operation and result reads require
the original request ID and domain grant credential in protected JSON input;
the service credential authenticates transport independently and grants no domain authority.
Reads may reconcile and persist owner state. No submit operation is exposed. The credential stays in its
protected file. There is no fallback to a direct owner. JSON uses
`work-engine.session-query.service.v1`: the complete server `envelope` is
preserved for owner success and owner/server errors, which exit nonzero; local
errors and transport errors are separate fields. The timeout bounds the service
query as a whole, including a peer that continues sending data slowly.
