# Session query client

This temporary local consumer calls the existing session-events MCP `Server.call_tool`. MCP remains a sibling consumer; the owner retains transcript composition, bridge authorization and read reconciliation. This crate does not own an operation journal or submit commands. Native session IDs are session identities, not lifecycle or task IDs.

Create a private mode `0600` JSON profile owned by the invoking user. All paths are absolute and designate the local operator's scope:

```json
{
  "version": 1,
  "owner": "/absolute/path/to/session_events.py",
  "owner_sha256": "5095b4b7c6ed3ab23a92b6763253c45409a3abbdbcb0223e3a0a93feac736739",
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
