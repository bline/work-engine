# Objective budget effectiveness, 2026-10-30 to 11-04

Objective budgets have worked as a ceiling, not as a forecast. Across 138
closed objectives, the median objective spent **49% of its tightest dollar
limit** and **43% of its hours limit**. Only 9 of 131 objectives with a dollar
limit reached 95% of it. When objectives failed, money was rarely the reason:
half of the not-accepted objectives stopped below half their limit, and their
close reasons name blockers instead. The larger controllable costs sit outside
the limit itself:

- fixed root work at the start and end of every objective;
- root work between objectives;
- the half of all spend that no budget covers.

This is a retrospective observation. There is no comparison group, so none of
it measures what budgeting itself caused. The
[observations JSON](budget-effectiveness-20261104.json) has every per-objective
row, every summary figure, and the hashes of its inputs and scripts. The
pipeline that produced it is in
[`experiments/budget-effectiveness-20261104/`](experiments/budget-effectiveness-20261104/).
No paid agents were launched, and no record, budget, skill or runtime was
changed by this analysis.

Costs are speed-adjusted API-equivalent USD under the scenario
`usage-ledger-standard-api-v1` of [usage_ledger.py](scripts/usage_ledger.py),
SHA-256 `ed026f40f93801c9fffab4a9d5d5fc32745fd1d94a6137edd22ed875b5a67291`.
They are neither an invoice nor subscription quota.

## Results

Unless stated otherwise, figures use the **plausible-end set**: 138 closed
objectives and $614.14 of spend (how the set is defined is under What was
measured). The headline calibration holds on all three subsets:

| Subset | Objectives | Spend / tightest $ limit: median (IQR), p90 | Reached 95% of $ limit | Elapsed / hours limit: median |
| --- | ---: | --- | ---: | ---: |
| Explicit close time | 74 | 0.48 (0.32–0.66), 0.83 | 6 of 71 | 0.44 |
| Plausible end | 138 | 0.49 (0.32–0.69), 0.88 | 9 of 131 | 0.43 |
| All closed | 154 | 0.49 (0.28–0.69), 0.89 | 11 of 145 | 0.43 |

**Limits are about twice actual spend.** Six objectives exceeded their dollar
limit, by factors from 1.002 to 1.324. Six exceeded their hours limit:

- by 1.45, 1.39, 1.03, 1.02 and 1.00, using explicit or closure-written end times;
- by 2.21, where the end time comes from the record file and is uncertain.

In this set the longest objective ran 6.25 hours. The all-closed subset adds
the Dot bridge parent record of 11-01, which was never closed. Its file-time
window spans 17.1 hours (3.4× its 5-hour limit, $60.02 of $62). That window
includes later nested objectives with their own grants, so the overrun is not
established. Apart from that record, nothing resembles the 17.6-hour S10
session of 2026-10-28/29. With no pre-budget cohort measured the same way,
though, that is a contrast, not an effect.

**Failures are not budget exhaustion.** 51 of the 138 objectives were not
accepted (43 `not_accepted`, 8 `abandoned`), and they account for $191.31, or
31% of spend. Of the 48 with a dollar limit, **24 stopped below half of it**
and 5 reached 95% of it. Most of the 9 objectives that reached 95% did fail (5
of the 9), but they are a small share of all failures. The close reasons of the
8 abandoned objectives (all from 11-03 on) give these reasons:

- missing client tools;
- input/tool isolation not established;
- unsupported or unqualified routes;
- a feasibility no-go;
- one reached budget reserve;
- one stopped because the account meter read 100%.

The status strings of the 43 `not_accepted` objectives, grouped by keyword,
break down as:

- unfinished, unreviewed or held: 20;
- admission or readiness failed: 11;
- review findings or changes required: 6;
- a budget, quota or hours stop: 5;
- other: 1.

**By type.** Types were assigned by hand from each objective's text (n = 1
rater):

| Type | n | Accepted / not | Median spend | p90 spend | Median tightest limit | Median spend / limit | Median root share |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| plan | 45 | 35 / 9 | $3.23 | $5.82 | $6.25 | 0.50 | 38% |
| review | 25 | 13 / 12 | $1.72 | $3.99 | $5.00 | 0.44 | 50% |
| test | 17 | 8 / 9 | $1.84 | $3.12 | $4.50 | 0.50 | 37% |
| build | 27 | 17 / 10 | $5.27 | $14.50 | $10.25 | 0.43 | 49% |
| live | 24 | 13 / 11 | $4.05 | $18.13 | $11.50 | 0.55 | 61% |

"Live" means qualification or integration against a real runtime. Its
not-accepted objectives cost $76.09, the most of any type. For reviews, "not
accepted" often means that the review completed and the subject failed it,
not that the review failed.

