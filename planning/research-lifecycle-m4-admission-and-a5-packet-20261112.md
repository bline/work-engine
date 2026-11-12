# M4 — research admission, A5 and task adoption packet

Date: 2026-11-12. Revision 1. **Planning candidate for review; no implementation or live-use authorization.** Baseline `80d6f80df9e9e7fb5c667fdabe6cd8946dc66b63`. The [M4a authorization](../records/research-lifecycle-m4a-authorization-20261112.md) covers this packet and review. The [M0 approval](../records/research-lifecycle-m0-approval-20261111.md), [M0 LC-1–7](research-lifecycle-m0-contract-and-first-use-20261111.md) and [Rust roadmap M4](research-lifecycle-rust-implementation-plan-20261111.md) retain authority. This packet does not change D-6 or reserved historical-card semantics.

## 1. Result and existing owners

The selected first use remains a bounded repository analysis of `context-boundary.h2-levels`, producing a proposed wording correction or supported no-change report. An A5 recommendation to commission that audit and the later audit are distinct work. Neither changes thresholds, deploys a skill, runs a measurement or installs grants.

The root selects these engineering targets for a future implementation proposal:

| Boundary | Owner and present evidence | Proposed addition |
| --- | --- | --- |
| Qualified research and exact-use authorization | Research/decision owners; `research-register` validates upstream records | Operator-installed trust root and exact approval bindings, with admissible installation evidence |
| A4 projection | `research-register`, accepted M3b current-use card/codec/reconstruction | Production installation constructor only after its protection route is qualified |
| A5 recommendation | Separate advisory task using the existing `retained-repository-analysis-v1` task host/profile | Bounded advisory input assembly and actual consumer-delivery/result binding |
| Preparation and adoption | `retained-task-workflow::TaskStore` | Owned preparation revisions, invalidation history, adoption and reconsideration records |
| Trusted producer admission and lifecycle command construction | `work-engine-task-host` | Startup-admitted producer capabilities and exact research submission/readback |
| Native admission, grant fences, effects and settlement | Existing lifecycle owners | Reuse existing contracts; no new lifecycle state machine |
| Audit acceptance and research feedback | User/design owner and separately established verifier/research authority | Future acceptance and candidate feedback, never automatic standing changes |

Current code is implementation material, not proof that these additions exist. `TaskStore::submit_inner` uses `BEGIN IMMEDIATE` and saves immutable input and its outbound admission command together. `submit_with_admission` is the extension seam. `work-engine-task-host::dispatch` performs exact duplicate readback before constructing a fresh command; `admission_basis`, `build_admission`, `grant_for_context` and `reconcile_admission` retain lifecycle ownership. The native task registry exposes snapshot read, question open and result publication, not task submission or research authorization.

M3b's `Installation::synthetic` is feature-gated; default construction is unavailable. `card_admission::verify` binds exact records, purpose, request, decision revision, disclosure scope and both approval roles. It does not authenticate the issuer merely by reading an approval's fields. A saved reconstruction and its digest establish custody, not authority.

## 2. Production authority route and its acceptance evidence

**P1, proposed route:** extend the existing task host's owner-admitted startup boundary. Add an optional research-admission configuration naming an immutable authority installation, its exact digest, admitted producer roles and bounded artifact roots. Omission keeps research admission unavailable. The ordinary task socket and native message text cannot install or modify that configuration. Keep research authority and lifecycle execution grants separate.

The installation binds the admitting owner/delegation evidence, allowed qualification and applicability issuers, exact purpose/request/decision revision, disclosure scope and exact approval/event pins. Do not interpret a UID, filename, `issuer`, `owner` or `authority_ref` string as the owner's decision. The trusted installer must bind the saved authenticated owner decision or actual delegated grant to those exact bytes. A production constructor may consume that established startup installation; it may not expose an arbitrary public JSON-to-authority conversion or turn the synthetic feature into a production shortcut.

