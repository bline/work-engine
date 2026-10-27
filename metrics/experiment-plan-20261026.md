# Experiment plan: where inference goes and what it buys (2026-10-26, v2)

Status: **draft plan v2, not a pre-registration.** v2 incorporates an outside
review (ChatGPT, 2026-10-26; disposition in §10). Each experiment gets its own
pre-registration in `metrics/probes/` before it launches. Nothing here authorizes
spend.

Quota on 2026-10-26, as reported by the owner:
- Codex weekly at 95–99% used.
- Claude weekly 7% used.
- Fable weekly 0% used.

Experiments are therefore split into an Anthropic-only phase that can run now and
OpenAI arms that wait for the next Codex reset.

**Objective.** Find the least expensive execution policy that maintains
independently demonstrated correctness. This is not a model ranking. Two economic
measures are kept apart throughout:

| Measure | Answers |
|---|---|
| Cost per first artifact | Builder capability and efficiency (E1) |
| Cost per accepted, validated outcome, failures included | End-to-end Work Engine economics (E5) |

## 1. Why: what the survey found

Measured 2026-10-26 from Codex rollouts (`~/.codex/sessions`, plain `.jsonl`
modified on or after 10-19; 0 compressed files fall in that range) and Claude
transcripts. The scripts were session scratch and are rebuilt as F1.

- **Codex quota by role, current window (resets 11-01 23:25),** modelled with the
  per-model fit below: root ~32%, supervisors ~26%, reviewers ~25%, builders ~9%,
  other ~7%.
  - Root, supervisor and reviewer threads run on gpt-6-astra; builders mostly on
    gpt-6-sol.
  - Roles come from name patterns in `agent_path` (heuristic, ~10% unclassified).
- **Per-model quota rate (modelled, not observed per call).**
  - Method: a fit of 2,319 `rate_limits.primary.used_percent` readings across 5
    weekly windows, per weighted token (`uncached + cached/10 + 5*output`).
  - Result: Astra ≈1.85× and Sol ≈0.42× the baseline 7.826 Plus-units/Mwt. The
    ratio is 4.4–4.7 and stays there when any one window is left out.
  - Cross-check: the four-builder trial's frozen API prices put Astra at 5× Sol,
    so Codex quota appears roughly proportional to API dollars, about $6–7 per 1%
    of a Pro week.
  - Assumptions: all quota use came from rollouts on this machine. Only medium and
    high effort data exists for Sol.
- **Claude quota (unverified).**
  - 7-day utilisation moved 0.04 → 0.05 across the Opus and Sonnet builder runs
    (~$11 API-equivalent). That is roughly $6–12 per 1% (n=2 builds, 1% reading
    granularity).
  - Same-day volume: Codex ≈$684 API-equivalent, Claude ≈$69.
- **Effort barely moves per-call cost,** because cached context re-send dominates.
  - Astra supervisor calls average 14.3K / 14.6K / 16.6K weighted tokens at medium /
    high / xhigh.
  - The only Sol xhigh thread with turns on record (a builder, 2026-10-08, 394
    calls) averaged 16.5K, against 14.2K for Sol-medium builders (n=1).
- **Building is the smallest part of accepted work.**
  - Source: four-builder trial,
    `~/.local/state/work-engine/anthropic-builder-baselines-20261027/result-v1.json`.
  - The components below do not overlap. `builder_plus_reviewers_usd` plus
    `coordination_usd` equals `arm_cost_before_shared_overhead_usd` exactly for
    every arm.
  - Shared root and audit overhead ($35.27) is excluded.
  - n=1 task per builder. Sol's first-implementation score is unavailable (not
    zero).
  - Displayed cents are rounded, so Terra and Opus rows appear one cent off. The
    unrounded values reconcile exactly (residual 0.0, and 7e-15 for Opus).