**The root is about half of objective spend.** The median root share is 0.48.
It is 0.40 in the 10-29 to 11-01 grant-per-item period (n = 100, median
objective $2.35) and 0.53 from 11-02 (n = 38, median objective $4.53).

**Fixed root work per objective.** For the 110 objectives with child work,
root spend falls into three phases:

- before the first child call (setup): **11.7%** of their spend;
- after the last child call (closeout): **6.8%**;
- while children ran: 26.7%.

Setup plus closeout is a median **17%** of each objective (IQR 10–31%). Its
share shrinks with objective size, in tertiles of n = 36, 36 and 38:

| Objective spend | Setup + closeout share (median) | Accepted / not |
| --- | ---: | --- |
| $0.53–$2.60 | 29% | 20 / 15 |
| $2.65–$4.83 | 18% | 26 / 10 |
| $4.86–$52.03 | 10% | 24 / 14 |

**Root work between objectives.** From 10-30, the roots that own budget records
spent $324.82 inside objective windows and **$276.26 (46%) outside** them. Of
the outside spend:

- $60.07 (22%) fell within 30 minutes after a close;
- $41.89 (15%) fell within 30 minutes before a start;
- $174.30 (63%) was further from any objective.

**Budget commands run by the root model.** Model calls whose tool cells invoke
`scope_budget.py <subcommand>`, `usage_ledger.py tree|control|build` or
`--budget-record` take **5.4–12.9%** of root spend across 7 roots in two
windows. This is a lower bound: budget arithmetic written into general record
scripts is not counted.

**Half of all spend is outside any budget.** From 10-30 to the 2026-11-04
ledger build (22:43Z), objective trees cover **$691.97 of $1,355.82 (51%)**.
The unbudgeted $663.86 breaks down as:

- budget roots outside their objectives: $277.29 (42%);
- other Codex sessions, such as single-thread projects and unlinked workers: $266.59 (40%);
- Claude sessions: $119.98 (18%).

Daily coverage ranged from 26% (11-03) to 71% (11-01).

**Daily spend fell, but not attributably.** Daily spend was $600–754 on 10-23
to 10-26 and $105–329 on 10-31 to 11-04 (11-04 is a partial day). The drop
overlaps the Astra → Sol 6.1 routing change of 10-28, so it is not attributed
to budgets.

## Implications

These follow from the observations above. They are proposals, and none has
been tested.

1. **Fewer, larger objectives.** Fixed root setup and closeout take 29% of the
   smallest third of objectives, against 10% of the largest. Root work just
   outside objective windows adds $102 on top of $325 inside. Batching
   sequential items under one objective should shrink both. How large the
   saving would be is not established.
2. **Budget observation should not be model work.** Root calls running budget
   commands are 5–13% of root spend, before counting hand-written accounting
   records. This is consistent with the
   [budget observation plan](../planning/work-engine-budget-observation-implementation-plan.md)'s
   intent that observation be host-side and deterministic. That intent is
   evidently not yet fully realized.
3. **Check feasibility before funding live objectives.** Failed objectives
   usually stop well below their limit, so a limit does not catch them. A
   short precondition check is the pattern that did work: the
   `runner-qualification` objective reached an early no-go at $2.88. Such a
   check would cover tool availability, the isolation route and account
   headroom.
4. **Per-type history for setting limits.** If warning marks should carry
   information, the type table above can supply starting values, for example
   a first mark near the type median and the limit near its p90. Whether
   tighter limits would change spending is not established.
5. **Decide what budgets are for.** If the goal is to control total spend,
   per-objective limits currently cover about half of it. Root time between
   objectives and unbudgeted sessions would need an account-level or daily
   envelope.

## What was measured

**Records.** 291 files have a top-level `scope_budget` object. Discovery had
two parts:

1. A depth-limited scan: every `.json` file up to four levels below
   `~/.local/state/work-engine` modified since 2026-10-29. That is 44,178 files,
   of which 161 are records.
2. A targeted walk of the 133 top-level directories modified since 10-29,
   seven named roots, the B3 qualification package, and
   `~/Documents/Codex/2026-10-29` to `10-31`. That found 291 records.

All 161 records from the scan are in the targeted set. Records deeper than
four levels in directories outside the targeted set are not established. A
first pass found only 124 files, because an unbounded `grep` over the state
tree was killed by its timeout. That pass's results were discarded.

**Objectives.** The 291 files hold 235 distinct objective IDs.

- 9 have no bound session, so they cannot be measured, and one is a smoke
  test.
- 66 are views or copies, such as Dot's observer, window and sprint-analysis
  files. Those carry their own objective IDs and duplicate a root's item.
- The remaining canonical files (`assignment.json`, `scope.json`,
  `budget.json` and two single-file objectives) are kept with one record per
  root session and start minute.

