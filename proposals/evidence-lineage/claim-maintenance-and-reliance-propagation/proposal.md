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
current meaning. Supporting artifacts elaborate its
[semantic model](semantic-model.md),
[repository integration](repository-integration.md),
[acceptance needs](acceptance.md), [placement](placement.md), and
[relationships](relationships.md). The verified current-provider observations
are retained separately as
[Codebase Memory feasibility evidence](evidence/codebase-memory-feasibility.md).

The user authorized formation of both production phases with the stated intent
to complete the entire claim-lineage consequence. This proposal preserves the
second phase as committed product direction for later evaluation; formation
does not itself prioritize, accept, schedule, or authorize implementation.

## Current formation state

Architectural feasibility of the initial repository-maintenance integration is
resolved at the current evidence cutoff. Codebase Memory is structurally
capable of serving as the probable default current-world graph projection and
traversal provider after a bounded, reindex-safe projection-synchronization
extension. A second graph engine is not presently justified.

The current placement judgment is:

- canonical claims, authored relationships, nominations, judgments, reliance,
  and obligations remain Git-backed claim-system state;
- a projection compiler derives a cacheable projection specification from
  those canonical records;
- Codebase Memory may materialize that specification against one exact current
  index generation for traversal;
- `repo-search` remains the backend-neutral evidence contract presented to
  builders, reviewers, and other roles;
- the code-change profile owns immutable before/after structural identity and
  reconciliation; and
- claim maintenance owns candidate-impact nomination and every semantic
  disposition.

The remaining uncertainty is implementation-contract correctness rather than
basic graph feasibility. Formation has not yet selected the exact realization
of:

- structural namespace ownership and owner-aware edge identity;
- a content-bound index-generation token, including dirty-worktree state;
- atomic inclusion of evaluated projections in a published graph generation;
- truthful unresolved endpoint records;
- preservation and rebuild of compiled projection specifications without
  creating a second authored registry; or
- the boundary between retrieved impact candidates, derived cache edges, and
  canonical `may_affect` nominations.

These are bounded design questions for later evaluation and implementation
planning. They do not reopen the ownership conclusion unless Codebase Memory
cannot satisfy the required invariants through a bounded extension.

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

One initial production integration is bidirectional code-and-documentation
maintenance. Stable claims and relationships may reference implementation
symbols, tests, schemas, configuration objects, and documentation anchors. An
exact repository change can nominate the relationships it may affect, and a
documentation change can nominate implementation and verification surfaces
that may no longer realize its meaning. Ordinary movement or line-number drift
does not require canonical identity replacement.

## Dependency and independently decidable boundary

This proposal depends on
[`production-claim-evidence-interface`](../production-claim-evidence-interface/proposal.md).
It consumes that proposal's stable identities, immutable revisions, authority
profiles, exact-revision reliance, publication mechanics, and projection
provenance. It does not redefine them.

This phase is independently decidable because its consequence is operational:
maintain and propagate the implications of already-productionized claims. The
foundation remains useful if this phase is delayed; this phase is invalid
without the foundation. Acceptance of phase one therefore neither accepts nor
completes this proposal.

## Semantic chain

An immutable source event may produce an attributed `may_affect` nomination
against one exact claim revision. Nomination asserts candidate impact only. It
does not make a claim false, stale, inapplicable, insufficient, reopened,
refreshed, or causally changed.

An authorized domain refresh owner investigates the nomination and publishes
an immutable judgment. Retained-unchanged and changed outcomes create new
evidence-bound claim revisions; inapplicable, insufficient, contested,
deferred, and superseded outcomes preserve their distinct consequences without
manufacturing a revision. Changed causality is recorded only when the
authorized owner adjudicates it.

Reliance remains bound to exact claim and consumer revisions. Later support
does not silently advance a consumer. When candidate impact or changed support
reaches active reliance, the owning domain workflow may create a durable
selective-reopening obligation. Delivery, receipt, acknowledgement, semantic
disposition, and completion remain separate, recoverable consequences.

Concurrent branches, conflicts, authority changes, inactive recipients, and
context replacement never select or erase semantic state implicitly. The full
record contracts, state distinctions, query consequences, and recovery model
are elaborated in [`semantic-model.md`](semantic-model.md).

## Repository and documentation integration

The slice lifecycle supplies both the accepted plan's expected semantic-impact
reference and an immutable attributed before/after repository subject. A
replaceable code-change profile derives physical and structural entity lineage
across those exact worlds. Planned scope, actual change, structural matching,
candidate impact, and semantic judgment remain distinct evidence and authority
boundaries.

