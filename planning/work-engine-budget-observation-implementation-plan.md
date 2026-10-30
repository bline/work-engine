# Objective cost observation (plan)

**Revision 3, 2026-10-29.** Owner: Claude (Claude Code, `claude-opus-5-5`), at
the user's request. Revision 1 was drafted by ChatGPT in the web interface with
Extra High reasoning (model not shown) against `471f6106`, and is committed
unchanged in `c2ccc569`. Revision 2 (`ef013ac3`) was Claude's rewrite. Revision
3 records the user's answers to revision 2's four decisions (§9) and what
follows from them.

**Status:** plan. Implementation has not started, and each phase starts on the
user's go-ahead. Changes to released skills, or to an active experiment's
environment, follow those owners' rules.

**Scope:** the outer development root: `skills/`, the coordination board
(`chatboard`) and `metrics/` tooling. These are Work Engine's original
workflows, which are still used to develop the Rust replacement. The plan
extends existing Python tools in that root and adds no component to the Rust
engine. `app-server/` was abandoned and is out of scope.

## 1. Direction

The user, 2026-10-29:
- the weekly percentage is not a good measure, because several agents run at
  once and all of them move it;
- the current approach is too simple;
- agents should not have to monitor their own usage, which adds complexity
  when the work can be done deterministically.

So this plan measures what each objective's agents spend, from their
transcripts, in a host process. It tells the coordinator only when something
changes that bears on a decision. When the work is finished, no agent runs a
usage check, estimates its own cost or polls. `DESIGN.md` §9 and §14 say the same: fully determined work belongs
to deterministic machinery, not to model reasoning.

## 2. What revision 2 changes

| Revision 1 | Revision 2 | Why |
|---|---|---|
| Agents estimate per-assignment cost. The coordinator issues allocations, keeps reserves and approves leeway, and allocations are conserved. | Observation first. Limits are optional: a user limit binds, and a coordinator mark only produces an event for the coordinator. Allocations, leeway and expected costs are deferred to phase 5 and decided from shadow data. | Every allocation is a number the coordinator must choose at dispatch, and every wrong one becomes a request and an approval. In the S10 session of 2026-10-28/29 (n=1), calls doing only coordination or bookkeeping were already 58% of the tree's spend (a regex classification of call purpose, spot-checked 8 of 8). `DESIGN.md` §11 also says not to manufacture approval requirements. |
| An incremental cost engine, reconciled against the batch ledger | The existing ledger rebuilds the objective's window at each tick | A full build took 1.2 s for 24 hours of both providers (4,648 calls) and 3.4 s for 7 days of Codex (31,580 calls), one run each. With one counter there is nothing to reconcile. |
| A new budget registry beside `scope_budget` | Version 2 of the `scope_budget` record holds the binding and the limits | One record and one owner script, so no competing authority |
| Lineage left to discovery | Lineage sources named and measured (§4.2) | Most of the tree is already linked |
| None | Phase 0 corrects `scope_budget`'s weekly-point limit now | It is live and charges each objective with everyone's spend (§3) |
| None | Ledger robustness | A 7-day build stops on one bad record (§4.1) |

Kept from revision 1:
- the weekly meter as an account-wide signal only;
- each call charged once, with direct and inclusive totals;
- unknown never shown as zero;
- observation grants no authority;
- a separate cursor in `chatboard wait`;
- shadow mode before any enforcement;
- the shared-account isolation check.

## 3. The problem in the current release

`scope_budget.py`, released in `471f6106`, works like this:
- `init` records the account's Codex weekly meter;
- `check` stops the objective (exit 4) when the meter's rise reaches
  `max_weekly_points`;
- `coordinator.md` tells coordinators to propose limits when the user gave none,
  and to run `check` with every dispatch.

The meter is account-wide:
- **S10 window** (2026-10-28 22:00Z to 2026-10-29 15:37Z): the S10 root's tree
  spent $133.73 of the account's $290.57 Codex API-equivalent spend, 46%. The
  tree was built from Codex parent links plus `role_host` launch links.
- **2026-10-29, 20:53Z to about 02:00Z:** three roots ran: RP3 `01a18390`, S10
  `01a18305` and the bounded-inference B3 successor. In 8 records (assignment
  records and continuations), their coordinators set `max_weekly_points` limits of 2 to 10 on the same meter.
  The S10 tree was 26% of the Codex spend in that window. In dollar terms, about
  three quarters of the spend that moved the meter came from other trees, and
  all of it counted against S10's 2-point limit. Its share of quota is not
  established, because the meter's charge per dollar differs by model.

## 4. Existing machinery

### 4.1 The ledger

`metrics/scripts/usage_ledger.py` (F1/F1.1) already does the counting:
- Codex calls are counted by unique `response_id`, and Claude calls by
  `message.id` across main and subagent transcripts. Forked counters are
  handled.
