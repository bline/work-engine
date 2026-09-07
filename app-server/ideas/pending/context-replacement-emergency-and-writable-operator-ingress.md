# Idea: Context-Replacement Emergency Recovery and Writable Operator Ingress

**Status:** Post-migration proposal input

**Scope:** Provider context replacement, retained-role recovery, writable
operator access, and operator-message framing

**Authority:** Exploratory only. This document does not change the active
migration, authorize a runtime restart, alter context-lifecycle policy, or
authorize implementation.

## Summary

The migration should first restore the intended live semantic-context lifecycle:
periodically preserve continuation-critical state, choose an efficient
replacement point, issue a fenced retirement directive, and reconcile a fresh
window before resuming work.

Two deeper concerns should remain explicit post-migration proposal candidates:

1. recovery when the provider reaches its own emergency token-budget boundary
   before the Work Engine lifecycle completes an authorized transition; and
2. a writable, atomic operator-ingress route for retained roles when the Codex
   proxy cannot provide the required access; and
3. policy-conditioned tool projection so a role is not offered operations that
   its fixed approval and sandbox policy can never admit.

These concerns interacted during migration, but they should not be made one
implementation obligation merely because they appeared in the same incident.

## Incident evidence and motivation

The retained builder was scoped to repair a reviewer-code blocker. The Codex
proxy did not provide a way to connect to that running builder with write
access, so the operator used the standalone switchboard and manually relayed
messages between the builder and supervisor. That was a necessary operational
workaround, not the desired long-term interface.

The switchboard consumes terminal input one physical line at a time. Multiline
operator messages and copied lifecycle output were therefore delivered as
separate turns with separate client message identifiers. One lifecycle episode
identifier was split in the middle of its UUID; concatenating the adjacent
turns recovered the complete identifier. The lifecycle renderer and App Server
adapter had retained the complete bytes, so the incident does not establish an
identifier-generation defect.

During the same relay, the provider's token-budget boundary instructed the
builder model to invoke `new_context`. No Work Engine retirement lease,
accepted checkpoint, or rehydration request existed, and the recovery tools
named by the provider reminder were unavailable. The replacement history
contained only bootstrap material. The active reviewer-repair request was not
carried into the fresh window, and its remaining physical lines arrived later
as independent operator turns.

The supervisor role exhibited the same provider-side emergency replacement
pattern. Its runtime was observing the shadow lifecycle rather than executing
the live lifecycle, so those replacements were not Work Engine decisions.

After live lifecycle recovery, the intentionally read-only supervisor tried to
run validation in the builder worktree. When temporary-resource restrictions
blocked the command, the model submitted an `exec` request with
`sandbox_permissions: "require_escalated"` and a human approval question. The
role was configured with `approval_policy: "never"`, so Codex correctly rejected
the request before command entry with `approval policy is Never; reject
command`. The denial preserved the policy boundary, but the model had still
been offered an impossible escalation control and spent a turn selecting it.
The same episode also exposed a routing problem: validation that required the
builder's execution environment should have been returned to the builder or a
host-owned gate runner rather than attempted through supervisor escalation.

## Candidate A: provider-emergency context recovery

The normal lifecycle should retire well before emergency exhaustion. A
post-migration proposal should nevertheless define truthful behavior when the
provider reaches its own boundary first.

Questions for proposal formation include:

- How can the host distinguish a Work Engine-leased retirement request from a
  provider-originated emergency reminder?
- Can the provider-side `new_context` actuator be admitted only under an exact
  Work Engine lease, or is it necessarily available throughout the process?
- What durable state can still be captured if an emergency arrives during an
  active domain turn?
- How are the current operator message, queued later messages, tool effects,
  unresolved human authority, and active campaign references fenced before
  replacement?
- What failure or quarantine state is recorded when preservation cannot be
  proven?
- May work resume after an unleased `context_compacted` observation, and what
  exact reconciliation evidence would be required?

The emergency path must not manufacture a successful checkpoint, silently
accept provider summaries as canonical state, or allow a fresh model window to
infer that unresolved operator intent was completed.

## Candidate B: writable atomic operator ingress

Retained roles need an operator route that combines the proxy's stable runtime
attachment with the access required by the selected role. An operator should
not have to choose between an inaccessible retained role and a standalone
terminal relay that changes message boundaries.

Proposal formation should consider:

- a proxy-owned route to attach to an existing retained writable role without
  widening the supervisor's own sandbox or authority;
- explicit authorization and audit evidence for who may send writable-role
  input;
- one submission, one `clientUserMessageId`, and one role turn, including text
  containing embedded newlines;
- a framed CLI protocol such as length-delimited JSON or an explicit
  begin/send/end operation rather than readline-defined message identity;
- structured transfer of lifecycle and campaign references rather than copying
  terminal-rendered text;
- reconnection, idempotency, interruption, and queued-input behavior; and
- preservation of the distinction between access to a writable builder and
  authority to expand its accepted implementation scope.

## Candidate C: policy-conditioned tool affordances

A role with a fixed `approval_policy: "never"` cannot successfully request
approval escalation. Advertising `require_escalated` in that role's tool schema
creates an invalid apparent action even though runtime enforcement remains
fail-closed.

Proposal formation should consider:

- whether tool schemas can omit escalation arguments and approval-question
  fields when the effective policy can never admit them;
- whether sandbox and approval policy should be projected as explicit
  machine-readable capability constraints rather than left for repeated model
  inference;
- how a rejected impossible request is attributed and surfaced without
  misclassifying it as command execution or user denial;
- when a blocked operation should nominate another role or a host-owned
  capability rather than suggest policy escape; and
- tests proving that `approval_policy: "never"` roles cannot construct an
  escalation request while write-capable roles can execute already-admitted
  operations without asking for escalation.

This candidate does not imply that the runtime denial was faulty. The observed
failure is the mismatch between the advertised action space and the effective
policy, plus the absence of a direct route to the correct execution owner.

## Relationship and sequencing

The immediate migration repair is a prerequisite operational correction: run
retained roles through the existing live lifecycle whenever `token_budget` is
enabled. It should not wait for either post-migration candidate above.

Candidates A, B, and C may later become separate proposals. Emergency recovery
concerns context-transition correctness. Writable atomic ingress concerns
operator access and message custody. Policy-conditioned affordances concern the
accuracy of the model-visible action space. They share incident evidence and
should cross-reference each other, but none should silently absorb another's
scope or authority.

## Non-goals

This idea does not:

- change the active migration roadmap or its current accepted slice;
- conclude that the existing live lifecycle is correct in every emergency;
- authorize provider calls, process restarts, or runtime replacement;
- authorize direct mutation of retained role or campaign databases;
- authorize widening the supervisor sandbox or changing its approval policy;
- treat terminal presentation as canonical lifecycle state; or
- accept either candidate as a proposal.

## Reopening conditions

Form post-migration proposals when the live lifecycle is operating reliably
enough to investigate and implement these concerns without depending on the
broken path being repaired. Reopen earlier if another unleased replacement or
operator-message fragmentation incident threatens active migration state.
