# Idea: OpenCode Substrate Evaluation

**Status:** Idea / reconnaissance candidate  
**Decision type:** Candidate implementation evaluation  
**Architecture dependency:** [Provider Turn, Harness Runtime, and Operator Projection](./provider-turn-harness-runtime-and-operator-projection.md)  
**Primary question:** Which independently admitted Work Engine ports, if any, can OpenCode implement without acquiring Work Engine orchestration, lifecycle, or state authority?

## Summary

Evaluate OpenCode as one possible implementation source for Work Engine's
provider-turn, harness-runtime, and operator-projection ports rather than
treating it as a wholesale replacement for either Work Engine or Codex App
Server.

The provider-neutral port architecture is owned by
[Provider Turn, Harness Runtime, and Operator Projection](./provider-turn-harness-runtime-and-operator-projection.md).
This document owns only the OpenCode-specific evidence, admission questions,
bounded pilots, and unresolved maintenance tradeoffs. Any architecture diagrams
or capability classifications retained below are evaluation context, not a
competing definition of the ports.

The promising architectural observation is that OpenCode appears to separate several concerns that Work Engine currently receives together from Codex:

- model/provider integration,
- model-turn execution,
- session/runtime management,
- context management,
- server/API transport,
- generated client SDK,
- and terminal operator interface.

If these boundaries are sufficiently open, Work Engine may be able to reuse substantial OpenCode infrastructure while continuing to own the semantics that matter most:

- orchestration,
- supervisor/builder topology,
- context lifecycle,
- authority boundaries,
- durable workflow state,
- admission/reconciliation rules,
- and provider- or harness-specific capabilities.

The key constraint is therefore not:

> Can OpenCode run Work Engine agents?

It is:

> Can Work Engine retain full context-lifecycle and orchestration authority while using OpenCode's provider, runtime, server, SDK, and TUI machinery?

If the answer is yes, OpenCode may eliminate a large amount of infrastructure that Work Engine would otherwise have to build and maintain independently.

---

# 1. Motivation

Work Engine currently benefits from Codex App Server as much more than a model API.

Codex supplies a runtime abstraction containing capabilities such as:

- long-lived threads/sessions,
- turns and streaming events,
- tool execution,
- shell execution,
- approvals,
- file changes,
- cancellation,
- usage information,
- context lifecycle operations,
- and an existing terminal client.

As Work Engine expands toward multiple model providers and multiple coding harnesses, Codex App Server creates two strategic limitations.

First, it is primarily tied to the Codex/OpenAI runtime.

Second, important Work Engine semantics risk becoming coupled to capabilities supplied by one proprietary harness.

A future multi-agent system may need to support combinations such as:

```text
Work Engine
    |
    +-- Codex
    |
    +-- Claude / Claude Code
    |
    +-- OpenAI API
    |
    +-- Anthropic API
    |
    +-- OpenRouter
    |
    +-- local or future providers
    |
    +-- specialized agent runtimes
```

Implementing provider transport, event normalization, terminal UI, session plumbing, model configuration, tools, MCP, LSP, cancellation, and related infrastructure separately for every path would create substantial maintenance cost.

OpenCode may already contain much of this generic infrastructure.

---

# 2. Architecture Criteria Applied to OpenCode

The provider-neutral architecture is defined in
[Provider Turn, Harness Runtime, and Operator Projection](./provider-turn-harness-runtime-and-operator-projection.md).
The classifications below are retained as the evaluation criteria used to
separate OpenCode mechanisms from Work Engine authority.

Work Engine should not adopt a lowest-common-denominator abstraction that hides capabilities of the underlying model or harness.

Instead, distinguish three classes of capability.

## 2.1 Work Engine capabilities

These remain owned by Work Engine.

Examples:

- planning,
- orchestration,
- supervisor topology,
- builder assignment,
- authority and governance,
- durable workflow state,
- context lifecycle policy,
- pause/reconcile/resume semantics,
- checkpointing,
- handoff policy,
- workspace coordination,
- admission rules,
- quality gates.

These define Work Engine itself.

## 2.2 Harness capabilities

These belong to an agent harness/runtime rather than directly to the model provider.

Examples may include:

```text
Codex App Server
    new_context
    thread/turn lifecycle
    harness-native tools
    sandbox/approval behavior
    runtime-specific continuation

Claude Code
    Claude Code context lifecycle
    harness-native tools
    Claude Code session semantics
    harness-specific compaction/context behavior
```

These should remain accessible through specialized runtime adapters whenever their semantics cannot be reproduced through a generic model API.

## 2.3 Provider capabilities

These belong directly to the underlying inference provider.

Examples may include:

