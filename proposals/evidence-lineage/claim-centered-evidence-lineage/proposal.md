# Proposal: Claim-Centered Evidence Lineage

## Identity and state

- Proposal ID: `work-engine.claim-centered-evidence-lineage`
- Family ID: `work-engine.evidence-lineage`
- State: permanent shared placement deferred for dogfooding; semantic review
  dispositions are owned under
  `reviews/proposals/claim-centered-evidence-lineage/`; this candidate is not
  accepted as a permanent owner or authorized for implementation
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. Supporting artifacts describe its [placement](placement.md)
and [relationships](relationships.md).

The existing authority decision is bound to an earlier proposal revision. Its
`defer_for_dogfooding` disposition, uncertain placement, and non-authorization
remain in force. They do not constitute review, approval, or implementation
authority for the expanded event model in this later semantic amendment.

## Candidate and consequence

Define a minimal durable evidence-lineage contract for evidence-backed semantic
statements such as proposal-research claims and specialist review findings. A
consumer can identify the exact statement and evidence world examined, who
produced the judgment under which authority and limitations, which changes may
warrant reopening it, and how later judgments retain, contest, refresh,
compose, supersede, correct, or invalidate it without rewriting history.

The contract preserves evidence and attributed judgment. It does not decide
whether a statement is true, fresh, applicable, sufficient, accepted, or
authoritative merely because it is structurally valid, durable, or reused.

## Proposed product change

Define the smallest human-readable, mechanically checkable shared surface that
real research and review consumers can dogfood. Candidate semantics include:

- a bounded subject identity with repository or source namespace, subject kind,
  stable subject ID, immutable evidence baseline, and content-set identity;
- stable identity for each material statement and its exact revision;
- the conclusion or finding, material assumptions, limitations, confidence or
  support qualification, and evidence references;
- producer identity, actual evidence mode, judgment kind, decision scope, and
  the authority reference under which the judgment was made;
- sensitivity surfaces and semantic reopening conditions proportionate to the
  statement;
- explicit, non-destructive lineage among statement revisions and judgments;
  and
- a distinction between the revision containing the durable record and the
  earlier repository, external, contract, proposal, or runtime baseline that
  the evidence examined.

The initial contract should support recordability and decision-triggered
revalidation. It does not imply continuous monitoring, a graph watcher,
automatic semantic invalidation, or a runtime service.

## Versioned epistemic history

A stable claim identity persists across immutable claim revisions. Each
revision binds the proposition and its support qualification to the exact
evidence baseline, producer, authority scope, assumptions, limitations, and
sensitivity surface examined at that time.

```text
claim C17
  C17/r1  supported against evidence baseline A
  C17/r2  refreshed against evidence baseline B
  C17/r3  revised against evidence baseline C
```

The enclosing proposal, review, plan, or architecture artifact may reference an
exact claim revision. It does not absorb the claim's history or silently follow
the claim's latest revision. Later use must establish applicability to the
consumer's exact decision.

This is an epistemic history, not merely a sequence of snapshots. It preserves
which proposition persisted, why it was reconsidered, what evidence changed,
whether the conclusion held, and which attributed judgment established any
semantic causal relationship.

### Stable claim identity boundary

A stable claim identity names a durable question, not whichever conclusion
happens to be current. Its identity boundary includes:

- an owning namespace and claim kind;
- a bounded subject identity;
- the question or proposition intent being evaluated;
- material quantification and scope; and
- the domain contract that defines permitted revisions.

A revision may change evidence, support qualification, limitations, confidence,
freshness, or the answer to that same bounded question. A change to the subject,
question, material quantification, or authority domain creates a new stable
claim identity with an explicit derivation, replacement, or correction
relationship. It must not inherit the old identity merely to preserve convenient
downstream links.

Claim, revision, nomination, refresh episode, judgment, reliance, and obligation
identities are collision-safe immutable identifiers independent of display
ordinals such as `C17/r3`. Branch-local counters are labels only. The revision
record identifies its enclosing immutable Git object or separately published
transition receipt; it does not require a self-referential digest of the commit
that contains itself.

### Type-specific relationship topology

The shared vocabulary does not make every relationship universally
many-to-many:

| Relationship | Source → target | Topology consequence |
| --- | --- | --- |
| `refreshes` | claim revision → prior claim revision | same stable claim; one direct predecessor; acyclic |
| `corrects` | claim revision → prior revision | same claim when the bounded question is conserved; otherwise the source has a new claim identity; acyclic |
| `supersedes` | claim revision → prior claim revision | may cross claim identities; does not pretend identity continuity; acyclic |
| `composes` | composition judgment/revision → constituent revisions | one or more immutable constituents; no implied revision ancestry or joint review |
| `may_affect` | impact nomination → exact claim revision | one nomination may target several revisions through distinct edges; candidate impact only |
| `sourced_by` | impact nomination → implementation completion or other immutable source event | one authoritative source event per nomination; does not imply semantic causality |
| `reopened_by` | refresh episode → nomination or other immutable trigger | one episode may have several triggers; proves opening/triggering only |
| `produces` | terminal refresh judgment → resulting claim revision | exactly one for `retained_unchanged` or `changed`; absent for enumerated zero-result outcomes |
| `changed_because_of` | authorized refresh judgment → causal event | one judgment may cite several adjudicated causes; never emitted by a projection |
| `relies_on` | versioned consumer reliance → exact claim revision | exact revision and decision scope; never silently follows latest |

All edges have their own collision-safe identity, endpoint types, schema
version, producer provenance, and authority scope. Self-edges are forbidden.
Revision ancestry, correction, and supersession graphs are acyclic. Canonical
publication rejects missing endpoints; staged cross-owner delivery may carry an
unresolved reference only inside a separately identified pending obligation,
not as an established lineage edge.

## Epistemic event model

Implementation completion, repository change, contract change, proposal
transition, external observation, and later review may nominate effects on
claims. Impact nomination, investigation, and semantic causality remain
different events:

### `may_affect`

An implementation completion record or deterministic impact process may
identify an exact claim revision as plausibly affected and give its change
evidence. This produces candidate impact only. It does not establish that the
claim is false, stale, inapplicable, or insufficient.

```text
nomination N42 --sourced_by--> implementation I42
nomination N42 --may_affect--> C17/r2
```

### `reopened_by`

An attributed refresh episode records which impact nomination, implementation,
contract transition, external event, or decision need opened revalidation of an
exact claim revision. This proves that the episode was triggered, not that
substantive investigation or a terminal judgment occurred, and it does not
imply that the conclusion changed.

```text
refresh J9
  subject: C17/r2
  reopened_by: N42
```

### Refreshed but unchanged

When investigation re-establishes the same conclusion against a newer evidence
world, it creates a new claim revision with a `refreshes` relationship and an
explicit `retained_unchanged` outcome. The newer baseline and support are
durable even though the proposition text is unchanged.

```text
C17/r3 --refreshes--> C17/r2
C17/r3 outcome: retained_unchanged
C17/r3 evidence baseline: B
C17/r3 changed_because_of: absent
```

This distinguishes “investigated and still supported” from “nobody
investigated.” It also permits measurement of stability without manufacturing
a semantic change.

### `changed_because_of`

When an authorized semantic judgment concludes that one or more events caused
the supported conclusion to change, the terminal refresh judgment records the
adjudicated causal inputs and produces a new claim revision. The revision
records the appropriate non-destructive relation to the prior revision and an
integrity-bound reference to that judgment.

```text
refresh judgment J10 --changed_because_of--> I42
refresh judgment J10 --produces--> C17/r3
C17/r3 --corrects or supersedes--> C17/r2
```

An implementation may produce an impact nomination carrying `may_affect`; it cannot assign
`changed_because_of` merely by completing. The domain's authorized evidence or
research owner produces the refresh judgment. Temporal proximity, touched
files, or graph reachability never establishes semantic causality by itself.

Many-to-many causality is preserved through typed edges rather than permissive
revision ancestry. One refresh judgment may adjudicate several implementation,
contract, proposal, external, or review events. One event may be nominated
against claims in several domains. Composition retains every constituent and
its distinct baseline without forcing a false one-change/one-claim history.

## Event, refresh, reliance, and obligation lifecycles

Every impact nomination has a stable identity, one source event and revision,
target claim revision, producer, evidence, and current disposition. Its
disposition distinguishes:

```text
pending
investigating
resolved_unchanged
resolved_changed
inapplicable
insufficient_evidence
contested
deferred
superseded
```

