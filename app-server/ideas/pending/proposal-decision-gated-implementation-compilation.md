# Proposal: Decision-Gated Implementation Compilation

## Identity and state

- Proposal ID: `work-engine.decision-gated-implementation-compilation`
- State: candidate proposal and implementation track; not accepted, prioritized,
  or authorized for implementation
- Decision owner: user or future explicitly authorized portfolio owner
- Primary consumers: proposal workflow, claims capability, slice supervisor,
  implementation planner, builder, reviewer, and context lifecycle manager

This proposal introduces a durable compilation boundary between an accepted
proposal and implementation. It does not change proposal authority, authorize
implementation, select a permanent model-routing policy, or require context
replacement at any workflow boundary.

## Candidate and consequence

After a proposal has sufficiently stable meaning and placement, Work Engine
identifies the small set of unresolved choices whose routes would produce
materially different durable consequences. An authorized decision owner
resolves or delegates those choices in a versioned decision set. A
repository-grounded planning role then compiles the accepted proposal,
relevant claims, placement, and exact decision-set revision into an
implementation contract that a less capable or less expensive execution model
can follow without silently acquiring architectural judgment.

The intended consequence is not an implementation plan that cannot fail. It is
an execution environment in which every remaining choice is either:

1. mechanically constrained and verifiable;
2. explicitly delegated as bounded implementation discretion; or
3. recognized as unresolved and returned to the appropriate decision owner.

This creates a measurable opportunity to spend expensive-model inference on
research, architectural judgment, and implementation compilation while routing
bounded mechanical realization to a cheaper model when the compiled contract
is sufficiently constraining.

## Problem

Current proposal artifacts are precise about desired consequences, invariants,
authority, evidence, placement, uncertainty, and acceptance. They intentionally
leave multiple valid mechanisms available. That is necessary for sound
proposal formation, but it means an implementation agent may still need to
make architectural and mechanical design decisions while coding.

Sending such a proposal directly to a cheaper implementation model therefore
does not merely ask it to translate a decided design into code. It asks the
model to discover repository facts, resolve material ambiguities, select
mechanisms, construct tests, and implement the result. Prior Spark experience
suggests that this combined judgment surface is too large for reliable use.

Making the proposal narrative progressively more detailed is not a sufficient
solution. It can duplicate implementation in prose, increase expensive-model
output, prematurely select mechanisms during semantic formation, and still
fail to describe the exact repository transformation.

## Proposed workflow

The workflow adds two durable products after proposal meaning and placement are
sufficiently stable:

1. a material decision set; and
2. a repository-grounded implementation contract.

The resulting flow is:

1. Research produces evidence-backed claims and unresolved semantic questions.
2. Proposal formation defines the desired consequence, invariants, boundaries,
   placement, uncertainty, and acceptance needs.
3. Decision-surface formation identifies only unresolved route choices with
   materially different durable consequences.
4. The authorized decision owner selects, edits, delegates, defers, or rejects
   those choices.
5. The resulting decision set is sealed as an immutable revision.
6. An implementation compiler validates the repository against an exact
   revision and produces a bounded implementation contract.
7. A plan-conformance gate determines whether the contract is complete enough
   for the proposed executor class.
8. The selected builder implements the contract.
9. Verification and review measure semantic conformance, plan deviations,
   failures, repair cost, and total economic consequence.
10. Any newly discovered material decision returns through a versioned decision
    amendment rather than being selected silently during planning or coding.

No artifact advances merely because it exists. Proposal acceptance, decision
authority, plan acceptance, implementation authority, and review acceptance
remain separate consequences.

## Material decision surface

### Inclusion rule

An unresolved choice belongs on the material decision surface only when all of
the following are true:

1. at least two routes remain compatible with the proposal's current meaning,
   invariants, placement, and available evidence;
2. the routes differ in at least one durable or externally meaningful
   consequence; and
3. no existing authority-bound decision already determines the route.

Material consequence classes are closed initially to:

- canonical ownership or authority;
- public interface or behavior on which another component may rely;
- persistent identity, schema, history, compatibility, or migration;
- concurrency, failure, durability, or recovery guarantees;
- security, privacy, trust, or admission boundaries;
- dependency or operational footprint;
- difficult-to-reverse physical placement;
- accepted product scope or observable behavior; and
- evidence required to establish implementation acceptance.

The governing route-invariance question is:

> If either admissible route were selected, could every downstream consumer,
> owner, migration, recovery path, acceptance test, and authorized future plan
> behave identically?

If yes, the choice is ordinarily implementation discretion. If no, it is a
candidate material decision.

