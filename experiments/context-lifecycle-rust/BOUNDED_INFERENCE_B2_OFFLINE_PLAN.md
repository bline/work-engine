# B2 offline implementation and qualification preparation

**Candidate for coordinator admission, 2026-10-21.** This decomposes the user's
authorized B2 **offline** work. Root owns dispatch and integration; this planning
result is not implementation acceptance or live release. It extends the work map
in [the decomposition](BOUNDED_INFERENCE_DECOMPOSITION.md), without changing the
[original plan](BOUNDED_INFERENCE_IMPLEMENTATION_PLAN.md), the
[post-S10 amendment](POST_S10_BACKEND_ABSTRACTION_AMENDMENT.md), or their live and
production acceptance conditions.

The useful result is a small direct-client implementation exercised with fake
credentials and local HTTP peers, plus an isolated preparation of the verifier
evaluation. Three coherent groups are sufficient: **B2a authentication lifecycle
→ B2b transport/profile composition → B2c qualification preparation**. The
coordinator can revise the route when evidence changes; the ownership,
compatibility, uncertainty and acceptance boundaries below remain binding.

## Authority and source baseline

The lifecycle owner's decision at board **2352**, preserved in
`/home/bline/.local/state/work-engine/s10-session-quota-repair-20261021/b2-offline-ownership-decision-v1.json`,
permits bounded-inference synthetic auth/HTTP work and isolated verifier evidence.
It excludes existing native auth, lifecycle service/store/provider/runtime,
existing semantic-source changes, real login/credentials, S10 corpus/heldout/trial,
and production state. Any new shared auth or exposure interface returns to that
owner. The native experimental access-only lease is neither a direct SIWC
credential interface nor funding authorization.

Admission references are B1a `6138c43a`, B1b `4fe007b5`, native prompt prerequisite
`65c23e27`, and chatboard completion `0f85e86f`. B1a and B1b root-acceptance receipts
are under `/home/bline/.local/state/work-engine/bounded-inference-b1a-20261021/`
and `bounded-inference-b1b-20261021/`. Current source hashes and snapshots are in
`/home/bline/.local/state/work-engine/bounded-inference-b2-offline-20261021/planner/source-manifest.json`.
Working-tree bytes, including shared Cargo files, contain concurrent work;
reference HEAD is not an immutable implementation subject.

Only public documentation reads and local synthetic executions belong to these
packets. They do not open the authorization browser, register an actual client,
read a real credential directory or environment token, query an account, make
public inference, or select API/credit funding as fallback. An existing grant for
other work is not borrowed by this implementation.

## Current protocol evidence

The selected protocol is **SIWC open-source/local HTTP Responses**, not a ChatGPT
backend endpoint or a native Codex session. The following official pages were
opened and their Markdown fetched on **2026-10-21**. Exact retrieval timestamps,
URLs, bytes and SHA-256 values are in `planner/official-source-manifest.json`;
`planner/official-docs/` retains the sources. These pages do not expose a stable
protocol version or publication date; retrieval date is not publication date.

