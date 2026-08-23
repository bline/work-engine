### `evidence-backed-proposal-evaluation.md`

**Status:** Formation candidate / idea record. The material has an R1-like shape,
but no authorized formation judgment in this document classifies it as `formed`,
`R1`, evaluated, accepted, prioritized, or implementation-ready.

**Idea:** Research proposal characteristics using specialized agents and produce
evidence-backed claims and typed observations rather than intuitive scores.

**Problem:** Roadmap prioritization currently lacks compatible information about
proposal value, implementation surface, risk, maintenance consequences,
validation, and evidence quality. An untyped list of “dimensions” also risks
mixing durable evidence domains, semantic claims, quantitative observations, and
epistemic metadata into one pseudo-comparable score surface.

**Proposed consequence:** Every evaluated proposal can expose:

```text
facets
    durable domains of investigation

claims
    evidence-backed semantic conclusions within those domains

evidence items
    observations supporting or challenging claims

measures
    typed evidence items with declared units, direction, and scope

readiness judgments
    decision-specific conclusions issued by their authorized owner
```

Candidate evaluation concerns include:

```text
product value and expected consequences
implementation surface and complexity
architectural placement and contract alignment
dependency impact and fan-out
risk and reversibility
maintenance consequences
validation and acceptance burden
expected resource cost
evidence quality, freshness, and confidence
```

These are candidate concerns, not an accepted facet registry. Before schema use,
an authorized semantic judgment must decide whether each item is a facet, claim,
measure, metadata field, or relationship to another concept. In particular,
candidate association questions include:

- Should expected impact be represented by claims and measures within product
  value?
- Does fan-out measure dependency impact, inform architectural placement, or
  require several typed measures?
- Is design alignment one claim spanning placement and contracts rather than a
  standalone dimension?
- Is validation burden a measure family inside validation and acceptance?
- Should evidence strength and confidence remain claim-level epistemic metadata
  rather than proposal-value dimensions?

The questions deliberately avoid declaring equivalence. Any adopted mapping
needs stable judgment identity, evidence, producer attribution, owning
authority, and contract-change classification.

Each claim or measure carries its evidence basis, baseline, limitations,
freshness, provenance, and confidence.

## Conditional portfolio comparison

No weighted sum or universal ranking formula becomes canonical. Value claims
across unlike proposals often describe incommensurable goods and should not be
forced onto one scalar scale merely because a portfolio UI can display a
number.

Pareto or dominance views are a useful candidate presentation when a declared
comparison contract makes the selected measures compatible. Such a contract
must identify:

- the selected measures and excluded concerns;
- definitions, units, directionality, and scope;
- treatment of missing, contested, and stale evidence;
- confidence or evidence-quality requirements;
- the evidence cutoff; and
- the authority that selected the comparison surface.

A mechanically generated finding should therefore be conditional:

> Given measurement contract M, evidence cutoff E, and selected measures D,
> proposal A is currently no worse than proposal B on every included measure and
> better on at least one.

This is a candidate dominance finding, not automatic priority or implementation
authority. A frontier over cost and risk does not establish overall dominance
when proposal value is qualitative or excluded. Portfolio decision owners still
own tradeoffs, incomplete evidence, strategic fit, and incommensurable value.

A Studio portfolio view may:

- show dominated and non-dominated proposals under a named comparison contract;
- expose the measures producing each relation;
- overlay confidence, freshness, and contested evidence;
- show qualitative value claims beside quantitative measures; and
- allow different authorized comparison views without silently changing the
  canonical proposal record.

## Possible research roles

```text
repository / ownership analyst
design-alignment reviewer
metrics analyst
risk / reversibility analyst
proposal synthesizer
```

These are candidate organizational functions, not accepted role templates or
owners. Proposal formation must still establish the semantic owner, boundaries,
alternatives, invalidation conditions, and evidence needs before recording a
formed proposal.

Research maturity, decision-specific readiness, Git evidence baselines,
authority, and staleness propagation are developed in
[Research Maturity, Evidence Snapshots, and Staleness](./research-maturity-evidence-snapshots-and-staleness.md).

---

### `proposal-backed-roadmap.md`

**Status:** Formation candidate / idea record. It is not classified, accepted,
prioritized, or implementation-authorized by this document.

**Idea:** Reduce the roadmap to a portfolio/index of accepted proposal packets.

**Problem:** Rich proposal reasoning becomes duplicated or lost when copied into
roadmap prose.

**Proposed consequence:** The roadmap contains only enough information to
identify, order, and select work:

```text
title
short description
status / priority
proposal packet pointer
```

When a campaign begins, each role receives an authorized bounded projection from
the canonical packet rather than reconstructing proposal meaning.

**Additional consequence:** A rejected, deferred, or unselected proposal retains
its research and can later be refreshed rather than rediscovered.

## Packet projection contract application

The structural mechanism already exists conceptually:

```text
canonical proposal packet
        ↓ explicit MAY_OBSERVE authority
role-scoped bounded projection
        ↓ named on-demand evidence references
consumer role
```

The remaining work is not a new projection ontology. It is a bounded application
contract defining which packet state each consumer may observe, what its
consequences require, and which sensitive or irrelevant material remains hidden.

Candidate consumer consequences include:

| Consumer | Candidate projection consequence |
| --- | --- |
| Supervisor | Receive accepted objective, decision authority, blockers, and authorized execution-envelope inputs without reconstructing repository research |
| Builder | Receive accepted scope, placement evidence, affected contracts, constraints, and named deeper evidence useful to implementation |
| Reviewer | Receive acceptance claims, protected boundaries, evidence under review, and the independence-preserving context required for falsification |
| Research role | Receive current claims, uncertainty, evidence needs, baselines, limitations, and reopening conditions for its bounded investigation |

These rows are candidate projection requirements, not adopted field mappings.
Each concrete projection requires an owning contract, observation authority,
source revision and freshness identity, required fields, excluded fields, and
on-demand reference policy. A projection remains a convenience view and never
becomes a competing owner of proposal meaning.

The existing cross-workflow observation pattern is described in
[Work Engine Studio: Design, Control, and Forensics](./work-engine-studio-design-control-forensics.md#cross-workflow-observation),
and role-scoped execution projections are developed in
[Context-Derived Organizational Execution Envelopes](./context-derived-role-execution-envelopes.md).
