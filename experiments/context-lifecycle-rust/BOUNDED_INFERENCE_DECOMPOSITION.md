# Bounded inference: decomposition and first dispatch

**B0 candidate, 2026-10-21.** This is the user-requested decomposition of the
[bounded inference plan](BOUNDED_INFERENCE_IMPLEMENTATION_PLAN.md), prepared for
the coordinating root's integration. It identifies the work that can start
offline and the exact shared boundary that still needs its owner's decision.
It does not mark the original plan, the [post-S10 amendment](POST_S10_BACKEND_ABSTRACTION_AMENDMENT.md),
S10, provider qualification, or production migration accepted.

The input plan is SHA-256
`d2f084ea491d3183230d952271c7985fb7b57bb980efaa844f82aed3ef965f5e`;
the amendment is `40d1285388247c79e7628d91b03335e12aca217de0d103ebe84081715bb03304`.
Both remain unchanged. [Repository design](../../DESIGN.md) governs; the
[philosophy](../../PHILOSOPHY.md) supplies rationale. Existing operator authority
continues within its scope. The B0 assignment authorizes this document and its
review, with no product-source or Cargo changes. The root owns the next dispatch.

## Consumer boundary established by B0

The intended first vertical slice remains:

```text
frozen source + candidate + policy
  -> verify_candidate -> exact verifier prompt
  -> bounded direct observation -> VerifierReport
  -> assess_candidate -> non-authoritative replay result
```

There is no call to `establish_checkpoint` in that replay. Its production
composition publishes artifacts and can publish a checkpoint, so using it as an
evaluation driver would cross the intended boundary.

The current public semantic interface cannot honestly represent a direct
invocation unchanged. This is a source finding, not a requirement to wait for
all of S10:

| Current source | Consequence for this consumer |
| --- | --- |
| [continuation.rs](../../rust/crates/lifecycle-semantic/src/continuation.rs), `SemanticInvocationBinding` lines 17–73 | Validation requires `NativeInvocationPurpose`, nonempty session and thread IDs, and the exact source/phase/policy binding. A direct request cannot invent native session/thread evidence to enter it. |
| Same file, `Candidate` lines 134–143, `VerifyRequest` lines 164–176, `candidate_from_output` lines 360–475, `verify_candidate` lines 477–530 | Compiler provenance also carries session identity. The verifier rejects the compiler's invocation/session and binds the candidate bytes and source. Changing only the verifier result envelope would leave this seam open. |
| [assessment.rs](../../rust/crates/lifecycle-semantic/src/assessment.rs), `VerifierResult`, `ObservedVerifierExposure`, `validate_result_binding`, `derive`, `assess_candidate` lines 55–319 | Host assessment regenerates the expected request; checks invocation, attempt, session and input/output hashes; checks exposure/model observations; and derives verdict from attributed obligations. Parsing a report or matching a request hash does not establish semantic acceptance or isolation. |
| [semantic.rs](../../rust/crates/lifecycle-service/src/semantic.rs), `establish_checkpoint` lines 143–430 | Production composition obtains compiler and verifier native evidence, assesses, rechecks dependencies and publishes. The replay needs only the semantic consumer; its driver has no store/publication handle. |
| [semantic_control.rs](../../rust/crates/lifecycle-service/src/semantic_control.rs), `run_control_turn` lines 389–566 | Native control claims a session, starts a thread, records entry and checks completed turn/history before returning native evidence. Its current isolation/auth repair stays with S10. |
| [lifecycle-provider](../../rust/crates/lifecycle-provider/src/lib.rs), `TextTurnObservation` and `TextTurnPort` lines 29–59 | This port also requires provider thread/turn IDs. Implementing it with made-up handles would repeat the problem; it is not the mandatory interface for the offline foundation. |

