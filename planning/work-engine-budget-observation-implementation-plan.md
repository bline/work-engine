# Work Engine — Objective and Agent Budget Observation

**Implementation handoff / proposed plan — 2026-10-29**  
**Repository:** `bline/work-engine`  
**Repository baseline inspected:** `rescue-detached-20260911` at `471f61068d88999208cfeff477d42de67a6ce6f6`  
**Standing:** Proposal for local engineering review and implementation; not a recorded approval, changed contract, or permission to deploy. Check the actual checkout, dirty worktrees, installed skills, and current owner instructions before editing.

## 1. Purpose and desired result

Give the coordinator and every assigned role useful, reliable information about **the modeled inference cost of work they own**, without requiring model calls for accounting or routine polling. Extend the **existing chatboard `wait` delivery mechanism** that already multiplexes board messages and local context-pressure observations. A coordinator controls additional **discretionary leeway** for subordinate assignments within the objective's authorized resources; a worker, reviewer, or supervisor cannot silently enlarge its own allocation.

The outcome is a complete, auditable cost picture for an accepted engineering objective: coordinator preparation and integration; supervisors and discovery; workers/builders; independent reviewers, including separately launched top-level reviewers; retries, remediation, failed launches, collections, and handoff or successor work. The same model call is charged **once** to its owning logical assignment even though its cost appears in ancestor *inclusive* subtotals.

This is **not a bidding/auction system** and should not impose a new model consultation or approval ceremony on every dispatch. Estimates inform decisions; explicit allocations bound delegated work; human authorization bounds the objective. Cost measurement does not create task, mutation, review, provider, security-grant, launch, or publication authority.

## 2. Existing assets and important constraints

Read `AGENTS.md`, `DESIGN.md`, and, for this planning/design change, `PHILOSOPHY.md` in the actual working repository. Preserve DESIGN's distinction between binding invariants and adaptable routes.

Inspected sources:

- `skills/manual-orchestration/SKILL.md` and `references/coordinator.md`, `supervisor.md`, `worker.md`, `organization-formation.md`, `context-control.md`, `measurement.md`, `context-boundary.md`.
- `skills/manual-orchestration/scripts/scope_budget.py` and `tests/test_scope_budget.py`. Today `scope_budget` stores original scope, additions, elapsed wall-clock hours, and a **shared weekly meter delta**; it does not assign individual agent spending. `--set-by user` is a recorded assertion, not a cryptographically authenticated approval.
- `metrics/scripts/usage_ledger.py` (F1/F1.1), its existing tests, and `metrics/scripts/task_profile_record.py`. The ledger reconstructs individual Codex/Claude calls, cached input, model and effort, priced or unpriced API-equivalent cost, Codex service tier, and uncertainty. It records Codex parent/root ancestry when available and Claude subagent transcript identifiers. The task-profile attribution currently joins child sessions to observed models and efforts, **not** an objective-level spending tree.
- `skills/manual-orchestration/scripts/role_host.py`: its append-only launch/delivery ledger gives cross-host parent links **only for roles launched through it**. Native spawned children and separate top-level reviewers need their actual bindings; a role name or inherited environment session ID is insufficient.
- `skills/durable-state/scripts/codex_chatboard.py`, `docs/codex-chatboard.md`, `skills/coordination-channels/SKILL.md`, `references/session-control.md`, and `skills/durable-state/tests/test_context_control.py`. `wait` already watches Git refs and bound transcript files through inotify; `ContextControl` reads pinned telemetry; `--follow` maintains cursors; local `control_events` are **not board publications**, and the listener's output must actually be collected by the host.
- `docs/model-selection.md` and `metrics/dogfood-selection-plan-20261027.md`. Round 2's measurement and blinded reviewer conditions must not be silently changed by this feature. Existing campaign contracts take precedence where manual-orchestration is not the owner.

**Accounting truth:** API-equivalent dollars and Fast-tier-adjusted **modeled quota weight** are scenarios, not actual subscription deductions or invoices. An account-wide seven-day percentage is an independent global capacity signal; it must not be subtracted across timestamps to charge a single agent. Keep provider, model, pricing version, and quality/coverage metadata attached to every estimate. Do not add heterogeneous measured quantities together or silently treat an unknown price as $0.

