# Study plan: what owner involvement consists of, and how much engineering can remove (2026-10-31, draft v1)

Status: **draft plan, not a pre-registration.** Each experiment below gets its
own pre-registration in `metrics/probes/` before it launches. Nothing here
authorizes spend. The rules of
[experiment-plan-20261026.md §2](experiment-plan-20261026.md#2-rules-for-every-experiment)
apply to every experiment here: one factor per controlled comparison,
pre-registration, sealed keys, complete non-overlapping cost, failed runs kept,
result classes, and n stated with every result.

Origin: a conversation between the owner and ChatGPT on 2026-10-31. The owner's
claim was that scope drift and ballooning are a main reason for
human-in-the-loop (HITL) supervision, and that if the reasons for HITL were
decomposed, much of it could be solved in engineering. The motivating case is the
S10 root `01a18305` (`gpt-6.1-sol` xhigh) on 2026-10-28/29. The root ran two
turns, 22:00:06Z–02:21:44Z and 02:51:23Z–15:37:54Z (12.78 h); this plan
recomputed both from the analysis ledger's `turns.jsonl`. The 2026-10-29 analysis
found the scope growing from the 21:59Z plan and no real-host run after 21:53Z;
those two findings were not re-checked for this plan
([analysis](#s10-reference)).

## 1. Question

> Of the owner attention that agent work consumes, how much is needed only
> because a buildable mechanism is missing; how much is a decision the owner
> could delegate in advance; how much needs the owner at the time; and which
> mechanisms remove the most attention for the least residual risk?

The doctrine already names the classes this study measures against.
[DESIGN.md §11](../DESIGN.md#11-operator-authority-and-approval) separates
decisions the model may make silently, consequential decisions it may make but
should explain, and decisions that need new operator authority, preference,
ownership choice or approval. §13 lists operator attention as a consequence to
measure. The study asks where actual owner involvement falls against those
classes, and why.

| Class (this study) | Meaning | DESIGN §11 relation | Target |
|---|---|---|---|
| **Compensatory** | The owner supplied something a mechanism could have supplied: a check, a durable record, an alert, a capability, a transport | The decision was already in the silent or explain class, or the owner caught a system failure | Remove by engineering |
| **Delegable** | The decision needs owner authority, but the owner could grant it in advance as a standing policy | Owner-authority class today | Owner decides whether to delegate |
| **Reserved** | The decision needs the owner at the time: new intent, preference, values, conflicting objectives, accountability | Owner-authority class | Keep, and make it cheap |
| **Absent** | The owner did not intervene, and work continued at a cost an intervention would have stopped | None: no decision was surfaced | Detect by engineering |

Two consequences of the doctrine shape the design.

- **Moving a decision out of the owner-authority class is a contract change**
  ([DESIGN.md §1.1](../DESIGN.md#11-contracts-and-invariants)). The study can
  identify delegable decisions and replay a proposed policy against history. It
  cannot infer a delegation; each one is an owner decision.
- **Fewer interventions is not the objective.** A system that never asks and
  drifts would score perfectly on an intervention count. An intervention count
  cannot see the S10 case either: its cost accrued inside a single 12.78 h turn.
  Every reduction is therefore paired with a bad-outcome measure (§5).

## 2. Unit of analysis

**Owner input event.** One human-originated input to agent work:

- a human-typed message in a Claude Code session or a Codex root thread;
- an interrupt (Esc, `[Request interrupted by user]`, a Codex `turn_aborted`), an
  answer to an agent's question, or a tool approval or rejection;
- a board post by the owner, or an owner decision recorded in a plan, a scope
  record, a pause record or a review override;
- text the owner pasted from another model or person (the message that started
  this study is one). These are kept as their own class, **relay**, because the
  owner is acting as the transport between agents.

Messages a parent agent sends to a child thread are not owner events, even when
the transcript records them as `user`. Separating the two is the extractor's
hardest job (§4, Phase 0).

**Intervention episode.** The owner events on one session tree that concern one
decision, plus the agent turn that preceded them and the agent turns that
followed until the decision took effect.

**Actors.** The owner; an owner-delegated agent acting on owner authority (the
Dot, as budget delegate, since 2026-10-30); and agents. Delegate actions are
coded separately. On 2026-10-30 the Dot stopped S10, and the owner later called
it a false stop. That is a delegated-supervision error, not an owner
intervention.

## 3. Codebook (v0, to be revised once after the pilot and then frozen)

The codebook is an analysis instrument. It must not become a routing table for
agents ("if drift, do X"); [PHILOSOPHY.md §16](../PHILOSOPHY.md#16-why-exhaustive-rules-are-dangerous)
explains why. Every dimension has `other` (with free text) and `not established`.

| Dim | Name | Values | Coded by |
|---|---|---|---|
| D1 | Trigger | agent asked (question, request, escalation); owner unprompted; alert (session-events, budget listener, Dot); scheduled check-in | deterministic where the record shows it, else coder |
| D2 | Content | objective or intent; authorization (go, commit, deploy, push, spend); correction; redirect or scope (stop, narrow, expand, reprioritize); information only the owner had; preference; relay; recovery action the agent could not take; status question; acknowledgement | coder |
| D3 | Decision class at the time | silent; explain; owner authority (DESIGN §11), judged against the authority in force at that time | coder; owner is the reference for owner authority |
| D4 | What would have made the event unnecessary | see below | coder, from the pre-event view |
| D5 | Recurrence | first statement; repeats an earlier owner statement; repeats a recorded rule (memory, AGENTS.md, CLAUDE.md, skill guidance); with a link to the earliest instance | deterministic candidate search, coder confirms |
| D6 | Effect and correctness | changed course / confirmed course / no effect; correctness class 1, 2 or 3 as in E3 | coder, with outcome visible |
| D7 | Cost | owner words typed; words pasted; agent wall-clock blocked awaiting the owner; spend since the last progress event before the intervention; spend the intervention caused | deterministic |

**D4 values: the missing element.**

| Value | The event would have been unnecessary if… | Class |
|---|---|---|
| `durable_record` | information the owner had already given reached the agent (context continuity, memory, doctrine) | compensatory |
| `check` | a deterministic check had enforced an invariant or limit (gate, scope ledger, budget, stall detection) | compensatory |
| `verification` | independent verification had caught the error before the owner did | compensatory |
| `capability` | the agent had the action it lacked (permission, tool, interactive authentication) | compensatory, or reserved when the capability is withheld on purpose |
| `transport` | agents could exchange the content without the owner relaying it | compensatory |
| `observability` | the owner could see status without asking | compensatory |
| `standing_policy` | the owner had granted this class of decision in advance | delegable |
| `new_information` | nothing: the owner supplied intent, preference or a fact for the first time | reserved at first occurrence; a recurrence is `durable_record` |
| `owner_judgment` | nothing: values, conflicting objectives, accountability, or an authority the owner keeps | reserved |

D4 is coded **from the pre-event view**: the transcript up to and including the
owner event, without what happened afterwards. Coding with the outcome visible
invites hindsight. D6 is coded in a second pass with the outcome visible.

D5 separates two failures. A first-time preference is new information. The same
correction repeated after it was recorded means the record exists and did not
take effect, which needs a different fix from writing another rule.

## 4. Phases

Each stage in §9 ends with an owner review before the next is released, as in
the experiment plan's §3 staging.

### Phase 0. Corpus, extractor and descriptive baseline (deterministic, no model coding)

- **Freeze the corpus** to a cutoff, with a manifest and sha256 per file (sources
  in §7).
- **Build the extractor.** It emits owner input events with their session tree,
  the preceding agent turn, and D1 and D7 where these are deterministic.
  Session-tree attribution reuses the usage ledger's lineage. Owner-origin rules
  per host (§7.1):
  - **Claude:** `origin.kind:"human"` and mid-turn `queued_command` records.
    Excluded: successor prompts, SDK sessions, subagent files. Interrupts,
    `AskUserQuestion` answers and tool rejections are included as their own event
    types. Pasted content is tagged relay.
  - **Codex:** a root-thread user message counts only when it matches a
    composer-history entry for the same session. Everything else in a root is
    `agent_or_unknown`, never owner. Interrupts and question replies are included.
  - **Before 2026-09-27 for Claude:** prompts survive in `history.jsonl` without
    context. They count toward R0's event counts, and are excluded from coding.
- **Validate the extractor** before anything is coded. A checker who did not build
  it hand-labels a stratified sample of candidate events and non-events. Proposed
  gate: precision at least 0.95 for "human-originated", and no parent-to-child
  message admitted in the sample. The gate is fixed in the pre-registration.
  - Two further checks use records the extractor does not read.
  - **Recall against structured decisions.** Each owner decision in §7.2 (for
    example the 10 completion-offer decisions with human authority, the pilot
    pause, the review override) should match an extracted owner event near its
    timestamp. Misses are counted and read.
  - **Codex marker coverage.** Composer-history coverage is measured on Codex
    sessions the owner is known to have driven, such as roots whose first message
    the owner typed. If coverage is low, Codex owner counts are reported as lower
    bounds.
- **R0, the descriptive baseline,** needs no model:
  - owner events, typed and pasted words, and blocked wall-clock, per session tree
    and per accepted outcome. Denominators come from run and assignment records
    where these name an acceptance. Assignment records have no fixed schema
    (§7.2), so R0 reports how many trees have a readable denominator;
  - attention episodes: owner events clustered by a gap threshold, counted as
    context switches;
  - each measure by host, role, period and trigger.

  R0 alone answers how much attention goes where, without any judgment about why.

### Phase 1. Codebook pilot and coding

- **Pilot.** 40 episodes, stratified by host, role, period and D1. Two independent
  coders from different model families code D2 to D6.
  - Neither coder is a session that appears as an actor in the episode.
  - Neither is this session (§6).
- **Owner calibration.** The owner codes 20 pilot episodes blind to the coders.
  - Whether a decision is reserved is the owner's call, so the owner is the
    reference for D3's owner-authority value and D4's `owner_judgment`.
  - Owner recall is itself retrospective, so it is recorded as one rating, not as
    ground truth for the other dimensions.
- **Agreement.** Cohen's κ per dimension.
  - Proposed gate to proceed: κ of at least 0.6 on D2 and D4.
  - Below that, revise the codebook once and re-pilot on fresh episodes.
  - Disagreements on coded episodes go to a third coder; disagreements about
    whether a decision is reserved go to the owner.
- **Main coding.** A stratified sample, sized from pilot cost per episode.

### Phase 2. Reducibility estimate and mechanism inventory

- Group coded episodes by D4 value into candidate mechanisms.
- Report attention in each group as a range, never a point:
  - lower bound: episodes where both coders agree;
  - upper bound: episodes where either coder assigns the value.
- For each candidate mechanism, record:
  - the existing infrastructure that covers part of it (§8);
  - what is missing;
  - its kind: deterministic machinery, durable record, standing policy, capability,
    transport, or guidance text;
  - the result of [DESIGN.md §3.8](../DESIGN.md#38-command-review-test)'s command
    review test, if it would add a command.
- **Guidance text is flagged.** "Infrastructure failures fixed by adding cognitive
  procedure to the model" is a DESIGN §14 anti-pattern. The manual-orchestration
  `references/` directory was created on 2026-10-21. It held 35,070 bytes in 8
  files at 2026-10-22T00:00Z and holds 101,515 bytes in 11 files at this plan's
  base (both from `git ls-tree`).
- **Retrospective on past fixes.** For corrections that were already answered by
  a change (memory entries, CLAUDE.md rules, skill releases such as `471f6106`),
  measure recurrence before and after the change. This is a strategy comparison
  (several things changed at once), not a single-cause result.

### Phase 3. Absent interventions: drift and stagnation

The S10 class. Interventions that did not happen leave no owner event, so they
are found from outcomes instead.

- **Progress events.** Define them per objective from records: an integrated
  commit, an accepted review verdict, a gate pass, a closed accepted assignment, a
  required real-host run, an objective closure.
  - If an objective's progress events cannot be read from its records, a
    stagnation detector cannot exist for it. That is itself a finding about the
    progress contract.
- **Unattended waste.** Find session trees with spend or wall-clock since the last
  progress event above a threshold, and no owner event in that window.
- **Code each case** as drift (distance from the objective's original items, from
  `scope_budget` records where they exist), stagnation (repeated cycles without
  progress), or legitimate long work. Also code whether a standing owner
  condition contributed: a gate or sequence the owner set earlier that kept a
  cheaper route closed (§8).
- **Replay candidate detectors** deterministically over the recorded history:
  - stall alert;
  - scope additions against original items, cumulative rather than per step;
  - spend since progress;
  - wall-clock since progress.

  For each detector, report how many hours before the eventual stop or owner
  action it would have fired, and its false-alarm rate on trees that were making
  progress. This replay needs no model calls.

### Phase 4. Decision replay for owner decisions

This phase reuses E3's fixture format, correctness classes and scorer
conventions
([E3](experiment-plan-20261026.md#e3-supervisor-decision-replay),
[pilot record](probes/e3-pilot-fixtures-20261027.md)).

- **Fixtures** are owner decision points whose episodes Phase 1 coded as
  compensatory or delegable. Each fixture holds what the agent had at that point.
  - In the mechanism arm, it also holds what the candidate mechanism would have
    supplied: the recorded preference, the scope ledger state, the progress state,
    or the proposed standing policy.
- **Key.** The owner's actual decision, with its correctness class. A session
  that runs no arm builds and seals it.
- **Arms:**
  - A0, the agent alone at the decision point;
  - A1, the agent plus the mechanism's output;
  - B, scored deterministically: "accept the agent's own recommendation".
- **Confirmatory baseline.** B is E3's follow-the-reviewer weakness turned into
  a measure. If the owner's decision equals the agent's recommendation in most
  fixtures, those escalations were confirmatory.
  - Confirmatory does not mean unnecessary. Knowing it will be checked may change
    what the agent proposes, which the replay cannot see. That is a limitation,
    stated with the result.
- **Critical unsafe decisions are scored separately,** whatever the overall
  agreement:
  - false autonomy: the arm proceeds where the owner stopped or redirected, in a
    class 1 or 2 fixture;
  - crossing an authority boundary the owner keeps.

### Phase 5. Shadow-mode prospective trial

- **Shadow run.** A candidate mechanism runs beside live work. It records what it
  would have alerted, decided or escalated, and takes no action. The owner works
  as normal.
- **Pairing.** Each shadow output is matched with the owner's actual behaviour:
  - a shadow alarm with no owner action is a false alarm or an absent
    intervention, settled at the episode's outcome;
  - an owner action with no shadow alarm is a miss.
- **Sealing.** The owner does not see shadow outputs until the window closes, so
  they cannot change owner behaviour.
- **Activation.** Letting the mechanism act is a separate owner decision. If it
  moves an authority boundary, it is also a contract change.

## 5. Metrics

**Primary, reported together and never summed into one score:**

- **Owner attention per accepted outcome:**
  - owner events;
  - attention episodes;
  - words typed;
  - words relayed;
  - agent wall-clock blocked awaiting the owner.
- **Bad outcomes per accepted outcome:**
  - defects found after acceptance;
  - critical unsafe autonomous decisions (Phases 4–5);
  - unattended waste in dollars and hours (Phase 3).

**Secondary:**

- Share of owner events by D3 class and by D4 value, with the coder-disagreement
  interval.
- Recurrence rate: owner corrections that repeat a recorded rule.
- Confirmatory escalation rate (Phase 4 baseline B).
- Detector lead time and false-alarm rate (Phase 3).
- Delegate error rate: Dot actions later reversed by the owner.

**Human minutes are not observable** from these records. The proxies above
stand in for them, and the study says so wherever it uses them. An owner-kept
diary during Phase 5 would calibrate them; it is the owner's choice (§10).

## 6. Threats and limits

| Threat | Handling |
|---|---|
| One owner (n = 1). Findings describe this owner's practice | State it with every generalisation; no claim beyond this repository |
| Hindsight in coding | D4 coded from the pre-event view; D6 coded separately |
| Coders are models judging model work | Two model families; owner as reference for reserved decisions; agreement reported |
| Designer conflict: this session designed the study and is an actor in many transcripts | This session neither codes nor scores; it may verify arithmetic and denominators, as in the experiment plan's rule 3 |
| Asymmetric retention: Claude transcripts before about 2026-09-27 were deleted; Codex rollouts go back further | Report coverage per host; compare hosts only over the window both cover |
| Confounds over time: model routing (Astra to Sol 6.1), guidance releases, owner availability | Before/after comparisons are strategy comparisons |
| Owner interventions can themselves be wrong | D6 records correctness; a wrong intervention is a finding, not a gold label |
| Absent interventions are undercounted where records do not define progress | Report coverage: objectives with readable progress events against all objectives |
| Goodhart: "fewer interventions" rewards not asking | Primary metrics pair attention with bad outcomes |
| Observer effect in Phase 5 | Shadow outputs sealed until the window closes |
| Owner text in the corpus is private | The corpus stays local; fixtures are scrubbed as in E3 |

## 7. Data sources

Inventoried 2026-10-31 by read-only sweeps. Counts are as reported by the sweep,
with the command named in its report; they were not re-run for this plan. The
main finding: **no store records owner interventions as typed data.** The
transcripts are the primary corpus. The structured stores below are small, and
serve as anchors and validation sets for the extractor.

### 7.1 Transcripts (primary)

**Claude Code** (`~/.claude/projects/-home-bline-code-work-engine/`). Human
input is labelled, with known exceptions.

- **Files and coverage:** 86 session files and 161 subagent files. First-record
  timestamps run from 2026-09-27 to 2026-11-01; anything earlier was deleted by
  the 30-day cleanup.
  - The archive in `~/cbm-work/claude-transcripts-archive/` holds no extra file
    for this project today.
  - `~/.claude/history.jsonl` keeps 1,339 prompts from 2026-09-04, of which 335
    predate the surviving transcripts. Those 335 have text but no surrounding
    context.
- **Human input:**
  - 877 records of `type:user` with `origin.kind:"human"`. Three of them are
    successor prompts, so 874 are owner messages (recounted for this plan);
  - 75 sent mid-turn: an attachment of type `queued_command` with `humanTurn:true`;
  - 7 interrupt markers, one of them in an SDK session.
- **Decisions on agent requests:**
  - 41 `permissionDecision.source:"user_temporary"` records: 40 are answers to
    `AskUserQuestion` and 1 is a Bash approval;
  - 2 user rejections of a tool call (the other 25 records with that marker are
    configuration-driven, in SDK sessions).
- **Traps the extractor must handle:**
  - successor prompts from `metrics/scripts/launch_claude_successor.sh` carry
    `origin.kind:"human"` (3 found);
  - SDK sessions (`entrypoint:"sdk-cli"`) are scripted;
  - between 237 and 255 owner messages carry `<pasted_content>`, depending on the
    matching rule (the sweep's rule, or any occurrence in this plan's recount).
    The owner chose to send them, but another model often wrote the text. These
    are the **relay** class, and the extractor's pre-registration fixes the rule;
  - subagent files hold no human records. Parent input there is
    `origin.kind:"coordinator"`.

**Codex** (`~/.codex/`). Human input is **not** labelled.

- **Files and coverage:**
  - 418 live rollouts (2026-10-15 to 2026-11-01; 335 in this repository);
  - 349 archived rollouts (2026-08-26 to 2026-10-25, by filename);
  - `thread_history_1.sqlite` with 10,893 `userMessage` items from 2025-12-29,
    including threads whose rollouts are gone. Whether it is complete for those
    threads is not established.
- **Root does not mean human.** A root thread that an agent started through the
  app-server daemon carries `provenance.type:"user"` like a human one. One such
  thread opens "You are the ONE fresh independent Astra … reviewer".
  - Child threads are identifiable by `thread_source:"subagent"` and by spawn
    edges in `state_5.sqlite`.
  - A child's instructions from its parent are encrypted in the rollout.
- **The usable human marker is the composer history** `~/.codex/history.jsonl`:
  10,561 prompts with session id, from 2025-12-29 to 2026-10-30.
  - 412 of 853 root-thread user messages match it.
  - None of the 61 messages in `codex_exec` threads match it.
  - Whether it captures every human message (for example from Codex Desktop) is
    not established.
- **Interrupts:** 18 `turn_aborted` events with `reason:"interrupted"`.
- **Owner answers to agent questions:** 9 records tagged
  `send_user_message_question_reply`.

**Spend and lineage.** `metrics/scripts/usage_ledger.py` attributes calls to
session trees (`root_thread`, `parent_thread`, depth) and to `scope_budget`
objectives (`tree`), with per-call latency and Codex turn duration.
`metrics/scripts/run_registry.py` maps run directories to threads, sessions and
outcome fields (v7: 41 runs). Neither marks human turns.

**Existing classifiers.** The scripts in `~/cbm-work/s10-session-analysis-20261029/`
and `~/cbm-work/skill-change-analysis-20261030/scripts/` classify tool calls by
regex (for example `call_purpose.py`). None looks at human messages.

**Session-events** started on 2026-10-30, and its log holds 31 deliveries. It is
too recent to be a corpus. It is the natural host for Phase 5's shadow
detectors.

### 7.2 Structured records (anchors and validation)

| Source | What it holds | Owner signal | Coverage, count |
|---|---|---|---|
| Coordination board, ref `refs/work-engine/codex-chatboard/…` (payload base64 in a git blob; `read --since 0 --limit 500` pages) | 3,984 messages, sequences 1–3984 with no gaps | No owner kind or role. 3,907 messages (98%) have no kind. 2 have author `user:bline` (seq 3009, 3071, both relayed). Owner decisions appear as free text ("Human accepted…", "user authorized…") | 2026-08-30 to 2026-10-31 |
| Completion offers, refs `refs/work-engine/completion-offers/<run>/slice-N` | 33 commit offers | 10 carry `decision{create\|decline, authority{kind:"human", observed_at, reference}}`: 9 create, 1 decline | 2026-09-05 to 2026-10-08 |
| Slice-supervisor receipts (`docs/ai-workflow-metrics.jsonl`, 32 rows; `metrics/review-remediation.jsonl`, 9 rows) | Campaign receipts | `plan_acceptance` = `human_approval` 5 times; `stop_on` may include `human_judgment` | 2026-08-20 to 2026-10-08 |
| Proposal decisions, `proposals/**/decision.json` (12) | Disposition and authority | `authority.exercised_by:"user"` with the quoted statement | 2026-08-22 to 2026-10-10 |
| `scope_budget` inside assignment records (202 files with the object, 167 distinct objectives; some are `.pre-*` copies) | Objective, original items, limits, cost limits, additions | `limits_set_by` and `cost_limits[].set_by` (user or supervisor). **Additions have no approved-by field;** approval shows only in the free-text reason. Only 10 additions in 9 records. One close | started_at 2026-10-29 to 2026-11-01 only |
| Budget listener logs (`budget-listener*.jsonl`, 14 files) | 28 budget events (limit_fraction, mark_crossed, limit_reached, …) | Alerts, not decisions | 2026-10-30, one objective tree |
| Task-profile round 2 (`~/.local/state/work-engine/task-profile-pilot-2/`) | 1 pause with `owner_approval`; 1 owner override of a review verdict (`r2-owner-decisions-20261031/`); 17 closes; 150 screening events | `escalations` in close records counts **model** escalations, not escalations to the owner. 36 of 75 screening reasons cite owner authority | 2026-10-28 to 2026-10-31 |
| Owner-decision tables in plans (e.g. [experiment plan §9](experiment-plan-20261026.md#9-owner-decisions)) | Hand-written rows with quoted owner words | Free text | Per plan |
| Root rotation `authorized-by` files | Named approver token | `author=user:bline`; one file found | 2026-10-25 onward |
| Role-host ledger (`role-host/ledger.jsonl`, 332 events) | Deliveries, launches, retirements | No actor field, so owner deliveries cannot be told from agent deliveries (not established) | 2026-10-25 to 2026-10-31 |
| Session-events log | 31 `session.turn.ended` deliveries to the Dot | Alert triggers, not decisions | 2026-10-31 only |

Assignment records (281 `assignment.json` parsed) have no fixed schema outside
`scope_budget` and the task-profile recorder's, so reviewer rejections,
re-dispatches and owner interventions there are in ad-hoc keys or free text.
`metrics/efficiency-ledger.md` (M1–M22) has no human-involvement metric.

**Consequences for the design.**

- Phase 0's extractor works on transcripts. The structured records above are a
  **labelled validation set**: each structured owner decision should match an
  owner event the extractor finds in the transcript at that time. A structured
  decision with no matching transcript event is either an extractor miss or an
  owner action outside any session, and both are counted.
- Scope records begin 2026-10-29. Before that, Phase 3's scope distance is coded
  from the root's own plan messages and board posts, not from `scope_budget`.
- The recording gaps are themselves a finding for Phase 2. Additions carry no
  approved-by field, there is no stop, pause or redirect event type (apart from
  the pilot pause), and the board has no owner role. An owner intervention that
  is not recorded as one cannot feed a mechanism that learns from it.

## 8. Existing mechanisms against the proposed taxonomy

Mapped 2026-10-31 by a read-only sweep of DESIGN.md, PHILOSOPHY.md, `design/`,
`docs/`, `skills/` and Rust crate docs. Spot-checked for this plan:
`supervisor.md` "Scope and budget", the session-events `stalled` definition,
`material-decision-selection.md`'s reserved and delegated decisions, and the S10
amendment draft. **Most of the proposed taxonomy is already designed, and much of
it exists in the outer level.** The study is therefore not about inventing these
mechanisms. It measures whether the existing ones carry the load, and where the
remaining attention goes.

Status: **D** designed only; **O** implemented in the outer level (skills,
scripts, metrics); **R** in Rust; **gap** where noted, with the search domain.

| Proposed mechanism | What exists | Status |
|---|---|---|
| Three kinds of human involvement | DESIGN §11 (silent / explain / new operator authority). `material-decision-selection.md`: a reserved decision needs the named owner's explicit disposition; a delegated decision needs an explicit delegation of the materiality class, with bounds. "Absence of a reserved-decision marker is not delegation." | Doctrine; D |
| Delegated authority by policy | Slice-supervisor `approval.plan: procedural_when_safe \| human_required`; standing authorization in `experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md` ("no root approval for each conforming run"); `RoutingPolicy` and delegation modes in the architecture | O (config, plan text); D |
| Scope envelope | Packet carries "exact accepted authority, … outcome, checks, limits and escalation owner" (`supervisor.md`). `scope_budget` holds original items and limits. The full compiled contract with non-goals and a discretion envelope is in `implementation-contract-compilation.md` | O, partial; D |
| Scope change the agent cannot self-authorize | Slice-builder returns a boundary-change request; the worker reports a requirement gap before expanding scope; a large cumulative change "goes to the user before more dispatch" | O. The root supervisor classifies its own small changes, so at the root it can partly self-authorize |
| Cumulative drift detection | "Judge scope by the cumulative change from the original items, not by whether the newest addition looks small" (`supervisor.md`, citing S10). `scope_budget.py check` exits 3 when the additions trigger is reached | O, as a **count** of recorded additions plus model judgment. **Gap:** no measure of semantic distance from the original assignment (searched "distance\|cumulative\|add up to" over DESIGN.md, PHILOSOPHY.md, `design/architecture`, skills, `docs/`) |
| Autonomous containment | Slice-builder `deferred_scope` carried into the audit continuation; "continue toward the original objective unless authority or an invariant requires operator intervention" (DESIGN §10) | O for slice campaigns. No scripted deferral record in manual-orchestration (not established) |
| Progress contract: milestones | `IMPLEMENTATION_PLAN.md` milestones M0–M6 with exits; slice-supervisor's state machine; ordered increments in the compiled contract | O / D |
| Stagnation threshold | session-events `stalled`: "neither its transcript nor any worker's has changed for `stall_minutes`"; repair non-convergence (`stop_on: validation_nonconvergence`, two failed repairs of one review category); hours and cost limits (`check` exit 4, ledger `hours_reached`) | O. **Gap:** `stalled` measures liveness, not progress. A busy agent that makes no progress never triggers it. No progress-based detector in Rust either (searched stall, stagnat, milestone, progress over crate docs and `.rs`) |
| Progress judged independently of the agent | "Completion wakes collection; it does not certify success"; verify the subject the worker exercised | O for results. **Gap:** independent measurement of *progress* during work is not established. The S10 gap count itself came "from the root's progress messages, not audited" |
| Recovery and termination | Truthful `stopped`/`failed` receipts; at a limit "dispatch nothing new, collect…, report… the decision needed" | O |
| Compensatory supervision removed | `auto_continue.py`; "must not depend on another user message or a new approval for already authorized work" (`records/outer-context-automatic-resumption-requirement-20261028.md`); "a gate that waits on a human costs a whole reset cycle. Pre-stage both authorisations" (`notes/post-s10-direction-20261019.md`) | O |

**No reason-by-reason classification of owner interventions exists.** The
search covered `notes/`, `metrics/probes/`, `records/`, `planning/` and
`reviews/` for "intervention\|intervene\|human intervention\|user had to".
Two pending ideas name fewer supervisor interventions as a target metric
(`design/ideas/pending/proposal-decision-gated-implementation-compilation.md`
and the structural plan IR idea). The dogfood `escalations` field counts model
escalations, not escalations to the owner.

**What this changes in the study.**

- Phase 2 maps each D4 value to the row above that should already have covered
  it. "A mechanism exists, and the intervention happened anyway" is a different
  finding from "no mechanism exists". The first means the mechanism is unused,
  not enforced, not bound, or wrong. For example, the 2026-10-30 analysis found
  that RP3 and B3 did not bind the budget listener between 04:07Z and 13:34Z
  that day, while S10 did.
- Phase 3's detectors target the three gaps: semantic drift distance, progress
  rather than liveness, and progress observed independently of the agent's own
  messages.
- **Owner gates are a cause, as well as a cure.** The S10 amendment draft
  (`planning/s10-diagnostic-runs-amendment-proposal-20261029.md`, uncommitted)
  records that the condition keeping real-host runs out of the loop,
  "realstartup root-only after review", followed the sequence the owner approved
  on 2026-10-28. Phase 3 therefore codes, for each unattended-waste case, whether
  a standing owner condition contributed, alongside agent-side drift.

## 9. Staging and cost

- **Stage 1: Phase 0 and R0.**
  - Deterministic. The cost is building and validating the extractor.
  - The owner reviews R0 before any coding.
- **Stage 2: Phase 1 pilot.**
  - Cost is roughly episodes × bounded pre-event context × 2 coders.
  - The pre-registration fixes a context cap per episode.
- **Stage 3: main coding, Phase 2 and Phase 3.**
  - Phase 3's detector replay is deterministic.
- **Stage 4: Phase 4 replay.**
  - E3's pilot fixtures cost $5.70 to build, and E3's arm budget is about
    $10–20.
- **Stage 5: Phase 5 shadow trial,** on owner go-ahead.

The study runs under its own `scope_budget` objective, with original items,
limits and a close. It is subject to the drift it measures.

## 10. Owner decisions

| Decision | Proposed default |
|---|---|
| Corpus | This repository's Claude and Codex sessions, board, scope and budget records, pause and decision records, up to a fixed cutoff |
| Relay (pasted ChatGPT or other text) | Counted as owner events, class relay |
| The Dot | A delegate actor, coded separately from the owner |
| Owner calibration coding | 20 pilot episodes |
| Coders | Two families from the cheapest adequate models; model choice per [docs/model-selection.md](../docs/model-selection.md) |
| Budget | A cap per stage, set at each stage's release |
| Shadow-mode target | Chosen after Phase 3 ranks the detectors |
| Owner diary in Phase 5 | Optional |

## 11. Predictions, registered before any data is read

These predictions are recorded so they can fail. None of them has been checked.
The §7 inventory counted records, for example 874 owner messages in Claude
transcripts. No message content was read or classified before these were
written.

| # | Prediction | Falsified if |
|---|---|---|
| P1 | Authorizations and relays together outnumber corrections among owner events in root sessions | R0 plus Phase 1 coding show corrections ≥ authorizations + relays |
| P2 | Among corrections, a material share repeats a recorded rule (D5) | Under 10% of corrections are recurrences |
| P3 | Most escalations are confirmatory (Phase 4, baseline B) | The owner's decision differs from the agent's recommendation in at least half of the fixtures that have a recommendation |
| P4 | Unattended waste (Phase 3) costs more agent spend than all attended episodes' pre-intervention waste combined | It costs less |
| P5 | A cumulative scope detector would have fired on S10 at least 6 hours before the root's last turn ended | It fires later, or does not fire, on S10's recorded history |

## References

- <a id="s10-reference"></a>S10 overnight session analysis, 2026-10-29: data in
  `~/cbm-work/s10-session-analysis-20261029`; written up in
  `planning/s10-diagnostic-runs-amendment-proposal-20261029.md` (uncommitted
  draft) and in the S10 paragraph of
  [`supervisor.md` "Scope and budget"](../skills/manual-orchestration/references/supervisor.md#scope-and-budget).
  The resulting changes landed in commit `471f6106` (custody, `scope_budget.py`,
  scope-and-budget guidance, since moved to `supervisor.md`).
- [Experiment plan 2026-10-26](experiment-plan-20261026.md): rules, E3, spending
  conventions.
- [E3 pilot fixtures](probes/e3-pilot-fixtures-20261027.md).
- [DESIGN.md](../DESIGN.md) §1.1, §3.8, §11, §13, §14; [PHILOSOPHY.md](../PHILOSOPHY.md) §13, §16.