- Calls are priced at the frozen Standard API scenario. Claude cost is kept as a
  range when the cache TTL is unknown, Codex Fast tier is labelled with its quota
  multiplier, and unknown prices stay null.

Measured 2026-10-29, one run each:

| Build | Time | Memory |
|---|---|---|
| 24 h, both providers | 1.2 s | 36 MB |
| 7 days, Codex only | 3.4 s | 107 MB |

**Defect:** `native_usd` raises `ValueError('TTL exceeds cache write')`, and the
whole build stops. In the 7 days before 2026-10-30 02:00Z, 4 Claude messages
(8 records) report `cache_creation_input_tokens: 0` with nonzero
`ephemeral_1h_input_tokens`. That is out of 34,717 records with a cache split.

**Price gaps.** The ledger prices two Claude models, Sonnet 5.5 and Opus 5.5.
In the 7 days before 2026-10-30 02:00Z, Claude transcripts also held messages
from Opus 5 (3,365), Fable 5.1 (222), Opus 4.8 (51) and Haiku 5.5 (15), each
message counted once per transcript file. Calls to those models come out
unpriced. All 31,580 Codex calls in the same 7 days had a price, and 880 of
them ran on Fast tier.

**Edits take effect at once.** Coordinators run
`python3 -I metrics/scripts/usage_ledger.py build` from this working tree, and
`task_profile_record.py attribute` reads that build's `calls.jsonl`. So an edit
to the ledger reaches every coordinator immediately, while a skill change waits
for a release. The ledger is also the experiment programme's F1 instrument
(`metrics/experiment-plan-20261026.md`), and that plan's re-pin rule makes a
change to it an environment change.

`metrics/scripts/` is not part of skill releases, which export only `skills/`,
so a released tool uses the ledger by a pinned path (§9, decision 4).

### 4.2 Lineage

| Source | Link |
|---|---|
| Codex native children | `session_meta.parent_thread_id`. The ledger computes `root_thread` from these links only. |
| Roles launched through `role_host.py` | Its ledger (`~/.local/state/work-engine/role-host/ledger.jsonl`) records `parent_session_id` for Codex and Claude launches. Codex `deliver` rows carry the `turn_id` they started. |
| Claude subagents | Transcripts under the parent session's `subagents/` directory, which the ledger already reads |
| Root successors | `root_rotation.py` verifies the successor's adoption, but nothing records the successor in a cost binding yet. |

Coverage in the 24 h before 2026-10-30 02:00Z (one window):
- 32 of 46 Codex threads had a native parent link.
- Of the 14 top-level threads, 11 were `role_host` launches: 8 with a parent
  and 3 roots.
- 3 had no link. They spent $10.55 of the $115.50 the 14 top-level threads spent
  in their own calls. Whether they belong to an objective is not established.

### 4.3 The listener

`chatboard wait --follow` combines board messages with a context source.
`ContextControl`:
- runs a reader pinned by path and SHA-256;
- keeps an opaque `next_control_cursor`;
- accepts only `observation_only` events;
- stops with an error when the reader fails.

Delivery to the model follows `skills/manual-orchestration/references/context-control.md`:
a named collector, and delivery verified from collected output, never claimed
from stdout alone.

## 5. Invariants

Each is binding because the failure named with it follows when it breaks.

1. **One charge per call.** A call belongs to at most one node of one objective,
   and an ancestor's inclusive total is a view, not another charge. Otherwise
   totals overstate spend, and limits stop work early.
2. **Unknown is not zero.** Unpriced calls, cost ranges, unlinked sessions and
   reader failures are reported as such. A zero would let work continue past a
   limit it has already reached.
3. **The weekly meter is account-wide.** It is never subtracted across time to
   charge an objective (§3).
4. **Observation grants no authority.** Budget events are local, typed
   `observation_only` notifications. They are not board posts, approvals or
   user messages.
5. **Only the user changes a user limit.** A coordinator mark is the
   coordinator's own tool. It needs no approval to set or move, and crossing it
   is information, not a stop.
6. **The binding is durable, and observations can be rebuilt.** The binding and
   limits live in the assignment record. After a listener restart, any snapshot
   can be rebuilt from the record and the transcripts.

## 6. Design

### 6.1 Binding: `scope_budget` version 2

The coordinator's assignment record already holds `scope_budget`: objective,
original items, additions, limits and history. Version 2 adds:
- `sessions`: the root session, plus each successor appended at a verified
  rotation, with the time it took over;
- `bindings`: explicit links for sessions no lineage source connects, such as a
  top-level reviewer started outside `role_host`, each with who bound it and
  when;
