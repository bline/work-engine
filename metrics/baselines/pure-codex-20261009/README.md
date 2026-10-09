# Pure-Codex development baseline (2026-10-09)

Efficiency baseline for Work Engine development performed directly in Codex
(root session plus multi-agent v2 sub-agents), before development moves inside
Work Engine. Numbers are in [`baseline.json`](baseline.json); the scripts that
produced them are in [`scripts/`](scripts/). Compare a future Work Engine-hosted
period with the same units and caveats rather than against these figures alone.

## Units

- **Plus-unit**: 1% of a 1x Plus weekly Codex quota. Pro Lite is 5x (verified
  from equal gpt-5.6-sol tokens per unit on Plus and Pro Lite); Pro is 10x
  (user-stated, unverified).
- **Weighted tokens**: `uncached_input + cached_input/10 + 5*output`. These
  price ratios are assumptions. The fitted quota price is 4.381 units per million
  weighted tokens for gpt-5.6-sol and 7.826 for gpt-6 (1.79x, R² 0.92, 53 days).

## Headline results

| Measure | Value |
| --- | --- |
| Tokens per quota unit, gpt-5.6-sol / Pro Lite → gpt-6 / Pro | 1.68M → 0.99M (−41%) |
| Code lines per billion tokens | 19.5K → 22.9K (+17%) |
| work-engine code lines per unit: Python/Node build-out / Node migration / Rust | 25 / 17 / 24 (28 incl. uncommitted) |
| Oct 5–9 spend by role | builders 50%, root orchestrators 23%, Node-era 16%, review 6%, planning 4% |
| Cached-context re-send share of cost | ~73% (avg 126–144K context per call) |
| Pure polling (wait-only calls) | 3,320 calls, ~380 units (~11%) |
| Lifecycle slice cost | 25–40 units per 1k Rust lines; domain libraries ~16 |
| Quota consumed by Rust work | three full Pro weeks, Oct 6 21:29 → Oct 8 17:49 (local time) |
| Candidate attempts to acceptance (14 Rust slices) | mean 2.8, range 2–5; no slice accepted on its first candidate |
| Review findings resolved (11 slices with stated counts) | 66 total, ~6 per slice |
| Known failed or unfinished gates | C3/R3 HOST responsiveness gate failed; lifecycle S8 not accepted |

Quality figures come from `candidate_attempt` in the accepted-checkpoint records
and finding counts stated in `docs/rust-*-validation.md`, result docs and commit
messages. `scripts/quality.py` scans the staging folders, but it double-counts
findings repeated across files, so use the stated figures in `baseline.json`.

## Comparison checklist for the Work Engine-hosted period

Record the same quantities, then add the quality measures this baseline lacks:

1. Plus-units per accepted slice and per 1k lines, split by role and activity.
2. Share of cost from cached-context re-send, polling and startup reading.
3. Orchestration overhead as a share of slice cost.
4. Candidate attempts, review findings, failed gates, contract revisions and
   evidence reruns per accepted slice (Rust-period values in `baseline.json`).
5. Model, reasoning effort and plan multiplier in force.

## Reproducing

The scripts are the exact one-off analysis scripts. Paths and windows are hardcoded:

- `rl.py`: weekly quota windows and resets.
- `usage.py OUT.pkl`: parse all rollouts into a token/quota event series.
  `agg.py`, `daily.py IN.pkl ROWS.pkl` and `reg.py IN.pkl` consume it.
- `work.py ROWS.pkl GIT.txt` and `era.py IN.pkl NUMSTAT.txt`: git output per
  quota unit. Generate the numstat input with `git log $(git for-each-ref
  --format='%(refname)' refs/heads) --no-merges --numstat`.
- `OUT=x.pkl sess.py`, `OUT=act.pkl act.py`, then `S=<dir> report.py` and
  `drill.py`: per-session, per-activity and per-slice spend. The window
  2026-10-05..09 and the file-mtime cutoff are hardcoded.
- `quality.py`: per-folder candidate, failure-named file, test-log and finding
  scan of `~/.local/state/work-engine/rust-*` (an indicator only; see above).

Codex log fields relied on: `session_meta` (`agent_path`, `agent_role`,
`parent_thread_id`), `turn_context` (`model`), `token_count`
(`total_token_usage`, `rate_limits.primary`) and tool-call payloads. A
Work Engine-hosted run needs equivalent telemetry for the comparison to hold.
