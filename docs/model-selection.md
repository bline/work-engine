# Model selection for engineering work

Current guidance: 2026-10-27. This is the single place for model and effort
choices. It replaces the 2026-10-20 version, which covered only `gpt-6-astra`
and `gpt-6-sol`, and the role choices that skills stated for themselves (see
[Where choices are encoded](#where-choices-are-encoded)).

This is repository-wide decision guidance, not product doctrine or an automatic
router. Explicit user choices and owning campaign, provider or experiment
contracts take precedence. It does not change existing configuration, authorize
agent launches, or relax review, budget, provider-use or acceptance boundaries.

## Choose for the outcome, not the token price

Prefer the model and effort with the lowest expected total cost of reaching an
accepted result. That cost includes reasoning, latency, rework, review,
coordination, operator attention and maintenance.

- **The builder's own inference is a small share of an accepted slice.** In the
  four-builder trial (n = 1 per builder) the build was 2–16% of each arm's cost,
  and review and coordination were the rest
  ([experiment plan §1](../metrics/experiment-plan-20261026.md)).
- **A cheaper builder pays off only if its first attempt is good enough.** Every
  extra repair round repeats review and coordination.
- **A larger model is not automatically useful for every step.**

Contracts constrain the result; model selection stays judgment wherever the
owning contract leaves it open ([DESIGN.md](../DESIGN.md)).

## Current defaults

Use exact model IDs. Effort is a starting point, not a claim that more effort
always helps.

| Role | Default | Effort | Status and evidence |
| --- | --- | --- | --- |
| **Builder** for implementation, integration, tests and ordinary remediation under a clear contract | `gpt-6.1-terra` | `medium` | **Owner decision 2026-10-27, provisional.** Usage-export task: 3 first-version failures in the four-builder trial (n = 1), the cheapest arm; 7, 7 and 11 confirmed failures at $0.13–0.15 per run in [E1a-Terra](../metrics/probes/e1a-terra-result-20261027.md) (n = 3). Its tests are thinner than the Claude builders' (2–4 against 10–14). |
| **Builder** for concurrency, persistence or schema, protocol or IPC, lifecycle or recovery, cross-crate, or open design questions | `gpt-6.1-sol` | `medium` | **Recommendation 2026-10-27, owner may change.** Terra has no evidence on these task classes yet. `gpt-6.1-sol` has the same input and output prices as `gpt-6-sol` and cheaper cached input; not yet measured by us. |
| Builder on the Claude host, with a fully specified packet | `claude-sonnet-5-5` | `high` | Matched Opus on gates and defect incidence at 38% of the cost ([S2 builder probe](../metrics/probes/claude-s2-builder-20261020-protocol.md), lines 467–560, n = 1). At `medium`, Sonnet had 11–15 confirmed failures in [E1a](../metrics/probes/e1a-result-20261027.md) (n = 3). |
| Builder on the Claude host, where unresolved reasoning or complexity warrants it | `claude-opus-5-5` | `high` | 9 confirmed failures at `medium` in E1a (n = 1). |
| Successor after a Claude builder refusal | the default builder above | as above | The refusal is recorded; the successor gets a fresh packet. |
| Architecture, ownership contracts, and planning with unresolved design questions | `gpt-6-astra` | `xhigh` | Spend reasoning where mistakes propagate. On the Claude host: `claude-opus-5-5` at `high`. |
| Supervisor / coordinator | mixed today: `gpt-6-astra` (mostly `high`) and `gpt-6-sol` (`medium`) | as recorded | By call count in the frozen usage ledger (roles inferred from thread names): Sol medium 2,361, Astra high 1,403, Astra medium 316, Astra xhigh 260. **Changes at the next Codex reset (2026-11-02):** `gpt-6.1-sol` at `xhigh`, after supervisor decision replay (O2) and as a monitored pilot with rollback triggers ([dogfood plan](../metrics/dogfood-selection-plan-20261027.md), [experiment plan](../metrics/experiment-plan-20261026.md) §6 E5). On the Claude host: `claude-opus-5-5`. |
| Adversarial review, where an OpenAI reviewer is permitted | separate `gpt-6-astra` | `xhigh` | Challenge the composed result from a separately established context. Partitioned Sol review is planned as experiment O1. |
| Tier-1 reviewer, verifier or assessor on the Claude host | `claude-opus-5-5` | `high` | No matched Sonnet review comparison yet. |
| Tier-2 reviewer on the Claude host | `claude-fable-5-1`, fresh context | default | **Scarce.** One contract review cost 2% of the weekly Fable meter (n = 1), and Fable also draws the main Claude meter at Fable prices. Use it only for a shared contract, the final acceptance of a workstream, or an unresolved P1 between builder and tier-1 reviewer. Give it a small input (contract, diff, tier-1 findings, with paths), and never a retained follow-up. |
| Reconnaissance, routine documentation, straightforward edits | the default builder | `medium` or `high` | Match effort to the remaining uncertainty and consequence. |
| Experiment arms | as pre-registered | as pre-registered | An arm's model and effort come from its pre-registration, never from this table. |

These are project recommendations, not performance guarantees or mandatory
phases. A small change need not visit every role. A builder can plan a
well-bounded implementation, and Astra can implement when the hard part cannot
be separated from architectural reasoning.

**The provisional Terra default** stays under watch in its first real slices.
Watch, through the existing receipts and the dogfood records:
- independent review findings;
- repair rounds;
- total cost to acceptance;
- test adequacy.

Review and acceptance requirements are unchanged. Revisit the default if review
findings or repair rounds exceed those of the previous builder default on
comparable slices.

## Escalation

- **Effort:** escalate one level at a time, `low` → `medium` → `high`. Stop for
  human judgment before `xhigh` unless configuration or the user authorizes it.
  Record replaced workers, efforts, evidence and reasons.
- **Builder model:** move from `gpt-6.1-terra` to the hazard-class builder when
  any of these happens:
  - the task turns out to belong to a hazard class above;
  - review returns a P1 or P2 finding that reopens the design;
  - a second repair round is needed.
  
  Move to Astra when evidence exposes an unresolved ownership boundary,
  contradictory assumptions, or a causal explanation the investigation cannot
  establish.
- **What doesn't justify a model change:** a compiler error, a capacity error,
  or a fixed number of failed tests. A model change does not repair a harness,
  test or contract defect, and a failure count alone doesn't establish that the
  model caused the failures.

## Cost and quota facts that bear on selection

| Fact | Source |
| --- | --- |
| Frozen API prices per million tokens (input, cached input, output): Terra 1 / 0.05 / 5; Sol 6 2 / 0.2 / 10; Sol 6.1 2 / 0.1 / 10; Astra 10 / 1 / 50; Sonnet 5.5 2 / 0.1 / 10; Opus 5.5 4 / 0.2 / 20 | `CODEX_RATES` and `NATIVE_RATES` in `metrics/scripts/usage_ledger.py` |
| Codex quota tracks API cost: Astra uses about 4.5× Sol's quota per weighted token, and 1% of a Pro week is about $6–7 of API-equivalent spend | experiment plan §1 (2,319 readings over 5 weekly windows); efficiency ledger, Cost structure |
| 1% of the Claude weekly meter is about $6–12 of API-equivalent spend (rough: whole-percent readings, few intervals) | experiment plan §3 |
| Effort barely changes Astra's and Sol's cost per call, because cached context dominates; total cost still depends on how many calls a task takes | experiment plan §1 |
| In the Codex window measured on 2026-10-26, Astra-run root, supervisor and review work used about 83% of the quota, builders about 9% | experiment plan §1 (roles from thread names, about 10% unclassified) |

API prices do not establish subscription quota for models outside the quota
fit (Terra, Sol 6.1, Fable).

## Session defaults versus assignments

Sessions started without an explicit selection run their client's default. Today
those are `gpt-6.1-sol` at `xhigh` in `~/.codex/config.toml`, and Opus at `xhigh`
in `~/.claude/settings.json`. Those defaults suit roots and coordinators.
Delegated assignments always select model and effort explicitly, from this guide.

## Make selection and execution distinguishable

- **Select explicitly.** Inherited defaults can silently select a different model
  from the one intended. For a delegated task, pass the exact model ID and effort
  through the launch mechanism.
- **Exact IDs only.** Aliases (`sonnet`, `opus`, `default`) and bracketed
  variants can't be compared with a requested identifier.
- **Record requested and observed identity separately,** in the existing task
  record or receipt:
  - Claude's observed model is `message.model` in the transcript;
  - Codex's is `turn_context.model` in the rollout;
  - `metrics/scripts/task_profile_record.py attribute` resolves them from the
    usage ledger.
  
  An unavailable identity is unknown, not verified because a configuration file
  names it.
- **Contract-bound selections:**
  - an observed mismatch leaves the requirement unmet;
  - an unobserved identity leaves it unestablished.
- **Capacity failure doesn't authorize a silent fallback.**
  - Where retry is authorized and safe, retry the same model within the deadline
    and budget, and never replay work already entered.
  - If a different model is permitted, record the reason and the actual change.
  - If the selection is fixed, get the owning authority's agreement first.
- **No automatic upgrade** to a newer model is implied by this document.

## Preserve implementation and review ownership

The builder owns implementation through integration, validation and ordinary
remediation within its authority. Unfinished builder work does not automatically
transfer to a stronger model, and retaining a useful builder context avoids
reconstructing the problem.

Review isolation follows the owning review claim:
- **A fresh context** without an inherited builder or designer conversation is
  the default for an initial adversarial review. On its own it doesn't prove
  independence.
- **Retain a useful reviewer** through bounded remediation, unless the review
  premise or the required isolation changes.
- **A designer can't review its own design,** however it is renamed.
- **Astra and Sol are both OpenAI models,** so pairing them doesn't meet a
  cross-provider requirement. A Claude builder with an Astra reviewer does.

## Learn from outcomes without creating a new gate

Use existing receipts to compare accepted outcomes, escaped defects, review
repairs, total time, token and cache costs where observed, and operator
intervention. Separate model performance from differences in task, context,
tools, harness and validation; missing measurements stay unavailable.

The [dogfood plan](../metrics/dogfood-selection-plan-20261027.md) records a task
profile and the assigned configuration at each dispatch, so that later
selection can be evaluated per role. Revise these defaults when evidence changes
their value. Collecting a new benchmark is not a prerequisite for ordinary
authorized work.

## Where choices are encoded

These carry this guide's choices and must match it. Where they differ, this
guide states the intended choice and the copy is stale.

| Location | What it encodes | State on 2026-10-27 |
| --- | --- | --- |
| `skills/slice-builder/scripts/resolve_provider.py`: CLI defaults (`--builder-model`, `--builder-reasoning-effort`) and the `CURRENT_CODEX_MODELS` allowlist | campaign builder default; which builder and reviewer models are accepted | aligned: `gpt-6.1-terra` / `medium`; the allowlist is `gpt-6.1-terra`, `gpt-6.1-sol`, `gpt-6-sol`, `gpt-6-astra` |
| `skills/slice-supervisor/scripts/campaign-preflight.mjs`: `defaults.builder` and the `currentBuilderModels` allowlist | campaign builder default; preflight rejects a builder model not on its list | aligned: `gpt-6.1-terra` / `medium`; same allowlist |
| `skills/slice-supervisor/references/work-engine-config.md` | documented campaign defaults | aligned: `gpt-6.1-terra` / `medium` |
| `skills/slice-builder/SKILL.md` | default statement and effort ladder | points to this guide |
| `skills/claude-builder/SKILL.md` and `references/astra-review.md`, `references/native-session.md` | Claude builder, successor and reviewer choices | point to this guide |
| `skills/manual-orchestration/references/anthropic-host.md` | Claude-host role table | points to this guide; the host mapping (`Agent` `model`, observed `message.model`) stays there |
| `resolve_builder_context()` signature default in `resolve_provider.py` | internal default for callers that omit the model | stays `gpt-6-sol` / `high` on purpose: production paths pass the model explicitly, and tests use the default to build same-model review cases |
| `skills/slice-supervisor/scripts/append_metrics.py`, `.get("model", "gpt-6-sol")` and `.get("reasoning_effort", "high")` | how a receipt with no recorded builder model is read | stays as is on purpose: receipts written without an explicit model ran under the earlier default |
| `~/.codex/config.toml`, `~/.claude/settings.json` | session defaults | see above |

The alignment and pointer edits were applied on 2026-10-27, and the
slice-builder, slice-supervisor, campaign-preflight and manual-orchestration
suites pass on that tree.

None of the 9 slice-supervisor campaign files sets `builder.model` or
`builder.reasoning_effort`, so all take the default above; all 9 pass preflight.
7 of them named `gpt-5.6-sol` at `low` until 2026-10-27, when that model left the
allowlists; each records the removal as an amendment. The 4 with a Codex
adversarial reviewer (`agent-instruction-review`, `code-change-characterization-baseline`,
`raw-idea-intake`, `review-remediation`) now name `gpt-6-astra` at `xhigh` with
the different-model evidence class. Their earlier amendments, which record the
owner's approval of a Sol low reviewer, are unchanged.
`campaigns/comparative-repository-analysis.yaml` is a comparative-analysis
configuration with no builder.

## Decisions

| Date | Decision | By |
| --- | --- | --- |
| 2026-10-20 | `gpt-6-sol` at `medium` for builders (superseded) | owner |
| 2026-10-27 | `gpt-6.1-terra` at `medium` as the default builder, provisional | owner |
| 2026-10-27 | `gpt-6.1-sol` at `medium` for hazard-class builder work until Terra is measured there | recommendation; owner may change |
| 2026-10-27 | Campaign builder defaults align to this guide | owner |
| 2026-10-27 | Supervisor moves to `gpt-6.1-sol` at `xhigh` at the next Codex reset, after O2 and as a monitored pilot | owner |
