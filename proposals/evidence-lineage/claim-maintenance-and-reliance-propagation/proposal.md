# Proposal: Claim Maintenance and Reliance Propagation

## Identity and state

- Proposal ID: `work-engine.claim-maintenance-and-reliance-propagation`
- Family ID: `work-engine.evidence-lineage`
- State: formed; probable split ownership across shared claim semantics, domain
  workflows, and general delivery; not evaluated, prioritized, accepted, or
  authorized for implementation
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. Supporting artifacts describe its [placement](placement.md)
and [relationships](relationships.md).

The user authorized formation of both production phases with the stated intent
to complete the entire claim-lineage consequence. This proposal preserves the
second phase as committed product direction for later evaluation; formation
does not itself prioritize, accept, schedule, or authorize implementation.

## Candidate and consequence

Extend the production claim-evidence interface into an active, truthful
maintenance system. Repository, contract, external, proposal, review, or
runtime events can nominate possible effects on exact claim revisions;
authorized domain roles can investigate and record unchanged, changed,
inapplicable, insufficient, contested, deferred, or superseded outcomes; and
downstream consumers can selectively reconsider exact-version reliance through
durable obligations without raw change signals acquiring semantic or execution
authority.

The completed system preserves every stage from observed change through
candidate impact, refresh, resulting support, reliance, applicability,
reopening, delivery, and semantic completion. It remains truthful under
branching, duplication, out-of-order events, partial publication, inactive
recipients, authority loss, and context replacement.

## Dependency and independently decidable boundary

This proposal depends on
[`production-claim-evidence-interface`](../production-claim-evidence-interface/proposal.md).
It consumes that proposal's stable identities, immutable revisions, authority
profiles, exact-revision reliance, publication mechanics, and projection
provenance. It does not redefine them.

The phase is independently decidable because its consequence is operational:
maintain and propagate the implications of already-productionized claims. The
foundation remains useful if this phase is delayed; this phase is invalid
without the foundation. Acceptance of phase one therefore neither accepts nor
completes this proposal.

## Formation evidence

Repository evidence cutoff for the cited existing artifacts:
`cdc9e3fa5d300e5edc737faf38edf85a336fbdcf`.

Formation used the reviewed semantic parent's event, refresh, reliance,
obligation, delivery, and recovery model; its still-binding deferred decision;
the final two-fixture dogfood report; and the simultaneously formed production
interface dependency. The dogfood positively represented the four backbone
behaviors but explicitly did not exercise real consumers, adversarial
concurrency, full delivery recovery, or outcome-independent falsification.
Those limitations become this proposal's evidence and acceptance boundary
rather than disappearing as deferred implementation detail.

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

The domain's authorized maintenance owner records an explicit canonical-support
selection, contested set, or absence against exact predecessor revisions. That
selection is itself immutable, attributed, authority-bound, and replaceable by
a successor. Authority revocation prevents new transitions under the revoked
grant but does not erase judgments validly published while it was effective.

Correction, supersession, composition, and identity forks retain their distinct
topologies and never conceal a changed subject, question, quantification, or
authority domain as an ordinary revision.

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
obligation pending or awaiting authority; it cannot disappear or become a false
terminal result.

The domain workflow owns the obligation's semantic consequence. The claim
capability owns the impact and reliance facts. A general control-plane delivery
capability may route the obligation but does not acquire claim meaning,
reopening authority, or role-activation authority.

## Publication, delivery, and recovery

Same-owner publication produces one recoverable durable consequence. Cross-owner
transfer preserves stable source identity, idempotent reception, durable
pending state, retry reconciliation, and proof that acknowledgement cannot
precede the receiving owner's protected consequence.

The mechanism may use atomic Git publication, transactions, outbox/inbox,
compare-and-swap state, or another valid realization. Correctness depends on
the protected outcome, not a fixed protocol.

Failures before or after nomination, episode opening, judgment, resulting
revision, reliance update, obligation publication, receipt, acknowledgement,
or completion are recoverable without duplicate semantic effect. Reconciliation
reports ambiguity when it cannot prove the protected consequence; it never
replays a possibly completed non-idempotent transition merely to discover
state.

## Context replacement and inactive roles

Canonical claim, judgment, reliance, and obligation owners—not model context—
preserve semantic continuation. Compaction before durable publication leaves
the unpublished judgment unavailable or uncertain. Compaction after durable
publication recovers exact state from the owner without treating a summary,
transcript, or retained provider session as the missing consequence.

A claim event does not activate an inactive role. When no authorized recipient
is active, the obligation remains pending. Scheduling or control-plane delivery
may surface it only under separately established role and activation authority.

## Query and projection consequences

The production projection adds bounded queries for:

- nominations by source event, target claim revision, disposition, or owner;
- open, awaiting, contested, deferred, and terminal refresh episodes;
- canonical-support selections and competing branches;
- exact and reverse reliance;
- candidate-impacted consumers at an exact source watermark;
- pending, delivered, acknowledged, completed, deferred, and superseded
  obligations; and
- provenance-bearing paths from source event through nomination, judgment,
  resulting revision, reliance, reopening, and completion.

Every query retains projection version, canonical inputs, per-source watermarks,
freshness, exclusions, failed inputs, and truncation. Partial traversal cannot
report complete absence or silently choose a branch.

## Ownership boundary

The shared claim-evidence capability owns:

- nomination, refresh, canonical-support-selection, reliance, and obligation
  record contracts;
- stable identities, typed relationships, validation, publication mechanics,
  and query projection semantics; and
- non-destructive history and cross-record integrity.

Domain workflows own:

- claim meaning, materiality, maintenance authority, refresh judgments,
  applicability, reliance decisions, selective reopening, and semantic
  completion.

General delivery machinery owns:

- transport admission, durable routing, receipt, retry, and delivery
  reconciliation under the supplied obligation identity.

Role state may reference exact revisions and pending obligations but does not
copy, advance, or own their semantic history. The scheduler may route timed
delivery but does not acquire claim semantics or activation authority.

## Invariants

- Mechanical impact never becomes semantic invalidation or causality.
- Opening, investigation, judgment, resulting revision, reliance,
  applicability, reopening, delivery, acknowledgement, and completion remain
  separately addressable consequences.
- Unchanged refresh produces a new evidence-bound revision without pretending
  the proposition changed.
- Changed support names only causality adjudicated by an authorized domain
  judgment.
- Reliance remains exact-versioned and never silently follows latest support.
- Branches, conflict, uncertainty, inactive recipients, and authority loss
  remain visible.
- Delivery is idempotent and acknowledgement cannot masquerade as semantic
  completion.
- Context replacement cannot erase or manufacture a durable semantic
  consequence.
- No phase-two mechanism changes proposal, review, roadmap, role-activation, or
  implementation authority by itself.

## Boundary

This proposal completes the operational concepts already formed in the parent
claim-centered evidence-lineage proposal. It still does not establish:

- continuous monitoring or a mandatory graph watcher;
- research maturity levels, readiness scoring, or organizational compilation;
- automatic acceptance, reopening, scheduling, role activation, or execution;
- one universal maintenance role across every claim domain; or
- a graph database or always-running runtime as canonical semantic state.

Those outcomes require their own independently owned product consequences if
they are later desired.

## Evidence and acceptance needs

Acceptance requires a real end-to-end lifecycle across both initial domain
profiles and an adversarial matrix that exercises:

- retained-unchanged and changed judgments;
- inapplicable, insufficient, contested, deferred, and superseded outcomes;
- concurrent and branching revisions, explicit conflict, identity forks, and
  canonical-support selection;
- duplicate and out-of-order nominations and deliveries;
- authority revocation and writer replacement;
- exact reliance retention, retirement, supersession, and selective reopening;
- inactive recipients and later authorized delivery;
- failures on both sides of every durable publication boundary;
- projection rebuild, schema migration, cycles, dangling or retracted
  references, partial watermarks, and stale reads; and
- context replacement before and after semantic publication.

The vertical proof follows at least one exact path from a real source event to
candidate impact, authorized refresh, resulting claim revision, existing
reliance, domain-owned reopening obligation, delivery, semantic disposition,
and truthful completion without any intermediary acquiring authority it does
not own.

## Uncertainty

- Which domain workflows need active canonical-support maintenance rather than
  immutable historical claims plus on-demand assessment.
- Whether one obligation record core remains truthful across proposal research,
  review, and later consumers.
- Which delivery affordances already available in the control plane can satisfy
  the cross-owner consequences without a claim-specific coordinator.
- What concurrency and scale require from the replaceable query projection.
- Which residual uncertainty downstream decision owners may accept without
  changing their existing authority contracts.

## Authority

This proposal is formed to preserve the full planned outcome, but it is not an
implementation authorization or an accepted roadmap commitment. It grants no
producer refresh authority, no consumer reopening authority, no delivery or
activation authority, and no permission to mutate proposal, review, role,
scheduler, or implementation state. Those consequences require acceptance and
the exact domain and integration contracts that own them.

## Acceptance consequence

If accepted and implemented after its production dependency, Work Engine can
maintain evidence-backed claims across real change, preserve unchanged and
changed support truthfully, identify exact downstream reliance, and route
selective reconsideration to the correct decision owner. The system remains
auditable and recoverable under conflict, partial failure, inactive recipients,
authority changes, and model-context loss without turning detected change,
transport, persistence, or projection output into semantic authority.
