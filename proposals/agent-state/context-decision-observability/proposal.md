# Proposal: Context-Decision Observability

## Identity and current state

- Proposal ID: `work-engine.context-decision-observability`
- Family ID: `work-engine.agent-state`
- State: reviewed and decided; proposal meaning approved with probable
  distributed semantic placement; not prioritized or authorized for
  implementation
- Decision owner: user or future explicitly authorized architecture owner
- Evidence cutoff: Work Engine
  `cdc9e3fa5d300e5edc737faf38edf85a336fbdcf`, validated intake checkpoint
  `2c9e4d85b4abb7830a3a607f93f7c5e821e8b031`, attributed runtime and
  documentation observations in [`formation-evidence.md`](formation-evidence.md),
  and the advisory review summarized in
  [`reviews/proposals/context-decision-observability/6eebd58/`](../../../reviews/proposals/context-decision-observability/6eebd58/)

The canonical lifecycle and placement projection is in
[`packet.json`](packet.json), and the authority-authored disposition is in
[`decision.json`](decision.json). This narrative owns the candidate's current
meaning.

Following specialized review and retained-context remediation, the user
approved this proposal's meaning. The decision provisionally places the
distributed semantic contract in `work-engine.agent-state` while preserving
the unresolved physical composition seam among host, shared Work Engine
access-plane, and role-local adapters. It does not settle permanent
architecture, change roadmap priority, authorize implementation, or authorize
a context replacement.

## Independently decidable consequence

Provide an authenticated, provenance-bearing, consumer-scoped observation
projection that helps an authorized role judge whether intentional context
replacement is likely beneficial without treating benefit evidence as safety
proof or replacement authority.

Canonical Wind Walker doctrine owns the continuation-safety invariant:
replacement is valid only when correct continuation depends on no meaning
represented solely in the current context. The currently authorized role or
agent owns the situated retain-or-replace judgment within human and role
authority. Stronger role or domain owners establish and attest any required
continuation state. The replacement mechanism may act only after independently
verifying the exact current safety and effect-authority grant.

The projection supplies only bounded benefit evidence: current context
pressure, context and representation lifecycle identity, automatic-context-
management state, a referenced continuation checkpoint, quantitatively
comparable change since that checkpoint when available, and the estimated or
measured footprint of the smallest sufficient authorized rehydration manifest.

The consequence is evidence for model judgment, not a reset policy. It does not
define a threshold, schedule replacement, prove continuation completeness,
authorize an effect, or own a replacement episode.

## Problem

Work Engine has a safety doctrine and the active Codex environment exposes an
intentional replacement capability, but Wind Walker cannot currently compare
retained-context burden with truthful rehydration cost from one bounded
observation surface.

The active environment can report remaining context tokens. The installed
runtime also distinguishes automatic context-management capabilities
internally. Those facts are not presently composed with:

- an exact invocation, worker, context, and active-representation identity;
- the invocation's effective hard context limit and acquisition contract;
- the latest applicable continuation checkpoint and a comparable token
  baseline when one exists;
- a true runtime growth counter or a same-epoch net occupancy delta;
- an integrity-bound smallest-sufficient rehydration manifest;
- estimated and later observed rehydration work bound to one transition; or
- authenticated consumer, source, freshness, applicability, and visibility
  constraints.

Cumulative input/output token metrics do not establish current active-context
occupancy. An advertised model maximum does not necessarily establish the
effective limit for one invocation. Equal source-local window strings do not
establish equal runtime or representation generations. A checkpoint may make
replacement safe without making it worthwhile. A hash authenticates bytes, not
the source's authority or the content's instruction status.

Without these distinctions, a numerically plausible packet can still describe
no coherent invocation state, leak stronger-owner references, or appear to
authorize an unsafe effect.

## Ownership and composition

No component acquires all context-lifecycle meaning merely by serving or using
the projection.

