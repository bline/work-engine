# Model selection for engineering work

Current guidance: 2026-11-04; the
[Claude host](#claude-host-and-claude-only-development) section dates from
2026-11-07. This is the single place for model and effort choices. It replaces the 2026-10-20 version, which covered only `gpt-6-astra`
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
  extra repair round repeats review and coordination. In dogfood round 2 a
  first review cost a median four times the build it reviewed
  ([round 2 outcomes](../metrics/probes/dogfood-round2-builder-outcomes-20261104.md)).
- **A larger model is not automatically useful for every step.**

Contracts constrain the result; model selection stays judgment wherever the
owning contract leaves it open ([DESIGN.md](../DESIGN.md)).

## Current defaults

Use exact model IDs. Effort is a starting point, not a claim that more effort
always helps. **`gpt-6-astra` is the review model:** use it for adversarial
review, not for supervising, coordinating, planning or building.

| Role | Default | Effort | Status and evidence |
| --- | --- | --- | --- |
| **Builder** for implementation, integration, tests and ordinary remediation under a clear contract | `gpt-6.1-sol` | `medium` | **Owner decision 2026-11-04.** In randomized dogfood round 2 ([outcomes](../metrics/probes/dogfood-round2-builder-outcomes-20261104.md), n = 9 Sol, 10 Terra), Sol needed 0.44 repair rounds per task against Terra's 1.00, and passed first review 5 of 9 times against 3 of 9. Neither difference is significant at this n (p = 0.15 and 0.32). Sol's builds cost no more: median $0.93 per builder session against Terra's $1.08, because Terra made more calls. A first review cost a median four times the build. |
| **Builder** for a small, mechanical, fully specified change (rename, configuration, fixture or documentation edit) with focused validation | `gpt-6.1-terra` | `medium` | **Narrowed 2026-11-04** from the provisional default of 2026-10-27. [E1a-Terra](../metrics/probes/e1a-terra-result-20261027.md) (n = 3, $0.13–0.15 per run): 0 failures in all three runs on the 12 battery cases that trace to the packet; the full battery gives 7, 7 and 11, but most of its cases do not trace to the packet ([traceability check](../metrics/probes/e1a-result-20261027.md#battery-traceability-post-hoc-not-pre-registered)). Usage-export task: 3 first-version failures in the four-builder trial (n = 1), the cheapest arm. Its tests were thinner than the Claude builders' (2–4 against 10–14). In round 2 it took 1.00 repair rounds per task, and one task drew three P1 findings at first review. |
| Any role on the Claude host, including Claude-only development | see [Claude host and Claude-only development](#claude-host-and-claude-only-development) | | |
| Successor after a Claude builder refusal | the default builder above | as above | The refusal is recorded; the successor gets a fresh packet. |
| Architecture, ownership contracts, and planning with unresolved design questions | `gpt-6.1-sol` | `xhigh` | Spend reasoning where mistakes propagate; an Astra review at `xhigh` challenges the result (row below). |
| Supervisor / coordinator, including supervisors spawned as subagents | `gpt-6.1-sol` | `xhigh` | **Owner decision 2026-10-28.** `spawn_agent` gives a child its parent's model unless overridden, so a supervisor spawned from a `gpt-6.1-sol` root needs no override. Before this date supervisors were a mix (frozen usage ledger, by call count: Sol medium 2,361, Astra high 1,403, Astra medium 316, Astra xhigh 260). O2 and O4 ([experiment plan](../metrics/experiment-plan-20261026.md) §7) remain available as checks. |
| Adversarial review, where an OpenAI reviewer is permitted | separate `gpt-6-astra` | `high`; `xhigh` when the subject is architecture, an ownership contract, or planning with unresolved design questions | Challenge the composed result from a separately established context. No measured review comparison of `high` and `xhigh` exists; on supervisor calls `xhigh` used about 14% more weighted tokens per call ([experiment plan §1](../metrics/experiment-plan-20261026.md)). Partitioned Sol review is planned as experiment O1. |
| Reconnaissance | the default builder | `medium` or `high` | Match effort to the remaining uncertainty and consequence. Routine documentation and straightforward edits follow the Terra row above. |
| Experiment arms | as pre-registered | as pre-registered | An arm's model and effort come from its pre-registration, never from this table. |

These are project recommendations, not performance guarantees or mandatory
phases. A small change need not visit every role. A builder can plan a
well-bounded implementation.

**Select for the work remaining after discovery and contract formation.**
Sol at `medium` is the default for implementation under a clear contract,
including ordinary changes to persistence, schemas, protocols, lifecycle code or
several packages. Choose Terra only when what remains is mechanical: a small,
fully specified change checked by focused validation, where a first-review
failure is unlikely and cheap to repair. Consider the actual implementation
difficulty and consequences, not just the subsystem's name.

Open design questions belong in planning (Sol at `xhigh`, row above), not in a
builder's effort level. The builder does not inherit the planner's model
selection. Record the reason for a non-default builder in the existing
assignment.

**Why the default changed.** The 2026-10-27 Terra default was provisional: the
guide said to revisit it if review findings or repair rounds exceeded those of
the previous builder default on comparable slices. Dogfood round 2 made that
comparison, randomized on production slices. Terra took 10 repair rounds over
10 tasks against Sol's 4 over 9, and its builds were not cheaper
([round 2 outcomes](../metrics/probes/dogfood-round2-builder-outcomes-20261104.md)).

Keep watching both choices, through the existing receipts and the dogfood
records:
- independent review findings;
- repair rounds;
- total cost to acceptance;
- test adequacy.

Review and acceptance requirements are unchanged. Revisit if Sol's repair
rounds or review findings rise on comparable slices, or if changes chosen as
mechanical for Terra start needing repair rounds.

## Escalation

- **Effort:** escalate one level at a time, `low` → `medium` → `high`. Stop for
  human judgment before `xhigh` unless configuration or the user authorizes it.
  Record replaced workers, efforts, evidence and reasons.
- **Builder model:** move a Terra assignment to Sol when the work turns out not
  to be mechanical, or when repeated failures on clear requirements support a
  model-capability explanation. Reaching a repair-round count is insufficient on
  its own. Diagnose missing contract facts, validation gaps and harness failures
  before attributing the problem to the model.

  A review finding that reopens design goes to the appropriate decision owner;
  select the builder again for the work remaining once that question is settled.
  
  Move to `gpt-6.1-sol` at `xhigh` when evidence exposes an unresolved ownership
  boundary, contradictory assumptions, or a causal explanation the investigation
  cannot establish.
- **What doesn't justify a model change:** a compiler error, a capacity error,
  or a fixed number of failed tests. A model change does not repair a harness,
  test or contract defect, and a failure count alone doesn't establish that the
  model caused the failures.

## Claude host and Claude-only development

This section holds the model choices for every role that runs on Claude Code:
a Claude role inside Codex-led work, and Claude-only development, where the root
and every child run on Anthropic models. Host mechanics (the `Agent` tool's
`model` and `effort`, transcripts, worktrees, root transfer) are in the
[Claude Code host reference](../skills/manual-orchestration/references/anthropic-host.md).
Claude-only work coordinates through
[manual orchestration](../skills/manual-orchestration/SKILL.md) on Claude Code.
`skills/claude-builder` does not apply there: it assumes a Codex caller and an
Astra reviewer.

Every comparison below is n = 1 per model and effort unless stated, and most come
from one subject, the S2 slice. Read the rows as a starting point, not as
measured rankings.

| Role | Model | Effort | Status and evidence |
| --- | --- | --- | --- |
| Supervisor / root | `claude-opus-5-5` | the session default (`xhigh` in `~/.claude/settings.json` on 2026-11-07) | What the measured Claude roots ran: the R1 and R2 roots (`bf60b4c1`, `c1d31e88`) record only `claude-opus-5-5` in `message.model`. None of the 259 studies in the 2026-11-06 evidence inventory (`~/cbm-work/evidence-map-20261106`) compares supervisor models on Claude. E3 built 10 fixtures from past supervisor decisions for Sonnet `high`, Opus `high` and Haiku `xhigh`, and its arms were never run ([E3 fixtures](../metrics/probes/e3-pilot-fixtures-20261027.md)). R1's orchestrator used 36.2% of the campaign's weighted tokens, against 25% in the pure-Codex baseline ([R1 score](../metrics/probes/claude-builder-campaign-r1-score-20261022.md), n = 1 campaign). |
| Architecture, ownership contracts, and planning with unresolved design questions | `claude-opus-5-5` | `high` | Carries over the Codex row's reasoning: spend reasoning where mistakes propagate. Not measured on Claude. R1's planners ran this configuration. |
| Builder with a fully specified packet | `claude-sonnet-5-5` | `high` | [S2 builder probe](../metrics/probes/claude-s2-builder-20261020-protocol.md) (lines 467–579), same plan and prompt: Sonnet and `claude-opus-5` (the earlier Opus, not 5.5) both passed 7 of 7 gates with 44 tests on exactly the declared paths, and both left the shutdown join loop unsafe to outer cancellation. Sonnet cost $4.71 against $12.50 and took 15.6 against 32.6 minutes. At `medium` in [E1a](../metrics/probes/e1a-result-20261027.md) (n = 3, $0.67–0.73 per run), Sonnet failed 2, 0 and 0 of the 12 cases that trace to the packet. R1 and R2 used Sonnet `high` builders for about 10,200 and 11,100 Rust lines (n = 1 campaign each). |
| Builder where unresolved reasoning or complexity warrants it | `claude-opus-5-5` | `high` | In E1a at `medium` (n = 1, $2.35, 7.6 minutes), Opus failed 0 of the 12 traced cases. |
| Reconnaissance, documentation and small mechanical edits | `claude-sonnet-5-5` | `medium` | There is no Claude counterpart to the Terra row. Haiku was not cheaper as a builder: in E1a, Haiku 5.5 at `high` cost $0.78 over 59 calls and 11.2 minutes, against $0.67–0.73 and 3.1–3.8 minutes for Sonnet at `medium` (n = 1 Haiku run); 43 of its 59 calls fell in the higher price tier for prompts over 100,000 tokens. |
| Tier-1 reviewer, verifier or assessor | `claude-opus-5-5` | `high` | Kept as tier 1, with the second pass below. In Test R generation 2 (same subject, unchanged prompt, all at `xhigh`; [protocol](../metrics/probes/claude-s2-review-independence-20261019-protocol.md), lines 276–462), one reference defect separated the models: a task spawned before it was registered. Haiku matched it, Sonnet declared it unresolved, and both `claude-opus-5-5` and `claude-opus-5` cleared it as verified. `claude-opus-5` also cleared it at `high` and at `max`, so effort did not change the outcome ([generation 3](../metrics/probes/claude-s2-review-generation3-20261020-protocol.md), lines 113–137). Cost per review: Haiku $0.39, Sonnet $1.24, Opus 5.5 $1.94. |
| Second review pass | `claude-sonnet-5-5` or `claude-haiku-5-5`, fresh context | `xhigh` | **Recommendation 2026-11-07.** Give it a concern statement (the properties to examine, naming no defect or file) or a small group of contract obligations, not the whole contract as obligations. Evidence under [Review on Claude only](#review-on-claude-only). |
| Tier-2 reviewer | `claude-fable-5-1`, fresh context | default | **Scarce.** One contract review used 2% of the weekly Fable meter (n = 1), and Fable counts against the all-models weekly limit as well as its own (owner-confirmed 2026-10-30). Use it only for a shared contract, the final acceptance of a workstream, or an unresolved P1 between builder and tier-1 reviewer. Give it a small input (contract, diff, tier-1 findings, with paths), and never a retained follow-up. |

**Pass model and effort explicitly in every `Agent` call;** this table is the
owning instruction for the effort. R1 did so: its subagents' `meta.json` records
`sonnet` at `high` for builders and `opus` at `high` for reviewers and planners.
The `model` parameter takes an alias, so record the observed `message.model`.

### Review on Claude only

Claude-only review is same-provider review. Record it as such: it does not meet a
cross-provider requirement
([review ownership](#preserve-implementation-and-review-ownership)). The evidence
on what it misses, and on what helped:

- **Inside review missed contract-class defects.** Across two slices that review
  inside the Claude orchestration had passed with P3 findings only, an outside
  non-Claude reviewer found 1 P1 and 3 P2s. One of the slices had also passed the
  Fable tier-2 final
  ([P1 port cohort](../metrics/probes/claude-p1-port-cohort-20261022.md), lines
  258–311; [R1 score](../metrics/probes/claude-builder-campaign-r1-score-20261022.md),
  lines 209–249; n = 2 slices, 1 outside reviewer). The R1 record reads this as
  review at these tiers finding P3-class issues reliably and provenance- and
  contract-class issues unreliably.
- **Naming what to examine changed what was found.** Sonnet at `xhigh`, given a
  concern statement (happens-before, ownership across cancellation, truthful close
  facts; no defect or file named), matched the spawn-before-register defect it
  had declared unresolved without one, and cost less: $0.99 against $1.24
  ([generation 3](../metrics/probes/claude-s2-review-generation3-20261020-protocol.md),
  lines 238–262).
- **Obligations find what they target, and only that.** No Claude review found
  the reference cancellation defect in 11 runs across 4 models and 4 effort
  levels (generation 3, lines 275–280). Obligations aimed at that defect class
  found it; obligations aimed at another class did not
  ([S2 builder probe](../metrics/probes/claude-s2-builder-20261020-protocol.md),
  lines 615–672). Haiku given one obligation alone found it for $0.37, where the
  same text inside a 25-obligation packet did not
  ([obligation compiler](../metrics/probes/claude-s2-obligation-compiler-20261020-protocol.md),
  lines 772–836). Keep obligation groups small; the best size is not established.
- **Re-grade severity.** All four generation-2 reviewers matched the reference's
  only high-severity defect and graded it low, and Haiku graded another matched
  defect low where the reference graded it medium (obligation compiler, lines
  861–877). Judge detection and severity separately: a reviewer's low grade alone
  does not make a finding non-blocking.
- **Check attribution before a revert.** Across 17 Claude review runs on one
  subject, reviewers attributed every discrepancy caused by the plan itself to the
  implementation, and all five arms whose full findings were kept prescribed
  revert or remove
  ([finding attribution](../metrics/probes/claude-finding-attribution-20261020.md),
  lines 33–38). Before applying such a remedy, check whether the contract
  requires what the finding flags.
- **Size review by what it must read.** In R2, end-of-slice review plus
  remediation cost 105% of the builds they reviewed, and two reviewers reached
  the context trigger and handed off
  ([R2 score](../metrics/probes/claude-builder-campaign-r2-score-20261023.md),
  lines 344–368). Divide a large subject into named obligation groups, as the
  [supervisor reference](../skills/manual-orchestration/references/supervisor.md)
  describes.

Where an owner requires an outside reader, for example for a shared contract, a
published interface or a parity or provenance claim, Claude-only review does not
satisfy the requirement. Record the gap; do not describe the review as
independent.

**An outside reader on API credits (2026-11-07).** `gpt-6-astra` can run from a
separate Codex home, `~/.codex-api`, logged in with an OpenAI API key and billed
to that account's API credits (USD 100, auto-recharge off, per the owner). The
main `~/.codex` stays on the ChatGPT login, so other Codex sessions are not
affected. Each use spends from the credits, so it needs the owner's approval for
that objective: an objective authorized only for Claude reviewers does not cover
it. Launch one fresh, read-only process per review:

```bash
CODEX_HOME=$HOME/.codex-api codex exec -C <checkout of the reviewed commit> \
  -m gpt-6-astra -c model_reasoning_effort=high -c service_tier=default \
  -s read-only "<review prompt>"
```

- Use `xhigh` instead of `high` when the subject is architecture, an ownership
  contract or planning with unresolved design questions, as for any Astra review.
- The review contract is [Codex adversarial review](../skills/codex-adversarial-review/SKILL.md).
  Its `spawn_agent` binding steps assume a Codex caller; record the session ID
  that `codex exec` prints instead.
- Read the observed model, tier and token use from the rollout under
  `~/.codex-api/sessions/`. `metrics/scripts/usage_ledger.py` reads only
  `~/.codex`, so these reviews do not appear in its totals or in budget checks.
- **Cost:** an Astra first review in dogfood round 2 cost $1.30–9.67 at frozen API
  prices, median about $4.50 (n = 16), so the credits cover roughly 10–20 first
  reviews. The frozen Astra prices match OpenAI's published Standard prices
  (checked 2026-11-07; [cost table](#cost-and-quota-facts-that-bear-on-selection)),
  except that OpenAI also sells a 12-hour cache write at twice the 30-minute price
  the table uses. Which one Codex requests is not established, so a frozen-price
  figure understates the writes if Codex asks for 12 hours. The N1–N3 final
  acceptance review on these credits (session
  `01a1b74b-f8b6-7912-bb35-4b88587d68a2`, 18 calls) cost $3.61 with 30-minute
  writes and $5.43 with 12-hour writes; writes were half of the lower figure. The
  billed amount is on the OpenAI usage page, not in the rollout.
- Spend it where Claude review has been weakest: contract-class changes and the
  final acceptance of a workstream.
- A one-call probe on 2026-11-07 observed `gpt-6-astra` at `high` on the `default`
  tier (session `01a1b6fb-0dee-7b93-9d45-ee5d56250d0e`, 13,169 tokens, 13,161 of
  them cache writes: $0.16 at frozen prices, $0.33 if the writes were 12-hour).

### Supervisor practice on Claude

- **Root context size is the one measured cost lever; the root model has never
  been varied.** A Claude Code root reached 656,196 tokens with no automatic
  compaction. Transferring it to a fresh successor cut context by 92% and made
  each coordination call 5.0 times cheaper, breaking even after 1.4 calls
  ([root transfer](../metrics/probes/claude-root-transfer-20261022.md), n = 1).
  Plan root transfers
  ([procedure](../skills/manual-orchestration/references/anthropic-host.md#root-transfer-on-claude-code))
  rather than waiting for compaction.
- **Limit parallel high-effort agents.** Four parallel `xhigh` reviewers moved the
  five-hour window about 8 points (n = 1 batch), and earlier review arms failed on
  the five-hour limit, not the weekly one
  ([Test R protocol](../metrics/probes/claude-s2-review-independence-20261019-protocol.md),
  lines 499–515). Three parallel Fable agents with an Opus coordinator moved it
  from 1% to 13% in four minutes
  ([experiment plan §3](../metrics/experiment-plan-20261026.md#3-spending-controls),
  n = 1 interval).
- **Compare Claude tiers in dollars, not weighted tokens.** In the RP2 pilot four
  Claude arms fell within 0.09–0.19 Mwt while their cost spanned 58 times
  ([RP2 protocol](../metrics/probes/claude-rp2-comprehension-20261019-protocol.md),
  lines 335–342).

### Open questions and the checks that would settle them

| Question | Check | State on 2026-11-07 |
| --- | --- | --- |
| Can Sonnet supervise as well as Opus, for less? | E3 supervisor replay ([experiment plan §6](../metrics/experiment-plan-20261026.md#e3-supervisor-decision-replay)) | fixtures built; arms never run |
| Which reviewer model and structure finds the most defects per dollar? | E2: Sonnet `high` as one reviewer against four concern partitions, with Opus single and Haiku partitioned as strategy comparisons ([experiment plan §6](../metrics/experiment-plan-20261026.md#e2-review-structure-and-model)) | planned; needs 4–6 artifacts with known defects |
| Does the best builder depend on the task type? | E1b: Haiku `high`, Sonnet `medium` and Opus `medium` on T0 tasks ([experiment plan §6](../metrics/experiment-plan-20261026.md#e1-builders-first-artifact-quality-builder-only)) | blocked on fixing 12 cases of the T0 R2-E2 battery |
| Does the second pass catch what Opus tier 1 clears? | Record, per review, the findings each pass made that the other did not, through existing receipts | starts with Claude-only work; no new gate |

Revisit the tier-1 choice if the second pass repeatedly finds P1 or P2 defects
that Opus cleared, or the reverse, or if an outside reader finds blocking defects
after Claude-only acceptance.

## Cost and quota facts that bear on selection

| Fact | Source |
| --- | --- |
| Frozen API prices per million tokens (input, cached input, cache write, output): Terra 1 / 0.05 / 1.25 / 5; Sol 6 2 / 0.2 / 2.5 / 10; Sol 6.1 2 / 0.1 / 2.5 / 10; Astra 10 / 1 / 12.5 / 50; Sonnet 5.5 2 / 0.1 / 2.5 (5-minute) or 4 (1-hour) / 10; Opus 5.5 4 / 0.2 / 5 or 8 / 20. A Codex call over 272,000 input tokens pays 2× on all input (cached input and writes included) and 1.5× on output. On 2026-11-07 OpenAI's [pricing page](https://developers.openai.com/api/docs/pricing) matched the Standard-tier prices for Astra, Sol 6.1 and Terra, including the long-input rule. It also lists a 12-hour cache write at twice the 30-minute price shown here, which the table does not carry. Sol 6 and the Claude prices were not rechecked. | `CODEX_RATES`, `NATIVE_RATES` and `PRICING_RULES` in `metrics/scripts/usage_ledger.py`; OpenAI pricing page, read 2026-11-07 |
| Codex quota tracks API cost: Astra uses about 4.5× Sol's quota per weighted token, and 1% of a Pro week is about $6–7 of API-equivalent spend **Not established for current model-selection use (both claims; owner, 2026-11-07):** the experiment plan §1 fit is `not established` until rechecked with the tier, per F1.1 (`metrics/probes/usage-ledger-f11-20261028.md:60-66`). The values and source are kept as the historical record. Separate context: the 2026-10-31 audit (`~/cbm-work/quota-audit-20261031/findings.json`, sha256 `8a54666c452b6099d29dd16b57d907c394225d39a0eaf8b2fe3b30149b7e2c81`) examined the later `quota-fit-20261031` report, not the §1 fit. | experiment plan §1 (2,319 readings over 5 weekly windows); efficiency ledger, Cost structure |
| 1% of the Claude weekly meter is about $6–12 of API-equivalent spend (rough: whole-percent readings, few intervals) | experiment plan §3 |
| Effort barely changes Astra's and Sol's cost per call, because cached context dominates; total cost still depends on how many calls a task takes | experiment plan §1 |
| In the Codex window measured on 2026-10-26, Astra-run root, supervisor and review work used about 83% of the quota, builders about 9% | experiment plan §1 (roles from thread names, about 10% unclassified) |

API prices do not establish subscription quota for models outside the quota
fit (Terra, Sol 6.1, Fable).

## Session defaults versus assignments

Sessions started without an explicit selection run their client's default. As
read on 2026-11-05, those are `gpt-6.1-sol` at `xhigh` in `~/.codex/config.toml`
(changed from `high` at 04:58Z), and Opus at `xhigh` in `~/.claude/settings.json`.
The same Codex change set `model_auto_compact_token_limit = 250000`, which
applies to sessions started after it and to their children
([context boundary](../skills/manual-orchestration/references/context-boundary.md#measured-basis)).
Delegated assignments always select model and effort explicitly, from this
guide.

For Codex manual orchestration, use `/fast` for the root's
supervisor/builder tree and separate top-level Astra reviewers at the standard
tier, explicitly set with `service_tier="default"` on reviewer launch
([Codex host](../skills/manual-orchestration/references/codex-host.md)); the
probed `/fast` toggle also changed shared configuration. Keep `/fast` off in
campaign supervisors until their review launch and binding support this separation.

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

| Location | What it encodes | State on 2026-11-04 |
| --- | --- | --- |
| `skills/slice-builder/scripts/resolve_provider.py`: CLI defaults (`--builder-model`, `--builder-reasoning-effort`) and the `CURRENT_CODEX_MODELS` allowlist | campaign builder default; which builder and reviewer models are accepted | aligned: `gpt-6.1-sol` / `medium`; the allowlist is `gpt-6.1-terra`, `gpt-6.1-sol`, `gpt-6-sol`, `gpt-6-astra` |
| `skills/slice-supervisor/scripts/campaign-preflight.mjs`: `defaults.builder` and the `currentBuilderModels` allowlist | campaign builder default; preflight rejects a builder model not on its list | aligned: `gpt-6.1-sol` / `medium`; same allowlist |
| `skills/slice-supervisor/references/work-engine-config.md` | documented campaign defaults | aligned: `gpt-6.1-sol` / `medium` |
| `skills/slice-builder/SKILL.md` | default statement and effort ladder | points to this guide |
| `skills/claude-builder/SKILL.md` and `references/astra-review.md`, `references/native-session.md` | Claude builder, successor and reviewer choices | point to this guide; the skill pairs its builder with an Astra reviewer, so it serves Codex-led work only |
| `skills/manual-orchestration/references/anthropic-host.md` | Claude-host role table | points to this guide; the "Claude-host rows" it names are now the [Claude host](#claude-host-and-claude-only-development) section. The host mapping (`Agent` `model`, observed `message.model`) stays there |
| `resolve_builder_context()` signature default in `resolve_provider.py` | internal default for callers that omit the model | stays `gpt-6-sol` / `high` on purpose: production paths pass the model explicitly, and tests use the default to build same-model review cases |
| `skills/slice-supervisor/scripts/append_metrics.py`, `.get("model", "gpt-6-sol")` and `.get("reasoning_effort", "high")` | how a receipt with no recorded builder model is read | stays as is on purpose: receipts written without an explicit model ran under the earlier default |
| `~/.codex/config.toml`, `~/.claude/settings.json` | session defaults | see above |
| `rust/crates/session-bridge/src/provision.rs` admission check and its README | the fixed profile of a read-only diagnostic session | stays `gpt-6.1-terra` / `medium`: a provisioning contract for a read-only diagnostic session, not a builder default; this guide does not change it |

The alignment and pointer edits were applied on 2026-10-27 and moved to the Sol
default on 2026-11-04. On that tree the slice-builder (26 tests, 1 skipped),
slice-supervisor (105), campaign-preflight (6) and manual-orchestration (193)
suites pass.

None of the 9 slice-supervisor campaign files sets `builder.model` or
`builder.reasoning_effort`, so all take the default above. On 2026-11-04 all 9
pass preflight and resolve `gpt-6.1-sol` / `medium`.
7 of them named `gpt-5.6-sol` at `low` until 2026-10-27, when that model left the
allowlists; each records the removal as an amendment. The 4 with a Codex
adversarial reviewer (`agent-instruction-review`, `code-change-characterization-baseline`,
`raw-idea-intake`, `review-remediation`) now name `gpt-6-astra` at `high` with
the different-model evidence class. Their earlier amendments, which record the
owner's approval of a Sol low reviewer, are unchanged.
`campaigns/comparative-repository-analysis.yaml` is a comparative-analysis
configuration with no builder.

## Decisions

| Date | Decision | By |
| --- | --- | --- |
| 2026-10-20 | `gpt-6-sol` at `medium` for builders (superseded) | owner |
| 2026-10-27 | `gpt-6.1-terra` at `medium` as the default builder, provisional (superseded 2026-11-04) | owner |
| 2026-10-27 | `gpt-6.1-sol` at `medium` for hazard-class builder work until Terra is measured there (superseded 2026-11-04) | recommendation; owner may change |
| 2026-10-27 | Campaign builder defaults align to this guide | owner |
| 2026-10-27 | Supervisor moves to `gpt-6.1-sol` at `xhigh` at the next Codex reset, after O2 and as a monitored pilot (superseded 2026-10-28) | owner |
| 2026-10-27 | Adversarial review: `gpt-6-astra` at `high`, `xhigh` for architecture, ownership-contract or open-design subjects | owner |
| 2026-10-28 | The supervisor change follows O2 and a real-slice validation (O4 or E5), not the Codex reset date (superseded the same day) | owner |
| 2026-10-28 | Supervisors and coordinators run `gpt-6.1-sol` at `xhigh`, and so do architecture and open-design planning; `gpt-6-astra` is used only for review | owner |
| 2026-10-28 | Codex manual roots use `/fast` for supervisors and builders; Astra reviewers launch as separate top-level threads with an explicit standard-tier override. The live probe found `/fast` also persists a shared Fast default, so top-level launch alone is insufficient. Campaign supervisors keep `/fast` off pending separate reviewer launch/binding support. | owner; [observed probe](../metrics/probes/codex-manual-service-tier-20261028.json) |
| 2026-10-28 | Clarify Terra/medium as the default for bounded implementation after discovery: select Sol for concrete residual decisions and risk, not domain labels, the planner's model, or a repair-round count alone. (Superseded 2026-11-04.) | guidance clarification following owner feedback |
| 2026-11-04 | `gpt-6.1-sol` at `medium` is the default builder; `gpt-6.1-terra` at `medium` narrows to small, mechanical, fully specified changes. Campaign defaults align. | owner; [round 2 outcomes](../metrics/probes/dogfood-round2-builder-outcomes-20261104.md) |
| 2026-11-07 | Claude-host choices move into their own [section](#claude-host-and-claude-only-development) for Claude-only development. Added: Sonnet 5.5 `medium` for Claude reconnaissance and mechanical edits; a fresh Sonnet or Haiku second review pass beside Opus tier 1; Claude-only review recorded as same-provider. Corrected: the tier-1 reviewer row said no matched Sonnet review comparison existed (Test R generation 2 is one, n = 1 per model), and the Sonnet builder row's "Opus" was `claude-opus-5`, not 5.5. | owner requested the section; the two additions are recommendations the owner may change |
| 2026-11-07 | Claude-only development runs without an outside non-Claude reader: the Codex quota is spent. Review there is recorded as same-provider, and a requirement for an outside reader is recorded as unmet, not satisfied. (Superseded the same day.) | owner |
| 2026-11-07 | An outside reader is available: `gpt-6-astra` from a separate Codex home on OpenAI API credits (USD 100, auto-recharge off), used per objective with owner approval. Review without it is still recorded as same-provider. | owner; [probe](#claude-host-and-claude-only-development) session `01a1b6fb` |
