# Review: same-session context continuity — subject-v1 (`dbbefc6c…`)

**Verdict: `changes_required`.** The proposal gets its central idea right: same-session compaction is a different operation from transferring the root, and continuation must be covered by current authority. But the subject leaves out several distinctions it needs:
- when newly arrived human direction stops an irreversible compaction;
- which human input paths the ordering guarantee covers;
- where the automatic wakeup sits in the input order;
- who may start or dispatch a transition;
- what the root is actually told.

Several pilot cases would pass a realization that violates the contract.

**Durable state:** the episode was begun at revision `c6db38b7a60eaa97a1c3d8c3e6a9f5f93a9d97a4`. The initial result was recorded with transition `same-session-context-continuity-opus-20261028-initial-result-1`, giving durable revision **`e4d7732fea579c3b45e77aa5b1267c6741dbfb88`** at ref `refs/work-engine/independent-review-state/84a85818…`. Read-back confirms 11 open findings, status `active`, `pending_next_action: await_remediation`.

**Specialist applicability (agent-instruction-review): applicable.** Two parts of the subject are normative instructions:
- The proposed contract insertion (`proposal.md:40-114`) governs host designers and implementing agents. Most of its commands carry their reason inline.
- The changed `context-boundary.md:16, 18-40` is a conditional reference loaded by manual-orchestration parents and roots. Its gaps are in SSC-OPUS-3 and SSC-OPUS-6.

This is advisory review only; it carries no acceptance authority. The contract stays proposed, with `implementation_authorized=false`.

## Findings

### SSC-OPUS-1 — P1: new human direction vs. irreversible compaction
- **Where:** `proposals/agent-state/same-session-context-continuity/proposal.md:47-58, 64-70`; `pilot.md:65-66`.
- **Missing distinction:** new human input treated as a *content change* (recompile and carry on) vs. as an *authority change* that may cover the pending compaction itself. Likewise, pausing domain work vs. restricting the compaction.
- **Evidence (observed):**
  - The proposal reuses the existing lease: competing input invalidates readiness.
  - In the existing loop, a changed revision goes back to recompilation and readiness is granted again (`semantic-context-lifecycle-manager.md:601-606`).
  - An `open` + `exact` interaction is a valid disposition, and `escalate` is optional (`:475, :482-490`).
  - Abort is allowed only from `reserved` (`context-lifecycle-transition-protocol.md:542-549`).
  - Retirement cannot be undone (`manager:708-713`).
- **Failure path (inferred):**
  1. The human replies "pause, don't refresh" to the announcement.
  2. The lease is revoked.
  3. The message is compiled as `open`/`exact`.
  4. Readiness is granted again.
  5. Compaction runs anyway.

  Every pilot consequence still holds, so the pilot accepts this run.
- **Minimal remediation (restate):**
  - Before readiness is granted again, assess new input for its effect on the pending lifecycle operation. Input whose scope covers, or may cover, that operation leaves re-actuation unestablished until current authority resolves it.
  - Reason: the operation is irreversible, which is the constraint condition in DESIGN.md §3.9.
  - Add a pilot case where the pause targets the compaction itself, with the consequence "no actuation ordered after the restriction".
- **Confidence:** high for the pilot gap; medium-high for the failure path.

### SSC-OPUS-2 — P1: which human input paths the ordering covers
- **Where:** `proposal.md:64-67`; `pilot.md:31-33`, `:55-57` ("Use synthetic input").
- **Missing distinction:** input that goes through the host (a lease can revoke it) vs. input the human sends directly through the provider's own client (the host can at most observe it).
- **Evidence (observed):**
  - The existing ordering depends on a mediated switchboard plus rejecting direct provider routes, and the document says these are not universal pause proofs (`manager:633-649`).
  - The reference gate "does not make unmediated ingress safe" (`manager:626-631`).
- **Inferred:** a Codex root is a daemon thread that the user's own terminal reaches directly (`context-boundary.md:145-149`).
- **Consequence:**
  - The reused lease cannot see that input, so ordering is unestablished in exactly the human-facing case this contract targets.
  - Synthetic input sent through the host's own path only proves a narrower claim (DESIGN.md §3.9, claim identity). Acceptance would then over-generalize.
