# B2 public host (offline candidate)

The public model catalog has a 2 MiB response-body ceiling and a five-second
request deadline, with at most 512 parsed model entries. Retained catalog bytes
use the same 2 MiB ceiling during later model selection. A larger or incomplete
response produces no discovery observation; the single-use action remains spent.
The saved observation metadata has a separate 256 KiB ceiling, enforced before
saving either the raw response or observation and again during selection.

This small executable composes the bounded-inference auth store, public HTTPS policy, request builder, stream observer, and attempt journal. It is an offline implementation candidate. Its tests use synthetic TLS peers and tokens. Nothing in this package authorizes real registration, account access, discovery, inference, or a live release.

The binary is `b2-public-host` in this crate. It has these operator actions:

```text
b2-public-host usage
b2-public-host identity
b2-public-host account <absolute-credential-dir> <stable-host-id>
b2-public-host sign-in <absolute-credential-dir> <stable-host-id> <action-grant.json>
b2-public-host models <absolute-credential-dir> <stable-host-id> <action-grant.json>
b2-public-host disconnect <absolute-credential-dir> <stable-host-id> <action-grant.json>
b2-public-host smoke <absolute-credential-dir> <stable-host-id> <action-grant.json> <smoke-grant.json> <absolute-private-work-dir>
```

`sign-in` binds a real listener to a random `127.0.0.1` port before creating the authorization attempt. It prints a first-registration browser URL, accepts one bounded HTTP callback within 600 seconds, validates the exact host/path/state in `AuthStore`, exchanges the code once, and validates the signed ID token against the discovered JWKS. It currently supports first registration only. A later returning-account UX needs a dedicated protected browser handoff for the ID-token hint; printing that URL or putting it in a process argument would expose credential material. Auth and token responses remain credential state, never inference artifacts. The listener is owned by the call and is dropped on return or error.

`account` reads the selected client, verified subject, and credential revision from local protected state without discovery. `models` performs one bounded account-specific GET. It saves the original response bytes and an account/revision-bound observation in protected credential custody; stdout contains its unique observation ID, exact response digest, byte count, timestamp, revision, and model count. Identical response bytes in a later admitted observation retain the original raw response and create a new revision/time-bound observation record. The protected observation contains the parsed slugs for later selection. `usage` prints the documented account usage/settings URL. `disconnect` commits local disable before one bounded revocation attempt and works when issuer metadata/JWKS are unavailable; it reports remote revocation as confirmed or unconfirmed. An unconfirmed result must not be treated as remote settlement or funding closure.

`smoke` consumes an existing protected discovery observation with the same credential revision, checks the original response bytes and digest, and requires the selected slug to have been visible. It makes no implicit model GET. Discovery and smoke need separate action grants. It uses exactly `Return exactly "B2_TRANSPORT_OK." including the final period. Do not include quotation marks or any other text.` as synthetic input, empty instructions, no tools, `store:false`, and `stream:true`. The request is at most 4 KiB; the response capture at most 1 MiB; parsed output at most 16 KiB; connect timeout 3 seconds and stream deadline 30 seconds. These local limits do not cap remote tokens or cancel provider processing. No retry, refresh, account switch, model substitution, billing fallback, API key route, or alternate endpoint exists in this host.

Every network action also requires a distinct, single-use action grant. The action's `release_id` names that action; it is separate from the smoke grant's one-shot `release_id`, `operation_id`, and `attempt_id`. The action binds the exact smoke-grant bytes by digest. The admitted API parses those bytes and rejects any different grant object before consuming the action or sending. The smoke grant names the unique model observation record ID by SHA-256; the response digest remains a separate raw-body identity. The `identity` command prints the currently built source and executable digests for binding by the release owner. The action grant binds the exact command, credential directory, host ID, optional smoke work directory and smoke-grant byte digest, actor, issue/expiry times, source digest and executable digest. It also requires an attributed credential custody owner and custodian, an exact owner acceptance/contract reference, a retention purpose, and a supported cleanup disposition. Sign-in, models, and smoke admit retention until owner disconnect; disconnect admits local deletion with a bounded remote revocation attempt. Unsupported dispositions fail before network entry. First sign-in stores the custody contract in the protected credential directory; later actions must match its owner, custodian, exact contract reference, and retention purpose. A different contract requires a distinct custody directory and separately admitted sign-in. These are supplied assertions and require actual owner acceptance before real use. It is consumed durably in credential custody before any network entry. The action expires within 630 seconds for sign-in, 15 seconds for models/disconnect, and 45 seconds for smoke. The CLI supervises each external action in one owned child process and collects or terminates it at the corresponding outer limit; the child still validates and consumes the grant before network entry. A local timeout or process exit leaves remote and funding outcomes unresolved. Example fields (these placeholders are invalid):

