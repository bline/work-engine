# Owner-derived coding checkpoints and verified continuation

**Standing:** Proposed implementation plan, 2026-11-20. This record does not
change lifecycle doctrine, admit a new runtime profile, or authorize rollout.
**Source baseline:** `rescue-detached-20260911` at
`f4736f0eec12d0a1c5adf60b1b1774f7194a7e41`. Code observations are from
source inspection; the acceptance checks below are proposed, not run results.

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

| Current seam | Reuse and gap |
| --- | --- |
| [`lifecycle-store/src/pi_coding_proof.rs`](../rust/crates/lifecycle-store/src/pi_coding_proof.rs) and [`pi_coding.rs`](../rust/crates/lifecycle-store/src/pi_coding.rs) | The v4 proof joins exact Host result, workspace manifest, and settled native history. `live_checkpoint_of` reconstructs the owner checkpoint without substituting model final text. The grammar is closed around one `workspace_exec`, not a general coding-session ledger. |
| [`lifecycle-service/src/material_port.rs`](../rust/crates/lifecycle-service/src/material_port.rs) and [`retained-task-workflow/src/material.rs`](../rust/crates/retained-task-workflow/src/material.rs) | `CanonicalMaterialPort` supplies exact material, dependency observations, and a short currentness fence. `TaskStore` supplies those for its own namespace and retained analysis profile; changing `project()` alone is not enough. |
| [`lifecycle-service/src/semantic.rs`](../rust/crates/lifecycle-service/src/semantic.rs), [`engine.rs`](../rust/crates/lifecycle-service/src/engine.rs), and [`lifecycle-semantic`](../rust/crates/lifecycle-semantic/src/lib.rs) | Reuse attributed compilation, independent candidate and successor assessment, and the fenced commit. Today the semantic successor prompt carries the candidate and a challenge, not a separately supplied coding capture and admitted coding assignment. The admitted Direct checkpoint path stops before switch; it is not an activation shortcut. |
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
required obligations from owner records. Admit a bounded coding material policy
explicitly; do not label coding work as retained repository analysis. Keep the
existing `ContinuationPlan` shape unless the vertical proof finds a concrete
missing distinction. Preserve the decision and its source or invalidation
condition, not the full reasoning transcript.

Project only those facts the relevant owner establishes. Exact capture and a
hash do not upgrade stdout or model-authored text to `HostCanonical`. A
settled command's exit status is an observation; it is not independent test
acceptance. The pilot's [validator](../rust/crates/work-engine-manual-host/src/pi_coding_validation.rs)
runs after continuation, so its result is pending at capture in the first
vertical. A known pending obligation is citable state; an unavailable required
owner record is not evidence of absence. If material is over bounds or required
evidence is missing or conflicted, record ineligibility rather than silently
dropping the item.

**Exit:** An offline fixture uses the actual semantic capture and candidate
verification path. The fixture verifier accepts a faithful continuation and
refuses candidates that omit the obligation or claim completed validation. Builder
narrative changes cannot change the projected operation, workspace, or
obligation facts. These tests demonstrate binding and refusal, not the accuracy
of a live verifier. This slice does not enable successor activation.

## 2. Activate a fresh successor through one driver

Use the existing semantic stages with coding-specific readiness evidence and
trusted composition. The retained analysis readiness predicate depends on an
exact yielded delivery; do not apply it to coding evidence or infer readiness
from final text. Prove that predecessor writers are quiescent and enforce the
existing custody barriers for entered effects. Never interpret a captured
unresolved effect as permission to proceed or retry.

Bind the successor generation, exact assignment revision, and grant identity
and revision from admission. Neither `ContinuationPlan.next_action` nor the
pilot's hardcoded `context:new` chooses the successor. Adapt the reset boundary:
[`with_pi_coding_reset`](../rust/crates/lifecycle-store/src/pi_coding.rs) expects
the legacy settled basis. Give the new profile an exact binding to the accepted
hybrid package without weakening legacy checks. Rehydrate into an observed fresh
Pi session without predecessor conversation or domain tools enabled for the
rehydration turn. Deliver canonical facts, accepted interpretation, and the
admitted assignment as separate inputs. Bind the challenge to the assignment
and require a task-specific answer rather than a digest echo. Admit domain
operation entry only after activation and under its own grant.

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
captured; it doesn't prove that a task, plan, selected workspace, or review is
still current. Observe required dependencies before use and fence mutable
selections across the short commit, without holding locks across inference.
Recheck exact successor authority in the lifecycle transaction even when there
is no queued input to reassign. Keep same-store checks in that transaction;
for external owners establish a documented fence order and ensure the check and
commit cannot be separated by a relevant mutation. If a required owner cannot
provide this property, do not activate that profile. Preserve stale captures
as historical evidence; recapture and reverify when a required revision changes.

**Exit:** The fresh session receives only the bound input, demonstrates the
correct next action, and activates once through the existing reducer and
commit path. It does not repeat the completed effect. A superseding plan,
changed workspace selection, revoked grant, source change at the commit fence,
uncertain reset, or rejected successor response cannot activate it. Test reopen
at the reset and activation boundaries; unresolved effects stay fenced rather
than replayed.

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
live support. The existing [hosted qualification
record](../records/dev-vertical-hosted-qualification-20261117/README.md) leaves
external grant issuance and owner acceptance outside its proof.

## Follow-up experiment, not a fallback

Once the hybrid is qualified, compare semantic-only, hybrid, and
deterministic-only recovery in controlled replay. Hold the coding task,
admitted assignment, underlying evidence, and fault cases constant; vary what
is delivered and verified. Measure accepted recovery, repeated effects, lost
obligations, review-status errors, cost, and latency. A deterministic-only
production profile would need its own explicit admission and verification
contract; omitting a semantic call from a hybrid run does not implement it.

**First PR:** One validated v4 operation, one frozen workspace, and one open
owner obligation reach the real semantic pipeline, with a passing faithful
candidate and a refused contradictory one. No protocol widening, new reset
rights, or production route changes.
