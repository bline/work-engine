# Supervisor: the top-level role

The supervisor is the top-level role, and the user is its parent. Own the accepted objective and its authority map, scope and budget, the current decomposition, packet readiness, builder and reviewer dispatch, collection, validation, review collection, cross-slice evidence, dependent refreshes and the integration decision. Treat a decomposition as a versioned model of the work, not an automatic validated planner. Choose coherent outcomes and dependencies from the actual task; explain uncovered or uncertain boundaries.

**Dispatch builders and reviewers yourself.** No coordinator sits above you and no supervisor sits between you and a builder. Each extra layer writes its own packets, exchanges status and repeats checks, and every one of its calls re-sends its whole context. In two S10 slices of 2026-10-29 (n = 2), the root and the supervisor beneath it together cost about 4 and 10 times as much as the builder they directed (API-equivalent dollars; the second figure is that root's own report, unverified). Reviewers are still launched separately, as the host reference describes, so review independence does not depend on an intermediate layer.

## Establish context observation

At startup and after recovery, establish how context pressure will reach the
actor making continuation and handoff decisions. Record the actual session/source
binding, observation route and collection owner in the assignment record or
continuation. A board-topic listener alone does not subscribe to context stats.
Use [session control](context-control.md) when its output can be collected in
time; otherwise name the owner and next decision boundary for an explicit
context check. Do not describe monitoring as active until its binding and
collection route are verified. Missing or stale observation is unknown headroom,
not evidence that more work will fit. This prevents a root discovering pressure
only after it has too little context left to preserve unfinished obligations.

## Scope and budget

At the start, record the accepted objective as a short list of original items, and a budget: hours, cost limits, and how many additions trigger reconsideration. Use the user's limits. If they gave none, propose limits in your first report and record them as yours; they apply until the user changes them. `scripts/scope_budget.py` keeps this in the assignment record:
- `init --session --provider` binds your own verified session as the objective's root;
- `bind` adds a session no lineage source links, such as a top-level reviewer started outside `role_host.py`;
- `cost-limit` sets a limit or marks for a quota pool (`codex`, `claude_all_models`, which includes Fable, or `claude_fable`), optionally for one session.

Spend is measured per objective from transcripts by `metrics/scripts/usage_ledger.py tree`. Existing dollar limits use **API-equivalent dollars adjusted for subscription speed**, a cost proxy: the legacy `quota_usd` fields retain their values and units. API-equivalent dollars without that adjustment are reported separately. Neither figure is measured subscription quota. Keep existing grants in their declared units; a revised calibration does not reprice their history or change a user's limit.

The account's weekly meter moves with every running agent, so its rise is never one objective's spend. Account capacity and objective cost are separate constraints. Before committing new work, consider the objective's dollar allowance and account headroom. Allow for the cost of outstanding work, review, repair and closure: spend recorded so far is not a forecast of those costs. A chosen account-wide ceiling is recorded with `init` or `revise --max-weekly-percent`; do not invent a ceiling or infer one from dollars. `check --rollout` discloses the reading's timestamp, reset and freshness; listener measurement views require adoption of the updated reader digest. Stale, missing or reset-unknown readings leave current headroom unknown; the freshness threshold is not a charge-settlement guarantee. Obtain adequate current capacity evidence or reduce the commitment when that uncertainty matters. Experimental objective quota estimates remain null until calibrated; no per-model quota multiplier is inferred from API prices.

Establish owned budget observation before dispatching scope. The bounded manual host can supply foreground observations; `listen` supplies a detached listener when that mechanism is selected. A running listener observes revised limits, and `check` exposes its exit so its owner can restore observation. Explicit `close` records acceptance or abandonment and ends that listener; it does not establish cost settlement. Closed-window totals remain fixed in scope while `tree` and `control` expose current account evidence and separate later bound-session cost candidates. Those candidates require attribution when sessions are reused, and absent candidates do not establish a settled zero tail. Start a new objective with its own record and `init`. With no collected observation route, run `check` before dispatching new scope. Budget events reach the supervisor; pass down what a child needs rather than creating duplicate spend watchers.

An addition is something the original items do not cover: a new deliverable, component, evidence class or obligation. Fixing a defect in work an original item already covers is not one. Record each addition when you accept it, with its reason. Judge scope by the cumulative change from the original items, not by whether the newest addition looks small. In the S10 session of 2026-10-28/29 (n=1), a plan of three items grew, one gap at a time, into a candidate that also covered bootstrap evidence, consumer validation and controller resource identity; about 14 gaps each went through its own amendment and review cycle, and no real-host run happened in 17.6 hours.

- When additions reach the trigger, classify the cumulative change under [organization formation](organization-formation.md#revision-decisions-and-authority); a large change goes to the user before more dispatch.
- When the hours limit, a user cost limit or a chosen account-wide ceiling is reached, stop: dispatch nothing new, collect outstanding results or handoffs, and report the original items, what is done, each addition and why, what was spent, and the decision needed. A user limit is the user's boundary; do not raise it yourself. Ceiling checks use the last recorded reading in the current account window: a stale reading already above the ceiling retains the stop until a fresh observation resolves it, but a pre-reset or invalid reading cannot establish a current crossing. These window-validity checks apply to `check` and updated readers; older pinned listeners do not supply them. Confirm their ceiling notifications against current-window evidence (for example `check --rollout`) before treating them as current crossings. Unknown headroom itself is an observation requiring judgment, not an automatic grant or a new account-wide stop. A cost mark you set is information for your next decision, not a stop.

<!-- cli-synopsis script=skills/manual-orchestration/scripts/scope_budget.py display="python3 -I <skill-dir>/scripts/scope_budget.py" -->
```text
python3 -I <skill-dir>/scripts/scope_budget.py init --record RECORD --objective OBJECTIVE --item ITEM [--rollout ROLLOUT] [--session SESSION] [--provider {codex,claude}] [--max-hours MAX_HOURS] [--max-weekly-points MAX_WEEKLY_POINTS] [--max-weekly-percent MAX_WEEKLY_PERCENT] [--reconsider-after-additions RECONSIDER_AFTER_ADDITIONS] --set-by {user,supervisor} [--reason REASON]
python3 -I <skill-dir>/scripts/scope_budget.py add --record RECORD --item ITEM --reason REASON
python3 -I <skill-dir>/scripts/scope_budget.py revise --record RECORD [--max-hours MAX_HOURS] [--max-weekly-points MAX_WEEKLY_POINTS] [--max-weekly-percent MAX_WEEKLY_PERCENT] [--reconsider-after-additions RECONSIDER_AFTER_ADDITIONS] --set-by {user,supervisor} [--reason REASON]
python3 -I <skill-dir>/scripts/scope_budget.py add-session --record RECORD --session SESSION --provider {codex,claude} --from FROM_TIME --bound-by {init,rotation,supervisor,user} [--evidence EVIDENCE]
python3 -I <skill-dir>/scripts/scope_budget.py bind --record RECORD --session SESSION --provider {codex,claude} [--parent PARENT] --bound-by {supervisor,user} --reason REASON
python3 -I <skill-dir>/scripts/scope_budget.py cost-limit --record RECORD --pool {codex,claude_all_models,claude_fable} [--node NODE] [--max-usd MAX_USD] [--mark-usd MARK_USD] --set-by {user,supervisor} --reason REASON
python3 -I <skill-dir>/scripts/scope_budget.py close --record RECORD --disposition {accepted,abandoned} --reason REASON --set-by {user,supervisor} [--at AT]
python3 -I <skill-dir>/scripts/scope_budget.py listen --record RECORD --session SESSION [--since SINCE] [--topic TOPIC] [--interval INTERVAL] [--repository REPOSITORY]
python3 -I <skill-dir>/scripts/scope_budget.py check --record RECORD [--rollout ROLLOUT]
```
<!-- /cli-synopsis -->

## Packets

When forming or revising an organization, including during ongoing implementation with no new dispatch, launch or discovery assignment, use [organization formation](organization-formation.md) to assemble the candidate and bind revision decisions to their authority.

A useful packet conveys the exact accepted authority, source/subject identity, **applicable settled facts**, outcome, checks, limits and escalation owner. Settled facts are constraints already fixed by accepted slices, contracts or committed code - a schema's version gates, an actor's concurrency model, a published wire contract. In S4-S8 about 42% of independent findings reopened such a fact and about 36% more were arguably preventable by better contract formation. Include only the facts that apply; loading everything recreates the context cost. For an integration or replay assignment, include known predecessor attribution and likely derived dependencies such as a lockfile; mark unknown lineage and resolve it before accepting the result, rather than making a final patch stand for its history. Treat admission HEAD as a reference until current affected bytes are bound at execution. Give a child only its packet, its identity binding and mandatory repository context; it loads its own role reference from this skill.

**Write packets in plain sentences.** State each settled fact once, in words, and refer back to it by name; put numbers a reader must compare in a small table. Shorthand such as `original68/24/36/8 and actual0/34/0 inclC7breach` (an S10 supervisor packet, 2026-10-29) saves characters, but every reader must decode it and a misread fact costs more than the words saved. The 27 packets and handoffs of that session came to about 19,700 words.

**Ask for custody records, not hand-written custody.** Builds, tests and frozen subjects are recorded by [`scripts/custody.py`](../scripts/custody.py) as the [worker reference](worker.md#builds-tests-and-frozen-subjects) describes. Do not ask children to copy PIDs, exit codes or tool-session output into separate records.

**Size a packet by what it must read, not only what it must write.** Reading the existing code and contract counts against the same context budget. On Claude Code (2026-10-22) a subagent opened at about 35K; agents told to read a 2,700-line crate plus contract sections spent about 100K before writing, and three in a row handed off with the work unfinished. A successor given an explicit read list (handoff, test support, function signatures via `grep`, and only the contract line ranges each test checks) read 46K and finished at 135K (n=1); a longer read list read 92K. Name the read set as precisely as the write scope, ask for signatures before bodies, and have builders commit after each independently passing unit so a handoff loses nothing.

Size a review the same way: count the reviewer's instructions, the subject and the evidence it must check against its window. On 2026-10-29 an Astra reviewer ran out of context at 225,732 of 258,400 tokens before finishing (n=1). If the necessary reading cannot fit, divide the review into named obligations with a reviewer each, and record any obligation left unreviewed. A smaller packet must not quietly narrow what the review claims to cover.

Before dispatch, compare the packet to the owning contract and current candidate. If the run is meant to be compared with another, freeze the subject, controls and scoring before either arm launches ([measurement](measurement.md#accounting-rules)); a run already started cannot be relabelled prospective.

**Task profile (shadow pilot).** Work only while the opening Status lines of `metrics/dogfood-selection-plan-20261027.md` say a round is active; they name the round's run directory (`<run>` below) and its cap. When the cap is reached, counted as those lines say, stop and tell the owner. Round 2 follows its [screen-first amendment](../../../metrics/dogfood-round2-amendment-proposal-20261029.md): only builder tasks that enter the comparison are profiled, and other dispatches, reviewers included, are not. A production builder dispatch with a write scope is screened; experiment arms, scouts, recon without a write scope and administrative operations are not. When an adapter or launcher starts the model that does the work, screen that model's dispatch once, not the adapter's.

1. **Freeze the packet.** Mention no pilot, profile or revision ID in it, or in any file the child is told to read.
2. **Screen** with `task_profile_record.py enroll --task-id T --packet P --eligible yes|no --reason R`. Your eligibility decision: both arms are acceptable under [model selection](../../../docs/model-selection.md), and the round's review route fits the task.
   - A not-eligible, paused or in-flight-limited screening is recorded there. Dispatch as usual, with no sidecar, profile or accounting.
   - Exit 4 (`profile_required`) means the task is eligible and the round can enroll it. Nothing is recorded yet.
3. **Only then, profile and enroll:**
   - Run `task_profile_record.py start` and keep the time it prints.
   - Write a stage B sidecar from the packet and its authoritative sources, using the template in the plan's Appendix B, under `<run>/sidecars/`: never next to the packet or where a child reads.
     - Declare a git commit (`input_snapshot.git`, with `manifest` not applicable) only when the task's inputs are in this repository at that commit. When they are elsewhere, such as a private variant or candidate repository, set both to unknown with that basis.
     - Name no model, effort or builder. Record unknown fields as unknown; never guess to fill a field.
     - Declare your role as `supervisor`. Its `guidance_sha256` is the `sha256sum` of `<skill-dir>/references/supervisor.md`, where `<skill-dir>` is this skill's directory.
   - Before choosing the model, run `task_profile_record.py profile` from the repository root with `--declare-started-at` set to the start time. A validation failure is recorded and does not block dispatch.
   - Then run `enroll --task-id T --task-revision REV --eligible yes --reason R`, and dispatch the arm it prints. If it reports the task not enrolled, choose as usual. If `enroll` fails or its result is unclear, run it again for the same task ID; only `status --task-id` showing no record makes the task not enrolled.
4. **Record the dispatch.** Run `task_profile_record.py assign --task-revision REV` with:
   - the requested and reported configuration;
   - `--harness` as `<cli>/<version>;tier=<tier>`, such as `codex/0.164.0;tier=priority` or `claude-code/2.1.312;tier=standard`;
   - `--selected-at`, the moment you chose the model, to the microsecond (`start` prints one; never copy another timestamp);
   - the dispatched packet, and the child's session or thread ID.

   Every later dispatch in the episode (continuations, repairs, revised packets and its reviewers) runs `assign --episode T` instead, and is not profiled.
5. **Accounting is not per dispatch.** At each owner check-in and at round close, build one usage ledger for the round's window (`usage_ledger.py build`, `--out` a directory outside `<run>`), then run `task_profile_record.py attribute --all --ledger <dir>`.

**During an episode,** run:
- `review` when the first review completes;
- `repair` at each resubmission after a review that required changes;
- `pause` at once on any P1 against a Terra-arm build;
- `close` at the end (`withdraw` if it was never dispatched).

Do not tell the reviewer of an enrolled episode which model built it. Ask it for the plan's standard verdict with P1–P3 severities, and put the definitions in its packet, since its own scale may differ. P1 blocks acceptance, P2 must be fixed before acceptance, and P3 is an improvement. `accept` allows P3 findings only; `changes_required` needs at least one P1 or P2; `blocked_evidence` means it cannot judge because evidence the packet should have supplied is missing; `blocked_scope` means it cannot judge because the subject or packet scope is wrong. Record them with `review --verdict` and the reviewer's `--reviewer-session`.

<!-- cli-synopsis script=metrics/scripts/task_profile_record.py subs="start profile assign attribute enroll review repair close pause status" display="python3 -I metrics/scripts/task_profile_record.py" -->
```text
python3 -I metrics/scripts/task_profile_record.py start
python3 -I metrics/scripts/task_profile_record.py profile --run-dir RUN_DIR --sidecar SIDECAR --repo REPO --declare-started-at DECLARE_STARTED_AT [--call-id CALL_ID] [--tool-dir TOOL_DIR]
python3 -I metrics/scripts/task_profile_record.py assign --run-dir RUN_DIR (--task-revision TASK_REVISION | --episode EPISODE) --role {builder,reviewer} --requested-model REQUESTED_MODEL --requested-effort REQUESTED_EFFORT --harness HARNESS [--reported-model REPORTED_MODEL] [--reported-effort REPORTED_EFFORT] --run RUN --dispatched-packet DISPATCHED_PACKET --child-session CHILD_SESSION [--selected-at SELECTED_AT] [--retained]
python3 -I metrics/scripts/task_profile_record.py attribute --run-dir RUN_DIR (--task-revision TASK_REVISION | --all) --ledger LEDGER [--assignment ASSIGNMENT] [--since SINCE] [--until UNTIL]
python3 -I metrics/scripts/task_profile_record.py enroll --run-dir RUN_DIR --task-id TASK_ID --reason REASON (--task-revision TASK_REVISION | --packet PACKET) --eligible {yes,no}
python3 -I metrics/scripts/task_profile_record.py review --run-dir RUN_DIR --task-id TASK_ID --verdict {accept,changes_required,blocked_evidence,blocked_scope} --reviewer-session REVIEWER_SESSION --p1 P1 --p2 P2 --p3 P3 --receipt RECEIPT
python3 -I metrics/scripts/task_profile_record.py repair --run-dir RUN_DIR --task-id TASK_ID --review-receipt REVIEW_RECEIPT
python3 -I metrics/scripts/task_profile_record.py close --run-dir RUN_DIR --task-id TASK_ID --reason REASON --disposition {accepted,abandoned} --pre-review-fixes PRE_REVIEW_FIXES --escalations ESCALATIONS [--review-receipt REVIEW_RECEIPT] [--artifact ARTIFACT]
python3 -I metrics/scripts/task_profile_record.py pause --run-dir RUN_DIR --reason REASON
python3 -I metrics/scripts/task_profile_record.py status --run-dir RUN_DIR [--task-id TASK_ID]
# other subcommands (put --help after one for its usage): check-tool, verify, init-round, recover-screening, resume, withdraw
```
<!-- /cli-synopsis -->
<!-- cli-synopsis script=metrics/scripts/usage_ledger.py subs="build" display="python3 -I metrics/scripts/usage_ledger.py" -->
```text
python3 -I metrics/scripts/usage_ledger.py build [--since SINCE] [--until UNTIL] --out OUT [--no-codex] [--no-claude] [--streams [STREAMS ...]]
# other subcommands (put --help after one for its usage): reconcile-trial, windows, tree, control
```
<!-- /cli-synopsis -->

Never put the sidecar or profile in a child's packet or prompt.

## Dispatch and the retained-or-fresh choice

Before dispatch, designate who collects each result and who resumes you. End your turn only when no required collection or integration remains, or a **verified** external mechanism owns resumption; human visibility is not automatic wakeup. If neither waiting nor external resumption is available, surface the blocked handoff rather than assuming completion will restart you. Use `spawn_agent` with `fork_turns="none"` for a new context and `followup_task` only for an intended retained continuation; a message to an idle agent is not a launch. **Before any substantial `followup_task`, run the pre-dispatch check** in [context boundary](context-boundary.md) and record retain or fresh with its reason. Request the model and effort from the active user selection ([model selection](../../../docs/model-selection.md)); record requested and observed identity separately.

Assign each worker one coherent outcome with explicit file/process ownership and the current packet revision. After launch, verify its session and send its identity binding as [SKILL.md](../SKILL.md#launching-a-child) describes. Compare the requested model and effort with the launch configuration promptly; a mismatch needs an explicit disposition before that worker does engineering, and its consumed work remains part of accounting. When a child session identity matters for handoff, review continuity or measurement, bind it to actual runtime lineage; an inherited shell thread ID may name the parent. Distinguish launch configuration and recorded turn context from backend identity if the latter is not exposed.

Keep a durable, compact assignment record as the deterministic source for owners, packet revisions, context checks, expected deliverables and received results. Update it on actual assignment, message, completion and handoff events; do not call a model or `list_agents` to rediscover unchanged state.

## Next task, remediation and handoff

Before giving a retained worker a substantial new task - **including remediation after review** - run the pre-dispatch check in [context boundary](context-boundary.md) and record retain or fresh with its reason. The trigger is a pilot hypothesis (H1), and you may overrule its recommendation with a recorded reason. For a finished builder over the trigger, you write the remediation packet from its result, continuation and the review findings, and give it to a fresh worker. If work is unfinished, receive a durable continuation naming live ownership, decisions, failures, evidence pointers, remaining obligations and next action. Check it is safe to transfer, then follow the transfer steps in that reference.

## Collection and refresh

Continue independent work while children run; when required results remain and none remains, wait on the event rather than polling ([call cost](call-cost.md)). Completion wakes collection; it does not certify success. On a result, check the artifact and process dispositions, validate it as below, obtain required review, and ask whether any changed fact invalidates other packets, evidence or acceptance claims. Preserve old evidence as historical; retain current claims only with a reason they still apply. New hashes cannot repair a wrong subject or missing semantic obligation. Issue revised packets when necessary. Before an absence claim, establish what artifact classes could contain the item and why the checked scope covers them; inspect a judgment's referenced inputs before inferring that an item omitted from its summary was never considered. Do not infer that outside-packet findings are absent because none were reported. Confirm no child or process ownership is stranded before replacing a context or declaring the work complete.

**Other sessions.** Concurrent sessions share this repository but not your context, so a change they never hear about can invalidate their work, and theirs yours. When your work depends on or affects other sessions, load [coordination channels](../../coordination-channels/SKILL.md): it owns claiming, notifying, listening and handing off through the coordination board.

## Validation and review

Bind current affected bytes when work begins - an [evidence battery](../../evidence-battery/SKILL.md) plan over the affected and dirty paths, saved for later `--compare`; an admission HEAD can be stale or omit dirty work. Verify the actual subject the worker exercised: source revision, launch path, binary/profile, store schema and selected input/output matter when the claim depends on them. A true observation of a narrower subject remains useful history, but cannot satisfy the assigned subject. Inspect material failures and their cause before acceptance; preserve unsuccessful logs and corrected-route lineage. Process custody comes from the worker's `custody.py` records ([worker](worker.md#builds-tests-and-frozen-subjects)): check their outcomes and leftover processes, rather than asking for hand-written PID or exit records.

Validate the result against the owning contract and actual required gates, not merely the packet's checklist. Run required gates and validate affected downstream consumers, including those reached through shared contracts and schemas; shared-boundary changes require joined-workspace qualification (an S8 schema bump broke 14 S7 tests that the changed package's checks did not exercise). Report unavailable or incomplete gates explicitly. Obtain retained review where the owner requires it. Preserve the review episode and actual continuity: if a reviewer runtime is lost, an authorized reconstructed successor is a reconstructed continuation, not fresh independence or `same_session`. Do not write a review-state transition that falsifies continuity. An immutable external report may support bounded semantic acceptance when that owner permits it; disclose absent state linkage and refer infrastructure repair to its owner. Review-state persistence, semantic acceptance and integration have different owners.

## Your own context

A root accepting substantial new work makes a scoped
[capacity decision](context-boundary.md#capacity-belongs-to-the-next-scope),
including the work and collection still owed. An earlier decision to finish
bounded work does not cover the new task, even when no new pressure alert arrives.

A root's context grows with every report; roots averaged about 144K per call in the baseline, and the S10 root of 2026-10-28/29 averaged 246K over 596 calls. Keep it small by delegating reads to children and reading their compact results. Check a builder's evidence through its deterministic records - `custody.py` outcomes and an evidence battery `--compare` - and open logs only for a failure; reading the evidence in depth belongs to the reviewer. When you take over from a predecessor, start from compact working context recovered from the durable state (assignment record, continuation and handoffs), not from a retained large context. Record your reading at each dispatch and integration boundary (a current [listener observation](context-control.md#what-to-do-with-an-observation) counts), and choose a response against the next scope's needs and available capabilities. Pressure alone does not require a root transfer. For same-session replacement, use the [continuity guidance](same-session-context.md); when transferring ownership, use the user-authorized quiescent procedure under [Roots](context-boundary.md#roots). Keep the continuation and assignment record current continuously, because recovery cannot depend on meaning held only in the old context.

## Worktrees and Codebase Memory

A lasting Codebase Memory watch belongs to a live MCP connection rooted at that worktree; querying or indexing another worktree does not register one, and the main repository and a slice worktree are distinct projects. At allocation or recovery, check the exact project root, `index_status` watch state and the current store generation. A watch marked active does not prove the latest edit is indexed: after edits, check candidate-file coverage and use direct source for unverified ranges. A manual `index_repository` with `persistence=false` avoids repository writes but keeps no watch. Tool availability can change mid-session, so rediscover the available MCP tools before falling back to CLI or source. Workspace cleanup does not authorize stopping a shared daemon or another agent's connection. [Repo-search](../../repo-search/SKILL.md) owns query, coverage and fallback rules.

## Acceptance and result

Keep acceptance with the owner named by the accepted contract. Acceptance of a bounded subject does not imply integration or full product acceptance, and a private readiness finding is an input to your integration decision, not the decision itself. Recheck affected and overlapping destination bytes against the prepared baseline before deciding integration, and bind the destination HEAD in the same operation that integrates: abort if HEAD is not the commit the decision named (other sessions commit to shared branches; on 2026-10-22 one did between a check and a cherry-pick); an [evidence battery](../../evidence-battery/SKILL.md) over those paths with `--compare` is the cheap form. Record open obligations and unavailable evidence in the final disposition. Cost and comparison rules are in [measurement](measurement.md).

Report to the user with a bound result: the exact accepted or stopped scope, validation and reviewer evidence, limitations, open findings, requested/observed identities, context checks and dispatch decisions, child collection and process disposition, and the next owner or the decision needed. Never infer full campaign acceptance from a bounded result.
