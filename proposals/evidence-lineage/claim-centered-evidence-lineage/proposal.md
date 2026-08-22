# Proposal: Claim-Centered Evidence Lineage

## Identity and state

- Proposal ID: `work-engine.claim-centered-evidence-lineage`
- Family ID: `work-engine.evidence-lineage`
- State: placement uncertain; revised after bootstrap review continuation and
  not closure-reviewed, evaluated, accepted, or authorized
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. Supporting artifacts describe its [placement](placement.md)
and [relationships](relationships.md).

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

## Authority

This formed candidate does not establish an artifact schema, declare any
evidence current, produce or rely on an applicability assessment, judge
readiness, assign unresolved role authority, change another contract, accept a
proposal, alter roadmap priority, or authorize implementation. Those
transitions remain with their named owners.

## Acceptance consequence

If later accepted and implemented, research and review consumers can reuse the
same minimal evidence identity and lineage semantics without either workflow
absorbing the other's judgment, mistaking detected change for semantic
invalidity, or allowing persistence to inflate authority.
