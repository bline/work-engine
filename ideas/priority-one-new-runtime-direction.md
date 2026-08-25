We have converged on a new runtime direction for Work Engine: build a parallel Codex App Server scaffold rather than porting the current Codex harness in place.

## Core architecture

The new scaffold should preserve Work Engine’s semantic architecture while replacing harness-specific execution machinery.

The governing model is:

* Role definitions describe behavior, authority, and responsibilities.
* Durable logical role instances bind to App Server threads.
* A thread is a durable reasoning runtime and replaceable runtime binding, not the canonical owner of workflow state.
* Each thread can contain successive model context windows.
* Skills remain reusable capabilities explicitly supplied to role threads.
* Provider-specific mechanics belong behind runtime adapters.
* Proposal packets, authority records, checkpoints, claims, findings, schedules, contracts, receipts, and exact review subjects remain separately durable and authoritative.
* Thread history, native notes, and history lookup support recovery, but they do not replace canonical domain state.

The shorthand “roles become threads” is useful, but the more precise formulation is:

> Logical role instances bind to threads, and threads contain successive context windows.

## Why App Server fits

App Server provides native forms of machinery currently simulated through agent instructions and harness conventions:

* persistent thread start, resume, read, fork, archive, and history;
* retained thread goals and settings;
* turn steering and follow-up queues;
* explicit skill injection by exact path;
* runtime skill discovery and invalidation;
* thread-scoped dynamic tools;
* token-usage events;
* explicit compaction control;
* independent thread, turn, and context lifecycles.

These replace or reduce reliance on `spawn_agent`, `followup_task`, session-ID conventions, mailbox handoffs, implicit skill placement, context-recovery instructions, and some MCP indirection.

Two qualifications remain:

1. Process-scoped skill roots and apparently non-thread-local skill enablement are unsuitable for concurrent role isolation. Role threads should receive explicit skill input items and thread-scoped dynamic tools.
2. Several useful APIs are experimental. The scaffold must capability-negotiate them behind an adapter rather than putting unstable App Server method names into durable role contracts.

## Verified context behavior

A live experiment was run using the unexpectedly available `codex-cli 0.150.0-alpha.5`.

The experiment:

* started App Server with `token_budget` enabled;
* created one ephemeral App Server thread;
* forced a 12,000-token model context;
* consumed 11,033 tokens on the first turn;
* observed Codex inject native `notes`, `history`, `get_context_remaining`, and `functions.new_context` lifecycle instructions;
* started a second turn after exhaustion;
* observed an automatic transition to a fresh context window;
* confirmed that the App Server thread ID remained unchanged.

Observed identities:

* App Server thread: unchanged across the transition;
* original context-window ID: `…8d6fd4664ac6`;
* replacement context-window ID: `…a5300e564917`;
* runtime confirmation: the second window explicitly reported itself as fresh and identified the previous window.

Therefore:

> App Server inherits `token_budget` and native fresh-context replacement. A durable App Server thread can survive multiple independent model context windows.

The important protocol wrinkle is that App Server emitted an item named `contextCompaction` even though the observed semantic result was a fresh context window with a new identity. The scaffold must never infer transition semantics from that event name alone.

It should classify transitions using multiple signals:

* thread identity;
* prior and current context-window identity;
* preservation/readiness state;
* event ordering;
* whether an opaque generated summary is being relied upon;
* post-transition runtime evidence.

Possible classifications should include:

* `fresh_context_transition`;
* `summarized_compaction`;
* `unknown_transition`.

Manual `thread/compact/start` has not yet been tested. It may still perform legacy summarizing compaction and must remain a separate adapter operation until verified.

Because this evidence comes from an alpha Codex build, supported versions must be capability-detected and integration-tested. Fresh replacement must not be inferred solely from the overloaded `contextCompaction` label.

## Context lifecycle policy

Opaque summarizing compaction is prohibited because it is a black-box transformation that can silently lose workflow meaning.

Native fresh-context replacement is allowed when continuation is safe.

The critical correction to an earlier simplification is:

> Pressure triggers preservation. Optimality triggers clearing.

These are independent judgments.

* When pressure rises, preserve continuation-relevant meaning even if the current context remains useful.
* Clear the context when keeping it is no longer economically or semantically preferable to rehydrating a fresh one.
* A small logical work-unit boundary is a good clearing opportunity, but not a mandatory clearing point.
* If replacement is unsafe, preserve useful state but retain the current context.
* Automatic budget exhaustion is an emergency guardrail, not the normal lifecycle.

