# Objective cost observation (plan)

**Revision 3, 2026-10-29.** Owner: Claude (Claude Code, `claude-opus-5-5`), at
the user's request. Revision 1 was drafted by ChatGPT in the web interface with
Extra High reasoning (model not shown) against `471f6106`, and is committed
unchanged in `c2ccc569`. Revision 2 (`ef013ac3`) was Claude's rewrite. Revision
3 records the user's answers to revision 2's four decisions (§9) and what
follows from them.

**Status (2026-10-30):** phases 0 to 3 are implemented on branch
`budget-obs-integration` (head `1ecc1917`). Each unit passed independent review:

| Unit | Accepted at | Review rounds |
|---|---|---|
| `wait` budget source | `07bd595f` | 2 |
| `scope_budget` version 2 and rotation binding | `7c131795` | 2 |
| ledger `tree`, `control`, prices and pools | `9b911be2` | 3 |

The branch includes main's removal of the coordinator role (`5695755f`). It is
not merged into the main working tree and not released. Supervisors run the
ledger from the working tree, so a merge there takes effect at once. Merging and releasing need the user's go-ahead, and round 2 is revised
for the change then.

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
transcripts, in a host process. It tells the top-level supervisor only when something
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

**Calls missing from transcripts.** Claude Code makes calls that leave no usage
in the transcript, including compaction summaries and session-title calls
(Haiku). One session compared (n=1), session `460f623d` at about 2026-10-30
02:30Z:

| Source | Cost |
|---|---|
| Claude Code's own usage screen | $20.93 (Opus 5.5: 7.0k input, 328.0k output, 36.4M cache read, 882.9k cache write) |
| Ledger, same transcript, up to the same moment | $20.10 |

- **Prices agree.** Claude Code's token counts, priced at the ledger's Opus 5.5
  rates, give $20.93, so the gap is in coverage.
- **Part of the gap is explained.** The session had three automatic
  compactions, each over 213K to 247K tokens of context, and they account for
  part of the gap. The rest is not established.
- **No cross-check ran.** The transcript has no `cost-state` record, so the
  ledger's existing check against Claude Code's counter could not run.

**Edits take effect at once.** Supervisors (formerly coordinators) run
`python3 -I metrics/scripts/usage_ledger.py build` from this working tree, and
`task_profile_record.py attribute` reads that build's `calls.jsonl`. So an edit
to the ledger reaches every supervisor immediately, while a skill change waits
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
5. **Only the user changes a user limit.** A supervisor mark is the
   supervisor's own tool. It needs no approval to set or move, and crossing it
   is information, not a stop.
6. **The binding is durable, and observations can be rebuilt.** The binding and
   limits live in the assignment record. After a listener restart, any snapshot
   can be rebuilt from the record and the transcripts.

## 6. Design

### 6.1 Binding: `scope_budget` version 2

The top-level supervisor's assignment record already holds `scope_budget`: objective,
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

Events go to the top-level supervisor only (§9, decision 3). Builders and
reviewers get none of their own; the supervisor passes down what it judges
useful.

### 6.4 At a limit

At a user limit, the supervisor does what its guidance (`supervisor.md`, Scope and budget) already says:
dispatch nothing new, collect outstanding results or handoffs, and report.

Phase 5 can add a deterministic refusal in `role_host launch` for an objective
at its user limit. Native Codex spawns cannot be intercepted there, and the
coverage report says so.

A running call can overshoot a limit before its record appears. The event
reports the overshoot; it does not claim a hard stop.

### 6.5 Deferred, and what would justify each

- **Per-assignment allocations, reserves, leeway requests and approvals:** phase
  4 shows supervisors acting on subtree spend often enough that marks are not
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
- In the Scope-and-budget guidance (`supervisor.md` since the coordinator role was removed), stop asking for weekly-point limits.
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
- Add prices for the unpriced Claude models, the quota pools each call counts
  toward (a Fable call counts toward two), and quota-weighted dollars (§9,
  decision 2).
- Report calls the transcript does not record, at least each compaction, as
  unrecorded calls with an estimated range, not as zero. Check against Claude
  Code's own session counter where one can be read.
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
  supervisor runs an explicit `tree` query instead.
- Release phases 0 to 3 together, and revise round 2 once for the combined
  change.

