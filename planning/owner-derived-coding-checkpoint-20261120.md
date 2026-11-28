# Owner-derived coding checkpoints and verified continuation

**Standing:** Proposed implementation plan, 2026-11-20. This record does not
change lifecycle doctrine, admit a new runtime profile, or authorize rollout.
**Source baseline:** `rescue-detached-20260911` at
`f4736f0eec12d0a1c5adf60b1b1774f7194a7e41`. Code observations are from
source inspection; the acceptance checks below are proposed, not run results.
**Review status:** This revision addresses the Fable review of `1532896f`;
the amended plan has not been re-reviewed.

## Objective and placement

A fresh Pi context should recover the actual work position, open obligations,
and next authorized assignment without relying on the outgoing builder to
remember or correctly describe its own actions.

- **Canonical capture:** Rust projects validated, bounded facts from their
  owners into an immutable artifact. It is authoritative for the captured
  revisions, not a source of live state, an effect ledger, or a grant.
- **Semantic continuation:** A model explains decisions, consequences, and
  unresolved questions with citations. Separate verification checks its use of
  the source basis and required obligations. Neither model can change a typed
  fact, settle an effect, close an obligation, or authorize execution.
- **Successor binding:** Admission selects the exact assignment revision,
  generation, and authority. The lifecycle transition owner alone makes the
  successor active, after exact delivery, verification, and a currentness
  check. An accepted interpretation is not itself permission to act.

Use the existing lifecycle reducer and stage sequence. Place coding evidence
validation with the existing producer and settlement owners, compose the
read projection at the trusted host boundary, and use `lifecycle-service` as
sole transition driver for the hybrid profile. Do not build a second driver
in the Pi host or teach `lifecycle-semantic` to query domain stores.

Production runs on the existing dev-vertical Pi/Docker Host: the inference
container has no network or provider credentials; the trusted Host proxy applies
provider credentials outside the container. Reuse that boundary for both coding
and semantic control. This is lifecycle integration, not a new inference
transport. Codex app-server is neither a production prerequisite nor a fallback
for this profile. A provider or model choice for semantic control is not made by
this plan.

| Current seam | Reuse and gap |
| --- | --- |
| [`lifecycle-store/src/pi_coding_proof.rs`](../rust/crates/lifecycle-store/src/pi_coding_proof.rs) and [`pi_coding.rs`](../rust/crates/lifecycle-store/src/pi_coding.rs) | The v4 proof joins exact Host result, workspace manifest, and settled native history. `live_checkpoint_of` reconstructs the owner checkpoint without substituting model final text. The grammar is closed around one `workspace_exec`, not a general coding-session ledger. |
| [`lifecycle-service/src/material_port.rs`](../rust/crates/lifecycle-service/src/material_port.rs) and [`retained-task-workflow/src/material.rs`](../rust/crates/retained-task-workflow/src/material.rs) | `CanonicalMaterialPort` supplies exact material, dependency observations, and a short currentness fence. `TaskStore` supplies those for its own namespace and retained analysis profile; changing `project()` alone is not enough. |
| [`lifecycle-service/src/semantic.rs`](../rust/crates/lifecycle-service/src/semantic.rs), [`engine.rs`](../rust/crates/lifecycle-service/src/engine.rs), and [`lifecycle-semantic`](../rust/crates/lifecycle-semantic/src/lib.rs) | Reuse the semantic roles, validation, and fenced commit, not the production control runner. On the baseline the successor prompt lacks a separately supplied coding capture and assignment, and activation qualifies only the existing Codex-native semantic path. The Direct checkpoint path stops before switch; it is not an activation shortcut. |
| [`pi-pressure-context-host/src/coding.rs`](../rust/crates/pi-pressure-context-host/src/coding.rs) | Reuse the physical Pi replacement and exact input evidence. The existing pilot's `prepare_reset` and `finish_rehydration` drive facts directly against the shared reducer. In the hybrid profile the Host is the actuator and evidence producer, not a competing transition driver. |

## 1. Feed one existing coding proof into semantic verification

Add a typed, read-only coding projection in `lifecycle-store` over the validated
v4 settlement basis. Expose the certified fields without requiring consumers
to parse legacy checkpoint JSON or revalidate raw events. Keep the v3/v4/v5
profile/protocol bindings, grammar, and existing checkpoint bytes unchanged.
Start with one settled operation, one frozen workspace manifest, and one open
obligation from an owner record.

Compose that projection with the admitted objective and task revision through
`CanonicalMaterialPort`. Generate `SourceMaterial`, dependency revisions, and
required obligations from owner records. Admit a distinct coding material policy
in both `project_material` and `within_frozen_limits`; do not label coding work
as retained repository analysis. Fit the existing 16 KiB per source item and
64 KiB total source limits by projecting certified fields (operation disposition,
exit status, proof and manifest digests), not raw stdout/stderr or the full v4
checkpoint. Keep the existing `ContinuationPlan` shape unless the vertical proof
finds a concrete missing distinction. Preserve the decision and its source or
invalidation condition, not the full reasoning transcript.

