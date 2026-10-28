# Lifecycle integrated pluggable agent runtime

**Status:** Captured architecture idea, deferred while the API diagnostic is completed.

**Origin:** User discussion on 2026-10-24 UTC. The user asked whether a pluggable
context-lifecycle integration could replace Codex with our own implementation,
giving Work Engine tighter control over context and session management. The
follow-up direction was: “write that down and let's finish the api stuff.”

**Authority:** Capture for later design. This does not authorize a runtime
replacement, change the roadmap, or expand the current API diagnostic.

## Intended outcome

Make the agent runtime replaceable while retaining one context-lifecycle owner
for durable meaning, continuity, transition admission, and recovery. A Work
Engine runtime would construct inference context and manage its own conversation
and tool loop, making context replacement a native runtime operation. Codex could
remain a separate runtime implementation under its supported capability profile.

The useful control is over client-supplied context and local session execution.
It does not imply control over model internals or independently observed provider
history. Owning the runtime should enable stronger local evidence and more direct
lifecycle integration without inventing those broader claims.

## Proposed ownership

| Component | Responsibility |
| --- | --- |
| Context lifecycle | Required preserved meaning, safe replacement, successor acceptance, unresolved obligations and recovery |
| Agent runtime | Conversation history, request construction, tool execution, interruption and session persistence |
| Inference adapter | Provider request transport, streamed responses, usage and error observations |

Each runtime would declare the controls and evidence it supports. A single
generic prompt-submission operation would hide material differences in history
access, tool admission, interruption and recovery. One owner executes each tool
loop and effect; integration must not introduce overlapping executors.

Logical session, context generation, inference request, provider response and
process incarnation remain distinct. A logical task may survive many context
generations and process restarts. Credential refresh is another separate event.

## Lifecycle integration

At a known execution boundary, the runtime could pause new work, preserve
completed tool results and unresolved effects, obtain an accepted continuation,
assemble a new context generation, and resume the same logical task. Semantic
continuation and within-generation transcript storage remain different products.
Context replacement would not itself settle an uncertain external effect or
authorize its retry.

The design hypothesis is that direct control over request assembly, durable
history and tool boundaries can reduce the coordination needed around an
externally managed harness. Quality, quota consumption and recovery behavior
would need measurement; replacing the harness also changes its instructions,
tools and execution behavior.

## Small first prototype

Use one retained task, one tool, durable history, context replacement and crash
recovery. Demonstrate that the task retains an unresolved commitment and resumes
without replaying a completed tool effect. Include interruption around tool
entry and result persistence so a successful demonstration exercises recovery.

This prototype would test the lifecycle integration benefit before expanding
into a broader coding-agent implementation. Rust is the existing first-party
implementation direction. Continuous dual operation with Codex is not assumed
necessary; migration and saved-state treatment remain separate decisions.

## Related work

- [Proposed post S10 backend abstraction amendment](../../../experiments/context-lifecycle-rust/POST_S10_BACKEND_ABSTRACTION_AMENDMENT.md): existing proposal for managed Codex and direct Responses under shared lifecycle semantics; reconcile this idea with it rather than create a second session manager.
- [Provider turn, harness runtime and operator projection](provider-turn-harness-runtime-and-operator-projection.md): existing discussion of independently replaceable execution and interface responsibilities.
- [Rust lifecycle design](../../../experiments/context-lifecycle-rust/DESIGN.md): owning lifecycle contracts and provider/tool boundaries.
- [Rust development direction](../../../docs/rust-development.md): implementation and migration direction.

The immediate priority from the user remains finishing the bounded API
diagnostic. This idea is retained for the subsequent session-management work.