| Builder | Build | Review (Astra) | Coordination | Arm total | Coordination share |
|---|---:|---:|---:|---:|---:|
| Terra | $0.56 | $10.15 | $16.40 | $27.10 | 60.5% |
| Sol | $3.34 | $12.85 | $21.27 | $37.46 | 56.8% |
| Sonnet | $4.02 | $11.97 | $20.46 | $36.45 | 56.1% |
| Opus | $7.19 | $13.72 | $24.45 | $45.35 | 53.9% |

- **Partitioned review once beat monolithic review on recall.** Source: E8 in
  `probes/claude-s2-obligation-compiler-20261020-protocol.md`. n=1 per arrangement.

  | Arrangement | Cost | Sealed-key recall |
  |---|---:|---:|
  | Four Haiku contexts, one concern each | $2.25 | 2/4 |
  | One Haiku context | $0.72 | 1/4 |
  | One Opus-5 context | $4.09 | 1/4 |
- **The decomposition experiment left cost open.**
  `docs/manual-decomposition-experiment-result-20261021.md` found a workable route,
  but no cost or quality advantage: there was no matched baseline and the counters
  were not collected.

Common gap: every model comparison so far is n=1 per cell, and run-to-run variation
has never been measured.

## 2. Rules for every experiment

1. **One factor per controlled comparison.**
   - Same task, frozen packet, baseline commit, acceptance conditions, coordinator
     and review route, except the factor under test.
   - Arms that change several factors at once are labelled *strategy comparisons*,
     and no single cause is drawn from them.
2. **Pre-register** in `metrics/probes/<name>-prereg.md` before launch:
   - question, arms, metrics, falsifying outcomes;
   - the replication trigger and maximum runs;
   - the result-class criteria (rule 7).

   Arms are assigned before the coordinator starts. A run already launched cannot
   be relabelled prospective.
3. **Seal the keys.**
   - Whoever writes arm prompts or runs arms does not read the answer key.
   - Scoring is done by script, or by an independent session with arm labels
     stripped. Separation helps but does not make a model-assisted scorer fully
     independent; record who scored.
   - This session has read the S2 sealed key (F1–F4 on `7fdf392f`), so it neither
     designs nor scores arms on that subject.
   - This session has not read the 34-case battery
     (`usage-export-quality-audit-20261025/cases-v1.json`) and will not.
4. **Complete, non-overlapping cost.**
   - Every actor is a separate row: coordinator, planner, battery builder, builder,
     reviewer, repair, scorer.
   - Rows reconcile to a cumulative total.
   - Costs are API-equivalent dollars at the frozen prices in
     `anthropic-builder-baselines-20261027/freeze-v1.json`.
   - Quota is reported two ways, never merged:
     - **observed movement:** utilisation reading after minus reading before;
     - **modelled consumption:** from token counts and the per-model fit, with the
       fit's residuals.
5. **Isolation by process controls, not only by clone.** Each arm runs:
   - in a clone that holds only the baseline commit: a single-ref export, no later
     refs or reflog;
   - under an explicit allowlist of readable paths, tools and network endpoints;
   - with user-level memory, skills and project instructions either excluded or
     pinned and recorded;
   - with an environment manifest written before launch. Exact `claude -p` flags
     and sandbox settings are verified and fixed in each pre-registration.

   Prior model exposure to this code cannot be excluded and is recorded as a
   limitation.
6. **Failed runs are kept.**
   - *Infrastructure failure* (capacity error, harness error, lost stream): the run
     is classed invalid, its cost is still counted, and it is retried at most once.
   - *Behavioural failure* (refusal, gives up, exceeds its per-run budget): this is
     a result, not retried. No substitute model.
   - Each run has a pre-set per-run cost limit, about 2× its pilot estimate, and
     stops there.
7. **Result classes:** confirmed advantage, confirmed disadvantage, inconclusive,
   or invalid experiment. The criteria are fixed per pre-registration.
   Inconclusive is expected to be common and is a legitimate result.
8. **State n with every result.** Results go in a `probes/` result file and a row
   in `efficiency-ledger.md`.

## 3. Spending controls

