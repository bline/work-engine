# S10 context reset adapter integration note — 2026-11-04

Status: user direction recorded; future adapter integration work. The existing
context-lifecycle contract remains intact. This clarifies the
[earlier compatibility follow-up](rust-lifecycle-s10-context-replacement-followup-20261104.md):
there is no current basis or assignment for a lifecycle redesign.

The future task is to configure or connect the context replacement actuator and
verify the existing checkpoint, replacement and recovery semantics:

- An actual Codex CLI/App Server adapter can select its native configuration
  and enable the required `token_budget` feature. Actual `new_context`
  availability and transition/recovery evidence still need qualification.
- Pi's direct `openai-codex` provider does not read Codex CLI configuration.
  For that route, Pi's fresh-session interface is a candidate actuator to wire
  into the same lifecycle contract. Its physical session identity and event
  subscriptions require explicit integration.

This is expected to be configuration or adapter seam repair. Ordinary summary
compaction is not presumed equivalent to the required replacement. The task
remains deferred until integration reaches context replacement and rehydration;
it does not block integration of the accepted bounded coding profile.

Current work resumes at source integration. The independently authorized
backend integration owner already holds `unified-event-delivery:dot-s10-coding`
and has verified all 26 accepted S10 source files. That session owns the
MCP/coding join and its exact edit-packet approval. S10 supplies the accepted
frozen source, evidence and scoped handoff rather than duplicating that work.
Neither this note nor the handoff reopens the closed coding attempt or
authorizes a context-reset experiment, live edit, commit, deployment or release.
