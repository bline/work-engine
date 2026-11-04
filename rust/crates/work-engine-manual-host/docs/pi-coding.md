# Bounded Pi coding profiles

These private profiles reuse the existing Rust Pi custody Host and journal.
The original CLI and observation profiles retain their scopes. Neither a
provider-disabled fixture nor receipt delivery asserts real inference, cognition,
domain settlement, production applicability or subscription billing.

`work-engine.pi-coding-fixture.v1` qualifies the actual native SDK tool path
with a fixed fake assistant stream and explicitly canned command. It uses fake
credential material only and network-none. Actual qualification demonstrated a
seeded minute assertion failure, an explicit fixture edit, three original pinned
acceptance tests passing, both containers removed and receipt acknowledged.

`work-engine.pi-coding-sdk.v1` adds a distinct, explicitly admitted subscription
lane. Its `coding.provider` object requires `lane`, `task`, `catalog`,
`access_adapter`, exact `model`, `max_requests`, `budget_microusd` and
`max_context_bytes`. The live profile has an empty `fixture_command`; fixture
commands cannot substitute for inference. Current bounded task mutation permits
only `src/lib.rs` in a disposable workspace. Rust runs original immutable
acceptance tests after the native session ends and retains actual outputs.

## Authority and configuration

Provider execution requires an exact `job.authority` JSON document with schema
`work-engine.pi-coding-runtime-admission.v1`, `job_id`, current executable
`binary_sha256`, `model`, explicit `native_auth_source`,
`pi_config_manifest_sha256`, `task_sha256`, `catalog_sha256`, `max_requests`,
`budget_microusd`, and both `allow_native_access_projection:true` and
`allow_provider:true`. Missing, unknown or mismatched fields fail before native
credential loading. This local operator-bound document is an authority reference;
it is not a new authorization owner. Source review alone does not supply it.

Each lane explicitly names its operator-controlled Pi public configuration tree
and native Codex auth source. Public inputs are pinned separately from secrets.
The configuration tree supports `settings.json` and empty `models.json` only;
custom provider endpoints, executable discovery and model definitions are refused
in this single subscription lane. Rust verifies and copies the public bytes to a
private immutable realization mounted at `/config`. No global home is assumed.
`CODEX_HOME` is not used: no Codex CLI subprocess or nested harness is launched.
No host Pi installation or separate Pi login is required.

Only the runtime-admitted Rust branch opens the native source. It checks private
file metadata and a bounded stable read, never writes the source and never hashes
or exports it. The exact currently observed native shape is `auth_mode:chatgpt`,
empty/null `OPENAI_API_KEY`, and `tokens.access_token/account_id` with an access
JWT. Nonempty API key, malformed JWT, mismatched account and insufficient expiry
are refused. JWT decoding checks structure, expiry and consistency; it does not
verify signature or backend eligibility. The grant must outlive the whole job
plus Pi's five-minute minimum so no refresh is required.

Rust stages only access token, expiry and account in a private0600 file outside
exports; neither refresh nor ID token is copied. The full native cache is never
mounted. A minimal read-only SDK CredentialStore maps that projection to OAuth,
exposes only `openai-codex`, rechecks expiry and refuses mutation without invoking
its callback. Refresh, login, deletion, API-key fallback and ambient credential
inheritance are unsupported. Access staging is removed after inference cleanup.
The SDK internally calls its bearer field `apiKey`; this OAuth access-token field
is not an API-key fallback. Do not print token/account values or secret digests.

## Isolation and custody

The inference container exposes only `workspace_exec`. Extensions, MCP, skills,
context files, templates, retries, compaction and cache warming are disabled.
The callback sends bounded correlated requests to Rust; it never runs a local
shell or opens model-selected inference paths. All model-selected commands run
in a separate network-none tool container containing only disposable writable
workspace, immutable acceptance source and pinned offline compiler realization.
It has no credential mount, provider environment, host root, inference processes
or Docker socket. Docker executes outside both containers with ambient env clear.

Both containers use UID1000, read-only rootfs, dropped capabilities,
no-new-privileges, finite CPU/memory/PIDs and separate PID/private IPC namespaces.
Inference tmpfs is noexec; tool tmpfs explicitly permits compiled test execution.
Rust inspects exact identities, commands, mounts, network and security settings
before start/exec and collection. Live inference uses bridge networking for the
fixed SDK subscription endpoint; tools remain network-none. No model-facing tool
can issue arbitrary inference-container network operations. This is not a claim
that a general network firewall enforces provider-only egress.

Inference retains the pinned node image. The already-local tool image is
`cbm-doc-claims-lint-tidy@sha256:dda6d32b2c2df1064f056db0db4cff894c833089ccf92862331359c0e6072754`
with an inspected `/bin/sh` entrypoint override; GCC13.3 and staged Rust1.92 were
observed compiling offline. The node image alone lacks cc. The compiler's entire
staged bin/lib realization is inventoried; no download or image pull occurs.