“Save state” means semantic compilation, not transcript summarization. Preserve only what correct continuation may depend upon:

* current objective and governing authority;
* completed semantic consequences;
* accepted decisions and premises;
* commitments and pending obligations;
* unresolved uncertainty;
* current workflow state;
* exact canonical artifact and evidence references;
* authorized next action.

Exploration, rejected reasoning, conversational history, and raw tool output should normally be discarded unless future judgment can legitimately depend upon them.

The normal lifecycle becomes:

1. Observe the current context and durable state.
2. Preserve context-only meaning when pressure or semantic risk justifies it.
3. Determine whether retaining or clearing the context is preferable.
4. Bind the judgment to an immutable context revision.
5. Obtain bounded validation or correction from the target role.
6. Verify that correct continuation no longer depends on context-only meaning.
7. Invoke or permit native `new_context`.
8. Confirm a new window on the same durable thread.
9. Rehydrate from explicit durable state.
10. Reconcile the new window and record uncertainty.

Snapshot binding and a write fence remain essential: a preservation judgment against context revision C1 must not authorize retirement after the target creates new continuation-critical meaning in C2.

## Wind Walker / Context Manager

Wind Walker should not be discarded. Its failure was one of placement.

It should move from:

> A model-visible skill requiring every task agent to manage its own context

to:

> A specialized durable observer role centrally coordinating context lifecycles.

The repurposed Wind Walker role should:

* receive host-supplied thread/window telemetry and bounded context snapshots;
* observe token pressure and semantic differences;
* request preservation when context-only meaning is at risk;
* identify when clearing is safe and advantageous;
* propose the smallest faithful durable state changes;
* obtain bounded validation, correction, or veto from the target role;
* authorize or request native fresh-context replacement after readiness;
* classify the actual transition;
* verify rehydration;
* record missed interventions and uncertainty.

It must not:

* own another role’s domain truth;
* fabricate or extend authority;
* summarize another role’s context into an opaque continuation;
* clear a context that is not continuation-ready;
* infer transition semantics from `contextCompaction`;
* violate reviewer isolation;
* require every task role to continuously manage its own context.

The correct ownership boundary is:

> Centralize observation and lifecycle coordination; keep semantic preservation role-owned.

The target role owns the meaning of its objective, commitments, pending work, uncertainty, and authority state. Wind Walker inspects, proposes, verifies structural sufficiency, and coordinates retirement. The target role only performs bounded correction or veto; it should not repeat the complete context analysis.

Wind Walker should have its own durable thread and an external lifecycle ledger containing:

* logical role identity;
* current thread binding;
* observed context-window identity;
* inspection revision;
* token-pressure state;
* preservation request and readiness state;
* target validation or correction;
* transition authorization;
* transition event and classification;
* post-transition reconciliation status;
* uncertainty or missed-intervention state.

The ledger—not Wind Walker’s own context—is the authoritative lifecycle projection.

Wind Walker does not require another recursive context manager. One inspection is a bounded logical unit; it can record its result in the ledger and clear its own context when advantageous.

## Role migration

Current role-like skills should become App Server thread templates or external provider-backed roles:

* `slice-supervisor`: durable supervisor instance per campaign;
* `slice-builder`: builder instance per slice, retained through planning, implementation, gating, and remediation;
* `strategic-planner`: long-lived advisory planning instance;
* `proposal-former`: instance per proposal-formation stream;
* `idea-intake`: instance per bounded intake or raw-idea reconciliation;
* `agent-instruction-review`: bounded specialist-review instance retained through remediation;
* `wind-walker`: centralized context-lifecycle observer;
* `claude-recon-implementation`: split into an independent-review role, provider adapter, and disposable reconnaissance capability.

A Codex reviewer must start in a fresh thread rather than fork the builder because `thread/fork` copies history and would violate the fresh-entry review boundary. If Claude remains necessary for cross-provider review, it stays an external runtime behind a provider-neutral adapter.

## Non-role skill migration

Copy without semantic change, including references, scripts, schemas, and tests:

* `chrome-vision`;
* `claim-evidence`;
* `code-change-profile`;
* `durable-state`;
* `proposal-packets`;
* `repo-search`;
* `slice-checkpoint`;
* `slice-completion-commit`.

Retain but redesign for App Server:

* `agent-environment-graph`;
* `codex-adversarial-review`;
* `comparative-repository-analysis`;
* `independent-review-state`;
* `review-bench`;
* `role-scheduler`;
* `work-engine-mcp`.