The smallest useful shared decision is **the semantic owner's representation
and evidence contract for native versus direct invocation provenance**, covering
the request, compiler/candidate provenance, result and exposure together. Its
owner is the lifecycle/S10 coordinating session
`5b98429e-2a67-444e-a81a-32b85a42e738`, not a transport worker. The bounded-inference
root can coordinate this seam without accepting the broader amendment or waiting
for every S10 exit condition.

Proposed resolution for that owner: preserve native identity and historical
encoding, add an explicit direct identity whose local request/exposure identity
is distinct from any provider response ID, and retain one semantic derivation of
obligation/citation outcomes. Versioned types or a composed adapter are possible;
the owner chooses the compatibility shape after checking actual consumers.
Neither optional native fields nor a synthetic native wrapper establishes the
new meaning. The direct exposure contract must bind the actual instruction/input,
configured capabilities, source custody, requested and observed identity, and
separation from compiler reasoning/hidden evaluation answers. Unavailable evidence
remains unestablished; no `false` exposure flags may be inferred merely because
the request omitted tools. A synthetic offline exposure fixture proves checks
on fixture observations, not an actual provider's history.

B0 therefore closes consumer selection, first placement, scope and dispatch
ownership. **Shared identity/exposure closure remains open.** The original B1
bundles offline mechanics and real semantic replay under one exit; the groups
below split those consequences explicitly so foundation success cannot be
reported as completion of the vertical slice.

## Coherent groups and dependencies

These groups follow shared contracts/files. They are dispatch units, not one
agent per feature. The coordinator may combine or revise routes while preserving
the owned result, authority and evidence. Each implementation group has one
supervisor, one coherent worker and a separate retained reviewer. Rows after the
first ready group are bounded planning packets, not active assignments.