### Excluded implementation discretion

The decision surface does not ordinarily include:

- local helper names;
- equivalent internal decomposition;
- unobservable control-flow choices;
- test-file organization;
- formatting or comment style;
- replaceable local data structures;
- equivalent library choices with no operational or compatibility consequence;
  or
- another choice whose alternatives are route-invariant under the test above.

An excluded choice becomes material when repository evidence shows that it
changes one of the closed consequence classes. The classification depends on
consequence, not on whether the choice appears architectural in the abstract.

### Decision authority classes

Material decisions are classified as either reserved or delegated.

#### Reserved decision

Explicit disposition by the named decision owner is required when the choice:

- changes accepted product meaning or placement;
- contains a product, value, cost, or priority tradeoff;
- consumes authority reserved to a human or another owner;
- creates a security, privacy, migration, or destructive consequence;
- is expensive or difficult to reverse; or
- materially expands the accepted scope.

#### Delegated decision

An authorized planning role may select the route when the decision owner has
explicitly delegated the materiality class and bounded the valid alternatives,
constraints, evidence requirements, and reopening conditions. The selected
route and rationale remain durable and inspectable.

Absence of a reserved-decision marker is not delegation. Possession of the
proposal, repository, planning role, or implementation tool does not create
decision authority.

## Decision-set artifact

Each decision record contains at least:

- stable decision identity and schema version;
- exact proposal and placement revisions;
- the bounded decision question;
- material consequence class;
- explanation of why the choice is route-variant;
- governing invariants and authority boundary;
- evidence claims and repository observations with exact revisions;
- genuinely admissible alternatives;
- consequence analysis for each alternative;
- recommended route and confidence or unresolved uncertainty;
- decision owner and authority reference;
- disposition: selected, rejected, delegated, deferred, or returned for more
  evidence;
- selected route or exact delegation envelope;
- rejected routes and reasons when consequential;
- reopening conditions; and
- producer, evidence cutoff, predecessor, and integrity digest.

A decision set becomes sealed only when every included decision is either
resolved, explicitly delegated, or explicitly deferred with a consequence that
prevents unauthorized downstream work. Sealing makes that revision immutable;
later changes create a successor and reopen every implementation contract that
relied upon the superseded revision when applicability requires it.

Sealing does not prove that the decision surface was complete. Planning and
implementation retain a duty to return newly discovered material choices.

## Relationship among claims, decisions, and plans

Claims, decisions, and implementation instructions remain distinct durable
objects.

- A claim records an evidence-backed answer about the repository, environment,
  mechanism, or observed behavior.
- A decision selects or delegates one route among materially different valid
  possibilities.
- An implementation contract derives an exact repository transformation from
  accepted meaning, evidence, and decisions.

For example:

- Claim: proposal-packet discovery recursively recognizes every `packet.json`.
- Decision: experimental lineage records may be placed adjacent to the proposal
  family without changing proposal-packet meaning.
- Implementation constraint: no experimental artifact may be named
  `packet.json`, and an integration test must demonstrate discovery isolation.

The claims capability caches semantic research questions and their evidence so
the implementation compiler does not repeat broad repository research. It does
not turn recommendations into decisions or visibility into applicability. The
decision set owns route selection. The implementation contract owns the
derived patch instructions.

## Implementation basis

Before compilation, the workflow assembles an immutable implementation basis
containing:

- accepted proposal revision;
- placement revision;
- sealed decision-set revision;
- relevant claim revisions and evidence cutoffs;
- governing invariants and acceptance requirements;
- rejected routes whose reintroduction would change meaning;
- unresolved uncertainty that remains valid for the planned slice;
- repository commit and required generated or indexed-state revisions;
- applicable authority grants; and
- predecessor plans or implementations whose contracts remain binding.

The basis is a bounded dependency manifest, not a transcript or research dump.
Every included item must explain its implementation relevance. Missing
completeness remains visible; the manifest must not claim that retrieval found
all relevant evidence when its projection or coverage cannot support that
conclusion.

## Implementation compiler

The implementation compiler is initially a Sol-class planning role. It consumes
the implementation basis, performs targeted repository validation, selects
only delegated or route-invariant mechanisms, and emits a versioned
implementation contract.

The compiler may discover repository facts that revise a claim or expose a new
material decision. It must not conceal that discovery by adjusting the plan
silently. It returns one of three outcomes:

1. `compiled`: the slice is sufficiently constrained for a named executor
   class;
2. `returned_for_decision`: a newly discovered material route choice requires
   disposition or delegation; or
3. `blocked_by_evidence`: the proposal, claim basis, placement, authority, or
   repository binding is stale, contradictory, or insufficient.

