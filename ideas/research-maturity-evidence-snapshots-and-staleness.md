# Research Maturity, Evidence Snapshots, and Staleness

## Status

Future architecture idea / proposal seed

Durably captured on 2026-08-22. This document records an architectural direction
for later decomposition; it is not an accepted proposal-packet schema, product
contract, or implementation plan.

## Core idea

Proposal research should accumulate in durable, reusable layers. Each layer
should identify the evidence world it observed, the claims it supports, and the
changes that could make those claims worth reopening.

```text
research maturity
        +
decision-specific readiness
        +
immutable evidence baseline
        +
claim dependency surfaces
        +
change-impact and semantic consequence signals
        ↓
truthful research freshness
```

A Git commit identifies a repository state. It does not by itself establish
that research remains current. File changes can affect a conclusion, but new
files, new implementations, changed contracts, completed proposals, runtime
changes, and newly credible alternatives can matter even when no originally
observed file changed.

The durable model must therefore preserve both reproducibility and semantic
reopening conditions.

The downstream use of organization-qualified research is developed in
[Context-Derived Organizational Execution Envelopes](./context-derived-role-execution-envelopes.md).

---

# 1. Relationship to proposal packets

The proposal packet remains the durable owner of proposal identity and current
proposal meaning. Git owns its history and review. Mechanical packet validation
may verify closed structure, references, identities, digests, and vocabulary; it
does not prove evidence sufficiency, freshness, value, placement, or acceptance.

Research records may live inside the packet or in packet-referenced artifacts.
The important ownership properties are:

- the proposal packet identifies the current supported conclusions;
- research artifacts preserve evidence, provenance, limitations, and
  sensitivity;
- generated reports and freshness findings are subordinate views or run
  evidence;
- semantic research judgments remain attributed; and
- proposal or implementation authority remains with its named owner.

Mechanical validity and execution readiness remain distinct:

```text
packet is mechanically valid
        ≠
research is sufficient for a decision
        ≠
proposal is authorized for implementation
```

---

# 2. Cumulative research maturity

Research maturity describes the minimum durable understanding established so
far. These levels are epistemic states, not a mandatory ceremony. A role may
combine investigations, skip an unnecessary intermediate label, or deepen one
facet before another as long as its claims remain truthful.

## R0 — Captured

Enough information exists to preserve the idea without pretending it is a
coherent proposal:

- originating observation;
- rough problem statement;
- desired consequence;
- origin and producer provenance; and
- obvious uncertainty.

## R1 — Formed

Enough information exists to identify a coherent proposal:

- objective;
- intended consequences;
- boundaries and non-goals;
- meaningful alternatives;
- likely owner;
- falsification or invalidation conditions; and
- material unanswered questions.

## R2 — Situated

Enough information exists to understand where the proposal touches the current
system:

- repository and architectural placement;
- credible competing owners or placements;
- affected contracts and invariants;
- dependencies and related proposals;
- current machinery;
- direct supporting evidence; and
- unresolved placement uncertainty.

## R3 — Characterized

Enough information exists to support comparison or a portfolio decision:

- expected value and impact;
- implementation complexity;
- risk and reversibility;
- maintenance consequences;
- validation burden;
- likely fan-out;
- meaningful alternatives; and
- confidence and evidence limitations.

This level can support roadmap decisions without necessarily supporting
execution.

## R4 — Organization-qualified

Enough information exists to propose the organization needed to solve the
problem:

- required consequences;
- required and conditional capabilities;
- ownership requirements;
- independence boundaries;
- information-flow requirements;
- mutation and publication authority;
- collaboration and mediation needs;
- context-lifetime requirements;
- plausible realization paths;
- known organizational hazards; and
- decisions requiring human authority.

Only this level supports problem-specific organizational-envelope assembly.

## R5 — Activation-ready

Enough current information exists to begin the concrete work:

- accepted objective and scope;
- sufficiently fresh evidence;
- resolved authority decisions;
- selected organizational structure;
- compatible available implementations;
- validation and acceptance requirements;
- remaining uncertainty explicitly tolerated; and
- stop and escalation conditions.

Activation readiness combines durable proposal understanding with current
configuration and runtime facts. It is not a permanent property of the packet.

---

# 3. Research facets and readiness profiles

