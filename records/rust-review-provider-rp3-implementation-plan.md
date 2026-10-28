# RP3: qualify one installed Claude initial review

## Status and decision boundary

Planning proposal, 2026-10-17. This document does not authorize code changes,
installation, credential access, a provider call, spending, activation, or a
retained-session entry. It proposes two separately decided steps: implement and
qualify the installed profile's **local** launch boundary without a real
provider call; then request an exact, bounded live-initial authorization only
if the local milestone below is observed and the accepting owner explicitly
accepts the remaining live-only uncertainty. This is a proposed correction to
v1's ambiguous “viable profile” gate, not evidence of full review-path fit.
If that owner requires completed review-path compatibility before any live
entry, the gate is unmet and the owner must approve a different qualification
route or amend that condition; the builder cannot waive it. SC3-H owns any
later retained inference. RP0's 4,096-request relay behavior and hard cleanup
deadline remain unestablished.

The baseline is the published RP1/RP2 implementation, including RP2 commit
`90edc6f268729b52cfdad4f420484ac90cfcf984` and its accepted offline
candidate `2d8099f6780f6c503374c58f4057df005fc3f087`. RP2 established
one local guarded peer, Applied-only initial entry, exact candidate selection,
checked custody and a responsive host under a held offline peer. It did not
install the protected production acceptance pin, read a production credential,
call Claude, validate today's installed CLI, or establish a live review.
RP2's historical helper overlay is qualification-only, not a pass on the
unmodified baseline. Publication's raw-index stat-refresh postcondition
remains unestablished; its committed source/tree and current projections were
verified separately. RP3 does not repair or reinterpret that publication.

The intended outcome is one selected immutable candidate and one fresh
campaign Applied initial entry, consumed by E1 through the permanent host.
E1 must record a real locally observed guarded launch, exact prompt/session,
bounded response and retained session snapshot. Cold recovery must join the
same checked E1 result through episode, campaign and claim owners (J1/J2), or
report a specific false, unestablished, refusal or unresolved outcome without
another entry. A syntactically valid Claude response alone is not completion.
No fixture root, test constructor, descriptive locator or recovery path can
mint production authority.

## Existing contract and actual gaps

| Owner | Current behavior | RP3 consequence |
| --- | --- | --- |
| `slice-campaign/src/installed_authority.rs` | A protected root-owned deployment registry selects a digest-pinned acceptance record. The record binds campaign/candidate, distinct claim pair, selected profile/program/guard/CBM/catalog/model, credential **metadata**, session/attempt, prompt template, strict resource policy and horizon. Only fresh Applied dispatch yields `FreshInitialEntry`. | The deployment owner must install a real selected record; startup JSON and the offline pin cannot do it. No general campaign-admission API is proposed. |
| `review-workflow/src/startup.rs`, `provider.rs`, `initial.rs` | Protected qualified startup loads pinned executable and boundary, validates credential metadata, opens the production pin, and installs authority. The workflow consumes Applied entry and calls E1 once, then joins checked E1/episode/CE/campaign owners. | Qualify one concrete installed startup and exact owner documents. Preserve the existing initial-only route and recovery-only replay behavior. |
| `review-execution-evidence/src/claude/credentials.rs` | Startup identifies a canonical user-owned mode-0600 regular source without reading its bytes. After Applied entry and preflight, E1 reopens the exact file identity, reads at most 64 KiB, extracts only `claudeAiOauth.accessToken` and checks expiry beyond the horizon. | The credential owner must approve that exact source and execution-time read. A metadata digest proves file identity, not authentication success. Do not persist or echo token bytes. |
| `review-execution-evidence/src/claude/isolation.rs`, `launch_guard.rs`, `session.rs` | Guarded Bubblewrap launch mounts selected source read-only, gives Claude writable `/session` plus four small tmpfs mounts, and enters a cgroup before exec. Guard checks realized memory and zero-swap policy before READY; E1 sends token/prompt only after READY. Active cgroup memory budget is the room left under the **same 64 MiB aggregate** custody limit and includes processes, tmpfs and kernel metadata. | No separate large provider runtime allowance exists. An installed Claude build may not start or finish under this cap. Fail closed; do not raise it by configuration or infer fit from metadata. |
| `review-execution-custody/src/claude/selection.rs`, `transport_validation.rs`, `record.rs` | Pinned CLI argv includes model, effort, fresh session, strict MCP and JSON schema. Response validator checks successful JSON, exact session, one allowed observed model, one matching `modelUsage` key and episode result/subject. Observation records guarded local realization and `claude-cli-json-v1`. | This proves checked local CLI output and selected model claims, not independent remote serving identity, paid route, charge ceiling or provider settlement. Preserve that evidence limit in acceptance and receipts. |
| `work-engine-review-host/src/control.rs`, `worker.rs` | One worker owns startup/recovery and operations while status is serviced independently; stop closes admission without cancelling in-flight work. | Keep status useful during the live attempt and after failure. Do not make status, stop or restart a new entry route. |

