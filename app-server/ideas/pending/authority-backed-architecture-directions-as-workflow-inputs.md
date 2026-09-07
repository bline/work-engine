# Idea: Authority-Backed Architecture Directions as Workflow Inputs

**Status:** Architecture and workflow idea

**Scope:** Idea intake, proposal formation, strategic planning, implementation
compilation, review, and durable product-direction ownership

**Authority:** Exploratory only. This document does not accept an architecture
direction, amend the roadmap, change instruction precedence, authorize
implementation, or designate an architecture decision owner.

## Summary

Work Engine should represent accepted architectural direction as a durable,
versioned, authority-backed input to the workflows that interpret ideas and
design implementations.

Today an idea can be evaluated against repository state, general design
principles, existing proposals, and local evidence. Those inputs do not
reliably answer a separate question:

> Does this idea advance, preserve, extend, or contradict an architectural
> direction that the product has already chosen?

An architectural direction should not live only in remembered conversation, a
large prompt, an implementation plan, or a pending idea document. It should
have a stable identity, exact revision, bounded applicability, named invariants,
explicitly open and undecided space, decision authority, exception route, and
reopening conditions.

The accepted destination must also remain distinct from current adoption and
the authorized route toward it. A repository may truthfully be transitional
without violating the direction, and a bounded intermediate slice may take an
enabling detour without turning every regression into an architectural
exception.

Workflows should receive projections of applicable directions:

```text
accepted architecture directions
    decided target, open space, authority
                         +
current architecture adoption state
    satisfied, transitional, unknown, evidence
                         +
authorized transition contracts
    route, detours, restoration, admissibility
                         |
          +--------------+----------------+
          |              |                |
          v              v                v
      idea intake   proposal formation  strategic planning
          |              |                |
          +--------------+--------+-------+
                                  |
                                  v
                       implementation compilation
                                  |
                                  v
                       implementation and review
```

The purpose is not to reject unconventional ideas automatically. It is to make
the product's chosen direction visible early enough that every downstream role
can distinguish:

- conformance;
- extension into an intentionally open area;
- tension that requires evidence or clarification;
- conflict requiring an explicit exception or architecture revision; and
- irrelevance because the direction does not apply.

The core principle is:

> Product architecture should be a durable, scoped, authority-backed dependency
> of design work, not context that each agent must rediscover or silently infer.

## 1. Motivation

Work Engine separates raw ideas, evidence, proposals, decisions, plans,
implementation, and review so that no artifact silently acquires authority it
does not own. Architectural direction needs the same treatment.

Without a durable direction layer, several failures are likely:

- an intake role treats an idea as locally attractive without noticing that it
  collapses an accepted system boundary;
- proposal formation spends substantial effort exploring routes the product has
  already rejected;
- two proposals use incompatible architectural premises without making the
  conflict visible;
- an implementation planner must reconstruct product direction from prose,
  repository shape, or conversation history;
- a builder makes a locally reasonable choice that reverses a long-term product
  direction;
- review discovers the architectural mismatch only after implementation; or
- obsolete architectural preferences remain in prompts after the product has
  changed direction.

The desired outcome is not more instruction text. It is a semantic owner that
allows each workflow to obtain the smallest applicable architectural
projection, bound to the exact revision on which its work depends.

## 2. Architectural direction is a distinct durable object

An architecture direction is not interchangeable with any existing artifact.

### It is not a raw idea

A raw idea records what its author proposed. It does not establish that the
product has chosen that direction.

### It is not a proposal

A proposal owns candidate meaning, placement, relationships, and uncertainty.
Formation and evaluation do not by themselves create permanent product
architecture.

### It is not a claim

A claim records evidence-backed knowledge. Evidence that a boundary exists or
that a mechanism works does not decide that Work Engine should preserve that
boundary.

### It is not a roadmap item

A roadmap orders intended outcomes and dependencies. It may rely on an
architecture direction, but schedule and priority are different from
architectural meaning.