One scalar level cannot express every kind of readiness. A proposal may be well
placed but weakly justified in product value, or strongly valuable but still
organizationally unresolved.

Useful evidence facets include:

```text
problem and consequence
product value
repository placement
contracts and authority
dependencies and proposal relationships
risk and reversibility
capabilities and organization
validation and acceptance
runtime feasibility
evidence freshness
```

Each facet may carry a state such as:

```text
unknown
provisional
supported
contested
stale
```

Consumers should request a readiness profile rather than rely on a universal
score:

```yaml
readiness:
  triage:
    status: ready

  placement:
    status: ready

  roadmap_selection:
    status: ready

  organizational_compilation:
    status: blocked
    missing:
      - independence requirements
      - information-flow analysis
      - mutation-authority owner

  activation:
    status: blocked
    missing:
      - accepted organizational envelope
      - current runtime availability
```

Readiness is an attributed semantic conclusion. Deterministic tooling may verify
that required fields and evidence references exist, but cannot infer that the
evidence is sufficient merely from their presence.

---

# 4. Research judgment authority

Attribution identifies who produced a judgment. Authority determines whether
that producer may make the judgment for the stated purpose. Research records
must preserve both.

Authority follows the semantic decision, not merely possession of the packet:

| Judgment | Authorized owner |
| --- | --- |
| Produce a provisional or supported research claim | A research role authorized for that facet and bounded investigation |
| Judge whether a claim remains fresh after revalidation | The packet's authorized research-maintenance or refresh owner for that claim/facet |
| Judge whether a claim applies to a particular downstream decision | The owner of that downstream decision, using evidence within its authority |
| Declare a readiness profile satisfied | The named owner of that readiness profile |
| Accept residual uncertainty for one decision | That decision's authority owner, only within authority it already possesses |
| Change an invariant, ownership boundary, or contract | The owning contract's required human authority |
| Emit mechanical change or candidate-stale findings | Deterministic tooling, with no semantic decision authority |

This means a research producer may establish a bounded claim without thereby
authorizing roadmap selection, organizational compilation, activation, or
implementation. Likewise, a downstream decision owner may conclude that old
research remains applicable to its decision without rewriting the canonical
claim as universally fresh.

Readiness profiles therefore require explicit owners. Conceptually:

```yaml
readiness_authority:
  formation:
    owner: role.proposal_former
    contract_ref: proposal-formation-contract
    status: existing

  research_refresh:
    owner: unresolved
    required_contract: proposal-research-refresh-contract

  organizational_compilation:
    owner: unresolved
    required_contract: organizational-planning-contract

  downstream_decisions:
    owner_from: packet.authority.decision_owner
    scope: exact named decision only
```

The current proposal-formation contract names the Proposal Former as formation
owner, and the packet names a decision owner. The present architecture does not
yet name a general research-refresh owner or an organizational-compilation
owner. Those are explicit unresolved ownership decisions, not permissions that
the compiler, packet validator, strategic advisor, or campaign supervisor may
infer. A concrete configuration must reference actual canonical owners or
remain blocked.

## 4.1 Applicability judgments

Applicability asks whether a claim is relevant to a particular question,
proposal revision, organization, or decision. The owner of that downstream
decision may issue the applicability judgment because that owner bears the
consequences of relying on the evidence.

An applicability judgment should identify:

- the claim and exact revision consumed;
- the decision context and intended use;
- the evidence baseline and known changes;
- material limitations or excluded scopes;
- the deciding role and authority reference; and
- whether the conclusion is reusable or scoped only to this decision.

Applicability does not imply freshness, sufficiency for another decision, or
permission to alter the underlying proposal.

## 4.2 Freshness judgments

Freshness asks whether the evidentiary basis still supports the canonical claim
against a newer world state. An authorized research-maintenance or refresh role
owns this judgment for the bounded claim or facet.

The refresh role may retain, revise, contest, or recommend superseding a claim
within its contract. It may not approve a roadmap decision, organizational
envelope, activation, or implementation merely because it found the claim
fresh.

## 4.3 Readiness and uncertainty judgments

Readiness belongs to the owner of the specific downstream transition. There is
no universal “proposal ready” authority.

