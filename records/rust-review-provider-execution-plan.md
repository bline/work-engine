# Permanent Rust reviewer execution: bounded successor to H1

## Status, authority and recommendation

Planning only, 2026-10-13. J2/H1 originated at
`c4952af0542763de365ead11ba69d70a1347d831` (H0 parent `7068bb0`);
the current qualified planning baseline is `7b39d6905d7d2c120b182daec2b92914a5e728f1`.
This document proposes a permanent Rust **direct Anthropic Claude Code** execution
profile for one initial implementation review, retaining the actual native session
for a separately admitted SC3 round. It does not authorize implementation,
credentials access, provider calls, spending, activation, migration or retirement.
The planning session performed source reads only; no tests or provider probes ran.
The original J2/H1 review reported `J2H1-R1` (derived native identifier
preflight) and `J2H1-R2` (all relied-on evaluation admissions on aggregate
recovery). Both fixes are committed in the current baseline. Combined repair
validation recorded 79 passing tests, zero failures and one existing ignored
test; the retained review verified both findings resolved with no new material
finding. The historical report remains at
`/home/bline/code/.work-engine-tmp/j2-h1-review-20261013/review-report.json`.
RP2 binds the exact accepted fix commit and does not reopen those settled
findings absent a changed premise.

Implement provider-specific execution and custody inside `review-execution-evidence`,
then compose it through the existing `review-workflow` and
`work-engine-review-host`. Keep campaign admission, episode transitions, claim
establishment and finding publication with their current owners. Do not insert a
Node shim, a lifecycle Codex substitute, a second orchestration loop, a permanent
proxy service, or a generic multi-provider runtime.

The smallest useful outcome is one exact immutable candidate reviewed by the
selected real provider, with independently observed read-only realization,
checked provider/session facts and durable production evidence that J1/J2 can
consume. A successful provider response alone is insufficient. An unsupported
realization or unavailable evidence remains unestablished. The accepted H1
same-UID controlled profile remains truthful: its `mutationAuthorized: true`
observation and resulting `EvidenceUnestablished` outcome are not defects.

## Current source contract and limits

| Owner / source | Observed behavior and implication |
| --- | --- |
| `app-server/reviewer-profiles.yaml` | Enabled native profiles select Anthropic Claude Code: `sonnet`/medium and `opus`/high, with immutable configuration digests. The OpenRouter Codex profile is separate. Profile presence is not permission to run it or proof of model identity. |
| `reviewer-runtime/native-claude-code-adapter.mjs`: `execute`, lines 509–666 | Native adapter binds profile/catalog, exact candidate/diff/evidence catalog, prompt digest and deterministic session UUID. It invokes Python `claude_transport.py`, then Claude `-p`, explicit model/effort, `--session-id` or `--resume`, strict MCP configuration, Read/Glob/Grep plus named Codebase Memory tools, JSON schema and skip-permissions. Rust must preserve the applicable semantics, not copy every implementation detail. |
| Same adapter: `validateTransportReceipt`, 111–144; `parseResult`, 255–266 | Requires one direct-Anthropic attempt, exact command/prompt/session/mode/model binding, no paid/batch/failover route, successful structured envelope and matching returned session. Observed model is distinct from requested alias; `upstream_provider_observed` remains false. This is harness/gateway provenance, not independent identification of remote serving hardware. |
| Same adapter: `#bindSession`, 337–384; `recoverResult`, 478–507 | Retains native session binding and artifacts. Legacy recovery searches successful transport receipts by mtime and takes a matching latest structured output. New Rust recovery must use exact registered attempt and artifact identities instead of importing this heuristic. |
| Same adapter: environment/authentication helpers, 31–109; [authentication record](../app-server/docs/native-claude-isolated-review-authentication.md) | Removes API/OpenRouter credentials and alternate routing, rejects cloud selectors, projects only a private canonical login access token plus expiry. No refresh token reaches reviewer configuration. Historical compatibility was measured on Claude 2.1.267; this does not qualify today's executable. Authentication status is not inference proof. |
| Same adapter receipt, 644–664; `native-review-closure.mjs`, 106–128 | Host receipt declares mutation unauthorized and converts its facts to a production-path observation. Requested CLI tools or self-described receipts alone do not prove the stronger enforced realization proposed here. |
| `review-execution-evidence/src/contract.rs`, `producer.rs`, `reader.rs`, `store.rs` | `QualifiedProvider` is a closed evidence class/profile name, but the concrete writer, root pin, event interpretation and reader are currently `controlled-process-v1`. A DTO/profile toggle cannot upgrade controlled evidence. Checked-result construction is crate-private. |
| `producer.rs`: `build_observation`, 579–700 | Controlled execution explicitly records ordinary process mutation access, fresh-initial continuity and controlled transport. Keep this behavior unchanged. |
| `process.rs`: `spawn` / `RunningChild::capture` | E1 owns one child, cleared environment, bounded stdout/stderr, five-second capture and two-second cleanup, durable launch intent and local failure facts. These limits and local-exit semantics are not a real review timeout or provider-settlement guarantee. |
| `claim-evidence/src/production_path.rs`: `evaluate`, 868–933 | CE checks subject, covered state, allowed mechanism/observer, realization, capabilities, continuity and artifact integrity. Mutation authorized makes the claim false; missing/mismatched evidence is unestablished. CE does not independently inspect sandbox syscalls or discover provider identity. Its trusted evidence producer must do that job. |
| `review-workflow/src/startup.rs`, 180–188, 363–388, 405–456 | H1 admits only frozen controlled SC0, manifest-only-unverified compiler mode, controlled custody and one production grant. A live profile cannot be enabled by replacing the peer executable. Profile, compiler and grant extensions require explicit bounded contracts. |
| `lifecycle-provider/src/lib.rs`; `lifecycle-codex/src/transport.rs` | Lifecycle port owns text-effect facts; Codex harness pins a Codex app-server binary/protocol. It does not implement Claude reviewer semantics or native reviewer-session continuity. S8 explicitly rejects silently converting same-native-session subjects into fresh-thread continuation. |
| `lifecycle-runtime/src/native_stdio.rs`, 1–140 | Useful local child-lifetime precedent, but JSON-RPC frame transport discards stderr. It is not directly suitable for exact Claude JSON output plus stderr custody. No shared-runtime extraction is needed for this bounded slice. |