**4. Shadow pilot.** One or two real objectives, with events delivered to the
top-level supervisor and no new stop behaviour. Measure:
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
   - a compaction, whose call the transcript does not record;
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
   multiplier (Codex Fast is 2.5). Each pool has its own limits: Codex
   weekly; Claude all-models weekly, which includes Fable; and Claude Fable
   weekly. Reasons:
   - **Codex quota followed dollars, not tokens.** A fit over five weekly
     windows (2026-10-20 to 10-26) found that Astra drew 4.4 to 4.7 times Sol's
     quota per weighted token, close to their list-price ratio. A token limit
     would let an Astra-heavy objective use 4 to 5 times the quota of a Sol one
     under the same number. The fit's scripts were not kept, so it is not re-run
     here.
   - **List dollars leave out Fast tier's 2.5 times charge.** The ledger already
     records each call's tier.
   - **The pools are metered separately.** Their dollar rates differ and are
     only partly measured, so one combined figure would mix them.
   - **A Fable call counts toward both Claude pools:** the all-models weekly
     limit and Fable's own limit. The user confirmed this on 2026-10-30.
     Claude Code's usage screen and a reading on 2026-10-27 had suggested it.
     Counting the call in both pools is not a second charge, because each pool
     is a separate limit.
   - **Dollars keep their meaning across the 2026-10-29 limit doubling;** meter
     percentages don't.

   Not established: whether Claude's quota follows list price the way Codex's
   did.
3. **Events go to the top-level supervisor only.** Information flows down from
   it, at its judgment. (The user decided "coordinator only" on 2026-10-29. On
   2026-10-30 the user removed the coordinator role, and the top-level
   supervisor took its place.)
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

## 10. Interfaces for implementation (settled 2026-10-30)

Three builders implement these in parallel. A change to an interface goes through
the plan owner, not through a builder.

### 10.1 Assignment record: `scope_budget` version 2

The record stays a JSON object file, and `scope_budget.py` owns only its
`scope_budget` key. Version 2 sets `"schema": "manual-orchestration.scope-budget.v2"`
and keeps every version 1 field. It adds:

| Field | Shape | Written by |
|---|---|---|
| `objective_id` | UUID string | `init`, or the first version 2 write to a version 1 record |
| `sessions` | list of `{"session_id", "provider": "codex" or "claude", "from": ISO time, "bound_by": "init", "rotation", "supervisor" or "user", "evidence": string or null}` | `init` (the root), `add-session` |
| `bindings` | list of `{"session_id", "provider", "parent_session_id": string or null, "bound_at", "bound_by": "supervisor" or "user", "reason"}` | `bind` |
| `cost_limits` | list of `{"pool", "node": session ID or null, "max_usd": number or null, "marks_usd": ascending numbers, "set_by": "user" or "supervisor", "set_at", "reason"}` | `cost-limit`, which also appends to `limit_history` |

Pools are `codex`, `claude_all_models` and `claude_fable`.

New or changed commands:
- `init` accepts `--session ID --provider P` and records that session as the
  root. Without them, it still writes version 2, with an empty `sessions`
  list.
- `add-session --record R --session ID --provider P --from ISO --bound-by B
  [--evidence E]` appends a session. Adding an identical entry again succeeds
  and changes nothing. Adding a different entry for the same session is
  refused with exit 2.
- `bind --record R --session ID --provider P [--parent ID] --bound-by B
  --reason TEXT` appends a binding.
- `cost-limit --record R --pool POOL [--node ID] [--max-usd X] [--mark-usd Y
  ...] --set-by S --reason TEXT` replaces the entry for that pool, node and
  setter. A user entry and a supervisor entry for the same pool and node
  coexist, so a supervisor command never changes or removes a user limit
  (invariant 5). Added 2026-10-30 after review finding 1 on the record
  change.
- `--max-weekly-points` on `init` or `revise` is refused with exit 2, and the
  message names `cost-limit`.
- `check` treats hours, additions and `max_weekly_percent` as before. The
  meter's rise is reported as `account_weekly_points` with
  `"account_wide": true`. An existing `max_weekly_points` limit:
  - stops the objective only when the user set it;
  - otherwise appears under `advisory`.

  A limit is user-set when `limits_set_by` is `user`, or when a
  `limit_history` entry set by `user` has the same value for it.
- Every command reads version 1 records. The first version 2 write upgrades the
  schema string and adds the new fields, empty, plus an `objective_id`.
- `--set-by` and `--bound-by` take `user` or `supervisor`. Records written
  before the coordinator role was removed (2026-10-30) may say `coordinator` in
  `limits_set_by`, `limit_history`, `bound_by` or `set_by`. Read it as
  `supervisor`, which is not a user setting.

### 10.2 `usage_ledger.py tree`

```
python3 -I metrics/scripts/usage_ledger.py tree --record PATH [--since ISO] [--until ISO] [--role-host-ledger PATH] [--out FILE]
```