## 3. Semantics and authority (binding design requirements)

### 3.1 Four distinct quantities

For each **logical assignment** (not simply a transcript), maintain:

1. **Estimate:** the agent's current expected *all-in* cost of the outcome it owns, with uncertainty and assumptions. It may be revised with an explanation; it grants no spending authority.
2. **Approved allocation:** the spending envelope for that assignment (direct inference plus descendants when the role owns a subtree), issued by the coordinator or another owner explicitly authorized by the active contract.
3. **Actual attributable consumption:** priced calls, upper/lower ranges where applicable, unpriced/unattributed counts, and freshness/coverage; tracked independently of what the agent claims.
4. **Objective ceiling and reserve:** a human-authorized hard limit only when the human or owning contract actually established one, plus whatever explicitly delegated coordinator-controlled, **unallocated** reserve is available. No human ceiling may be fabricated from an agent estimate.

An approved allocation and a user hard limit are different authorities. Exceeding a *forecast* creates a planning observation; crossing an *assignment allocation* requires a coordinator decision for additional spending; crossing a *user hard ceiling or hard deadline* requires the relevant human or owning-contract decision. Do not silently convert either an estimate or a weekly account-meter delta into an authorization limit.

Time constraints remain independently typed: **wall-clock deadline** vs **active-execution time estimate**, with an explicit clock origin. Expiring grants/credentials have a separate validity lifecycle. Refresh a grant only when its owning security/approval contract permits it; a budget amendment alone is not that permission.

### 3.2 Budget hierarchy and conservation

A stable `objective_id` owns one budget tree. Stable logical `assignment_id` / `role_id` and `reports_to` determine **budget reporting**, independently of runtime parent-child ancestry. Runtime IDs, transcripts, launches, reviews, context replacements, and root successor sessions are bound to those assignments with exact observed evidence.

At every node report both:

- `direct_consumption`: model calls charged to that logical assignment only;
- `inclusive_consumption`: direct consumption plus every descendant assignment's consumption **once**.

Similarly distinguish a parent's inclusive allocation from its explicitly allocated descendant amounts and its remaining unallocated reserve. Invariant: mutually committed direct/child allocations and reserves cannot exceed the parent's authorized envelope; a reallocation does not create new total resources. A child's cost appearing in a parent's inclusive view is **not another charge**. Calls must not be assigned to multiple objectives; unknown/ambiguous attribution stays excluded from established totals and visible as an unresolved liability, never silently zero.

**Illustration (not a default budget):** Objective authorized $30 modeled-equivalent; coordinator direct allocation $3; Supervisor A subtree $12; Supervisor B subtree $10; coordinator-controlled reserve $5. Within Supervisor A, its own $2 plus builder $7 plus reviewer $3 exhausts that branch allocation. A request for $1 more for the builder must be approved by the coordinator and covered by available authorized reserve/reallocation; it must not implicitly raise the $30 objective ceiling.

**Leeway policy:** no automatic 10% (or other) increase to a child allocation. A supervisor may recommend an increase and explain unfinished obligations; the **coordinator/orchestrator must approve the adjustment** and record its source, reason, old/new allocation, affected reserve, and current objective state. A coordinator cannot approve an increase to the human objective ceiling. Coordinator direct spending may use only its already-authorized own allocation or self-use authority expressly delegated under the objective; otherwise its own allocation overrun is escalated to the human/owner rather than self-approved. If no human ceiling was established, coordinator forecasts remain advisory—do not invent a human approval gate merely because its estimate changed.

Approval of additional budget never expands task scope or supersedes other authority gates. Existing scope-addition classifications and user approvals still apply. Ordinary in-scope defect repair is not automatically a new objective.

### 3.3 Observability and incomplete information

The measured ledger and approval history are authoritative for **what was observed and granted**, not a claim of exact per-call subscription quota. Cost ranges must remain ranges (e.g., Claude unknown cache TTL). Unknown model, provider, role mapping, missing transcript, incomplete attribution, or missing pricing is an explicit status. Conservative budget-admission behavior is required for true hard limits when current liability cannot be established, with a truthful blocked/unknown reason; preserve safe collection, evidence capture, and handoff. No false inference of completion or zero spend.