The native profile's mutation and provider boundaries remain authoritative; the
new realization is an explicitly proposed strengthening with a candidate-bound
Codebase Memory surface, not a retroactive statement that legacy production
evidence was qualified by E1. No source or state retirement follows from this
plan.

## Selected bounded profile

Proposed identifiers are `review-host-claude-initial-v1` (composition),
`claude-readonly-linux-v1` (realization) and `qualified-claude-v1` (custody).
Acceptance selects one exact Claude executable/version/digest, runtime dependency
manifest, direct-Anthropic native profile and catalog revision, supported Linux
isolation implementation, and fixed model alias/effort. Recommendation: select
the existing `anthropic.claude-code.opus-review-v1` / `opus` / high profile for
this consequential first review, with its higher expected cost bounded by the
single-attempt horizon; Sonnet/medium is the lower-cost alternative if the
accepting owner chooses it explicitly. Do not infer model choice from the
builder's own model. No silent model, billing-route or provider fallback.

Keep the initial campaign boundary as small as H1: one admitted campaign,
obligation, initial attempt and immutable candidate, using a new named startup
profile and new private roots. SC0 fixture identifiers currently hard-coded by
H1 and campaign configuration are qualification machinery, not a general live
admission API. The campaign owner must explicitly install a bounded real-review
selection for the chosen candidate; do not present a fixture admission as live
authority. This is a prerequisite to the joined host slice, not permission for
arbitrary campaigns.

Proposed first tool surface includes Read/Glob/Grep and **candidate-bound,
read-only Codebase Memory**. Read/Glob/Grep see the host-prepared immutable
candidate tree and exact baseline diff/evidence catalog. The CBM MCP process
is pinned to the installed `0.11.0-945-g3d228b96-integ` family, with its exact
binary digest fixed at qualification. It runs `--tool-profile=analysis`, whose
current positive allowlist excludes `index_repository`, `delete_project`,
`manage_adr` and `ingest_traces`; Claude's `--tools` further names only
`search_graph`, `trace_path`, `get_code_snippet`, `check_index_coverage`,
`index_status`, `query_graph` and `get_architecture`. `query_graph` is read-only
Cypher and must have bounded results. Neither an MCP annotation nor Claude's
tool list is the isolation proof; RP0 verifies the process and daemon's actual
filesystem, network and project scope. No shell, writes, network tool, agent
delegation, hooks, plugins, editor integration, browser UI or dynamic MCP
configuration enters this reviewer profile. `search_code` is omitted because
its current implementation starts a grep subprocess; exact literals and
coverage fallback remain available through bounded Read/Glob/Grep. Gate
execution remains supervisor owned and supplied as exact evidence.