Defaults:
- `--since`: the record's `started_at`;
- `--until`: now;
- `--role-host-ledger`: `$XDG_STATE_HOME/work-engine/role-host/ledger.jsonl`,
  with `XDG_STATE_HOME` defaulting to `~/.local/state`.

It prints one JSON document and writes nothing to the record.

```json
{
 "schema": "objective-cost-tree-v1",
 "snapshot_at": "ISO", "window": {"since": "ISO", "until": "ISO"},
 "record": {"path": "...", "sha256": "...", "objective_id": "..."},
 "reader": {"path": "...", "sha256": "..."},
 "pricing": {"source": "...", "fetched": "2026-10-30"},
 "status": "complete | partial | not_established",
 "totals": {"<pool>": "Spend"},
 "window_totals": {"<pool>": "Spend"},
 "nodes": ["Node"],
 "unattributed": {"totals": {"<pool>": "Spend"}, "sessions": [{"session_id": "...", "provider": "...", "calls": 0, "usd_low": 0, "usd_high": 0}]},
 "unrecorded": [{"session_id": "...", "kind": "compaction", "at": "ISO", "context_tokens": 0, "usd_low": 0, "usd_high": 0}],
 "issues": [],
 "known_gaps": ["Claude Code title and other side calls are not in transcripts"]
}
```

**Shapes.**
- `Spend` is `{"calls", "unpriced_calls", "usd_low", "usd_high",
  "quota_usd_low", "quota_usd_high"}`.
- `Node` is `{"node_id", "session_id", "provider", "parent_node_id", "link",
  "role_name", "models", "tiers", "first_call", "last_call", "direct":
  {"<pool>": Spend}, "inclusive": {"<pool>": Spend}}`.
- `link` is one of `root`, `successor`, `codex_parent`, `role_host`,
  `claude_subagent` or `binding`.
- A Claude subagent's `node_id` is `<sessionId>/agent:<agentId>`, because a
  subagent shares its parent's `sessionId`. Other nodes use the session ID.

**Pools.**
- Every Codex call counts toward `codex`.
- Every Claude call counts toward `claude_all_models`.
- A Claude Fable call (`claude-fable-*`) also counts toward `claude_fable`.

**Quota-weighted dollars.** The call's `usd` times its tier multiplier: Codex
`priority` 2.5, `default` 1. An unknown Codex tier gives the range 1 to 2.5.
Claude calls use 1.

**Attribution.**
- Roots are the record's `sessions`. A root's calls count from its `from` time.
- Children are joined repeatedly, until nothing new joins, across four sources:
  - Codex `parent_thread_id`;
  - `role_host` launch rows' `parent_session_id`;
  - Claude `subagents/`;
  - the record's `bindings`.
- Each call belongs to its own node only. Inclusive totals add descendants
  once.
- A session reachable by two links keeps the first, in this order: root,
  successor, binding, codex_parent, role_host, claude_subagent. The other link
  is recorded as an issue `multiple_parents`.

**Conservation.** For each pool, the objective totals plus the unattributed
totals equal `window_totals`.

**Status.**
- `partial`: the tree has unpriced calls, cost ranges, unrecorded calls or
  ledger issues.
- `not_established`: the record has no `sessions`, or the ledger cannot be
  built.
- `complete`: otherwise.

### 10.3 Changes to `usage_ledger.py build`

**Prices.** These are added from Anthropic's pricing page
(`https://platform.claude.com/docs/en/about-claude/pricing`, fetched
2026-10-30). Existing rows are unchanged; that page confirms Opus 5.5 and
Sonnet 5.5 at the ledger's current rates.

| Model | Input | 5m cache write | 1h cache write | Cache read | Output |
|---|---|---|---|---|---|
| `claude-fable-5-1` | 10 | 12.50 | 20 | 0.25 | 50 |
| `claude-opus-5` | 5 | 6.25 | 10 | 0.50 | 25 |
| `claude-opus-4-8` | 5 | 6.25 | 10 | 0.50 | 25 |
| `claude-haiku-5-5`, prompt up to 100,000 tokens | 0.10 | 0.125 | 0.20 | 0.01 | 0.50 |
| `claude-haiku-5-5`, prompt over 100,000 tokens | 0.50 | 0.625 | 1 | 0.05 | 2.50 |

Prices are $ per million tokens. A Haiku 5.5 request's prompt length counts its
input, cache reads and cache writes.

**Fast mode.** A Claude call with `speed` `fast` is left unpriced, with an
issue, because the page gives no fast-mode cache prices.

**Inconsistent cache split.** When the TTL parts exceed
`cache_creation_input_tokens`, record the issue `inconsistent_cache_split`, and
price the call as a range:
- low: price the total as a cache write of unknown TTL;
- high: price the parts at their TTL rates.

