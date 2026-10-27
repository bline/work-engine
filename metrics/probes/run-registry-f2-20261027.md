# F2 result: run registry (2026-10-27): accepted

Plan: [`metrics/experiment-plan-20261026.md`](../experiment-plan-20261026.md) §4 F2.
Script: [`metrics/scripts/run_registry.py`](../scripts/run_registry.py).

The script reads the frozen F1 ledger and never re-counts calls. It imports F1's
readers unmodified.

**Status: accepted by the owner on 2026-10-27.** It links runs to threads, sessions
and subagents with stated evidence, and it never forces attribution. One validation
gate passes: the four-builder trial run reproduces the trial's own per-actor
measurement.

## Inputs and outputs

| Item | sha256 |
|---|---|
| `metrics/scripts/run_registry.py` | `087c2d17ac80bfac37b7e3bb63cf0943359521f660d59b314d0f80e459535f65` |
| F1 frozen ledger `~/cbm-work/usage-ledger/f1-frozen-20261027T0430Z/calls.jsonl` | `29d1a6ed5f0da06561d85a33938a2b6da7529621b277b21134f69eba0230ffc7` (F1 record) |
| Seed list, 41 run directories: [`run-registry-f2-seed-runs-20261027.txt`](run-registry-f2-seed-runs-20261027.txt) | `d04941d5b84aec3225d28310030a275121352159884df0029e78799022ae59b0` |
| `~/.codex/state_5.sqlite` as read: live, so a later read differs (mtime 2026-10-26 22:37:03 -0600) | `97a4d69b52ea3ccf2b0507fffab98326e040c9c38dcca0eaf4069f8fe63cd425` |
| [`run-registry-f2-runs-20261027.json`](run-registry-f2-runs-20261027.json) (`runs.json`) | `cbd0b8d40cf7c51e6adaed08f561b99ca4d3cf30d72a434f90b8756a4bcd885b` |
| [`run-registry-f2-summary-20261027.json`](run-registry-f2-summary-20261027.json) | `a1045a2e4352ad67c2a191dda426f34fdc1d552559497300e3068336962de926` |
| `threads.jsonl`, outside the repo, `~/cbm-work/run-registry/v7/` | `32b52884a2b8eebab8dc668a5273d2f870404124b24abb3b928c84c42506435d` |
| `sessions.jsonl` (same directory) | `8cc3c64fa255d1ca970a6beedfcbb6ff20a3f6eff5800d908cadc24271787108` |
| `agents.jsonl` (same directory) | `ff9e53819a4530d3a18e89c016d16e3d93c53012aa4d4afebd45ed3c687a2674` |

```bash
python3 -I metrics/scripts/run_registry.py --ledger ~/cbm-work/usage-ledger/f1-frozen-20261027T0430Z \
  --runs $(cat metrics/probes/run-registry-f2-seed-runs-20261027.txt) --out DIR
```

The Claude transcripts, the Codex rollout headers and the run directories are read
live. A rerun reproduces these outputs only while they are unchanged.

## What each record holds

- **Codex thread** (1,116):
  - rollout files;
  - spawn parent from the rollout's `session_meta`, cross-checked against
    `state_5.sqlite` `thread_spawn_edges`;
  - `forked_from_id`, with the inherited starting counter from F1 level 1;
  - model and effort segments taken from per-call `turn_context`, not the index's
    latest setting;
  - resume gaps of 4 hours or more;
  - the index's role, model and effort, kept separately for comparison;
  - cost.
- **Claude session** (200):
  - transcript files and project directories;
  - cwd values, relocated cwds, worktrees;
  - `continued-in` / continued-from links;
  - `fork-context-ref` parents;
  - `model_refusal_fallback` events;
  - model segments and resume gaps;
  - cost two ways: transcript-derived (main thread, and with its subagents) and the
    CLI counter (each `cost-state` record, plus the maximum).
- **Claude subagent** (120): agent id, session, agent type, model segments, cost.
  Session cost covers main-thread calls only, so no call is counted at both levels.
- **Observed environment.** Each thread records its Codex CLI version and the
  repository commit and branch at thread start. Each session records its Claude
  Code versions and git branches. Each run summarises these over its attributed,
  windowed and shared entities, and says whether they are mixed.
  - Which instruction files were loaded was never recorded. Agents read skills
    through `~/.claude/skills` symlinks into the live working tree, so this is
    `not established` for every past run.
- **Run** (41):
  - every link, with its evidence (class, source file and JSON pointer), roles and
    declared time windows;
  - the attribution class of each linked entity;
  - cost buckets;
  - outcome fields copied verbatim with source path and sha256, or
    `not established`.

## Relationship findings (summary.json)

| Item | Count |
|---|---:|
| Codex threads forked (`forked_from_id`) | 80 |
| Threads with an inherited starting counter | 2 |
| Spawn parent: rollout and index agree / thread has no parent / disagree | 793 / 323 / 0 |
| Threads whose model or effort changed mid-thread | 7 |
| Threads with a resume gap of 4 hours or more | 60 |
| Claude sessions continued in another session | 3 |
| Sessions with a fallback model change | 1 |
| Sessions whose model changed | 2 |
| Sessions with transcripts in more than one project directory | 0 |
| Agent paths carried by more than one thread | 3 |
| Runs whose observed environment is mixed | 21 of 41 |

Mixed environments are common within a single run. For example, the four-builder
trial's 19 Codex threads started on 4 repository commits (`294a0ef579`,
`2a139a77d3`, `45b38e0600`, `971ea65490`), all on CLI 0.164.0, and its 4 Claude
sessions ran Claude Code 2.1.311. This is the drift the plan's frozen environment
manifest exists to prevent.

## Attribution, with ambiguity preserved