RP0 clarification: omitting `search_code` is a minimal selected-profile choice,
not a general ban on subprocess-backed read tools. A later tool expansion needs
its own boundary observation.

The trusted launcher first freezes the candidate source tree, host-computed
baseline diff, evidence catalog and gates. It builds a **new private CBM index
from that exact tree before reviewer entry**, then freezes the index and records
the candidate tree digest, project/root identity, CBM binary/profile digest,
index generation/store identity, build completion, watched/auto-index state,
and per-path source hashes/coverage for paths used as evidence. An incremental
index of an old project is not a valid substitute: this CBM release can leave
unchanged files on old extraction rules. The current main-project rebuild is
useful planning evidence but is not a candidate index or an admissible reviewer
source. A fresh private index is a proposed trusted preflight operation, not a
reviewer tool grant; failure to build or bind it refuses entry rather than
silently switching to an unbound graph.

RP0 clarification: the trusted build must quiesce the new SQLite store before
freezing it, reject WAL/SHM/journal sidecars, check the copied store's integrity,
then read back project/root/generation and coverage through the mounted review
copy. A digest of a live main-project DB or partial WAL copy is insufficient.

The private index contains exactly one candidate project. The launcher supplies
distinct private `CBM_CACHE_DIR` and `CBM_RUNTIME_DIR`, keeps the account's
normal daemon/socket and other project stores out of the sandbox, and disables
watching, automatic indexing and HTTP UI for this instance. The CBM daemon and
its subprocesses run under a narrower nested network-denied launcher while the
Claude harness retains only its selected inference network capability. They
also inherit candidate-only read mounts, private scratch and closed descriptors.
RP0 must prove the detached daemon stays inside that nested boundary; if the
installed platform cannot enforce it, the profile does not qualify. Current
CBM source shows the analysis MCP profile suppresses mutation tools and its
HTTP surface, while the runtime/cache selectors identify a separate
daemon/store. It does **not** prove
that a shared multi-project daemon could confine arbitrary `project` or Cypher
queries to one candidate. RP0 must observe the separate daemon's process tree,
endpoint, store inventory and effective mounts; any route to the shared daemon,
other projects, index mutation, UI, or host source fails qualification. This
uses CBM's existing profile and per-instance directories, with no new broker.

RP0 clarification: HTTP UI enablement is a separate per-instance daemon
configuration and listener question. Analysis-profile filtering alone does not
disable the UI. Pin the private instance's UI setting and observe no TCP
listener before and after the daemon's reload interval.

Graph output is untrusted retrieval evidence. New document `MENTIONS` edges
represent references, not behavior; diagrams, `DIAGRAM_ARROW` and OpenAPI
handler mappings describe source text, not executable flow. Missing or ambiguous
links and duplicate qualified headings remain limitations to resolve against
exact source. For material claims the reviewer records project/generation,
bounded query and
pagination, exact symbol/path, and `check_index_coverage` for every relied-on
path; partial, stale, skipped, excluded or unknown ranges require direct-source
fallback. Clean coverage means no recorded gap, not complete proof. A graph
result cannot replace the host's exact candidate-side citation catalog or
turn a path outside the candidate into authority. If the graph is unavailable
after entry, the reviewer may use the already granted immutable source for a
specific claim and report the graph limitation; missing required evidence or
undecidable behavior yields an explicit incomplete result. No provider identity
or execution-profile switch follows from using source fallback.

Role and result contracts remain canonical compiler/episode inputs. The permanent
host must bind the exact projected role instructions and verification status. It
must not mark H1's unverified manifest as verified. For this first profile, accept
an exact preverified compiler input/projection through the compiler's real owner,
or explicitly accept a profile requiring no unavailable verification claim. Any
required verification/AEG runtime dependency must be admitted before launch;
otherwise preflight refuses the whole operation. No temporary Python transport
is proposed. A compiled external dependency is not retired merely because this
profile does not use it.