```text
Anthropic
    context_management
    context editing
    provider-native usage metadata
    prompt caching

OpenAI
    Responses API
    reasoning controls
    prompt-cache controls
    response IDs
    provider-native tools

Other providers
    provider-specific request fields
    provider-specific streaming events
    provider-specific context features
```

A provider abstraction should normalize what is genuinely portable while retaining escape hatches for provider-native features.

---

# 3. OpenCode Candidate Mapping

Within the provider-neutral architecture, the OpenCode candidate was initially
mapped approximately as follows:

```text
                         Work Engine
                             |
                +------------+-------------+
                |                          |
         Orchestration              Context Lifecycle
            Authority                   Authority
                |                          |
                +------------+-------------+
                             |
                     Runtime Capability API
                             |
        +--------------------+--------------------+
        |                    |                    |
        v                    v                    v
 Codex adapter        Claude harness       Generic provider
        |                 adapter              runtime
        |                    |                    |
 Codex App Server      Claude Code?       OpenCode LLM layer
        |                    |                    |
   Codex models          Claude models      +------+------+
                                            |      |      |
                                           GPT   Claude  Other
```

OpenCode would therefore not necessarily replace every specialized runtime.

Instead, it could become the generic substrate beneath them.

Over time, specialized adapters could be collapsed into the generic substrate only where equivalence has been demonstrated.

---

# 4. Why OpenCode Is Interesting

Repository verification is required, but current reconnaissance suggests OpenCode may provide several useful layers independently.

## 4.1 Server layer

OpenCode appears to expose a server API covering functionality such as:

- sessions,
- messages,
- async prompts,
- event streaming,
- cancellation,
- shell execution,
- permissions,
- diffs,
- file operations,
- model/provider discovery,
- MCP,
- LSP,
- formatter integration,
- agents,
- configuration.

This makes it potentially comparable to Codex App Server as an application-runtime substrate rather than merely as a CLI.

## 4.2 SDK layer

OpenCode appears to expose a generated SDK around its server API.

If this boundary is stable enough, Work Engine could integrate against the SDK rather than internal OpenCode implementation details.

This should be preferred wherever possible.

## 4.3 Provider/LLM layer

A particularly interesting component appears to be:

```text
@opencode-ai/llm
```

Current reconnaissance suggests this layer separates a single model/provider turn from higher-level OpenCode session orchestration.

It appears to provide:

- normalized portable generation options,
- provider-specific options,
- provider-specific protocol implementations,
- and raw request overlays or equivalent escape hatches.

If confirmed, this may be more strategically important than adopting OpenCode's complete session runtime.

Work Engine could potentially use this layer as its generic provider substrate.

## 4.4 TUI

OpenCode already supplies a functional terminal UI.

It also supports an architecture in which a TUI can attach to an independently running OpenCode server.

Current repository work additionally appears to be extracting or modularizing the TUI behind an SDK-only domain boundary.

If confirmed, this could eliminate the immediate need to build a Work Engine terminal interface from scratch.

Potential adoption levels:

### Level 1 — use stock OpenCode TUI

Use the existing TUI as the first operator console with little or no modification.

### Level 2 — extend or fork OpenCode TUI

Add Work Engine-specific surfaces such as:

- orchestration topology,
- planner status,
- supervisor status,
- builder activity,
- work queues,
- leases,
- context epochs,
- lifecycle transitions,
- `:we agents`,
- approvals,
- reconciliation state.

This is currently the most attractive target if OpenCode exposes a sufficiently clean extension boundary.

### Level 3 — build a Work Engine TUI using OpenCode components

Only build a dedicated UI if Work Engine's operator model eventually diverges enough from coding chat that the OpenCode interaction model becomes restrictive.

Even in this case, reusable OpenCode/OpenTUI components could reduce implementation cost.

---

# 5. Context Lifecycle Is the Critical Admission Criterion

The most important question is whether adopting OpenCode would force Work Engine to surrender control over context lifecycle.

That must not happen.

Work Engine has already made context lifecycle a first-class architectural concern.

Relevant semantics include:

- explicit context boundaries,
- `new_context`,
- stable snapshots,
- handoff,
- reconciliation,
- successor context creation,
- resumption after context replacement,
- and durable workflow continuity across context changes.

An abstraction that silently owns compaction or exposes only a generic summarized conversation would be insufficient.

The desired invariant is:

> Work Engine determines when and why a context transition occurs. Runtime/provider layers implement that transition using the strongest available native mechanism.

For example:

```text
Work Engine lifecycle decision
             |
             v
    capability resolution
             |
       +-----+------+
       |            |
       v            v
Codex runtime   Anthropic provider
new_context     context_management
       |            |
       +-----+------+
             |
             v
  lifecycle result reported
      back to Work Engine
```

The mechanism may vary.

The lifecycle meaning must not.

---

# 6. OpenCode Context Model