- **Two Claude measures, kept separate:**
  - **Cumulative consumption,** the budget. At most 30 quota points for Phase 1,
    summed per interval across meter resets.
    - Each interval's consumption is the observed movement between its readings.
      When a reset falls inside an interval, it is the pre-reset movement plus the
      post-reset reading.
    - Readings are coarse (1%), so the ledger also carries modelled consumption:
      API-$ ÷ $6 per point, the conservative end of the unverified $6–12 range.
    - The budget uses whichever of observed and modelled is larger.
  - **Observed utilisation,** an operational safety limit. Stop if the provider
    reading reaches 80% in any window, whatever the budget says.
  - Why both: the meter can fall while consumption continues. All 24 readings show
    one fixed `seven_day` reset, 2026-11-02 09:00Z, and Phase 1 may straddle it.
    Whether usage also ages out within a window is `not established`.
- **Allocation of the 30 points:**
  - Support work (this session's implementation, battery and fixture building,
    scoring): at most 10.
  - Experiment arms: at most 20.
- **Staged release:**
  - Stage A: F1–F3, one validated T0 battery, the E3 pilot fixtures, E1a. At most
    10 points in total.
  - The owner reviews Stage A results before Stage B is released.
- **Fable:**
  - At most 20% of the Fable weekly meter.
  - Used only for independent battery building, fixture building and scoring.
  - Not an E2 arm until the core E2 comparisons are complete.
  - First, confirm whether Fable calls move the main `seven_day` reading. The Opus
    and Sonnet streams expose only `five_hour` and `seven_day` (24 readings).
- **Codex (Phase 2):** at most 15% of a Pro week for O-arms. Claude coordinates
  Phase 2, so root and supervisor work spends no Codex quota.
- **Hard stop, in quota points throughout:**
  - Take a quota reading before each arm launches.
  - Convert the arm's per-run dollar limit to points at $6 per point. This is an
    upper bound.
  - Do not launch if cumulative consumption plus that bound would exceed the
    budget, or if the bound added to the current reading would cross the safety
    limit.
  - Stop all work at either limit and report.

## 4. Foundations

These use no experiment-arm quota. Building them draws Claude quota as support
work under §3.

| ID | Deliverable | Done when |
|---|---|---|
| F1 | `metrics/scripts/usage_ledger.py`: one row per model call for both providers. Detail below the table. | **Reconciliation shown** at three levels: raw provider counters against reconstructed call totals per session; sessions against the four-builder `measurement/final-refresh-v2.json` per-actor totals; and today's window totals. Coverage gaps listed. |
| F2 | Run registry linking run directory → threads and sessions → outcome (acceptance, findings, repairs, tests). Seeded with: four-builder baselines, Terra trial, usage-export hybrid/end-only/incremental, manual-decomposition S10, Claude R1/R2, bounded-inference B0–B3, rp1–rp3. | Every seeded run has reconciled cost rows from F1 and a sourced outcome, or `not established`. |
| F3 | One command that scores any commit against the 34-case battery, generalised from `usage-export-quality-audit-20261025/{run_audit.py,adjudicate.py}` and the per-arm `audit/adjudicate_*.py`. Built by a session that runs no arms. | Reproduces the published initial and final scores for all four builders. |
| F4 | Repair `skills/code-change-profile`: its pinned `checkpoint.py` hash predates `fca03fe5` (found by reading; not yet run). Add Rust file and item-level counts. | Profiles the four builders' final commits. |

**F1 status:** accepted and frozen 2026-10-27; see
[`probes/usage-ledger-f1-20261026.md`](probes/usage-ledger-f1-20261026.md).
Interactive Claude sessions are reported two ways (transcript and CLI counter) and
budgeted against the larger. The difference is not treated as established
expenditure.