- cost limits in quota-weighted USD for each quota pool (§9, decision 2):
  `max_usd`, binding when the user set it, and `marks_usd`, levels that produce
  events. Either can also be set per node.

Changes to existing limits:
- `max_weekly_points` is no longer accepted for new limits.
- `max_weekly_percent` stays, as an account-wide ceiling, and is reported as
  account-wide.
- Version 1 records stay readable, and a user-set version 1 limit keeps its
  meaning until the user replaces it.

`root_rotation.py` appends the successor to `sessions` after verifying adoption,
so a rotation cannot reset the objective's spend.

### 6.2 Cost tree: `usage_ledger.py tree`

Given the record and a window (by default from `started_at`), `tree` builds the
ledger in memory and joins the three lineage sources with the record's bindings.
It prints one JSON document with:
- for each node:
  - session, provider, and role name where one is recorded (the `role_host` name
    or the Codex `agent_nickname`);
  - models and tiers;
  - direct and inclusive spend for each quota pool, in quota-weighted USD (low
    and high where ranged);
  - unpriced call count, and first and last call;
- totals for the objective and for the window;
- the unattributed remainder, with its session list, so coverage gaps are
  visible;
- the reader's SHA-256 and the snapshot time.

A session reused for several assignments counts as one node in this plan.
Splitting it at assignment boundaries is investigated in phase 1, and built only
if phase 4 shows it is needed. `role_host` delivery turn IDs cover Codex roles;
native children do not go through `role_host`.

### 6.3 Listener source: `wait --budget-record`

A `BudgetControl` adapter sits beside `ContextControl` and has the same shape:
- **Reader:** `metrics/scripts/usage_ledger.py`, pinned by path and digest, as
  the context reader is (§9, decision 4).
- **Cursor:** its own `next_budget_cursor`. Results go in `budget_events`, so
  `messages`, `next_since` and `control_events` are unchanged.
- **Interval:** it runs `tree` at most once per interval, a setting that starts
  at 60 s.
- **No repeats:** the cursor records the last reported state of each trigger.
  An unchanged state produces no event, and a restart from the last consumed
  cursor does not repeat it.

Events are sent only when a state changes:
- objective spend crosses a mark or a set fraction of the user limit, or reaches
  the user limit;
- the hours limit is approaching or reached;
- a node crosses its own mark, if one is set;
- coverage changes: the reader fails or recovers, or unpriced or ranged calls
  appear in the tree;
- the account meter crosses a user-set `max_weekly_percent`, labelled
  account-wide.

Events go to the coordinator only (§9, decision 3). Supervisors, builders and
reviewers get none of their own; the coordinator passes down what it judges
useful.

### 6.4 At a limit

At a user limit, the coordinator does what `coordinator.md` already says:
dispatch nothing new, collect outstanding results or handoffs, and report.

Phase 5 can add a deterministic refusal in `role_host launch` for an objective
at its user limit. Native Codex spawns cannot be intercepted there, and the
coverage report says so.

A running call can overshoot a limit before its record appears. The event
reports the overshoot; it does not claim a hard stop.

### 6.5 Deferred, and what would justify each

- **Per-assignment allocations, reserves, leeway requests and approvals:** phase
  4 shows coordinators acting on subtree spend often enough that marks are not
  sufficient.
- **Expected cost per assignment:** built only if phase 4 shows outlier events
  would have changed a decision. It is computed from history by role and model,
  never asked of agents.
- **Incremental reading:** rebuild time per tick becomes material in phase 4.
- **Splitting reused sessions by assignment:** see §6.2.
- **Converting quota-weighted dollars to meter percentages:** a measured rate
  exists for each pool and model.

## 7. Phases

**0. Correct the current release.**
- In `scope_budget.py`, stop accepting `--max-weekly-points` for new limits, and
  have `check` report the meter's rise as account-wide. It stops on that rise
  only when the user set the limit.
- In `coordinator.md`, stop asking coordinators to propose weekly-point limits.
  Keep the hours limit and its `check` until phase 3 delivers deadline events.
- Ship it in the same release as phase 3, so round 2 is revised once (§9,
  decision 1).

Until then, running roots keep their coordinator-set weekly-point limits. A root
that stops on one reports it, and the user can lift the limit with
`scope_budget.py revise --set-by user`.

**1. Ledger.** Develop it in a separate worktree and land it with the phase 3
release, because ledger edits take effect at once (§4.1).
- Fix the stop: record an issue and price the call as a range covering both
  readings.
- Add prices for the unpriced Claude models, a quota pool for each call, and
  quota-weighted dollars (§9, decision 2).
- Add the `role_host` join, Claude subagent nodes and the `tree` command.
- Test on synthetic transcripts for each lineage type, and replay offline on the
  S10 window and the 2026-10-29 evening window.