### Enforced read-only boundary and observer

Recommend a pinned Linux namespace/mount sandbox launcher (a narrowly configured
bubblewrap realization is a candidate, not an already-qualified installation).
RP0 below must freeze its exact implementation and prove these properties before
it can generate admissible positive observations:

- Candidate/baseline evidence, instructions and required runtime files are
  immutable read-only mounts. The private CBM index is read-only during review,
  and its source root is the exact candidate. No checkout, parent Git directory, host home,
  production store, canonical credentials, control socket or arbitrary inherited
  descriptor is reachable. Preserve exact candidate and baseline hashes.
- Only a private session/configuration directory and bounded scratch are writable.
  Session output is not the evidence owner: the host captures/checks exact bytes
  outside the reviewer write boundary. No writable path resolves back into the
  candidate or owner roots through symlinks, hardlinks or inherited descriptors.
- The reviewer cannot reach host process memory, host `/proc` views, ptrace,
  namespace escape, privileged device access or ambient Unix sockets. Drop
  privileges/capabilities and prevent privilege gain; close unused descriptors.
  Missing platform enforcement fails before prompt release. A requested flag or
  a post-run unchanged-tree comparison is insufficient.
- The trusted launcher establishes and verifies the boundary before releasing
  the prompt. The host observes exact executable, effective mounts/access scope,
  disabled tool/config surfaces, CBM daemon/process and project inventory,
  environment-name allowlist, session directory identity and cleanup facts.
  The model cannot supply or overwrite this record.
  Record a versioned realization digest and the decisive observed facts, not
  merely `readonly=true` supplied by the caller.
- Network is a capability of the trusted provider harness for its selected
  inference route, not a reviewer-controlled tool. Validate direct routing,
  trusted harness/configuration and absence of other remote tool surfaces;
  the CBM daemon and subprocesses have no network access. Do
  not claim a cryptographically verified upstream provider or arbitrary-host
  network isolation from a CLI model field. If the accepted claim requires
  endpoint-level confinement or provider attestation beyond this trusted-harness
  boundary, it needs an additional accepted realization before launch.

Threat boundary: the model and its granted tools cannot mutate reviewed state or
forge the independent host observation; the pinned provider harness, launcher,
OS and trusted Rust composition are in the trusted computing base. This does not
claim resistance to a malicious host administrator, kernel or compromised
credential-consuming harness. Protecting arbitrary linked Rust code from itself
is also outside H0's admitted trust boundary.

The positive observation is emitted only after the exact result and realization
artifacts are durably captured and checked. Failure to observe a required fact
produces absent/unestablished evidence; observed mutation authority remains false
claim evidence. New selected claims must name their actual covered state,
consumer, mechanism, observer, realization and continuity. Never weaken old
claims to fit available evidence, or overwrite an H1 false observation.

### Credentials and secrets

Retain access-only projection, with the canonical credential owner outside the
reviewer. Open the exact installed private regular file without following a
replacement symlink; check ownership/mode and validate one coherent bounded
access-token/expiry snapshot. Do not copy or expose refresh material, run login,
refresh, or mutate the canonical file. Reject alternate gateway/API billing
selectors and construct an allowlisted child environment from scratch.

The selected Claude harness receives the access token through its qualified
supported mechanism; the first candidate is the existing access-token environment
projection. The model's read surface must not expose environment, process memory,
canonical credentials or secret-bearing config. Qualify this concretely in RP0.
The host records mechanism, expiry and a non-secret owner revision when available;
no token, token hash/fingerprint, raw credential file, secret-bearing command or
unfiltered environment enters logs, receipts, prompts or source. Redaction alone
is not a substitute for preventing those writes. Bound and sanitize diagnostics;
retain raw provider streams only under the explicitly admitted private custody
policy and prove the selected harness does not emit credentials there.

Expiry must cover the installed attempt horizon; otherwise refuse before entry.
Expiry, quota and authentication failure are distinct observations. An operator
may advance the canonical credential tip outside this slice. Recovery never
refreshes it or retries an inference. A full credential broker, concurrent refresh
protocol and multi-reader rotation service remain deferred.

## APIs, state ownership and no-repeat execution

Names below are proposed Rust APIs, not claims that they exist.

Inside `review-execution-evidence::claude`:

