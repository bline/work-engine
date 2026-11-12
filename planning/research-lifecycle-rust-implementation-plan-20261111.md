# Research lifecycle: Rust implementation planning

Date: 2026-11-11. Status: **planning candidate; implementation is not authorized**.
Companion design: [workflow v0.2](research-knowledge-lifecycle-and-decision-workflow-v0.2-20261111.md).
Planning source HEAD: `1eaa698f6ade66097b4e7d8a8556983e056d570d` on
`rescue-detached-20260911`. Refresh affected subjects before future execution;
this HEAD is evidence identity, not a lock on concurrent work.

## 1. Objective, scope and readiness

Deliver one governed research-informed Work Engine decision with a preserved
contemporaneous basis, separately established execution authority, bounded
execution, independent verification where required, and candidate research
feedback. Measure a justified exploratory baseline alongside it.

The current pass corrects the design, reconciles existing work and develops
this plan. It may write planning documents and obtain bounded advisory review.
It does not implement A4, resume R-2, accept reserved semantics, integrate or
distribute R-2, mutate the research register, or authorize a live experiment.

Implementation is not ready for dispatch as one unconditional packet. The
meaning of the subject/authorization binding, historical products, change-at-use
policy and selected consumer must be accepted first. Several later work items
can be scoped now without pretending those decisions have been made.

The first result does not require an organization compiler, a new runtime,
automatic research spending, generalized scheduling, global confidence scores,
distributed sharing, or a parallel grant/receipt system.

## 2. Reconciled inputs and their standing

| Input | Standing and consequence |
| --- | --- |
| [Saved v0.1 principles](research-knowledge-lifecycle-and-decision-workflow-v0.1-20261111.md) and full draft in the owner's Codex message | Agreed direction plus proposed details; the saved file is a partial capture, not the full draft |
| [Phase handoff](research-to-decision-workflow-contract-handoff-20261111.md) | Defines the nine design questions and carries blockers; the current user request authorizes this planning pass |
| Integrated research-register at merge `6f3d6c71`, included at source HEAD | Accepted whole-register pinned reader/validator; its validity proof does not establish scientific adequacy or a runtime grant |
| [R-2 checkpoint](../records/research-register-r2-qualification-checkpoint-20261111.md) | Repair `0d021609482ad89841eb94f5c0a156e328efb7a6` remains unintegrated; original independent REJECT, delta recheck without verdict, coordinator ACCEPT WITH RESERVATIONS |
| A4 v1 specification and its seventeen frozen cards | Proposed technical baseline and regression evidence, subject to owner decisions; obsolete Python placement is not authority for the Rust design |
| U1–U3 and B4 approval record | Existing approved disclosure and historical safeguards, preserved by v0.2 |
| R-1a Rust direction and later readiness handoff | Rust and in-process anchors against the pinned snapshot; D-5 Python amendment is obsolete |
| Coordination board through sequence 4997 | Ownership/disclosure history and register integration notice; no board claim or coordinator opinion supplies missing owner approval |

Off-repository evidence is host-local. These exact inputs were read; their
paths are not portable publication artifacts:

| Artifact under `/home/bline/cbm-work/` | SHA-256 |
| --- | --- |
| `a4-implementation-planning-20261110/user-authorization-and-decisions.md` | `3f70ab37ef46b1e4f07fe99e4fa86bfdd4d0a64bae42f1fd7b347842b78d8867` |
| `a4-spec-finalization-20261110/a4-v1-specification-v1.md` | `4057e1d35bf844c7d28d3bbb2c67d35439624488696f150bbc29814696fc92bd` |
| `register-integration-20261111/a4-readiness-handoff.md` | `1b4ae7c88d7a07007d94f6e3ccf88995db59bf9a81d77db572b6c7656b36b570` |

The R-2 checkpoint names the candidate bundle, preservation manifest and frozen
review packets. It does not bundle the host-local review evidence. Any future
review must bind the required source/evidence bytes rather than assuming that
the branch name or preservation bundle contains every referenced artifact.

## 3. Ownership and placement boundaries

`rust/crates/research-register` already owns pinned reading, catalogue decoding
and rule validation. `PinnedSnapshot` and privately constructed `ValidRegister`
provide the current admission boundary. Extend this owner deliberately where
new research records require it; do not introduce another reader that decides
register validity independently.