**Pools.** Each call row gains `pools`, a list.

**Unrecorded calls.** Each Claude transcript `system` record with `subtype`
`compact_boundary` in the window is an unrecorded compaction call, written to a
new `unrecorded.jsonl`:
- `context_tokens` is `compactMetadata.preTokens`;
- the model is the session's latest assistant model before the boundary;
- `usd_low` prices `context_tokens` at that model's cache-read rate;
- `usd_high` prices `context_tokens` at the 1-hour cache-write rate, plus output
  of one token per 3 characters of the following `isCompactSummary` message.

**No stops.** No single record stops `build`. A record that cannot be priced
becomes an issue, plus an unpriced or ranged call.

### 10.4 `usage_ledger.py control`

```
python3 -I metrics/scripts/usage_ledger.py control --record PATH [--cursor CURSOR] [--limit-fractions 0.5,0.8,1.0] [--hours-fractions 0.8,1.0] [--role-host-ledger PATH]
```

It builds the tree, compares it with the record, and prints:

```json
{"schema": "objective-budget-control-v1", "topic": "control:budget:<objective_id>",
 "events": ["Event"], "next_budget_cursor": "<opaque>",
 "snapshot": {"snapshot_at": "ISO", "status": "...", "totals": {}}}
```

**Event shape.** `{"topic", "authority": "observation_only", "kind", "pool",
"node_id", "level", "observed", "snapshot_at", "summary"}`.

`kind` is one of:
- `mark_crossed`
- `limit_fraction`
- `limit_reached`
- `node_mark_crossed`
- `hours_fraction`
- `hours_reached`
- `coverage_changed`
- `account_ceiling`

**When events are sent.**
- An event is sent only when a trigger's state differs from the state in the
  cursor.
- With no cursor, only triggers already past a level are reported, once.
- Spend is compared as `quota_usd_high`, and events carry both bounds.
- Only a user entry's `max_usd` produces `limit_fraction` and
  `limit_reached`. A supervisor entry's `max_usd` is treated as one more
  mark. Every limit event carries `set_by`, and a trigger's key includes
  `set_by`. Added 2026-10-30 with the §10.1 setter rule.
- `account_ceiling` compares the latest Codex weekly meter reading in the
  window with a `max_weekly_percent` limit, and is labelled account-wide.
- `hours_*` events come from `max_hours` and `started_at`.

**Cursor.**
- The cursor is bound to `objective_id`, and a cursor for another objective is
  refused with exit 2.
- A trigger's key includes its level, so a changed limit or mark is a new
  trigger.

**Exit codes.** Exit 0 whenever a document is printed, including `status`
`not_established`. Exit 2 only for usage errors.

### 10.5 `chatboard wait` budget source

```
codex_chatboard.py wait ... --budget-record PATH --budget-reader PATH --budget-sha256 HEX [--budget-cursor C] [--budget-interval 60] [--budget-limit-fractions F] [--budget-hours-fractions F]
```

**The adapter.** `BudgetControl` mirrors `ContextControl`:
- It checks the reader's SHA-256 before every run, and a mismatch stops the
  listener with an error.
- It runs `python3 -I <reader> control --record ... [--cursor ...]` with a
  120-second timeout.
- It validates the schema, the topic and `authority` on every event, and keeps
  the cursor.

**Timing.** It runs at most once per interval: `wait` wakes at the next interval
even with no board activity, runs the reader, and returns if there are events.

**Results.**
- Results add `budget_events`, `budget_topic` and `next_budget_cursor`.
- `status` is `budget` when only budget events arrived.
- `messages`, `next_since`, `control_events` and `next_control_cursor` keep
  their meaning.
- With `--follow`, the budget cursor carries across iterations as the control
  cursor does, and the terminal result keeps it.

**Reader failure.** A reader that exits non-zero, times out or returns invalid
output produces one local event `{"kind": "reader_failed", "authority":
"observation_only", ...}` and keeps the last cursor. The next good run produces
`reader_recovered`. Board delivery must continue, so this failure doesn't stop
the listener; a digest mismatch does.

**Topics.** Topics starting `control:budget:` are never delivered as board rows
and cannot be posted, like `control:session:`.

**Session ID.** The budget source alone needs no `--session-id`.

### 10.6 Root rotation

`root_rotation.py` accepts an optional `--record PATH`. After it verifies the
successor's adoption, it runs `scope_budget.py add-session` with:
- the successor's session ID and host;
- `--from` the adoption time;
- `--bound-by rotation`;
- the adoption message as evidence.

If that fails, the transfer report records `cost_binding: not_established`, and
the rotation's own outcome is unchanged.