F1 details:
- **Codex:**
  - Reads `.jsonl` and `.jsonl.zst`.
  - Deduplicates by `response_id` from `token_usage_record`. That record is present
    in all 89 rollouts from 10-26; older rollouts fall back to thread id +
    timestamp + counters. Duplicate rollout files across `sessions/` and
    `archived_sessions/` collapse through the same keys.
  - Role comes from `~/.codex/state_5.sqlite` (`threads`, `thread_spawn_edges`) and
    `agent_path`.
  - Model and effort come from `turn_context`, not the index.
- **Claude:**
  - Reads transcripts plus `subagents/`.
  - Deduplicates by `message.id`: input and cache counters from any record of that
    id, `output_tokens` as the maximum.
  - Joins in stream-json quota readings.
- **Unknown attribution** (model, role or run) stays `unknown` rather than being
  forced into a class.

### T0. Task bank and structured task selection

Every task so far was chosen ad hoc, and every model comparison rests on one task.
T0 chooses tasks to span different kinds of difficulty, so that model × task-type
differences can show.

- **Pool:** accepted slices from F2 that have a recoverable baseline commit and an
  accepted final commit. The count is `not established` until F2 exists.
- **Replayable** requires all three:
  - the baseline can be rebuilt under rule 5;
  - a neutral packet can be written from the accepted plan or contract without
    leaking the solution or its review findings;
  - the outcome can be scored from outside: CLI output, file or wire format, or an
    interface the packet specifies.
- **Difficulty features,** deterministic where possible:
  1. *Scale:* lines, files, crates (F4).
  2. *Integration depth:* existing symbols modified against added, fan-in of the
     touched symbols, crate boundaries crossed.
  3. *Specification closure:* how much the plan decides. Plan length, explicit
     interfaces, open questions, obligation count.
  4. *Hazard class:* concurrency and locking; persistence, schema and migration;
     protocol and IPC; lifecycle and recovery; or pure logic.
  5. *Discovery demand:* historical findings outside the packet's obligations, and
     P1/P2 counts.
  6. *Historical effort:* attempts, repairs, cost per thousand lines. Confounded by
     whichever model did the work, so it describes a task and never ranks one.
- **Selection:**
  - Start with 4 tasks, expandable to 6, spanning hazard classes and closure levels.
  - Cap each task by its pilot-estimated inference cost and wall time, not by
    lines changed.
  - Usage-export stays as the anchor.
- **Batteries have two separately reported categories:**
  - **Contract cases** come from the externally specified behaviour. Each must
    accept every contract-compliant implementation: compare meaning, not
    structure, ordering or formatting the contract does not fix. An independent
    session checks that every case traces to a contract clause.
  - **Historical regression cases** come from past review findings. They sample
    only the defects someone once found, and are reported as their own measure.
- **Battery validation:**
  - Fails on the baseline.
  - Passes on the accepted final.
  - Catches the **historically rejected candidates**, the attempts review sent
    back. These are also E2 subjects.

  Passing the accepted final does not show the battery is complete.
- **Built and sealed** by a session that runs no arm.
- **Output:** a task-feature table, and over time descriptive evidence on whether
  task type predicts which model suffices. The "Plan IR model routing" lever in
  `efficiency-ledger.md` needs that evidence. It is not a fitted rule at this n.

## 5. Retrospective analyses (observational)

These run after F1 and F2. **R-b is deferred** until the measurement system is
trusted and the first E2/E3 results exist.

- **R-a: S10 decomposition cost.** Reconstruct the full cost of the
  manual-decomposition experiment. Still no matched baseline.
- **R-b: Plan analyser.**
  - Corpus: roughly 60–70 plan lineages (about 114 markdown plan bodies plus JSON
    plans and 13 acceptance records; inventory counts from filename search, not
    re-verified).
  - Plan features: size, structure, obligations, revisions, review rounds, time to
    acceptance.
  - Outcomes: findings, repairs, cost, planned files against the actual diff.
- **R-c: Supervisor overhead by model.** Sol-medium against Astra supervised
  campaigns: calls per builder turn, waits, rework rounds, cost. Confounded; used
  to choose E3/O2 metrics.