```text
canonical Wind Walker doctrine
  owns the replacement-safety invariant and its role-scoped projection

authorized role or agent
  owns the situated benefit judgment within current human and role authority
  and records the attributed decision with a stronger workflow owner

role or domain state owner
  owns objective, obligations, authority boundaries, continuation state,
  safety attestation, checkpoint meaning, and semantic recovery content

runtime or host adapter
  owns authenticated invocation, capacity, representation, compaction, and
  replacement-transition observations for the current runtime subject

source policy owners and access plane
  own read policy and enforce the intersection of grants for the authenticated
  consumer, purpose, role or episode, and exact source revision

context-decision projection
  owns bounded composition, compatibility checks, attenuation, derivations,
  and the model-facing view; it owns none of the referenced facts

workflow episode or transition-history owner
  retains the decision, grant, checkpoint, manifest, old/new generations,
  runtime result, measurement, and protected downstream consequence when a
  durable before/after join is required

runtime replacement mechanism
  performs an authorized effect only after independently verifying the exact
  current safety and authority attestation
```

The projection cannot mint or strengthen a safety attestation, effect grant,
source observation, read entitlement, checkpoint, manifest, decision, or
episode result. A missing, stale, conflicting, unauthenticated, inapplicable, or
visibility-denied source remains explicit and can make the view unsuitable for
a decision.

The likely neighboring owner for role checkpoints and durable decision episodes
is `work-engine.role-owned-durable-operational-state`, but this candidate does
not causally depend on accepting that entire broader proposal. Any authorized
implementation must instead identify a stronger owner that satisfies the
minimal checkpoint, attestation, manifest, and episode interfaces required by
the exercised role.

## Observation projection

The semantic surface should be capable of representing at least the following
distinctions. Names and serialization are illustrative.

```yaml
context_decision_observation:
  schema_version: 1
  packet:
    id: opaque-consumer-scoped-id
    revision: exact-projection-revision
    snapshot_cutoff: 2026-08-24T00:00:00Z
    consumer_binding:
      role_profile: non-human-facing-builder-v1
      episode: opaque-authorized-episode
      purpose: context-replacement-benefit-judgment
      policy_revision: exact-access-policy-revision
    coherence:
      decision_suitable: false
      reasons: [effective_limit_unverified]
    safety_not_established_by_packet: true

  runtime_subject:
    provider_session: opaque-scope
    invocation_generation: invocation-12
    worker_generation: worker-3
    context_generation: context-7
    representation_generation: representation-9
    predecessor_transition: transition-8
    automatic_context_policy_revision: exact-runtime-policy-revision

  source_snapshots:
    runtime-current:
      principal: authenticated-runtime-adapter
      verifier: host-observation-verifier-v1
      source_revision: exact-runtime-observation-revision
      integrity: exact-integrity-binding
      observed_at: 2026-08-24T00:00:00Z
      subject: invocation-12/context-7/representation-9
      freshness_rule: current-representation-only
      authorized_use: context-replacement-benefit-judgment

  fields:
    active_context_tokens:
      epistemic_origin: observed
      acquisition: runtime-active-context-counter
      source_observation: runtime-current
      availability: available
      validity: current
      applicability: decision-input
      value: 118000

    effective_hard_limit_tokens:
      epistemic_origin: derived
      acquisition: invocation-resolution-v1
      source_observations:
        - exact-invocation-config-observation
        - exact-model-catalog-observation
      availability: unavailable
      validity: unverified
      applicability: not-decision-input
      reason: invocation-effective-binding-not-established

    hard_tokens_remaining:
      epistemic_origin: derived
      derivation: subtract-v1
      exact_inputs: [active-context-observation, effective-limit-observation]
      availability: unavailable
      validity: input-unavailable
      applicability: not-decision-input

    continuation_checkpoint:
      epistemic_origin: referenced
      source_observation: exact-role-state-observation
      availability: available
      validity: current
      semantic_applicability: applicable-to-objective-and-obligations
      quantitative_comparability: incompatible-representation-generation
      value: attenuated-non-bearer-checkpoint-reference

    net_occupancy_delta_since_checkpoint:
      epistemic_origin: derived
      derivation: same-epoch-occupancy-subtraction-v1
      exact_inputs: [active-context-observation, checkpoint-baseline-observation]
      availability: unavailable
      validity: incompatible-input-epochs
      applicability: not-decision-input

    rehydration_footprint_estimate:
      epistemic_origin: estimated
      acquisition: manifest-tokenization-v1
      source_observation: exact-authorized-manifest-observation
      availability: available
      validity: current
      applicability: decision-input
      uncertainty: host-injected-context-unavailable
      value: authorized-minimum-precision-aggregate

    prior_rehydration_measurement:
      epistemic_origin: observed
      temporal_scope: historical-transition
      source_observation: exact-completed-transition-observation
      availability: unavailable
      validity: no-applicable-prior-transition
      applicability: not-decision-input
```