- `TrustedClaudeProfile::load(installed_selection)` validates immutable executable,
  catalog, tools, isolation, credentials-source metadata and custody profile.
  Private fields; no deserialization into an admitted capability.
- `QualifiedScope` binds existing `ExecutionProvenance`, exact request and
  candidate/selection/claim digests, prompt and boundary catalog, native session
  identity, candidate CBM project/build/generation/coverage manifest, realization
  selection, budgets and evidence root. Its trusted
  constructor checks the installed profile; it cannot upgrade `ControlledScope`.
- `QualifiedWriter::{create,resume,execute_initial,reconcile}` and
  `QualifiedReader::open` retain E1's single execution owner and private checked
  result construction. Reuse immutable artifact/event/root mechanics through
  sealed profile dispatch; keep old controlled marker/schema semantics intact.
  `resume` and `reconcile` are observation-only and have no provider launch path.
- `RetainedReviewSession` is a private-field, non-Clone, non-Serialize/
  non-Deserialize checked capability from protected session-owner readback.
  `SessionLocator` is a serializable descriptive coordinate. Loading a locator
  never grants dispatch. The owner validates session/root/profile/custody and
  observed session response before issuing the capability.
- A per-attempt interrupt handle accepts only the registered active attempt and
  reports local signal/reap facts. It cannot declare provider cancellation,
  request non-entry, refund, retry safety or downstream review completion.

Workflow still owns the campaign dispatch permit and private launch branch.
Only `Applied` authorization may enter; replay, serialized requests, reconstructed
locators and successful owner reads cannot create a new launch capability.
E1 owns execution custody, not campaign admission. Trusted in-process composition
is deliberate; no public JSON constructor is an authority channel.

Before entry, durable records bind campaign root/obligation, episode, selection,
provider profile/catalog, candidate/request/prompt, execution-attempt ID, original
dispatch operation/revision, session UUID, realization and credential metadata.
Reserve result/failure space before starting the process. Record launch intent
before a syscall capable of starting the harness; never retry from intent alone.
Release prompt bytes only after isolation verification. Distinguish local child
start, prompt submission, any provider-reported entry and final response.

The no-repeat guarantee here is one host-authorized launch/prompt operation, not
exactly one upstream HTTP request or model invocation. A provider CLI may retry
transport internally; freeze and report its supported retry configuration and
observations. A local entry counter cannot prove remote at-most-once execution.
If the owning claim requires that stronger property, an admissible provider
idempotency/operation ledger or a separately qualified no-retry contract is a
prerequisite. This slice neither claims such a ledger nor retries uncertainty.

Record requested alias/effort separately from all observed model identifiers,
requested session/mode separately from the returned session, and configured
gateway separately from provider-reported identity. No successful envelope may
hide mixed/missing/unacceptable model identities or a session mismatch. Require
exact structured result subject and canonical episode validation; malformed
output requests correction only when a later owner authorizes it, never a hidden
second call.

Evidence includes bounded raw response and diagnostics under private custody,
parsed result, command/prompt/configuration digests, observed identity, exact
session artifact manifest, host-observed CBM process/project/index binding,
reviewer-attributed query/coverage references where a claim relies on them,
realization observation and local process facts. The host does not claim to
have witnessed every MCP query merely because the reviewer reported it.
Session directory modifications between turns are legitimate provider state;
freeze immutable per-attempt manifests and preserve predecessor links rather
than pretending one growing directory has a permanent digest. Original result
and admission evidence remain immutable.

| Recovery cut | Permitted consequence |
| --- | --- |
| Before dispatch or definite local preflight rejection | Report exact state; no call. A future entry still needs fresh owner authorization. |
| Dispatch committed, launch token lost; intent without a witnessed terminal result | Unresolved original attempt. Do not respawn, infer non-entry, or mint a replacement UUID. |
| Child/prompt entered then interrupt, EOF, timeout or host death | Record local facts and reconcile exact original artifacts. Remote execution may remain unknown. No automatic inference retry. |
| Complete captured response, terminal evidence not committed | Verify and finish immutable evidence publication from the original capture; do not run the provider. |
| Evidence committed, CE/episode/J2 incomplete | Use existing J1/J2 exact owner operations/recovery; preserve original result, observation, admission and receipt identities. |
| Session lost, changed or inaccessible | Retained continuation unavailable/unresolved. No reconstruction labeled same-session and no fresh fallback. |

