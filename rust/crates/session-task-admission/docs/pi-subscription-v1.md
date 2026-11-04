# Signed Pi subscription backend candidate v1

This opt-in backend candidate reuses PiOwner, signed-answer verification, key
policy, current registry, admission/outbox/capacity, effect uncertainty and Host
custody/reservations. It asserts no real owner identity, provider eligibility,
subscription billing, deployment, UI acceptance or owner acceptance. Synthetic
validation remains synthetic. The fixture v1 contracts and fake labels remain
separate.

The accepted intent tuple is `dot.task-intent/pi-coding-subscription-v1`, provider
`pi`, operation `start`, execution profile `disposable-pi-coding-subscription-v1`.
The workspace profile is `dot.pi-coding-subscription-profile/v1`, model
`gpt-6-luna`, effort `off`, tier `subscription`. Exactly `src/lib.rs` is persistent
writable scope, one concurrent task, zero children. Network true describes the
inference subscription adapter; tools and validator remain network-none. Scratch
is ephemeral. A boolean network grant does not select an endpoint or provider.
Unknown tuples, schemas and fields fail closed. Existing signing bytes and
`work-engine.owner-answer/v1` apply; no signing codec changes.

The profile uses the existing Profile shape: task/proposal/mode/configuration/
parent refs, actor/objective, exact serialized Host job SHA, native helper SHA,
transport SHA, expiry and classification
`subscription-backend-candidate; owner-and-provider-qualification-not-claimed`.
The exact Host job binds runtime inventory, image, workspace, acceptance,
subscription model/catalog/task/context/request/cost bounds, public configuration,
private native-source reference and access adapter. Credential contents are not a
pin or exported artifact. Descriptor v1 retains its shape and interpretation:
protected config/profile/admission request and the fixed authenticated transport
tuple. HTTP identity verification remains with the endpoint. Opaque PiOwner and
EffectLease remain non-deserializable with no public constructors.

The Host profile is `work-engine.pi-coding-signed-subscription.v1`. Only
`Host::admit_signed_subscription(job, owner)` admits it; ordinary Host admission
refuses it. Trusted startup chooses every path/model/job/helper/config input.
The MCP protocol `work-engine.mcp-pi-coding-subscription.v1` delegates the same
public domain operation, with only handle/revision/idempotency_key arguments.
It is disabled unless `PI_CODING_SUBSCRIPTION_ENABLED=1` and all existing
PI_CODING binary/config pins are present. Fixture and subscription switches
conflict. The authenticated coding-only endpoint is still required. Status,
result and exact duplicate submit are immutable reads, including unknowns;
changed bindings conflict. No public request chooses test behavior.

Public preparation creates an empty private access directory and public staged
configuration. The inference container mounts the directory read-only, not an
already-existing access file. After durable Allocation intent, actual SDK
SessionManager.create reports its UUID and absence of access. PiOwner seals that
UUID once for the exact job. Current Initialization authority then fences the
bounded native-access projection. The receiver cannot construct ModelRuntime
until the current-checked initialization IPC arrives; createAgentSession gets the
same manager. Ready and subsequent frames must match the sealed UUID.

Before each provider grant the existing Host journal reserves the full catalog
maximum, then PiOwner enters a unique Provider effect. Current signed authority
is rechecked immediately before actual IPC flush. The actual SDK subscription
stream has retries disabled and a fixed endpoint, no redirects. Each tool intent
gets a unique lease, rechecked at final process spawn after preparation. Signed
control observations order through the existing registry; observed revocation
cannot roll back on denied dispatch. Tool and provider unknowns never replay.
Raw backend model/status/completed usage must be present to settle the catalog
proxy reservation; normalized SDK zeros do not establish measured usage.

Result `dot.pi-coding-subscription-result/v1` is stored runtime evidence, not
owner acceptance. It retains operation/profile/descriptor/job/native identity,
outbox and effect uncertainty, plus Host process/tool/provider/validator/export/
cleanup evidence. The Host records stored receipt separately from receipt ACK;
`owner_pending`, `start_unknown`, `not_claimed` and retry false are not upgraded.
Reopen stops/collects existing containers and never reallocates/restarts/resends.
A crash between external action and observation remains unknown. Cross-store
result publication is not an atomic remote transaction or exactly-once proof.

Provider-disabled qualification uses synthetic signing/JWT/access and a pinned
private copy of the SDK runtime with a test preload import. Only that test
realization intercepts the exact fixed subscription fetch and returns canned SSE
Responses through the actual SDK parser. It denies other URLs, methods,
credentials, redirects and excess requests. Production receiver/binary/domain
contain no preload flag, fetch injection callback or arbitrary endpoint option.
That test proves bounded SDK/control/tool composition; it does not prove a live
request, real access eligibility, real human signing or all crash histories.

Trusted host/SDK/Docker/OS, protected current-source/key selection and verified
HTTP identity remain trust assumptions. Same-UID/root/daemon compromise and
kernel escape are outside this boundary. Bridge networking is not a provider-only
egress firewall. JWT decoding checks structure/expiry/account consistency, not
signature or account eligibility. Refresh, API keys and provider fallback remain
excluded.

AH2/A4.2 must compose these owned operations and display the exact Pi/profile,
provider/model, persistent path, ephemeral scratch, network scope, native-before-
access rule, task/job/cost/deadline/signing bindings and unqualified status. Their
implementation/acceptance is separate. A later live pilot needs reviewed frozen
backend pins, compatible endpoint and UI/signing contracts, current owner/key/
activation/task authority, selected native source and explicit credential/provider
execution permission. This candidate grants none of those permissions.
