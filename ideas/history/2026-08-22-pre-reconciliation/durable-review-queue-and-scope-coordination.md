# Durable Review Queue and Mutation-Scope Coordination

## Status

Idea / architecture proposal

This proposal develops the review, recovery, and resource-claim consequences
described in:

- `persistent-agent-state.md`;
- `persistent-agent-state-and-runtime-introspection.md`; and
- `persistent-strategic-planner.md`.

It focuses on one problem:

> **How can review remain useful when provider availability is intermittent and
> other slices may continue planning or executing in the same repository?**

---

## Motivation

Independent review may become unavailable after implementation and
deterministic validation are already complete.

Treating that condition as ordinary completion is false. Treating it as an
irrecoverable campaign failure discards useful work. Keeping the work only in
model context makes recovery fragile.

A durable review queue can preserve the pending obligation until the required
capability becomes available.

However, a queue alone is insufficient.

Suppose one slice is waiting for review while another slice rewrites a material
contract, state owner, or consumer path. The delayed reviewer can still review
the original immutable subject truthfully, but its result may not apply to the
later integrated state. The queue therefore preserves the obligation and the
evidence needed for a consequential consumer to judge applicability.

Overlap analysis, protected scopes, and reservations can coordinate mutations
when an actual shared-write race, exclusive resource, or authority boundary
would otherwise permit an invalid transition. They are capabilities, not a
global prerequisite for unrelated work.

---

## Core consequence

A pending review identifies the exact change, material assumptions, reviewed
claim, and downstream consequence that the review is intended to evaluate.

Conceptually:

```text
implemented change
      ↓
deterministic evidence
      ↓
durable review request bound to an immutable subject
      ↓
review against that subject and its material assumptions
      ↓
applicability judgment for consequential use
      ↓
reuse, refresh, composition, supersession, or rejection
```

A later slice does not change what an immutable review result established. If
that result is used against a later candidate, the consumer establishes whether
the reviewed assumptions and consequence still apply. A confirmed conflict
prevents stale evidence from authorizing the integrated state; absence of a
conflict need not block unrelated work.

> **A review result is valid only for the code, assumptions, and downstream
> consequence identified by its review snapshot.**

---

## Role and authority boundaries

### Builder

The builder discovers and proposes the slice's expected mutation scope. When
implementation exposes necessary scope outside the accepted plan, it returns a
boundary-change request instead of expanding silently.

### Campaign supervisor

The supervisor:

- observes durable review evidence when it is material to the decision;
- establishes whether a review remains applicable before consequential reuse;
- may use overlap analysis and acquire or revise reservations when a concrete
  concurrency or authority constraint warrants coordination;
- rechecks applicability when material assumptions change;
- schedules or resumes review when the required capability is available; and
- decides the campaign consequence of the review result within its authority.

The supervisor does not adjudicate architectural overlap by filename alone and
does not silently supersede protected work.

### Reviewer

The reviewer receives the accepted review snapshot, deterministic evidence,
scope, material assumptions, and later finding dispositions. It does not
mutate the repository or redefine the slice boundary.

### Strategic planner

The strategic planner consumes compact queue consequences when review backlog,
scope contention, capability availability, or blocked foundational work could
change roadmap priority or campaign selection.

It does not enforce operational reservations.

### Human authority

Human approval is required when resolving overlap would discard or supersede
user-owned work, change a consequential product decision, amend authority, or
otherwise exceed the supervisor's delegated scope.

---

## Review request as a durable snapshot

A review request should identify both the obligation and the thing to be
reviewed.

An illustrative shape is:

```text
review_id
campaign_id
run_id
slice_number
attempt_id

state
created_at
updated_at
queue_revision

review_requirement
review_provider
independence_requirement
capability_state
available_after

base_revision
workspace_snapshot_id
task_owned_files
before_content_digests
after_content_digests
patch_digest

affected_symbols
affected_components
affected_contracts
affected_state_owners
affected_generated_artifacts
material_consumers

reviewed_claim
material_assumptions
established_consequence
limitations
invalidation_conditions

placement_certificate
accepted_plan_version
deterministic_evidence_refs
open_findings

protected_scope_id
supersedes
superseded_by
composes
result_ref
```

Not every repository can identify every semantic dimension mechanically. An
unavailable field remains unavailable rather than being guessed.

The snapshot should preserve references and digests, not raw model transcripts.
The review artifact owns the claim, assumptions, established consequence,
limitations, and refresh, supersession, or composition lineage. The checkpoint
artifact owns immutable content identity and reconstruction provenance. Together
they contain enough evidence for a later consumer to judge applicability
without silently retargeting the original result.

