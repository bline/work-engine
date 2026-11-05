# Pi harness choice: benchmark evidence and remaining qualification

Recorded 2026-11-05 following the maintainer's supplied research summary.
Status: dated research note, with a bounded primary-source check. No benchmark
was rerun and no A/B experiment or architectural amendment is authorized here.

The external evidence supports continuing to qualify Pi as Work Engine's
coding harness. Its quality/cost tradeoff looks credible on bounded tasks.
Long-running development, recovery and governed execution remain separate
qualification outcomes.

## What the primary sources establish

**[HarnessTax](https://harnesstax.github.io/)** lists UC Berkeley and Arena
Intelligence authors and compares seven models across Pi, Codex CLI and Claude
Code on SWE-bench Lite and Terminal-Bench 2.0. Its setup describes 30 tasks per
benchmark and three attempts per model/harness/task: 3,780 scheduled attempts
by arithmetic. It uses native harness configurations, each harness's own high
effort setting, a 100-turn limit and a fixed API price list. Its confidence
intervals resample task averages; three attempts on one task do not become three
independent tasks. The authors report stronger harness effects on cost than
success and describe Pi as competitive. This supports the direction; it does
not establish identical effective effort or capabilities across harnesses.

Verification limit: the supplied GPT-5.6 Sol chart row and “9 of 12” statistic
were not independently checked against the chart's underlying data in this
pass. They remain verification targets. Initial direct page extraction was
empty; the primary site's indexed text supplied the setup and stated findings.
No raw trajectory audit is claimed.

**[Personal Agent Benchmark](https://github.com/treygoff24/personal-agent-benchmark)**
publishes this corrected headline table:

| Harness | Passed / scoreable | Pass rate | Median time on passes | API-equivalent cost / success |
| --- | --- | --- | --- | --- |
| Pi | 154 / 208 | 74.0% | 4.6 minutes | $0.93 |
| Claude Code | 158 / 214 | 73.8% | 4.8 minutes | $2.14 |
| Codex CLI | 158 / 215 | 73.5% | 4.1 minutes | $1.16 |

The author documents excluded configuration failures, a grading correction and
unequal scoreable denominators. The quality assessment also has a Claude Code
selection-bias caveat involving committed fixes missing from worktree diffs.
Treat this as an informative engineering pilot; nearby percentages alone do
not prove equivalence or a general quality ranking. The reported figures were
checked in the README, without reproducing its exclusions or raw-run analysis.

**[FrontierHarness Eval](https://github.com/frontier-harness-eval/eval)** reports
the same Kimi K3 model and 30 tasks:

| Harness | Pass rate | Median cost per pass |
| --- | --- | --- |
| Codex | 66.7% | $3.47 |
| Claude Code | 63.3% | $18.34 |
| Pi | 60.0% | $2.43 |

Here Codex's observed success is higher and Pi's reported cost is lower. This
is useful counterevidence to a universal Pi quality advantage. The repository
also warns that new runs need a matched control to establish comparability;
the original baseline's applied network allowlist was not recorded. Its cost
statistic differs from the preceding benchmark's cost per success.

**[Orbi's same-model comparison](https://orbi.build/blog/pi-coding-agent-vs-claude-code/)**,
dated November 1, used DeepSeek V4.1 Flash on two small tasks. Both harnesses
passed all runs with open execution permissions. Default Claude Code used
roughly seven to eight times Pi's input tokens; its `--bare` configuration was
comparable to Pi, falling below Pi in five of six paired runs. This implicates
default instructions and tool definitions as an overhead source. It is a small
author-operated experiment, with different provider wire formats, not evidence
that Pi always has an inherent resource advantage. Its Pi 1.2.0 setup is also
different from Work Engine's pinned Pi SDK 1.5.0 profile.

Two related papers add context; only their abstracts were checked here:

- [An Empirical Study of Harness Design for Coding Agents](https://arxiv.org/abs/2609.20804)
  reports benefits from context management under tighter windows and efficient
  combinations of deterministic elision and model summarization.
- [Beyond the Model: Demystifying Harness Effects in Software Engineering Agents](https://arxiv.org/abs/2609.32459)
  studies other harnesses, finding task/model-dependent benefits from richer
  machinery and adverse results from some compression and general-subagent
  choices. It is not a direct Pi comparison.

## Consequence for Work Engine

The architectural hypothesis is a capable, small coding loop composed with
Work Engine's Rust owners for authority, effect admission, budgets, custody,
continuity, review and publication. Published task success does not qualify
those owner contracts. A useful improvement is measured across the whole
accepted workflow, including its review, repair, recovery and coordination.

Our own [current system evidence](../records/local-development-system-qualification-20261105/README.md)
is narrower: an actual Events/MCP → Rust command → job-owner chain passed with
a fixture Host. The prior real Pi trial completed four provider requests and
physical reset/recovery, but its edit matched nothing and its validator never
ran. The [runtime owner decision](../records/local-development-system-qualification-20261105/S10-OWNER-DECISION.json)
preserves that failure. Current repair work supplies missing local evidence;
the external benchmarks do not convert it into success.

This supports retaining Pi as the selected qualification route while keeping
the implementation small and its obligations explicit. It establishes no need
to reproduce every feature of the Codex or Claude Code harnesses, retire other
engineering/review roles, or widen the existing one-tool continuity profile.

## Proposed later matched pilot

After the bounded development path works, form a separate Pi-versus-Codex
comparison with a pinned identical model/version, declared effort, repository
snapshot, task, permissions, tool access, runtime/network policy and acceptance
tests. Default versus minimal harness configuration should be an explicit
treatment. Record harness/SDK versions, prompts and startup inputs so hidden
configuration changes cannot masquerade as a harness effect.

Measure accepted fixes, regressions, raw model calls and usage, cache behavior,
elapsed time and total workflow cost per accepted outcome. Include longer work
that exercises context replacement, further coding, interruption and recovery.
Use blinded semantic review where required by the acceptance claim. Distinguish
injected pressure from measured pressure and recoverability from actual
successful continued work.

API-equivalent dollars, provider billing and subscription quota are different
measurements. A shared weekly account meter cannot attribute consumption to one
trial. Report per-trial quota as unavailable unless its observation and
attribution are established; keep that uncertainty out of claimed savings.

This is a future experiment proposal. It is outside the current
[25 USD prerequisite repair round](../planning/local-development-prerequisite-repair-budget-20261105.md).