- **Minimal remediation (restate):**
  - The ordering claim should cover every path by which human direction can reach the role. Any path the host neither mediates nor can order before actuation leaves the claim unestablished, mirroring the existing transport sentence at `proposal.md:78-79`.
  - The pilot should freeze the list of human-usable input paths and inject race input through each one, or mark a path unsupported.
- **Confidence:** high on the reuse premise (observed); medium on the root's input topology (inferred).

### SSC-OPUS-3 — P1: the changed root guidance reads as permission and omits what the actor needs
- **Where:** `skills/manual-orchestration/references/context-boundary.md:16, 20-23, 25-29`.
- **Missing distinction:** permission vs. limitation. Also missing: the consequences the agent who actually performs the route must protect.
- **Evidence (observed):**
  - The new text offers same-session compaction as an alternative to transfer, and says lifecycle control and resumption "remain inside the current role, host and user authority".
  - The proposal's condition is missing from the loaded text: coverage only "when the governing role and host contracts permit" (`proposal.md:47-50`).
  - So is its boundary: the clarification "does not … authorize an improvised controller" (`:126-128`).
  - The agent loading this text is the kind of actor that performed S10: a manual root that combined compaction, its own controller and a user-role wakeup (`formation-evidence.md:56-61, 78-82`).
  - The text covers authorship and recovery. It does not cover ordering human input across the boundary, not repeating compaction or wakeup after a restart, or a notification route when the turn ends.
- **Delta (observed):** a 2026-10-27 copy (`~/.local/state/work-engine/anthropic-builder-baselines-20261027/opus/final-review-repository/…/context-boundary.md:16`) reads "at a quiescent boundary, write a root handoff; the user starts the successor". I did not verify that this copy matches the recorded pre-change digest `3165149e…`.
- **Consequence (inferred):** once released, a root under pressure can read this as permission to self-compact and auto-resume with the tools it has today. That reproduces S10's separate input check and the limit of one attempt that did not survive a controller restart, with no supported host behind it.
- **Minimal remediation (restate; no approval gate):** either
  - state that no role or host contract currently establishes this route, so automatic same-session resumption is unestablished (recovery from provider-initiated compaction still goes through Wind Walker); or
  - give the root the minimal consequences, each with its reason.
- **Confidence:** high on the text; medium on the behavioral consequence.

### SSC-OPUS-4 — P2: who may start or dispatch a transition is unassigned
- **Where:** `proposal.md:22-24, 47-50, 83-86, 104`; `packet.json:18`.
- **Missing distinction:** who *requests* a transition vs. who *decides* readiness and actuation vs. who *dispatches* the effect. Also, effects the host dispatches vs. effects a role dispatches itself.
- **Evidence (observed):**
  - The owner document keeps roles out of pressure and persistence decisions (`manager:49-53, 62-73`), backed by invariant 9 (`manager:964-965`).
  - It rejects direct compact routes from role-owned threads (`manager:644-649`).
  - Key invariant 1 of the design view says "The model never decides 'my context is too full'" (`design/architecture/context-lifecycle.md:143`; status proposed, names this document as owner at `:197`).
  - Wind Walker, by contrast, leaves it to model judgment whether replacement improves the work (`SKILL.md:25-27`).
- **Consequence (inferred):** the insertion either contradicts the owner document or says nothing on the point. A role holding covering authority that dispatches compaction itself (the S10 shape) falls outside the custody clause, which binds only "effects it [the host] dispatches". The duplicate-effect and ordering obligations then bind no one.
- **Minimal remediation (restate/split):**
  - Say whether a role may request or decide a transition, and that the host keeps readiness and admission.
  - Say that any lifecycle effect not dispatched under the host's durable transition operation leaves this contract's ordering, custody and no-duplicate claims unestablished.
- **Confidence:** high on the text; medium on the consequence. The design view is outside the frozen manifest and I computed no digest for it.

### SSC-OPUS-5 — P2: the wakeup's place in the input order is undefined
- **Where:** `proposal.md:66-69, 92-95`; `pilot.md:67-68`.
- **Missing distinction:** a host-originated resumption input vs. queued human input. Also, "participates in successor reconciliation" vs. the existing "released after accepted reconciliation".
- **Evidence (observed):**
  - Queued input is released only after reconciliation is accepted, in custody order, to the domain scheduler (`protocol:561-570, 120-121`; `manager:633-641`).
  - Reconciliation checks the next action while that input is still held in custody (`manager:674-682`).
  - The rehydration turn has no tool access (`manager:694-697`).
  - The wakeup is given no admission identity.
