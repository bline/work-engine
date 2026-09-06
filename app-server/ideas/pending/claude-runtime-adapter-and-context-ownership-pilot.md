# Claude Runtime Adapter and Context-Ownership Pilot

## Status

Adopted raw architectural direction awaiting independent intake and proposal
formation. This document does not authorize implementation, roadmap
priority, or pilot execution.

## Objective

Add Claude as a second realization of the provider-neutral runtime-adapter
boundary already established by `ideas/runtime-adapter.md` and demonstrated
for Codex by `ideas/codex-app-server-scaffold-and-role-port.md` (both still
in the legacy top-level `ideas/` tree pending migration to this directory).
Separate two decisions
that are easy to conflate:

1. **Production integration path** — which Claude execution mechanism backs
   role turns today.
2. **Context-ownership question** — whether Work Engine-owned continuation
   state measurably improves long-horizon Claude execution over native
   context management, and under which conditions.

The second question is open research. The first should not wait on it.

## Provider boundary

No new top-level abstraction. `ManifestRoleRuntime.deliverTurn` already calls
exactly one interface — `deliverTurn` / `waitForTurnCompletion` /
`runEphemeralTurn` — and `CodexAppServerAdapter` is already one
implementation behind it. `ideas/runtime-adapter.md`'s compact statement governs
here unchanged:

> Runtime adapters answer where and how computation is executing; Work
> Engine remains the owner of what that computation means.

`ClaudeRuntime` is a second implementation of the same interface, not a
sibling ontology. Role code (`StrategicPlannerRuntime`, `SliceBuilderRuntime`,
future roles) requires zero changes to gain Claude support. Any design that
introduces a parallel `AgentRequest` / `AgentRuntimeAdapter` type duplicates
`ideas/runtime-adapter.md`'s boundary and is out of scope for this idea.

## Why not target the Claude Code CLI directly

Investigated and rejected as the primary integration surface:

- It is a closed, compiled binary (confirmed: the public `claude-code`
  GitHub repository contains no CLI implementation, only docs/plugins/
  scripts; the actual harness ships as a ~200MB compiled executable).
- Its native context management (auto-compaction, "auto memory," memory
  stores) is real, mostly on by default, and only partially controllable —
  confirmed directly from the installed binary (`autoCompactEnabled`,
  `CLAUDE_CODE_DISABLE_AUTO_MEMORY`, `CLAUDE_MEMORY_STORES` gating).
