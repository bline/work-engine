# Proposal: UI Experience Evidence Interface

## Identity and state

- Proposal ID: `work-engine.ui-experience-evidence-interface`
- Family ID: `work-engine.ui-experience-evidence`
- State: placement uncertain; formed but not reviewed, evaluated, accepted,
  prioritized, or authorized for implementation
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. Supporting artifacts describe its [placement](placement.md),
[relationships](relationships.md), and [formation evidence](../formation-evidence.md).

The user authorized durable formation of this direction. That authorization
does not approve the candidate, accept its dependencies, choose a roadmap
position, or authorize a dogfood or production implementation.

## Candidate and consequence

Define a shared, revisioned UI experience-evidence interface through which
several authorized consumers can identify an exact human-facing view or
transition, discover and extend current multimodal evidence, reuse cached
provider analysis, bind exact claim or observation reliance, and selectively
refresh affected evidence without each consumer independently reconstructing
the browser and product state.

A fresh consumer can determine what experience was observed, how its identity
was established, which route merely demonstrated reachability, which access and
interaction conditions were covered, what each producer actually observed,
which provider-derived results remain current, which claims or review findings
relied on exact revisions, and what changed or remains unsupported.

The interface preserves evidence and questions. It does not decide whether an
interface is good, whether an observation proves a claim, whether a claim is
accepted, or what consequence a proposal or implementation should receive.

## Required distinctions

### Semantic view identity and reachability

A stable view-contract revision identifies the application surface, bounded
subject and product context, observable state predicate, relevant semantic
anchors, and access conditions needed to distinguish the view. A URL, raw
selector, screenshot path, or preferred sequence cannot silently become the
semantic identity.

An attributed navigation witness identifies a starting condition, interactions,
settling evidence, resolved anchors, resulting predicate, side effects,
limitations, and cleanup outcome for one demonstrated route. Multiple witnesses
may satisfy one view contract. Exact route, sequence, or trigger belongs in the
view contract only when correctness of the experience causally requires it.

### Experience state and transition topology

The interface can address view states and transitions at enough granularity to
distinguish viewport, theme, device, data, actor, input modality, document and
container scrolling, hover and pointer state, keyboard focus, selection,
expansion, overlays, errors, and material temporal conditions when they affect
the evidence question.

Scrolling is not reduced to extra layout tiles: it can be both an interaction
edge and part of the resulting state. Hover, focus, pointer, keyboard, drag,
input, resize, and wait conditions can likewise create meaningful state or
transition evidence. The topology may branch and may contain several journeys;
the contract does not require one linear tour.

### Observation, claim, and review judgment

An observation remains an attributed fact about an exact captured state,
transition, region, or element. A claim remains a revisioned proposition whose
support may use observations. A review judgment applies an attributed design
lens to evidence and claims for a named decision scope.

Durable storage, provider structure, repeated agreement, or downstream reuse
does not promote an observation into a claim or inflate the authority of any
judgment. The interface exposes ownership and provenance rather than merging
these semantic acts.

### Evidence kinds and correspondence

The candidate can preserve and correlate, without conflating:

- DOM, accessibility tree, computed style, geometry, relationship, and scroll
  observations;
- full-view, regional, tiled, or otherwise bounded pixel artifacts;
- before/after or bounded temporal evidence;
- interaction events, targets, modalities, and resulting states;
- accessibility-probe inputs, results, governing-profile references, and
  limitations; and
- repository or domain evidence about the machinery the interface represents.

Every material artifact or observation binds its exact state, producer,
capture epoch or equivalent lifetime, source digest, coverage, and limitations.
Structural evidence cannot silently support a human-perceptual conclusion, and
pixel evidence cannot silently support exact semantic or behavioral claims.
Unsupported evidence sufficiency is a first-class outcome.

### Cached provider analysis

A provider-analysis record binds the immutable source artifact or region,
view-state revision, analysis-contract revision, producer and provider/model,
configuration and prompt digest, timestamp, usage, raw response, normalized
observations, uncertainty, and limitations. Later claims and findings reference
the derived record rather than being embedded into it.

Reuse requires identity of every material analysis input. A cache hit proves
input correspondence only. A new provider, model, analysis contract, source
artifact, or material region can create a distinct revision without rewriting
historical output. Exact measurements or element correspondence remain eligible
for structural corroboration when material.

### Accessibility evidence across the human access surface

The candidate represents explicit accessibility-probe evidence without making
accessibility a fifth UI-design vote. Applicable probes may address accessible
name/role/state, keyboard reachability and order, focus visibility and
obstruction, text and non-text contrast across interaction states,
color-independent meaning, reflow, clipping, target size, pointer alternatives,
reduced motion, dynamic announcements, and instruction or error association.

