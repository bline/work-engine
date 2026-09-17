# Idea: Context-Replacement Emergency Recovery and Writable Operator Ingress

**Status:** Post-migration proposal input

**Scope:** Provider context replacement, retained-role recovery, writable
operator access, and operator-message framing

**Authority:** Exploratory only. This document does not change the active
migration, authorize a runtime restart, alter context-lifecycle policy, or
authorize implementation.

```yaml
idea_status:
  architectural_supersession: none
  architectural_supersession_note: "The 2026-09-16 canonical-view test below found each of Candidates A/B/C maps cleanly to existing mechanisms (Transition Fencing, Authority-Preserving Intent Projection, Authority Projection SS8) but nothing has been absorbed into any canonical view -- all three remain proposed, not accepted."
  residue: present
  residue_ledger: "Candidate A's 'Questions for proposal formation' (SS168-192, un-numbered) include the exact-invariant question this session's own reconciliation flagged as a genuine extension to mechanisms/transition-fencing-and-leases.md not yet decided (whether the new_context actuator can be admitted only under an exact Work Engine lease), now tagged inline [KIND: RESIDUE] [OPEN] (corrected 2026-09-17 -- previously only described here, not tagged in the document body, which the deterministic validator correctly flagged as unsupported). Candidates B and C were not re-audited as a formal ledger this pass; their own bullet lists were not individually tagged."
  backlog: present
  backlog_ledger: "Corrected 2026-09-17, item-level drain: Candidates B (7 items) and C (5 items) individually classified -- all 12 are KIND: BACKLOG (protocol/transport/schema/testing detail applying already-established authority/capability-projection/delegation principles, none proposing new ownership) and confirmed OPEN (checked against real code: clientUserMessageId/custody plumbing exists in src/operator-switchboard.mjs and src/context-input-custody.mjs, but the retained-role-specific writable-ingress feature this candidate needs is not built). No dedicated staged-plan section in any of Candidates A/B/C (each is a proposal-formation direction, not an operative plan with named stages), but that no longer matters -- the individually-classified BACKLOG items establish present on their own."
  audit_scope:
    - open-question-ledger
    - keyword-scan: full_document
    - close-read: "Canonical-view reconciliation section (2026-09-16)"
  audit_scope_completeness: complete
  audit_scope_completeness_note: "Corrected 2026-09-17: Candidates A, B, and C's bullet lists are now all individually classified with a checked disposition (zero UNCHECKED tags remain anywhere in the document)."
  status_as_of: 2026-09-17
```