The current sandbox binds `/usr`, `/lib`, `/lib64`, `/etc/ssl` and the pinned
program under `/claude/bin`; it clears environment and passes only the access
token to the child. Those mounts and flags are source facts, not proof that a
selected current CLI can start, resolve its endpoint, authenticate, reach its
intended service, stay within memory, or produce the expected JSON. The current
response validator does not enforce a cost field or a remote route receipt.
The first implementation phase can measure only the local facts it actually
exercises without a production credential or provider call. Network,
authentication, structured-result and full review-path memory remain unknown
until an authorized live attempt or another owner-approved equivalent proof.

## Owner selections required before implementation is called live-ready

The deployment/campaign owners supply an exact immutable candidate and source
tree, accepted boundary, existing distinct builder/campaign claim pair,
production grant and completion authority, selected schema-4 campaign and
native episode roots, frozen gate inputs, compiler/role/request inputs and
prompt template. The selected checkout must equal the Git tree byte-for-byte
and mode-for-mode before CBM indexing, at host startup, and before launch.
Every root is new; a preexisting root is recovery territory, not a replacement
or fresh attempt. Protected startup/registry/acceptance are installed by the
deployment owner through their own procedure, not generated by a test helper.

The provider owner separately selects one exact installed executable digest,
guard and runtime dependency closure, a supported direct Claude CLI profile,
catalog and observed-model allowlist, requested model/effort, fresh native
session ID, provider horizon (at most 600 seconds), and the credential owner
and canonical source path. No model is chosen by this plan. The credential
owner selects the access-only credential format and authorizes an
execution-time read for this one attempt; preflight inspects metadata only.
The model/provider owner must decide what evidence suffices for a **direct
local CLI route** and what cannot be inferred about the remote upstream. If
their required claim needs independently observed upstream identity, introduce
and review a bounded route-observation mechanism before live admission rather
than labeling the CLI's own JSON as that observation.

The spend owner supplies a numeric maximum exposure and a mechanism capable of
enforcing it for this selected CLI/account before any live call. The present
argv and response parser impose no hard spend ceiling: the 600-second horizon,
one host dispatch, and one top-level CLI invocation do not bound the CLI's
internal provider requests or bill. An account-side hard limit or a verified
CLI budget control may be proposed and tested, with its exact semantics and
remaining race/overrun described. If no enforceable upper bound is available,
the live step is blocked for an owner decision; an estimate is not a hard cap.
The selected control and maximum become protected, digest-bound admission
inputs, not protocol caller fields. The owner also sets a no-retry decision:
no second Applied attempt after uncertainty or remote failure in this slice.

## Proposed implementation sequence