## 6. Phase 1: Anthropic-only, run now

Arms run as headless `claude -p --output-format stream-json`, which captures quota
readings. A Claude coordinator runs them; no Codex actor is involved. Dollar
figures are planning estimates from cited runs, **unverified**, and each
pre-registration replaces them with a pilot measurement.

### E1. Builders: first-artifact quality (builder only)

One neutral packet, no design review, no repair loop. Historical initial-failure
counts (Terra 3, Sonnet 9, Opus 8; Sol unavailable) had an Astra design review
first and are **not** comparable.

**E1a. Pilot estimate of run-to-run variation (anchor: usage-export).**
- **Arms:** Sonnet 5.5 medium ×3, Haiku 5.5 high ×1, Opus 5.5 medium ×1, on
  baseline `1e21e71c` with the frozen neutral packet, scored by F3.
- **Prerequisite:** before any Haiku run is priced, verify the exact Haiku 5.5
  model identifier and a dated, authoritative price source, and add both to the
  frozen price table. The F1 ledger currently leaves Haiku unpriced.
- **Reading:** describes the observed spread for Sonnet on this one task. It is
  **not** a significance threshold and says nothing about Haiku, Opus or other
  tasks.
- **Budget:** about $20–25.

**E1b. Model × task type (T0 tasks, as each battery validates).**
- **Arms per task:** Haiku 5.5 high, Sonnet 5.5 medium, Opus 5.5 medium, one run
  each. Terra and Sol follow in O3.
- **Reading:** report the observed differences, with E1a's spread as context only.
- **Adaptive replication:** a cell is repeated when its result could change a
  routing decision. The trigger is fixed in the pre-registration, for example: a
  cheaper model shows one or more additional failure families, or costs under half
  as much at equal failures. Maximum 3 runs per cell and a capped replication
  budget.
- **Budget:** about $50–70 for 4 tasks before replication.

**Metrics for both:** contract-case failures and failure families,
historical-regression failures (reported separately), tests added, F4 profile,
cost, observed quota movement, turns, wall time.

### E2. Review: structure and model

- **Subjects:** 4–6 artifacts with known defects:
  - historically rejected candidates from T0 tasks, whose original findings give a
    second key;
  - E1 first artifacts;
  - the four-builder trial's `subjects/initial`.

  The key is the battery failure families plus the historical findings, sealed
  from reviewers.
- **Controlled structural comparison:** Sonnet 5.5 high as one reviewer, against
  Sonnet 5.5 high split into 4 concern partitions. The partitions are fixed in
  advance from E8's generic groups: boundary and dependency; ownership and
  admission; time, framing and shutdown; proof and tests.
- **Strategy comparisons:** Opus 5.5 high as one reviewer; Haiku 5.5 xhigh in 4
  partitions. A Sonnet-medium partition arm is optional and runs only if the
  budget allows.
- **Finding adjudication:** an independent session classifies every finding as one
  of:
  - confirmed known (matches the key);
  - confirmed novel (a real defect outside the key, shown by test or by an
    adjudicator against stated criteria);
  - false positive;
  - indeterminate.
- **Metrics:** known-defect recall; confirmed novel defects; false positives;
  cost per confirmed defect; false positives per review as a proxy for wasted
  repair work.
- **Budget:** about $50–60.

### E3. Supervisor decision replay

- **Fixtures** are built and sealed by a session that runs no arm. Pilot with 10,
  then grow to 20–40.
  - Source: past supervisor decision points from run directories. Kinds: plan
    accepted or rejected; finding accepted, deferred or dismissed;
    continue/stop/escalate; handoff timing.
  - Each fixture holds the context the supervisor actually had at the time.
  - Each fixture records the evidence for its answer and **one correctness class:**
    1. *Objectively verifiable,* e.g. a dismissed finding later failed a gate, or
       an accepted plan later hit a contract contradiction.
    2. *Independently adjudicated* against criteria written before adjudication.
    3. *Historical only:* what was decided, with no independent evidence that it
       was right.

    Only classes 1 and 2 count toward accuracy. Class 3 measures agreement only.