The integrated catalogue represents `Judgment`, `Applicability`, `Decision`
and materialized claim references. It is not already a complete implementation
of the v0.2 research/assessment revision model. Adding fields without changing
the catalogue/validation contract is not an implementation strategy. Decide
whether minimal domain-owned record extensions or an explicitly joined owned
companion record meet the accepted semantics. Either choice needs exact
revision linkage and qualification; avoid a second canonical standing value.

A4 can begin as a module and CLI surface using the register's validated input
and same-snapshot anchor API. Extract a public projection crate only if its
consumer/dependency boundary justifies it. Public wire records remain distinct
from internal authority permits. Unknown schema/codec versions fail explicitly
under their domain contract; existing content identities must not be silently
re-encoded through a new universal digest helper.

The A5/execution integration requires a named existing consumer and its actual
admission/receipt contract. The presence of a Rust `Grant` type or similarly
named function does not establish a production grant API. A narrow adapter
should attach research-basis references to that owner's request/receipt path
without taking over grant issuance, runtime effects or settlement.

Decision-readiness, portfolio selection and experiment-control are candidate
neighbors, not interchangeable lifecycle owners. Their readiness/acceptance
and shared-workspace compatibility must be established before selecting one.
The bounded reconnaissance found the following concrete seams and limits:

| Existing Rust material | Planning consequence |
| --- | --- |
| `research-register/src/model.rs` and `src/validate.rs` | Judgment/applicability records and validated pins are reusable; assessment-revision identity, full scientific uncertainty and authenticated authority remain work |
| `decision-readiness/src/lib.rs`, `src/model.rs`, `src/check.rs`, `src/evaluate.rs` | Reuse its decision-relative sufficiency contract if suitable; checking structure/digests needs evaluation rederivation beside it and does not prove provenance or observe live state |
| `decision-readiness/Cargo.toml` and `portfolio-decision/Cargo.toml` | Each is a standalone workspace, outside the shared workspace membership; choose a qualified CLI composition or deliberately qualify library/workspace integration |
| `portfolio-decision/src/model.rs` | Its decision record is portfolio-specific; it cannot serve as a universal research decision receipt |
| `retained-task-workflow/src/lib.rs` (`SubmitTask`, `TaskCommit`, `submit_with_admission`) and `work-engine-task-host/src/lib.rs` (`dispatch`, private `grant_for_context`) | A concrete candidate for a repository-analysis task; owner input and lifecycle admission already compose. An analysis profile cannot authorize implementation effects. The nearby `grant` function is test-only |
| `slice-campaign/src/recovery.rs` and `src/completion.rs` | Candidate owner for an engineering campaign if that effect is selected; applied/replayed/no-effect/unknown outcomes and execution evidence retain their campaign meaning |
| `experiment-control/src/model.rs` | Its experiment-resource grants and delivery receipts are not general engineering execution grants or receipts |
| `claim-evidence/src/contract.rs`, `src/reliance.rs`, `src/findings.rs` | Exact revision/consumer reliance is reusable material, but native review findings have a constrained payload; a scientific-research profile needs explicit compatible admission rather than relabeling |

These paths are relative to `rust/crates/`. Source snippets and manifests were
checked in the bounded reconnaissance; no live composition or new dependency
was qualified. The report is
`/home/bline/cbm-work/research-lifecycle-planning-20261111/recon.md`. No direct
dependency on the candidate owners is accepted by this plan.

## 4. Owner decision packet before code dispatch

The following proposed choices make implementation reviewable. They do not
convert the review recommendations into approved product rules.

| ID | Decision needed | Proposed direction / remaining choice |
| --- | --- | --- |
| LC-1 | Research subject and applicability authority | Bind exact claim meaning/revision and decision revision; explicitly define the effect of reassessment on existing authorization scope |
| LC-2 | Qualification and competing assessments | Name the authorized assessment publishers, qualification consumer, required independence evidence and unresolved-conflict behavior; A4 projects their records |
| LC-3 | Historical basis | Preserve actual delivered inputs and authority state; distinguish later historical association and present reassessment; settle the D-8 interaction |
| LC-4 | Change-at-use policy | Name the consumption/admission boundary, required context/change checks and disposition of a known material change, including an admission interleaving |
| LC-5 | Minimal durable records | Choose catalogue extensions versus a joined owned companion record, exact identities/encoding, publication consistency and retention sufficient for reconstruction |
| LC-6 | First real decision and consumer | Select an actual decision revision, owner, task, evidence, operating context, grant path and receipt owner; demonstrate the adapter without creating a general runtime |
| LC-7 | Comparison | Bind baseline, exposure, acceptance and cost accounting before outcomes; the initial conclusion remains exploratory |