1. **Freeze an installed feasibility packet without secret reads.** Record
   hashes, owner/mode and nonsecret metadata for the selected binary, guard,
   Bubblewrap/CBM pieces, host release and dependency closure; identify the
   selected CLI's exact invocation and budget-control contract. Check the
   selected Git tree and private CBM index, protected registry/startup and
   accepted record, credential metadata digest/expiry *requirement* and fresh
   root identities. Do not execute the provider binary in this planning step.
   In separately authorized implementation, use a distinct, non-authorizing
   local probe for the selected binary only after the reviewer namespace has
   `--unshare-net` installed and verified **before any installed child exec**;
   clear proxy/cloud environment, expose no outbound socket, use a synthetic
   canary token and prompt, and retain the production guard, executable,
   command flags, mounts, cgroup/zero-swap policy, READY framing and bounded
   process capture. The local probe has no campaign Applied permit or E1
   production writer. If those egress conditions cannot be enforced and
   witnessed, do not start the installed program; metadata alone leaves local
   launchability unknown and blocks the proposed live packet.
2. **Close only demonstrated installed-profile gaps in their owners.** E1 owns
   launcher/guard, credential projection, bounded capture, session custody and
   response/observation validation; custody owns shared immutable profile and
   checked reader contract; workflow owns qualified startup and join; campaign
   owns protected acceptance and fresh Applied entry; host owns control/status.
   Keep Rust-first changes narrow. Test same production validation core with
   synthetic credentials, false files, wrong model/session/subject, changed
   dependency, secret-in-output, malformed usage, timeout/OOM, cgroup or swap
   unavailable, and corrupted/replaced custody. Do not introduce a fixture
   conversion to `QualifiedProvider` or a generic provider runtime. Any
   new budget/route fields require explicit versioned acceptance/startup,
   E1 prepared/reader and receipt bindings with old records read on their
   original terms.
3. **Establish the narrow local milestone under the unchanged policy.** On an
   isolated selected installation, verify realized `memory.max`,
   `memory.swap.max=0`, writable mount/inode/data bounds and network-namespace
   denial before guard READY. Then deliver the canary token/prompt frame and
   require a post-READY witness that the guard successfully spawned the
   *selected executable with the selected review argv* inside the private
   cgroup. Record child PID/executable identity, frame acceptance, terminal
   status or timeout, bounded output, cgroup peak/OOM events and mount/network
   facts. READY by itself is insufficient: current guard emits it **before**
   reading frames and before `Command::spawn`. `--help`/version, a static
   digest or an authentication refusal without the post-spawn witness does not
   satisfy this milestone. Record any actually reached MCP handshake, session
   file or JSON-parser stage separately; do not infer them from spawn. The
   canary and network denial differ from live credential and egress, and an
   early refusal may precede real tool use. Thus this milestone proves local
   selected-argv spawn and early bounded behavior only. Full prompt/review
   memory, authentication, network reachability, MCP use, structured response
   and remote billing remain unestablished. If spawn OOMs or the cap prevents
   the milestone, this selected profile is locally unsupported. If the full
   review later exceeds the cap, the live outcome is a bounded failure; no
   automatic second entry follows. A successor policy requires a separate
   design/owner approval: independently enforce the original 64 MiB aggregate
   retained/live session storage while choosing and enforcing a separate
   process-memory ceiling, zero swap and accounting relationship; rerun
   hostile capacity, crash and durability proofs. Merely changing
   `memory.max` or a policy string is rejected.
4. **Freeze an implementation candidate and requalify affected paths.** Run
   default and fault-enabled debug/release E1, campaign, workflow and host
   tests, restart/cut tests at pre-dispatch, Applied, intent, READY, prompt,
   capture, snapshot, J1/J2 and claim publication boundaries, same-core
   production validation tests, Clippy/format and affected downstream
   episode/CE regressions. Preserve raw logs, source/lock/toolchain/binary
   hashes and failures; distinguish historical helper-overlay gates from
   exact unmodified-source passes. Check host status under one held operation,
   including errors/timeouts/busy and raw latency samples. Offline mechanism
   gates and the installed local milestone cannot be labeled full live
   viability or remote provider proof. Request the accepting owner's explicit
   decision on those unexercised facts before preparing the live envelope.
5. **Request a separate live authorization** using the exact envelope below.
   After approval, install the protected owner selections, perform a final
   read-only preflight and make at most one fresh Applied initial attempt. E1
   records intent before launch, guarded READY before token/prompt, bounded
   stdout/stderr and exact response, checked session snapshot, reader readback
   and immutable observation. Workflow then joins episode/CE/J1/J2 using the
   exact E1 reference. Stop admission and recover the original owners on
   restart. A timeout, missing capture, ambiguous spawn or downstream failure
   stays unresolved or unestablished; no automatic relaunch or second remote
   entry. A complete durable capture may be finalized by recovery without a
   new provider entry. Retained inference remains closed.

