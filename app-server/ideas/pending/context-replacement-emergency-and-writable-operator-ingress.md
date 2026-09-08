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

Three deeper concerns should remain explicit post-migration proposal candidates:

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

## Live lifecycle audit: 2026-09-08

A later retained-builder episode provided the first useful end-to-end evidence
from the repaired live lifecycle. The result was mixed: preservation and
fail-closed reconciliation worked, but the observed cycle was not token
efficient and did not produce a usable continuation.

Observed facts:

- the builder reached 211,732 live-context tokens in a 258,400-token window
  before replacement, approximately 81.9 percent of the window;
- the provider's recorded native auto-compact limit was 244,800 tokens,
  approximately 94.7 percent, so Work Engine acted about 33,068 tokens earlier;
- the lifecycle published a durable checkpoint whose attribution, authority
  preservation, interaction closure, source binding, and sufficiency checks all
  passed;
- compaction reduced the next visible context to 9,842 tokens, removing about
  201,890 tokens from the live context;
- reconciliation truthfully reported that the checkpoint's Work Engine skills
  were not exposed in the fresh runtime's available skill catalog;
- because governing-environment applicability could not be established, input
  admission remained closed and no lifecycle episode was admitted; and
- no operator request or unresolved authority was silently discarded. The
  lifecycle stopped instead of guessing.

The absence of an admitted lifecycle episode must not be interpreted as an
absence of lifecycle activity. In this case the checkpoint, verification,
compaction observation, and rejected reconciliation were durable in separate
stores, while the episode table remained empty because admission never
completed.

### Observed token economics

The primary retained thread consumed approximately 590,142 gross tokens between
the completed domain turn and the end of lifecycle reconciliation. Of its
588,235 additional input tokens, 551,168 were reported as cached input. The
separate compiler and verifier calls carried approximately 61,512 additional
gross context tokens. The observed lifecycle cost was therefore about 651,654
gross tokens, although the available telemetry cannot translate cached and
ephemeral inputs into an exact quota or monetary cost.

The replacement removed approximately 201,890 tokens from the live context.
On gross context volume alone, the lifecycle would need roughly four later
productive inference calls before the avoided replay exceeded the observed
overhead. No such productive calls occurred in this episode: reconciliation
kept admission closed after the domain task had already completed. This episode
therefore improved durability and safety but does not establish any token
saving. It most likely increased token consumption.

These figures are an incident measurement, not a general benchmark. In
particular, cumulative thread token usage is not live context size, cached input
does not have the same cost characteristics as uncached input, and compiler and
verifier accounting is not yet projected with enough detail for an exact
break-even calculation.

## Bounded repair direction

The immediate correctness repair should make a fresh context epoch realize and
attest the same activated Work Engine role environment that the checkpoint
names. The compiler must not issue a continuation whose required skills or
governing instructions are absent from the runtime catalog. A mismatch should
continue to fail closed, but it should produce one durable, queryable failed
episode that links the checkpoint, compaction, realization evidence, rejected
reconciliation, and admission state.

After correctness is restored, token-efficiency work should be evaluated as a
separate optimization:

1. Project per-stage input, cached-input, output, compiler, verifier, and
   reconciliation usage into one lifecycle accounting receipt.
2. Record the live-context reduction and the number of productive post-recovery
   calls so realized savings can be distinguished from predicted savings.
3. Include expected remaining work in replacement judgment. Avoid an expensive
   proactive replacement when the domain turn has already reached a terminal or
   handoff boundary unless preservation risk independently requires it.
4. Reduce repeated compiler and verifier context, reuse immutable projections
   where their contracts permit it, and measure rather than assume cache
   effectiveness.
5. Surface checkpointing, retirement, compaction, reconciliation, admission,
   and failure as visible operator events without treating UI rendering as the
   canonical record.

The repair should be proven with a controlled retained-role exercise that:

- crosses the configured replacement threshold before native compaction;
- preserves a known operator obligation and active campaign reference;
- exposes the exact required role skills in the fresh epoch;
- successfully reopens admission only after reconciliation;
- performs enough productive post-recovery calls to measure break-even; and
- demonstrates a catalog-mismatch variant that remains closed with a complete
  failed-episode receipt.

This bounded repair does not require implementing generalized continuation
packets or branchable workflow history. Those ideas may later reuse its
checkpoint, environment-realization, and accounting evidence, but they should
remain separate proposals.

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