Project only those facts the relevant owner establishes. Exact capture and a
hash do not upgrade stdout or model-authored text to `HostCanonical`. A
settled command's exit status is an observation; it is not independent test
acceptance. The pilot's [validator](../rust/crates/work-engine-manual-host/src/pi_coding_validation.rs)
runs after continuation, so its result is pending at capture in the first
vertical. A known pending obligation is citable state; an unavailable required
owner record is not evidence of absence. If material is over bounds or required
evidence is missing or conflicted, record ineligibility rather than silently
dropping the item.

**Exit:** An offline fixture joins the real coding projection to the
`lifecycle-semantic` capture and candidate verification API. Test deterministic
refusal of a changed fact binding or a missing or foreign citation; a missing
required finding yields `Unresolved`. Separately script a faithful assessment
and an attributed violation for a candidate falsely claiming validation is
complete. Assert their respective acceptance and rejection branches; this proves
how the system handles the assessment, not that a live verifier detects the
contradiction. Builder narration cannot change projected operation, workspace,
or obligation facts. This slice requires no hosted control runner and cannot
activate a successor.

## 2. Activate a fresh successor through one driver

**Qualify Pi for semantic control.** On the baseline, `CommitSuccessor`
requires an accepted successor-adequacy result for the current capture and
settled attempts for all three semantic phases, each under a matching,
unrevoked, unexpired `semantic_control` grant. The only existing producer of
those qualifying attempts, `run_control_turn`, uses the Codex app-server.
Implement a Pi/Docker realization for compile, candidate verification, and
successor adequacy in the lifecycle-service runner. Use distinct, scoped
control sessions with no domain tools or predecessor conversation; verifiers
receive only their bounded assessment inputs. Use the existing Host proxy and
its route admission and settlement, with no provider credentials in the
container. Bind each control attempt and result to its phase, exact capture,
delivered input, grant revision, and Host receipt. Record requested model and
route separately from observed dispatch/served identity; do not fill an
unobserved identity from configuration.

Teach the lifecycle store to qualify that Pi realization for this profile in
its existing activation transaction. Require all three Pi phases, the accepted
successor verdict, and grants still current after rehydration at commit. Do not
count mixed, unsettled, unqualified, or Direct results as a complete Pi proof.
Keep existing Codex and Direct behavior for their own profiles. If Pi control
cannot be qualified, hold activation rather than switching runtimes or skipping
verification.

**Drive the replacement under lifecycle-service custody.** Add a coding
profile path to `drive_transition`; do not reuse the retained analysis
readiness predicate, which requires an exact yielded delivery. Establish
`PredecessorSafe` from the settled predecessor turn and Host journal evidence
that the workspace effect is settled, the tool container is removed for this
profile, and no predecessor writer remains. Expose the Host evidence through
settlement; a pilot assertion or an intent without a result is not evidence of
quiescence. Entered but unresolved effects stay fenced and are not replayed.

Bind the successor generation, assignment revision, and grant identity and
revision from admission, not from `ContinuationPlan.next_action` or the pilot's
hardcoded `context:new`. Port the pilot's `bridge_replace` → `fresh_successor` →
`with_pi_coding_reset` → rehydration sequence into the lifecycle-service Pi
runner under actor custody. Adapt the reset binding from the legacy settled
basis to the exact accepted hybrid package for this profile without weakening
legacy checks. The Host performs the physical replacement and emits exact
input and result evidence; the production path does not invoke the pilot to
apply transition facts. Rehydrate an observed fresh Pi session without reading
predecessor conversation or enabling domain tools. A parent-session reference
may survive as lineage metadata; it must not import or expose the old history.
Deliver canonical facts, accepted interpretation, and the admitted assignment
as separate inputs. Bind the challenge to the assignment and require a
task-specific answer, not a digest echo. Admit domain operation entry only after
activation and under its own grant.

Use the existing semantic capture/package where it suffices. If a consumer
needs a versioned envelope, make it a manifest of references:

| Section | Contents |
| --- | --- |
| `canonical` | Reference to the versioned capture of task and plan revisions, exact workspace or Git subject, owner-derived outcomes, open obligations, and the source manifest. |
| `semantic` | Accepted candidate and candidate verification references bound to the same source basis; no executable authority. |
| `successor` | Exact admitted assignment, generation, and authority references with their revisions. |

Bind the exact delivered prompt and assessed response to that package. Keep
rehydration, successor verdict, and activation receipts subsequent to the
package; don't make a package depend on its own future receipt. The factual
capture survives a missing or refused semantic verdict, but the hybrid profile
stays held; it never falls back to deterministic activation.

**Currentness is part of correctness.** An immutable digest proves what was
captured, not that the admission or its selected sources are still current.
For the bounded vertical, use this owner and fence split:

| Source | Owner and commit fence |
| --- | --- |
| Current coding admission: selected task/objective and plan revisions, successor assignment and generation, selected workspace manifest, and pending validation obligation for that admission | Store the current admission and its revision in lifecycle SQLite. Superseding any selection or changing the obligation bumps the bound revision. Compare it inside `apply_transition_fact`'s activation transaction, not through an external port holding another write transaction on the same database. This owns admission, not the contents of external task or plan records. |
| Semantic-control and successor authority | Lifecycle SQLite `grants`; recheck the exact identities, revisions, scopes, and validity in the same activation transaction, even with zero queued inputs. |
| Settled operation and exact manifest contents | Host journal and immutable artifacts, joined by lifecycle-store's validated settlement. Verify their exact references and settlement; mutable workspace **selection** is the admission record above, not a new truth inferred from a workspace path. |

Observe required revisions before use and keep the commit fence short; do not
hold locks across inference. The first vertical uses pinned task and plan inputs,
not mutable external current pointers. If a later profile requires the current
revision of an external task, plan, or review, name its actual owning store and
use its `with_current_fence` or equivalent across the short commit in documented
lock order. A pinned copy is not an external currentness check. If an owner
cannot provide that property, hold activation. Preserve stale captures as
historical evidence and recapture and reverify changed dependencies.

**Exit:** A service-level fixture drives the coding profile through
`drive_transition` and the existing reducer to `Reconciled`, with no pilot
transition driver. Exercise all three control phases through the Pi realization
using offline provider fixtures. Check that the fresh Pi receives only the
bound inputs, has no domain tools before activation, demonstrates the correct
next action, and activates once without repeating the completed effect. A
revision bump between dependency observation and commit makes the capture stale
and applies no activation. Refuse an expired semantic-control grant after an
accepted successor verdict, a revoked successor grant with zero queued inputs,
a missing Host effect result, an uncertain reset, and a rejected successor
response. Test reopen at reset and activation; neither a verified verdict nor
an unresolved effect grants permission to replay.

## 3. Widen coding evidence under a new versioned profile

After the first end-to-end proof, support multiple sequential operations with
explicit limits. The one-call assumption lives in more than the proof: the
[qualification driver](../rust/crates/pi-pressure-context-host/src/bin/coding-qualification.rs)
and [server preparation](../rust/crates/work-engine-server/src/bin/coding-job-prepare.rs)
also pin it. Roll out a separately identified profile/protocol and update every
bound producer and consumer for that profile, including Host and receiver.
Reuse only the framing and validation mechanics with common semantics.

Keep the Host journal as the effect owner. Join exact task, invocation, call
and attempt identity, dispatched arguments, outcome evidence, and workspace
identity. Distinguish not entered, settled failure, and entered with unknown
outcome. Record commit and manifest references separately when the commit does
not identify the whole captured tree. Project test outcomes only from verified
validation evidence bound to the exact tested snapshot and validation policy.

**Exit:** More than one operation reconstructs the same projection after
reopen. Foreign, duplicate, mismatched, and out-of-bound records fail closed.
An observed command success cannot stand in for validation of a different
snapshot; a crash after intent but before collection cannot create either a
successful effect or a retry entitlement. Legacy fixture and live-proof
vectors continue to pass unchanged.

## 4. Join real review evidence and qualify the hosted route

Use the read side of [`review-execution-evidence`](../rust/crates/review-execution-evidence/src/lib.rs)
for qualified execution provenance and the review owner for disposition and
open findings. Preserve distinct records for the admitted request, actual
dispatch, observed model and route, findings returned, and owner acceptance for
the exact reviewed subject. No qualified execution evidence means the required
review remains unestablished, not that nothing could have happened. The coding
review and the continuation verifier answer different questions and cannot
substitute for each other.

**Exit:** Reproduce the omitted `model` and `task` dispatch with captured
arguments. The checkpoint still shows the review requirement pending, with no
qualified execution of the requested reviewer, regardless of the builder's
claim. Include an execution with the wrong model, a genuine result for a
superseded candidate, and a qualifying accepted review of the current subject.
Run the exact hosted subject under separately admitted authority before claiming
live support. Show that the semantic control phases and successor run on the
networkless, credential-free Pi/Docker route and that the Host receipts support
the activation decision without a Codex app-server. The existing [hosted
qualification record](../records/dev-vertical-hosted-qualification-20261117/README.md)
qualifies the coding transport on its own subject, not this semantic activation
path or external grant issuance and owner acceptance.

## Follow-up experiment, not a fallback

Once the hybrid is qualified, compare semantic-only, hybrid, and
deterministic-only recovery in controlled replay. Hold the coding task,
admitted assignment, underlying evidence, and fault cases constant; vary what
is delivered and verified. Measure accepted recovery, repeated effects, lost
obligations, review-status errors, cost, and latency. A deterministic-only
production profile would need its own explicit admission and verification
contract; omitting a semantic call from a hybrid run does not implement it.

**First PR:** One validated v4 operation, one frozen workspace, and one open
owner obligation reach the semantic API in an offline fixture. Test exact
binding and citation checks separately from scripted assessment handling. No
protocol widening, hosted control runner, reset rights, or successor activation.