```json
{
  "release_id": "unique-action-id", "actor": "release-owner",
  "custody_owner": "account-owner", "custody_custodian": "named-token-custodian",
  "custody_contract_ref": "owner-accepted-custody-reference",
  "retention_purpose": "bounded public host auth and model inspection",
  "cleanup_disposition": "retain-until-owner-disconnect",
  "action": "models", "credential_dir": "/absolute/dedicated/custody",
  "host_id": "stable-host-id", "work_dir": null, "smoke_grant_sha256": null,
  "executable_sha256": "exact-binary-digest", "source_sha256": "exact-source-digest",
  "issued_at_unix": 0, "valid_until_unix": 0
}
```

The executable/source checks establish byte consistency with the running program. Grant actor, funding and account statements remain attributable assertions, not independent proof of external authority. Missing, stale, inconsistent, malformed or extra command inputs fail before external calls. A typical **placeholder** smoke grant is:

```json
{
  "release_id": "owner-chosen-unique-id",
  "operation_id": "owner-chosen-operation-id",
  "attempt_id": "owner-chosen-attempt-id",
  "client_id": "issued-client-id",
  "subject": "verified-subject-from-account-command",
  "credential_revision": 1,
  "selected_model": "slug-observed-by-models",
  "model_observation_sha256": "64-hex-unique-observation-id-from-models-output",
  "funding_assertion_ref": "protected-setting-observation-reference",
  "funding_actor": "account-owner-identifier",
  "funding_custodian": "named-custodian",
  "funding_setting_scope": "exact-client-workspace-or-account-wide",
  "funding_observed_at_unix": 0,
  "credit_use_disabled": true,
  "included_plan_enabled": true,
  "funding_observation_valid_until_unix": 0,
  "account_owner_accepts_open_custody": true,
  "accepted_closure_basis": "owner-accepted-remote-evidence-or-assumption-reference",
  "release_actor": "release-owner-identifier"
}
```

The timestamp placeholders above deliberately fail validation. An actual authorized release would need a fresh saved-OFF account-setting observation, an owner-accepted protected interval over all potentially chargeable processing, and a custodian who keeps the obligation open until its accepted closure basis is met. The host checks freshness, consistency, account revision, selected model, and local flags. It cannot observe the real account setting or provider billing state. A written grant is therefore an assertion, not independent funding proof.

The private work directory must be absolute, owner-owned, mode 0700, and without symlink path components. Smoke release, operation, and attempt claims are created with `create_new` in the stable credential custody directory before a POST; changing the work directory or action grant does not reset them. The work directory holds the open funding obligation, prepared attempt journal and durable send intent. A claim or evidence-write failure fences the send. The host checks the selected credential, saved model observation, funding assertion, action lifetime and bearer again at the final possible-send boundary. It requires at least the intended 30-second stream interval to remain; it does not refresh or retry. Failed currentness writes a `pre-send-fence` record asserting no POST was dispatched by this process, while leaving consumed claims and the open obligation in place. A prior intent still expresses conservative recovery uncertainty. The funding owner must still establish the protected interval across potentially chargeable remote processing.

