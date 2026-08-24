# UI Experience Evidence and Visual Claims

## Status

Active exploratory idea. It is the authorized idea source for the formed
candidate in
[`proposals/ui-experience-evidence`](../proposals/ui-experience-evidence/family.md).
The proposal owns candidate meaning; this document preserves the broader
unresolved design space and does not authorize implementation.

## Problem

Material UI review needs evidence about a human experience, not merely a URL,
a static layout, a DOM dump, or an isolated screenshot. The relevant view may
exist only when product data, application state, a particular surface, and a
sequence of interactions align. Its behavior may change under hover, focus,
keyboard use, scrolling, viewport changes, overlays, motion, or alternate
human access contexts.

If every reviewer independently operates Chrome, gathers DOM facts, captures
screenshots, invokes a vision provider, and reconstructs the same state, review
cost and context grow with the panel rather than with the evidence. The results
are also difficult to share, invalidate selectively, replay, or distinguish
from the later claims and judgments they support.

## Desired and invalid outcomes

### Desired outcomes

- An exact rendered experience and the conditions that identify it can be
  addressed durably.
- A fresh consumer can distinguish the view itself from one demonstrated route
  used to reach it.
- Structural, pixel, temporal, interaction, and accessibility evidence captured
  for the same state can be correlated without being flattened into one kind.
- Expensive provider analysis can be cached, inspected, shared, and qualified
  against its exact source artifact and analysis contract.
- Several reviewers can rely on exact evidence and claim revisions without
  repeating acquisition or silently following a later revision.
- Repository or product changes can nominate only the affected evidence units
  for refresh; historical observations remain truthful about their original
  evidence world.
- The system can say which access contexts, interactions, and surfaces were
  observed and which remain unsupported.
- A future human-facing client can replay states, transitions, observations,
  claim revisions, and review consequences in time and interaction order.

### Invalid outcomes

- A URL, selector, screenshot filename, or preferred navigation sequence becomes
  the unexamined identity of a semantic view.
- DOM geometry is treated as sufficient evidence for perceptual judgment, or a
  screenshot is treated as sufficient evidence for exact semantics and
  interaction behavior.
- A vision provider's prose becomes an accepted claim merely because it was
  cached or emitted in structured form.
- Reviewers repeatedly pay to rediscover evidence already current for the same
  state and question.
- A changed implementation retroactively invalidates an honest historical
  capture instead of changing its applicability to current work.
- One mechanical change marks an entire visual tour stale when only a bounded
  state, transition, region, access context, or derived observation may be
  affected.
- Missing hover, focus, scrolling, narrow-viewport, keyboard, or accessibility
  evidence is silently projected as successful coverage.
- Persistence, repeated provider agreement, or reuse inflates the authority of
  an observation or review judgment.

## Core distinctions

### View contract and navigation witness

A **view contract** identifies what must be observably true for a named view to
exist. It can bind an application and surface, product-data or actor context,
state predicates, required and absent semantic anchors, and relevant access
conditions. A URL may contribute to entry context but is not universally the
view identity.

A **navigation witness** is attributed evidence that one starting condition and
one route reached a particular view-contract revision. Several witnesses may
reach the same view. A witness may record selectors or mechanisms used during
that run without making them the durable semantic identity. Route, sequence,
or trigger becomes part of the view contract only when causality makes that
exact experience material.

Semantic anchors such as role, accessible name, product-owned identity, or
domain meaning are preferable view predicates. Their resolution to concrete
nodes remains evidence with its own provenance and lifetime.

### Experience trace

An experience trace is a graph of exact states and attributed transitions. A
state may include:

- application, surface, data, actor, and feature context;
- viewport, theme, device, input modality, and relevant environment;
- document and container scroll positions;
- pointer target and hover chain;
- keyboard focus and focus visibility;
- selected, expanded, pressed, disabled, error, and overlay conditions; and
- relevant temporal or motion conditions.

Transitions identify the interaction, target, modality, prior state, resulting
state, observed outcome, and limitations. Scrolling is both an interaction edge
and part of the resulting state. The graph may branch; it is not required to be
one scripted tour.

### Observation, claim, and judgment

- An **observation** records what a particular producer captured about an exact
  state, transition, region, or element.
- A **claim** is a revisioned proposition supported, qualified, or contested by
  observations.
- A **review judgment** applies an attributed design lens to relevant claims and
  evidence for a named decision scope.

Observation producers do not acquire claim-refresh or review authority. UI
reviewers may publish findings or nominate candidate claims within their role,
but an owning evidence or research authority adjudicates shared claim revisions
and a downstream authority decides proposal or implementation consequences.

## Evidence kinds and sufficiency

The interface should preserve correspondence among:

- structural evidence such as DOM, accessibility tree, computed style,
  geometry, relationships, and scroll state;
- pixel evidence such as viewport images, bounded regions, and tiled surfaces;
- temporal evidence such as before/after states or bounded frame sequences;
- interaction evidence such as pointer, keyboard, scroll, drag, input, and
  resulting-state observations; and
- product-mechanism evidence explaining the underlying state, action,
  consequence, uncertainty, reversibility, and ownership.

