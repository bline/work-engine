# RP1: Claude transport and retained-session custody

## Status and authority

Proposed implementation slice, prepared on 2026-10-15 against
`af4546c0c3f65e25ff55f67a06ee32e94433972c`. The user authorized preparing this
bounded plan after RP0 and SC3-D were committed. This document does not record
RP1 implementation approval. The earlier RP0/SC3-D authorization explicitly
excluded RP1; publication of those slices did not expand it.

The [provider plan](rust-review-provider-execution-plan.md) owns the intended
execution contract. [DESIGN.md](../DESIGN.md) owns design doctrine;
[Rust development](../docs/rust-development.md) owns the language direction. This plan
specializes RP1 without authorizing RP2 host wiring, RP3 live proof, SC3-H resume,
credential access, provider spending, deployment, or S8/S9/UI changes.

## Outcome and placement

After an authorized initial entry, the execution-evidence owner can preserve and
read back the exact response, observed identity, realization evidence, and
protected session state. Reopening the owner can finish publication from a
verified durable capture or report an unresolved attempt; it cannot start the
provider again. A session locator describes state and cannot authorize entry.

The owner remains `review-execution-evidence`. Transport, capture, session
custody and profile-specific checked readback belong there because its existing
consumers already rely on `ExecutionEvidenceOwner`. Workflow owns campaign
admission and consumes the eventual checked result. RP1 supplies the execution
capability and an offline driver; RP2 installs the production launch composition.

Two alternatives are rejected for this slice: a second provider journal would
split custody ownership, and implementing transport inside the host would mix
execution custody with campaign admission. Reuse E1 storage mechanics through
sealed profile selection, preserving the old controlled profile. Do not make a
caller-supplied profile string a way to upgrade evidence.

## Verified starting points

These observations describe the baseline, not new requirements by themselves.

| Existing source | Consequence for RP1 |
| --- | --- |
| `producer.rs::EvidenceWriter::execute` records preparation, launch intent, spawn outcome and terminal evidence; an existing attempt goes to reconciliation. | Preserve the one-entry boundary. Separate transport-specific behavior from reusable evidence storage. |
| `producer.rs::EvidenceWriter::reconcile` reads terminal evidence or reports unresolved state; it does not spawn. | Keep reconstruction APIs incapable of issuing a launch capability. |
| `reader.rs::EvidenceReader::read_checked` explicitly selects the controlled profile and its exact event/artifact inventory. | Add a distinct qualified verifier; changing a profile label on a controlled record is insufficient. |
| `store.rs::Store::create` installs the controlled profile/schema marker. | Reuse root fencing and immutable storage with sealed, versioned profile dispatch; preserve existing markers and reader behavior. |
| `claude/profile.rs::TrustedRp0Profile` pins Bubblewrap and CBM programs; `CandidateBoundary::run_hostile_probe` mounts and runs a Python probe. | RP0 is not yet a Claude transport launch interface. Narrow launcher/profile edits are part of the proposed RP1 scope. |
| `review-workflow/src/initial.rs::Workflow::{launch,join}` uses `ControlledScope`, `EvidenceWriter` and controlled reader selection. | Qualifying the RP1 library alone does not make the permanent host use it. That wiring remains RP2. |

RP0's accepted candidate-specific CBM relay is retained. The older provider
plan's no-broker expectation was superseded for that bounded realization by the
accepted RP0 review; it is not reopened here. The 4096-request threshold and
hard cleanup deadline were not established by RP0. RP1 must report what its
changed launch path actually proves rather than inherit a stronger claim.

## Proposed owned surface

All new executable implementation and adversarial peers are Rust. Existing RP0
test fixtures remain valid historical evidence; replacing their language is not
an RP1 objective.

| Surface | Proposed change boundary |
| --- | --- |
| `review-execution-evidence/src/claude/{transport,session,writer,reader}.rs` | Exact transport envelope, process ownership, private session capability, qualified publication and checked readback. Names are proposed, not existing APIs. |
| `src/claude/{mod,profile,isolation,credentials}.rs` | Exports, exact installed profile, narrow reusable launch/observation interface, and only required access-projection integration. Retain RP0's confinement and descriptor-relative credential checks. |
| `src/{lib,contract,store}.rs` and, if needed, `producer.rs`, `reader.rs`, `process.rs` | Sealed profile dispatch and demonstrably shared mechanics. Controlled behavior and existing artifact identities remain unchanged. This is not a generic provider framework. |
| Crate-local migration and manifest | Fresh qualified roots with an explicit version/profile. A migration is needed only if the new durable format requires it; no upgrade of existing controlled roots. |
| `tests/{claude_execution,claude_recovery,claude_custody}.rs`, Rust peer and compile-fail fixtures | Provider-disabled behavior, hostile transport, custody and capability boundaries. Existing test-support/fault features are the starting mechanism; fixture authority must remain unavailable to production. |
| RP1 validation document and external evidence | Exact source/lock/features/binary/profile bindings, raw nonsecret results, limitations and review references. |