### It is not an implementation contract

An implementation contract derives a bounded repository transformation from
accepted meaning and decisions. It consumes architecture direction; it should
not become the first durable owner of that direction.

### It is not an agent instruction

An agent-facing projection may communicate applicable constraints, but prompts
and skills are delivery mechanisms. They must not become the sole canonical
owner of product architecture or silently strengthen its authority.

### It is not runtime-selection policy

Operator overlays may choose among presently admissible runtimes. Architecture
directions define longer-lived product boundaries within which those choices
are made. A runtime preference must not revise architecture by accident.

### It is not architecture adoption state

Adoption state records where an exact repository revision currently stands
relative to the direction. A known gap or completed boundary changes adoption,
not the accepted destination.

### It is not an architecture transition contract

A transition contract authorizes and constrains one route toward the direction.
Its milestones and temporary detours may expire while the target architecture
remains unchanged.

## 3. Candidate `ArchitectureDirection` artifact

Conceptually:

```yaml
# Illustrative accepted-direction record; not current Work Engine state.
schema_version: 1

identity:
  direction_id: runtime-architecture/three-port-split
  revision: 1

lifecycle:
  status: accepted
  predecessor: null
  accepted_at: "<authority-recorded timestamp>"

scope:
  domains:
    - provider execution
    - coding-harness integration
    - operator presentation
  applicability:
    any:
      - changes provider-turn ownership
      - changes harness-runtime ownership
      - changes operator projection ownership

direction:
  required_boundaries:
    - ProviderTurnPort
    - HarnessRuntimePort
    - OperatorProjection
  continuous_invariants:
    - Work Engine owns orchestration and durable workflow state
    - runtime ownership cannot be ambiguous or duplicated
  target_invariants:
    - provider and harness capabilities remain distinguishable
    - operator projections are not canonical runtime state
    - implementations spanning several ports are admitted per port
  open_points:
    - exact port method surfaces
    - physical module placement
    - which provider, harness, and projection implementations are admitted
  explicitly_undecided:
    - whether OpenCode is adopted for any port
    - whether direct providers replace any existing harness path
    - whether all roles use the same runtime composition
  excluded_implications:
    - three ports does not require every realization to bind all three
    - independent admission does not assert semantic equivalence

authority:
  decision_owner: human/product-owner reference
  decision_reference: immutable authority record

exceptions:
  mode: explicit_decision_required
  decision_owner: human/product-owner reference

reopening_conditions:
  - demonstrated inability to preserve required semantics
  - implementation or operational cost contradicts the accepted premise
  - a superseding product decision changes Work Engine ownership

source:
  narrative: provider-turn-harness-runtime-and-operator-projection.md
  accepted_revision: immutable Git or content identity
```

The exact schema remains a formation question. The important semantic fields
are:

- stable identity and revision;
- lifecycle status;
- bounded scope and applicability;
- the chosen direction and temporally classified invariants;
- open points, explicitly undecided questions, and excluded implications;
- the authority that accepted it;
- explicit exception ownership;
- reopening conditions;
- predecessor or supersession lineage; and
- immutable references to the explanatory narrative and decision evidence.

The structured artifact should contain only information downstream consumers
must interpret mechanically. The associated narrative should continue to own
the architectural explanation, tradeoffs, examples, and uncertainty.

Open points and explicitly undecided implications are first-class direction
content, not merely prose non-goals. They define the negative space around an
accepted decision so downstream roles do not extrapolate a coherent but
unauthorized architecture beyond what was actually chosen.

## 4. Direction, adoption, transition, and authority

The accepted architectural destination, the repository's current position, and
the authorized route between them require separate semantic owners:

```text
ArchitectureDirection
    what the product has chosen
    target and continuous invariants
    open and explicitly undecided space
    exception and reopening authority

ArchitectureAdoptionState
    where an exact repository state currently stands
    satisfied boundaries
    inherited transitional gaps
    unknown or unverified areas
    authorized exceptions and unplanned regressions
    attributed evidence cutoff

ArchitectureTransitionContract
    how an authorized body of work may move adoption forward
    transition invariants
    permitted and contained enabling detours
    restoration obligations and milestones
    integration and release posture

Roadmap, proposals, and campaigns
    the particular work intended to realize the transition
```