Use one workflow worker and the H1 control runtime. Add exact-attempt interrupt
and bounded provider status without placing provider work on the control runtime
or adding an unbounded queue. Caller disconnect does not authorize another entry.
Proposed first budgets: one in-flight attempt, no mutation queue, 10-minute
provider horizon, 5-second local cleanup; retain H1's 64 KiB command/status,
1 MiB result/observation and 4 MiB stream bounds. Reserve a bounded 64 MiB custody
budget plus an explicitly bounded retained-session budget (proposed 64 MiB for
this one initial attempt). Oversize refuses or ends with truthful capture failure;
never truncate into success. RP0 measures exact harness fit before freezing these
limits. Lower operational timeouts are configurable only within the accepted
profile and do not alter no-repeat semantics.

## SC3 handoff

Shared with [SC3 remediation planning](rust-campaign-sc3-remediation-plan.md):
SC3 owns replacement-subject preparation, independently admitted successor claim
selection and new per-round execution-attempt/operation identities. The campaign
and original episode identities remain; continuation uses the **same observed
native reviewer session**. `prepare_remediation` yields a descriptive locator;
`authorize_remediation_dispatch` yields a fresh nonclone/nonserde permit only on
Applied. This provider owner consumes the privately composed launch authority
plus checked `RetainedReviewSession`; recovery never enters that path.

The request/evidence binding includes replacement candidate, exact remediation
request, predecessor initial execution/session manifest and result, same observed
session, round attempt, selected retained-continuity claims, dispatch operation
and revision. Requested `--resume` alone is insufficient. UUID equality and protected local session lineage establish the selected native-session contract; they do not prove an unchanged remote model context or prohibit provider-side compaction. Missing or changed
session evidence blocks usable continuation; no lost-session replacement,
correction or episode terminalization is included in the first provider slice.
SC3-H binds a **new** private CBM index generation to the replacement immutable
candidate. The initial candidate's graph remains historical evidence; it cannot
answer changed-tree claims. The same isolation and coverage rules apply on the
retained call without treating fresh graph retrieval as a fresh reviewer session.

SC3 reports that episode remediation retains old admission bytes; a result with
no fresh admissions becomes `stale_required_admissions`. Therefore initial
fresh-continuity claims cannot be reused for the changed subject. SC3 proposes
new exact claim documents/establishments with campaign-owned predecessor links,
not a general CE succession implementation. This plan relies on that separate
owner decision and does not implement campaign or CE reducers.

## Bounded implementation slices and owned paths

| Slice | Proposed ownership / APIs | Exit evidence |
| --- | --- | --- |
| RP0 — qualify one realization, provider disabled | New `review-execution-evidence/src/claude/{mod.rs,profile.rs,isolation.rs,credentials.rs}`; `tests/claude_boundary.rs`; `docs/rust-review-provider-execution-validation.md`. Freeze installed launcher/CLI/CBM configuration contract and threat boundary; no production provider call. | Actual isolated hostile reviewer and CBM peer cannot mutate candidate/index or reach owner roots/secrets/host processes/network, other projects or the shared daemon; no CBM HTTP UI, indexing, trace injection, editor or browser path. The trusted preflight builds and binds a private candidate index; independent observation checks process tree, mounts, sockets, project inventory, tool allowlist and negative escape cases. Missing enforcement refuses before prompt. A failed premise remains failed rather than becoming flags-only evidence. |
| RP1 — exact Claude transport, custody and session | Add `src/claude/{transport.rs,session.rs,writer.rs,reader.rs}`; bounded edits to E1 `lib.rs`, `contract.rs`, `store.rs` and crate manifest; new versioned profile-specific migration only if required; `tests/{claude_execution,claude_recovery,claude_custody}.rs`. | Provider-disabled adversarial peer exercises exact envelopes/identity, persistent session manifests, CBM candidate/index/query/coverage binding, hostile `project`/Cypher/path requests, graph/source disagreement, credential canaries, bounded output, interrupts, crash cuts and no second entry. Qualified checked-result creation requires verified profile-specific custody; controlled evidence remains inadmissible to this reader. |
| RP2 — permanent host join, initial only | `review-workflow/src/{startup,initial,recovery,protocol}.rs` plus new `provider.rs`; `work-engine-review-host/src/{control,worker}.rs`; crate-local manifests; focused `tests/qualified_initial.rs` and `qualified_recovery.rs`. Minimal Cargo workspace/lock delta under coordinated owner. | New accepted startup/selection/compiler/grant profile, actual permanent binary, J1/J2 full completion, exact recovery, findings grant only where J2 publication needs it. The real initial call uses a fresh accepted schema-4 campaign root so its retained session can later enter SC3; RP2 issues no remediation permit or SC3 command. |
| RP3 — separately authorized bounded live proof | Validation artifact and installed exact configuration, no new feature scope. | One approved initial real review through the qualified permanent binary, actual model/session/route, independently observed boundary and CE/episode/J2 outcome. No automatic corrective/retry call. A later explicitly admitted same-session SC3 call is the continuity proof, not a second initial review. |