`git-backed-slice-checkpoints.md` develops one concrete representation for
this identity: immutable candidate checkpoints before review, replacement
checkpoints after repair, and one accepted checkpoint for continuation, all
created without moving the user's branch or taking ownership of unrelated
workspace state.

---

## Review-queue states

The exact vocabulary belongs to the eventual state owner. A useful open model
includes:

```text
pending
awaiting_capability
available
leased_for_review
in_review
awaiting_remediation
completed
stale
superseded
cancelled
```

These are live operational states, not campaign terminal receipts.

Provider quota exhaustion after completed implementation can therefore produce:

```text
slice attempt: awaiting_review
review request: awaiting_capability
campaign: active but blocked on this protected scope
```

It need not immediately produce a final stopped campaign.

If an older runtime has already written an immutable stopped terminal, later
review must create a new recovery attempt or campaign lineage. It must not
rewrite the historical terminal as though the stop never occurred.

---

## Mutation scope

The useful abstraction is broader than a changed-file list.

An accepted mutation scope may include:

```text
files
directories
symbols
components
schemas and contracts
state owners
interfaces
generated artifacts
material producers and consumers
```

The scope should be the smallest boundary that truthfully covers the intended
change and the evidence that will establish it.

An overly broad scope blocks unrelated work. An artificially narrow scope
permits semantic conflicts and makes review evidence unreliable.

---

## Overlap detection

Overlap evidence has different strengths.

### Mechanically decidable overlap

Examples include:

- the same file or directory;
- a rename of a protected path;
- the same generated artifact;
- a changed content digest inside the review snapshot;
- the same deterministically identified symbol or schema;
- incompatible exclusive resource claims.

Confirmed mechanical overlap is strong coordination evidence. It blocks
mutation automatically only where a governing concurrency, exclusive-resource,
or authority contract makes that transition invalid; otherwise it informs the
applicability and coordination judgment.

### Semantic overlap

Examples include:

- different files implementing the same state owner;
- a producer changed while its protected consumer is under review;
- a shared interface or lifecycle transition changed indirectly;
- a test bootstrap or generated source invalidating prior evidence;
- a different implementation route changing a placement premise.

Repository-relationship evidence can identify candidates, but model judgment
may be required to decide whether they materially invalidate the review.

Semantic overlap should not be mechanically reduced to an exhaustive closed
taxonomy. The supervisor records observed relationships, the overlap decision,
and its rationale.

### Unresolved material overlap

If available evidence cannot establish compatibility, the uncertainty remains
explicit. It prevents consequential reuse of the review when the missing fact
could invalidate its claim. Mutation itself is blocked only when an identified
concurrency or authority invariant requires that outcome; unrelated work is not
blocked merely because semantic non-overlap cannot be exhaustively proven.

---

## Scope reservations

Reservations are one available mechanism for coordinating an accepted mutation
scope. They are warranted when an actual shared-write race, exclusive resource,
or authority boundary needs an atomic claim to prevent an invalid transition.
Review truth does not itself require every mutating plan to acquire one.

An illustrative reservation includes:

```text
reservation_id
owner_agent_id
campaign_id
slice_attempt_id
plan_version
scope
scope_digest
mode
queue_revision
lease_version
acquired_at
renewed_at
lease_expires_at
status
```

Execution claims may use bounded leases so crashed agents can be recovered.
Lease expiry must not silently authorize conflicting mutation. Expiry produces
a state requiring deterministic recovery or ownership transfer.

A pending review's protected scope has a different lifetime. It remains
protected until the review completes, becomes stale, is explicitly superseded,
or is cancelled by the appropriate authority. Provider unavailability must not
cause protection to disappear merely because time passed.

---

## Initial plan acceptance

When an identified concurrency or authority contract requires reservation, a
possible acceptance route is:

```text
proposed mutation scope
        +
current review-queue revision
        +
active reservations
        ↓
compatibility decision
        ↓
atomic scope reservation
        ↓
accepted plan version
```

An accepted plan records any queue revision and reservation identity that were
material to its decision. Plans without that causal coordination need remain
bound only by their actual authority, workspace-integrity, and evidence
contracts.

---

## Mid-flight plan changes

Review-queue evidence matters when a changed plan could affect a pending review
or when the plan relies on that review. It is not a global preflight for every
route revision.

When evidence invalidates the accepted boundary or reveals necessary new
scope, the builder pauses mutation and returns a boundary-change request.

The supervisor then:

1. preserves evidence that remains valid;
2. marks scope-dependent plan decisions stale;
3. proposes the revised mutation scope;
4. observes relevant pending reviews and any active reservations;
5. determines whether their claims or protected transitions are affected;
6. revises a reservation atomically when its causal contract requires one; and
7. reaccepts the new plan version before mutation resumes when the accepted
   boundary or authority changed.