### Architecture direction

`ArchitectureDirection` owns durable product intent. It should change only
through an authorized architecture decision, not whenever repository adoption
advances.

### Architecture adoption state

`ArchitectureAdoptionState` is an attributed, evidence-bound assessment of one
repository revision relative to one direction revision. It may change
frequently without revising product architecture.

Conceptually:

```yaml
direction: runtime-architecture/three-port-split@1
repository_revision: "<immutable Git revision>"
assessment_revision: 7

boundaries:
  ProviderTurnPort:
    status: inherited_transitional_state
    observation: principal builder path remains Codex-bound
    evidence: executable-generation composition reference
  HarnessRuntimePort:
    status: partially_satisfied
    evidence: Codex and Claude adapter references
  OperatorProjection:
    status: inherited_transitional_state
    evidence: current operator switchboard reference

unknowns:
  - whether all reviewer paths preserve equivalent raw evidence
```

Observed adoption classifications should initially distinguish:

- `satisfied` — evidence demonstrates the relevant target boundary;
- `partially_satisfied` — some bounded portion is realized;
- `inherited_transitional_state` — the gap existed when the direction was
  accepted and is not thereby an authorized exception;
- `planned_transitional_state` — a temporary state explicitly permitted by an
  architecture transition contract;
- `authorized_exception` — an authority-approved divergence from the target;
- `unplanned_regression` — observed movement away from the target with no valid
  transition authorization; and
- `unknown` — current evidence cannot support a stronger judgment.

The adoption record owns observations and conformance assessment. It must not
manufacture normative transition constraints merely because it is updated more
often than the architecture direction.

### Architecture transition contract

`ArchitectureTransitionContract` owns the normative route for a bounded body of
work. This is where a direction or roadmap authority may permit an intermediate
state that would otherwise appear to move away from the target.

Conceptually:

```yaml
direction: runtime-architecture/three-port-split@1
adoption_basis: architecture-adoption/three-port@7
transition_id: three-port/provider-turn-extraction@2
authority_reference: "<immutable transition decision>"

objective:
  establish one provider-turn interception seam before adapter extraction

transition_invariants:
  - continuous authority and evidence invariants remain satisfied
  - no new consumer may depend on the temporary provider/harness coupling

permitted_detours:
  - id: centralize-before-separating
    bounded_scope:
      - executable-generation composition
    restoration_milestone: provider-turn-extraction/slice-4
    expires_when:
      - restoration milestone is accepted
      - transition is stopped or superseded

admissibility:
  development_successor: allowed
  integration: conditional
  production_release: prohibited_until_restored
```

A mutable adoption observation and a normative transition rule may be
distributed together for convenience, but their authority and provenance must
remain distinct. The fact that coupling exists is not authority to preserve,
increase, or reproduce it.

### Lifecycle

Architecture directions need a lifecycle distinct from proposal formation.

Conceptually:

```text
candidate architecture idea
        |
        v
proposal formation and evaluation
        |
        v
explicit architecture decision
        |
        v
accepted direction revision
        |
        +------> amended by successor revision
        |
        +------> superseded by another direction
        |
        +------> reopened under a named condition
```

Only an explicitly authorized architecture decision may create, amend,
supersede, or retire an accepted direction. Neither an intake assessment, a
proposal-forming role, a planner recommendation, implementation success, nor
mechanical validation creates that authority.

The current version-1 proposal-decision transition deliberately does not change
permanent architecture. This idea therefore requires either a distinct
architecture-decision contract or an explicitly revised future decision model;
it should not overload the existing proposal decision and silently broaden its
authority.