The selected protection claim is limited to the task profile's realized capabilities: owner-controlled startup and artifact storage outside the model's tool surface, with native operations unable to replace them or call producer operations. The existing 0700/0600 local-UID host boundary is reusable but explicitly does **not** isolate hostile processes running as that same UID. Record that exclusion. If the selected model/provider realization exposes shell, arbitrary filesystem/network access, host socket access or another write path to authority material, the proposed protection claim is unestablished until that route is fenced or a separately qualified isolation route is selected. File mode checks and the model's assurance are insufficient.

Production readiness requires attributable installation/caller admission evidence, exact image/source/configuration identities, independently observed realized tool/capability restrictions, rejection of forged or changed installation/approval inputs, and an actual default-build successful admission through that protected route. The user or delegated qualification/applicability owners still supply their respective decisions. This planning packet supplies none of them. A failure of those conditions leaves production admission unavailable; synthetic qualification remains explicitly synthetic.

For the first use, the selected notice producers are (a) a trusted research-publication adapter for assessment/applicability/correction revisions, and (b) a trusted decision/operator adapter for request/target/policy/context changes. Each operation is admitted by a role capability installed at startup and bound to its exact command payload; a payload's producer label cannot select the role. A native task receives neither capability. The initial packet may use an explicit operator producer in place of automated monitoring, with that narrower covered scope stated. It cannot claim to observe all outside changes.

## 3. A5 consumer and an acyclic durable basis

**P2, selected placement:** reuse an advisory repository-analysis task and a separate audit task under the existing task host, with distinct subject/task, command and native context identities. Proposed advisory locator: `research-lifecycle-h2-advice-1`; the M0 audit locator remains `research-lifecycle-h2-audit-1`. These are proposed names, not existing tasks or grants. A general supervisor service, organization compiler or campaign execution path is unnecessary for this bounded analytical use.

The advisory task receives the exact audit-use request, target/context, alternatives, acceptance rubric and qualified A4 card as bounded canonical material. It may recommend the audit, a bounded investigation, more evidence, deferral or escalation. Its attributed result records alternatives, material uncertainty/exclusions and the exact delivered basis. It cannot create a preparation, install authority, submit the audit or grant execution. Both the advisory invocation's own inference/execution authority and the later audit's authority must be established by their existing owners before any live call.

The following downstream records reference already existing immutable bytes, with domain/version-tagged exact UTF-8 byte digests. They are not added to the upstream manifest:

| Record | Required bindings |
| --- | --- |
| Consumer delivery | A4 request/card/reconstruction identities; advisory task/subject/context/attempt; exact model input or tool-read payload that actually included the complete card; realized capabilities and delivery/readback outcome |
| Recommendation | Consumer-delivery digest; exact request/target/context and selected research revisions; attributed advisory result and producer; proposed action, alternatives, uncertainty and exclusions |
| Prepared basis | Upstream source/bundle and retained reconstruction; actual delivery and recommendation; immutable dependency revisions and correction heads; exact audit task/subject, `SubmitTask` payload and approved snapshot; required authorization/install/configuration evidence |
| Adoption | Prepared-basis digest, winning preparation revision, operation/payload digest, task/input IDs and owner revision, exact saved lifecycle command ID/digest, and admitting producer evidence |

Retain all required payloads as bounded immutable owner artifacts before publishing their referencing record. Use the companion's exact-byte revision contract only for new research records; preserve existing lifecycle and task digest codecs. No record hashes itself or a future receipt. Existing `retained-task-input-v1` locators need not change: an owner-owned adoption row can bind the exact immutable input ID to its prepared basis, and canonical task material can expose that basis without teaching lifecycle a research schema.

A projected card or a prepared advisory payload is not consumer delivery. The host must observe actual complete model-input/tool-read bytes and retain their identity through the recommendation; material omitted by context compilation or a truncated/unacknowledged delivery cannot support a claimed delivered card. Define the concrete observation hook and its outcome states in the A5 integration packet before implementing or running it. Missing delivery evidence makes this recommendation/basis unavailable, even if the output repeats a card digest. A lost reply remains unknown; it does not prove non-execution or authorize a new advisory invocation.