The earlier owner-reserved A4 items also need disposition:

| Item | Current standing / required disposition |
| --- | --- |
| D-1 | This planning request is authorized; A4 engineering objectives still need their own scope and budget |
| D-2 | Confirm the pending-notice identification and whether R/X in the approved notice are per-record substitutions; retain the approved wording meanwhile |
| D-3 | Reconcile and accept the amended technical baseline against the Rust design; no blanket adoption of the old Python composition route |
| D-4 / D-5 / D-7 | Later Rust direction is the applicable language/snapshot baseline; the Python amendment is obsolete and anchors are in process |
| D-6 | Approved: invalid register yields no card |
| D-8a–c | Historical grouping of current-revision records, declined records at another revision, and cross-decision declined records |
| D-8d–f | Historical correction closure/current lists; historical markers without successor pointers; current `decision_finding` delivery and its non-applicability label |
| D-8g–i | Authorization timing against supersession; historical cross-decision key set; exact historical/time wording; distinguish later association from original basis |
| D-9 / D-10 | Future implementation/reviewer model, tier and budget selections; the current planning assignment's selection does not authorize future paid work |
| Q-R2-2 | Rename/unqualified-anchor policy: limited affected delivery versus refusal; neither policy may infer establishment or `absent` from null |
| Q-B1 | Resolve native diff provenance/licensing through its owner before R-2 integration/distribution under the saved handoff |
| R-2 acceptance | Obtain a completed independent verdict on the exact candidate, or an explicit owner decision accepting different evidence/limitations; coordinator acceptance remains separately attributed |

The old matrix identifies D-8g timing against supersession, not actual decision
consumption. LC-3 therefore cannot be implemented merely by copying that timing
field. A later authorized historical association is still not a contemporaneous
decision basis.

## 5. Proposed bounded implementation sequence

These are coherent outcomes and dependencies. Independent preparation can
overlap; no mandatory phase order is implied beyond the named prerequisites.

### M0 — Accept the minimum contract and bind the real use

Owner: design/decision owner with the implementation lead.
Output: dispositions for LC-1–7 and the A4 choices required by the selected use;
exact source references, fixture semantics and concrete acceptance conditions.
Select one already real Work Engine decision. A bounded engineering configuration
choice is a candidate if the actual research links, anchor, owner and task are
qualified; this plan invents no authorized relation to make it convenient.

Exit: a code packet can distinguish valid from invalid states without asking
the builder to decide reserved semantics. Remaining choices are explicitly
excluded from the slice rather than hidden in a catch-all policy enum.

### M1 — Establish the minimum research/basis records

Owner: research-register and the selected receipt/basis owners, with one named
integration lead. Work: implement only the accepted claim/assessment subject
binding, uncertainty facets and durable basis references required by the first
case. Keep prior observations and assessments reconstructible. Bind exact
external evidence references without making a mutable URL a sufficient basis.

Validation: changed claim under unchanged decision ID/revision; assessment
conflict; unknown versus unassessed; unknown versions; malformed/missing links;
publication interruption and reconstruction. Preserve existing register rule
qualification and give any schema change an explicit data disposition.

Exit: one fixture's research and basis can be pinned and validated without
conferring applicability or execution authority.

### M2 — Resolve and qualify the R-2 dependency

Owner: R-2 owner plus the acceptance/distribution authorities. This is a
dependency lane, not authorization to resume it. Bind candidate `0d021609`, its
base and frozen evidence; resolve Q-B1 and Q-R2-2; complete the required recheck
or obtain the owning alternative acceptance disposition. Refresh the evidence
if a provenance repair changes the implementation.

Retain the executed F-01 discrimination and historical regression evidence.
Classify missing packed objects, post-validation ambiguity, resource bounds,
history-dependent CI and workspace qualification against the accepted use.
They are recorded limitations; this plan does not silently promote all carried
items to blockers or waive a limitation required by the selected environment.

Exit: exact accepted anchor API and same-snapshot semantics suitable for the
chosen A4 use, with truthful scope and resource/admission evidence.

### M3 — Implement the minimum Rust A4 projection

Owner: decision-aware projection owner; register and claim owners supply their
own readers. Inputs: validated pinned register, accepted anchor interface,
exact decision request and scoped evidence reads. Outputs: versioned card or a
distinct failure; invalid input cannot produce a degraded card.