RP2 needs a narrowly owned campaign configuration/profile addition for real
admission, coordinated with SC3-D's fresh schema-4 initializer and round
contract; exact candidate/selection/grant APIs must be frozen together before
either owner edits `TrustedConfig`. SC3-D can be built and tested without RP0/1
or live provider use, but the **joined** RP2 initial call intended for later
SC3 continuity must start in a fresh schema-4 root. It must not silently broaden
`TrustedConfig::controlled_sc0`, accept arbitrary wire authority or use synthetic
CE grants. If the positive result needs finding publication, install the actual
bounded finding grant and J2 bridge; H1's production-only grant cannot stand in.
No change to CE establishment semantics or episode schema is preauthorized by
this provider plan. The SC3 owner handles its separately planned schema changes.

Do not edit lifecycle/S8/S9/UI sources. Reuse existing E1 storage/integrity
mechanics where their invariants match, and retain its controlled profile; avoid
an independent provider journal competing with E1. The generic `NativeStdio`
helper is a considered but rejected direct reuse because it loses stderr and
assumes framed I/O. Any later genuinely common primitive extraction requires its
owner and evidence of shared semantics, not symmetry between providers.

## Qualification and final receipt

Provider-disabled gates use actual permanent Rust processes and the actual
isolation launcher, with an instrumented executable in place of Claude. They
establish adapter/boundary/recovery behavior only; they cannot establish live
Claude compatibility, authentication, serving identity or same-session inference.
Test peers use test-only installed profile/root identities and cannot satisfy the
production binary pin or manufacture a production `QualifiedProvider` receipt.
Tests may exercise the same verifier logic without promoting fixture provenance.

Required cases: executable/catalog/profile/CBM binary or index drift;
requested/observed model or session mismatch; absent/mixed model identity;
extra tools/hooks/MCP/alternate
route; malformed/noncanonical/oversized result; changed subject/prompt/diff;
credential expiry/unsafe file/symlink/race/secret-canary disclosure; write/path/FD/
process escape; CBM cross-project, stale-generation, incomplete-coverage,
hostile Cypher/path, daemon/UI/network/index-mutation escape; graph/source
disagreement and direct-source fallback; missing realization observation;
evidence/root/session replacement;
controlled evidence presented as qualified; every durable publication crash cut;
interrupt before/after prompt/result and client disconnect; lost session; CE and
episode recovery without a second execution. An independent peer-entry witness
counts zero/one starts across repeated recovery. Do not derive this count from
the same journal under test.

Run affected E1/H0/episode/campaign/J1/J2/H1 regressions, default and optimized
fault-enabled cases serially, all-target Clippy and formatting on a frozen exact
Cargo subject. Repeat H1's external responsiveness measurement with a held
provider, domain contention, cold reconstruction and four simultaneous mutation
requests (one admitted): 700 status probes, report p50/p95/p99/max, timeout/busy/error
counts, actual provider duration and exact control binary. Proposed gate retains
p99 <= 50 ms, max <= 250 ms, no status timeouts, one entered operation and honest
busy responses; unrelated setup/compilation is excluded. A fast mock completion
is not the responsiveness workload.

The live proof is separately authorized with exact profile/model, immutable
candidate/data exposure, credential owner, horizon/cost bound and stop condition.
Record actual observed provider facts and whether both required claims are
established; a successful JSON response with unestablished claims fails that
acceptance gate truthfully. Do not grant review acceptance or source retirement
from this proof. Publish source/toolchain/lock/features/executable/launcher/profile
identities, raw nonsecret logs and exact owner receipt locators. Distinguish
offline qualified mechanisms from provider-observed live evidence.