## 4. Task-owner preparation, invalidation and adoption

**P3, proposed owner interface:** add `research_admission` within `retained-task-workflow`, using its existing connection, transaction and writer ownership. Names below are engineering targets, not an approved public wire schema:

```text
prepare_research(key, expected_preparation_revision, verified_basis, producer)
invalidate_research(key, notice_id, exact_payload, producer)
adopt_research_with_admission(submit, key, expected_preparation_revision,
                              prepared_basis_digest, producer, build_command)
read_research_preparation(key)
read_research_adoption(command_id, original_payload_digest)
```

`key` binds task, subject and audit-request digest. A monotonic preparation revision is distinct from task/cursor/lifecycle revisions. Preparation stores immutable dependency references and the checked basis, not a second research assessment. Producer arguments are host-established role capabilities; do not deserialize an internal authority permit from these DTOs. Tests may construct explicitly controlled qualification capabilities; their presence must not create a default production constructor.

The research command's idempotency digest covers the complete submit plus key, expected preparation revision and prepared-basis digest, under its own operation kind. Hashing only `SubmitTask` could return a receipt for a different research adoption. Canonical basis sources use an owner-reserved namespace that ordinary caller facts cannot populate; their trust comes from the saved adoption and installed producer, not the text's labels.

Use owner-owned tables for preparation head/history, processed notices, adoption and reconsideration obligations. Give the owner schema a new version and a tested migration from the actual schema-2 store that preserves store identity and existing task/command/effect bytes. No archive, reset, broad schema-compatibility framework or historical data rewrite is implied. Unsupported/corrupt schemas refuse before mutation.

Preparation is a compare-and-swap on the exact current preparation revision, including an explicitly absent initial head. It verifies the full immutable dependency set and producer scope before a ready revision is stored. A notice increments the preparation revision and marks the affected preparation ineligible. The owner records the notice even when it arrives before first preparation; a later preparer cannot erase that head by creating a fresh slot. A key's dependency selectors and producer scope must therefore be admitted before it can become ready. Preparation against the new head must resolve the notice using changed checked inputs, preserving the notice history; changing only a digest or replaying the former basis cannot clear it.

The named research/decision producers may record relevant changes. Only the trusted preparer may establish a newly checked ready basis. Native task operations cannot perform either role. Exact notice retries return their original receipt; reusing a notice ID with a changed payload conflicts. Record reason and source revision, without treating an invalidation as revocation of an owner's grant.

Within one task-owner `BEGIN IMMEDIATE` transaction, adoption:

1. Resolves an exact duplicate command before consulting a newer preparation. A mismatched operation kind, task, payload or basis binding conflicts.
2. Requires a ready preparation with matching task/subject/request, expected preparation revision and exact basis. Checks installed producer scope and all prepared admission requirements; independently obtained execution authority remains required.
3. Saves immutable task input/canonical basis material, the exact outbound lifecycle command and adoption receipt together. Any refusal/build failure rolls back all three. A transaction cannot fetch remote evidence or invoke a model to decide readiness.

The host obtains lifecycle preflight through the existing grant/context owners; the task transaction does not make that read atomic with lifecycle. The lifecycle command and later fence retain their existing revision/grant checks. The prepared basis binds the approved submit payload, not a prediction of a future command digest; the adoption receipt adds the actual command identity.

Trusted invalidation and adoption serialize through this same connection/transaction. A notice committed first makes the earlier preparation comparison fail. A notice committed after adoption appends a reconsideration obligation referencing the exact adoption/basis; it does not rewrite input, revoke grants, retract an already accepted command or imply settled execution. The decision owner chooses reconsideration or any separately authorized stop/contraction. Unobserved outside changes remain outside the stated guarantee.

**Guard the alternate route:** after a task/request is registered as research-governed, every fresh ordinary submit path, including library `submit`, `submit_with_status` and `submit_with_admission`, must reject a bypass. Exact old command readback remains available. Trusted configuration also guards the nominated audit task before first preparation, so omission of the key cannot evade the gate. A fresh research submit must not append unchecked input to an existing task. Unrelated ordinary tasks keep their existing semantics.