Rust writes tool intent before exec and provider reservation before grant.
Duplicate IDs, unknown prior effects, wrong native identities, late requests,
excess counts/output/context and unsupported frames fail closed. A failed tool
command is a semantic tool result with stdout/stderr, allowing normal debugging;
its nonzero exit cannot by itself establish task failure or success. Timeouts,
unknown processes and final acceptance failures prevent component success.
Reopen never retries create/start/exec/provider entry. Both containers require
stop/wait/remove/absence evidence and bounded raw exports plus receipt ACK.

## Catalog proxy budget and unresolved live evidence

The exact pinned catalog pair is `openai-codex/gpt-6-luna`, API
`openai-codex-responses`, endpoint `https://chatgpt.com/backend-api`.
ContextWindow272000/maxTokens128000; base per-million rates input0.1/output0.5,
cacheRead0.01/cacheWrite0.125 USD, with a higher tier0.2/0.75/0.02/0.25.
Rust reserves the full context at the greatest input/cache rate0.25 plus full
output at0.75, conservatively rounded upward by one microUSD:164001/request,
328002/two,492003/three,984006/six. The maximum is six attempts within1000000
microUSD, with finite context bytes and tool/output/deadline limits.

Pinned `openai-codex-responses.js` does not map generic maxTokens into the request
body. This profile therefore does not claim a smaller provider output limit is
enforced. Its proxy reservation covers full catalog output. SSE and maxRetries0
avoid WebSocket fallback and internal retries; redirects are refused. A request
with unknown/error entry or unavailable usage retains its full reservation and
cannot authorize another request. Reported successful usage is bounded and
reconciled using the conservative catalog rates; backend model/status identity
must be observed. The catalog is source evidence, not backend eligibility or
subscription billing. Actual provider/model availability and live compatibility
remain unestablished until a separately reviewed exact runtime admission.

Fake Rust and Node tests cover projection privacy, expiry/API-key refusal,
refresh callback refusal, explicit lane references, durable grants, unknown/no
replay, budget bounds and backend identity. The ignored actual Docker test
requires fresh `S10_CODING_EVIDENCE`, explicit `S10_PI_RUNTIME` and
`S10_RUST_TOOLCHAIN`. It proves only its provider-disabled fixture path.


## Reviewed correction: acceptance, raw usage and stable exports

A durable RPC completion does not establish final acceptance. Success now also
requires a separately persisted proof binding the validation process stdout,
original acceptance digest, validator identity and tested workspace manifest.
Cleanup-only recovery never creates or starts a validator; absent proof remains
not_established even if native RPC completion and container cleanup are present.

After inference ends, Rust stops/removes the model tool container and confirms
absence before copying its final workspace. The exact src/lib.rs-only boundary
is checked on that stable tree. A separately bounded network-none validator
mounts the copied snapshot read-only, with the same pinned compiler and original
acceptance source. Its tmpfs alone is executable/writable. Validator process,
daemon wait/exit, removal and absence are all collected. Only the tested snapshot
is exported; its manifest, final change boundary and each exported digest must
match the proof. Surviving model writers are killed before snapshot creation,
and acceptance code cannot mutate the read-only workspace it tests.

Provider settlement additionally requires the raw response.completed event with
explicit status completed and integer input_tokens/output_tokens/total_tokens,
with consistent totals. Normalized SDK stop/usage defaults are retained only as
observations. Missing, malformed, nonterminal or inconsistent raw evidence keeps
the full reservation and refuses a subsequent grant. Fake SDK SSE fixtures
exercise missing usage/token/status cases without real inference or credentials.

## Opt-in persistent source-file containment

`workspace_write_boundary: "src_lib_rs"` selects a read-only `/workspace`
bind plus one writable bind at `/workspace/src/lib.rs`. The source must be a
regular file with one link and regular directory parents; the Host checks the
copied source before container creation and checks the exact mount set through
Docker inspection before tool execution. Compiler output belongs in the existing
`/tmp` tmpfs. In-place writes work; editors that replace files with a rename
receive a filesystem error. The snapshot validator still mounts the entire tested
workspace read-only, with the original pinned acceptance file, and final export
continues to require the existing snapshot/validator proof.

Omitting the field preserves the legacy workspace mount and serialized profile.
This selector establishes filesystem containment only. It carries no signature,
current-registry admission, provider qualification, or owner acceptance. The
provider-disabled actual test can opt into this boundary with
`S10_STRICT_SOURCE=1`; its synthetic stream remains labelled as a fixture.

The signed Pi/MCP composition is not implemented by this selector. In particular,
the existing live path projects access before Pi allocates and reports its native
session. Binding an actual session before access requires a separately implemented
pre-access native handshake and signed relation, plus authority fences in the
existing admission owner. Until that conjunction exists, source containment and
fixture success do not qualify a signed Pi coding request.
