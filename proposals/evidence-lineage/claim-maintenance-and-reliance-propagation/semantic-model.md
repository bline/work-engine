# Semantic Model: Claim Maintenance and Reliance Propagation

This document elaborates the semantic record and lifecycle model owned in
compressed form by [`proposal.md`](proposal.md). It does not independently own
proposal state, acceptance, priority, or implementation authority.

## Event and impact nomination

An immutable implementation completion, repository change, contract change,
proposal transition, external observation, review result, or runtime event may
source a stable impact nomination. The nomination:

- identifies one exact source event and revision;
- targets one exact claim revision through `may_affect`;
- records observed evidence and why impact is plausible;
- names its producer and evidence capability; and
- begins with a visible disposition independent of any refresh episode.

Nomination asserts candidate impact only. It cannot make the claim false,
stale, inapplicable, insufficient, reopened, refreshed, or causally changed.
Duplicate and late delivery preserve one semantic nomination consequence and
retain delivery history. Current disposition is derived from immutable
successors rather than destructive field mutation.

Nomination dispositions include pending, investigating, resolved unchanged,
resolved changed, inapplicable, insufficient evidence, contested, deferred,
and superseded. Terminal disposition cites the authorized judgment that
resolved it; unresolved outcomes remain visibly unresolved.

Retrieved graph relationships and derived impact candidates are evidence for
nomination, not canonical nominations by themselves. A `MAY_AFFECT` edge in a
replaceable query graph is either an explicitly marked derived cache edge or a
projection of an already canonical nomination.

## Refresh episode and judgment

A refresh episode has stable identity, exact subject revision, triggers,
domain profile, authorized owner, evidence cutoff, current writer generation,
and immutable lifecycle transitions. Opening records `reopened_by` only; it
does not prove investigation or a terminal result.

The lifecycle distinguishes:

```text
opened
  -> active / awaiting_evidence / awaiting_authority
  -> retained_unchanged / changed / inapplicable
  -> insufficient / contested / deferred / superseded
```

Every terminal outcome has an attributed judgment. `retained_unchanged` and
`changed` each produce exactly one new claim revision against the evidence
world actually examined. An unchanged refresh preserves the proposition while
updating its evidence support and does not emit `changed_because_of`.

A changed judgment records `changed_because_of` only for source events whose
causal effect the authorized domain owner actually adjudicated. Mechanical
reachability, temporal proximity, touched paths, projection output, or the
implementation producer cannot manufacture causality.

Inapplicable, insufficient, contested, and deferred outcomes produce no claim
revision. Supersession identifies the exact successor episode, judgment, or
revision. Every outcome resolves or visibly preserves each triggering
nomination.

## Branching, conflict, and canonical support

Concurrent writers may create competing proposed revisions or refresh
judgments only when the domain profile permits branching. The projection shows
every branch and conflict. No wall-clock ordering, file position, writer
priority, or largest revision label silently selects canonical support.

The domain's authorized maintenance owner records an explicit
canonical-support selection, contested set, or absence against exact
predecessor revisions. That selection is immutable, attributed,
authority-bound, and replaceable by a successor. Authority revocation prevents
new transitions under the revoked grant but does not erase judgments validly
published while it was effective.

Correction, supersession, composition, and identity forks retain their
distinct topologies and never conceal a changed subject, question,
quantification, or authority domain as an ordinary revision.

## Versioned reliance and applicability

A reliance record binds:

- one consumer artifact and immutable consumer revision;
- one decision scope and owning decision authority;
- one exact claim revision;
- the applicability assessment, if any, actually considered;
- accepted limitations or unresolved uncertainty; and
- active, retired, or superseded state.

Later claim revisions do not silently advance reliance. A candidate-impact
view evaluates the reliance revision active at its declared projection
watermark. After refresh, the downstream decision owner—not the claim owner,
projection, scheduler, or implementation producer—decides whether the new
revision applies and whether its artifact or decision must reopen.

Applicability assessment, reliance, readiness, and execution remain separate
judgments. An assessor may advise without acquiring reliance or transition
authority. A consumer may preserve an older exact reliance with visible
limitations when its domain contract permits that decision.

## Selective-reopening obligations

When candidate impact or changed support reaches an active exact-revision
reliance, the owning domain workflow may create a stable refresh or
selective-reopening obligation. Identity binds source event, reliance revision,
consumer, and obligation kind.

The lifecycle distinguishes publication, pending delivery, receipt,
acknowledgement, semantic disposition, completion, deferral, supersession, and
retirement. Acknowledging transport does not complete the semantic obligation.
Losing an active recipient, authority, runtime, or model context leaves the
obligation pending or awaiting authority; it cannot disappear or become a
false terminal result.

The domain workflow owns the obligation's semantic consequence. The claim
capability owns the impact and reliance facts. A general control-plane delivery
capability may route the obligation but does not acquire claim meaning,
reopening authority, or role-activation authority.

## Publication, delivery, and recovery

Same-owner publication produces one recoverable durable consequence.
Cross-owner transfer preserves stable source identity, idempotent reception,
durable pending state, retry reconciliation, and proof that acknowledgement
cannot precede the receiving owner's protected consequence.

The mechanism may use atomic Git publication, transactions, outbox/inbox,
compare-and-swap state, or another valid realization. Correctness depends on
the protected outcome, not a fixed protocol.

Failures before or after nomination, episode opening, judgment, resulting
revision, reliance update, obligation publication, receipt, acknowledgement,
or completion are recoverable without duplicate semantic effect.
Reconciliation reports ambiguity when it cannot prove the protected
consequence; it never replays a possibly completed non-idempotent transition
merely to discover state.

## Context replacement and inactive roles

Canonical claim, judgment, reliance, and obligation owners—not model
context—preserve semantic continuation. Compaction before durable publication
leaves the unpublished judgment unavailable or uncertain. Compaction after
durable publication recovers exact state from the owner without treating a
summary, transcript, or retained provider session as the missing consequence.

A claim event does not activate an inactive role. When no authorized recipient
is active, the obligation remains pending. Scheduling or control-plane delivery
may surface it only under separately established role and activation authority.

## Query and projection consequences

The production projection adds bounded queries for:

- nominations by source event, target claim revision, disposition, or owner;
- repository nominations by plan or scope revision, checkpoint, attributed
  path, stable entity, relationship, or unmapped candidate;
- open, awaiting, contested, deferred, and terminal refresh episodes;
- canonical-support selections and competing branches;
- exact and reverse reliance;
- candidate-impacted consumers at an exact source watermark;
- pending, delivered, acknowledged, completed, deferred, and superseded
  obligations; and
- provenance-bearing paths from source event through nomination, judgment,
  resulting revision, reliance, reopening, and completion.

Every query retains projection version, canonical inputs, per-source
watermarks, freshness, exclusions, failed inputs, and truncation. Partial
traversal cannot report complete absence or silently choose a branch.