### Implementation-contract contents

A compiled contract contains at least:

- stable plan identity, version, producer, and predecessor;
- exact implementation-basis digest;
- exact repository revision and dirty-worktree observations;
- bounded slice objective and non-goals;
- exact files, symbols, interfaces, or creation boundaries involved;
- selected mechanisms and the decision or delegation authorizing each material
  selection;
- data structures, schemas, operation signatures, and error behavior to the
  degree required by the executor class;
- ordered implementation increments and intermediate validity conditions;
- integration points and repository patterns to preserve or imitate;
- explicit implementation-discretion envelope;
- forbidden changes and rejected routes;
- tests, fixtures, commands, and expected observations;
- failure, recovery, migration, compatibility, and cleanup requirements where
  applicable;
- plan-deviation and stop conditions;
- expected evidence and completion receipts; and
- proposed executor class with an evidence-backed readiness assessment.

The plan should encode the minimum sufficient mechanical constraint. It should
not repeat proposal narrative or reproduce the intended patch line by line
unless exact text is itself the accepted consequence.

## Plan-conformance gate

Before implementation, an independent gate checks that:

- every material mechanism is bound to a sealed decision or valid delegation;
- every proposal invariant and acceptance consequence is either exercised by
  the slice or truthfully outside its boundary;
- repository references resolve against the bound revision;
- the executor receives objective tests or observations for every claimed
  completion consequence;
- implementation discretion is bounded rather than implied;
- newly discovered material ambiguity is absent or explicitly returned;
- the plan does not expand proposal or implementation authority; and
- the selected executor class is supported by prior evidence for comparable
  plan entropy and task shape.

Passing the gate establishes plan readiness only. It does not authorize the
slice or predict that implementation cannot fail.

## Execution model routing

Spark is the first candidate for bounded execution, not a permanent role
binding. Routing depends on the compiled contract and observed performance.

A cheaper executor is a candidate when:

- the repository locations and integration points are exact;
- material decisions are closed or validly delegated elsewhere;
- remaining discretion is local and route-invariant;
- tests provide objective feedback;
- the change is bounded and safely reversible;
- failure can be detected before acceptance; and
- expected repair cost does not erase the model-cost advantage.

A Sol-class builder remains appropriate when material architectural judgment,
cross-cutting semantics, weak observability, unsafe migration, broad discovery,
or high repair consequences remain.

The builder must stop and publish a plan-deviation report when repository
reality conflicts with a material plan assumption, a required reference is
missing, an acceptance observation cannot be constructed, or an unbound
route-variant choice appears. It may repair local mechanical defects within
the discretion envelope but may not reinterpret proposal meaning to make tests
pass.

## Context lifecycle boundary

Task models do not monitor, request, schedule, or optimize their own context
replacement. Proposal states, sealed decision sets, compiled plans, review
outcomes, token metrics, latency, durable continuation state, and workflow
transitions are observable inputs to the external context lifecycle manager.

The lifecycle manager independently determines whether replacement is safe,
beneficial, and ready. A workflow boundary is evidence about possible changes
in context value; it is not a reset command or a mandatory reset point. The
manager may retain the existing context when repository understanding, cached
inference, remaining work, transition cost, or measured quality makes
continuation preferable.

When the lifecycle manager selects replacement, the task model participates
only in the mechanically necessary `new_context` transition and the existing
narrow reconciliation boundary. Snapshot binding, sufficiency validation,
write fencing, exact checkpoint injection, successor verification, and
reconciliation remain owned by the context-lifecycle protocol.

This proposal therefore adds no context-management responsibility to proposal
formers, decision planners, implementation compilers, or builders. It only
makes new durable workflow transitions available as external optimization
signals.

## Invariants

- Proposal meaning, decision authority, plan acceptance, implementation
  authority, and implementation acceptance remain separate.
- Proposal formation does not silently choose mechanisms merely to make later
  execution cheaper.
- Materiality is determined by route-variant consequence, not by the apparent
  size or technical sophistication of a choice.
- Claims describe evidence-backed state; decisions select routes; plans derive
  transformations.
- A decision set is exact-versioned and never silently follows a successor.
- The implementation compiler cannot acquire reserved decision authority.
- The builder cannot acquire planning or proposal authority from possession of
  an implementation contract.
- Newly discovered material ambiguity returns upstream instead of being hidden
  as implementation discretion.
- Verification measures the realized consequence, not merely textual plan
  compliance.
- Total model economy includes planning, execution, review, retries, repair,
  latency, and failure consequence.