The exact changed-path manifest is frozen for qualification. Shared Cargo edits
need coordination if existing workspace dependencies cannot serve the slice.
No campaign, episode, CE, workflow or host semantic change is implied by these
paths. A concrete missing owner API is a scoped amendment, not permission to
duplicate its authority in E1.

## Contract to establish

### Profile and authority

`TrustedClaudeProfile` and `QualifiedScope` have checked private construction.
They bind executable/runtime/catalog, requested model and effort, command and
prompt, exact candidate/selection/claims, CBM store/generation/coverage,
realization, root, attempt, dispatch identity and budgets. Deserialization or
conversion from `ControlledScope` cannot produce an admitted scope.

The launch path consumes a non-replayable in-process capability. E1's launch
ticket prevents repeated local entry; it does not grant campaign authority.
The future workflow composition must possess its Applied dispatch authorization
before using that path. Offline tests use an explicitly test-only entry owner.
No JSON command or successful custody read constructs a production permit.

The exact production model remains unselected. Offline tests can pin an explicit
test model/catalog without choosing it. A later production installation must
bind the owner's selected model, executable and catalog; the builder's model
and the parent plan's Opus recommendation do not make that decision.

### Entry, transport and identity

Reserve bounded failure/result custody before a launch syscall. Durable intent
precedes that syscall because a crash afterward cannot establish non-entry.
The trusted launch boundary is verified before prompt release; otherwise a
prompt could be consumed before its required isolation evidence exists.

Construct the child environment and tool/config surfaces from the selected
allowlist. Parse the pinned transport envelope separately from the canonical
episode result. Requested and observed model/session/route fields remain
distinct. Missing, mixed, unacceptable or mismatched identity cannot become a
successful checked result. Malformed output never triggers an automatic
corrective call. Preserve stderr under the bounded private custody policy.

Track local start, prompt submission, provider-reported entry, capture and exit
as separate observations. Timeout, EOF, interruption or local reap does not
prove remote cancellation or non-entry. The guarantee is one host-authorized
launch/prompt operation, not one upstream HTTP request. Any internal CLI retry
behavior is pinned/reported, not hidden behind that local guarantee.

### Capture and recovery

A `result_captured` label alone is insufficient for recoverable publication.
RP1 persists the complete bounded raw capture and its identity/integrity binding
before it reports a recoverable capture. Terminal publication consumes that
exact capture and the required observation/session evidence. Torn or incomplete
capture remains unresolved. Do not silently change the controlled profile's
historical stage interpretation to achieve this.

| Observed durable state | Allowed recovery result |
| --- | --- |
| Verified pre-entry rejection | Record local non-entry; a later entry needs new owner authorization. |
| Intent/spawn/prompt state without complete verified capture | Report the original unresolved attempt, preserving its identity; no respawn or replacement session. |
| Complete verified capture, terminal publication unfinished | Finish the original publication from custody, without executing the peer/provider. |
| Committed terminal record | Return the same exact reference after revalidation. |
| Missing/replaced session or root | Refuse retained capability issuance; do not describe reconstruction as same-session continuity. |

`resume` means reopening the evidence owner; it does not mean a Claude resume
call. Actual retained inference entry remains SC3-H. Recovery code has no
transport-launch path, and its tests observe peer starts outside the evidence
journal under test.

### Session and secret custody

`SessionLocator` is serializable data. `RetainedReviewSession` is a checked,
private-field capability with no Clone/Serialize/Deserialize authority route.
Its owner verifies root/profile/session identity and immutable per-attempt
manifests linked to their predecessors. Legitimate later session evolution does
not retroactively change the initial attempt's evidence. RP1 can prove this
mechanism with a peer; it cannot claim live inference continuity.

Canonical credentials remain outside the reviewer. Synthetic credentials alone
exercise access-only projection, expiry and replacement behavior. No token,
token fingerprint, refresh material or secret-bearing environment/command is
written to receipts, logs or prompts. A canary appearing in captured output
fails the test; redaction after publication is not the protection mechanism.

