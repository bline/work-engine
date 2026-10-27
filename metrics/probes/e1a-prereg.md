# E1a pre-registration: run-to-run variation of first artifacts on usage-export (2026-10-27)

Plan: [`metrics/experiment-plan-20261026.md`](../experiment-plan-20261026.md) §6 E1a.
Written before any arm launched. Arms are assigned here and cannot be relabelled
after launch.

## Question

How much do first artifacts vary between repeated Sonnet 5.5 builder runs on one
task, with the same packet, environment and scoring? Single Haiku 5.5 and Opus 5.5
runs give context only. This is a pilot estimate, not a significance threshold, and
says nothing about other tasks.

## Arms

| Arm | Model | Effort | Per-run cost limit | Wall limit |
|---|---|---|---|---|
| S1, S2, S3 | `claude-sonnet-5-5` | medium | $6 | 3 h |
| H1 | `claude-haiku-5-5` | high | $4 | 3 h |
| O1 | `claude-opus-5-5` | medium | $12 | 3 h |

- All five arms launch together, after one quota reading, and run in parallel.
- Limits are about 2× the four-builder build component (Sonnet $4.02, Opus $7.19).
  That component also covered design review and repairs, so it overstates a first
  artifact. Haiku's limit allows for the >100K-prompt price tier.

## Fixed inputs

| Input | Identity |
|---|---|
| Baseline | `1e21e71c5dea67d143ebd7696d7884789018ee69`, single-ref clone per arm (one ref, no later commits) |
| Contract | `~/cbm-work/e1a/packet/contract-e1a-v1.json`, sha256 `b2bd7207a8a2b9a8…`. Four-builder contract with the review route and trial measurement removed |
| Interface | `~/cbm-work/e1a/packet/interface-v1.md`, sha256 `7dd9525a160bc524…`. Terra `88b035f2` structure; reason codes fixed only for conditions the contract names |
| Prompt template | `~/cbm-work/e1a/packet/builder-prompt-v1.txt`, sha256 `b21a82d4c9f4fe2f…` |
| Environment | manifest [`e1a-environment.json`](e1a-environment.json), sha256 `71d7ad2badc48eff…`: 2,189 instruction and config files, Claude Code 2.1.311 |
| Runner | `~/cbm-work/e1a/tools/arm_run.py`, sha256 `86c9d73fcd6e86f7…` |
| Scorer | F3.1, `battery_score.py` sha256 `51ef39399c3d…`, `--interface e1a-v1` |

## Harness

- **Production builder path.** The worker role and Anthropic host mapping come from
  frozen copies. The launcher is the production `claude_transport.py` with the
  exact-model guard:
  `claude -p --session-id … --tools default --permission-mode acceptEdits --allowedTools Bash --add-dir <evidence> --output-format stream-json --verbose --include-partial-messages`.
- **One assignment, no design checkpoint, no review.** The builder commits on branch
  `e1a` and writes `evidence/completion.json`. Production context checks
  (`child_context.py`) are available to it.
- **Handoff.** If a builder reports `handoff_ready`, one successor turn on the same
  model and effort is allowed, under the same per-run limit. Its cost counts toward
  the arm.
- **Sandbox (bubblewrap).**
  - The home directory is empty except for the toolchain (read-only), the arm's
    directory, and the arm's frozen Claude home mounted at `~/.claude`. The
    credentials file is bound, not copied.
  - The run directories, the main repository, other clones, the battery and
    scorer, `~/.codex` and the main transcripts are absent inside it.
  - The codebase-memory index is a fresh, private, per-arm cache.
- **Watchdog.** It prices the arm's own transcript every 20 s and stops the sandbox
  at the cost or wall limit. Tested on 2026-10-27: stopped at 20 s.

## Scoring and metrics

- **Headline:** confirmed failures and confirmed-failure families from F3.1 at the
  arm's final `HEAD`. If the builder left uncommitted changes, the coordinator
  commits them as a separate commit, records that it did so, and scores that state.
- **Reported separately:**
  - nonblocking differences;
  - indeterminate cases;
  - interface non-conformance (`interface_non_conformant`, which is a behavioural
    result, not a pass).
- **Also recorded:**
  - tests added (new `#[test]` functions in the diff);
  - crate tests, `cargo fmt --check` and clippy, run by the coordinator inside the
    sandbox;
  - cost from the arm's transcript (F1 rates, with Haiku from
    [`price-table-addendum-20261027.json`](price-table-addendum-20261027.json));
  - calls, wall time, turns, context checks, compactions and handoffs;
  - model-guard outcome.
  - F4 profile: `not established`, because F4 is not built.
- **Quota:** one reading before the batch and one after. Observed movement is
  reported for the batch only, since parallel arms are not separable; modelled
  consumption is reported per arm.
- **Who scores:** the scorer script. The coordinator reads only its counts-only
  output, never `--detail-out`.
- **Contract versus regression split:** `not established` for this battery. Counts
  by fixture-provenance group are reported instead.

## Run classes (plan §2 rule 6)

- **Invalid (infrastructure):** the environment check fails at preparation, a
  capacity or harness error, or a lost stream. Its cost is counted, and it is
  retried at most once.
- **Behavioural result, not retried:**
  - a refusal or model-guard violation;
  - the cost or wall limit reached;
  - `blocked`;
  - no commit and no changes.

  It is scored as it stands if anything was committed, and reported otherwise.

## Reading and result criteria

- **Spread** is the range of confirmed-failure counts and of failure-family counts
  across S1–S3, with each run listed. n = 3; no variance estimate is claimed.
- **E1b consequence, fixed now:**
  - If the Sonnet range in confirmed failures is 3 or more, single E1b runs are
    treated as unable to separate models on differences smaller than that range,
    and the E1b replication trigger applies to them.
  - If the range is 2 or less, single runs are kept, with the range reported as
    context.
- **H1 and O1** are described against that range only. No model ranking is drawn
  from n = 1.
- **Result class for E1a as a whole:** *pilot estimate obtained*, or *invalid
  experiment* if fewer than 2 Sonnet runs produce a scoreable, conformant artifact.
  E1a has no confirmed-advantage outcome.

## Budget check (plan §3)

- Stage A cap: 16 points. The sum of the five arms' limits is $34, which is 5.7
  points at $6 per point.
- The pre-launch reading and the cumulative position are recorded in the result
  file. No launch happens if cumulative consumption plus 5.7 exceeds 16, or if 5.7
  points added to the current reading would cross 80%.

## Known limitations

- **Network** is not confined. The sandbox cannot allowlist endpoints without
  cutting off the model API; web tools remain available, as in production.
- **Prior model exposure** to this code cannot be excluded.
- **The interface** is a reviewed design, so it carries some review-derived
  structure. First-artifact counts are not comparable with the historical
  four-builder initial counts.
- **Project instructions** are the baseline clone's own `AGENTS.md`, `CLAUDE.md`
  and `DESIGN.md`, as in any replay. Operational skills are today's frozen copies.
- **The clone's project MCP server** (`work-engine`) fails to start in the sandbox.
  It is not loaded in production sessions either.
- **The environment differs from the historical arms' in recorded ways:**
  - no design review;
  - no Codex supervisor;
  - a Claude coordinator that only collects.
