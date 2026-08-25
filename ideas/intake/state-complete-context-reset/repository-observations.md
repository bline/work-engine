# Repository observations for state-complete context reset intake

Evidence cutoff: Work Engine `cdc9e3fa5d300e5edc737faf38edf85a336fbdcf`
plus the isolated raw-source checkpoint created for this intake.

## Effective local context capabilities

- The active Codex environment exposes `new_context`, which starts a new
  context window.
- The environment exposes `get_context_remaining`, whose current contract
  returns only `tokens_left` or unavailable. During intake it returned an
  observed value of `134587` tokens left.
- Installed `codex-cli` reports version `0.149.1`.
- `codex features list` reports `runtime_metrics` and `token_budget` as under
  development and disabled in the installed configuration.

These observations establish that intentional replacement and one pressure
measurement are available in this session. They do not establish an effective
hard context limit, occupancy percentage, window age, checkpoint baseline,
post-checkpoint growth, rehydration cost, or reset-outcome telemetry.

## Current Work Engine ownership

- `skills/wind-walker/SKILL.md` owns the continuation-independence invariant
  and leaves beneficial context replacement to model judgment. It does not
  consume quantitative context observations.
- `proposals/agent-state/role-owned-durable-operational-state/proposal.md`
  owns candidate recovery and role-state consequences. It does not define the
  model-facing context-benefit telemetry described by the raw idea.
- `proposals/empirical-agent-research/raw-agent-execution-evidence-archive/proposal.md`
  owns a candidate forensic raw-evidence archive and explicitly leaves compact
  operational measurements to metrics. It can supply later research evidence
  without owning the live replacement decision.

## Bounded implementation search

At Codebase Memory generation `2026-08-24T18:51:09Z`, task-directed indexed
searches plus direct literal searches over `proposals/`, `skills/`, `docs/`,
`planning/`, `campaigns/`, and `metrics/` found no implementation of the raw
idea’s distinctive telemetry fields: `active_context_tokens`,
`effective_hard_limit_tokens`, `context_tokens_at_checkpoint`,
`post_checkpoint_context_growth`, or `estimated_rehydration_tokens`.

Coverage metadata reported no recorded issue for the relied-on source files.
Repository-wide excluded areas were `.git`, `.vscode`, `node_modules`, and
generated `__pycache__` directories; direct searches intentionally excluded
those non-source scopes. Best-effort coverage is not proof of completeness.