- **Failure path (inferred):** a pause queued after actuation is not yet visible when reconciliation validates the next action. Then either the wakeup resumes the paused work, or the pause is delivered first and the later wakeup restarts that work.
- **Minimal remediation (restate):**
  - Give the automatic continuation its own admission identity in the same order as human input.
  - Check its productive admission against every human input ordered before it.
  - Mark the change from release-after-acceptance as a delta to the protocol, or define how the two map.
- **Confidence:** medium-high.

### SSC-OPUS-6 — P2: the packet and the contract disagree on the authority default
- **Where:** `packet.json:23` (`reopening_conditions[0]`) vs. `proposal.md:47-50`.
- **Evidence (observed):**
  - The packet reopens only if an owner *reserves* the operation, which implies it is permitted by default.
  - The contract requires an *affirmative* permission from role and host contracts, and names no such contract.
  - `context-boundary.md:28-29` supports either reading.
- **Consequence:** the decision owner may accept on a default-permit premise while agents hold a different default (see SSC-OPUS-3).
- **Minimal remediation (restate):** match the reopening condition to the affirmative condition, then either name the contract expected to grant permission or record that as an open owner decision.
- **Confidence:** high.

### SSC-OPUS-7 — P2: the authorship rule assigns a model-behavior outcome to the host, and its pilot case doesn't discriminate
- **Where:** `proposal.md:73-79`; `pilot.md:69`.
- **Missing distinctions:**
  - Host-side attribution (envelope, ledger, origin label) vs. how the successor model actually treats the wakeup. The second is model behavior the host cannot guarantee.
  - Who wrote the wakeup content (the pre-compaction root) vs. who delivered it (role_host, `formation-evidence.md:59-60, 80-81`).
- **Evidence (observed):** inclusion in a projection grants no authority (`manager:154-158`); reconciliation receipts are model assertions (`manager:702-703`).
- **Consequence (inferred):**
  - A host can meet "envelope plus label" while the successor still acts on the wakeup beyond its existing authority.
  - The pilot case passes trivially if the wakeup doesn't itself claim closure or a wider scope.
- **Minimal remediation (split):**
  - Separate the host obligation from the required outcome, and name the evidence for the outcome.
  - Record author and deliverer separately.
  - Use a wakeup whose content would close the pending decision or widen scope if it were treated as human authority.
- **Confidence:** medium-high.

### SSC-OPUS-8 — P2: operator visibility when resumption fails
- **Where:** `proposal.md:107-109` ("resumption or notification route"); `pilot.md:91-94, 114-116, 118-119`.
- **Missing distinction:** the success route vs. the failure or unresolved route. Also, "owner visible" vs. "state visible on the normal operator surface".
- **Evidence (observed):**
  - S10 announced the refresh, then ended its turn.
  - Invariant 19 says an attempted route is not delivery (`manager:986-987`).
  - Existing root-transfer guidance says to keep collecting when no verified delivery route exists (`context-boundary.md:216-222`).
- **Consequence (inferred):**
  - "Resumption *or* notification" allows a realization whose only owned route is the resumption itself.
  - If the controller crashes after compaction, the operator sees an announcement, then silence that looks like progress.
  - The pilot scores visibility only on nominal runs.
- **Minimal remediation (restate):**
  - A failure or unresolved outcome of the resumption route must reach the operator through an owned route. Alternatively, the announcement says where the outcome will appear and what its absence means.
  - Score transparency on crash, denied and interrupted cases too.
- **Confidence:** high on the text; consequence inferred.

### SSC-OPUS-9 — P2: no admissible evidence for the pilot's absence claims
- **Where:** `pilot.md:55-57, 63-74, 84-90, 111-113`.
- **Missing distinction:** actor-recorded vs. independently observed evidence. Also, one counted effect vs. every action channel the role has.
- **Evidence (observed):**
  - The independent effect counter covers duplicate effects only.
  - "No fresh wakeup replay" relies on the custody record of the host under test.
  - Nothing names an evidence source or completeness basis for "no affected domain work", "unauthorized actions" or the fixture's "prohibited action".
  - DESIGN.md §3.9 (`:424-429`) and PHILOSOPHY.md §18 (`:798-803`) treat these as claims about world history.