Where a reservation protects an actual race or authority boundary, the old
reservation remains effective until its replacement is acquired or the plan is
abandoned. This is a property of that concurrency contract, not a universal
review-validity requirement.

The same consequence applies to:

- route reopening;
- repair work after a failed check;
- remediation after review findings;
- generated-artifact expansion;
- migration scope discovered during execution; and
- replacement builders inheriting an active attempt.

> **A plan and any review evidence it consumes remain valid only while the
> assumptions, authority, and consequences on which the decision depended
> remain applicable.**

---

## Queue changes after plan acceptance

A new review request may enter the queue after another plan was accepted.

When queue insertion transfers an existing exclusive claim whose temporary
release would permit an invalid concurrent transition, insertion and transfer
share one concurrency boundary. Other review requests need only preserve their
immutable subject and applicability metadata; they do not create a global
mutation lock.

Before consequentially reusing a review result, the supervisor establishes its
applicability to the decision state. A changed queue revision or worktree is
evidence to interpret, not automatic proof of conflict or staleness.

This keeps queue observation adaptive without polling the entire strategy on
every file write.

---

## Valid conflict dispositions

When a proposed slice overlaps protected review scope, the supervisor may:

- wait for the review;
- prioritize the queued review when capability is available;
- narrow the new slice and prove non-overlap;
- reorder other non-conflicting work;
- combine the work into a new explicitly reviewed final state;
- supersede the old review through an authorized lineage transition; or
- stop for human judgment when ownership or product intent is consequential.

It may not:

- ignore the queued review;
- treat provider delay as release of the protected scope;
- review an obsolete snapshot and apply the result to newer code;
- silently merge two scopes under one slice's identity;
- rewrite a stopped or superseded history as accepted; or
- claim non-overlap solely because filenames differ.

---

## Original truth, later applicability, and integrated acceptance

Before review begins, the reviewer verifies the immutable subject it received.
The resulting review remains truthful about that subject even when the worktree
or a later candidate changes. Before consequential use against a different
state, the consumer judges whether changes to material assumptions, producers,
consumers, contracts, or placement affect the established consequence.

A conflict makes the old result inapplicable to the integrated state; it does
not falsify the old result. A non-conflicting change may retain the result with
a recorded applicability judgment.

The system may then:

- create a replacement review request for the new snapshot;
- compose the changes and review the combined state;
- preserve still-valid findings with explicit lineage; and
- supersede the obsolete request without erasing its history.

Review results should identify the exact snapshot and review attempt they
establish. Acceptance must not detach a result from that identity.

### Three applicability cases

**Non-conflicting continuation.** Review `R0` establishes claim `C0` for
immutable subject `S0`, assuming `A0`, with consequence `K0`. Later change
`delta-1` produces `S1` without changing `A0` or `K0`. The consumer records that
`R0` remains applicable to the relevant decision and work continues. This does
not broaden or rewrite the original truth `R0(S0)`.

**Conflicting integrated change.** Later change `delta-2` produces `S2` and
changes a material producer, consumer, contract, or assumption in `A0`. The
original `R0(S0)` remains truthful, but it cannot authorize `S2`. Acceptance of
the integrated state requires a refreshed review, explicit composition with new
evidence, or a provenance-bearing superseding result.

**Delayed immutable review.** A delayed reviewer evaluates immutable `S0` while
the working tree later becomes `W1`. Its result continues to name `S0`, `A0`,
and `K0`, so the worktree change neither retargets nor falsifies it. Any use
against `W1` or a derived candidate receives a separate applicability decision
and lineage outcome.

---

## Duplicate review prevention and capability windows

A review worker acquires a queue lease before invoking a limited provider.
The lease prevents two sessions from spending the same capability window on
the same obligation.

Idempotency should bind at least:

```text
review_id
review_attempt_id
snapshot_digest
review_requirement
provider identity
```

Provider availability may be recorded as a hint such as `available_after`, but
it is not guaranteed capacity. A failed attempt records its exact cause and
returns the request to a truthful waiting state when retry remains authorized.

A queue may prioritize work using signals such as downstream blockage,
foundation importance, age, cost, and capability fit. Those signals should
inform judgment rather than becoming a universal numeric scheduling formula.

---

## Relationship to persistent agent state

Persistent agent state supplies the live identities and transitions needed to
make this queue reliable:

- campaign, supervisor, builder, reviewer, and attempt identities;
- current phase and plan version;
- active child bindings;
- handled events and idempotency keys;
- current mutation reservation;
- waiting reason and capability state;
- review request and review-attempt bindings;
- baseline and workspace snapshot references;
- transition history and recovery ownership.