Current reconnaissance suggests OpenCode itself has a concept resembling a context epoch.

Its context architecture appears to include concepts such as:

- immutable or baseline system context,
- projected conversational context,
- compaction,
- context snapshots,
- and new context epochs after compaction.

This conceptual overlap is encouraging.

However, compatibility of concepts is not sufficient.

The important distinction is:

```text
OpenCode owns context lifecycle
```

versus:

```text
Work Engine owns context lifecycle
and OpenCode executes requested operations
```

The latter is required.

OpenCode's own session abstraction may currently be opinionated about compaction and context reconstruction.

Therefore Work Engine should not initially depend on OpenCode session semantics for lifecycle correctness.

---

# 7. Anthropic Context Management

Anthropic is a useful test case because the underlying provider exposes explicit context-management functionality.

The important question is whether OpenCode's provider layer preserves access to those native capabilities.

The desired path is:

```text
Work Engine
    |
    | lifecycle command
    v
provider adapter
    |
    v
OpenCode LLM/provider layer
    |
    | provider-native request
    v
Anthropic API
    |
    | context-management result
    v
provider adapter
    |
    v
Work Engine lifecycle state
```

Current reconnaissance suggests OpenCode's low-level LLM abstraction may permit native provider fields through provider-specific options or raw HTTP/body/header overlays.

If confirmed, request-side access to Anthropic context-management features may be straightforward.

The larger question is response-side fidelity.

Work Engine may need information about:

- which context edit was applied,
- what material was removed,
- resulting token state,
- provider-specific context-management metadata,
- failure or fallback behavior.

If OpenCode normalizes these fields away, a small extension to its provider protocol layer may be necessary.

Because OpenCode is open source, this is potentially acceptable if the extension boundary is clean.

---

# 8. Codex `new_context` Is a Different Capability Class

Do not assume that OpenCode's OpenAI provider can reproduce Codex `new_context`.

These are different abstraction levels.

```text
OpenAI model API
        !=
Codex harness/runtime
```

Codex App Server's `new_context` capability belongs to the Codex execution harness.

It is therefore a harness capability, not merely an OpenAI provider feature.

Initial architecture should retain:

```text
CodexRuntimeAdapter
    -> Codex App Server
    -> new_context
```

even if generic OpenAI provider calls move behind OpenCode.

Only remove the specialized Codex adapter if later testing proves that equivalent Work Engine lifecycle semantics can be implemented another way.

---

# 9. Do Not Flatten Harness and Provider Capabilities

Avoid an abstraction such as:

```text
ModelProvider {
    prompt()
    compact()
    tools()
}
```

if it implies every runtime offers equivalent versions of the same operations.

Prefer capability discovery.

Conceptually:

```text
RuntimeCapabilities {
    inference
    native_context_transition?
    context_editing?
    continuation?
    tool_runtime?
    sandbox?
    approval_protocol?
    native_compaction?
    context_usage?
}
```

A runtime/provider adapter can expose supported capabilities without pretending unsupported capabilities exist.

Work Engine then resolves policy to mechanism.

Example:

```text
requested lifecycle transition: FRESH_CONTEXT

candidate mechanisms:

1. Codex native new_context
2. Claude harness-native context replacement
3. Anthropic provider context-management operation
4. Work Engine reconstruction into a fresh provider session
5. unsupported
```

The Work Engine policy layer chooses among admitted mechanisms.

---

# 10. OpenCode Should Not Become the Orchestrator

OpenCode also supports agents/subagents.

That does not mean Work Engine should delegate orchestration semantics to OpenCode.

The preferred authority boundary remains:

```text
Planner
   |
   v
Orchestrator
   |
   +----------+----------+
   |          |          |
   v          v          v
Supervisor Supervisor Supervisor
   |          |          |
Builders    Builders    Builders
```

OpenCode should sit beneath that system.

Conceptually:

```text
Work Engine
    owns:
        orchestration
        workflow semantics
        agent authority
        context policy

OpenCode
    supplies:
        provider transports
        model execution
        normalized events
        generic runtime functionality
        server/API
        SDK
        TUI infrastructure
```

OpenCode's own agent abstractions may still be reusable as execution primitives, but they should not become the source of Work Engine's orchestration truth.

---

# 11. Candidate Migration Shape

Do not begin with a full Codex App Server replacement.

Adopt incrementally.

## Phase A — reconnaissance

Map every capability currently consumed from Codex App Server.

Classify each as:

```text
WORK_ENGINE
HARNESS
PROVIDER
GENERIC_RUNTIME
OPERATOR_UI
```

Then locate the corresponding OpenCode implementation or determine that none exists.

## Phase B — generic provider pilot

Attempt to execute a representative Work Engine builder through `@opencode-ai/llm` or the lowest suitable OpenCode provider layer.