- **Critical unsafe decisions** are scored separately and reported per arm whatever
  the overall accuracy: dismissing or deferring a P1/P2 defect, accepting a plan
  with a contract contradiction, accepting an artifact that fails a gate.
- **Anthropic arms:** Sonnet high, Opus high, Haiku xhigh.
- **Metrics:** accuracy on classes 1 and 2 by kind; critical errors; agreement on
  class 3; cost per decision.
- **Reading:** replay accuracy screens candidates. It does not qualify a model for
  production supervision; that takes E5 or O4.
- **Budget:** about $10–20.

### E4. Build-side decomposition: deferred, conditional

Same question as v1: on fixed work, does splitting into 2–3 packets with fresh
builders change total cost or quality once planning and coordination are counted?
It runs only after F1/F2 are trusted, the first E2/E3 results are in, and E1
variation looks small enough for the expected difference to show. Pilot size,
n=1–2 per arm.

### E5. End-to-end qualification

- **Question:** Does a cheaper execution policy suggested by E1–E3 produce
  accepted, independently validated work at lower total cost than the current
  policy?
- **Design:**
  - 1–2 T0 tasks, run end to end: coordinator, builder, reviewer, repair loops,
    and final independent acceptance (battery plus independent review).
  - Arms: the current production policy (Astra supervision and review; needs the
    Codex reset) against the best cheaper candidate.
- **Metric:** cost per accepted, validated outcome. Unsuccessful runs are retained
  and their cost counted, and the failure rate is reported.
- **Gate, by task class:**
  - No production model substitution, including the supervisor, without E5 or
    O4.
  - A pass authorizes only a limited, monitored production pilot, and only for the
    T0 task classes it covered.
  - Broader deployment needs passing evidence in each additional task class,
    plus a monitored pilot with pre-set rollback triggers. For example: any
    critical unsafe decision, or a repair rate above the reference policy's.

Phase 1 arm estimate: about $130–175 API-equivalent before replication. Support
work is capped separately under §3, and the stage gates may trim E1b and E2.

## 7. Phase 2: OpenAI arms, after the Codex reset

Kept small and run with a Claude coordinator.

| ID | Arms | Pairs with | Rough Codex quota (modelled) |
|---|---|---|---|
| O1 | Astra xhigh as one reviewer (production reference); Sol medium in 4 partitions | E2 subjects, key and adjudication | ~1.5–2% per Astra review |
| O2 | Astra high (reference), Sol high, Sol xhigh | E3 fixtures, correctness classes, critical errors | small: one call per item per arm |
| O3 | Terra, Sol: builder only, as in E1, with adaptive replication | E1a/E1b | small to moderate |
| O4 | One real slice supervised by Sol xhigh against Astra high | Only if O2 shows no critical errors and comparable class-1/2 accuracy | one slice each |

O2 also gives the first Sol xhigh calls in a supervisory role, which lets the quota
fit be refit for that setting.

## 8. Order

1. **Accounting:** F1 and F2, with reconciliation demonstrated and coverage gaps
   recorded. Then F3.
2. **Prove the instruments:** one complete T0 battery beyond the anchor, validated
   against known-bad candidates, and 10 E3 fixtures with correctness classes.
   Confirm that correct and incorrect outcomes can be told apart before building a
   bank.
3. **E1a:** Sonnet ×3, Haiku, Opus. End of Stage A; owner review.
4. **E3 Anthropic arms,** then O2 after the reset. Coordination is the largest
   cost component (54–61% of each four-builder arm).
5. **E2 Anthropic arms,** then O1 after the reset.
6. **E1b and O3,** selectively, where more evidence could change an assignment.
7. **E5 / O4** before any production substitution.
8. **Deferred:** R-b and E4. R-a and R-c run whenever F1/F2 are ready; they need no
   arms.