The owning product, legal, ethical, or strategy authority establishes the
intended human access boundary and any governing profile. Evidence producers
record applicable probes, omissions, unsupported conditions, and limitations.
Passing one probe cannot establish the accessibility of the entire experience.

### Exact reliance and selective refresh

Historical captures and analyses remain truthful about the evidence world they
observed. Separately revisioned applicability can identify a capture run,
journey, view, state, transition, access context, scroll region, artifact,
analysis, observation, claim, or review reliance as a candidate for refresh.

An implementation completion or other immutable source event may nominate
`may_affect` relationships using observed change evidence. It cannot declare a
visual claim false, attribute semantic causality, or reopen a downstream
decision. Re-observation establishes whether rendered evidence was retained,
superseded, or expanded; the authorized semantic owner adjudicates claim
change, and the downstream owner decides whether exact reliance is replaced or
reopened.

The candidate consumes the claim-centered evidence-lineage semantics as a
hypothesis under test. It does not establish a second claims database or accept
the parent's permanent placement.

## Consumer boundary

An agent-facing consumer can:

- identify or inspect an exact view state or transition;
- discover current observations, analyses, claims, coverage, and limitations;
- pose an evidence question without prescribing Chrome, DOM, screenshot, or
  provider mechanics;
- request missing evidence and receive a durable attributed result;
- reuse current evidence across several review dimensions;
- bind reliance to an exact observation or claim revision; and
- nominate plausible impact against exact evidence revisions.

These are required affordances, not prescribed method names, triggers, query
order, escalation policy, or storage mechanics. A raw Chrome or provider escape
hatch may remain available without becoming the normal semantic interface.

## First real dogfood evidence boundary

Before permanent placement or a broad production interface is accepted, one
real product view and meaningful interaction branch should exercise the minimum
end to end. The subject must exist independently of the experiment rather than
being authored solely to make the representation succeed.

The dogfood should establish or truthfully fail to establish:

1. one exact view contract with an observable predicate and one attributed
   navigation witness;
2. a base state, one transient interaction state such as hover or focus, and
   one meaningful document or container scroll transition;
3. correlated structural and pixel evidence bound to the same exact states;
4. one cached provider-analysis artifact retaining raw output, normalized
   observations, provenance, usage, and limitations;
5. one explicit accessibility probe, with contrast as a useful candidate when
   applicable to the selected view;
6. exact evidence reuse by separately attributed Truth, Maintainability,
   Explainability, and Aesthetics consumers without averaging their findings;
7. an immutable change event that nominates a bounded evidence unit through
   `may_affect`; and
8. a targeted re-observation that records retained, superseded, expanded, or
   unsupported evidence without silently advancing downstream reliance.

This is a purposive representability exercise, not broad usability evidence,
provider benchmarking, accessibility certification, or proof of architectural
placement. A valid run may produce negative, incomplete, authority-blocked, or
unsupported outcomes. Its criteria and selected subject must be bound before
the produced result is interpreted.

## Explicitly deferred

This candidate does not presently establish:

- automatic discovery of all views, interactions, dependencies, or impacts;
- exhaustive interaction, device, browser, language, accessibility, or human
  coverage;
- a universal screenshot-first, DOM-first, provider-first, or escalation route;
- a required provider, model, storage engine, graph database, cache, daemon,
  service boundary, or MCP method surface;
- provider truth, exact visual grounding, or automatic claim adjudication;
- reviewer selection, four reviewer-role implementations, synthesis,
  acceptance gates, or remediation workflow;
- wake-up, delivery, scheduler, control-plane, concurrency, privacy, retention,
  authentication, or distributed recovery semantics;
- a Work Engine Studio experience or visual replay UI; or
- acceptance, priority, or implementation of the parent evidence-lineage
  candidate.

## Authority and ownership

If later accepted, this boundary owns UI-specific experience identity,
evidence-question and coverage semantics, domain projections over shared
lineage, and correspondence among exact states and evidence kinds. It does not
own:

- Chrome transport and lifecycle;
- product-specific fixtures, view meaning, selectors, safe navigation, and
  cleanup;
- the generic truth, refresh authority, or acceptance of claims;
- provider behavior;
- project design doctrine or the four review judgments;
- the intended human access boundary;
- review coordination and synthesis;
- proposal or implementation decisions; or
- a generated query or client projection as semantic truth.

This formed packet performs none of those actions and grants no implementation
authority.

## Acceptance consequence

If later accepted and implemented, a fresh UI-review consumer can ask a bounded
question about an exact human-facing state, reuse trustworthy current evidence
already acquired for that state, obtain missing evidence once, distinguish
observation from claim and judgment, see unsupported access or interaction
coverage, and react selectively to later changes without reconstructing the
entire browser investigation or treating historical evidence as erased.