- Its one genuinely strong context-lifecycle interface — `PreCompact` /
  `PostCompact` / `SessionStart[source=compact]` hooks, with `additionalContext`
  injection — is real and confirmed (schema found directly in the binary,
  and observed live in this session's own `SessionStart` hook), but it acts
  only through hooks on an otherwise opaque process, not through code Work
  Engine owns or can read.
- Anthropic states the Claude Agent SDK runs the **same execution loop,
  tools, and context management as Claude Code** — moving from CLI
  subprocess invocation to the SDK does not, by itself, buy independent
  context management. It buys a nicer, in-process control surface over the
  same harness family.

## Execution paths considered for `ClaudeRuntime`

| Path | What it gives | What it withholds |
|---|---|---|
| **Agent SDK** (native) | Same harness as Claude Code, in-process, no subprocess/JSON-RPC boundary — the tightest integration of any adapter in this codebase, including Codex's | Compaction policy remains Anthropic's; not independently confirmed whether hooks are configurable programmatically vs. only via `settings.json`, or whether they fire under headless invocation (see Pilot 0) |
| **Agent SDK + hooks** (overlay) | Everything above, plus `PreCompact`/`PostCompact`/`SessionStart[compact]` wired to Work Engine's own continuation state, making the native summary a fallback rather than sole continuity mechanism | Still physically carries Anthropic's summary in the transcript; injection happens after compaction completes, not before |
| **Raw Messages API + SDK tool runner + `context_management`** | Full host control: `instructions` fully replaces (not appends to) the compaction prompt; `pause_after_compaction` halts generation before continuation resumes, giving true pre-use intervention. Confirmed real and unrestricted at the CLI/API level (`anthropic-cli`'s `betamessage.go:211-217`, `context-management.edits: []map[string]any`, fully untyped passthrough) | Loses Claude Code's tool implementations and any proprietary behavioral engineering in its system prompt/tool descriptions; no free session durability |
| **Managed Agents** (`beta:agents`/`beta:sessions`) | Durable, server-resident sessions — the closest Claude analog to Codex's daemon-attach model; advisor model, memory-store mounting, outcome/rubric grading all confirmed real (`anthropic-cli` `betamessage.go`, `betasession.go`, `betasessionevent.go`) | Compaction is observable (`agent.thread_context_compacted`-class events exist) but not overridable — trades one opaque harness for another |

Managed Agents is a **comparison condition**, not a candidate for
`ClaudeRuntime`'s production path. It puts the exact layer this idea studies
back behind a provider boundary Work Engine cannot see into. It may separately
be worth investigating for reasons unrelated to context lifecycle — e.g.
durable multi-process attach to a live thread — but that is a distinct
question, tracked outside this idea, not folded into it.

## Production default

Ship the **overlay** (Agent SDK + hooks), not bare native, as the default
from day one. Two reasons this costs approximately nothing beyond native:

1. The hook wiring (`PreCompact` capture, `SessionStart[compact]`
   reinjection) is required infrastructure to *measure* the pilot below.
   Building it only for the pilot and not for production is redundant work,
   not a saved step.
2. Shipping bare native means Claude-backed roles run, for however long the
   pilot takes, without the host-owned-canonical-state guarantee already
   established for Codex (`ideas/codex-app-server-scaffold-and-role-port.md`:
   "a thread is a durable reasoning runtime... not the canonical owner of
   workflow state"). That is a real, if temporary, doctrine regression with
   no offsetting benefit.

**Structural requirement:** the overlay must be built as the native adapter
plus a composable hook layer — not a parallel `ClaudeRuntime` implementation
that happens to agree with native today. If they are separate
implementations, they will drift; if the overlay is native-plus-hooks, there
is nothing to reconcile regardless of what the pilot concludes.

## Pilot 0 — CLI-to-SDK admission test

Before anything else lands: confirm the Agent SDK invocation path does not
regress behavior relative to the CLI invocation `native-claude-code-adapter.mjs`
already uses today. Not a statistical-superiority test — an admission test.

- 8–12 representative role packets (drawn from existing role/skill contracts,
  not synthetic).
- Same model, provider route, tools, repository state, developer
  instructions, and budget across both invocation paths.
- **Explicit added criterion, beyond what was originally scoped:** confirm
  `PreCompact` / `PostCompact` / `SessionStart[compact]` hooks fire and carry
  the expected payload shape under the SDK's headless/programmatic invocation
  path specifically — not just under the interactive CLI. This was flagged
  and left unconfirmed twice already in this research thread (Agent SDK
  options docs do not enumerate an auto-compact control, and whether hooks
  fire in one-shot `-p`-style invocation was never directly confirmed). Both
  the overlay and the context-owned path depend on this; if it fails, that is
  a finding Pilot 0 should surface before either downstream path is built on
  top of an assumption that doesn't hold.
- Environment note: neither the SDK path nor the raw-Messages path is
  currently runnable in this repository — no `@anthropic-ai/sdk` or Claude
  Agent SDK dependency is installed (Node or Python), and no
  `ANTHROPIC_API_KEY` is set. The CLI path already works today via the
  existing subscription OAuth credentials `native-claude-code-adapter.mjs`
  copies into an isolated `CLAUDE_CONFIG_DIR`. Provisioning the SDK dependency
  and credentials (or confirming the SDK can reuse the existing CLI OAuth
  session rather than requiring a separate API key) is a prerequisite this
  document does not resolve.

## Claude Harness and Context-Ownership Pilot

**Testable proposition:**

> Does Work Engine-owned continuation state improve long-horizon Claude
> execution across context transitions relative to native Claude context
> management, without degrading performance when context pressure is absent?

**Non-claims:** this pilot does not establish that Anthropic's harness
engineering is inferior in general, that Managed Agents is the wrong
architecture for problems other than context lifecycle, or that any effect
found here transfers to non-Claude models. It also does not claim novelty of
the general mechanism — see prior art below — only that it has not been
measured against Work Engine's own role workloads and fidelity criteria.

### Prior art

Independently verified against primary sources during this research thread
(not taken on secondhand summary):

- [Lewis, *Same Model, Different Harness: Different Coding-Agent Results*
  (arXiv:2608.26218)](https://arxiv.org/abs/2608.26218) — holding model
  weights, tasks, tools, evaluator, and execution protocol fixed, a
  deterministic (non-LLM-generated) change to how old tool results are
  represented raised SWE-bench Verified complete solutions from 43/169 to
  72/169 (mean fail-to-pass 28%→49%) under a 20,480-token limit. At a
  262,144-token limit the effect vanished (−0.3pp, 95% CI crossing zero).
  The frozen treatment transferred without retuning to three further models
  (Qwen3.6, Nemotron, Qwen3.8, Devstral 22→53). The paper's own stated
  conclusion is narrower than the headline numbers suggest: *"coding-agent
  evaluations should treat the model and harness together as the tested
  solver,"* not a causal decomposition of the mechanism.
- [Anthropic, *Harness design for long-running application
  development*](https://www.anthropic.com/engineering/harness-design-long-running-apps)
  and [*Scaling Managed Agents: Decoupling the brain from the
  hands*](https://www.anthropic.com/engineering/managed-agents) — Sonnet 4.5
  exhibited "context anxiety" (premature task wrap-up near its perceived
  context limit) severe enough that compaction alone was insufficient;
  Anthropic's harness added context resets to compensate. Opus 4.5 in the
  same harness did not exhibit the behavior, making the reset mechanism dead
  weight for that model — direct evidence of a model×harness-policy
  interaction, not a model-quality-only effect.
- [AIMultiple, AI Agent Platforms
  Benchmark](https://aimultiple.com/ai-agent-platforms) — Sonnet 4.6 under
  Claude Managed Agents vs. a ~150-line local Messages API loop, 10 tasks ×
  3 runs: both 30/30 task success; Managed Agents used 93k tokens vs. 464k
  for the local loop (1,172s vs. 794s wall time; $2.50 vs. $1.96). Includes a
  compaction canary test (plant a UUID in turn 1, 23 unrelated padding turns,
  recall on turn 25 with no file lookup) directly reusable as a fidelity
  probe design.
- [TrueFoundry, *TrueForge vs. Claude Managed Agents
  benchmark*](https://www.truefoundry.com/blog/engineering/trueforge-vs-claude-managed-agents-benchmark/)
  ([VentureBeat coverage](https://venturebeat.com/orchestration/truefoundrys-open-source-ai-agent-harness-trueforge-boasts-30-75-cheaper-task-completion-than-claude-managed-agents)) —
  Opus 4.8, 14 Enterprise-Bench cross-system tasks, identical MCP tools,
  blind grading: TrueForge and Claude Managed Agents each solved ~11/14;
  TrueForge used ~40% as many tokens ($8.50 vs. $11.80). Vendor-produced,
  weighted accordingly, but corroborated by independent outlets.

None of these measured Work Engine's actual role workloads
(agent-instruction-review, strategic planning, slice-building), which
resemble SWE-bench-style coding tasks only loosely. They establish that the
mechanism class is real and the magnitude can be large under pressure — not
that it transfers here. That is what this pilot tests.

### Conditions

Harness treatment:

```text
H0 — Native:            Agent SDK, native Anthropic context management
H1 — Governed overlay:  Agent SDK, native compaction + Work Engine canonical
                         continuation state reinjected via SessionStart[compact]
H2 — Governed context:  SDK tool runner + Messages API, Work Engine's own
                         context compiler and compaction/state policy
```

Crossed with context pressure:

```text
C0 — context abundant       (mirrors Lewis's 262,144-token null condition)
C1 — context constrained    (mirrors Lewis's 20,480-token treatment condition;
                              must actually trigger context-pressure-controller.mjs's
                              replacement_candidate/critical disposition, not merely
                              use a "small" token budget)
```

Interpretation, matching the logic Lewis's own design supports:

- H2 beats H0 only in C1 → the context treatment works, and only where it's needed.
- H2 beats H0 in both → a generally superior scaffold, not just a compaction fix.
- H2 better in C1 but worse in C0 → a real tradeoff to weigh, not a clean win.
- **H1 captures nearly all of H2's C1 benefit → do not replace the harness.**
  Given H1 is the already-shipping production default, this is the outcome
  that costs the least to discover and the most to have missed.

### Fidelity metrics (in addition to task success)

Preservation of: objective and acceptance criteria; decisions *and their
stated reasons*; unresolved work; correct continuation after the boundary.
Failure modes to score explicitly: contradictory or reopened decisions,
unnecessary rediscovery/re-reading, premature completion. Cost: input/output/
tool tokens, wall time, tool call count, operator intervention count.

**Deliberately include state that only becomes relevant after the context
transition.** Without this, the pilot measures ordinary task quality, not
continuity — a role that never needs its preserved facts back cannot
discriminate between conditions.

**Treatment-fidelity gate, checked before any condition-outcome is
interpreted:** did the intended context transition actually occur; did the
correct canonical state get injected; did every condition receive materially
equivalent task evidence. A condition that silently failed to trigger its
own mechanism is an implementation bug, not evidence about the mechanism.

## Relationship to neighboring owners

- `ideas/runtime-adapter.md` owns the general provider-neutral execution
  boundary; this idea supplies Claude as its second concrete realization,
  exactly as `ideas/codex-app-server-scaffold-and-role-port.md` supplied
  Codex as the first. Neither idea redefines the other's
  adapter-responsibility boundary. Both source docs still live in the
  legacy top-level `ideas/` tree; this doc is written under
  `app-server/ideas/pending/` per the migration to `app-server/` as root,
  and the cross-references above should be re-pointed if/when those two
  docs are themselves migrated.
- `app-server/docs/skills-migration-plan.md` owns the campaign this idea
  must fit inside, not the other way around. Two specific things it already
  established take precedence over anything written here:
  - The corrective `supervisor-native-review-hosting` slice (recorded under
    "Post-S12E operational inhabitation correction") already put a
    **production** Claude path in place: "direct Anthropic through the
    native Claude Code harness," scoped to one mediated, already-selected
    reviewer obligation — the supervisor receives no shell, filesystem,
    credential, model-routing, or acceptance authority through it. That
    decision is accepted and implemented. This idea does not revisit or
    compete with it; `ClaudeRuntime` is a general-purpose provider-adapter
    question for role turns broadly, not a proposal to replace that narrow,
    already-closed boundary. If Pilot 0 or the later pilot ever produced
    evidence that bears on that specific slice's own decision, that would
    need to re-enter through the campaign's own acceptance process, not
    this idea's authority.
  - S13 ("Reviewer research and admission evidence") is in progress and its
    candidate-responsibility sequence already names, as item 2, "an
    immutable provider-neutral reviewer-turn event schema and normalizers
    for Codex, OpenRouter, Claude, Gemini, and later providers." Everything
    in this idea's "Execution paths considered" section is directly
    relevant evidence for that item, not a parallel or competing effort.
    Whichever `ClaudeRuntime` path this idea eventually recommends should
    be read as a candidate input to S13 item 2's normalizer work, not as a
    new slice outside the campaign's existing dependency graph.
- `context-transition-lease.mjs`, `context-pressure-controller.mjs`, and
  `context-lifecycle-evidence.mjs` already own the host-side pressure/ledger
  machinery. This idea adds a Claude-shaped observation source
  (`claude-lifecycle-notification-source.mjs`, by analogy to the existing
  Codex one) and a Claude-shaped `MODEL_CONTEXT_REPLACEMENT_CAPABILITY`
  profile in `capabilities.mjs` — it does not redesign the ledger or
  disposition ladder.
- `ideas/codex-app-server-scaffold-and-role-port.md`'s own unresolved question,
  "the Claude-versus-Codex independent-review strategy," is upstream of but
  not answered by this idea; this idea concerns Claude as a general-purpose
  role backend, not specifically the independent-review provider question.

## Unresolved questions

- Whether the Agent SDK exposes hooks as programmatic callbacks distinct
  from `settings.json`-defined shell hooks, and whether either form fires
  under headless/programmatic invocation — gates Pilot 0.
- Whether a JS/TS equivalent of `anthropic-sdk-go`'s `agenttoolset` exists,
  or whether H2's Bash/Read/Write/Edit/Glob/Grep tools must be built from
  scratch.
- The exact capability-negotiation shape for three `ClaudeRuntime` variants
  under one profile, given `capabilities.mjs`'s existing
  `MODEL_CONTEXT_REPLACEMENT_CAPABILITY` assumes a single `mechanism`/
  `invocation` pair per provider; H0/H1/H2 have three different mechanism
  shapes (opaque-and-unobserved, hook-triggered-host-owned, and
  API-parameter-mid-generation-pause respectively).
- Whether Managed Agents' durable-session model is worth investigating on
  its own terms (independent of context lifecycle) — tracked separately,
  not as part of this idea.

## Non-authorization

This raw idea does not authorize implementation, dependency additions,
credential provisioning, pilot execution, changes to existing role
contracts, or roadmap priority. Those consequences remain with their owning
workflows.
