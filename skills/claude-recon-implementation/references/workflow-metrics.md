# Workflow metrics

When available, report:

- reconnaissance, supplemental, and final-gate call counts;
- placement calls, candidate count, placement conflicts, placement reconsiderations, and targeted-reconnaissance calls;
- placement risk and whether the vertical semantic proof passed;
- successful, failed, timed-out, and infrastructure-failed calls;
- attempt outcomes and available token, cost, and time measurements partitioned
  by the evidence capability actually used;
- provider failure primary causes and any transition from indexed structural
  evidence to direct source observation, including whether the reason was index
  availability, coverage, graph ambiguity, or provider failure;
- Claude wall-clock duration, cost, cache creation/read usage, output tokens, and thinking tokens;
- Claude Code version, gateway, exact requested model, transport attempts,
  quota-failover outcome, and whether the upstream provider was observed;
- files, exact ranges, and approximate lines supplied to Codex;
- whether missing context blocked implementation;
- local repository exploration outside Claude-provided ranges;
- selected workflow route, route revisions, preserved evidence, and stale decisions;
- validation and review stages selected or omitted with their risk rationale;
- final test totals and review findings by severity.