Require:

- streaming,
- tool calls,
- usage data,
- cancellation,
- provider options,
- provider-native escape hatch,
- deterministic event mapping into Work Engine.

Do not change lifecycle ownership.

## Phase C — context capability pilot

Exercise at least:

### Codex

Existing:

```text
Work Engine -> Codex adapter -> new_context
```

Use as control.

### Anthropic

Attempt:

```text
Work Engine
 -> OpenCode provider layer
 -> Anthropic native context-management feature
```

Verify both request and response semantics.

### Generic fallback

Test Work Engine-managed reconstruction into a fresh session where no native context operation exists.

Compare the observable lifecycle properties of all three.

## Phase D — OpenCode server pilot

Test whether OpenCode server/SDK can replace generic App Server plumbing without taking lifecycle authority.

Candidate functionality:

- sessions,
- streaming,
- shell,
- tools,
- file changes,
- cancellation,
- permissions,
- model selection,
- MCP/LSP,
- diffs,
- usage.

## Phase E — operator UI pilot

Run Work Engine against an OpenCode server while using the stock OpenCode TUI.

Determine what is missing from the operator experience.

Only then decide whether to:

- keep stock TUI,
- extend/fork TUI,
- or create a Work Engine-specific TUI.

---

# 12. Verification Tasks for the OpenCode Repository

The checked-out repository should be treated as authoritative.

Verify the following claims before architecture admission.

## Provider layer

Determine whether `@opencode-ai/llm` or its current equivalent:

- exists as an independently usable package,
- can execute provider turns without OpenCode session ownership,
- exposes provider-specific options,
- permits arbitrary provider request fields,
- permits arbitrary headers,
- preserves raw or extensible provider response metadata,
- allows provider protocol extensions without forking large parts of OpenCode,
- supports Anthropic and OpenAI directly,
- supports OpenRouter or provider-compatible endpoints.

## Anthropic

Inspect the Anthropic protocol implementation.

Determine:

- whether native `context_management` fields are currently supported,
- whether they can be injected,
- whether required beta headers can be supplied,
- whether context-management response fields survive normalization,
- what changes would be required to preserve those response fields,
- whether tool-use/context-editing semantics are preserved correctly.

## Session/context layer

Determine:

- how OpenCode constructs model context,
- when compaction occurs,
- who decides to compact,
- whether auto-compaction can be disabled,
- whether callers can start a fresh context/session explicitly,
- whether the complete provider request context can be inspected,
- whether context epochs are externally visible,
- whether lifecycle decisions can be overridden by the caller.

## Server/SDK

Map the current API for:

- create/resume/fork session,
- messages,
- async model execution,
- event streaming,
- cancellation,
- tool events,
- shell execution,
- file mutations,
- approvals/permissions,
- diffs,
- usage/tokens,
- models/providers,
- MCP,
- LSP,
- formatting,
- agents.

Compare these with the subset of Codex App Server currently consumed by Work Engine.

## TUI

Verify:

- whether the TUI can attach to a separately running server,
- whether server URL/client injection is supported,
- whether the TUI now depends only on the public SDK,
- status of the standalone TUI package,
- extension/plugin surfaces,
- route registration,
- custom commands,
- notifications,
- session/model controls,
- feasibility of Work Engine-specific panels or views.

Determine whether Work Engine can add operator features without maintaining a large permanent fork.

---

# 13. Capability Matrix to Produce

Reconnaissance should produce a matrix similar to:

| Capability | Codex App Server | OpenCode Server | OpenCode LLM | Specialized Adapter Required? |
|---|---|---|---|---|
| Streaming inference | yes | ? | ? | |
| Session persistence | yes | ? | n/a | |
| Cancellation | yes | ? | ? | |
| Tool execution | yes | ? | ? | |
| Shell | yes | ? | ? | |
| File mutations | yes | ? | ? | |
| Approval protocol | yes | ? | ? | |
| Diff reporting | yes | ? | ? | |
| Token/usage data | yes | ? | ? | |
| MCP | yes | ? | ? | |
| LSP | harness-dependent | ? | n/a | |
| Native fresh context | Codex `new_context` | ? | provider-dependent | |
| Anthropic context editing | n/a | ? | ? | |
| Provider-native options | limited by Codex | ? | ? | |
| Raw request escape hatch | n/a | ? | ? | |
| Provider response metadata | Codex-specific | ? | ? | |
| Existing TUI | Codex CLI | ? | n/a | |
| TUI extension boundary | limited | ? | n/a | |

Unknown cells should remain unknown until verified from source or execution.

---

# 14. Required Invariants

Any OpenCode integration must preserve these invariants.

## Context authority

Work Engine decides lifecycle transitions.

No runtime may silently redefine a Work Engine context transition.

