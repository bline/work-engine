# Scratch inference ideas compared with the Rust implementation

Recorded 2026-11-15 against repository HEAD
`01e41b04fdcaf34eddbe07ac95421ffe3a13e2e5`. Status: dated source assessment;
not an accepted implementation plan or a production-readiness determination.
The [idea collection](README.md) owns the extracted proposals and attribution.

The Rust implementation already supplies much of the proposed enforcement and
evidence machinery. The largest gaps are a shared execution path, management
of all four funding streams, and decisions informed by measured cache and
quality outcomes. Some proposals therefore extend existing owners; others add
capabilities that were not found in the surveyed runtime. The work is more than
connecting interfaces: the existing request profiles, streaming behavior and
qualification boundaries differ materially.

## Comparison

“Partial” below means relevant source exists, with the stated integration or
qualification still missing. It does not imply live acceptance. “Not found” is
bounded by the survey described at the end.

| Extracted idea | Current Rust implementation | Remaining difference |
| --- | --- | --- |
| [Trusted host gateway](host-inference-gateway.md) | **Partial, split across paths.** Isolated native Pi reconstructs authorized synthetic requests. `bounded-inference` separately implements host credentials, fixed-origin HTTPS, SSE observation and journals; task-host explicitly composes its Direct adapter. | A general Pi coding path combining these capabilities, real incremental response delivery, cancellation/backpressure, and qualification of that exact composition. |
| [Resource management](inference-resource-management.md) | **Strong local admission; partial resource coverage.** Pi's subscription lane binds model/payer/configuration, reserves before entry and enforces request/budget limits. Other native paths have explicit auth modes. | Shared availability and failure policy across OpenAI subscription/API and Anthropic subscription/API; fresh quota/credit observations and authorized billing failover. |
| [Optimization and routing](inference-optimization-and-routing.md) | **Configuration and analytical building blocks.** Compiler projects model, effort and service tier; context-status compares continuing and fresh-context cost scenarios. | A runtime consumer ranking qualified candidates by quality, cost, latency, reliability and cache opportunity, then submitting them for resource admission. |
| [Cache affinity and effort](cache-affinity-and-effort.md) | **Telemetry and scenario support, uneven by path.** Rust public-stream parsing records cached-input tokens; Claude transcript export distinguishes cache reads/writes and missing values. Pi receivers explicitly select `cacheRetention: 'none'` and fixed effort profiles. | Qualified cache policy per transport, affinity-aware route decisions, and evidence for supported effort changes within a continuing conversation. |
| [Usage and attempt evidence](usage-and-attempt-evidence.md) | **Substantial primitives.** Durable send intent, unknown outcomes, selected/observed identity, partial usage and replay rejection already exist. | Common observations across the intended gateway routes, including raw cache evidence, funding identity, decision inputs and reconciliation with financial owners. |
| [Research and compiled policy](research-and-compiled-policy.md) | **Supporting infrastructure.** Immutable research-use bundles, applicability state, task-host research seams and pure organizational compilation exist. | A traced, admitted connection from qualified inference findings to executable routing policy and subsequent outcome evaluation. |
| [Qualification experiments](qualification-experiments.md) | **Bounded fixtures and test sources exist.** Native IPC and public-stream tests cover important transport/failure properties. Current component documents retain explicit unfinished gates. | The proposed controlled cache matrix, real route/auth compatibility, and complete qualification of streaming, failover and recovery on the composed path. |

## The execution paths are distinct

