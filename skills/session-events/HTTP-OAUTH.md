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

The policy requires a signed RS256 JWT with `typ` = `at+jwt`, nonempty `client_id` and `jti`, `iat`, `exp`, issuer, subject, and an audience including the exact configured resource. `nbf`, when present, must be valid and elapsed. The trusted write scope must be present in the space-separated `scope` claim. Key selection uses only the pinned local snapshot; token-provided key URLs are ignored. Missing or invalid authentication returns 401 with protected-resource metadata; a verified token without the write scope returns 403. The handshake, discovery, delivery status and event replay stay public. Session reads and budget lookups update local caches and require authorization. Bridge operation/result/target reads require authorization because they can reconcile state or update custody health. All other operations, including subscriptions and unknown future methods or tools, require the write scope. This authentication does not replace separate bridge grants or per-decision approvals.

**Provider prerequisite:** these are a strict resource-server policy and synthetic fixtures, not evidence that the Stytch test project currently issues compatible tokens. Its observed access-token template and standard scopes do not establish this audience, access-token type, required claims, or write-scope issuance. Verify and configure those provider properties before enabling the feature; a project-ID-only audience is insufficient. No client registration, deployment, live token, key rotation, or revocation was exercised by these source tests.

Protected-resource discovery publishes the configured write scope through `scopes_supported`. The resource-specific discovery URL inserts `/.well-known/oauth-protected-resource` before the entire configured resource path; the root discovery alias is also served. The configured path must end in `/mcp`, and its listener path must match. A verified token missing the write scope receives an `insufficient_scope` challenge naming that scope.

## Stytch Connected Apps profile (opt-in)

The omitted `profile` retains the existing `rfc9068-v1` policy requiring `typ: at+jwt`. For the explicitly configured Stytch Connected Apps access-token issuance contract, set all three additional trusted fields:

```json
"profile": "stytch-connected-apps-v1",
"expected_client_id": "connected-app-test-7607c19e-b18a-4ef5-b675-480c2b7457c5",
"expected_project_id": "project-test-5f16c2cc-3020-4a9a-93af-b3a0f22aa9bd"
```

This profile requires `typ: JWT`, RS256 and the same issuer, owner, key, time and scope checks. It additionally pins the client ID and requires the exact project and resource as audience members. Unknown profiles and missing bindings fail closed. The resource audience must be issued by the provider for this client; a project-only audience cannot pass. The configured project, issuer and client must be the intended Connected Apps issuer and grant, and the keys must be provisioned from that issuer.

These checks assume the issuer reserves the bound client/resource claim combination for Connected Apps access tokens. A different JWT type signed with the same issuer keys and carrying identical claims is not cryptographically distinguishable by this profile; do not enable it unless provider issuance and token-type separation establish that assumption. A session or ID token lacking the bound client, project/resource audiences, or required access-token claims is rejected, but merely labeling a token an ID or session token cannot make otherwise identical bytes distinguishable. The authenticated identity does not grant separate owner decision approval. Actual provider issuance and write-scope availability require independent verification.
