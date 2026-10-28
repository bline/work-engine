# Coordinator: decomposition, dispatch and collection

Own the accepted objective and its authority map, current decomposition, packet readiness, cross-slice evidence, dependent refreshes, child collection, and root integration decision. Treat a decomposition as a versioned model of the work, not an automatic validated planner. Choose coherent outcomes and dependencies from the actual task; explain uncovered or uncertain boundaries.

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

## Packets

A useful packet conveys the exact accepted authority, source/subject identity, **applicable settled facts**, outcome, checks, limits and escalation owner. Settled facts are constraints already fixed by accepted slices, contracts or committed code - a schema's version gates, an actor's concurrency model, a published wire contract. In S4-S8 about 42% of independent findings reopened such a fact and about 36% more were arguably preventable by better contract formation. Include only the facts that apply; loading everything recreates the context cost. For an integration or replay assignment, include known predecessor attribution and likely derived dependencies such as a lockfile; mark unknown lineage for the supervisor to resolve rather than making a final patch stand for its history. Treat admission HEAD as a reference until current affected bytes are bound at execution. Give a child only its packet, its identity binding and mandatory repository context; it loads its own role reference from this skill.

**Size a packet by what it must read, not only what it must write.** Reading the existing code and contract counts against the same context budget. On Claude Code (2026-10-22) a subagent opened at about 35K; agents told to read a 2,700-line crate plus contract sections spent about 100K before writing, and three in a row handed off with the work unfinished. A successor given an explicit read list (handoff, test support, function signatures via `grep`, and only the contract line ranges each test checks) read 46K and finished at 135K (n=1); a longer read list read 92K. Name the read set as precisely as the write scope, ask for signatures before bodies, and have builders commit after each independently passing unit so a handoff loses nothing.