The readiness owner consumes applicable, sufficiently fresh claims and decides
whether the missing evidence and remaining uncertainty are compatible with its
decision contract. It may accept residual uncertainty only within authority it
already holds. It cannot waive an invariant, another owner's acceptance
condition, or a human approval boundary.

## 4.4 Judgment artifact

A durable judgment should carry enough authority and scope to prevent reuse as
a broader approval:

```yaml
judgment:
  id: freshness-placement-parser-owner-2026-08-22
  kind: freshness
  subject:
    claim_id: placement-parser-owner
    claim_revision: 3

  conclusion: refreshed
  decision_scope: proposal.parser-refactor

  producer:
    role: role.proposal_research_refresh
    identity: research-run-41

  authority:
    contract_ref: proposal-research-refresh-contract
    facet: repository_placement

  evidence:
    baseline_commit_oid: <old baseline>
    current_commit_oid: <new baseline>
    change_findings:
      - parser scope gained one test file; ownership relationships unchanged

  limitations: []
  changes_contract: false
  human_approval: null
```

Deterministic tooling may validate the artifact's shape, referenced identities,
declared authority slot, and approval presence. It cannot decide whether the
producer's semantic conclusion is correct.

---

# 5. Research records are claim-centered

A research level should not be one undifferentiated snapshot. Material
conclusions need stable claim identity so they can be refreshed, contested, or
superseded independently.

A conceptual record may contain:

```yaml
claim:
  id: placement-parser-owner
  facet: repository_placement
  conclusion: parser package is the probable implementation owner
  status: supported

  producer: proposal-research-role
  produced_at: 2026-08-22T19:00:00Z

  evidence:
    - kind: source
      path: src/parser/engine.py
    - kind: symbol
      qualified_name: parser.Engine
    - kind: structural_query
      query_id: ownership-and-callers-v1

  confidence: medium
  limitations:
    - plugin-owned parser extensions were not evaluated

  reopening_conditions:
    - a new parser implementation is introduced
    - parser/runtime ownership changes
    - the parser interface contract changes
```

The record preserves the consequence of research, not a full reasoning
transcript.

---

# 6. Two different Git identities

The Git revision containing a research record and the Git revision observed by
that research are different identities.

```text
packet revision
    Which commit contains this durable research state?

evidence baseline
    Which repository state did the research examine?
```

The packet revision usually exists only after the research has been written, so
it cannot generally be the baseline the research originally observed.

A record may identify:

```yaml
research_revision:
  packet_commit_oid: <commit containing the durable packet state>

evidence_baseline:
  repository_commit_oid: <commit examined by the research>
  repository_tree_oid: <tree examined by the research>
  contract_catalog_digest: <canonical contract identity>
  environment_catalog_digest: <role/capability identity>
  related_proposals:
    proposal.parser-runtime-boundary: <packet revision or digest>
```

When the packet cannot self-record the commit that contains itself, Git history
or a separate transition receipt may supply `packet_commit_oid`. The packet
should not use an impossible self-referential digest scheme.

Repository identity alone is insufficient when a conclusion depends on external
sources, provider state, metrics windows, or runtime availability. Those inputs
need their own source identity, observation time, and refresh conditions.

---

# 7. Why file-level invalidation is insufficient

Exact file dependencies are useful for local positive claims. They are not
enough for architectural, negative, exhaustive, or future-sensitive claims.

Research can become questionable when:

- an observed file changes;
- a caller, consumer, implementation, schema, or configuration changes
  elsewhere;
- a new file introduces a competing implementation;
- a new capability or architectural owner appears;
- an invariant or role contract changes;
- another proposal is implemented first;
- the original problem disappears or moves;
- a previously exhaustive search gains new matches; or
- runtime/provider assumptions expire.

For example, “no independent-review provider supports capability X” can become
false when a new provider is added. None of the files originally examined need
to change.

---

# 8. Claim dependency surfaces

Each material claim should describe the semantic surface whose change may make
revalidation valuable. The surface should be as narrow as the claim permits and
as broad as its truth requires.

```yaml
sensitivity:
  paths:
    - src/parser/engine.py

  scopes:
    - src/parser/
    - src/runtime/

  symbols:
    - parser.Engine

  structural_queries:
    - ownership-and-callers-v1
    - parser-implementations-v1

  contracts:
    - INV-006
    - capability.repository_evidence

  proposals:
    - proposal.parser-runtime-boundary

  change_kinds:
    - new_implementation
    - ownership_change
    - interface_change
```