| Group / consumer and result | Prerequisites; deferred uncertainties | Ownership and excluded overlaps | Validation, done state and handoff |
| --- | --- | --- | --- |
| **B0: decomposition and seam decision.** Consumer: root and lifecycle owner. Result: this dependency map, first packet, and exact semantic-owner question. | Source-bound architecture review; no account/deployment decision needed for this document. Shared identity representation still open. | This document and task-local evidence only. Original plan/amendment and every product file excluded. | Separate fresh review covers original plan plus this decomposition and normative assignments. Root integrates the bounded decisions. Decomposition completion does not mean semantic seam acceptance. |
| **B1a: offline one-shot foundation — ready for root dispatch.** Consumer: B1b's replay driver. Result: bounded request bytes, stream/attempt observations and replayable synthetic evidence. | This packet and ordinary Cargo ownership coordination. No semantic-owner decision, credentials or live provider required. Actual protocol/account/model behavior deferred to B2. | New `rust/crates/bounded-inference/` only; root owns minimal workspace/lock integration. No changes to semantic, service, store, native or auth crates. | Fault/stream/accounting tests, exact request/output/evidence hashes, focused lint/test checks and separate review. Done means **offline foundation accepted**, explicitly `semantic_replay_not_exercised`; handoff supplies typed observations for B1b. |
| **B1b: semantic identity plus offline verifier replay.** Consumer: existing `verify_candidate` → report → `assess_candidate`. Result: one honest native/direct mapping and disposable replay driver. | B1a observations and the lifecycle owner's bounded seam decision/ownership grant. Includes a disposition for synthetic candidate provenance; no fake compiler session. Live funding still deferred. | One worker owns the accepted changes across `lifecycle-semantic` request/candidate/result/exposure types and tests, plus the new replay driver. Owner names any necessary serialization consumer updates before dispatch. Production service/store/native changes excluded unless that same owner expands the bounded packet. | Existing native fixture/codec meanings preserved; exact bindings, citation/obligation failures, exposure unknowns and wrong model tested; no checkpoint/publication route. Done means **offline consumer replay exercised** on synthetic inputs, not model adequacy. Root and lifecycle owner accept their respective integration surfaces. |
| **B2: direct verifier qualification.** Consumer: evaluator of the direct verifier profile. Result: real tool-free inference and semantic outcomes on a separately bound corpus. | B1b; explicit direct SIWC/account/deployment and funding authority; registration/model eligibility, supported protocol and strict plan-quota evidence resolved for the selected live use. | Same bounded-inference package's selected auth/transport realization and isolated qualification driver/evidence. Credential owner controls credential state; independent synthetic corpus belongs to this coordinator. S10 trial/heldout material and existing Codex login excluded. | Session/account binding, credential lifecycle, renewal failure, no API fallback, runtime request/exposure evidence, actual completed/failed outcomes and funding/usage observations. Separate review; root accepts only the qualified profile. Unknown credit/quota exclusion remains an unmet live condition, not a reason to substitute funding. |
| **B3: compiler and production control integration.** Consumer: lifecycle semantic service. Result: separately exposed direct compilation/verification composed with existing admission, currentness and publication owners. | B2 quality/evidence and lifecycle owner's integration decision reconciled with current S10 obligations and schema/saved-state compatibility. No blanket S10 label substitutes for this decision. | One coherent integration worker owns the admitted semantic/service/provider/store consumer set. Coordinator serializes overlap with S10. No new universal operation registry or automatic routing. | Actual service composition, compiler/verifier separation, stale-input fencing, recovery and native compatibility; focused integration tests and retained review. Root plus lifecycle owner accept admitted control use. Candidate success alone is not successor adequacy. |
| **B4: bounded tool loop, only for an accepted consumer.** Consumer: selected snapshot/task operation. Result: complete calls enter under existing grants, return attributed results and preserve uncertain effects. | Accepted tool profile and per-operation admission/settlement contract; B1 foundation; B3 only where the selected composition needs it. No tool loop is prerequisite to tool-free B2. | One worker owns loop and the particular domain dispatch/admission adapters under the lifecycle owner. Exactly one loop/effect executor. Generic shell, code mode, arbitrary plugins and unrelated tools excluded. | Partial/duplicate calls, revoked/stale grants, restart around effect/result persistence, cancellation and uncertainty tests; actual tool composition and retained review. Done means selected tool profile qualified, not general harness parity. |
| **B5: retained execution and selective retirement.** Consumer: lifecycle continuation/successor/recovery. Result: retained direct work across replacement and restart; retirement only where supersession is established. | B3, B4 when needed, explicit amendment disposition, source projection/context identity/currentness mapping, saved-state and live qualification authority. | Lifecycle owner coordinates core/store/material/service/retained-work changes as one contract group. Account, deployment and other harness owners retain their decisions. | Actual successor use, pending interaction, completed-effect preservation, queue order, stale-source fencing and restart under the owning acceptance. Separate review. Root/owner accept precise profile; retired consumers and state have explicit dispositions. Offline replay and native success cannot discharge this result. |

Dependency shape: `B0 → B1a`; `B1a + semantic-owner seam → B1b`;
`B1b + live authority/funding → B2`; `B2 + lifecycle integration decision → B3`;
`B4` is driven by its selected tool consumer; `B3 + relevant B4 + retained-state
decisions → B5`. B2 auth work can be designed offline without authorization to
read credentials or make calls. It is not part of the first dispatch.

## First implementation dispatch: B1a

**Outcome.** Implement the smallest Rust foundation that B1b can consume: take
an exact instruction/input payload and selected one-shot profile, assemble
bounded request bytes, consume a supplied stream, and return attributed terminal,
output and usage observations. This group intentionally stops before invoking
`verify_candidate`/`assess_candidate`; representing that result needs the open
semantic seam. It is an offline foundation, not completed direct verifier replay.