Nomination disposition changes are immutable successor records. “Current
disposition” is a derived projection over that append-only lineage, not a
mutable field that erases earlier states. Terminal dispositions reference the authorized judgment that resolved the
nomination. Unresolved, contested, insufficient, and deferred outcomes remain
visible; they are not reported as unchanged. Duplicate or late nominations are
idempotently related to the same source event and may be superseded without
erasing their delivery history.

A refresh episode has its own identity, exact subject revision, triggers,
authorized owner, evidence cutoff, and lifecycle:

```text
opened -> active / awaiting_evidence / awaiting_authority
       -> retained_unchanged / changed / inapplicable
       -> insufficient / contested / deferred / superseded
```

Episode state changes are likewise immutable attributed successor records from
which current state is derived. Every terminal episode has an attributed refresh
judgment. A `retained_unchanged` or `changed` judgment that establishes the new
canonical support state produces exactly one resulting claim revision.
`inapplicable`, `insufficient`, `contested`, or `deferred` outcomes produce no
claim revision. `superseded` produces no revision of its own and references the
successor episode, judgment, or claim revision that displaced it. Every outcome
resolves or visibly preserves each associated nomination.

A downstream reliance record is also versioned. It binds consumer artifact and
revision, decision scope, exact claim revision, any applicability assessment,
owning decision authority, and an `active`, `retired`, or `superseded` state.
Changing reliance creates a successor record; it does not rewrite which claim
revision governed an earlier decision. Candidate-impact projections evaluate
the reliance revision that was active at their declared source watermark.

A refresh or selective-reopening obligation has a stable identity derived from
its source event, reliance revision, consumer, and obligation kind. Delivery,
acknowledgement, semantic disposition, and completion are separate states. Loss
of an active recipient leaves the obligation pending rather than completing or
discarding it.

Publication and delivery must not lose an obligation, expose a false terminal
state, duplicate a semantic consequence, or let acknowledgement masquerade as
completion. Same-owner records require one recoverable publication consequence.
Cross-owner transfer requires stable source identity, idempotent reception,
durable pending state, retry reconciliation, and proof that acknowledgement
cannot precede the receiving owner's protected consequence.

An atomic Git commit or transaction is one possible same-owner realization. An
idempotent outbox and receiving-side inbox is one possible cross-owner
realization. These are dogfood candidates, not required shared architecture;
another mechanism may satisfy the protected outcomes truthfully.

## Downstream reliance and selective reopening

A downstream artifact references exact claim revisions:

```text
proposal P9
  relies_on: C17/r2, C24/r5, C81/r1
```

If an implementation produces a nomination that targets `C17/r2` through
`may_affect`, a derived view may say that `P9` relies on a candidate-impacted
claim. It must not mutate or reopen the proposal. After an authorized refresh
records an unchanged or changed claim revision, the downstream decision owner
judges applicability, readiness, and whether `P9` requires a separately
authorized reopening consequence.

The propagation chain is therefore:

```text
implementation or other event
  -> candidate claim impact
  -> attributed refresh episode
  -> retained_unchanged: one refreshed revision; candidate warning resolved
  -> changed: one revised revision; affected reliance identified
  -> inapplicable: scoped nomination resolved without a claim revision
  -> insufficient / contested / deferred: warning remains visibly unresolved
  -> superseded: successor disposition identified
  -> exact versioned downstream reliance
  -> decision-specific applicability and selective reopening judgment
```

This semantic layer prevents commit-level change signals from propagating
directly into proposal, plan, architecture, or execution authority.

## Canonical history and query projections

The typed history requires graph-shaped queries, but that does not select
a graph database as its canonical owner. The first dogfood may use Git-backed
claim, judgment, and edge artifacts while building a replaceable query
projection in memory, SQLite, a graph store, or another suitable index.

```text
canonical claims, revisions, judgments, and edges
  -> replaceable query projection
  -> bounded consumer queries
```

Canonical input includes versioned claim identities, revisions, judgments,
impact nominations, reliance records, obligations, and typed edges. Records
remain reachable while referenced; correction and retirement use explicit
successors, retractions, or tombstones rather than destructive disappearance.

Projection rebuilding uses declared schema versions, deterministic ordering by
immutable identity and causal predecessor rather than wall-clock arrival,
duplicate suppression, cycle and dangling-reference rejection, and explicit
migration rules. Every projection exposes its build version, source identities,
per-source completeness watermarks, excluded or failed inputs, and freshness.
A consumer requiring complete traversal fails closed or reports a visibly
partial/stale result when those conditions are not met.