Adoption is task-input admission, not lifecycle admission or task completion. Preserve the existing selected-host reconciliation rules: an authorized retry can send only the exact saved lifecycle command where the existing policy permits it after a valid domain-unknown readback. Read failure, absent evidence or a newer preparation does not authorize rebuilding a command with new context, grant or basis. Recovery-only startup retains exact readback and refuses fresh mutation. Later notices do not cause automatic re-adoption or a new native attempt.

## 5. Bounds and truthful failure

For the first M4b fixture contract, propose identifiers of at most 128 UTF-8 bytes; 16 request keys per nominated task; 256 dependency pins and 128 immutable artifact members per basis; 128 KiB per new basis/delivery/recommendation metadata record; and 8 MiB total retained artifact bytes per preparation. Keep payload bytes outside the ordinary RPC and bind them through the admitted artifact root. Queries return at most 64 complete rows, with a 65th producing capacity refusal rather than partial completeness. A task may retain at most 64 adoptions and a key 256 preparation revisions and 1,024 detailed notices before new work requires an owner storage disposition. These are bounded engineering targets, not universal lifecycle policy.

Capacity cannot leave a known-invalid ready slot usable: if a notice cannot be retained normally, the owner must atomically block the affected head for admission and retain a bounded capacity/notice-digest marker before acknowledging it. Failure to commit that block is unavailable, never a successful invalidation acknowledgment. A full head never becomes ready by dropping history. Preparation revision overflow is a permanent admission refusal pending owner disposition. Test these fail-closed boundaries, not just successful inserts.

Preserve the existing 128 KiB host envelope, 8 KiB native input locator and task text limits; the stricter applicable bound wins. Validate retained referenced bytes before preparation; an arbitrary path or digest is not a capability. Unknown versions, duplicate keys, missing payloads, scope mismatches and limit exceedance refuse before partial publication. No global unbounded dependency scanner is needed.

Unavailable authority, delivery, input, producer or owner readback remains a distinct unavailable/refused result. Do not turn it into `absent`, an empty complete set, a qualified card or a fresh grant. Queries must distinguish complete empty results from capacity refusal. Trusted preparation and ordinary read APIs expose only their declared task scope. Diagnostics must not disclose approval payloads or unrelated research content.

## 6. Required evidence and validation

| Gate | Required observable result |
| --- | --- |
| Authority split | Forged labels/configuration/approval pins and native producer calls cannot prepare, invalidate or install authority; absent production installation refuses |
| Exact basis | Wrong finding/study, request, decision/claim revision, target/snapshot, delivery or recommendation is rejected before publication; all eight uncertainty facets and competing evidence survive unchanged |
| Revision comparison | Task/cursor/lifecycle revisions cannot substitute for preparation revision; stale expected preparation loses |
| Notice ordering | Controlled before-adoption notice prevents input/command/adoption; post-adoption notice preserves the original basis and appends exact reconsideration; pre-prepare and duplicate/conflicting notices are covered |
| Bypass | Guarded task submission through every ordinary owner/host path cannot omit the research gate; unrelated tasks and exact old readback remain valid |
| Atomic owner commit | Failure before/within commit leaves no partial publication; crash after commit returns the same input, command and adoption without a newer preparation or duplicate task revision |
| Host reconciliation | Grant/context change rejects through the existing owner; valid unknown versus unavailable lifecycle readback remain distinct; only existing authorized same-command retry, never rebase |
| Recovery and result | Recovery-only readback, cancellation/unknown effect states, exact receipt-to-basis association and retained-copy reconstruction; no result text is promoted to verification |
| Resource and privacy | Exact limits at/beyond boundary, complete-empty/capacity distinction, malformed/missing retained bytes and no unrelated content disclosure |
| Data disposition | Schema-2 migration preserves original store/task/command/effect identities; interrupted/corrupt migration refuses truthfully and can be recovered under the owner contract |

