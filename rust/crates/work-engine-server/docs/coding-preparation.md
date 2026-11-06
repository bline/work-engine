# Prepare an exact coding job

`coding-job-prepare` is a local Rust operator entry. It inventories explicit
inputs, checks a separately trusted preparation authority, and writes a fresh
Host job, packet, manifests, shim and server job profile. It never launches a
Host, Docker, SDK or provider. The existing server owns one-shot submission;
the Host and lifecycle store retain execution and settlement ownership.

Build the server package, then use:

```text
coding-job-prepare inspect /absolute/spec.json
coding-job-prepare prepare /absolute/spec.json /absolute/operator-authority.json
work-engine-command /absolute/service-profile.json submit_coding_job --json /absolute/submit.json
work-engine-command /absolute/service-profile.json read_coding_job_operation --json /absolute/read.json
work-engine-command /absolute/service-profile.json read_coding_job_result --json /absolute/read.json
```

The spec has schema `work-engine.coding-preparation.v1` and these exact fields:

| Field | Meaning |
| --- | --- |
| `job_id` | Fresh bounded job identity, distinct for each separately admitted job |
| `output_dir`, `state_dir`, `operation_root` | Absolute, disjoint bundle, fresh Host state and private server operation roots |
| `host_binary`, `docker` | Explicit existing executable paths; content is pinned |
| `task`, `workspace`, `validator` | Task file, disposable source tree and immutable Rust acceptance file |
| `runtime`, `toolchain` | Existing pinned Pi1.5.0 and offline Rust trees; preparation does not download or upgrade |
| `image` | Existing immutable Docker image digest |
| `deadline_unix_seconds`, `command_timeout_seconds` | Host deadline and 1–30 second command bound |
| `collector_deadline_seconds`, `cancel_grace_seconds` | Separate server collector deadline (1–900 seconds) and cancellation grace (1–30 seconds) |
| `fixture_command` | Explicit deterministic command for provider-disabled preparation; empty for Sol/high |
| `sol_high` | Absent/null for offline fixture, or the explicit future live preparation inputs below |

All files and tree roots are canonical absolute references. Source mutation
between inspection and preparation invalidates the input digest. A bundle or
Host state directory cannot be reused. Preparation failures may leave partial
fresh output directories; inspect them and choose fresh paths rather than
turning preparation into replay. The generated job profile must still be
selected through the operator's existing server startup configuration.

`inspect` reports `spec_sha256`, `inputs_sha256` and `job_id`, with
`authority: observation_only`. A trusted operator separately supplies:

```json
{
  "schema": "work-engine.coding-preparation-authority.v1",
  "spec_sha256": "<inspection value>",
  "inputs_sha256": "<inspection value>",
  "job_id": "<exact spec job_id>",
  "authorization_id": "<one-shot authorization>",
  "caller": "<configured transport caller>",
  "credential_sha256": "<SHA-256 of separately provisioned job credential>",
  "allow_prepare": true,
  "allow_provider": false
}
```

Keep an immutable copy of the exact Host producing binary at the selected path
so later rebuilds cannot invalidate readback. Preparation does not create this
authority or a credential. Keep its file private. The exact spec bytes and the full inventory/artifact identity are
bound independently. The generated profile pins both the job-file digest and
the Host's typed serialization digest; these are separate contracts even when
compact serialization makes their values equal. Do not edit generated files.
For another task or job, use a fresh spec, fresh state and separate authority.

Submission arguments are the existing `request_id`, `authorization_id`,
`job_credential`, `job_sha256` and `packet_sha256`; reads use the first three.
Keep the separately provisioned credential in a private argument file or stdin.
Submit/read are also available through the existing MCP tools and HTTP API.
Exact repeat reads the original operation and cannot admit replacement work.
The result's `owner_pending` acceptance and exact raw receipt numbers remain
unchanged by command rendering or transport success.

## Sol/high preparation

The optional `sol_high` object has exact fields `pi_config` (public config root),
`native_auth_source` (reference only), `payer_account_id`, `runtime_admission`
(an existing `{path,sha256}` artifact), and `budget_control` (the existing
canonical observation-only continuity budget). Preparation never opens the
native source reference. It requires an independently supplied future runtime
admission; the preparation grant's `allow_provider:false` cannot create one.
A successful preparation is not permission to execute the resulting live job.

The new Host profile is `work-engine.pi-coding-lifecycle-sol-high.v1`, with
runtime admission `work-engine.pi-coding-runtime-admission.v2` and tuple:

```json
{"schema":"work-engine.sol-high.v1","reasoning_effort":"high","payer_account_id":"<explicit account>"}
```

It admits only `gpt-6.1-sol`, `openai-codex`, `openai-codex-responses`,
`https://chatgpt.com/backend-api` and four requests. Prepared jobs use 24,000
context bytes (the server ceiling) and the conservative 17,680,004 microUSD hold. The pinned catalog's maximum tier rates
and full context/output reservation are retained: 4,420,001 microUSD/request.
This is a hold, not expected use, measured billing or subscription quota.
The receiver validates the projected payer and selects high reasoning. Host
frames bind the same effort and payer before reserving provider entry.

Lifecycle v5 preserves native thinking/signature blocks in exact history,
derives final text only from text blocks, and creates the separately named
`work-engine.pi-coding-sol-high-continuity.v1` checkpoint. Legacy v3/v4 grammar,
profile digests, encoding and receipts keep their old meanings. New profiles
cannot settle through the generic text-settlement entry.

The opt-in `sol_wire` test captures the actual pinned Pi adapter's serialized
request through substituted SSE fetch, including its zstd wire bytes. It uses
a deterministic synthetic JWT, removes authorization from the capture, and
stops transport before any network request. The same receiver session-options
helper supplies its effort. This establishes preparation/serialization only;
actual account/model eligibility, backend responses and coding quality require
separate live authority and qualification.

## Failure evidence

Collector deadlines, Host workspace command deadlines and container closure
have separate evidence. The server retains bounded exact stdout and stderr
prefixes as `collector-<action>.stdout` and `.stderr`, with digest/byte-count
references in the collector record, even for malformed, interrupted or hanging
output. `stdout_truncated`, `stderr_truncated` and collection-observation flags
stay explicit. This does not settle unknown task effects.

The Rust owned-child fixture tests a process that ignores actual SIGTERM,
forced collection after grace, partial-output preservation, a healthy quiet
control and interrupted server/restart/replay with a separate effect counter.
That fixture establishes collector/process-group properties only. Opt-in
Docker cases use fresh actual Pi/tool containers and retained raw Host receipts;
only their directly observed timeout and absence checks can establish the
selected workspace/container properties. Synthetic reasoning/reset fixtures
are lifecycle proof tests, not real provider completions or acceptance.