**Assignment and files.** The root launches a scoped supervisor using the
[manual candidate](../../planning/manual-orchestration-skill-candidate-v2/SKILL.md).
The supervisor assigns one new `gpt-6-sol`/`medium` worker to the new
`rust/crates/bounded-inference/` package, its tests and synthetic fixtures. The
package owns this one-shot direct-profile machinery, with no lifecycle
authority dependency and no general backend framework. Module/file layout is
the worker's choice. The root or a specifically assigned integrator reserves
and applies the minimal `rust/Cargo.toml`/`rust/Cargo.lock` delta, preserving
concurrent edits. The worker does not independently claim shared manifests or
modify existing crates. Exact current Cargo bytes and package availability are
rechecked at launch; a collision returns to the coordinator for ownership
resolution, not deletion or replacement of another actor's work.

**Inputs and state.** Supply this document's exact hash, the source manifest
below, plan input hash and the scoped role reference. Use newly authored
synthetic instruction/input and response fixtures with declared expected
outcomes. The coordinator owns that corpus; it is not an independent quality
holdout. Evidence stays under a new B1a task directory selected at dispatch;
scratch attempts are disposable and never stored in a production lifecycle DB.
No credential loading, OAuth operations, public-network requests or provider
CLI launches are in this packet. Scripted in-process peers or bounded loopback
fixtures are sufficient. They do not impersonate a real account/model observation.

**Boundary consequences.** The foundation preserves exact instruction/input
bytes and profile revision; distinguishes logical operation, local attempt,
request digest and optional provider response identity; and carries no native
session/thread fields. Model selection/configuration are explicit inputs, with
observed identity separately unavailable or observed. The first fixture profile
is tool-free and retains the plan's documented request constraints as an
attributed test premise, not current live qualification. At B2, recheck the
official protocol and actual account before binding a live profile.

The request path has one owner. Durable attempt observations distinguish
prepared, possibly entered, terminal observed and outcome unknown: a local
crash after send intent cannot become “never sent.” Replaying observations has
no sending capability; retry would be a distinct, explicitly attributed attempt.
The first group may test a simulated send boundary without supplying live HTTP
or authentication. Evidence records selected bounds and their provenance; exact
positive byte/event/output limits are implementation parameters bounded by the
consumer input/output contract, not universal constants. A local deadline or
output cap does not prove remote cancellation or a quota cap.

Only a coherent completed terminal for the attributed response may supply
completed output. EOF, HTTP success, text deltas, a failed/incomplete terminal,
malformed or conflicting identities and timeout cannot be upgraded to success.
An unexpected tool item is unsupported for this profile and never executed.
Usage remains per distinct attempt/observation, deduplicated when events replay;
cumulative and incremental counters retain their meanings. Missing final/cache/
reasoning usage remains unknown. Do not equate token counts with current
context or plan-quota consumption. Logs/evidence omit credentials, authorization
headers and unrelated environment content; exact synthetic payload artifacts
remain available for replay without putting arbitrary payloads in diagnostics.

**Validation and review.** Exercise fragmented UTF-8/SSE boundaries, interleaved
response/item identities, duplicates, contradictory terminal observations,
missing/failed/incomplete terminal, malformed and oversized frames/output,
after-start quota refusal, unexpected tool output, and disconnect/timeout.
Fault cases around intent/send/terminal persistence must preserve unknown
outcomes. Verify duplicate events do not duplicate output/usage, replay cannot
send, and two attempts retain separate accounting. Synthetic verifier-shaped
JSON is opaque output here: its successful decoding is not an assessed verdict.
Use focused crate tests, formatting and clippy on the exact final source. The
supervisor obtains a fresh read-only Astra/xhigh review and retains that reviewer
through bounded fixes. Tests and review bind source, executable/profile when
executed, fixtures and evidence hashes; actor-reported conduct is distinguished
from independently observed capabilities/history.

