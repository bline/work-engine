# Proposal: Claim Maintenance and Reliance Propagation

## Identity and state

- Proposal ID: `work-engine.claim-maintenance-and-reliance-propagation`
- Family ID: `work-engine.evidence-lineage`
- Decision record: [`decision.json`](decision.json)
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
The production **operation-contract surface** for impact nomination and
refresh-episode admission — the bounded next unit of work named in
[`app-server/docs/claim-maintenance-and-reliance-propagation-reconciliation.md`](../../../app-server/docs/claim-maintenance-and-reliance-propagation-reconciliation.md) —
is designed in
[`operation-contract-surface.md`](operation-contract-surface.md), not yet
reviewed or implementation-authorized.

The user authorized formation of both production phases with the stated intent
to complete the entire claim-lineage consequence. This proposal preserves the
second phase as committed product direction for later evaluation; formation
does not itself prioritize, accept, schedule, or authorize implementation.

## Current formation state

The placement was revised on 2026-10-10 on later evidence
([`evidence/codebase-memory-correspondence-20261010.md`](evidence/codebase-memory-correspondence-20261010.md)).
Two conclusions are distinct:

- **Architectural feasibility is supported.** Correspondence between claims
  and changing code needs no externally writable graph projection. The
  evidence-anchor substrate observes declared anchors, and Codebase Memory is
  the probable code-observation backend and discovery source. A second graph
  engine is not presently justified.
- **Provider readiness is not.** The tested Codebase Memory revision
  demonstrably cannot attribute an observation to one coherent,
  content-verified snapshot through its supported interface. Its exposed
  generation is frozen across incremental re-indexes. Its snippets apply
  indexed line ranges to current disk bytes and can return another entity's
  code. Its freshness is judged by mtime and size. Production use is blocked
  by that demonstrated deficiency, not merely by an unselected design.

The current placement judgment is:

- canonical claims, authored relationships, declared anchors, nominations,
  judgments, reliance, and obligations remain Git-backed claim-system state;
- evidence-anchor observers resolve declared anchors against one exact
  observed snapshot, compare them with their bound observations, and derive
  nomination candidates;
- Codebase Memory supplies structural observation and non-authoritative
  discovery, and hosts no claim-system records;
- `repo-search` remains the backend-neutral evidence contract presented to
  builders, reviewers, and other roles;
- the code-change profile owns immutable before/after structural identity and
  reconciliation; and
- claim maintenance owns candidate-impact nomination and every semantic
  disposition.

Beyond that provider dependency, the remaining uncertainty is
implementation-contract correctness. Formation has not yet selected the exact
realization of:

- the declared code-anchor locator and its bound observation;
- the snapshot-observation contract a backend must satisfy, including
  dirty-worktree state;
- truthful unresolved and successor-candidate reporting;
- binding each observation to the exact canonical revision it interpreted; or
- the boundary between retrieved impact candidates and canonical `may_affect`
  nominations.

These are bounded design questions for later evaluation and implementation
planning. They do not reopen the ownership conclusion unless no observation
backend can satisfy the required invariants. The placement remains probable;
production readiness waits on a backend that provides coherent snapshot
observation.

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

The claim-maintenance view joins current structural entities to exact
canonical claim references and active reliance. It can expose related claims,
unresolved anchors, and unmapped behavior or normative documentation meaning
within declared coverage. A clean result is not proof that no semantic effect
exists.

The view is derived on demand from canonical records and snapshot-bound
observations. It has no canonical owner of its own and no permanent provider
dependency, and its loss requires only recomputation. Full integration
semantics, preserved observation consequences, role-facing queries, dependent
consumers, and provider reopening conditions are in
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
slice lifecycle, code-change profile, observation backend, `repo-search`, role
state, scheduler, and provider sessions can supply evidence or transport but do
not acquire claim meaning or semantic authority. Detailed placement and
rejected-owner reasoning is in [`placement.md`](placement.md).

## Invariants

- Mechanical impact never becomes semantic invalidation or causality.
- Deriving or recomputing correspondence never grants authority to create,
  replace, revise, or delete canonical claims, relationships, declared anchors,
  nominations, judgments, reliance, or obligations.
- Every observation binds the exact canonical revision and repository subject
  it interprets to one coherent, content-verified index snapshot.
- An incomplete or inconsistent observation remains explicit and is never
  presented as complete.
- An unresolved anchor retains its expected identity and cause, and successor
  candidates never silently rebind it.
- Derived correspondence views are recomputable from canonical records and
  observations without a second canonical registry. Losing them requires
  recomputation, not semantic recovery or human reconstruction.
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
desired. The formerly placed `sync_projection` mechanism is no longer part of
this candidate's placement. A generic projection capability would need an
independently desired product outcome from another consumer.

## Evidence and acceptance needs

Formation evidence includes the reviewed semantic parent, its deferred
decision, dogfood evidence, the production-interface dependency, the
slice-checkpoint lifecycle, the deterministic physical code-change profile,
the formed code-change-characterization proposal, direct Verify-tier
inspection of Codebase Memory, and later shadow correspondence experiments
against a newer Codebase Memory revision.

Repository evidence cutoffs:

- Work Engine initial formation: `cdc9e3fa5d300e5edc737faf38edf85a336fbdcf`
- Work Engine repository-integration refinement:
  `2d99c5b63e983d9e6e0c9e5e2fc4a436eb21e6aa`
- Codebase Memory feasibility inspection:
  `51c48e2093e9ad6c375b396e9624a0135cfb8351`
- Codebase Memory correspondence experiments (2026-10-10):
  `ec4fbd3a`, recorded in
  [`evidence/codebase-memory-correspondence-20261010.md`](evidence/codebase-memory-correspondence-20261010.md)

The dogfood positively represented the backbone semantics but did not exercise
real consumers, adversarial concurrency, complete delivery recovery, or
outcome-independent falsification. The Codebase Memory evidence supports the
observation placement and demonstrates that the tested provider is not ready
for it; it does not establish implementation correctness. Acceptance
therefore still requires a real end-to-end vertical and the adversarial matrix
in [`acceptance.md`](acceptance.md).

## Uncertainty

- Which domain workflows need active canonical-support maintenance rather than
  immutable historical claims plus on-demand assessment.
- Whether one obligation-record core remains truthful across proposal research,
  review, and later consumers.
- Which delivery affordances already available in the control plane can satisfy
  cross-owner consequences without a claim-specific coordinator.
- What concurrency and scale require from derived correspondence views and
  their observation backend.
- Which residual uncertainty downstream decision owners may accept without
  changing their existing authority contracts.
- Which documentation and architectural claim profiles should enter the first
  production vertical and which stable entity or anchor identities they need.
- Whether candidate-bound impact nomination should be a universal slice
  affordance or enabled only when a campaign, claim domain, or observed change
  makes semantic maintenance relevant.
- Whether Codebase Memory, or another backend, will provide coherent,
  content-verified snapshot observation through a supported interface, and
  what that contract must guarantee.

Reopening conditions are canonical in `packet.json`. The probable Codebase
Memory placement reopens if it cannot provide coherent, content-verified
snapshot observation and explicit unresolved anchor state through a supported
interface, or if another backend becomes necessary for correctness rather than
convenience or scale.

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