The projection may use a uniform field envelope, a source-snapshot table, or an
equivalent representation. It must keep at least these dimensions independently
legible when material:

- semantic owner and authenticated source principal;
- exact source observation, revision, integrity, subject, and cutoff;
- epistemic origin such as direct observation, durable reference, derivation,
  estimate, or historical observation;
- acquisition or derivation method and exact input observation identities;
- availability and access status, including denied without identifier leakage;
- freshness, validity, applicability, conflict, correction, and supersession;
- uncertainty and precision; and
- consumer purpose and permitted use.

No exclusive four-value field is required to carry all of these meanings.

## Coherence and lifecycle identity

Each packet has its own opaque identity, monotonic revision, consumer binding,
and snapshot cutoff. Every material input binds the composite runtime subject
and its own exact observation. The projection must either establish a coherent
cutoff and compatible subjects or identify the incompatibility. It does not
repair conflict by silently selecting the newest value.

The runtime contract determines when automatic compaction advances context or
active-representation generation. The projection reports that observed rule and
the exact transition; it does not infer compaction from a token discontinuity.
At minimum, it keeps these events distinct:

```text
ordinary continuation
automatic compaction or representation replacement
intentional Wind Walker replacement
worker, invocation, model, or provider replacement
```

A checkpoint can remain semantically applicable to the workflow while its token
baseline is quantitatively incomparable with the active representation. Those
states are separate. Cross-generation arithmetic is unavailable unless the
owning runtime provides an explicit compatible counter contract.

Subtracting checkpoint occupancy from current occupancy yields only a net
occupancy delta and only when tokenizer, counting basis, model resolution,
context generation, and representation epoch are compatible. Calling that
value growth requires a monotonic runtime growth counter with its own exact
contract.

Every derived value is bound to exact input observations and becomes
unavailable or conflicting when an input is missing, denied, stale,
superseded, unauthenticated, inapplicable, or quantitatively incompatible. The
packet exposes `decision_suitable: false` when any profile-required benefit
input fails those conditions. This suitability flag says nothing about safety
or effect authority.

## Source truth and non-substitution

No source may establish a stronger claim than its authoritative contract.
Attribution is not authentication; integrity is not observation authority;
availability is not freshness or applicability.

An observation admitted as current must identify its principal, verifier or
verification rule, exact subject and generation, source revision, integrity,
cutoff, freshness or anti-replay rule, and authorized use. Verification failure
produces unavailable or unsuitable evidence, never a model-authored repair.

Current examples of invalid strengthening include treating cumulative token use
as active occupancy, an advertised catalog maximum as an invocation-effective
limit, an ordinary summary as an owner-attested continuation checkpoint, or a
token discontinuity as proof of compaction. These are non-exhaustive examples,
not permanent bans on source forms: a future source is valid only to the extent
its authenticated contract explicitly establishes the target meaning.

An effective-limit value is directly observed only when the runtime supplies
the exact resolved invocation value. Otherwise it is derived from exact
provider, model, invocation, configuration, and catalog observations with a
named method, or unavailable when that binding cannot be established.

## Safety, authority, and situated judgment

The packet may be inspected while safety is unresolved. It may inform a cheap
judgment that replacement has no plausible benefit. It must never establish,
relax, or substitute for continuation safety.

Before any effectful intentional replacement, the runtime mechanism must
independently verify a current role-owner safety attestation and effect grant
bound to the exact role, workflow or episode, objective and obligation cutoff,
checkpoint and manifest revisions, old runtime/context/representation
generations, intended transition, and authorized actor. The projection cannot
mint that attestation or grant, and a `continuation_complete` label inside the
packet is not a substitute.

