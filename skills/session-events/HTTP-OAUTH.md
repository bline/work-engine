# Optional HTTP OAuth resource server

The HTTP server remains unauthenticated by default. Enable this only with a trusted launch configuration and a trusted local verifier executable. The existing loopback listener, Host and Origin checks still apply; the tunnel or reverse proxy must forward the Authorization header and discovery requests. Stdio is unaffected.

Build the verifier with `cargo build --locked --release -p mcp-auth` in `rust/`. Provision a JWKS snapshot from the configured issuer's `/.well-known/jwks.json` using a trusted operator-controlled process. The server itself makes no network request. Install the snapshot atomically at an absolute path and refresh it before its configured maximum age; its filesystem modification time is the freshness clock. Unknown keys, stale, future-dated, malformed, oversized or unreadable snapshots fail closed. Snapshots are read on each protected request, so atomic replacement installs rotations; no live rotation or revocation behavior has been qualified. A removed key stops validating after replacement, but an accepted signed token otherwise remains usable until expiration. Refresh and revocation latency are operator responsibilities.

Example trusted JSON configuration (replace the two local absolute paths):

```json
{
  "resource": "https://mcp-test.devwits.ai/mcp",
  "issuer": "https://checkered-replace-0099.customers.stytch.dev",
  "subject": "user-test-8b1b7b52-0fa4-4b70-8594-b2b043592a1c",
  "write_scope": "mcp:write",
  "jwks_snapshot": "/absolute/operator-managed/stytch-jwks.json",
  "jwks_max_age_seconds": 3600,
  "verifier": "/absolute/trusted/mcp-auth"
}
```

Run `session_events.py serve-http --auth-config /absolute/trusted/auth.json`. Keep configuration, verifier and snapshot protected from untrusted modification. The configuration is loaded at startup; restart to change it. The snapshot may contain at most 32 keys and 256 KiB; a token is limited to 16 KiB. Verification is bounded by a three-second subprocess timeout. The verifier receives token and immutable launch configuration through stdin; tokens are not arguments or request logs. Errors produce only a generic denial. There is no network fetch or unknown-key retry.

The policy requires a signed RS256 JWT with `typ` = `at+jwt`, nonempty `client_id` and `jti`, `iat`, `exp`, issuer, subject, and an audience including the exact configured resource. `nbf`, when present, must be valid and elapsed. The trusted write scope must be present in the space-separated `scope` claim. Key selection uses only the pinned local snapshot; token-provided key URLs are ignored. Missing or invalid authentication returns 401 with protected-resource metadata; a verified token without the write scope returns 403. The known reads and handshake stay public. All other operations, including subscriptions and unknown future methods or tools, require the write scope. This authentication does not replace separate bridge grants or per-decision approvals.

**Provider prerequisite:** these are a strict resource-server policy and synthetic fixtures, not evidence that the Stytch test project currently issues compatible tokens. Its observed access-token template and standard scopes do not establish this audience, access-token type, required claims, or write-scope issuance. Verify and configure those provider properties before enabling the feature; a project-ID-only audience is insufficient. No client registration, deployment, live token, key rotation, or revocation was exercised by these source tests.