Accepted does not mean irreversible. Every direction should name conditions
under which its premises should be reconsidered. Reconsideration produces a
new decision and revision rather than rewriting the historical artifact.

Updating adoption evidence does not amend the direction. Authorizing or
revising a transition contract does not amend the target unless that contract
explicitly passes through the architecture-decision authority required to do
so. Completing a transition milestone updates adoption state and dependent
plans; it does not manufacture new architectural meaning.

## 5. Applicability and projection

Loading every architecture document into every role would create noise and
eventually contradictory prompt law. Work Engine should resolve applicability
before projecting directions to a consumer.

Applicability may be established from:

- the idea or proposal's declared domains and affected ownership boundaries;
- referenced ports, services, schemas, roles, or lifecycle mechanisms;
- placement and relationship metadata;
- repository paths or public interfaces when those are stable enough to be
  useful selectors;
- explicit dependency on another direction; and
- a human or authorized role's attributed applicability decision when semantic
  judgment is required.

Keyword similarity may nominate a direction for consideration. It must not
make an accepted direction applicable by itself.

The projection supplied to a workflow should contain only:

- direction identity and exact revision;
- why it is considered applicable;
- relevant continuous and target invariants;
- open points, explicitly undecided questions, and excluded implications;
- relevant current adoption observations and evidence cutoff;
- an applicable transition contract and remaining restoration obligations;
- exception owner and route;
- reopening conditions relevant to current evidence; and
- references through which the full narrative and decision can be inspected.

The projection is derived state. The accepted direction and its authority
record remain canonical.

## 6. Intake-time behavior

Idea intake should preserve the raw idea unchanged, then evaluate it against
the applicable direction projection.

For each applicable direction, intake should record one of these candidate
relationships:

```text
aligned
    the idea preserves and advances the direction

extends
    the idea acts in an area the direction intentionally leaves open

tension
    the relationship depends on unresolved meaning or evidence

conflicts
    the idea proposes a consequence incompatible with an accepted invariant

outside_scope
    the direction was considered but does not govern the idea
```

These classifications are attributed assessments, not automatic dispositions.
A conflict does not make an idea worthless or authorize its rejection. It may
mean:

- the idea should be revised;
- a distinct exception proposal should be formed;
- the accepted direction's reopening conditions have been met;
- the applicability assessment was wrong; or
- the product owner intends to change direction.

The intake projection should carry the applicable direction and relationship
into proposal formation so the same question does not need to be reconstructed
from conversation.

## 7. Proposal-formation behavior

Proposal formation should include a conformance mapping for every applicable
accepted direction.

For example:

```yaml
architecture_conformance:
  direction: runtime-architecture/three-port-split@1
  applicability: confirmed
  adoption_basis: architecture-adoption/three-port@7
  preserved_continuous_invariants:
    - runtime ownership remains unambiguous
  target_invariants_advanced:
    - provider and harness ownership become separately representable
  target_invariants_remaining:
    - operator projection does not own runtime truth
  extended_open_points:
    - capability observation transport
  explicitly_undecided_preserved:
    - whether OpenCode is adopted
  transition_contract_required: false
  tensions: []
  conflicts: []
  exception_requested: false
```

The proposal former may challenge the intake classification when repository
evidence or clearer proposal meaning changes the relationship. The revised
judgment must remain attributed and explain the evidence or interpretation that
changed it.

When a proposal conflicts with an accepted direction, formation must not hide
the conflict by choosing implementation language that appears compliant. It
should preserve one of these consequences:

- revise the proposal to conform;
- form an independently decidable exception candidate;
- nominate the direction for reopening;
- establish that the direction is not applicable; or
- remain unresolved pending the owning human decision.

Proposal formation still does not accept the proposal or the exception.

## 8. Strategic-planning behavior

Strategic planning should treat accepted directions as governing doctrine with
exact revision identity.

The planner should determine whether new proposals, implementation evidence, or
campaign outcomes:

- remain compatible with the current direction;
- advance, preserve, or regress the recorded adoption state;
- reveal a dependency not represented in the roadmap;
- invalidate an assumption behind the direction;
- make an exception strategically preferable;
- require a bounded transition contract or restoration milestone;
- leave an enabling detour active longer than its authority permits;
- require sequencing work to establish an architectural boundary first; or
- support reopening or superseding the direction.

A planning recommendation may propose an architecture decision. It does not
perform that decision or mutate the accepted direction without authority.

## 9. Implementation-compilation behavior

Applicable architecture-direction revisions, current adoption assessments, and
authorized transition contracts should be explicit, separately attributed
dependencies of the implementation basis.

The implementation compiler should translate relevant invariants into bounded
constraints such as:

- required ownership boundaries;
- forbidden dependencies or state transfers;
- continuous invariants that every admitted intermediate state must preserve;
- target invariants the slice advances or intentionally leaves transitional;
- interfaces that must remain independently replaceable;
- compatibility and migration requirements;
- permitted enabling detours, containment, expiry, and restoration obligations;
- checkpoint, integration, deployment, and release admissibility;
- evidence needed to demonstrate conformance; and
- decisions that must be returned rather than selected during coding.

For example, an implementation contract governed by the three-port direction
could permit a new OpenCode provider adapter while forbidding the slice from
making an OpenCode session the canonical Work Engine role or context identity.

The implementation compiler may discover that the proposed repository change
cannot satisfy an accepted direction directly but can serve an authorized
multi-slice transition through a bounded enabling detour. It must distinguish
that case from an unplanned regression and bind the detour to its transition
authority and restoration milestone. When neither direct conformance nor an
authorized transition route exists, it must return the conflict rather than
silently weakening the direction or improvising an exception.

## 10. Implementation and review behavior

Builders should receive the smallest applicable implementation constraints,
not the entire architecture library. Every constraint should identify its
direction revision or derived implementation decision.

Review should verify conformance against the exact architecture-direction
revision, adoption basis, and transition contract in the implementation basis.
It should distinguish:

- implementation violates the direction;
- implementation contract failed to project a relevant invariant;
- direction was incorrectly considered applicable;
- implementation remains an inherited transitional state;
- implementation advances or preserves the authorized adoption route;
- a local regression is an authorized, contained enabling detour;
- an enabling detour exceeded its scope, expiry, or restoration obligation;
- repository evidence satisfies a reopening condition;
- implementation took an authorized exception; and
- direction leaves the route to implementation discretion.

A reviewer does not acquire authority to reject or revise an accepted direction
merely by finding that another route appears better. It reports the consequence
to the architecture decision owner.

## 11. Slice transition assessment and operational admissibility

Conformance during a slow architectural migration needs more than one axis.
Each accepted slice boundary should carry three independent assessments:

```text
Target conformance
    Where is the resulting state relative to the accepted destination?

Local trajectory
    Did this slice move closer, remain neutral, or move farther away than its
    predecessor?

Transition role
    How does that local movement serve the authorized multi-slice route?
```

Candidate closed values are:

```yaml
target_conformance:
  satisfied | transitional | conflicting | unknown

local_trajectory:
  advances | preserves | regresses | unknown

transition_role:
  direct_advance | enabling_detour | restoration | neutral | terminal | unplanned
```

An enabling detour may therefore state truthfully:

```yaml
target_conformance: transitional
local_trajectory: regresses
transition_role: enabling_detour

transition_contract: three-port/provider-turn-extraction@2
detour: centralize-before-separating
restoration_milestone: provider-turn-extraction/slice-4
new_dependents_permitted: false
```

The transition role does not erase the local regression. It explains why that
regression remains admissible under an exact, authority-backed route.

### Operational admissibility

Architectural assessment is separate from whether the resulting checkpoint may
be used operationally:

```yaml
operational_admissibility:
  development_successor: allowed
  integration: allowed | conditional | prohibited
  deployment: allowed | conditional | prohibited
  production_release: allowed | conditional | prohibited
```