Typical mappings include:

| Claim type | Candidate freshness signals |
| --- | --- |
| Exact implementation behavior | File, symbol, caller, and callee changes |
| Interface behavior | Interface, consumer, and implementation changes |
| Architectural placement | Relevant scopes, owners, dependencies, and competing implementations |
| Negative or exhaustive claim | The original query and search scope, including additions |
| Contract conclusion | Referenced invariant and contract revisions |
| Proposal relationship | Target proposal revision and lifecycle state |
| Runtime/provider feasibility | Configuration/runtime identity and observation time |
| Product-value estimate | Product evidence and explicitly time-sensitive assumptions |

For negative and exhaustive claims, preserve the query intent and bounded scope,
not only the result files. New files or symbols matching that query are precisely
the changes that matter.

---

# 9. Detection capability layers and cost boundary

The metadata model and the automation model have very different engineering
costs. The architecture should not imply that recording a dependency surface
automatically supplies a continuously operating watcher.

Separate three capabilities:

```text
RECORDABILITY
    Claims declare baselines, sensitivities, and reopening conditions.

ON-DEMAND REVALIDATION
    A role compares those records with current state when a decision needs it.

AUTOMATIC MONITORING
    Persistent machinery proactively detects affected claims as the repository,
    contracts, proposals, and runtime environment evolve.
```

Recordability is primarily schema, provenance, and authoring work. On-demand
revalidation is bounded but meaningful engineering. Automatic monitoring is a
new subsystem with lifecycle, indexing, scheduling, failure, and notification
costs; it must be scoped and estimated independently.

The different sensitivity classes also have different costs and evidence
strength:

| Sensitivity | Relative automation cost |
| --- | --- |
| Exact path changed since a baseline commit | Low |
| File added or deleted under a bounded scope | Low to medium |
| Contract or configuration digest changed | Low |
| Related proposal lifecycle advanced | Medium |
| Referenced symbol changed | Medium and index-dependent |
| Caller, consumer, or implementation impact | Medium to high |
| Stored structural query gains new matches | High |
| A novel feature semantically affects a claim | Not generally mechanically decidable |

Structural-query replay is especially nontrivial. A durable implementation may
need query identity and version, backend and index identity, coverage
provenance, normalized result comparison, parser/index migration behavior, and
explicit handling for newly indexed or previously excluded material. A changed
result still establishes only candidate impact.

The initial consequence is therefore:

> **Before a consequential downstream decision relies on old research, evaluate
> its freshness in proportion to the claim and decision.**

It is not:

> **Continuously watch every declared dependency surface.**

A graduated implementation boundary may begin with metadata and manual or
decision-triggered refresh, then add exact path/scope comparison, explicit
proposal-consequence propagation, graph-assisted impact, and finally continuous
monitoring only when observed use demonstrates additional value.

---

# 10. Two staleness channels

When freshness is evaluated, it should be able to combine bottom-up mechanical
impact detection with top-down semantic consequence propagation.

## 10.1 Mechanical impact detection

On demand, compare current state with the evidence baseline for signals such as:

- directly changed evidence paths;
- added or deleted files inside watched scopes;
- changed symbols and relationships;
- changed callers, consumers, or implementations;
- changed contracts, schemas, configuration, and capability catalogs; and
- changed result sets for preserved structural-query intents.

These signals establish candidate impact, not semantic invalidity.

## 10.2 Semantic consequence propagation

When another proposal is implemented, its accepted completion evidence can
identify known downstream proposal and claim consequences:

```yaml
implementation_consequences:
  proposal_id: proposal.new-review-provider
  completion_commit_oid: <commit>

  may_affect:
    - proposal_id: proposal.execution-envelopes
    - claim_id: review-provider-exhaustiveness

  consequences:
    - introduces a new independent-review implementation
    - changes provider-selection assumptions
```

Typed proposal relationships such as `depends_on`, `enables`, `informs`,
`supersedes`, and `replaces` provide propagation candidates. They do not prove
that related research is stale. Completion evidence should record known semantic
consequences rather than requiring downstream roles to infer everything from
file diffs.

---

# 11. Freshness states

A relevant change should not automatically turn a supported claim into a false
claim. Suggested states are:

```text
current
    no relevant change is known

candidate_stale
    the dependency surface changed; semantic review is required

stale
    revalidation found that the conclusion no longer holds

refreshed
    the conclusion was re-established against a newer baseline

superseded
    another conclusion or implemented proposal replaced it
```

The mechanical/semantic boundary remains:

```text
Git, graph, and relationship analysis
    something relevant may have changed

attributed research judgment
    the conclusion remains supported, needs revision, or is superseded
```

Absence of a detected change is not proof that a broad semantic conclusion is
still true. Claims with time-sensitive or external assumptions may also have
explicit refresh dates or event-based reopening conditions.

`candidate_stale` may be discovered lazily when a proposal is reconsidered. It
does not require the system to have monitored the claim continuously since its
baseline.

---

# 12. Refresh behavior

Refreshing research should reuse durable evidence and investigate only the
dimensions plausibly affected by change.

```text
load prior supported claims
        ↓
compare baselines and dependency surfaces
        ↓
identify candidate-stale claims
        ↓
gather targeted new evidence
        ↓
retain, revise, contest, or supersede conclusions
        ↓
record a new packet revision and evidence baseline
```

A refresh should preserve:

- the prior conclusion and evidence identity;
- the change signal that reopened it;
- the new evidence gathered;
- the attributed judgment;
- the resulting status and confidence; and
- any revised reopening conditions.

This provides durable epistemic lineage instead of overwriting the fact that an
earlier conclusion was once reasonable against an earlier repository state.

---

# 13. Git storage choices

Normal commits are the preferred durable identity for proposal packet revisions.
They are reviewable, portable, and naturally ordered by Git history.

Tags should not be created for every research level by default. They enlarge a
global user-facing namespace, can create lifecycle and publication ambiguity,
and add little identity beyond the commit already containing the packet state.
Annotated tags may remain an option for rare human-significant milestones under
an explicitly owned contract.

Git notes are also a poor canonical owner because their separate refs and
synchronization behavior make research relationships less visible.

If intermediate research needs immutable identity before entering ordinary
history, a dedicated private-ref mechanism could use a namespace such as:

```text
refs/work-engine/proposal-research/<proposal-id>/<research-revision>
```

Such a mechanism would need its own ownership and lifecycle contract. It may
reuse the bounded temporary-index and immutable-commit affordances demonstrated
by slice checkpoints, but it must not silently inherit slice acceptance,
attribution, or publication semantics.

---

# 14. Interaction with organizational execution envelopes

Organizational compilation should consume an organization-qualified projection
of the packet, not raw research history.

```text
proposal packet
        ↓
current supported claims + limitations
        ↓
organization-readiness judgment
        ↓
organizational requirements projection
        ↓
authorized organizational proposal
        ↓
execution-envelope assembly
```

If required claims are stale, candidate-stale, missing, or contested, the
system may:

- stop and request a decision;
- launch bounded targeted research;
- use a smaller research-only organization;
- proceed only when an authorized owner explicitly accepts the remaining
  uncertainty; or
- declare the proposal not ready for organizational compilation.

Runtime availability remains separate. A packet can be organization-qualified
while no compatible provider is currently available; that is a linking or
waiting condition rather than automatic invalidation of the proposal research.

---

# 15. Central architectural statements

> **Research maturity describes supported understanding, not procedural
> completion.**

> **Readiness is specific to the next consequential decision.**

> **Freshness is judged by an authorized research-maintenance owner;
> applicability and readiness are judged by the owner of the downstream
> decision.**

> **Attribution does not imply authority, and authority for one judgment does
> not imply authority for another transition.**

> **The packet revision and evidence baseline are different Git identities.**

> **A commit makes research reproducible; it does not make it permanently
> current.**

> **Every material claim should expose the changes that would make reopening it
> valuable.**

> **File changes are one staleness signal; new implementations, contract
> changes, proposal consequences, and expiring external facts matter too.**

> **Mechanical impact detection produces candidate staleness. Attributed
> judgment determines semantic freshness.**

> **Recording a dependency surface does not imply that continuous monitoring
> machinery exists.**

> **On-demand freshness evaluation can preserve the required consequence before
> automatic monitoring is justified.**

> **Later research should build on durable earlier conclusions rather than
> reconstructing them.**