The projection may accelerate traversal and derived candidate-impact views. It
does not own claim meaning, semantic causality, authority, or history. Every
derived result carries projection provenance and cannot be promoted into
`changed_because_of` without a separately attributed judgment. Query
requirements and observed scale should discriminate storage and indexing
choices after dogfooding; the current repository's SQLite, JSONL, Git, or graph
machinery must not become permanent doctrine merely because it is available.

## Refresh obligations and delivery

A candidate impact or changed claim may create a durable obligation for a
domain consumer currently relying on that exact revision. Evidence lineage owns
the impact and refresh facts; the relevant domain workflow owns the semantic
obligation; role state may reference the relied-upon revision and pending
obligation. A shared control-plane delivery capability may route it to an
active authorized role.

The scheduler does not acquire claim semantics merely because scheduled work
and claim-impact obligations may share delivery machinery. A claim event does
not activate a role, create authority, interrupt execution, or prove a refresh
completed. When no authorized recipient is active, the obligation remains
durably pending under its owning workflow.

## Invariants

### Subject and evidence baseline are explicit

Reason: a repository commit can identify a world without identifying whether a
claim examined the whole tree, a proposal family, selected paths, a bounded
query, an implementation delta, or external evidence.

Required property: every material statement names both its bounded subject and
the immutable baseline it actually examined. The later revision that stores the
record remains a separate identity.

### Statements remain independently addressable

Reason: artifact-level identity cannot truthfully preserve partial
applicability, selective correction, conflict, or supersession.

Required property: each material claim or finding has stable identity, support,
assumptions, limitations, and lineage that can be referenced without treating
the enclosing artifact as one indivisible judgment.

### Attribution and authority remain separate

Reason: knowing who produced a judgment does not establish that the producer
could make it for every purpose.

Required property: an artifact records both producer provenance and the
authority scope for the exact judgment. Domain contracts name the roles. An
unresolved owner remains unresolved rather than being inferred from possession
of the packet, compiler, validator, planner, coordinator, or supervisor role.

### Assessment and downstream reliance remain distinct

Reason: maintaining a canonical live claim, assessing whether evidence applies
to a particular decision, relying on that assessment, and deciding that a
transition is ready have different consequences. Not every immutable
historical statement requires an active freshness-maintenance role.

Required property: a domain profile names an authorized maintenance role when
it owns a canonical live claim and may name an authorized role to produce an
attributed advisory applicability assessment. The downstream decision owner
judges whether to rely on that assessment, whether evidence is sufficient and
ready for its exact transition, and whether to accept residual uncertainty
within authority it already has. Producing an assessment does not acquire the
authority to rely on it or execute the transition. Contract changes remain
with the owning contract's required human authority.

### Mechanical impact is not semantic invalidation

Reason: a changed file, graph result, contract digest, proposal state, or
runtime observation can reopen a statement without proving it false.

Required property: deterministic machinery may verify shape and references or
emit candidate-impact evidence. An attributed, authorized judgment retains,
contests, refreshes, supersedes, corrects, or invalidates the semantic
statement.

### Lineage does not erase history

Reason: later preference, factual defect, partial reuse, combined evidence, and
new observation are different consequences.

Required property: applicability, partial applicability, refresh, composition,
supersession, correction, and invalidation remain distinguishable. A domain
profile may narrow allowed relations, but may not use one relation to conceal a
different semantic transition. Composition identifies every constituent
statement revision, its distinct subject and evidence baseline, and known
coverage gaps; it never implies that an integrated state was jointly reviewed
when only its constituents were examined separately.

## Boundary and placement

The probable first representation is a Git-backed sibling artifact referenced
by proposal packets and review artifacts rather than absorbed into either
manifest. The permanent semantic owner remains uncertain. A shared
evidence-lineage capability is plausible because proposal research and review
artifacts need overlapping identity and lineage properties, but no implemented
pair of consumers yet proves their minimal common contract.

This candidate owns only shared evidence-lineage semantics if accepted. It does
not own:

- proposal identity, current meaning, lifecycle, or decision authority;
- research maturity levels, readiness profiles, or refresh workflow;
- review severity, reviewer episodes, omission outcomes, or synthesis;
- the semantic judgment that makes a claim, finding, freshness,
  applicability, or readiness conclusion;