The currently authorized role or agent may use suitable benefit evidence to
compare such consequences as active occupancy, useful working-set age,
automatic-context-management state, comparable net change, rehydration
footprint, unresolved exploration, and the cost or risk of rebuilding runtime
orientation. The role records the situated judgment and reliance references
with the stronger workflow owner. No fixed ratio or threshold follows from this
proposal.

## Visibility, reviewer isolation, and metadata minimization

The projection is a consumer-specific view, not a generic shared packet. Each
source owner defines read policy; the access plane authenticates the consumer
and enforces the intersection of grants for role, episode, purpose, field,
reference, and exact revision. References are attenuated, scoped identifiers,
not ambient bearer credentials. Denial does not reveal the protected identifier
or aggregate information about inaccessible content.

Fresh initial reviewers receive no builder recovery content, persisted builder
judgments, or role-state manifest through this projection. A retained or
truthfully reconstructed reviewer may receive only its own review-episode state
and explicitly permitted immutable subject or gate evidence. Cross-specialist
and cross-role recovery is deny by default, regardless of whether a particular
review claims independence.

Audit-required provider, model, source, and integrity detail may remain in a
restricted provenance plane. The model-facing view exposes only fields and
precision causally useful to the authorized decision. Rehydration-size
estimation occurs inside the manifest owner's visibility boundary and exports
only an authorized aggregate. Live packets use consumer-scoped opaque
identifiers and profile-defined short retention that prevents them from becoming
cross-episode history or a correlation surface.

## Rehydration manifest and loading boundary

The smallest-sufficient rehydration manifest identifies exact durable owners,
revisions, integrity, ordered components, semantic classes, loading modes,
visibility grants, and unavailable host-injected components required for one
replacement. The observation projection receives only the authorized reference
and aggregates needed for benefit judgment.

Integrity does not confer instruction authority. Only governing instruction
owners may contribute directive entries through an instruction-authorized
loading mode. Role state, attributed judgments, repository content, external
text, and other untrusted material remain typed data or evidence with their
original authority and epistemic limits. They are isolated and rendered so
model visibility cannot silently promote them into governing instructions.

A pre-replacement estimate binds the exact manifest revision, component cutoff,
tokenizer or method, configuration, uncertainty, and omitted inputs. A later
measurement binds the actual ordered load, host-injected context, exact
transition, decision and grant references, and old/new runtime and context
generations. The live projection does not own this history. A stronger workflow
episode or transition-history owner retains the lossless join when required.

Role-specific semantic content remains owned by role state or stronger domain
artifacts. The separately captured bounded-ad-hoc-continuation-state idea may
later refine what a role checkpoint can reference or contain; this proposal
only observes an authorized checkpoint and manifest projection.

## First evidence vertical

The likely first consumer is a non-human-facing slice builder because it can
exercise a real unfinished engineering position, accepted boundary, repository
references, runtime observation, and concrete rehydration manifest. This route
can establish bounded feasibility only. It cannot settle permanent placement,
cross-role applicability, open-world continuation completeness, or the
`work-engine.agent-state` family boundary.

A first vertical should demonstrate consequence-equivalent evidence for:

1. authenticated current, referenced, derived, estimated, historical, denied,
   stale, conflicting, superseded, and unavailable states;
2. exact composite lifecycle identity, coherent snapshot composition, and
   invalidation of cross-generation arithmetic;
3. a role-owned checkpoint, safety attestation, manifest, and attributed
   decision episode referenced without semantic duplication;
4. independent runtime rejection of a missing, stale, replayed, or wrong-subject
   safety/effect grant;
5. consumer-scoped builder and deny-by-default fresh-reviewer views;
6. typed manifest loading that keeps instruction authority separate from
   durable data, judgments, and untrusted content;
7. rejection of a theoretical catalog maximum as an invocation-effective
   observation;
8. an externally owned oracle or seeded obligation that can reveal omitted
   continuation meaning, with any success reported only as bounded
   falsification; and
9. evidence discriminating host, shared Work Engine, and role-local seams,
   including at least one non-builder profile or an explicit role-local
   counterexample before any cross-role placement decision.