- Context replacement remains an external optimization decision and is never
  implied by workflow phase completion.

## Boundary

This proposal does not:

- replace proposal review, placement, or authority decisions;
- require every proposal to use Spark or another cheap executor;
- guarantee that a sufficiently detailed plan eliminates implementation error;
- turn claims into authoritative decisions;
- require line-by-line implementation specifications;
- require fixed context-reset points;
- choose the permanent schemas, filenames, or runtime owner for these artifacts;
- authorize automatic execution after plan compilation; or
- establish model-routing thresholds before measured evidence exists.

## Uncertainty and evidence needs

- Whether the material-decision criteria produce a small useful surface rather
  than flooding the user or omitting important choices.
- Whether a fresh Sol implementation compiler can rely on claims and the
  implementation basis without repeating most repository research.
- Whether preserving the research-bearing Sol context is sometimes cheaper or
  more accurate than fresh-context compilation.
- The minimum contract detail required for Spark across different slice types.
- Whether plan-conformance review costs less than the Sol implementation work
  it displaces.
- Which task characteristics predict Spark success strongly enough for routing.
- How often implementation planning discovers new material decisions after a
  decision set has been sealed.
- Whether a single decision schema remains truthful across architecture,
  persistence, interface, recovery, and security choices.
- Which claims require freshness checks immediately before compilation.

## Acceptance consequence

If accepted and implemented, Work Engine gains a durable, authority-preserving
path from semantically accepted proposal to executor-specific implementation
contract. Expensive-model judgment is reused rather than reconstructed,
material choices remain visible to their proper owners, and cheaper execution
is admitted only when measured plan structure has sufficiently reduced the
remaining judgment surface.

Acceptance does not establish that Spark is economical. That conclusion belongs
to the implementation track's measured evidence.

# Implementation Track

## Track objective

Determine whether decision-gated implementation compilation reduces total
model cost while preserving or improving semantic correctness, review burden,
latency, and recoverability.

The track separates artifact correctness from model-economy claims. A useful
decision surface or implementation contract may be retained even if Spark is
not economical. Conversely, one cheap successful implementation does not prove
the workflow generally useful.

## Stage 0: Baseline and measurement contract

Before changing routing, define a measurement record for recent Sol-built
slices and future compiled slices.

Record at least:

- proposal and slice identity;
- model and reasoning configuration by phase;
- research, proposal, decision, planning, implementation, review, retry, and
  repair tokens;
- cache use where observable;
- wall-clock latency by phase;
- number and severity of review findings;
- first-pass acceptance;
- plan deviations and upstream returns;
- human decision count and session time;
- tests introduced and defect-detection source;
- semantic defects, mechanical defects, and escaped defects; and
- total accepted-slice cost.

The comparison metric is total accepted consequence, not implementation-token
price in isolation.

Exit evidence: a versioned measurement schema and a usable baseline across a
small but varied set of completed slices.

## Stage 1: Decision-surface shadow mode

Generate material decision surfaces for already completed or currently
Sol-built proposals without changing implementation authority or model routing.
Compare the generated records with decisions actually encountered during
planning and implementation.

Measure:

- surfaced decisions later shown to be local implementation discretion;
- material decisions omitted and discovered downstream;
- alternatives that were not genuinely admissible;
- recommendation quality;
- human review burden; and
- stability of the closed materiality classes.

Use completed slices only when their known outcomes are labeled and cannot be
mistaken for prospective evidence.

Exit evidence: the decision surface is small enough for useful HITL review and
captures material downstream choices with an acceptable omission rate.

## Stage 2: Sealed decision-set pilot

For one bounded prospective proposal, run the decision-surface process and a
real HITL disposition before implementation planning. Preserve every edit,
delegation, rejection, and reopening condition in an immutable decision-set
revision.

Continue using a Sol-class builder. The purpose is to test whether the sealed
decision set actually constrains planning and implementation, not yet to test
Spark.

Exit evidence:

- the implementation planner can trace every material selection to the sealed
  decision set or a valid delegation;
- the builder does not need to invent an unauthorized material decision; and
- any newly discovered choice returns through the amendment path without
  losing or overwriting prior authority.

## Stage 3: Implementation-compiler shadow mode

Have a Sol implementation compiler generate a contract for a slice that will
still be implemented by Sol under the existing workflow. After completion,
compare the contract with the actual patch and implementation reasoning.

Classify each divergence as:

- valid local discretion;
- mechanical implementation error;
- stale or incorrect repository assumption;
- omitted material decision;
- necessary plan adjustment within delegated authority;
- unauthorized semantic drift; or
- overly prescriptive contract detail.