**2. Binding.** `scope_budget` version 2 with `bind`, the rotation append, and
version 1 compatibility tests.

**3. Listener.**
- `BudgetControl` and `wait --budget-record`.
- Documentation in `docs/codex-chatboard.md` and the manual-orchestration
  references.
- The listener replaces the per-dispatch `check`; with no listener bound, the
  coordinator runs an explicit `tree` query instead.
- Release phases 0 to 3 together, and revise round 2 once for the combined
  change.

**4. Shadow pilot.** One or two real objectives, with events delivered to the
coordinator and no new stop behaviour. Measure:
- rebuild time per tick;
- the events, and the model turns they caused;
- the unattributed share;
- verified delivery.

Report to the user with a recommendation on each §6.5 item.

**5. User decision.** Admission refusal at `role_host launch`, and any §6.5 item
the pilot justifies.

## 8. Acceptance checks

Each needs test or recorded evidence. Anything not tested live is reported as
not tested.

1. **Conservation:** for a window and per quota pool, the bound trees plus the
   unattributed remainder equal the ledger total, and no call is in two trees.
2. **Isolation:** in the 2026-10-29 evening window, the three roots' trees do not
   overlap.
3. **Lineage:** each of these lands under the right node, with no inherited
   counter counted twice:
   - a native Codex child;
   - a `role_host` Codex child and a `role_host` Claude child;
   - a Claude subagent;
   - a root successor.
4. **Honesty:** each of these produces an explicit state, and no single record
   stops the build:
   - an unpriced model;
   - an unknown cache TTL;
   - an inconsistent cache split;
   - a missing transcript;
   - an unreadable `role_host` ledger.
5. **One counter:** the listener's figures equal `usage_ledger.py tree` for the
   same snapshot.
6. **Listener:**
   - one `wait --follow` carries board, context and budget sources with separate
     cursors, and a budget event leaves the other cursors alone;
   - no event is sent while a state is unchanged;
   - restarting from the last consumed cursor does not repeat consumed events.
7. **Overhead:** the pilot measures rebuild time per tick and the model turns
   budget events caused.
8. **Compatibility:**
   - version 1 records stay readable;
   - existing `scope_budget`, chatboard and context tests pass;
   - user-set limits behave as before.
9. **No self-monitoring:** no released instruction asks a builder, reviewer or
   supervisor to check or estimate its own usage.

## 9. Decisions (user, 2026-10-29)

1. **Phase 0 waits for the rest.** Phases 0 to 3 ship in one release, and round
   2 is revised once for it. Round 2's plan reports each environment change and
   doesn't compare records from before and after one directly, so a single
   change splits its records once instead of several times. The cost of waiting
   is in §7, phase 0.
2. **The unit is quota-weighted dollars, kept per quota pool.** Quota-weighted
   dollars are API-equivalent list-price dollars times the call's tier
   multiplier (Codex Fast is 2.5). Each pool has its own limits: Codex weekly,
   Claude main weekly, and Claude Fable. Reasons:
   - **Codex quota followed dollars, not tokens.** A fit over five weekly
     windows (2026-10-20 to 10-26) found that Astra drew 4.4 to 4.7 times Sol's
     quota per weighted token, close to their list-price ratio. A token limit
     would let an Astra-heavy objective use 4 to 5 times the quota of a Sol one
     under the same number. The fit's scripts were not kept, so it is not re-run
     here.
   - **List dollars leave out Fast tier's 2.5 times charge.** The ledger already
     records each call's tier.
   - **The pools are metered separately.** The user sees Fable's usage as its
     own percentage, and the pools' dollar rates differ and are only partly
     measured, so one combined figure would mix them. One reading on 2026-10-27
     (n=1, 1% steps) also showed the main Claude meter rise from 8 to 10 in
     four minutes while three Fable agents spent $7.46 and Opus $2.62. That may
     mean Fable also draws on the main meter; it is not established. Either
     way, Fable spend is reported in its own pool.
   - **Dollars keep their meaning across the 2026-10-29 limit doubling;** meter
     percentages don't.

   Not established: whether Claude's quota follows list price the way Codex's
   did.
3. **Events go to the coordinator only.** Information flows down from the
   coordinator, at its judgment.
4. **The ledger stays at `metrics/scripts/usage_ledger.py` in Work Engine,** and
   the listener pins it by path and digest. These are the round 2 agent's
   reasons, checked here:
   - it is the experiment programme's F1 instrument, named by that path;
   - 16 tracked files refer to it;
   - so moving it would break references, and would be an environment change
     under the re-pin rule.

   That agent's proposed per-episode cost records for round 2, kept in
   `agent-routing-data`, would be extracted from the same ledger, so there is
   still one counter.