The claim-maintenance projection joins current structural entities to exact
canonical claim references and active reliance. It can expose related claims,
unresolved endpoints, and unmapped behavior or normative documentation meaning
within declared coverage. A clean projection is not proof that no semantic
effect exists.

Codebase Memory is the probable current provider for that projection, not its
canonical owner and not a permanent product dependency. Projection loss must
be recoverable by deterministic recompilation from canonical records. Full
integration semantics, projection-publication invariants, role-facing queries,
and provider reopening conditions are in
[`repository-integration.md`](repository-integration.md).

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

General delivery machinery owns transport admission, durable routing, receipt,
retry, and delivery reconciliation under the supplied obligation identity. The
slice lifecycle, code-change profile, projection provider, `repo-search`, role
state, scheduler, and provider sessions can supply evidence or transport but do
not acquire claim meaning or semantic authority. Detailed placement and
rejected-owner reasoning is in [`placement.md`](placement.md).

## Invariants

- Mechanical impact never becomes semantic invalidation or causality.
- Rebuilding an affected projection never grants authority to create, replace,
  revise, or delete canonical claims, relationships, nominations, judgments,
  reliance, or obligations.
- A projection owner can replace only its own projection material and cannot
  overwrite parser-produced structure or another owner's projection.
- Every synchronized projection is bound to the exact structural index
  generation and repository subject against which its endpoints were resolved.
- A published structural generation never silently omits its evaluated
  projection; unresolved endpoints remain explicit rather than disappearing.
- The preserved projection specification is compiled, cacheable state. Losing
  it requires recompilation, not semantic recovery or human reconstruction.
- Planned scope, observed repository change, structural matching, retrieved
  impact candidates, canonical nomination, demonstrated behavior, and
  authorized refresh judgment remain distinct.
- Code and documentation changes are symmetric trigger sources even when their
  extractors and domain owners differ.
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
- a particular graph provider as permanent product doctrine;
- research maturity levels, readiness scoring, or organizational compilation;
- automatic acceptance, semantic nomination, reopening, scheduling, role
  activation, or execution;
- one universal maintenance role across every claim domain; or
- a graph database or always-running runtime as canonical semantic state.

Those outcomes require independently owned product consequences if later
desired. The `sync_projection` mechanism does not presently justify its own
proposal because it is enabling machinery for this candidate rather than an
independently desired product outcome.

## Evidence and acceptance needs

Formation evidence includes the reviewed semantic parent, its deferred
decision, dogfood evidence, the production-interface dependency, the
slice-checkpoint lifecycle, the deterministic physical code-change profile,
the formed code-change-characterization proposal, and direct Verify-tier
inspection of Codebase Memory.

Repository evidence cutoffs:

- Work Engine initial formation: `cdc9e3fa5d300e5edc737faf38edf85a336fbdcf`
- Work Engine repository-integration refinement:
  `2d99c5b63e983d9e6e0c9e5e2fc4a436eb21e6aa`
- Codebase Memory feasibility inspection:
  `51c48e2093e9ad6c375b396e9624a0135cfb8351`

The dogfood positively represented the backbone semantics but did not exercise
real consumers, adversarial concurrency, complete delivery recovery, or
outcome-independent falsification. Codebase Memory inspection established
current-provider feasibility but not implementation correctness. Acceptance
therefore still requires a real end-to-end vertical and the adversarial matrix
in [`acceptance.md`](acceptance.md).

## Uncertainty

- Which domain workflows need active canonical-support maintenance rather than
  immutable historical claims plus on-demand assessment.
- Whether one obligation-record core remains truthful across proposal research,
  review, and later consumers.
- Which delivery affordances already available in the control plane can satisfy
  cross-owner consequences without a claim-specific coordinator.
- What concurrency and scale require from the replaceable query projection.
- Which residual uncertainty downstream decision owners may accept without
  changing their existing authority contracts.
- Which documentation and architectural claim profiles should enter the first
  production vertical and which stable entity or anchor identities they need.
- Whether candidate-bound impact nomination should be a universal slice
  affordance or enabled only when a campaign, claim domain, or observed change
  makes semantic maintenance relevant.
- Which projection synchronization realization best preserves owner isolation,
  generation binding, atomic publication, and explicit unresolved state.

Reopening conditions are canonical in `packet.json`. The probable Codebase
Memory placement reopens if it cannot preserve those projection invariants
through a bounded extension or if another provider becomes necessary for
correctness rather than convenience or scale.

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