Measure the incremental Sol output and repository-research cost required to
compile the contract. This stage directly tests whether the compiler merely
rewrites the implementation in prose.

Exit evidence: the contract predicts and constrains the implementation with
substantially less Sol cost than the implementation work it could plausibly
displace.

## Stage 4: Controlled Spark execution

Select one small, reversible, strongly testable slice whose contract passes the
conformance gate and whose remaining choices are route-invariant. Bind Spark to
the exact plan, repository revision, authority, and discretion envelope.

Spark may:

- implement the ordered increments;
- run the named tests and inspections;
- correct local mechanical defects inside the discretion envelope; and
- return evidence when a stop condition occurs.

Spark may not:

- resolve or reinterpret a material decision;
- weaken tests or acceptance conditions;
- broaden the slice;
- substitute a rejected route;
- infer new authority; or
- continue after a material plan/repository conflict.

Sol review remains mandatory for the first pilot. Any repair must record
whether the defect arose from compilation, execution, repository drift, or an
invalid routing decision.

Exit evidence: one accepted implementation with complete cost accounting and
no hidden material decision by the executor. A single success authorizes no
general routing policy.

## Stage 5: Comparative pilot matrix

Run matched or closely comparable slices across:

- direct Sol implementation;
- Sol compilation followed by Sol implementation; and
- Sol compilation followed by Spark implementation.

Vary task shape deliberately:

- mechanical established-pattern change;
- bounded new vertical path;
- schema or migration change;
- cross-component integration;
- weakly observable semantic behavior; and
- remediation after review findings.

Estimate which contract and task properties predict accepted Spark execution.
Do not aggregate away severe failures merely because average token cost falls.

Exit evidence: an initial routing profile with explicit unsupported classes,
confidence, and failure consequences.

## Stage 6: Adaptive routing

Only after comparative evidence exists, allow the supervisor to nominate an
executor class from contract characteristics and historical outcomes. Routing
remains advisory until the appropriate authority accepts it for the slice.

The routing profile should consider:

- number and class of material decisions;
- amount of delegated discretion;
- repository binding stability;
- interface and component breadth;
- test and oracle strength;
- reversibility;
- migration, concurrency, recovery, and security consequence;
- historical executor performance on comparable contracts; and
- expected review and repair cost.

Route-confidence failure, repository drift, or unexpected ambiguity returns
the slice to Sol planning or implementation without treating Spark completion
as a goal in itself.

## Context-lifecycle integration

Expose the following durable states and metrics to the context lifecycle
manager:

- proposal meaning or placement stabilized;
- decision surface formed;
- decision set sealed or superseded;
- implementation basis assembled;
- plan compiled, returned, blocked, or accepted;
- execution completed, failed, or returned;
- review opened or closed;
- phase token use, latency, and checkpoint size; and
- estimated remaining work where available from workflow state.

The lifecycle manager may use these observations in its existing optimization
algorithm. No stage requests replacement, declares itself a reset point, or
requires the task model to reason about context. Replacement decisions and
their measurements remain external.

## Track stop conditions

Pause expansion of the track when:

- HITL decision burden becomes disproportionate to the implementation value;
- decision-surface omissions repeatedly appear only during coding;
- Sol compilation cost approaches or exceeds displaced Sol implementation
  cost;
- Spark repair or review cost erases the apparent savings;
- severe semantic defects escape the plan-conformance gate;
- plans become line-level duplicate implementations;
- claims fail to provide sufficiently fresh repository evidence; or
- executor routing optimizes token price while worsening accepted-slice cost,
  latency, or quality.

A stop condition is evidence for revising, narrowing, or rejecting the routing
strategy. It does not by itself invalidate the decision-set or implementation-
contract artifacts for Sol-built work.

## Initial success criteria

The first comparative evaluation should establish all of the following before
routine Spark routing is considered:

- material decisions are visibly owned and exact-versioned;
- the implementation compiler does not silently consume reserved authority;
- late material decisions follow the amendment path;
- compiled contracts are materially smaller than a prose restatement of the
  patch;
- Spark implementations remain within the discretion envelope;
- tests and review detect plan or implementation failure before acceptance;
- total accepted-slice cost is lower for at least one clearly characterized
  task class; and
- the evidence identifies task classes for which direct Sol implementation
  remains preferable.

## Authority

This implementation track is an evidence plan, not authorization to modify the
proposal workflow, claims system, context lifecycle manager, supervisor,
builder, model routing, or repository. Each stage requires its own bounded
authority, repository binding, and acceptance decision. Later-stage formation
does not imply permission to skip earlier evidence gates.