The system can check **before the next controllable launch/dispatch/turn**, and alert on completed-call costs. It cannot guarantee an exact ceiling if a running call or another unmanaged host consumes more before the record appears. Report enforcement coverage and in-flight exposure rather than claiming an unimplemented hard kill-switch.

## 4. Registration and attribution

Implement a **durable, versioned objective/assignment budget registry** under existing Work Engine local state conventions (the repository's `planning/` is for this proposal, not mutable runtime state). The coordinator's durable assignment/continuation references the registry. Use atomic updates, concurrency control, an append-only decision/identity history or equivalent recoverable journal, and explicit ownership of state writes. Avoid competing sources of authority in the legacy `scope_budget` record and the new registry.

A registration records: objective identity, accepted objective/authority reference, estimate and applicable ceilings; logical parent/role; assigned cost envelope; an immutable approval/decision record; start and end **assignment boundaries**; and identity bindings to actual provider/host session, rollout/transcript, agent ID where required, model configuration and collection owner. An unlaunched proposed role has no measured session. Subsequent session/assignment changes append new bindings, not rewrite history.

The attribution reader should reuse the existing ledger's parsing, deduplication, pricing and uncertainty rules rather than writing a subtly different token counter:

- Codex: use unique `response_id`, own-thread usage, actual `turn_context`, service tier, and verified session lineage; handle inherited/forked counters correctly.
- Claude: deduplicate by `message.id`, include actual main and subagent transcripts, and use composite identities where a Claude subagent **shares its parent's `sessionId`**; session ID alone is not a unique role key.
- Cross-provider / separate top-level reviewers: join verified `role_host` launch relationships **and** explicit assignment bindings. An unrelated top-level session is not in the subtree just because it ran at the same time.
- Retained session changing assignments: attribute calls to an **explicit assignment interval/turn boundary**, not the entire historical session. Define interval boundaries and report ambiguous straddling calls honestly.
- Root rotation, context compaction, and same-session recovery: preserve `objective_id`, accumulated cost, registered outstanding children, and budget authority; a changed root session cannot reset spending.

Provide deterministic reports by **objective**, **assignment**, **provider/model/tier**, and **direct vs inclusive** totals. Include any unpriced calls, unknown attribution, first/last observed call, ledger version, snapshot time, source references, and reconciliation status. Preserve original raw call identities for audit and backfill.

## 5. Monitor and existing `chatboard wait` delivery

Extend the **existing delivery path**, not the model's instruction workload:

1. A single **owned host-side aggregation path per objective (or an equivalently efficient shared host)** consumes registered transcript updates and maintains a durable cost snapshot. Use filesystem notification/incremental input where feasible. Rebuilding the full historical `usage_ledger.py` on each file append or in every agent's waiter is not acceptable. Permit bounded reconciliation against the reference batch ledger for audit, rotations, unknown deltas, and recovery.
2. Existing per-agent `chatboard wait --follow` registers one budget observation binding in addition to its context-pressure binding. `RefChanges` may watch the relevant objective budget snapshot/registration state. Add a `BudgetControl`-equivalent adapter with its **own cursor**, identity/assignment/policy binding and changed-state suppression. Do **not** interfere with `ContextControl` or reuse its cursor.
3. Keep ordinary `messages` and `next_since` semantics unchanged. Prefer an **independent `budget_events` and `next_budget_cursor`** on the same `wait` result, so old `control_events` consumers continue to work. Budget events are local, typed, `observation_only` notifications, **not Git board posts, approvals, or user messages**. Reject attempts to spoof them as ordinary chatboard messages.
4. Only decision-relevant transitions notify: approaching an assignment allocation, estimate materially diverging, allocation exhausted, changed authorized allocation, significant source/coverage degradation, and recovery. Configurable advance notice must leave room for normal call-size/in-flight risk; avoid repeated same-state warnings. Suppress idle churn. Do not wake all descendants for one branch's warning.
5. Route **allocation decision** events to the coordinator, scoped observations to their responsible roles as needed, and a grant/denial outcome to the waiting requester. The external collector must demonstrably deliver these events to the correct live model session; stdout in a terminal is not delivery. When a model turn is unnecessary, coalesce the observation with the next natural turn. An earlier wake must use a *verified* host mechanism. `wait --follow` retains its bounded lifetime and rearming semantics; handle timeout, failure, replacement and ownership transfer explicitly.
6. A stopped listener, lost binding, stale budget snapshot, missed membership update, or uncollected stream must not be misrepresented as live observation. A fallback explicit status query is an allowed capability; it is not a request for routine model-driven polling.

**Important separation:** registration and budget approval records must be durable; local budget-pressure notifications need not be persisted to the Git-backed board. The live observed state should be reconstructible from durable authority and usage records after listener interruption.

## 6. Admission, request and decision semantics

At a controllable dispatch/new assignment boundary, consult the current **approved allocation and objective authority**, not the shared weekly percentage. If estimated remaining cost plus known outstanding/in-flight commitments threatens the allocation, the agent may request leeway early with a compact statement of current measured spend, unfinished obligations, anticipated additional cost and confidence, current reserve and effect on the parent objective. A supervisor can recommend; **only the designated coordinator** can grant additional child allocation inside its delegated objective authority.

An approval updates the budget registry transactionally, reduces available reserve or explicitly reallocates an uncommitted amount, links the approving coordinator's verified/current authority binding and exact source decision, and sends a meaningful change event to the requester's existing listener. A denial or unavailable approval leaves the old allocation in force. On root ownership transfer, the successor must establish adoption/authority before it can grant new allocations. Do not confuse a message's claimed session ID with verified identity; call out where OS-local tools cannot authenticate a claimed approval actor.

For a **hard authorized ceiling**, new controllable dispatches must be refused when the accounted and conservatively estimated outstanding cost cannot fit. No automatic task restart, unauthorized new model, or self-approval. Existing running work is collected/stopped only through the owning process contracts; preserve provenance and safe handoff. The global weekly meter may provide an independent account-exhaustion warning, not per-objective spending or a new source of spending permission.

**Legacy limits:** preserve existing human-imposed weekly-percentage and deadline conditions until the authorized owner explicitly migrates or replaces them. Do not silently reinterpret a previously accepted wall-clock deadline as execution time, and do not silently remove a hard limit because modeled API dollars are now observable. Design a versioned migration for `scope_budget.py` so no two independently authoritative budget guards disagree. Old records remain readable and tests remain valid or change only with an explicit, reviewed migration.

## 7. Implementation sequence and deliverables

Use judgment about the exact module breakdown; do not turn these into additional approvals or mandatory independent agent calls. Maintain an isolated worktree or otherwise avoid touching other sessions' uncommitted files, and use the existing chatboard coordination/claim rules for overlapping resources.

**A. Validate boundaries and freeze the minimal contracts.** Inspect real launch/rollout shapes for native Codex children, separate Astra reviewers, Claude Code subagents, cross-provider `role_host` children, and retained tasks. Verify the active branch/skills and required tests. Define the versioned objective-budget record, assignment membership rules, call ownership, decision/approval reference, and observer event/cursor semantics. Preserve relevant contract owners and active experiment isolation. Record uncertainty instead of inventing an identity.

**B. Offline cost-tree attribution (no live enforcement yet).** Build a small deterministic aggregation layer over `usage_ledger.py` output. Given a registered objective and bound assignments, produce direct and inclusive totals, coverage/ranges, and no double-counting. Reconcile selected controlled fixtures against the existing batch ledger. Do not implement a second pricing formula.

**C. Durable registration and coordinator leeway.** Add budget registrations and decision history with tested invariant checks for nested allocations, reserve conservation, scope and approval authority, rotation, and reassignment. Keep estimate changes distinct from allocation grants. Make the existing `scope_budget.py` transition explicit and backward compatible.

**D. Incremental observation and wait integration.** Maintain cost snapshots host-side without a model turn per append. Add the budget source to existing `chatboard wait`, with separate `budget_events`/cursor and unchanged context/board functionality. Test that one event source can wake a listener while the other two retain their cursors. Do not claim model-visible delivery until an end-to-end host collector confirms it.

**E. Enforcement at supported admission points.** Add deterministic pre-dispatch/admission checking where the actual launch path can be intercepted. Document uncovered host paths and in-flight overshoot; do not pretend monitoring alone enforces the budget. Keep collection, custody and safe stop paths working even after an exhausted allocation. New enforcement begins only after the owner accepts the semantics and coverage.

**F. Shadow pilot, evidence, and documentation.** First report observed costs and potential alerts with **no stop/approval behavior change** on one bounded mixed-role objective; run a second cross-provider/rotation case if representative traces are available. Measure listener idle overhead, notification count, model wakeups, accounting divergence and missing coverage. Request the appropriate owner decision before activating any real new hard gate or changing an active campaign's instructions. Update `docs/codex-chatboard.md`, manual orchestration/coordination references, CLI help and tests as needed; install/release changed skills through their owning mechanism, not by assuming a source edit changes loaded instructions.

## 8. Minimum acceptance checks

All must have durable test evidence; unknown should be reported, not silently passed:

1. **Three-level sum:** coordinator → supervisor → builder/reviewer; root inclusive cost equals the union of actual calls, and child spend is counted exactly once even when nested in both ancestor views.
2. **Shared-account isolation:** unrelated concurrent Codex/Claude sessions move account-wide quota but do not enter the objective's cost tree.
3. **Cross-host lineage:** Codex native children, top-level Astra reviewer, `role_host` Claude child, and Claude subagent are attributed to the correct logical assignments with exact bindings and without duplicate inherited usage.
4. **Retained and successor continuity:** one session handles successive assignments; a root rotates; budgets and call attribution survive without reset or leakage across assignment boundaries.
5. **Pricing/coverage honesty:** actual model/tier selection is used, Fast multiplier is labeled modeled quota weight, unknown model/cache TTL creates unpriced/ranged/unknown outcome, and missing readings are never zero.
6. **Conserved leeway:** only the active authorized coordinator increases a child's allocation; nested reservations and cumulative increments cannot oversubscribe the objective; denied or stale approvals do not mutate it; the coordinator never raises the user ceiling.
7. **Mixed simultaneous notifications:** one `wait --follow` handles board messages, context pressure and budget changes without losing, duplicating or conflating cursors/events; one changed budget wakes only applicable recipients.
8. **No quiet cost churn:** no repeated model turns when values stay within a state; no historical full-scan on every transcript append. Demonstrate an incremental cost path and reconcile it against batch output.
9. **Failure/restart:** watcher failure, truncated writer file, log truncation/replacement, missed inotify event, listener timeout, monitoring pause and recovery produce truthful unavailable/recovered state and no unauthorized new dispatch under a hard cap.
10. **Unchanged contracts:** existing chatboard/context tests and legacy budget tests still pass; active R2 blinding/model selection and configured Slice Builder gates are not silently changed. Report what hasn't been tested live.

## 9. Out of scope

No auction, mandatory per-task bidding, new AI monitor agent, hourly polling inference loop, hardcoded leeway percentage, speculative quota-point attribution, new global scheduler, forced stop of in-flight model requests, automatic credential reissuance, or independent source of authority. Do not rewrite the whole manual-orchestration system or the Rust lifecycle runtime to implement this focused capability.

## 10. What the local implementer should return

Return: (a) the exact implementation boundary and changed files; (b) the data model and cost attribution proof, including missing cases; (c) the observed watcher-to-model delivery path; (d) permission/approval and admission coverage, including unenforced hosts; (e) tests and measured overhead; (f) backward compatibility/migration disposition; and (g) unresolved decisions and a concise owner review packet. Do not claim the objective accepted merely because code or fixtures pass. Do not publish or authorize new resource ceilings without the owning approval.

**Central success criterion:** An agent can register an estimate and an approved allocation once; an independently owned host observes its actual direct and subtree spending; meaningful updates arrive through the existing chatboard waiting route; and any extra subordinate leeway is a visible, coordinator-approved reallocation, not a fresh human request or a silent enlargement of authority.