Before dispatch, compare the packet to the owning contract and current candidate. If the run is meant to be compared with another, freeze the subject, controls and scoring before either arm launches ([measurement](measurement.md#accounting-rules)); a run already started cannot be relabelled prospective.

**Task profile (shadow pilot).** Profile only while the opening Status lines of `metrics/dogfood-selection-plan-20261027.md` say a round is active; they name the round's run directory (`<run>` below) and its cap. When `<run>/task-profiles/` reaches the cap, stop and tell the owner. A dispatch is eligible when production work sends a builder or reviewer a write scope or a review subject; experiment arms, scouts, recon without a write scope and administrative operations are not. When an adapter or launcher starts the model that does the work, profile that model's dispatch once, not the adapter's.
1. Freeze the packet. Mention no pilot, profile or revision ID in it, or in any file the child is told to read.
2. Run `task_profile_record.py start` and keep the time it prints. Then write a stage B sidecar from the packet and its authoritative sources, using the template in the plan's Appendix B, under `<run>/sidecars/`: never next to the packet or where a child reads. Declare the snapshot as a git commit (`input_snapshot.git`) with `manifest` set to `not_applicable`. Name no model, effort or builder. Record unknown fields as unknown; never guess to fill a field. Its `guidance_sha256` is the `sha256sum` of `<skill-dir>/references/coordinator.md`, where `<skill-dir>` is this skill's directory.
3. Before choosing the model, run `task_profile_record.py profile` from the repository root with `--declare-started-at` set to the start time. A validation failure is recorded and does not block dispatch.
4. After dispatch, run `task_profile_record.py assign` with the requested and reported configuration; `--harness` as `<cli>/<version>;tier=<tier>`, such as `codex/0.164.0;tier=priority` or `claude-code/2.1.312;tier=standard`; `--selected-at`, the moment you chose the model, to the microsecond (`start` prints one; never copy another timestamp); the dispatched packet; and the child's session or thread ID. After the child finishes, build a usage ledger covering its run (`usage_ledger.py build`, `--since` the dispatch time, `--out` a directory outside `<run>`) and run `task_profile_record.py attribute` with it as `--ledger`.

<!-- cli-synopsis script=metrics/scripts/task_profile_record.py subs="start profile assign attribute" display="python3 -I metrics/scripts/task_profile_record.py" -->
```text
python3 -I metrics/scripts/task_profile_record.py start
python3 -I metrics/scripts/task_profile_record.py profile --run-dir RUN_DIR --sidecar SIDECAR --repo REPO --declare-started-at DECLARE_STARTED_AT [--call-id CALL_ID] [--tool-dir TOOL_DIR]
python3 -I metrics/scripts/task_profile_record.py assign --run-dir RUN_DIR --task-revision TASK_REVISION --role {builder,reviewer} --requested-model REQUESTED_MODEL --requested-effort REQUESTED_EFFORT --harness HARNESS [--reported-model REPORTED_MODEL] [--reported-effort REPORTED_EFFORT] --run RUN --dispatched-packet DISPATCHED_PACKET --child-session CHILD_SESSION [--selected-at SELECTED_AT] [--retained]
python3 -I metrics/scripts/task_profile_record.py attribute --run-dir RUN_DIR --task-revision TASK_REVISION --ledger LEDGER [--assignment ASSIGNMENT] [--since SINCE] [--until UNTIL]
# other subcommands (put --help after one for its usage): check-tool, verify
```
<!-- /cli-synopsis -->
<!-- cli-synopsis script=metrics/scripts/usage_ledger.py subs="build" display="python3 -I metrics/scripts/usage_ledger.py" -->
```text
python3 -I metrics/scripts/usage_ledger.py build [--since SINCE] [--until UNTIL] --out OUT [--no-codex] [--no-claude] [--streams [STREAMS ...]]
# other subcommands (put --help after one for its usage): reconcile-trial, windows
```
<!-- /cli-synopsis -->

Never put the sidecar or profile in a child's packet or prompt.

## Dispatch and the retained-or-fresh choice

Before dispatch, designate who collects each result and who resumes the parent. End your turn only when no required collection or integration remains, or a **verified** external mechanism owns resumption; human visibility is not automatic wakeup. If neither waiting nor external resumption is available, surface the blocked handoff rather than assuming completion will restart you. Use `spawn_agent` with `fork_turns="none"` for a new context and `followup_task` only for an intended retained continuation; a message to an idle agent is not a launch. **Before any substantial `followup_task`, run the pre-dispatch check** in [context boundary](context-boundary.md) and record retain or fresh with its reason. Request the model and effort from the active user selection ([model selection](../../../docs/model-selection.md)); record requested and observed identity separately.

Keep a durable, compact assignment record as the deterministic source for owners, packet revisions, context checks, expected deliverables and received results. Update it on actual assignment, message, completion and handoff events; do not call a model or `list_agents` to rediscover unchanged state.

## Collection and refresh

Continue independent work while children run; when required results remain and none remains, wait on the event rather than polling ([call cost](call-cost.md)). Completion wakes collection; it does not certify success. On a result, check the artifact and process dispositions, obtain required supervisor/reviewer decisions, and ask whether any changed fact invalidates other packets, evidence or acceptance claims. Preserve old evidence as historical; retain current claims only with a reason they still apply. New hashes cannot repair a wrong subject or missing semantic obligation. Issue revised packets when necessary. Before an absence claim, establish what artifact classes could contain the item and why the checked scope covers them; inspect a judgment's referenced inputs before inferring that an item omitted from its summary was never considered. Do not infer that outside-packet findings are absent because none were reported. Confirm no child or process ownership is stranded before replacing a context or declaring a group complete.

**Other sessions.** Concurrent sessions share this repository but not your context, so a change they never hear about can invalidate their work, and theirs yours. When your work depends on or affects other sessions, load [coordination channels](../../coordination-channels/SKILL.md): it owns claiming, notifying, listening and handing off through the coordination board.

## Your own context

A root accepting substantial new work makes a scoped
[capacity decision](context-boundary.md#capacity-belongs-to-the-next-scope),
including the work and collection still owed. An earlier decision to finish
bounded work does not cover the new task, even when no new pressure alert arrives.

A root's context grows with every report; roots averaged about 144K per call in the baseline. Keep it small by delegating reads to children and reading their compact results, not their evidence. Record your reading at each dispatch and integration boundary, and transfer at a quiescent boundary through the user as the [roots](context-boundary.md#roots) section describes. Keep the continuation and assignment record current continuously, because a ceiling replacement keeps only what was written.

## Worktrees and Codebase Memory

A lasting Codebase Memory watch belongs to a live MCP connection rooted at that worktree; querying or indexing another worktree does not register one, and the main repository and a slice worktree are distinct projects. At allocation or recovery, check the exact project root, `index_status` watch state and the current store generation. A watch marked active does not prove the latest edit is indexed: after edits, check candidate-file coverage and use direct source for unverified ranges. A manual `index_repository` with `persistence=false` avoids repository writes but keeps no watch. Tool availability can change mid-session, so rediscover the available MCP tools before falling back to CLI or source. Workspace cleanup does not authorize stopping a shared daemon or another agent's connection. [Repo-search](../../repo-search/SKILL.md) owns query, coverage and fallback rules.

## Acceptance

Keep acceptance with the owner named by the accepted contract. Supervisor acceptance of a bounded subject does not imply integration or full product acceptance. Recheck affected and overlapping destination bytes against the prepared baseline before deciding integration, and bind the destination HEAD in the same operation that integrates: abort if HEAD is not the commit the decision named (other sessions commit to shared branches; on 2026-10-22 one did between a check and a cherry-pick); an [evidence battery](../../evidence-battery/SKILL.md) over those paths with `--compare` is the cheap form. Record open obligations and unavailable evidence in the final disposition. Cost and comparison rules are in [measurement](measurement.md).