```yaml
idea_provenance:
  origin: direct_capture
```

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
- **[KIND: RESIDUE] [OPEN — this session's own reconciliation flagged this as a genuine extension to mechanisms/transition-fencing-and-leases.md not yet decided.]** Can the provider-side `new_context` actuator be admitted only under an exact
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

Proposal formation should consider (individually classified 2026-09-17, per item-level drain -- each is protocol/transport/implementation detail; the last bullet is already governed by runtime-realization.md SS11 / mechanisms/authority-preserving-intent-projection.md's already-established "projection may never mutate directly or enlarge authority" invariant, so this candidate must correctly apply that existing discipline, not invent a new one):

- [KIND: BACKLOG] [OPEN] a proxy-owned route to attach to an existing retained writable role without
  widening the supervisor's own sandbox or authority;
- [KIND: BACKLOG] [OPEN — checked: `clientUserMessageId`/custody plumbing already exists (`src/operator-switchboard.mjs`, `src/context-input-custody.mjs`), but the retained-role-specific writable attachment this candidate needs is not built] explicit authorization and audit evidence for who may send writable-role
  input;
- [KIND: BACKLOG] [OPEN — checked: `clientUserMessageId` already exists as a real concept in `src/operator-switchboard.mjs`, but not yet bound to one submission/one role turn for a retained role specifically] one submission, one `clientUserMessageId`, and one role turn, including text
  containing embedded newlines;
- [KIND: BACKLOG] [OPEN] a framed CLI protocol such as length-delimited JSON or an explicit
  begin/send/end operation rather than readline-defined message identity;
- [KIND: BACKLOG] [OPEN] structured transfer of lifecycle and campaign references rather than copying
  terminal-rendered text;
- [KIND: BACKLOG] [OPEN] reconnection, idempotency, interruption, and queued-input behavior; and
- [KIND: BACKLOG] [OPEN — this is an application of the already-established "projection never enlarges authority" invariant (runtime-realization.md SS11), not a new ownership question] preservation of the distinction between access to a writable builder and
  authority to expand its accepted implementation scope.

## Candidate C: policy-conditioned tool affordances

A role with a fixed `approval_policy: "never"` cannot successfully request
approval escalation. Advertising `require_escalated` in that role's tool schema
creates an invalid apparent action even though runtime enforcement remains
fail-closed.

Proposal formation should consider (individually classified 2026-09-17, per item-level drain -- schema/attribution/testing detail applying already-established capability-projection (runtime-realization.md's capability inventory, authority-preserving-intent-projection.md's discovery/rendering territory) and delegation (authority-and-ownership.md SS9's "nomination-only" mode) concepts to this specific tool-schema case, not proposing new ownership):

- [KIND: BACKLOG] [OPEN] whether tool schemas can omit escalation arguments and approval-question
  fields when the effective policy can never admit them;
- [KIND: BACKLOG] [OPEN — an application of the already-established capability-discovery/projection territory (mechanisms/authority-preserving-intent-projection.md) to sandbox/approval-policy capabilities specifically] whether sandbox and approval policy should be projected as explicit
  machine-readable capability constraints rather than left for repeated model
  inference;
- [KIND: BACKLOG] [OPEN] how a rejected impossible request is attributed and surfaced without
  misclassifying it as command execution or user denial;
- [KIND: BACKLOG] [OPEN — an application of authority-and-ownership.md SS9's already-established "nomination-only" delegation mode, not a new one] when a blocked operation should nominate another role or a host-owned
  capability rather than suggest policy escape; and
- [KIND: BACKLOG] [OPEN] tests proving that `approval_policy: "never"` roles cannot construct an
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

## Canonical-view reconciliation (2026-09-16)

Tested directly against `context-lifecycle.md`, `mechanisms/authority-
preserving-intent-projection.md`, `mechanisms/transition-fencing-and-
leases.md`, `mechanisms/resource-lease-and-fencing.md`, and `authority-and-
ownership.md`. The document's own instinct not to bundle Candidates A, B, and
C into one obligation is confirmed correct by this test: each maps to a
different, non-overlapping subset of the five views, and the split tracks
almost exactly onto **authoritative writes** (Work Engine's own canonical
lifecycle state) versus **bounded operator ingress** (a human relaying intent
to an already-authorized role without expanding what that role may do).

### Candidate A is authoritative-write territory: an unfenced actuator, not a stale fence

`context-lifecycle.md` §6 states this dimension "consumes, never owns"
transition fencing, and its one confirmed live instance is the preparation-
fence-then-transition-lease sequence `mechanisms/transition-fencing-and-
leases.md`'s own "Confirmed Instances" section verifies directly against
`semantic-context-lifecycle-manager.md`. The incident's defect —
"the provider's token-budget boundary instructed the builder model to invoke
`new_context`. No Work Engine retirement lease, accepted checkpoint, or
rehydration request existed" — is not a *stale-fence* failure (the mechanism's
usual concern: a fence invalidated by a competing revision during
preparation). It is a **fence-bypass**: the actuator fired with no fence
acquired at all, because the provider itself can invoke `new_context`
independently of Work Engine's own lease sequence. Candidate A's own question
— "Can the provider-side `new_context` actuator be admitted only under an
exact Work Engine lease, or is it necessarily available throughout the
process?" — asks exactly the right question of exactly the right mechanism:
whether the actuator itself can be brought under `mechanisms/transition-
fencing-and-leases.md`'s governing invariant ("No transition may activate a
successor reasoning environment from a world revision that ceased to be
authoritative while that transition was being prepared"), rather than only
Work Engine-initiated transitions. If accepted, this strengthens Context
Lifecycle's *existing* consumption of the mechanism — it does not require a
new fence class beyond the two `transition-fencing-and-leases.md` already
names (decision-episode, topology-transition), and does not make Context
Lifecycle an owner of fencing it still correctly does not own.

`authority-and-ownership.md` §12 ("Invalidation Never Mints Authority")
governs the failure mode directly, already, without needing new text: "The
emergency path must not manufacture a successful checkpoint, silently accept
provider summaries as canonical state, or allow a fresh model window to infer
that unresolved operator intent was completed" is this idea's own restatement
of §12 applied to a provider-forced, unleased replacement — an emergency
transition the Work Engine lifecycle did not admit must not be treated as
having produced authoritative continuation, exactly as an invalidated
realization may not silently keep exercising authority (`runtime-
realization.md` §7, the same invariant's other confirmed instance).

**This is squarely authoritative-write territory**: Candidate A is about who
may actuate a change to Work Engine's own canonical lifecycle state, and
under what fence — no operator or external intent is involved in the failure
itself.

### Candidate B is bounded-operator-ingress territory, and converges on an already-named gap

`mechanisms/authority-preserving-intent-projection.md` names the exact shape
Candidate B is asking for: "discovering and rendering available operations,
collecting bounded operator/human intent, submitting it without manufacturing
authority, and showing proposed / pending / admitted / refused / completed /
stale lifecycle feedback." Candidate B's own bullets — one submission bound
to one `clientUserMessageId` and one role turn, structured transfer of
lifecycle/campaign references instead of copied terminal text,
reconnection/idempotency/interruption/queued-input behavior — are the same
lifecycle-feedback and bounded-submission concerns this mechanism's page
already tracks as unbuilt.

**This is not a coincidental resemblance.** The switchboard the incident
actually used for the manual relay is `app-server/src/operator-
switchboard.mjs` — the identical file `mechanisms/authority-preserving-
intent-projection.md` already cites, by name, as a real but partial
precursor: it "implements part of this shape (discovery, runtime-binding
lookup, bounded projection routing)" but "explicitly lacks active-binding
fencing or runtime-selection-policy-overlay machinery," and its
lifecycle-feedback half "appears nowhere in `operator-switchboard.mjs`'s own
text, confirmed by direct read." Candidate B is a second, independent
pressure test — from an operational incident rather than an architectural
reconciliation — landing on the same real gap in the same real file. Read
together with `runtime-realization.md` §11 (the operator-policy-overlay,
already a confirmed partial instance of this same mechanism), this is now a
**three-way independent convergence** on one unbuilt mechanism, not two.

The document's own care to distinguish "access to a writable builder" from
"authority to expand its accepted implementation scope" is exactly the
invariant this mechanism exists to enforce ("may constrain and encode
authority; may never enlarge it"). It is also why `mechanisms/resource-
lease-and-fencing.md`'s named-but-unbuilt **fenced active-binding for logical
role instances** (`logical-role-instance:<id>:active-binding`) is the right
consuming mechanism for the exclusivity half of Candidate B: an operator
route that "attach[es] to an existing retained writable role without
widening the supervisor's own sandbox or authority" must not create a second
writer racing the builder's own realization for the same role — it must
attach to the one currently-fenced authoritative generation, exactly the
shape that resource type was accepted (2026-09-14) to express, not yet built.

**This is squarely bounded-operator-ingress territory**: the operator
supplies intent (`authority-and-ownership.md` §7's Observe/Recommend modes,
never Decide or Admit on the ingress channel's own account); the retained
role's own already-granted authority is what executes it, unchanged.

### Candidate C is neither — it is authority-projection accuracy, and maps to a third view

Advertising `require_escalated` to a role fixed at `approval_policy: "never"`
is a projection that overstates the real authority ceiling — not by granting
authority (runtime enforcement correctly fail-closed), but by depicting a
candidate action the ceiling can never admit. `authority-and-ownership.md`
§8's invariant — `child_authority ⊆ delegable(parent_authority)` — is the
right frame: a role's visible action space should be a truthful projection of
its own fixed ceiling, not a superset requiring the model to discover the
boundary by attempting and failing. `runtime-realization.md` §11 already
does exactly this classification for its own operator-policy-overlay
candidate states (`selected`/`preferred`/`admissible`/
`requires_operator_approval`/`prohibited`) — Candidate C is the same
discipline applied to tool-schema generation instead of a UI view, and would
reuse the identical, already-confirmed partial instance of
`mechanisms/authority-preserving-intent-projection.md`, not invent a new one.
Transition Fencing and Resource Lease and Fencing are not implicated at all:
nothing here is a mutual-exclusion or preparation-safety concern.

**Net:** the document's refusal to bundle A, B, and C is not just good
hygiene — the three concerns land on disjoint subsets of the five tested
views (A: Context Lifecycle + Transition Fencing + Authority & Ownership §12;
B: Authority-Preserving Intent Projection + Resource Lease and Fencing +
Authority & Ownership §7; C: Authority & Ownership §8 + Runtime Realization
§11 via the same intent-projection mechanism), confirming they are three
separate residue items against the settled architecture, not one obligation
wearing three descriptions.

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