## Capability fidelity

Native provider/harness capabilities remain reachable where they materially affect agent quality or lifecycle correctness.

## No false portability

A generic abstraction must not imply equivalence between operations that have different semantics.

## Durable orchestration ownership

Planner/orchestrator/supervisor/builder state remains Work Engine state.

## Replaceable substrate

OpenCode should remain replaceable beneath the Work Engine adapter boundary.

## Observable lifecycle

Work Engine must be able to determine:

- which context mechanism was used,
- whether it succeeded,
- what new lifecycle state exists,
- and enough metadata to reconcile the successor correctly.

---

# 15. Non-Goals

This investigation is not intended to:

- replace Work Engine orchestration with OpenCode agents,
- immediately remove Codex App Server,
- immediately replace Claude Code,
- force every provider through one lowest-common-denominator API,
- delegate Work Engine context policy to OpenCode,
- or build a new CLI before determining whether OpenCode's TUI is sufficient.

---

# 16. Possible End-State

A favorable outcome could look like:

```text
                           Work Engine
                               |
             +-----------------+-----------------+
             |                                   |
      Orchestration                       Lifecycle Policy
             |                                   |
             +-----------------+-----------------+
                               |
                       Runtime Abstraction
                               |
          +--------------------+--------------------+
          |                    |                    |
          v                    v                    v
  Specialized Codex    Specialized Claude     Generic Runtime
       Adapter               Adapter                 |
          |                    |                     |
   Codex App Server      Claude Harness        OpenCode Server
                                                  |
                                          @opencode-ai/llm
                                          /       |       \
                                         /        |        \
                                      OpenAI  Anthropic  Others
```

Operator interface:

```text
                   OpenCode TUI
                        |
                 OpenCode SDK/API
                        |
              Work Engine integration
```

potentially extended into:

```text
+------------------------------------------------------+
| Work Engine Operator Console                         |
|------------------------------------------------------|
| Planner | Orchestrator | Supervisors | Builders      |
|------------------------------------------------------|
| Active session / model interaction                   |
|------------------------------------------------------|
| Context epoch | lifecycle | queue | leases | status  |
+------------------------------------------------------+
```

This would give Work Engine a broadly portable OSS substrate while preserving specialized runtime paths where they provide meaningful capabilities.

---

# 17. Strategic Hypothesis

The strongest hypothesis is not that OpenCode can replace Codex App Server wholesale.

It is:

> OpenCode may already contain the generic infrastructure Work Engine would otherwise need to build around Codex App Server: provider abstraction, runtime execution, protocol handling, SDK/server boundaries, and terminal UI.

If its lower layers preserve provider-native escape hatches and its higher layers can be used without surrendering lifecycle authority, Work Engine can adopt those components selectively.

That could reduce future architecture from:

```text
Work Engine
 + custom Codex integration
 + custom Claude integration
 + custom OpenRouter integration
 + custom provider abstraction
 + custom runtime
 + custom event system
 + custom server
 + custom SDK
 + custom CLI/TUI
```

toward:

```text
Work Engine
 + lifecycle/orchestration authority
 + thin specialized harness adapters
 + OpenCode generic runtime/provider substrate
 + OpenCode-derived operator UI
```

That is a substantially different implementation burden.

---

# 18. Admission Question

The reconnaissance should ultimately answer one question:

> Can OpenCode be placed underneath Work Engine such that Work Engine retains orchestration and context-lifecycle authority, provider/harness-native capabilities remain accessible, and OpenCode supplies enough generic runtime and operator-interface infrastructure to materially reduce what Work Engine must own?

If yes, proceed to a bounded integration pilot.

If no, preserve the architectural findings and reuse only the OpenCode components whose boundaries are independently valuable.

---

# Repository Evaluation — 2026-09-05

## Status and attribution

This section preserves a read-only Codex repository evaluation performed on
2026-09-05. It narrows the preceding hypothesis; it does not accept the
provider-neutral architecture, authorize implementation, change the
skills-migration roadmap, or admit OpenCode as a dependency.

The evaluation used these source subjects:

- Work Engine `130ea8f1be720d1efc49ef397456bb44133c7ae0`, plus directly
  inspected working-tree files where the active App Server work was newer than
  the indexed commit;
- OpenCode `bbd72fb8b0bb6de580d2041a0150016227c63ac0` on `dev`;
- Work Engine's generated Codex App Server bindings for Codex CLI `0.149.1`;
- the ambient Codex CLI observed during evaluation, version `0.153.0`; and
- current official Codex App Server documentation, used only to identify
  capabilities that postdate or extend the pinned Work Engine surface.

No live provider call, runtime compatibility test, dependency installation, or
OpenCode execution pilot was performed. Repository structure and source were
inspected; conclusions that require execution remain hypotheses.