The review queue should be durable state referenced by agents, not an implicit
list reconstructed from conversations.

After compaction or restart, a supervisor should hydrate:

```text
active plan and plan version
current reservation
pending review obligations
protected scopes
queue revision
blocked relationships
available review results
```

Only then can it decide whether to continue, replan, review, or recover.

This makes review-queue integration an early consumer and proving case for the
persistent-agent-state architecture.

---

## Relationship to strategic planning

The strategic planner does not need raw queue records by default. It needs
compact consequences such as:

```text
foundational work awaiting review
downstream slices blocked by protected scope
provider window or capacity limitation
review backlog age and growth
repeated stale or superseded reviews
scope-contention pattern
```

These facts may change which campaign should run next or whether stabilizing
review capacity is more valuable than producing additional implementation.

Operational queue state remains owned by the runtime and supervisor.

---

## Recovery example

```text
slice 15 implementation completes
        ↓
deterministic gate passes
        ↓
independent reviewer is quota-limited
        ↓
review request becomes awaiting_capability
        ↓
slice scope converts to durable review protection
        ↓
later slice proposes overlapping mutation
        ↓
supervisor judges applicability and any actual concurrency or authority conflict
        ↓
review capability becomes available
        ↓
review worker leases exact snapshot
        ↓
review result binds to snapshot digest
        ↓
supervisor remediates or accepts through the owning lifecycle
```

If the historical implementation already finalized the campaign as stopped,
the later acceptance occurs through a new recovery attempt or campaign. The
old terminal remains truthful.

---

## Initial implementation boundary

A useful first version does not require a general scheduler or exhaustive
semantic-lock engine.

Its core evidence-validity boundary needs:

1. durable agent, campaign, slice-attempt, and review-request identities;
2. review snapshot digests and task-owned file manifests;
3. reviewed claims, material assumptions, established consequences,
   limitations, and lineage;
4. a monotonic review-queue revision;
5. overlap evidence and explicit semantic-scope candidates for supervisor
   judgment;
6. durable `awaiting_review` and `awaiting_capability` states;
7. review-attempt leasing and idempotency;
8. applicability judgments before consequential reuse; and
9. recovery that hydrates pending obligations and their immutable subjects.

Atomic reservation acquisition and transactional revision can extend this
boundary when a demonstrated shared-write race, exclusive resource, or
authority constraint requires them.

Symbols, components, contracts, and state-owner relationships can expand after
the file-level lifecycle is proven, provided the initial schema does not make
those dimensions impossible to add.

---

## Evidence of completion

The initial capability is convincing when executable evidence demonstrates:

- quota exhaustion queues a review without losing completed deterministic
  work;
- the queued review retains its immutable subject, assumptions, consequence,
  limitations, and lineage across restart or compaction;
- a non-conflicting later change continues with recorded applicability;
- a conflicting integrated change cannot reuse the old review for acceptance;
- delayed review remains truthful about its immutable subject after later
  worktree mutation;
- reservations prevent the concrete invalid transition in scenarios where a
  concurrency or authority contract actually requires them;
- duplicate review attempts are suppressed;
- a later successful review binds to the exact queued snapshot; and
- historical stopped or superseded state remains unchanged and explainable.

---

## Metrics worth preserving

Useful observations include:

- queue wait time separated from active review time;
- capability-unavailable attempts and causes;
- pending reviews by protected boundary;
- slices blocked, narrowed, reordered, or superseded because of overlap;
- mechanical versus semantic overlap decisions;
- stale review count and cause;
- duplicate review attempts suppressed;
- reservation conflicts and recovery transfers;
- time from deterministic readiness to accepted review result; and
- downstream work unblocked by review completion.

Metrics inform scheduling and architecture. They do not determine acceptance or
silently override authority.

---

## Open design questions

- Which state store owns queue transactions and reservation compare-and-swap?
- What is the minimum durable workspace snapshot: content digests, a patch
  object, a temporary commit, or another immutable artifact?
- Which semantic scope dimensions are reliable enough for mechanical matching,
  and which remain model judgments?
- How should long-lived protected review scope interact with urgent fixes?
- What authority may supersede an obsolete review automatically, if any?
- When may still-valid findings transfer to a replacement snapshot?
- How should reservation recovery distinguish a crashed agent from a slow but
  live agent?
- Which queue summaries materially improve strategic planning without flooding
  the planner with operational detail?

---

## Compact principle

> **Queue the unmet review obligation, bind it to the immutable state it is
> meant to judge, and establish applicability before consequential reuse.
> Coordinate mutation when an identified concurrency or authority contract
> requires it.**
