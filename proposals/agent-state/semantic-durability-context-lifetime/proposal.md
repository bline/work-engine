# Proposal: Semantic Durability and Context-Lifetime Judgment

## Identity and current state

- Proposal ID: `work-engine.semantic-durability-context-lifetime`
- Family ID: `work-engine.agent-state`
- State: formed with probable doctrine and runtime-projection placement; not
  evaluated, accepted, prioritized, or authorized for implementation
- Decision owner: user or future explicitly authorized architecture owner
- Evidence cutoff: Work Engine
  `cdc9e3fa5d300e5edc737faf38edf85a336fbdcf`, exact raw-source checkpoint
  `3d5a7574f508fd5f3b5861dff348fcb5a6234846`, validated intake-evidence
  checkpoint `21ececaee8b6d2ab97bfdbf70922239e9966c4c1`, and attributed formation
  evidence in [`formation-evidence.md`](formation-evidence.md)

The canonical lifecycle, placement, relationship, uncertainty, and authority
projection is in [`packet.json`](packet.json). This narrative owns the
candidate's current meaning. The raw idea and intake retain their separately
attributed source and interpretation authority.

## Independently decidable consequence

Define a model-facing semantic contract that treats context as conditionally
durable working memory, preserves continuation-relevant meaning when its
expected value and loss or reconstruction consequences justify externalization,
and permits context retirement only when continuation is semantically safe and
a fresh useful working set is expected to serve the next work better.

The contract protects a balance. It must avoid both losing meaning required for
correct continuation and turning durability management into per-turn state
inventory whose token, latency, attention, or rehydration cost equals or
exceeds the context cost it is intended to reduce.

Context pressure changes expected cost and risk. It is evidence for situated
judgment, not a trigger, threshold, cadence, urgency signal, or instruction to
save state. A context may become economically exhausted before its technical
limit when the remaining useful capacity is unlikely to support another
meaningful unit of work. That condition makes safe retirement a candidate; it
does not prove safety, benefit, or effect authority.

## Problem

Work Engine currently protects one side of context replacement well: Wind
Walker forbids replacement while correct continuation depends on context-only
meaning. Neighboring proposals describe role-owned recovery state and a
context-decision observation surface. They do not yet define the balancing
doctrine that tells an agent how to treat ordinary working context before that
safety boundary is reached.

Two apparently safe simplifications are harmful:

```text
retain context indefinitely
```

can repeatedly carry stale or low-value tokens, increase latency and context
competition, and leave too little room for another coherent unit of work.

```text
preserve every potentially useful thought
```

can consume more tokens, tool calls, attention, validation, and rehydration
work than retaining the context. It can also push the model toward anxious
self-monitoring and premature crystallization of tentative reasoning instead
of useful task work.

A fixed percentage, turn count, save cadence, or staged pressure procedure
would replace this false binary with another. The missing consequence is a
small semantic frame that leaves the actual preservation and retirement route
to informed model judgment.

## Semantic ownership and composition

No single context capability owns every meaning needed for this judgment.

```text
governing Work Engine design
  owns the general distinction among invariant safety, lifecycle objective,
  evidence affordances, and situated context-lifetime judgment

Wind Walker
  owns continuation independence and the compact runtime projection of the
  judgment for intentional context replacement

authorized role or agent
  owns the situated preservation and retirement judgment within current human
  and role authority

role or domain owners
  own correctness-, authority-, obligation-, and effect-critical continuation
  meaning and any publication contract that makes it durable

context-decision observability
  owns bounded composition of truthful pressure, lifecycle, checkpoint, and
  rehydration evidence without owning the decision

runtime or host
  owns direct context capacity, representation, compaction, and replacement
  observations and performs an authorized replacement effect

empirical context-lifetime calibration
  remains a deferred candidate for later outcome comparison and does not own
  live judgment
```

Model context may remain operationally available across many turns while never
becoming the durable semantic owner of workflow meaning. Calling it
"conditionally durable" describes expected availability for judgment; it does
not confer authority, canonical ownership, or guaranteed survival.

## Minimal semantic contract