- **Consequence:** "with admissible evidence" is undefined for these safety cases.
- **Minimal remediation (restate):**
  - For each safety consequence, name its evidence source and completeness basis. Examples: provider history for wakeup entries; tool and effect channels observed by the harness for work after a pause.
  - Keep host records classed as actor-attributed.
- **Confidence:** medium-high.

### SSC-OPUS-10 — P3: claimed reuse of the transition protocol is asserted, not shown
- **Where:** `proposal.md:35-37, 65-66`.
- **Evidence (observed):**
  - The protocol's attribution rules are shaped around `new_context`: they need a retirement control-turn identity, and "reservation alone never authorizes `new_context`" (`protocol:173-184`).
  - Abort is allowed only from `reserved` (`:542-549`).
  - The owner document says compaction is not equivalent to `new_context` (`manager:651-657`).
- **Consequence:** for a host-invoked compaction, attribution, abort and control-turn identity are unmapped.
- **Minimal remediation (restate):** record the mapping, or its absence, as an implementation evidence obligation or an open decision, rather than "existing contracts".
- **Confidence:** medium.

### SSC-OPUS-11 — P3: a credible neighboring owner was not examined
- **Where:** `proposal.md:64-71, 102-109, 118-121`; `packet.json:35-54`.
- **Evidence (observed; outside the manifest, no digests):**
  - `design/architecture/mechanisms/authority-preserving-intent-projection.md:3, 28-29` covers submitting human intent without manufacturing authority, and pending/admitted/refused feedback.
  - `design/ideas/pending/context-replacement-emergency-and-writable-operator-ingress.md:216-237, 333-377` covers retained-role input routes and queued-input behavior.
- **Consequence:** a parallel owner for input and feedback semantics could form (DESIGN.md §1.2).
- **Minimal remediation:** name the relationship and leave the choice of owner to the architecture owner. This review does not decide it.
- **Confidence:** medium.

## Optional advice
- In the pilot, predeclare what paired result would count *against* route A. Today "useful tradeoff" is judged after the fact, so the pilot can inform but not refute.
- On the operator surface, show the evidence class of "accepted recovery": a model-asserted receipt checked against host bindings (`manager:702-703`) vs. host observation.
- Carry the ordering command's failure mode (S10's input check separated from actuation) inside the insertion. "Why this delta" won't travel with it.
- State n per safety case. A single deterministic injection per point is fine if it is stated.

## Open owner decisions
1. Which role or host contract, if any, affirmatively permits a human-supervised root to administer its own same-session lifecycle and resume automatically? Is manual-orchestration in scope at all?
2. Is starting a transition host-only (design view, invariant 1) or something a role may request (Wind Walker judgment)?
3. Who owns input ordering and pending-disposition feedback for human-facing roles: the lifecycle host or the intent-projection mechanism?
4. The owner document lives in `app-server/`, which `app-server/README.md:3-5` marks superseded. Is that the right home for a mechanism-neutral contract motivated by a manual-orchestration case? The current design view (`context-lifecycle.md:197`) does name it as owner, so placement is consistent today.

## Implementation evidence gaps
- No supported host realization exists. One would need to show:
  - ordering across every human input path;
  - durable effect custody across restarts;
  - an attributable envelope through a user-role transport.
- The semantics of the compaction mechanism are not pinned against `new_context`.
- How the successor model treats user-role wakeups has not been tested.
- Preserving live children or processes is out of scope by design.
- Net cost is unestablished: 429,188 → 28,426 tokens is one input reading (n = 1), not a measure of savings.

## Limits
- This is design review only. No tests or commands were run, and the parent's deterministic checks were not repeated.
- I did not read the prior advisory review (`reviews/same-session-context-continuity-20261028/review.md`), so overlap with its findings is not established.
- Files read outside the frozen manifest were not digest-bound: the design view, the mechanism page, the pending idea, `app-server/README.md`, and the 2026-10-27 copy of `context-boundary.md`.
- This session is kept open for bounded re-evaluation of an exact remediated subject.