That leaves **159 objectives**, of which 154 are closed.

**End times.** An objective's window ends at its first available end time, in
this order:

1. `scope_budget.closed_at` (39 objectives);
2. a `closed_at` in a result file (12) or in the record itself (23);
3. otherwise, the record file's modification time (85).

A modification time is plausible when the record was last written at closure.
That is common: most such records carry closure statuses, and 50 of the 85 end
within an hour of the next objective on the same root. It is unreliable for
the Dot bridge `budget.json` files, which were written at creation and carry
no status. Hence three subsets:

- **Explicit close time**: no file-time ends.
- **Plausible end**: drops file-time ends whose outcome is unknown.
- **All closed.**

**Cost.** Each objective's spend is `usage_ledger.py tree` over its window, run
with the committed reader. All 165 trees report the same reader digest.
Totals add the `codex` and `claude_all_models` pools. `claude_fable` is a
subset of the second, so it is not added again.

The limit compared against is the **tightest** objective-wide dollar limit,
user or supervisor, using the latest entry for each setter. Per-child limits
are separate: 60 of them match a tree node, with median spend 0.67 of the limit
and 3 exceeded.

**Outcomes.** Outcomes come from `scope_budget.close.disposition` where it
exists. Otherwise the free-text status in the record or its result file is
mapped by `classify.py:outcome`. Statuses containing UNACCEPTED, UNFINISHED,
FAILED, CHANGES_REQUIRED, STOPPED, INCOMPLETE and similar words map to
`not_accepted`. Statuses containing ACCEPT, COMPLETE or PASSED map to
`accepted`. An empty status maps to `unknown`. "Accepted" is the root's own
disposition and often means a bounded partial result.

**Types.** Types were labelled by hand
([`type-labels.json`](experiments/budget-effectiveness-20261104/type-labels.json)).
A keyword classifier in the same file agreed with the hand labels on only 37 of
64, mostly because objective texts mention review as a step. It was not used.

**Root phases.** Setup is root spend in the window before the earliest call of
any child node, and closeout is root spend after the latest. Objectives with
several root sessions sum all of them. A first version processed each root
session separately and double-counted 3 objectives (111 instead of 110). The
corrected figures are reported here.

**Coverage.** A call is covered when its session is a node of some objective
tree and the call falls inside that objective's window. Each call is counted
once.

**Budget commands.** A model call is one `token_usage_record`. Its tool cells
are those issued since the previous record, following the earlier
skill-change analysis. A first, broader pattern matched any mention of
`scope_budget.py` or `usage_ledger.py` and produced 16–36%. Of 8 sampled
matches, 4–5 were development work on the budget tools themselves (reading or
editing their source, index checks) rather than budget-keeping. The pattern was
narrowed to invocations, and 8 of 8 sampled matches were then budget
operations. The 16–36% figure is wrong and should not be cited.

## Limitations

- Observational only. Roots, tasks and periods differ, and nothing here is a
  treatment effect.
- Types and the status-to-outcome mapping come from one rater.
- 85 of 159 end times come from file modification times. The three subsets
  bound this, and the medians move by at most 0.01.
- One objective (`FIFTH/native-message-g001`) attributes $0 with $1.69
  unattributed in its window. That is a lineage gap, not a measured zero.
- The setup and closeout split uses child call times only. Root work
  interleaved with child work is not divided further.
- The ledger build ended at 2026-11-04 22:43Z. Five objectives were still open
  and are excluded from closed-objective figures.
- The weekly meter readings in the ledger mix plans and reset times, so
  account-level headroom was not analysed.

## Reproduction

The pipeline is in
[`experiments/budget-effectiveness-20261104/`](experiments/budget-effectiveness-20261104/).
Its working directory is the private `~/cbm-work/budget-analysis-20261104`;
override it with `BUDGET_ANALYSIS_DIR` for `export.py`. The other scripts name
that directory directly.

1. `discover.py`: list record files.
2. `inventory.py`, then `resolve.py`: dedupe by objective ID, resolve end time
   and status.
3. `canon.py`: canonical objectives.
4. `run_tree.sh` per objective: `usage_ledger.py tree --since --until`.
5. `table.py`: per-objective table.
6. Add type labels and outcomes (`type-labels.json`, `classify.py:outcome`).
7. `phases.py`.
8. `export.py`: builds the JSON, including coverage, root outside-window spend
   and `overhead.py`.

The ledger is
`usage_ledger.py build --since 2026-10-23T00:00:00Z`. Its `calls.jsonl` hash,
the record-list hash and all script hashes are in the JSON. Rollouts,
transcripts and records stay private; the JSON carries their paths, counters
and per-tree hashes.