## Evaluated conclusion

The strategic premise is directionally correct, but the phrase "migrate to
OpenCode" combines three materially different seams:

1. provider-turn execution;
2. coding-harness and session runtime; and
3. operator presentation.

Those seams should be evaluated and admitted independently. The strongest
supported candidate is OpenCode's low-level provider-turn layer. The full
OpenCode session runtime is not presently admissible as a Work Engine runtime
replacement because it owns context projection, compaction decisions, tool-loop
continuation, and session execution behavior. The TUI is a credible pilot
candidate, but using it without a permanent fork still requires an OpenCode API
projection or a source-level embedding boundary.

Completing the current App Server migration materially improves portability
because it separates canonical meaning, domain services, runtime realization,
and replaceable provider-thread bindings. It does not by itself make a
provider- or harness-independent move easy: executable-generation composition,
context-transition mechanisms, lifecycle observations, and operator projection
remain Codex-shaped in important places.

## Confirmed observations

### Work Engine and Codex App Server

- `app-server/src/capabilities.mjs` declares eight foundation capabilities:
  thread start, resume, and read; experimental turn listing; turn start; exact
  skill input; thread-scoped dynamic tools; and client message identity. It
  declares one provider-runtime capability: model-context replacement through
  Codex `new_context`.
- `app-server/protocol-bindings.lock.json` deliberately retains a small type
  closure around that surface.
- The generated `0.149.1` `ThreadStartParams` already includes fields not
  exposed through the runtime manifest, including `modelProvider`, approval
  reviewer selection, arbitrary config, service name, and thread source.
  `TurnStartParams` additionally supports reasoning effort, reasoning summary,
  and output schema.
- `ManifestRoleRuntime` already accepts an injected object implementing
  `deliverTurn`, which is a promising provider-neutral composition seam.
  However, it describes that dependency as an App Server adapter, and
  `createExecutableGenerationRoleEnvironment(...)` directly constructs
  `CodexAppServerAdapter`.
- The role adapter consumes a curated App Server subset. Its provider request
  surface supports thread start/resume/read, turn listing/start, completion and
  token observations, and dynamic tool calls. Unsupported server requests fail
  closed.
- The proxy can relay non-intercepted App Server traffic, but the Work
  Engine-owned role presentation is intentionally narrower: exactly one text
  input, synthetic shell-turn lifecycle, final-output projection, and no
  incremental role item or tool-event projection to the UI.
- Work Engine already uses Codex as a provider-routing harness in the bounded
  S10 OpenRouter reviewer adapter by writing an isolated `model_provider =
  "openrouter"` Codex configuration. Provider diversity through Codex is
  therefore already partly demonstrated at a narrower execution boundary.
- Work Engine pins and enforces exact Codex version `0.149.1`; the ambient CLI
  observed during evaluation was `0.153.0`. If the live launch path uses that
  ambient executable, compatibility negotiation will fail. A separately
  managed pinned executable may make this observation operationally harmless.

These observations confirm that Work Engine does not use the full available
App Server surface. That is not automatically a defect. The narrow surface has
also protected Work Engine ownership. Additional App Server capabilities are
valuable when they remove protocol emulation or improve presentation without
transferring orchestration, context-policy, or workflow authority to Codex.

### OpenCode provider-turn layer

- `packages/llm` defines `@opencode-ai/llm` as an Effect Schema-first LLM core
  independent of session concerns.
- It models one provider turn through `prepare`, `stream`, and `generate`.
  Higher-level session persistence, permissions, telemetry, and continuation
  are intentionally outside this package.
- A runnable route composes protocol, endpoint, authentication, framing, and
  transport. Provider facades exist for OpenAI, Anthropic, OpenRouter, Gemini,
  Bedrock, and OpenAI-compatible deployments.
- The normalized request and event models include streaming text and reasoning,
  tools, finish state, usage, errors, and provider metadata.
- Request construction supports provider-specific options and raw HTTP body,
  header, and query overlays. Protocol-owned fields are denylisted from raw
  body replacement; other fields remain extensible.
- OpenRouter is represented as a configured route over the shared OpenAI Chat
  protocol rather than a separate session runtime.
- `@opencode-ai/llm` is marked `private: true`. It is independently factored in
  source but is not presently an ordinary stable external package dependency.
- OpenCode's own session product keeps this native LLM path opt-in and
  experimental. AI SDK remains the default, and unsupported routes fall back
  to it.

This is the best-supported OpenCode adoption seam, subject to dependency,
stability, cancellation, event-fidelity, and compatibility evidence.

### Anthropic context management

- OpenCode's HTTP request overlay can likely carry an Anthropic
  `context_management` body field and required beta headers because those fields
  are not protocol-owned denylist entries.