Fixtures, simulation, read-only host evidence, or another consequence-equivalent
route may establish these properties. An effectful replacement is not required
by this proposal. If a separately authorized evaluation includes one, it needs
an explicit current user or role grant, a fresh safety determination, and the
stronger episode owner before the effect occurs.

## Profile scope and placement uncertainty

Acceptance of a common semantic contract would not automatically enable it for
every role. Each consuming profile must declare its authenticated consumer and
purpose, human-facing applicability, safety and decision owners, effect-grant
boundary, source and visibility policies, required decision inputs, freshness
rules, lifecycle identity contract, retention, and independence constraints.

Evidence from the initial builder profile supports only that exercised role
class. Applying the projection to human-facing roles, fresh reviewers, or other
visibility-constrained consumers requires a separate evidence-backed profile or
architecture decision.

Permanent placement remains uncertain among runtime/host adapters, a shared
Work Engine composition access plane, and role-local adapters. Useful
discriminators include who can authenticate runtime facts, enforce intersected
source grants, maintain coherent cutoffs, avoid adapter duplication, preserve
role isolation, and retain the required episode join without making the live
projection canonical history.

## Deferred outcome-evidence candidate

The validated intake separately preserves a candidate for attributed
context-retention and replacement episodes joined to later resource,
correctness, and review outcomes. It has a research consumer and a longer,
potentially more sensitive lifecycle than this live operational projection.

Formation defers creating that proposal until stable subject, checkpoint,
decision, grant, transition, manifest, and measurement identities have been
exercised. A separately authorized first vertical may use a bounded evaluation
episode owner without deciding the future research owner's retention or
analytics contract. Deferral does not reject or merge the candidate.

## Evidence required before an authority decision

- Demonstrate that the semantic contract can represent exact lifecycle,
  source, coherence, access, applicability, correction, and derivation state
  without granting the projection canonical ownership.
- Establish which current or prospective host surface can authenticate active
  occupancy, invocation-effective limit, automatic-compaction transition, and
  composite runtime identity—or truthfully mark each unavailable.
- Bind an exercised role's checkpoint, safety attestation, manifest, and
  decision episode through minimal stronger-owner interfaces.
- Demonstrate fail-closed source verification, snapshot incompatibility,
  effect-grant rejection, least disclosure, reviewer isolation, and safe
  manifest loading through any valid evidence route.
- Exercise an external omission oracle or seeded obligation and report only the
  bounded falsification supported by the result.
- Compare placement discriminators across host, shared, and role-local seams;
  do not infer reusable placement from one builder implementation.
- Obtain architecture, authority/doctrine, lifecycle/evidence,
  security/visibility, and agent-instruction re-review of the remediated
  proposal before acceptance.

## Out of scope

- a universal role-state schema or ownership of role-specific durable meaning;
- proof that continuation is complete or that no context-only meaning exists;
- minting a safety attestation, read grant, source observation, or effect grant;
- a fixed reset threshold, schedule, automatic interruption, or mandatory
  replacement route;
- effectful replacement without separately authorized evaluation authority;
- raw transcript, hidden-reasoning, tool-log, or forensic archive ownership;
- canonical replacement-episode or long-lived outcome-history ownership;
- causal claims about correctness, quality, latency, or cost from one episode;
- permanent host, access-plane, role-profile, control-plane, storage, or family
  placement;
- automatic cross-role or human-facing applicability;
- proposal acceptance, roadmap priority, implementation authority, or
  formation of the deferred outcome-evidence candidate.

## Acceptance consequence

An authority decision on this proposal decides only whether Work Engine should
define a provenance-bearing, authenticated, consumer-scoped observation
contract that can compose runtime context pressure and lifecycle facts with
authorized role-owned checkpoint and rehydration references so an eligible
role can judge the benefit of a separately established safe intentional
replacement.

It does not decide whether or when a model resets, prove semantic completeness,
grant authority, own role state or history, require an effectful experiment,
select permanent placement, apply the contract to an unexercised role, form the
outcome-research candidate, change roadmap priority, or authorize
implementation.