A state may be valid as the basis of the next isolated development slice while
remaining prohibited from integration or production release. Those
consequences require explicit transition authority and mechanically visible
gates where enforcement is expected.

### Accepted boundary versus internal increment

A builder may temporarily break compilation or architecture inside an
unaccepted workspace while constructing one slice. That is ordinary
implementation state and does not require a durable architecture detour.

The transition model applies when a deviation survives as an accepted durable
checkpoint or becomes a dependency of another slice. At that point its scope,
necessity, restoration owner, expiry, and admissibility must no longer depend on
the builder's temporary context.

### Detour admission

An enabling detour is exceptional. It should be admitted only when it records:

- the exact transition objective it enables;
- why a direct advancing slice is not currently practical;
- the bounded modules, interfaces, or behaviors affected;
- continuous invariants that remain non-negotiable;
- whether new dependents may consume the temporary shape;
- the restoration milestone and responsible workflow owner;
- expiry, stop, and supersession conditions;
- rollback or safe-stop consequences where relevant;
- operational admissibility; and
- the authority reference permitting the temporary route.

A slice cannot make itself compliant by labeling an ordinary regression an
enabling detour. Admission depends on a pre-existing or separately authorized
transition contract.

## 12. Temporal invariant classes and filter behavior

An architecture direction should classify invariants by when they must hold.
This prevents a target-state property from being enforced prematurely and
prevents an authority or safety property from being waived as temporary.

### Continuous invariant

Continuous invariants must hold at every admitted slice boundary, including an
enabling detour. Examples include:

- human and operator authority remains bounded;
- active runtime operations have one unambiguous owner;
- durable evidence and identity remain truthful;
- fencing and concurrency safety are preserved; and
- cost or mutation authority is not silently expanded.

An architecture transition contract cannot weaken a continuous invariant unless
the owning architecture authority first revises the direction itself.

### Target-state invariant

Target-state invariants describe the accepted destination. Adoption may remain
transitional while work advances toward them. Examples include independent
replaceability of provider and harness implementations or migration of every
operator client behind `OperatorProjection`.

### Transition invariant

Transition invariants constrain the route between adoption states. Examples
include prohibiting new consumers of temporary coupling, requiring a
restoration milestone, or preventing production release until a detour is
removed.

An architecture direction should act as both a filter and a modifier, depending
on the strength of its content.

### Hard filter

An accepted, applicable continuous invariant excludes a conflicting route
unless an authorized successor direction exists. A target-state invariant
filters a claimed terminal state and an unplanned regression; it does not make
mere incomplete adoption a violation. An authorized exception may permit a
bounded target divergence but cannot silently waive continuous authority or
safety.

Examples:

- do not make an operator UI the canonical workflow owner;
- do not collapse provider and harness capability identity;
- do not create two owners for one runtime operation.

### Design modifier

Open points, preferences, and reopening conditions shape investigation without
predetermining the result.

Examples:

- evaluate provider and harness candidates independently;
- prefer replaceable boundaries where semantic equivalence is unproven;
- gather cost evidence if the direction's maintenance premise becomes doubtful.

The artifact must distinguish hard invariants from guidance. Otherwise a model
may strengthen a preference into a prohibition or weaken an accepted boundary
into optional advice.

## 13. Dependency and change propagation

Derived artifacts should record the exact direction, adoption assessment, and
transition contract revisions on which they depend:

```text
accepted direction revision changes
        |
        +-- intake assessments may require reconsideration
        +-- proposal conformance mappings become stale
        +-- strategic assumptions may require reconciliation
        +-- unexecuted implementation contracts require recompilation
        +-- active campaigns require an applicability decision
        +-- historical execution remains bound to its original basis

adoption assessment changes
        |
        +-- target and trajectory judgments may be recomputed
        +-- transition milestones may become satisfied or stale
        +-- no architecture authority changes

transition contract changes or expires
        |
        +-- dependent enabling detours require renewed admission
        +-- restoration or safe-stop obligations become current
        +-- operational admissibility may narrow
```

