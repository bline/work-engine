# Events coding-job bridge (disabled by default)

An opt-in transport from the Events MCP server (`skills/session-events/scripts/session_events.py`) to the existing Rust `work-engine-command` job operations. It is an operator-configured, offline-fixture seam. It does not claim a live provider job, a real identity, a native admission, or a deployed runtime, and it adds no job, authority, store, collector, retry or policy owner.

Capability: `coding-job-offline-fixture-v1`, from the private CLI candidate at `b7ddfd4c8c4c63f293aa3418a0202a687196ea04` (including its coding-job operating guide). Native Rust (`work-engine-command` → `LocalClient::job` → `work-engine-server`) owns job authorization, admission, custody and strict DTO validation. This bridge only forwards.

## Private offline fixture setup

Operational enablement is not accepted by this change. Keep the integrated tools disabled until a separately qualified command binary and an exact deployment/authority packet are approved by their owners. The private CLI candidate has only bounded DTO-contract acceptance; its parallel repeated-submit failure remains unresolved. Python/test/docs integration does not merge that Rust candidate, and the shared branch cannot yet build its job API. Do not use a self-attested qualification flag as a substitute.

The following settings are for authorized disposable fake-backend fixtures only. Nothing changes unless all of these are set; tool discovery is otherwise identical to the existing server. A switch without complete pins, pins without the switch, or any pin mismatch refuses startup.

| Variable | Meaning |
| --- | --- |
| `CODING_JOB_BRIDGE_ENABLED=1` | explicit switch |
| `CODING_JOB_BINARY`, `CODING_JOB_BINARY_SHA256` | absolute path and digest of the private `work-engine-command` fixture binary built from the pinned CLI candidate; owner-only-writable, executed from the pinned descriptor |
| `CODING_JOB_CONFIG`, `CODING_JOB_CONFIG_SHA256` | private (`0600`) bridge config and its digest |

Start with `serve-http --auth-config … --coding-job-only`. That mode requires OAuth for every call, restricts calls to the four tools below, filters `tools/list` to them, and refuses to start unless the configured `subject` equals the OAuth config subject and the configured `principal` equals `--principal`. It cannot be combined with the Pi fixture or observation helpers.

Bridge config (`work-engine.mcp-coding-job-bridge.v1`, unknown or missing members refused):

```json
{"schema":"work-engine.mcp-coding-job-bridge.v1","subject":"…","principal":"…","client":"…",
 "command_profile":"/abs/profile.json","command_profile_sha256":"<64 hex>",
 "authorization_ids":["…"],"timeout_seconds":30}
```

`command_profile` is the existing operator profile read by `work-engine-command` (loopback endpoint, service credential file, timeout). It is re-hashed before every invocation. The verified bytes are copied into a new owner-private temporary profile, read-only to the client, and removed after client collection; Rust consumes that snapshot rather than reopening the operator path. The caller cannot choose the binary, profile, credential file, endpoint or job JSON. `authorization_ids` (1–16) is a deny-only narrowing of which native authorizations this one principal may name; the native owner still decides every request. `client` is the operator-declared client identity echoed in results; the bridge cannot verify it equals the native authorization's `caller`, which the native owner checks.

## Tools

`submit_coding_job`, `read_coding_job_operation`, `read_coding_job_result`, `cancel_coding_job`. Arguments are exactly the native ones (`request_id`, `authorization_id`, `job_credential`, plus `job_sha256` and `packet_sha256` for submit); extra, missing or non-string members are refused before any process starts. There are no create, issue or approve tools and no qualified or approved flags.

Two credentials stay separate. The service transport credential lives in the operator's profile and authenticates the transport only. The job credential is supplied per call by the caller, forwarded on stdin to the native command and never logged, echoed or stored by the bridge. Request and hash binding are the native fields, unmodified.

Annotations make no side-effect-free promise: reads are not `readOnlyHint` because the native owner may reconcile backend custody while reading; cancel is `destructiveHint`; submit is `idempotentHint` only in the native sense that an exact repeat of one `request_id` observes the same operation.

## Results and uncertainty

Success returns `disposition: native_result` with the command's envelope under `native_envelope`, unmodified (receipt, usage, custody, `null` and `not_established` values keep their native meaning), the identity of the request, and `client_process` (exit code or signal). Process exit is client process closure only; a receipt or cleanup is not execution proof or settlement.

A native owner error returns an MCP error with `disposition: native_owner_error`, the unmodified envelope and `entry_uncertain: not_established` (the bridge does not claim either way).

A timeout, killed or crashed client, oversize output (over 1.5 MiB), malformed output, or a client-side transport or response-validation error returns `reason: coding_job_outcome_unknown`, `disposition: unknown_no_retry`, the exact `request_id` and `authorization_id`, and guidance to read the original operation. The bridge makes exactly one native invocation per tool call and never retries or resubmits with a new `request_id`; a different `request_id` is refused by the native owner as `authorization_consumed`.

## Tests

`skills/session-events/tests/test_coding_job_bridge.py`. Stub-binary tests always run. The real-CLI tests need `CODING_JOB_COMMAND_BINARY` set to a `work-engine-command` built from the pinned private CLI candidate and run it against a disposable local fake HTTP backend. The fake is narrower than the native server: it shows what the bridge forwards and how it surfaces responses, not native admission, Host behaviour or runtime.

## Limits

- Offline fixture and fake-backend evidence only. Native end-to-end behaviour is not established here, including the unexplained repeated-submit `operation_id: null` under parallel composition recorded in the CLI acceptance.
- The command binary is executed from a verified descriptor and the verified profile bytes are handed off through a private temporary snapshot. This is not isolation against a hostile process running as the same OS user.
- No Rust, Host, policy or skill-release change. The pending A2 policy design is future work and is not duplicated here.