Structural evidence supports exact machine-observable facts. Pixel evidence
supports human-perceptual facts. A judgment connecting implementation structure
to perceived experience normally needs both. Absence of sufficient evidence is
an explicit result rather than an automatic pass or a fabricated conclusion.

Capturing an artifact and placing it into a model context are different costs.
Large artifacts can remain under stable identities while reviewers consume
compact manifests, hashes, diffs, derived observations, thumbnails, or crops
and request higher-fidelity evidence when their judgment requires it. This is
an available composition surface, not a mandatory acquisition sequence.

## Cached provider analysis

A provider vision result is a derived evidence artifact, not the screenshot and
not an accepted claim. Preserve separately:

- the immutable source artifact or region and its digest;
- the exact view and interaction state;
- the analysis-contract identity and revision;
- producer, provider/model, configuration, prompt digest, timestamp, usage,
  uncertainty, and limitations;
- the raw provider response;
- normalized visual observations; and
- later claims and reviewer judgments that rely on those observations.

A reusable analysis identity must be sensitive to every input capable of
changing its meaning. A cache hit establishes input correspondence, not truth.
Changing the provider or analysis contract may produce another derived revision
from the same pixels; changing the pixels may nominate dependent analyses and
claims for refresh.

## Human access surface and explicit probes

Accessibility requires Truth, Maintainability, Explainability, and Aesthetics
to remain valid across the intended human access surface. Because important
properties are not reliably visible in ordinary screenshots, the evidence
boundary needs explicit, independently attributable probes. Candidate probe
families include:

- accessible name, role, state, relationships, and dynamic announcements;
- keyboard reachability, ordering, focus visibility, and focus obstruction;
- text and non-text contrast across default, hover, focus, selected, disabled,
  and error states;
- color-independent meaning;
- zoom, narrow-width reflow, clipping, and overlap;
- target size, pointer alternatives, and input-modality changes;
- reduced-motion and other sensory adaptations; and
- visible-label correspondence, errors, and instruction association.

Probe applicability follows the established product access boundary. Omission,
unsupported measurement, and limitations remain visible. A deterministic
contrast calculation may combine computed styles and configured governing
criteria; gradients, imagery, opacity, compositing, and other rendered effects
may require pixel evidence. Passing one probe does not establish accessible
experience as a whole.

## Identity, lineage, and selective refresh

Historical evidence remains immutable evidence of its original world. Current
applicability is revisioned separately. Individually addressable units may
include the capture run, journey, view, state, transition, viewport or access
context, scroll region, artifact, provider analysis, observation, claim, and
review reliance.

The existing claim-lineage candidate supplies the relevant semantic pattern:

```text
implementation or other source event
  -> may_affect an exact evidence or claim revision
  -> attributed refresh episode
  -> retained unchanged, superseded, expanded, or otherwise qualified evidence
  -> selective reconsideration by exact downstream reliance
```

Implementation completion may nominate plausible impact using observed changed
files, contracts, symbols, assets, or state behavior. It does not declare a
visual claim false or a review obsolete. Re-observation establishes what
rendered evidence changed; the applicable semantic owner adjudicates claim
causality and the downstream owner decides reopening.

The shared identity, lineage, reliance, and freshness substrate should not be
duplicated as a second claims database. UI experience evidence needs a
domain-specific façade and vocabulary over equivalent generic semantics while
leaving open what the dogfood proves is genuinely shared.

## Candidate consumer interface

An agent-facing interface should let a reviewer identify an exact state or
transition, discover current evidence and claims, inspect provenance and
limitations, ask an evidence question, request missing evidence without naming
a provider route, bind exact-revision reliance, and nominate possible impact.

The interface represents these consequences rather than prescribing method
names, one store, one provider, one screenshot policy, or a fixed escalation
order. Chrome Vision, product adapters, deterministic probes, provider vision,
repository evidence, and future capture mechanisms remain composable producers.

The same underlying surface may later support a human control-plane client that
shows interaction topology, evidence coverage, cached analyses, claim history,
review findings, refresh events, and selective reopening.

## Ownership boundaries

This idea does not own:

- Chrome transport, target lifecycle, or generic browser operations;
- product-specific view predicates, fixtures, recovery, selectors, or safe
  cleanup;
- the generic meaning of claims, semantic refresh, or downstream reliance;
- provider model behavior or truth;
- the four UI design principles or project-specific design doctrine;
- reviewer selection, review synthesis, proposal decisions, implementation
  acceptance, or roadmap priority;
- product, legal, ethical, or strategy decisions about the intended access
  surface; or
- a canonical database, graph store, query index, daemon, or control-plane UI.

## Open questions

- Which view, transition, observation, and analysis identities remain stable
  across a real product revision?
- What evidence is genuinely reusable across all four review dimensions, and
  what remains dimension-specific interpretation?
- Which accessibility probes are universally useful evidence and which require
  product-specific applicability profiles?
- Can one shared evidence-lineage substrate represent claims and experience
  observations without erasing their different authority and revision rules?
- What bounded evidence query contract prevents repeated provider and browser
  work while preserving legitimate reviewer exploration?
- Which artifact and dependency granularity permits useful selective refresh
  without pretending automatic semantic impact discovery is complete?