After possible entry, a known HTTP status and bounded retained response body identity are saved in the protected journal; 401, 403, 429, 500 and wrong-content responses retain their distinct status. Network failure before headers retains unknown status. The exact retained bytes, count and SHA-256 name one sequence. `received_bytes` may exceed `response_bytes` when a bound is hit. `full_capture_complete` is false after a limit, parser failure, interrupted read or write. A `capture-status` sidecar records unknown retained prefix when a write may have been partial. The capture starts in an owner-only quarantine path and moves to the ordinary protected capture path only after safe inspection and journal persistence. Evidence-write failures leave the durable intent and protected quarantine, with remote outcome unknown. Best-effort `local-closure` records owned local handle closure on every exit after release claim. The supervising CLI also writes an `owned-child-closure` marker after collecting or attempting to collect its bounded child, including a forced deadline. Neither marker proves provider or funding settlement. If a marker write fails, that local closure evidence remains unestablished.

The public JSON projection contains only owned terminal categories and failure categories, status, capture identity and fixture comparison. Provider error codes, reasons, model slugs and response text are not copied to stdout. The scanner checks raw bytes, assembled SSE frame JSON strings, and the resulting observation strings, including fragmented and JSON-escaped active bearer echoes, before journal or ordinary capture placement. Suspected material remains in owner-only quarantine and parsed identity/output/usage is suppressed in the protected journal. Detection is limited to the active bearer and selected credential markers; it is not a comprehensive secret detector. Exact captures and quarantine require custodian access control and do not belong in `CapturedFixtureStream`, model prompts, logs or support artifacts. EOF, timeout, HTTP 200, local process exit, token expiry and revocation do not close remote processing or funding.

Unestablished by this candidate: live issuer metadata/JWKS shape, registration eligibility, account/workspace choice, actual funding controls and interval, real public endpoint compatibility, provider settlement, power-loss durability, real lifecycle mapping, semantic adequacy, and any S10 condition. It does not create a native credential or lifecycle result.

Response Content-Type diagnostics preserve the case-sensitive
`text/event-stream` prefix check on the original first header value. The protected
HTTP journal retains the exact value only when it is at most 1024 bytes, printable
ASCII and free of the active-bearer/credential patterns detected by the existing
scanner. This is bounded screening, not a comprehensive secret detector. Metadata
distinguishes missing, captured, non-text, oversized and redacted values and carries
the original prefix-check result. Redacted values carry neither text nor length
or digest. Normal CLI output exposes only metadata; safe exact text remains in
the protected journal. Screening or size-based omission of a present header does
not alter its original prefix predicate. Older and loopback-producer evidence
has no new metadata.

For HTTP 200 responses only, an absent Content-Type now permits the same bounded
SSE parser used for an explicit `text/event-stream` prefix. Missing metadata is
not evidence of a valid stream: framing, event schema, selected-model identity,
coherent terminal and clean stream-end checks still apply. Explicit empty or
non-SSE Content-Type values remain HTTP failures. Failed stream validation keeps
the exact capture in quarantine, including failures discovered only at EOF or
by the final selected-model check. Header diagnostics continue to report the
actual missing header and false prefix predicate; they do not fabricate a media
type. This compatibility rule does not relax the stream grammar or imply remote
settlement or funding closure.

The public host now records profile `b2-public-siwc-stream-v2` and opts into
`StreamObserver::siwc`. The existing `new` and `selected` profiles retain their
prior grammar. This observed SIWC compatibility profile accepts response-linked
`response.metadata` objects before terminal completion; their contents are
uninterpreted and have no output, model, usage or terminal authority. Identity,
sequence deduplication, event/frame/stream bounds and post-terminal rejection
still apply. This does not claim comprehensive metadata or moderation semantics.

For a coherent `response.completed` with explicitly empty `output`, the SIWC
profile can use previously completed item snapshots only when all observed
indices are contiguous, every item was added and explicitly completed, and at
least one assistant message has a nonempty content-part array with matching text-done and
part-done witnesses for each added part. The existing terminal validator then
checks roles, identities, text consistency, item/part coverage and aggregate
output limits. Reasoning items contribute no output text. Nonempty terminal
output remains authoritative; missing/null output, missing witnesses, failure,
incomplete status and unclean EOF never gain authoritative text from this path.
The original wire capture is retained unchanged; the reconstructed view is
internal only. These behaviors are bounded compatibility with observed captures,
not a claim that every SIWC response shape is supported.

As in the existing terminal validator, assistant-message `phase` is not
interpreted; validated message texts are concatenated in output-index order.
Witnessed empty text is representable and does not match the smoke fixture.