Correct continuation must not depend on meaning that will disappear through an
intentional replacement. Replacement must not change or silently resolve the
objective, authority, commitments, obligations, uncertainty, or other workflow
meaning. These are validity boundaries regardless of the chosen route.

Within those boundaries, preservation and context lifetime remain model
judgment. Useful environmental affordances include suitable durable owners,
bounded representations, context evidence, and replacement capability.
Availability of those capabilities does not require their use and does not
prescribe polling, inventory, publication, or replacement order.

The governing objective is total lifecycle value: preserve correct
continuation and reasoning quality while reducing the combined cost of retained
context, preservation, validation, reconstruction, replacement, and
rehydration. Preventing information loss alone is insufficient when the
mechanism consumes comparable or greater resources or materially degrades task
reasoning.

## Mandatory and economic preservation remain distinct

Some meaning requires durability because its loss would make continuation
invalid, cross authority, duplicate an effect, abandon a commitment, or leave a
required obligation without an owner. The relevant role or domain contract
defines that protected consequence and remains binding regardless of context
economics.

Other meaning is useful but reconstructable. Its preservation value depends on
expected future relevance, loss risk, reconstruction cost and fidelity,
available stronger sources, preservation overhead, and likely future use.
Material reconstruction cost is evidence in that judgment; by itself it does
not create a universal publication command.

This distinction prevents two ownership failures:

- economic preference cannot relax a real role-state or domain invariant; and
- a broad durability command cannot absorb all valuable working knowledge and
  eliminate context-lifetime judgment.

When no typed owner fits useful continuation meaning, a bounded attributed
memento may be an available representation. It remains role-owned data or
evidence, not an instruction, fact upgrade, new authority source, or reason to
create a parallel universal-memory owner.

## Context pressure is evidence, not procedure

Declining useful capacity can increase the expected risk and cost of leaving
important meaning transient. It can also reduce the value of beginning work
whose coherent completion probably will not fit. Those effects may reasonably
change preservation sensitivity as the context evolves.

They do not establish a fixed behavioral progression. The contract does not
require the model to classify pressure into stages, poll capacity every turn,
lower a numeric threshold, save a fixed state set, or perform a full semantic
inventory at any percentage. Pressure remains one input among the value,
risk, reconstructability, authority, uncertainty, and cost consequences of the
actual work.

Successful behavior keeps durability management low-salience during ordinary
work. Task attention remains primary unless concrete semantic or lifecycle
evidence gives preservation or retirement credible decision value.

## Economic exhaustion and retirement judgment

A context can become economically exhausted before it reaches a technical hard
limit. Relevant evidence may include whether the next meaningful unit of work
is likely to fit, the value and interference of retained working context, the
footprint of residual meaning not yet durable, rehydration work, reconstruction
risk, automatic context-management state, and the consequences of delaying a
transition.

Retirement becomes a candidate when continuation is semantically safe and the
expected value of carrying the current context is lower than preserving the
remaining useful meaning and rehydrating a fresh working set. This comparison
is conceptual. Its inputs may be observed, derived, estimated, denied, stale,
conflicting, or unavailable. No formula, score, percentage, or single missing
measurement determines the result.

The model may judge that remaining capacity cannot support another meaningful
unit, preserve residual continuation-relevant meaning not already owned
elsewhere, and use an authorized context-replacement capability. That is an
illustrative valid route, not a required final-harvest ritual or sequence.

Semantic safety and effect authority remain independent. Economic benefit
cannot authorize replacement, and a safe checkpoint does not make replacement
worthwhile.

## Human-facing interaction boundary

Direct human interaction can carry unresolved preference, correction,
attribution, and authority that may not be faithfully externalizable. A durable
representation can preserve an unresolved interaction without resolving it or
transferring the human's authority.

The current runtime-projection scope is non-human-facing work whose continuation
state has explicit owners. The general semantic distinctions may inform
human-facing contexts, but automatic or model-initiated replacement in a human
conversation requires a separately established interaction contract and
authority. This proposal does not supply either.

## Relationship to neighboring candidates

### Context-Decision Observability

`work-engine.context-decision-observability` provides evidence useful to the
judgment. Its pressure and rehydration observations do not prove completeness,
set a threshold, or own preservation and retirement policy. This proposal
defines the consumer consequence without requiring every observation to be
available.