**Done and return.** Return changed-file hashes, targeted test/review results,
actual attempts and failed routes, remaining limitations and process disposition.
Root acceptance may establish `offline_foundation_accepted`; it cannot establish
`semantic_replay_exercised`, provider qualification, live semantic adequacy or
production admission. The next owner is B1b's semantic/replay supervisor after
the lifecycle owner settles its seam. If unfinished, return a durable
`handoff_ready` path and SHA-256 with ownership, decisions, failures, unresolved
obligations and next action; do not use a completion label.

## Orchestration and decisions left with their owners

The coordinating root owns decomposition, group dispatch and integration. A
supervisor collects its worker and required review, validates the exact composed
result and returns a bounded readiness judgment. Builders keep their useful
implementation context through ordinary fixes. Architecture and separate review
use Astra/xhigh; new builders use Sol/medium under the
[model-selection guidance](../../docs/model-selection.md). Requested launch
configuration and observed runtime identity are recorded separately; unknown
effective identity remains unknown. This task has no cross-provider requirement.
A read-only review assignment and fresh reasoning context are the bounded review
arrangement here, not independently attested capability confinement or proof of
provider independence.

At logical task boundaries, use the [context CLI](../../docs/context-status-cli.md)
when the agent's exact runtime session identity is established. An inherited
shell ID or the parent's rollout is not child telemetry. If unavailable, record
that limitation and decide from the task state. The agent judges whether a
handoff is useful; there is no fixed token threshold, polling rule or automatic
reset. Context/weighted-token estimates are advisory, not quota evidence. Before
replacement apply Wind Walker: the parent verifies a durable path/hash, launches
the successor and transfers ownership after acknowledgment. Unfinished handoff
is distinct from worker completion, supervisor acceptance and root integration.
The supervisor remains responsible for collection and owned processes; long
supported waits are appropriate when no independent work remains.

Only these decisions hold affected later work:

1. **Semantic owner:** native/direct request, candidate, result and exposure
   representation; historical encoding and downstream consumer compatibility.
   Holds B1b's real consumer mapping. It does not hold B1a.
2. **Account/funding owner and live root:** direct registration/deployment
   eligibility, authorized model/account, credential custody and strict quota
   evidence/controls. Holds B2 live entry and related credential operations.
3. **Lifecycle owner:** production integration, current S10 obligations, saved
   state and later amendment disposition. Holds the affected B3–B5 changes.
4. **Coordinator:** exact group acceptance and shared Cargo/resource ownership
   at dispatch. These are ordinary authorized coordination decisions, not new
   user-approval ceremonies.

## Evidence and review boundary

Source snapshots, hashes and graph evidence are retained in
`/home/bline/.local/state/work-engine/bounded-inference-b0-20261021/`:
`source-manifest-v1.json`, `source/`, and `graph-evidence.json`. The separate
review and supervisor result in that directory bind the exact document hash;
the coordinator records final integration disposition outside this candidate.

Tier 2 graph verification used project `home-bline-code-work-engine`, watched
and ready; final coverage generation `ube57cfc0a20bbcebg3`, indexed
`2026-10-21T18:32:47Z`. Relevant searches were complete. A path-filtered search
returned no symbols; broadened name search recovered the seam, with no negative
inference. Depth-one inbound/outbound traces for `verify_candidate` and
`assess_candidate` were complete; exact function/source reads established the
material claims. All 18 relied-on repository paths had content verification and
no recorded coverage issue. Clean coverage is best-effort, not exhaustive
dependency proof. Source snapshots, rather than HEAD alone, bind dirty bytes.
`semantic_control.rs` was read at its current changed source; later concurrent
changes require affected claims to be reconsidered at dispatch.

The board observation at B0 admission was sequence 2317: integrated/reviewed V4
session-auth repair, proxy/native isolation work continuing, no genuine
session-funded call reported and the frozen live4 subject still held. This is
attributed coordination state, not this document's qualification of S10. Its
owner's future evaluation of the input proposals remains open. No heldout
trial/corpus, auth material or provider operations were used. No product tests
or live semantic result are claimed by this document.