Superseding a direction does not rewrite history or automatically interrupt
running work. Work Engine should determine the consequence according to the
dependent artifact's lifecycle, active authority, reversibility, and safe
boundary.

Change propagation should therefore identify affected dependents and request
re-evaluation. It must not silently convert architectural invalidation into
implementation, campaign, spending, or interruption authority.

Historical adoption assessments and slices remain bound to the revisions under
which they were produced. A later assessment may change the current course
without rewriting whether an earlier detour was authorized at its own boundary.

## 14. Relationship to current Work Engine machinery

This idea fills a gap between existing owners rather than replacing them.

### Idea intake

The current [Idea Intake](../../../skills/idea-intake/SKILL.md) contract binds
an assessment to an exact raw-idea revision, preserves attributed
interpretation, and carries surviving candidates toward proposal formation.
Architecture applicability and conformance would become additional attributed
inputs and relationships; intake would still not rewrite the source or accept a
direction.

### Proposal formation and packets

The current [Proposal Former](../../../skills/proposal-former/SKILL.md) keeps
ideas, repository observations, model inference, human decisions, and accepted
contracts distinct. Architecture directions would be one class of accepted
contract projected into formation. Proposal packets would need a durable way to
reference exact applicable directions and conformance consequences without
making the packet their canonical owner.

### Proposal decisions

The current proposal-decision contract explicitly preserves permanent
architecture as unchanged. A new authority path is therefore required before
any pending architecture idea can become an accepted direction.

### Strategic planning

The current [Strategic Planner](../../../skills/strategic-planner/SKILL.md)
already consumes governing doctrine and checks whether product direction has
become stale. Exact architecture-direction revisions would make that dependency
explicit and queryable.

### Implementation compilation

[Decision-Gated Implementation Compilation](./proposal-decision-gated-implementation-compilation.md)
already proposes an implementation basis containing governing invariants,
accepted meaning, decisions, and evidence. Architecture directions would own
one important source of those invariants and give the basis an exact dependency
rather than requiring the compiler to reconstruct them.

### Architecture-specific proposals

[Provider Turn, Harness Runtime, and Operator Projection](./provider-turn-harness-runtime-and-operator-projection.md)
is a useful first candidate for exercising this model. It is currently a
pending architecture idea, not an accepted direction. If the owning human
chooses it as product direction, that choice should produce an authority-backed
direction revision rather than relying on stronger wording in the idea file.

## 15. Candidate initial vertical

A bounded first vertical could:

1. define one closed architecture-direction record and authority-decision
   record;
2. define separate adoption-state and transition-contract records with explicit
   provenance and non-authorizing boundaries;
3. represent the accepted or still-candidate status of the three-port runtime
   architecture without implying acceptance;
4. if accepted by the required owner, assess the existing Codex-bound builder
   path as inherited transitional state rather than an authorized exception;
5. make the three-port direction's open points and explicitly undecided
   implications projectable;
6. resolve applicability for one exact raw idea;
7. project the applicable direction and current adoption state into one intake
   assessment and one proposal-formation episode;
8. produce a conformance mapping with an aligned, extends, tension, conflicts,
   or outside-scope result;
9. carry exact direction, adoption, and transition revisions into an
   implementation basis; and
10. exercise two slices: one direct advance and one bounded enabling detour with
    a restoration successor, proving that continuous invariants and release
    restrictions remain enforced throughout.

The vertical should also demonstrate that a successor direction or adoption
assessment makes appropriate derived projections stale without rewriting their
history or automatically authorizing downstream mutation.

This would test the ownership and dependency model before attempting automatic
retrieval across a large architecture library.

## 16. Invariants

### Human and product authority remains explicit

No model role, validator, repository search, implementation outcome, or
similarity score may create or revise accepted product architecture without the
required authority.

### Raw ideas remain raw

Intake compares an idea with applicable direction without rewriting the source
to make it appear conformant.