Synthetic owner/host fixtures establish local engineering behavior. They do not establish production protection, actual A5 delivery, a live provider, owner research acceptance or executed H2. A later actual-host proof must identify its default/controlled build, source/image/configuration, observation hooks and capability realization, preserving failed attempts. Keep M3b's timing-sensitive qualification limitation rather than claiming it was repaired by M4 tests.

## 7. Smallest next engineering slice and later readiness

**M4b proposal: task-owner research preparation/adoption, with controlled host qualification.** Own `rust/crates/retained-task-workflow/src/research_admission.rs` (new), its existing `src/lib.rs`, tests and component documentation; and `rust/crates/work-engine-task-host/src/research_admission.rs` (new), existing `src/lib.rs`, focused host tests/documentation and only necessary Cargo declarations. A test fixture's versioned bounded DTOs belong with those owners. The builder confirms final path/consumer impact before its packet is accepted; adding lifecycle/kernel, provider, UI or campaign behavior is outside this slice.

Implement P3's state and transaction behavior, ordinary-route guards, typed host admission seam and controlled qualification producer. Default startup without a qualified production installer refuses fresh research prepare/adopt/invalidate. Exercise the table's owner/host gates with synthetic exact basis/delivery/approval evidence, clearly labelled. Reuse existing private canonical material/command records; no parallel task executor, research evaluation or A5 inference runner. Qualify affected default/controlled crate tests, focused crash/race/actual-host controls, formatting/Clippy and joined workspace checking. Retain inherited evidence when unchanged and disclose any pre-existing workspace limitation.

Recommended M4b allowance, **not approved here**: **$14 subscription cost proxy**, warning **$9**, no new engineering/review dispatch at **$12**, reserve **$2** for collection and closure; expected **$10–12**. This includes implementation, controls, separate Astra review, ordinary corrections/recheck and closure. Section 5 pins the proposed engineering limits; before dispatch, freeze exact fixture/control inputs and the final owned path set against the accepted packet. A structural draft is not permission to invent authority or widen this slice.

**M4c readiness/integration prerequisite:** qualify P1's production protection/installation route and P2's actual A5 delivery hook on the selected existing host, then bind real research acceptance, exact audit-use applicability, operating context, task/provider/startup and execution grants. Its engineering scope and budget depend on the M4b result and the selected capability realization; no reliable total is claimed now. The operator's owner/delegation decisions and installation evidence remain outstanding. If those cannot be established, report that condition instead of enabling synthetic authority in production.

Before either live arm, M0 LC-7 still requires a frozen paired exploratory protocol: common objective/snapshot, model/effort/tier, tools, budgets, rubric and exposure evidence; ordinary policy baseline contamination and blinding limits; shared/setup cost allocation; refusal/failure/unknown outcomes retained. One pair is a feasibility result. Do not launch the research-assisted arm first and retroactively invent its comparator. M5 feedback remains candidate observations after verified execution, without automatic research standing.

## 8. Evidence scope

Tier-2 graph project `home-bline-code-work-engine`, generation `2026-11-12T19:14:30Z`, followed by exact source fallback for unresolved calls. Material inspected paths: `rust/crates/retained-task-workflow/src/lib.rs` (input/record types, opening/schema, submit transaction, command validation and locator), `src/port.rs` (native operation boundary); `rust/crates/work-engine-task-host/src/lib.rs` (startup/dispatch/grant/construction/reconciliation); `rust/crates/research-register/src/card_admission.rs` (complete installed-authority verifier). `card_host.rs` was identified for the inherited reconstruction boundary; M3b's accepted result owns its qualification. The [task host contract](../docs/rust-retained-task-host-contract.md) supplies current supported boundary semantics, not a new live qualification verdict.

Graph search/trace is discovery evidence; its unknown call totals and clean content checks do not prove repository-wide completeness or runtime protection. No claim that every possible A5 implementation was ruled out is made. The root's bounded choice reuses the already selected task owner, avoiding an additional runtime. Exact commands, coverage/fallback record, immutable review subject and result are in `/home/bline/cbm-work/research-lifecycle-m4a-planning-20261112/`.