| Class | Links | Meaning |
|---|---:|---|
| attributed | 202 | declared / stream / cwd / unique agent_path evidence, one run only |
| windowed | 10 | declared with time windows, one run; only calls inside the windows count |
| windowed_shared | 9 | as windowed, but another run also claims the entity |
| shared | 52 | strong evidence from more than one run; listed in each, summed in none |
| inferred | 4 | descendant of a non-top-level thread that one run alone declares |
| candidate | 256 | only mentioned or listed, or an ambiguous agent path |

Only the attributed and windowed buckets can be summed within a run. Shared and
candidate amounts are listed in each run that names the entity, so they **must not
be summed across runs**.

Entities claimed by more than one run (25):

| Entities | Runs |
|---:|---|
| 8 | bounded-inference B1a and B1b |
| 4 | four-builder baselines, Terra trial, usage-export hybrid |
| 4 | four-builder baselines, Terra trial |
| 2 | B0, B1a and B1b |
| 2 | usage-export end-only and pilot |
| 1 | the root thread: four-builder baselines and usage-export pilot |
| 1 | B2 live-readiness and offline |
| 1 | one long-lived thread across rp0, rp1, rp2 and rp3 directories |
| 1 | rp0 and rp3 |
| 1 | rp1 plan review and implementation |

These are phases of one campaign, or a trial that re-measured another run's actors
as historical anchors. They are kept as shared rather than split.

## Validation gate: four-builder trial run

The registry's view of `anthropic-builder-baselines-20261027` against that run's own
`measurement/final-refresh-v2.json`:
- **19 of 19 Codex actors exact.** This includes the root thread, whose three
  declared windows sum to $50.134.
- **The four native Claude sessions sum to $11.2052, exact.** For these headless
  sessions the transcript and the CLI counter agree.
- **Buckets reproduce the trial's components.**
  - windowed $84.1005: the 8 new-arm Codex actors, the auditor and the accountant;
  - windowed_shared $86.3265: historical Sol, Terra and the root windows.

## Method decisions made while building, each from an observed failure

| Version | Observed failure | Change |
|---|---|---|
| v1 | rp1–rp3 directories each showed the same 63 shared threads ($3,183) | Directories that are repository checkouts (`AGENTS.md`, `.git`, `Cargo.toml`, `ARCHITECTURE.md`) and copied `metrics/` trees are no longer scanned. Their copies of `metrics/probes` list other experiments' threads. |
| v2 | R1/R2 subagents and S10's Codex workers were invisible | Claude subagent entities keyed by agent id; Codex `agent_path` links, strong only when exactly one thread carries the path. |
| v3 | The root showed $139.50 against the trial's $50.13 | Only a record's own subject key links. `parent_thread_id` and `root_thread` describe neighbours, and had attached open-start windows to the root. Snapshot-like files (board, lineage, context, inventory, census, …) become `listed`. |
| v4 | Five threads leaked back as attributed through quoted agent paths in `lineage-launch.json` | The listing rule also applies to agent-path evidence. |
| v5 | A rule demoting any file naming more than 20 entities demoted R1's `assignments.json` (28 agents) | That rule now applies only to loose agent-path mentions in non-assignment files. |
| v6 | Claude session cost was main-thread only while the CLI counter covers subagents | Added the with-subagents transcript figure, so the two-way comparison compares like with like. |
| v7 | The owner asked for each run's environment (plan §2, *Environment for experiment arms*) | Added the observed environment per thread, session and run. Attribution and costs are unchanged: four-builder buckets $84.1005 / $86.3265 / $11.2052. |

## Handling: key material in the outcome records

`run-registry-f2-runs-20261027.json` copies outcome fields verbatim, each truncated
to about 600 characters. For the four-builder run, that includes the battery audit's
`audit/*-result-*.json` findings and the Sonnet review findings. These are part of
E2's answer key for the four-builder subjects (plan §6 E2).
- Arms do not see them: each arm's clone holds only its baseline commit (plan §2
  rule 5), and every current baseline predates this file.
- Arm designers and reviewers for E2 must not read those records. They are
  `outcome_records` entries of run `anthropic-builder-baselines-20261027` whose
  source is under `audit/`, `sonnet/review-*`, or a builder repair result.

## Limits and open items

- **S10 decomposition is partial.** 9 threads ($119.99) are attributed through agent
  paths, all from slices C1, C2 and D; `/root/s10_integrated_astra` alone is
  $86.76. Slices A and B do not name their workers in the run directory, so S10's
  total cost remains `not established` (R-a).
- **Coordinator sessions are candidates.** The interactive Claude sessions that
  coordinated R1/R2 serve many runs, so their main-thread cost is a candidate in each
  run that mentions them.
- **Two-way Claude cost.** The R1 coordinator session shows $86.31 from transcripts
  including subagents, against $107.47 on the CLI counter. The S10-linked session
  shows $34.82 against $256.26.
  - Transcript dollars exclude models with no frozen rate (Opus 5, Fable, Haiku).
    The counter prices them by its own table, which explains part of the
    difference.
  - The remainder is the open F1 interactive-session gap, not established as spend.
- **No outcome record in 6 runs:** bounded-inference B0, bounded-inference planning,
  rp0-sc3d-implementation, rp0-sc3d-session-handoff, rp1-astra-plan-review,
  rp1-plan.
- **Outcome records are not adjudicated.** Fields are copied verbatim, and a run can
  carry many records, provisional and final, which are not ranked.
- **Heuristics that can misclassify:** the listing filename pattern, the subject-key
  list and the role-key list. Every link carries its source and pointer, so any
  misclassification can be traced and corrected.
- **Seed scope:** the 41 directories matching the plan's seed families, rp0
  included. Other runs are not registered yet.