### Role-Owned Durable Operational State

`work-engine.role-owned-durable-operational-state` proposes stronger ownership
and publication for resume-critical operational consequences. That boundary is
compatible when "resume-critical" names a concrete correctness, authority,
obligation, or effect consequence.

Its current reference to avoiding "material reconstruction" is broader. If it
were treated as an invariant by itself, it could require continual publication
of economically useful working meaning and erase this proposal's balancing
judgment. Formation therefore preserves reconstruction materiality as evidence
unless an owning role profile identifies the protected consequence that makes
publication causally necessary. This is a proposed boundary reconciliation,
not acceptance or revision of the neighboring candidate.

### Bounded continuation mementos

The bounded-ad-hoc-continuation-state idea may refine role-owned recovery when
important residual meaning has no typed home. This proposal neither forms nor
owns that mechanism. Repetition may nominate a typed concept but cannot create
one or grant authority automatically.

### Empirical context-lifetime calibration

The intake's second surviving candidate would join preservation and retirement
decisions to later resource, rehydration, correctness, review, and
reasoning-behavior outcomes. Formation defers that candidate until stable
operational identities and truthful measurements have been exercised. It
remains independently formable and is not merged into this live doctrine.

## Evidence needed before an authority decision

- Exercise the contract against a non-human-facing builder context without
  requiring an effectful replacement.
- Show that ordinary work remains primary and that the contract does not induce
  per-turn inventory, repeated saving, defensive preservation, or reset panic.
- Demonstrate that a fresh continuation can recover every seeded mandatory
  consequence while economically useful but reconstructable working meaning
  remains subject to judgment.
- Use truthful available, unavailable, estimated, stale, and conflicting
  context-decision evidence without manufacturing precision.
- Compare at least one retained-context decision and one safe-retirement
  candidate by total lifecycle consequences rather than token count alone.
- Review the present normative text for structural necessity, exact-route
  causality, semantic ownership, authority scope, audience, and loading reach.
- Determine the smallest canonical doctrine and Wind Walker projection that
  preserve the consequence without loading detailed economic exposition into
  every agent context.

## Present instruction consequence and review applicability

This proposal contains a present model-facing semantic contract, not merely a
request for a future instruction artifact. The materially normative subjects
are:

- `Minimal semantic contract`;
- `Mandatory and economic preservation remain distinct`;
- `Context pressure is evidence, not procedure`;
- `Economic exhaustion and retirement judgment`; and
- `Human-facing interaction boundary`.

Examples, possible evidence, and conceptual comparisons are identified here as
non-binding formation content. The applicable review contract owns whether an
exact route is causally required by the protected consequence. Review advice
cannot accept the proposal or authorize a design, runtime instruction, effect,
or implementation.

## Out of scope

- a fixed context percentage, token threshold, turn count, score, schedule, or
  preservation cadence;
- mandatory per-turn polling, semantic inventory, checkpointing, or state
  publication;
- proof that continuation is complete or that no context-only meaning exists;
- automatic context replacement or an effect grant;
- telemetry, storage, role-state, memento, compaction, or replacement-mechanism
  implementation;
- one universal schema or semantic owner for model memory;
- raw transcript, hidden-reasoning, or forensic archive ownership;
- empirical calibration or causal claims about an optimal retirement policy;
- automatic application to human-facing conversations;
- proposal acceptance, permanent placement, roadmap priority, implementation
  authority, or source cleanup.

## Acceptance consequence

An authority decision on this proposal decides only whether Work Engine should
adopt a semantic-durability and context-lifetime judgment contract that treats
context as conditionally durable but non-authoritative working memory,
preserves mandatory continuation consequences through their proper owners,
keeps discretionary preservation low-salience and economically situated,
allows pressure to inform judgment without becoming procedure, and permits
retirement only when it is both semantically safe and expected to improve the
next useful work's total lifecycle consequences.

It does not decide a threshold, cadence, procedure, observation schema, storage
mechanism, role profile, human-interaction contract, empirical policy,
replacement event, permanent placement, roadmap priority, or implementation
authority.