- continuous dependency monitoring or runtime coordination; or
- roadmap, organizational-compilation, activation, or implementation decisions.

## Relationships

Revision-bound review artifacts causally require statement identity, evidence
baselines, provenance, sensitivity, authority scope, and truthful lineage. If
this candidate is later accepted as the shared semantic owner, it enables that
profile by supplying those consequences. The causal requirement attaches to
the semantics, not to acceptance of this particular placement hypothesis.
Proposal packets are a neighboring owner: they may reference this evidence
while retaining the proposal's current meaning and lifecycle.

A later proposal-research-maturity candidate could also consume this contract,
but no such proposal is formed here. Prospective implementation-review reuse is
an evidence need, not current scope or implementation justification.

## Uncertainty and evidence needs

- Represent one proposal-research claim and one review finding to determine the
  genuinely shared minimum and domain-specific extensions.
- Prove that one stable claim identity survives multiple immutable revisions
  bound to different evidence baselines.
- Exercise an implementation completion that produces an impact nomination
  targeting one or more exact claim revisions through `may_affect` without
  acquiring semantic refresh authority.
- Exercise both refresh outcomes: a conclusion retained unchanged against a new
  baseline and a conclusion revised with attributed `changed_because_of`
  causality.
- Exercise downstream reliance on an exact claim revision and show that
  candidate impact remains a derived warning until authorized applicability or
  reopening judgment occurs.
- Exercise an adversarial lifecycle matrix covering concurrent and branching
  revisions, identity forks, duplicate and out-of-order nominations, unresolved
  and contested refresh, warning retirement, authority revocation, inactive
  recipients, and superseded reliance.
- Rebuild a query projection from empty canonical history; test schema
  migration, duplicate input, cycles, dangling or retracted references, partial
  watermarks, and stale reads without manufacturing completeness or causality.
- Inject failures before and after publication of nominations, refresh
  judgments, resulting revisions, outbox events, receiving obligations,
  delivery acknowledgements, and semantic completion; prove idempotent
  reconciliation.
- Exercise compaction before semantic publication and after durable publication.
  The former must expose uncertainty; the latter must reconstruct from durable
  owners without treating a summary or retained provider session as the missing
  consequence.
- Exercise a proposal revision where some statements remain applicable and
  others require refresh, correction, or supersession.
- Exercise a domain profile that actually needs semantic freshness maintenance
  and distinguish that role from advisory applicability assessment and the
  downstream authority that relies on evidence.
- Establish any future research-maintenance or organizational-planning
  authority in the domain contract that owns its transition, without making
  those speculative roles prerequisites for review artifacts.
- Determine which sensitivity references must be closed fields and which remain
  narrative, especially for negative or structural-query claims.
- Test on-demand revalidation before considering continuous monitoring.
- Distinguish permanent semantic ownership from the location of the first Git
  schema, validator, or proposal-local adapter.
- Record representative many-to-many traversal queries and use observed
  workload evidence to choose a replaceable query index without making that
  index canonical state.
- Reuse a general authorized delivery route for pending refresh obligations
  without making the scheduler their semantic owner or inventing a parallel
  activation mechanism.

## Authority

The authority-authored phrase permitting review-local use of equivalent
semantics is interpreted here only as allowing an already authorized review
artifact to express equivalent consequences without establishing this shared
owner. This formation revision does not infer permission to implement a new
review-local schema, adapter, service, or runtime. Such permission requires the
decision owner's revision-bound clarification or another applicable authority
contract.

This formed candidate does not establish an artifact schema, declare any
evidence current, produce or rely on an applicability assessment, judge
readiness, assign unresolved role authority, change another contract, accept a
proposal, alter roadmap priority, or authorize implementation. Those
transitions remain with their named owners.

## Acceptance consequence

If later accepted and implemented, research and review consumers can reuse the
same minimal evidence identity and lineage semantics without either workflow
absorbing the other's judgment, mistaking detected change for semantic
invalidity, or allowing persistence to inflate authority. Consumers can align
claim revisions with implementation and other change events, distinguish
uninvestigated candidate impact from refreshed-but-unchanged support and
adjudicated semantic change, and selectively reconsider downstream reliance
without propagating raw commit staleness into authority decisions.