| Source | Supported premise for this implementation |
| --- | --- |
| [Overview](https://developers.openai.com/siwc/token-sharing-open-source) | Client registration is separate from a stable per-host identifier. The issued client belongs to the authorizing user/workspace. The documented local/open-source route does not establish this deployment's eligibility. |
| [Registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in) | First authorization uses `dynamic_agent_client`; persist the callback's issued client ID for code exchange and later sign-ins. Authorization is at `https://auth.openai.com/api/accounts/authorize`; token exchange is form-encoded at `/api/accounts/oauth/token`. Bind fresh state, nonce and S256 PKCE to the exact `127.0.0.1` callback URI. Resource is `https://api.openai.com/v1`; request identity scopes plus `offline_access resource.invoke chatgpt.tokens.use.direct`. Validate the signed identity and returned permission before inference. |
| [OIDC validation reference](https://developers.openai.com/siwc/website) | The sign-in page explicitly refers here for signature verification. Discovery supplies issuer and JWKS location; check signature, issuer, issued-client audience, expiry, original nonce and nonempty subject. Its confidential website-client examples do not create a client-secret requirement for this public-client route. |
| [Accounts and sessions](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions) | Keep registrations separate; refreshes for one session must serialize. Atomically replace the credential set; sign-out attempts refresh-token revocation through discovery, then clears local tokens while preserving the registration/host. Unconfirmed remote revocation is a distinct result. Account controls may allow credits after plan limits. |
| [Token reference](https://developers.openai.com/siwc/token-sharing-open-source/token-reference) | The response carries access/refresh/ID tokens, scopes, type, expiry and `earliest_refresh_at`; replacement refresh tokens rotate. Documented lifetimes are one hour and 30 days respectively. Use returned expiry rather than hard-coded entitlement. Access-token metadata is opaque and is not a workspace or quota proof. The precise units/operational semantics of `earliest_refresh_at` are not established by this page. |
| [Models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference) | Account-specific `GET https://api.openai.com/v1/models` uses the same bearer as inference and returns a `models` array, with `slug` and visibility. `POST /v1/responses` requires `store:false`, `stream:true`, and a model available to that account. Success requires `response.completed`; failed/incomplete/interrupted streams remain distinct. |
| [Preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations) | Supply complete array input with instructions/developer content. Omit HTTP continuation and unsupported fields, including `max_output_tokens`, `temperature`, `metadata`, `background` and `conversation`. A local byte/deadline bound consequently cannot be advertised as a provider token/quota cap. This task exposes no tools, including otherwise supported function tools. |
| [Errors and recovery](https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery) | Admission errors can use a `detail` body rather than an API error object. Preserve status, request ID and structured code/parameter when present. Usage-limit/unavailable errors can occur inside a stream. Terminal refresh errors differ from temporary failures. The page's optional alternative billing suggestion is outside this user's authority. |
| [Responses streaming reference](https://developers.openai.com/api/reference/resources/responses/streaming-events) | The general schema includes message text, reasoning items/events, refusal events and sequence numbers. It justifies synthetic compatibility cases; it does not demonstrate which shapes a particular SIWC model emits. The generic create-response page could not be opened because of size; no claim depends on that unopened page. |

Still **unknown**: actual registration/deployment eligibility, selected live
account/model and reasoning controls, account-specific request/response details,
opaque reasoning replay, observed limits, actual model behavior/adequacy, and
controls/evidence excluding credits. An OAuth scope, successful synthetic token
exchange, absent API key or token counter cannot establish strict plan-quota
funding. These are later live-owner decisions, not blockers for fake-peer work.

## Existing contracts and minimal placement

Keep authentication and selected HTTP realization in the existing
`rust/crates/bounded-inference/` package. It has a real consumer and already owns
request/stream/attempt mechanics. Do not introduce a universal backend trait,
credential service, operation registry or replacement harness. Credential
selection is not semantic admission; credential refresh is not a new operation
attempt or context generation. Semantic meaning remains with lifecycle-semantic.

The source baseline establishes four compatibility obligations:

1. `assemble_request` (`request.rs:84`) emits exact bounded bytes, model/profile
   identity and a digest. Its current wire already matches the documented minimal
   HTTP request shape. Keep its byte contract; additional selected parameters
   require an explicit new profile and consumer compatibility decision.
2. `StreamObserver` currently rejects reasoning/refusal (`stream.rs:489,800`),
   unknown events and selected nonempty intermediate snapshots. Tool-free does
   not mean reasoning-free. B2b may add an explicit selected-profile mode for
   bounded non-tool reasoning observations; preserve the old default fixture
   behavior and tests. Refusal/unsupported content never becomes a verifier
   report. Tool items remain rejected and never execute. Do not silently ignore
   arbitrary event classes or weaken identity/terminal reconciliation.
3. `AttemptJournal::mark_send_intent` (`journal.rs:157`) durably enters
   `PossiblyEntered` before transport. Terminal-observed records require a real
   response ID and valid stream evidence; a pre-stream HTTP error has neither.
   Add transport evidence alongside the attempt rather than inventing a provider
   ID or turning every HTTP failure into a provider terminal. Recovery may remain
   conservative about remote outcome; it never automatically replays a send.
4. `replay_fixture` (`lifecycle-semantic-replay/src/lib.rs:97`) reassembles and
   compares exact wire/profile/model bytes before assessment. `FixtureScope`,
   `CapturedFixtureStream` and `CandidateOrigin::SyntheticFixture` remain
   synthetic. Existing `DirectExposure` fields distinguish known from unknown;
   setting a boolean is not proof of real isolation. No new real-capture or
   semantic-source mapping is authorized here.

Graph evidence is Tier 2, project `home-bline-code-work-engine`, generation
`ub3bdca936f6f3a0dg2`, indexed `2026-10-21T21:05:45Z`. Material symbols were
snippeted; relevant depth-one traces and search pages were completed. Trace
resolver matches for unrelated `clone`/`validate`/`len` symbols were not treated
as dependencies. All six relied-on code paths have content verification and no
recorded coverage gap. Across 18 manifest paths, Cargo.lock has untracked
freshness: its exact bytes are snapshotted/read for identity, not used for a
dependency-completeness claim. Coverage is best-effort, not proof of completeness.
See `planner/graph-evidence.json` and `planner/graph-coverage-all.json`.

## B2a — synthetic authentication and credential lifecycle

**Outcome and owner.** One supervisor/worker owns the bounded-inference auth
modules, fake peers and tests. Implement the narrow selected SIWC protocol and
an explicit app-owned credential store with fake fixtures. Keep pending
authorization separate from the active registration. No implicit home-directory,
native-login, environment-key or system-keychain discovery belongs to this API.

Expose only what B2b needs: a selected registration reference, a redacted
readiness/failure view, and a short-lived bearer access operation bound to that
registration and credential revision. Secrets remain internal to auth/transport;
diagnostics and semantic requests receive non-secret references. Resolve the
precise Rust API in this packet and hand it to B2b. A protected store, auth
transaction and refreshed credential set are distinct values, not one shared
mutable session object.

Implement callback/result validation, signed ID-token verification against
synthetic JWKS, atomic credential replacement, serialized refresh, selected
account switching and sign-out/revocation outcomes. Use maintained crypto/OIDC
mechanics with pinned dependencies; a test double that accepts unsigned claims
does not establish signature validation. Configure clock/randomness and fake
endpoint responses explicitly. Do not launch a real browser or contact discovery,
JWKS, token or revocation services. An unestablished refresh-timing field is
retained with its limitation rather than assigned invented semantics.

**Concrete fault/security evidence.** Exercise wrong/replayed/expired state,
nonce mismatch, callback path/port mismatch, missing or substituted issued client,
denied consent, bad signature/algorithm/key, wrong issuer/audience/subject,
expired identity and insufficient scopes. Same email across registrations must
not combine credentials. Test refresh rotation with competing processes, stale
writer/account-switch/sign-out races, and crash/failure before and after atomic
publication. A lost refresh response cannot justify reusing a possibly rotated
token indefinitely. Test owner-only files, unsafe path/symlink rejection, bounded
inputs, and redaction of tokens, code, verifier and ID-token-hint URLs from error,
Debug and durable evidence output. Temporary transport failures preserve usable
state; terminal token failure blocks new use. Revocation failure and local
sign-out remain distinguishable.

**Done.** `offline_auth_lifecycle_accepted`: fake-issuer cases and persistence
faults pass, exact source/dependency/fixture subjects are reviewed, B2b's
registration-bound access interface is documented, and no real auth capability
was invoked. This does not establish registration, real storage custody,
account/plan permission or funding. A dependency/semantic/native owner collision
returns to root before changing excluded files.

## B2b — local HTTP transport and selected stream profile

**Outcome and owner.** After B2a's interface is accepted, one coherent worker
owns transport, selected-profile stream changes and joined tests in
bounded-inference. Reuse B1a request bytes, stream evidence and attempt journal;
reuse B2a credential ownership. Use a conventional HTTP/TLS library rather than
implementing a general HTTP stack. The offline realization admits only explicit
loopback fake peers, with proxies and redirects disabled. Production endpoint
constants may be represented as data, but this packet provides no public send
release or real credential source.

Transport binds operation/attempt, complete assembled request, auth registration
revision and profile before entry. Persist send intent before handing request
bytes to the network. Capture HTTP status, safe selected headers/request ID,
bounded error-body shape, received stream digest and exact attempt association;
never authorization headers or credential values. Consume incrementally under
request/header/body/frame/event/output/deadline bounds. Model discovery is also
bounded and tied to the selected fake registration; the API-platform `data` model
list shape is not silently substituted for the documented SIWC `models` shape.

Refresh can prepare credentials for a new request. Neither an ambiguous send nor
a stream error triggers hidden retransmission, refresh-and-replay, model
substitution or funding fallback. A later retry has its own explicit attempt and
its own observations. HTTP success, a text delta, socket closure and local
cancellation are not completion or remote cancellation. Preserve missing usage
as unknown, cumulative versus incremental evidence, and per-attempt accounting.
The current `response.usage.delta` support is a fixture premise unless a source
or later observed profile establishes that event for SIWC.

An additive selected-profile parser may accept bounded reasoning envelopes and
summary events from the fetched generic schema while extracting only assistant
message text. Bind item identities/types/indices through terminal reconciliation;
reasoning bytes count against input-stream bounds and cannot smuggle tool calls
or become report text. It need not retain/replay encrypted reasoning into another
request. Keep unsupported shapes explicit. Do not claim that the account emits
the tested fixture sequence or that every generic Responses shape is admitted.

**Concrete faults and composition tests.** An actual local socket peer records
the request count and exact body and can split arbitrary UTF-8/SSE bytes, pause,
truncate, send conflicting identities, exceed each bound, emit reasoning/refusal/
tool items, or fail after text. Test 401/403/429/503, non-JSON/`detail`/structured
errors, wrong content type, bad headers, redirect/proxy/header-injection attempts,
and local TLS invalid-certificate/hostname cases without disabling verification,
connect failure, timeout, and duplicate/conflicting terminal/usage events. Prove
one intended attempt does not become two HTTP requests. Fault intent/terminal
fsync and crash windows, then reopen the journal without network capability;
unknown entry stays unknown and retained completion is not resent. Cross-account
switch/refresh cannot retarget an already-bound attempt. Error-body capture must
not leak echoed credentials. Run joined auth→HTTP→parser→journal cases, not only
module mocks.

**Done.** `offline_transport_profile_accepted`: exact request and fake-account
binding, durable entry uncertainty and all selected profile outcomes are
demonstrated; B1a compatibility passes; no public request or live model claim.
Return the unsupported-event list and proposed future-live evidence needs.

## B2c — isolated verifier qualification preparation

**Outcome and owner.** Consume accepted B2a/B2b through a small disposable Rust
runner and newly authored synthetic corpus under the B2 task's qualification
evidence directory. A standalone local Cargo workspace can depend on the exact
bounded-inference and existing semantic/replay packages without adding a
permanent product package. No existing semantic/replay source edits, service or
store handles, corpus borrowing, or production publication are needed.

Freeze case identity, synthetic source/candidate origins, instruction/input and
profile hashes, expected host outcomes, and fixture ownership before the run.
The runner sends only to a fake local peer, retains actual local request/stream
evidence and exercises the existing semantic consumer on compatible synthetic
text fixtures. Existing `CapturedFixtureStream` may describe that synthetic
stream truthfully; it must not be renamed as a live observation. Keep the
original bytes. If the selected profile's reasoning stream cannot pass the
unchanged replay API, record that boundary and use the explicitly shared
text-only fixture subset for this composition; never strip events to fabricate
compatible capture or silently broaden the consumer. B2b separately tests its
broader selected stream profile.

The evaluation inventory should cover valid reports, wrong source/candidate/
attempt/model bindings, stale/corrupt material, missing/false citations,
authority widening, revoked/superseded obligations, unavailable exposure,
malformed/refusal/noncompleted output, and local quota failures. Record expected
versus obtained host outcomes and actual parser failures. Known fixture answers
test plumbing and checks; they are not an independent semantic-quality holdout
or a model-adequacy score. A prospective live evaluation will need a separately
authorized corpus/custody decision.

Prepare a later-live evidence checklist with explicit **unestablished** entries
for deployment/registration, credential owner, account/model permission,
quota-only controls, complete real exposure/custody, stream-profile compatibility
and model quality. Real capture/result/exposure mapping remains a lifecycle-owner
decision. A request digest, tool-free body or locally constructed exposure flags
cannot close it.

**Done.** `offline_verifier_qualification_prepared`: joined local transport and
unchanged synthetic semantic consumer are exercised, evidence is replayable,
compatibility failures and missing live evidence are explicit. Neither
`direct_verifier_live_qualified` nor production/native/S10 acceptance follows.

## Dispatch, shared files and final collection

Each packet uses the approved manual topology: coordinator → scoped supervisor
→ **Sol/medium worker** → separate **Astra/xhigh read-only reviewer**. Root may
serialize the groups, which keeps auth/transport composition and same-crate
ownership coherent. Supervisor collects worker and retained reviewer, validates
the composed subject, and returns bounded readiness. Root owns integration and
all remaining collection. Requested model/effort, recorded runtime identity and
any unavailable attestation are separate facts; this is not cross-provider
independence or independently attested capability confinement.

Before dispatch, bind this plan, actual affected source, fixture/profile revision,
evidence destination and predecessor receipts. Each worker owns its named crate
files and must preserve others' work. **Root alone reserves and integrates
`rust/Cargo.toml`/`rust/Cargo.lock`**, including any crypto/HTTP/URL/randomness
dependencies; the scoped worker proposes exact version/features and its crate
manifest delta. Coordinate the shared export file when ownership changes.
Use task-specific build targets and source-bound snapshots when concurrent
changes would invalidate results. No workspace-wide formatting or cleanup.

Required validation is focused formatting/lint/tests on the final subject,
joined auth/transport faults, existing bounded-inference regression tests, and
existing lifecycle-semantic/replay compatibility tests affected by those APIs.
Broaden downstream checks when the actual dependency/API change warrants it.
Failure evidence remains retained; corrected hashes alone do not repair an
invalid premise. Review checks the authority boundary, secret handling, hidden
retries, stream/profile assumptions and downstream result meaning. Keep the
same separate reviewer through bounded remediation unless a changed premise
requires a new review entry, recording continuity truthfully.

All roles receive the current context-check pilot block with verified actual
session/rollout identity, pinned observer, continuation path and collection
owner. They record logical-boundary observations, epoch/lineage, transfer
availability and judgment; economics stay unestimated unless supported. Wind
Walker governs recovery/handoff. An unfinished `handoff_ready` is not completion;
the parent verifies the packet, launches/binds a successor and obtains ownership
acknowledgment. No child/process responsibility is abandoned at turn end.

Final root disposition can accept these three offline results and integrate the
authorized scoped changes; user-visible commits retain their owning authority.
It must still carry live registration/account/model/
quota qualification and real semantic exposure mapping as open, and production
integration as separately owned. Offline completion advances B2 without claiming
the original B2 live exit was achieved.