### Observation and fixture separation

The observer owns the actual realized mount/process/CBM/capability facts outside
the reviewer's writable surface. Missing facts yield absent/unestablished
evidence; observed mutation authority is not converted to absence. CE remains
the owner of establishment and admission.

Offline peers have distinct executable, profile and root identities. They may
exercise the same verifier logic, but cannot produce production-qualified
provenance or open the production-qualified reader. Test support is unavailable
as an authority constructor in the production build. An offline pass is a
mechanism qualification, not successful Claude authentication or a live review.

## Validation and delivery

The following is an evidence-oriented implementation route, not a mandatory
edit order. The invalid outcomes and owner boundaries above remain binding
within an approved slice even if another route is more effective.

1. Define the sealed profile/capability boundaries and exact transport fixtures.
   Add the vertical peer-to-durable-capture-to-reopened-reader proof, plus the
   negative controlled-as-qualified and locator-as-permit cases.
2. Integrate RP0 launch/observation, bounded transport and session manifests.
   Complete refusal and recovery behavior before claiming the vertical result.
3. Freeze the candidate and run serial deterministic qualification. Review the
   exact candidate, remediate within the retained review episode, then present
   acceptance and publication evidence separately.

| Evidence family | Required distinction |
| --- | --- |
| Envelope/result | Model/session/subject mismatch, malformed/duplicate/oversized JSON, unacceptable route and noncanonical episode result refuse without a second call. |
| Pre-entry | Executable/config/catalog/CBM/candidate drift or missing observation prevents prompt release; peer witness records zero prompts. |
| Recovery | Fault cuts at each newly introduced durable boundary, including capture and terminal publication; repeated recovery preserves original references and zero/one peer starts. |
| Interruption | Before prompt, after prompt and after capture; held peer and open pipes exercise bounded output/cleanup without blocking observation. Unproven reap or remote settlement stays unestablished. |
| Custody | Root/database/fence/session replacement, symlink/hardlink/FD escape, manifest corruption and lost session refuse checked readback or capability issuance. |
| Secrets | Expired/unsafe/replaced synthetic credential, missing expiry horizon and canary leakage fail with bounded nonsecret diagnostics. |
| CBM | Retain candidate-only relay/store/config/network confinement; exercise hostile project/Cypher/path queries, drift and graph/source disagreement with truthful fallback provenance. |
| Compatibility | Existing controlled E1 and RP0 gates; affected H0/episode/campaign/J1/J2/H1 consumers retain their contracts. |

Use Rust 1.92, locked/offline dependencies and isolated target/scratch roots.
Run E1 default and fault-enabled cases in debug and optimized modes serially,
plus all-target Clippy with denied warnings and formatting. Run affected
downstream package regressions on the same frozen Cargo subject. Retain the
SC3-D distinction between the excluded expensive debug capacity case and its
explicit optimized 100/101 proof; an exclusion is not a full-suite pass.

The parent plan's 700-probe permanent-host responsiveness and joined CE/episode
proof remain required for the joined host qualification. RP1's held-peer checks
cannot stand in for those RP2 observations. RP1 does not claim a usable permanent
host path or completed campaign from a library-only test.

Candidate budget defaults from the parent plan are one in-flight attempt,
10-minute attempt horizon, 5-second local cleanup target, 64 KiB command/status,
1 MiB result/observation, 4 MiB streams and bounded 64 MiB custody plus 64 MiB
session storage. Freeze supported values with evidence during qualification.
Do not claim RP0 already proved the cleanup target or these transport limits;
an unachievable required bound needs an explicit disposition, not silent success.

The proposed review follows the established Astra/high same-model review
preference, with no independence claim and retained remediation context.
Launching that review belongs to the approved implementation assignment; this
planning artifact neither launches an agent nor changes production model choice.

## Decision and handoff

The next decision is approval of this provider-disabled RP1 implementation scope,
including the narrow launcher/profile and controlled-store dispatch edits above.
The production model and concrete live execution authorization can remain
deferred without blocking the offline implementation. Publication remains a
separate action after implementation acceptance.

Evidence packet:
`/home/bline/.local/state/work-engine/rp1-plan-20261015/evidence.json`.
It records exact source hashes, bounded graph queries, coverage and limitations.
No implementation, test execution, provider call or fresh adversarial review
was performed to prepare this plan.