- The native Anthropic request schema does not currently model
  `context_management` explicitly.
- The Anthropic streaming event schema and parser do not currently model
  context-management response events or results.
- Provider metadata is an extensibility mechanism, but the current parser
  populates it only with selected recognized data such as usage, thinking
  signatures, and stop sequence. It is not a lossless raw-event channel.

Request-side access therefore appears feasible. Response-side lifecycle
fidelity is not currently sufficient for Work Engine reconciliation. Admission
would require an explicit protocol/event extension and fixtures proving which
operation was applied, resulting token state, failure/fallback behavior, and
the retained raw or authenticated provider evidence.

### OpenCode session and context runtime

- OpenCode has a substantive system-context algebra with stable keyed sources,
  durable snapshots, reconciliation, replacement, and fail-closed handling of
  temporarily unavailable admitted sources.
- Its session context epoch persists a baseline and snapshot. Ordinary changes
  reconcile into an active epoch; compaction after the baseline enables a
  replacement baseline.
- The session runner constructs model context, calls `compactIfNeeded(...)`
  before the provider turn, streams the request, executes tools, and decides
  whether continuation is required.
- Automatic compaction can be disabled by configuration, but the session layer
  remains the owner of context projection, compaction orchestration, and
  continuation.
- OpenCode's repository contract states that durable prompt admission is
  separate from execution, but local drains remain process-local. Post-crash
  continuation of provider work requires a separate future design.

The conceptual overlap with Work Engine context epochs is useful prior art, not
proof of runtime substitutability. Initial adoption of the OpenCode session
runtime would transfer mechanics and decisions that Work Engine currently
requires itself to own.

### OpenCode server and SDK

- The server exposes a broad typed HTTP API for sessions, status, messages,
  fork, abort, async prompts, commands, shell execution, permissions, diffs,
  providers, MCP, PTY, files, workspaces, event streaming, and TUI control.
- A generated JavaScript SDK exists and accepts an independently running server
  URL.
- The current server surface mixes V1 and V2 domain types. Multiple groups are
  explicitly annotated as experimental HTTP APIs.
- Session API breadth does not establish a neutral runtime boundary: the
  endpoints invoke OpenCode-owned session and lifecycle behavior.

The server and SDK are therefore useful compatibility or presentation
candidates, but should not initially become Work Engine's canonical runtime or
state owner.

### OpenCode TUI

- `opencode attach <url>` runs the full TUI against a separately running
  OpenCode server.
- The extracted TUI entry accepts a server URL, directory, custom `fetch`,
  custom event source, headers, and a plugin host.
- Its plugin API exposes custom routes, navigation, keymaps, dialogs, slots,
  commands, events, state, the renderer, lifecycle hooks, and the OpenCode SDK
  client. The built-in UI already defines several insertion and replacement
  slots around the home prompt, session prompt, application frame, and sidebar.
- `@opencode-ai/tui` is marked `private: true` and depends on several OpenCode
  workspace packages.
- The TUI's stores and primary views expect OpenCode SDK resources and event
  shapes. The stock attach command exposes remote URL and authentication but
  not the source entry point's complete fetch/event injection surface.

A Work Engine UI pilot is credible. A no-fork adoption would still need either
an OpenCode-compatible Work Engine facade, an OpenCode server integration that
does not acquire Work Engine authority, or a source-level TUI composition
boundary. The plugin surface appears strong enough for Work Engine-specific
panels, but that should be demonstrated rather than assumed.

## Mapping to the provider-neutral architecture

The evaluation supports the independently owned
[Provider Turn, Harness Runtime, and Operator Projection](./provider-turn-harness-runtime-and-operator-projection.md)
split. OpenCode should be evaluated as a candidate implementation at each port,
not treated as the owner of the split:

```text
Work Engine authority and durable services
        |
        +-- ProviderTurnPort
        |     OpenCode LLM or direct provider adapters
        |
        +-- HarnessRuntimePort
        |     Codex App Server, Claude Code, later harnesses
        |
        +-- OperatorProjection
              Codex TUI, OpenCode TUI, or another client
```

The neutral proposal owns the contracts, authority boundaries, composition
rules, and relationship to materialized realization. This evaluation supplies
the narrower findings that OpenCode's LLM layer is the strongest candidate for
`ProviderTurnPort`, its TUI is a plausible `OperatorProjection`, and its full
session runtime is not presently admissible as `HarnessRuntimePort` without
transferring context and execution authority.

The current `ManifestRoleRuntime` injection point remains relevant repository
evidence: it is a precursor to provider-neutral composition, but duck typing of
`deliverTurn(...)` does not by itself satisfy the neutral port contract.

## Relationship to S13