### Conflicts remain visible

The workflow records tension and conflict rather than silently rejecting the
idea or weakening the direction.

### Applicability is bounded and attributable

A direction affects only work within its scope. Semantic applicability is a
judgment with provenance, not a keyword side effect.

### Canonical ownership remains singular

Directions own accepted architectural meaning. Adoption state owns attributed
current conformance observations. Transition contracts own normative movement
within their authority. Workflow projections, prompts, plans, and review reports
reference those objects but do not become competing owners.

### Existing gaps are not retroactive exceptions

Accepting a target direction does not convert every pre-existing mismatch into
an authority-approved exception. Inherited transitional state remains visible
as an observed starting condition.

### Continuous invariants survive every detour

No transition role, intermediate milestone, or operational convenience may
silently waive continuous authority, ownership, safety, or evidence invariants.

### Detours carry restoration obligations

An enabling detour remains bound to its scope, transition objective, expiry,
admissibility, and restoration milestone. It cannot become ordinary architecture
through neglect.

### Historical work remains reconstructable

Every derived artifact names the exact direction, adoption, and transition
revisions it used. Supersession does not rewrite prior meaning.

### Direction changes invalidate judgment, not authority boundaries

Staleness requests reconsideration. It does not authorize implementation,
interruption, roadmap mutation, cost, or acceptance.

## 17. Non-goals

This idea does not propose:

- placing every design preference into a global constitution;
- loading every architecture document into every agent context;
- rejecting all ideas that conflict with current direction;
- treating current repository structure as accepted architecture;
- treating an inherited gap as a freshly authorized exception;
- allowing slices to self-declare regressions as enabling detours;
- requiring every accepted development checkpoint to be production-releasable;
- permitting a transition contract to waive continuous invariants;
- making architecture immutable;
- allowing a proposal or review role to manufacture product authority;
- replacing proposal evaluation, strategic planning, or implementation review;
- encoding detailed implementation mechanisms as permanent architecture; or
- making a UI, prompt, skill, or vector index the canonical direction owner.

## 18. Questions for proposal formation

1. Who owns architecture decisions, and how is that authority represented and
   verified?
2. Should accepted directions be independent records, a specialized proposal
   decision, ADRs with closed metadata, or another durable object?
3. Which lifecycle states are required beyond candidate, accepted, superseded,
   retired, and reopened?
4. What minimum applicability language is expressive without becoming an
   unmaintainable policy engine?
5. Which relationships can be nominated mechanically, and which require an
   attributed semantic judgment?
6. Where should conformance live in intake and proposal packet schemas without
   creating a competing owner?
7. What canonical owner and evidence contract should produce
   `ArchitectureAdoptionState`?
8. Which transition constraints require architecture-decision authority, roadmap
   authority, plan acceptance, or another owner?
9. Which direction, adoption, or transition changes stale proposals, plans,
   reviews, or active campaigns, and who decides the operational consequence?
10. How should exception requests relate to proposal decisions and implementation
   authority?
11. How are conflicting accepted directions detected, adjudicated, and ordered?
12. Which accepted directions should be projected into role prompts, and which
    should remain on-demand references?
13. Which invariant classes are continuous, target-state, or transition-specific,
    and who may change that classification?
14. When may an enabling detour cross an accepted slice boundary, and what
    minimum containment, expiry, restoration, and admissibility fields are
    required?
15. What evidence demonstrates that the direction layer improves design quality
    without suppressing valuable contrary ideas?
16. What is the smallest first vertical that can use a real accepted direction
    without prematurely generalizing a policy language?

## Core principle

> Work Engine should preserve accepted architectural direction, current adoption
> state, and authorized transition routes as distinct durable dependencies, then
> project only their applicable invariants, open points, current gaps, detours,
> restoration obligations, exceptions, and reopening conditions into each
> design and implementation workflow.

Architecture should shape ideas from intake onward while remaining challengeable,
revisable, attributable, and subordinate to the human authority that chose it.