Do not port `ui-design-principles` into the generic Work Engine scaffold. Relocate it to a Site2JSON-specific plugin or package if it is still needed.

Wind Walker’s former skill package should not remain a general task-agent capability. Its invariant and relevant semantics should be incorporated into the new Wind Walker role and host lifecycle manager.

## Scaffold responsibilities

A top-level `app-server/` runtime boundary should own:

* connection, initialization, reconnection, and event streaming;
* generated protocol bindings pinned to supported Codex versions;
* capability negotiation;
* a logical-role registry mapping role instances to current thread bindings;
* thread start/resume/fork/fresh-entry policies;
* role developer instructions;
* exact skill resolution and per-turn injection;
* thread-scoped dynamic-tool dispatch;
* model, permission, workspace, and environment bindings;
* idempotent turn delivery using client message IDs;
* token and context-window observation;
* context lifecycle and replacement policy;
* queue, goal, setting, and thread-status observation;
* references to canonical Work Engine artifacts;
* lifecycle ledgers and transition classification.

A thread ID remains a runtime binding, not the stable semantic identity of a supervisor, planner, reviewer, proposal former, builder, or Wind Walker.

## Migration order

Do not migrate every capability before exercising a real role. That would design a large capability surface against imagined consumers.

Recommended order:

1. Build the thin App Server adapter, protocol bindings, capability negotiation, logical-role registry, skill resolver, and dynamic-tool bridge.
2. Copy the category-1 capability packages.
3. Port `strategic-planner` as the first role vertical because it is read-only, continuity-sensitive, and has a compact durable handoff.
4. Pair the planner with the first Wind Walker observer vertical to test observe → preserve → replace → rehydrate → reconcile.
5. Port `proposal-former` and `idea-intake`.
6. Port specialist and independent review and resolve the Claude-versus-Codex provider decision.
7. Port `slice-builder`.
8. Port `slice-supervisor` last.
9. Pull category-2 capabilities into the scaffold when their first real consumer requires them.

The supervisor should be last because it contains the greatest concentration of current harness mechanics. Porting it first would encourage rebuilding the old orchestration topology on top of App Server.

## Durable repository capture

These discoveries, decisions, and open questions should now become durable repository state.

They should be split by semantic owner rather than placed into one undifferentiated conversation summary.

For the Context Manager/Wind Walker direction:

* revise `ideas/01-semantic-context-manager-proposal.md`;
* add `ideas/intake/semantic-context-manager/app-server-runtime-evidence.md` containing the exact `0.150.0-alpha.5` probe, configuration, observed identities, results, and limitations;
* extend `authority.md` with the adopted decisions;
* revise `repository-observations.md` to replace falsified runtime assumptions;
* create assessment revision 2 while retaining the earlier historical assessment;
* update `record.json`;
* regenerate `projection.json`.

The authority record should capture:

* App Server as the primary runtime rather than hooks-first;
* centralized Wind Walker observer role;
* pressure triggers preservation;
* optimality and semantic safety govern clearing;
* native fresh-context replacement is permitted;
* opaque summarizing compaction is prohibited;
* thread and context-window identities are distinct;
* `contextCompaction` does not establish transition semantics.

The wider scaffold and skill-port direction should become a separate neighboring idea, such as:

`ideas/codex-app-server-scaffold-and-role-port.md`

It should own:

* the App Server scaffold boundary;
* roles as thread-bound logical identities;
* skills as capabilities;
* provider-neutral runtime adapters;
* the full skill classification;
* role-port order;
* explicit skill injection;
* dynamic-tool bindings;
* experimental capability/version boundaries;
* the unresolved Claude-review decision.

It should reference rather than duplicate the Context Manager proposal and `ideas/runtime-adapter.md`.

After these exact-source revisions are committed, the intake can be reconciled and a formal semantic-context-manager proposal packet can be produced. Under the existing intake model, the Git revision—not merely the working-tree file—is the durable identity.

## Remaining uncertainties

The following remain unresolved and should be preserved explicitly:

* whether manual `thread/compact/start` performs legacy summarizing compaction under `token_budget`;
* whether the host can directly initiate `new_context`, or only configure and observe the native model/Core transition;
* the exact intervention and fencing mechanism before automatic replacement;
* how context-window identity should be observed without depending on internal raw-response details;
* capability differences across supported Codex releases;
* thread-local skill isolation behavior;
* the final Claude-versus-Codex independent-review strategy;
* recovery behavior when automatic replacement occurs before a readiness receipt exists.

No repository files have yet been changed for this durable capture.