## 9. Owner decisions

The reviewer's recommendations are adopted here as proposed defaults, pending
owner confirmation:

| Decision | Proposed default |
|---|---|
| Test bed | Usage-export as anchor, plus T0 tasks |
| Number of T0 tasks | 4, expandable to 6 |
| Maximum task scale | Pilot-estimated inference cost and wall time, not lines |
| Claude cap | 30 points total including support, staged as in §3 |
| Fable as independent builder and scorer | Conditional on confirming its quota behaviour and key isolation |
| Fable review arm | Deferred until the core E2 comparisons are complete |
| E4 | Conditional, deferred |
| Production supervisor replacement | Requires a real-slice validation (E5/O4) after E3/O2 |

## 10. Review disposition (ChatGPT review of v1, 2026-10-26)

| # | Finding | Disposition |
|---|---|---|
| P1-1 | E1a cannot set a noise threshold for E1b | Accepted. E1a is a pilot estimate; E1b reports differences without declaring them real; adaptive replication with a pre-registered trigger and maximum (§6 E1). |
| P1-2 | Batteries encode the historical implementation | Accepted. Contract cases must accept every compliant implementation and trace to a clause; historical regression cases reported separately; passing the accepted final is not completeness (§4 T0). |
| P1-3 | E2 confounds model, effort and structure | Accepted. Added Sonnet high ×4 partitions as the controlled pair against Sonnet high single; the other arms are labelled strategy comparisons; finding adjudication with false positives and confirmed novel defects (§6 E2). |
| P1-4 | E3 treats historical decisions as ground truth | Accepted. Three correctness classes, only 1–2 scored; critical unsafe decisions reported separately; replay screens and does not qualify (§6 E3). |
| P2-1 | Ledger reconciliation and uncertainty | Accepted. Codex dedup by `response_id`; three-level reconciliation; unknown attribution kept; observed quota separate from modelled (§2 rule 4, §4 F1). |
| P2-2 | Isolation by clone alone | Accepted. Single-ref clone, allowlist, excluded or pinned user-level context, environment manifest, prior-exposure limitation (§2 rule 5). |
| P2-3 | Cost categories cumulative or standalone | Presentation accepted, cumulative reading **not** supported. The trial's `coordination_usd` is standalone: build+reviews+coordination equals the arm total exactly in all four arms. Coordination is the largest component (53.9–60.5%) and review the second (30.2–37.4%). Table rewritten as non-overlapping (§1). |
| P2-4 | The 30% cap is not a spending control | Accepted. Separate support and arm caps, staged release, observed movement from a pre-run reading, hard stop, and failed-run and retry rules (§2 rule 6, §3). |
| §3 | First-artifact quality vs end-to-end economics | Accepted. Two measures defined (header); E5 added. |
| §5 | Revised execution order | Accepted with one reason added: E3 before E2 because coordination is the larger cost (§8). |
| §6 | Result classification | Accepted (§2 rule 7). |

Second review (v2, 2026-10-26): approved the §9 defaults and Stage A, starting with
F1/F2. Its first acceptance gate is F1/F2 reproducing all four trial arms' costs,
with unknown attribution and reconciliation differences explicit.

| # | Finding | Disposition |
|---|---|---|
| R2-1 | A rolling meter can fall while consumption continues; a dollar limit cannot be added to a percentage | Accepted. Cumulative consumption across resets, the larger of observed and modelled, is the budget; observed utilisation is a separate safety limit; the pre-launch check runs in points. The window's mechanism is reported as observed: a fixed reset at 2026-11-02 09:00Z in 24 of 24 readings, with ageing within a window not established (§3). |
| R2-2 | 1–2 tasks or one slice cannot qualify a general replacement | Accepted. Qualification is per task class, a pass authorizes a monitored pilot with rollback triggers, and broadening needs per-class evidence (§6 E5). |
| R2-3 | Cent discrepancies from rounding | Unrounded values reconcile exactly; stated in §1. |