## Decisions still owned outside this plan

1. Accept the first real profile's exact native provider/model and the proposed
   candidate-bound CBM plus immutable source surface. Recommendation: include
   graph access from the initial call because it supplies structural navigation
   and the installed analysis profile has a bounded read-only tool set. The
   price is a separately isolated daemon/store and a trusted index-preflight
   manifest; RP0 must demonstrate those facts. A proven inability to confine
   the daemon, its source/index, project queries or UI is a failed profile
   premise requiring an explicit revised decision, not a blanket convenience
   deferral. Existing profile registration does not settle the decision.
2. Accept the trusted-harness Linux boundary and pin a realization that actually
   satisfies it. Recommendation: use the pinned namespace/mount realization
   qualified by RP0, including nested CBM network denial. This is smaller than
   a remote attestation or endpoint proxy, but cannot establish stronger
   upstream identity, cross-host resistance or endpoint confinement; require
   those only if the accepted claim actually demands them.
3. Accept the joint SC3-D/RP2 checkpoint: fresh schema-4 roots only, immutable
   changed candidate tree and gates per generation, owner-accepted exact initial
   and per-round successor production claim pairs, retained-continuity profile,
   and private session plus fresh Applied campaign permit. Install the bounded real initial selection,
   compiler verification basis and grants through their owners. This lets RP2
   run the initial call in a future-remediable root while remaining initial-only;
   SC3-H alone enables retained entry. Schema-3 roots are not migrated and SC0
   fixture authority or stale initial claims are not reused.
4. Authorize RP3's concrete live proof separately. No credential read, provider
   request, login, refresh, deployment or retirement is authorized by planning
   or provider-disabled test acceptance. Recommendation: one exact initial
   bounded call after RP0–RP2 qualification, then stop for its observed facts;
   the separately admitted SC3 call proves continuity later. This caps paid
   exposure while leaving live compatibility unestablished until that call.

These are product/profile or live-execution decisions, not an added confirmation
requirement for ordinary implementation choices once the corresponding scope has
been accepted.

## Evidence scope and limitations

Tier 2 Verify, graph project `home-bline-code-work-engine`, generation
`2026-10-13T04:35:20Z`, store `u54f01405f69da80dg2`, watched Git every 10 seconds.
Exact native-adapter file query returned all 41 symbols; lifecycle-provider 23,
lifecycle transport 15. Broad reviewer/realization discovery was candidate-only,
not exhausted and not used for absence claims. Native adapter `execute` and E1
`EvidenceWriter::execute` were traced in both directions, depth one, complete
reported pages; dynamic/cross-language edges can be incomplete or misresolved,
so exact source supports semantic conclusions. E1 discovery's final six symbol
rows were unnecessary for these positive source-backed claims; no whole-module
call-graph completeness is claimed.

Coverage check consumed all 22 relied-on source/configuration/plan paths and
reported `content_verified: true`, no recorded gaps, at that generation. This is
best-effort, not proof of completeness. Direct source reads supplied exact
configuration values, process behavior and the reader/store restrictions that
symbol-only results cannot establish. Evidence capture and source SHA-256
manifest are under `/home/bline/code/.work-engine-tmp/review-provider-plan-20261013/`.

This revision also checked the rebuilt main graph generation
`2026-10-13T06:06:14Z` / store `ue03742e60f24fcbeg2`, the installed CBM
`0.11.0-945-g3d228b96-integ` help/config and bounded CBM implementation source.
The analysis profile is a positive MCP allowlist and disallows its HTTP surface;
`CBM_CACHE_DIR` selects the store and `CBM_RUNTIME_DIR` selects daemon rendezvous.
The current main index reports 2,545 document `MENTIONS` and 1,569 ambiguous
unresolved references; those counts are descriptive, not Rust behavioral edges.
The nine revised-plan/doctrine/adapter/contract/startup paths checked against
the new generation before these edits were content verified with no recorded
gap. Evidence notes
are under `/home/bline/code/.work-engine-tmp/rp-sc3-cbm-plan-20261013/`.
No candidate-private daemon/index or installed sandbox/CLI compatibility,
credential state or runtime behavior was probed in this planning turn. RP0 and
the separately authorized live proof must resolve those explicit uncertainties.