The newer isolated native Pi path is closest to the proposed credential-free
worker boundary. [`pi_native::request`](../../rust/crates/work-engine-manual-host/src/pi_native.rs#L604)
bounds and validates the body, checks the selected model and allowed tools, and
reconstructs a request instead of forwarding worker URL, authorization, account
or arbitrary headers. The
[`pi-native.mjs` receiver](../../rust/crates/work-engine-manual-host/receiver/pi-native.mjs#L27)
uses native Pi session/runtime machinery with a nonauthenticating marker and
intercepted fetch. However, the
[probe](../../rust/crates/work-engine-manual-host/examples/pi_native_probe.rs#L87)
reads the complete synthetic HTTP body and returns it in one base64 frame;
the receiver then constructs a `Response` from that whole body. SSE-formatted
content here does not establish incremental host-to-worker streaming.

The older Pi coding provider is a separate subscription execution path. Its
[`ProviderPlan` and reservations](../../rust/crates/work-engine-manual-host/src/pi_coding_provider.rs#L26)
enforce narrow admitted configurations. Its
[authentication projection](../../rust/crates/work-engine-manual-host/src/pi_coding_auth.rs#L89)
stages short-lived access for the worker and refuses API-key auth in that
profile. This supplies useful admission machinery but does not establish the
newer worker's credential-free boundary.

Separately, [`bounded-inference`](../../rust/crates/bounded-inference/PUBLIC_HOST.md)
already owns an OAuth store, fixed public HTTPS policy, stream observation and
attempt journals. [`AuthStore::refresh_with`](../../rust/crates/bounded-inference/src/auth.rs#L960)
has a durable in-flight refresh transition and preserves ambiguous outcomes.
The public smoke action itself intentionally performs no automatic refresh,
retry, account switch or billing fallback. A refresh primitive is therefore
present even though automatic refresh in that action is absent.

This crate is already connected to retained-task execution:
[`compose_direct`](../../rust/crates/work-engine-task-host/src/lib.rs#L175)
constructs `DirectAdapter<PublicHostEndpoint>` under explicit startup selection.
Failure is not a fallback to native. Its
[request builder](../../rust/crates/bounded-inference/src/request.rs#L63)
is a bounded tool-free, single-user-input profile, rather than Pi's multi-turn
coding request with tools and history. The inspected manual-host dependency
and module declarations do not connect its native Pi transport to this adapter.

There are also existing native Codex auth realizations, including an explicit
[runtime API-key mode](../../rust/crates/lifecycle-codex/src/transport.rs#L288).
Thus “no API-key support anywhere in Rust” would be incorrect. The missing
capability is common governed resource selection across the four proposed
streams, not the mere existence of multiple credential mechanisms.

## Admission and financial evidence have different meanings

Pi's subscription provider makes an unknown reservation durable before granting
possible network entry. It checks the admitted provider/model, payer and effort
where applicable, context bound, prior unsettled requests and remaining local
budget. See the
[admission branch](../../rust/crates/work-engine-manual-host/src/pi_coding_provider.rs#L354).

Its [cost calculation](../../rust/crates/work-engine-manual-host/src/pi_coding_provider.rs#L197)
uses conservative maxima across input/cache rates and catalog tiers. Its
[completion branch](../../rust/crates/work-engine-manual-host/src/pi_coding_provider.rs#L473)
requires raw completed usage before releasing the full reservation. The field
named `settled_microusd` is a local catalog-based accounting result; that name
does not establish an invoice, actual remaining plan quota or external
financial settlement. Cache discounts do not drive this calculation.

Likewise, the public host deliberately returns an
[open funding obligation](../../rust/crates/bounded-inference/src/public_host.rs#L1540)
even after local completion. Its documentation distinguishes owner-supplied
funding assertions from independent observation of real provider billing.
The [session admission accounting reader](../../rust/crates/session-task-admission/src/accounting.rs#L76)
consumes a narrow measured-budget format; it is not a four-stream quota and
credit controller.

The substantial [attempt-accounting prototype](../../experiments/pi-attempt-accounting/README.md)
also deserves separate standing: its accepted bounded pilot includes an
independent Rust consumer, but it remains a standalone experimental snapshot
with broader financial qualification and review open. Its presence is not
evidence that a production gateway financial reducer is integrated.

The existing [attempt journal](../../rust/crates/bounded-inference/src/journal.rs#L11)
already distinguishes prepared, possibly-entered, terminal-observed and unknown
states. [Direct endpoint recovery](../../rust/crates/bounded-inference/src/public_host.rs#L1836)
uses retained evidence rather than treating an interrupted exchange as permission
to resend. Those semantics are useful foundations for any later route switch;
a second attempt must preserve the first attempt's uncertainty.

## Cache instrumentation is ahead of cache policy

[`TokenCounts`](../../rust/crates/bounded-inference/src/stream.rs#L16) already has
optional `cached_input` and `reasoning_output` fields.
[`parse_usage`](../../rust/crates/bounded-inference/src/stream.rs#L1195) reads
`input_tokens_details.cached_tokens`; missing/null counts remain `None`.
Therefore, the scratch concern about missing cache extraction applies to
particular Pi paths, not all current Rust inference code.

The older Pi helper's
[`rawTerminalEvidence`](../../rust/crates/work-engine-manual-host/receiver/pi-native-access.mjs#L19)
extracts input/output/total counts without nested cache details. Rust retains
SDK-normalized usage as JSON, but raw cache attribution and cache-sensitive
settlement are not established by that. The newer native receiver exports
message snapshots through a different path; changing the older helper would
not instrument it. Its
[synthetic SSE fixture](../../rust/crates/work-engine-manual-host/src/pi_native.rs#L699)
also supplies only input/output/total counts.

Both newer native receivers explicitly pass `cacheRetention: 'none'`:
[no-tools receiver](../../rust/crates/work-engine-manual-host/receiver/pi-native.mjs#L40),
[tools receiver](../../rust/crates/work-engine-manual-host/receiver/pi-native-tools.mjs#L48).
That is an observed setting, not proof that all upstream caching is disabled;
its effect needs qualification for each actual transport. The profiles also
fix effort rather than exposing a cache-qualified adaptation policy.

Outside dispatch,
[`work-engine-context-status::economics`](../../rust/crates/work-engine-context-status/src/main.rs#L325)
compares continuation with a fresh context using measured cached input and
declared assumptions about cache weight, fresh input and reconstruction costs.
Its [Claude exporter](../../rust/crates/work-engine-context-status/src/claude.rs#L270)
preserves known/missing cache-read and cache-creation counts. These are reusable
analytical and observation capabilities, not evidence that an inference router
uses them today.

## Research has a foundation, but the routing connection is missing

The [manifest compiler](../../rust/crates/work-engine-compiler/src/manifest.rs#L170)
projects authored model, reasoning effort and service tier into thread options.
The [organizational compiler](../../rust/crates/work-engine-organization/src/lib.rs)
is explicitly pure and effect-free.
[Research-use bundles](../../rust/crates/research-register/src/use_bundle.rs)
pin evidence and applicability, while explicitly separating structural validity
from scientific qualification and authenticated authorization. The
[task-host research seam](../../rust/crates/work-engine-task-host/src/research_admission.rs#L6)
also distinguishes controlled fixtures and staged installation from production
authority. These are useful boundaries to retain when connecting research to
inference policy. No consumer that turns qualified cache/quality findings into
ranked runtime inference candidates was established in this survey.

## Practical implications

My assessment is that the most useful next comparison boundary is one exact Pi
request traveling through host admission, credential access, incremental
response delivery and durable attempt observation. Existing owners provide
starting points, but their request profiles and evidence must be reconciled.
The native IPC isolation work and the public host's credential/stream/journal
work can inform that composition without creating a second accounting owner.

Consistent cache and route observations would then make the proposed resource
and optimization policies testable. Resource admission needs authorized funding
routes, fresh availability evidence and explicit failure behavior. Optimization
needs qualified quality/efficiency evidence and switch-cost estimates. The
fully deterministic optimizer remains a proposal; the repository's
[model-judgment contract](../../docs/contracts/system-design.md#9-determinism-and-model-judgment)
still governs where deterministic rules are appropriate.

This suggests focused composition and qualification before broad automatic
failover. It does not authorize resuming paused implementation or provider use.

## Evidence basis and limits

This is static repository analysis. Relevant source, dependency declarations,
component status documents and test bodies were inspected; tests were not
rerun, and no Docker run, provider call, credential access or cache experiment
was performed. The
[native integration status](../../rust/crates/work-engine-manual-host/docs/pi-native-integration.md)
records an unfinished, operator-paused offline implementation with the full
failure matrix, broad gate and whole-vertical review still incomplete. The
public-host document labels its own offline candidate and unestablished live
properties. Source presence does not close either qualification boundary.

Retrieval used repository-search Verify tier and Codebase Memory project
`home-bline-code-work-engine`, index generation `2026-11-15T18:23:32Z` / store
`u82e3c8cc8e0edbe8g2`, plus direct filesystem observation. All cited
implementation paths received coverage checks; inspected paths matched source
content, with unresolved call edges common. A native-request trace also
returned ambiguous standard-method matches, so material wiring was verified
from exact source and imports instead. No graph completeness claim is made.

Discovery covered relevant provider/auth/cache/attempt/routing symbols and
literal searches across Rust crate sources, including nested modules. The
survey concentrated on manual-host, bounded-inference, task-host, admission,
compiler, context-status, organization, research and experiment infrastructure.
The wider crate graph reports coverage gaps; “not found” here means no
integrated capability was established in those inspected paths, not a proof
that every alternate implementation or archived experiment is absent.

Provider pricing, eligibility, cache behavior and SDK claims in the original
notes were not independently verified against external documentation for this
comparison. They remain hypotheses or qualification targets. The scratch
source and seven extracted topic documents were left unchanged.