S13 already owns the candidate work to define an immutable provider-neutral
reviewer-turn event schema and provider normalizers while preserving raw
payloads or explicit authorized omissions. That is the correct semantic owner
for the first provider-neutral event contract.

OpenCode `LLMEvent` should be evaluated as one adapter input to that contract,
not adopted as Work Engine's canonical event schema. Its normalized events are
useful, but the Anthropic context-management finding demonstrates why Work
Engine still requires explicit raw-evidence and omission semantics.

This evaluation does not amend S13 or insert OpenCode into its active execution.
A later bounded pilot may consume S13's accepted event boundary or be scheduled
as separately authorized research.

## Candidate bounded pilots

### A. Provider-turn contract pilot

Use fixtures before paid inference. Exercise:

- request preparation and deterministic inspection;
- streaming text, reasoning, tools, finish state, and usage;
- cancellation and interruption behavior;
- OpenRouter route selection;
- provider-specific options and raw request overlays;
- Anthropic `context_management` request injection; and
- exact preservation or explicit omission of context-management response data.

The pilot should compare OpenCode's normalized event stream with the accepted
Work Engine provider-turn event contract. Transport completion alone is not
semantic equivalence.

### B. Codex App Server capability inventory

Inventory the complete pinned `0.149.1` surface and separately compare it with
the intended upgrade version. Consider capabilities that remove existing
synthetic or private coupling, especially:

- model-provider and capability discovery;
- turn interruption and steering;
- thread listing, status, forking, and paginated history;
- richer user input and output-schema control;
- approval routing; and
- incremental item and tool-event projection.

Each capability should remain classified as Work Engine, harness, provider,
generic runtime, or operator presentation before adoption.

### C. OpenCode TUI projection pilot

Demonstrate one Work Engine-specific plugin route or panel showing a bounded
projection such as:

- logical roles and runtime bindings;
- context epoch and lifecycle status;
- admitted/running/queued work;
- leases or fences; and
- available administrative commands.

The pilot must record whether it used an API-compatibility facade, injected
fetch/events, a server plugin, or a source fork, and estimate the maintenance
surface of that route.

## Admission gates

OpenCode component adoption should remain unresolved until evidence answers:

1. What stable distribution and update policy replaces the current private
   workspace-package relationship?
2. Can provider requests and responses preserve every field needed for Work
   Engine lifecycle evidence, including explicit authorized omissions?
3. Are cancellation, streamed tool calls, tool results, errors, and retry
   boundaries deterministic enough for Work Engine reconciliation?
4. Can OpenCode session compaction and continuation be completely bypassed when
   Work Engine owns lifecycle policy?
5. Can the server/SDK contract be version-pinned without importing unstable V1
   and V2 session ownership?
6. Can the TUI present Work Engine state and commands without a large permanent
   fork or a false OpenCode-session source of truth?
7. Does the selected route preserve Work Engine's stronger admission, fencing,
   restart, and post-crash reconciliation semantics?
8. Is each provider- or harness-native capability still directly reachable and
   truthfully attributed?

## Revised strategic hypothesis

The source supports this narrower hypothesis:

> OpenCode is a promising provider-protocol and operator-interface substrate.
> Its low-level LLM package may materially reduce provider integration work,
> and its TUI may materially reduce operator-interface work. Its complete
> session runtime is not presently admissible beneath Work Engine without
> transferring context and execution authority. Specialized Codex and Claude
> harness adapters should remain until semantic equivalence is independently
> demonstrated.

Recommended current disposition:

- continue the App Server migration;
- selectively use more App Server capabilities where they reduce emulation
  without moving Work Engine meaning into Codex;
- retain specialized Codex and Claude harness adapters;
- investigate `@opencode-ai/llm` through a fixture-first provider-turn pilot;
- investigate the OpenCode TUI through an independent operator-projection
  pilot; and
- do not authorize a wholesale OpenCode runtime migration from this evidence.

## Reopening conditions and remaining uncertainty

Revisit the full OpenCode server/runtime candidate only if OpenCode exposes a
stable external execution boundary that lets a caller supply complete context,
disable or externalize compaction and continuation decisions, retain raw
provider lifecycle evidence, and reconcile interrupted work without making an
OpenCode session the Work Engine state owner.

Revisit direct package admission if `@opencode-ai/llm` or
`@opencode-ai/tui` becomes a supported published package, or if Work Engine
accepts a versioned vendoring/upstream-maintenance policy with a bounded patch
surface.

Unknown until execution evidence exists:

- real provider cancellation behavior across supported transports;
- end-to-end Anthropic context-management result fidelity after extension;
- API compatibility effort required by the stock TUI;
- practical plugin limitations for topology and lifecycle views;
- long-term stability of the extracted OpenCode V2 packages and HTTP API; and
- comparative maintenance cost versus extending the existing App Server host.