Preserve approved disclosure, historical-membership and notice semantics.
Represent orthogonal facets without deriving scientific adequacy from a
display category. Report selection/truncation limits and unavailable exact
claim copies. Do not add semantic research, question routing or grant issuance.

Validation: the original seventeen cards after explicit re-derivation where
owner choices change them; the v0.2 scenarios; malformed/unreadable register;
unqualified anchors under the chosen policy; disclosure scope; no network fetch
or register mutation hidden in projection; stable identities and reconstruction.

Exit: qualified projection behavior for the selected current and historical
uses, with independently reviewed semantics and frozen regression evidence.

### M4 — Bind one A5 recommendation to existing admission and execution

Owner: selected A5/decision consumer and execution/admission owners. Supply the
card, relevant uncertainty, operating context and existing capabilities to the
supervisor. Record its recommendation and exact basis. Apply the accepted LC-4
checks at the selected boundary; use the owning grant and receipt mechanisms.

Validation: wrong decision/claim revision, out-of-scope or missing grant,
correction after card issuance, change during admission, cancellation and
unknown effects, crash/retry, receipt-to-basis mismatch. Demonstrate the real
task's acceptance conditions; a successful CLI return alone is insufficient.

Exit: one actually authorized bounded execution with its basis and required
verification evidence. If the selected host cannot establish required evidence,
report the unmet condition rather than widening this into a runtime rewrite.

### M5 — Publish candidate feedback and compare

Owner: feedback publisher under research authority; verification and research
qualification remain separately owned. Convert the receipt into a candidate
observation, preserving conditions, failures, cost and unavailable values.
Prevent retry/import from counting the same source as independent replication.
Exercise one correction/reconsideration path for the selected basis.

Report the predeclared comparator and exposure limits, research/setup cost and
per-decision cost, accepted outcome and failures. A study claiming broad benefit
is a later separately qualified objective.

Exit: the complete chain is reconstructible, candidate feedback is admitted
without automatic promotion, and the exploratory comparison is reviewable.

## 6. Validation ownership and qualification

Use deterministic checks for shape, identity, revision linkage, approved
selection and grant/receipt consistency. Leave evidence adequacy, transfer and
risk judgment with their attributed owners. Preserve actual producer/reviewer
identity and the evidence required by any independence claim.

Run targeted crate tests and required lint/schema checks for changed owners.
Run joined-workspace checks when integrating shared interfaces or dependencies;
crate-only success does not establish composition. Use historical tests with
the history they require and disclose skipped coverage. New code follows the
workspace's pinned toolchain/dependencies and appropriate offline/locked gates.

Use immutable review subjects. A retained reviewer can check remediation against
the exact new subject; changed model, subject or continuity is recorded rather
than presented as the same evidence. A separate design review is advisory and
does not independently accept R-2 or settle licensing.

## 7. Future dispatch, budget and stop conditions

No engineering cost forecast is established while the contract and selected
consumer remain open. Price the first code packet from its concrete scope and
the actual chosen model/host, including root, review, repair and closure.
Allocation caps are proposed controls, not measured cost predictions.

The prior R-2 Pi recheck stopped with only USD 0.5159 consumed because the next
request's USD 1.5374 worst-case reservation exceeded the USD 2.00 cap. If that
route is selected again, size its grant for reservation as well as expected
spend. Do not transplant those dollar figures into another host's pricing.

Use manual-orchestration scope records, a verified context-status binding and
collected chatboard budget/context observations. Stop admitting work at the
applicable limit, collect outstanding results, and preserve unfinished owners.
Unknown telemetry or unpriced work remains unknown. Any implementation launch
needs its own scope, models, authority and budget; the planning cap is not a
standing engineering grant.

## 8. Planning-pass artifacts and limitations

Root owns the two new planning documents. The dated v0.1, phase handoff and R-2
checkpoint remain unchanged. Supporting assignment, reconnaissance, review,
cost and continuation records live under
`/home/bline/cbm-work/research-lifecycle-planning-20261111/` on this host.

Connected Events Server transcript discovery required reauthentication, so
related Claude work was reconciled through the local saved artifacts and
repository source. This is bounded discovery, not a claim that every Claude
session or unpublished branch has been exhaustively inspected.

This pass validates documentation and its evidence references. It provides no
new runtime, workspace-build, R-2 acceptance or distribution qualification.