## Separately approvable live envelope

Before the live step, present one reviewable packet with: exact candidate
commit/tree/patch identity and source archive; campaign/episode/claim/E1 root
IDs and paths; accepted source and distinct claim IDs; host release, protected
startup, deployment registry and acceptance record hashes; profile/program,
guard/CBM/catalog/dependency hashes; selected model, effort and allowed
observed aliases; direct-route evidence policy; credential source identity and
named owner approval for the one execution-time read; exact prompt-template
digest and permitted candidate information exposure; one fresh session/attempt
ID; horizon ≤600 seconds; numeric maximum spend and proven enforcing control;
the local milestone witness and its live-only unknowns; the accepting owner's
decision to admit this first live attempt despite those unknowns; and a
single-attempt/no-retry rule. Keep token bytes, refresh credentials and
secret-bearing environment values out of the packet and logs. State whether
the credential expiry safely spans the selected horizon. Include stop and
recovery contacts/owner and the exact evidence roots to inspect afterward.

Approval of an implementation candidate does not approve this live envelope.
If any field or protected installation changes, freeze and reassess before
Applied dispatch. A failed preflight may be retried only before entry under
the same approved packet; after intent or uncertain entry, reconciliation
supersedes another launch. Because a remote provider may still charge despite
a lost local response, report local attempt and spend observations separately
from any upstream settlement fact.

## Acceptance and stop rules

Implementation acceptance requires exact current source closure, protected
installed authority checks, synthetic same-core security regressions, a
realized bounded sandbox and the precise post-READY local spawn milestone in
step 3. It does **not** establish full review-path viability under 64 MiB.
A selected profile that cannot meet even this local milestone is blocked for
live admission; passing it leaves the enumerated live-only facts unknown. The
accepting deployment/campaign owner decides whether a separately authorized
first live attempt may test those facts under the fixed 64 MiB cap. If the
owner's acceptance condition instead requires full review-path viability
before live entry, it remains unmet until the owner approves a different
proof or explicitly changes that condition. Neither path relaxes RP1. If no
hard spend control or sufficiently honest direct-route observation is
available, stop before live Applied entry.

The proposed implementation review class is a fresh read-only Codex Astra/high
adversarial review of a Sol/high implementation candidate, with exact source
and gate packet plus retained remediation. It is **same-provider,
different-model evidence**, and no independence is claimed. The present
Astra/high plan review has that same evidence class and reviews this plan
only; it cannot accept a future implementation. The accepting owner must
select this class for implementation when authorizing that work or name a
different admissible class. If the owner requires genuinely independent
review, define the independence property and admissible reviewer/provenance
at that decision; that gate remains unmet until such evidence is supplied.
No reviewer or builder can convert same-provider advice into independence.

Live qualification, if separately authorized, requires a checked fresh
Applied entry, one observed local provider invocation under the selected
budget, complete E1 capture and retained snapshot, production-class E1 reader
readback, exact J1/J2/episode/CE/campaign refs and recovery with no duplicate
entry. Record requested and observed model separately, CBM observation,
resource/cleanup facts, local route evidence and its remote limits, host status
samples, charge information and unresolved effects. A valid provider result
with false or unestablished CE outcome is a truthful result, not success by
fiat. No SC3 retained entry, migration, activation, credential refresh or
broader campaign follows from a passing RP3 live initial.

## Review packet and limitations

The private RP3 planning packet freezes graph query/coverage results, exact
source hashes and this plan. Graph coverage is best-effort and a clean result
does not prove completeness. The current code's provider observation is a
local CLI claim; independent remote identity and a hard spend ceiling are
open owner/implementation questions. Installed binary fit, authentication,
network path and real model response have not been tested. No production
credential, provider, model-selection or live authorization was used to
prepare this plan.
