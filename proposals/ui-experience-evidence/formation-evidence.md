# Formation Evidence: UI Experience Evidence

## Evidence cutoff and modes

Formation used these distinct source classes on 2026-08-23:

- **Human-authorized idea source:** the user authorized durable capture of the
  conversation's UI experience-evidence direction. The resulting canonical
  source is
  [`ideas/ui-experience-evidence.md`](../../ideas/ui-experience-evidence.md).
  Formation authority does not imply proposal acceptance or implementation
  authority.
- **Current Work Engine repository observation:** revision
  `35d2dc68222d6134d976fa7f5c2a9efe4765a1e5`, observed through indexed
  structure and direct source.
- **Site2JSON repository observation:** revision
  `80beea1cc2601f49ac29cb355b0de25920bde000`, observed through indexed
  structure and direct source in `/home/bline/code/site2json`.
- **Model inference:** the proposed ownership boundary, minimum dogfood, and
  remaining placement alternatives are formation judgments subject to review.

The Work Engine graph generation used for current-repository evidence was
`2026-08-23T23:37:04Z` at revision `35d2dc6`. The Site2JSON graph generation was
`2026-08-20T17:15:01Z` and its Git metadata matched revision `80beea1`. Relevant
Site2JSON paths reported no recorded coverage issue; that best-effort signal is
not proof of completeness. Exact prose and bounded source were read directly.

## Current machinery

### Chrome Vision

[`skills/chrome-vision/SKILL.md`](../../skills/chrome-vision/SKILL.md) defines a
generic broker for bounded, provenance-bearing Chrome evidence packets. It
separates artifact payloads from packets, preserves limitations and lifecycle
epochs, and leaves product selectors, target meanings, recovery, and workflow
judgment to project adapters. It recommends document observation for bounded
facts and screenshots when pixels matter. It does not define stable semantic
view identity, experience-trace lineage, cached provider-analysis records,
accessibility-probe profiles, or reviewer-facing shared claims.

### Claim-lineage dogfood

[`ARCHITECTURE.md`](../../ARCHITECTURE.md) records an implemented bounded
claim-lineage dogfood with stable claim identity, immutable revisions, impact
nominations, authorized refresh judgments, exact-revision reliance, typed
edges, and a rebuildable non-owning projection. The parent
[`claim-centered-evidence-lineage`](../evidence-lineage/claim-centered-evidence-lineage/proposal.md)
candidate still defers permanent shared placement and explicitly does not
establish a production API, UI, registry, graph store, shared database, or
automatic monitoring. UI experience evidence can exercise those semantics as a
hypothesis without reporting the parent candidate as accepted.

### Existing UI review direction

[`ideas/ui-review-capability.md`](../../ideas/ui-review-capability.md) separates
rendered evidence, mechanism evidence, project design doctrine, and diagnostic
review judgment. It now records four co-equal lenses and accessibility across
the intended human access surface. It remains a consuming capability and does
not own Chrome Vision, evidence lineage, or implementation decisions.

## Real visual-tour evidence

At the observed Site2JSON revision:

- `scripts/capture-visual-tour.mjs` uses the shared Chrome Vision transport;
- `createTourProvenance` records journey, surface, route, preceding logical
  state, actions since that state, and the complete path;
- `captureState` captures multiple widths plus vertical document and horizontal
  overflow tiles and records their scroll positions;
- the manifest, contact sheet, route documents, and Mermaid graphs provide
  machine-readable and human-readable projections of the same captures; and
- `tests/visual-tour.test.js` verifies deterministic tiling, responsive grouping,
  provenance actions, route/state graphs, and selected workflow journeys.

A direct observation of the ignored local artifact
`tmp/visual-tour/2026-08-18T12-59-35-210Z/manifest.json` found 50 captures across
11 logical states, widths `320`, `480`, and `800`, and popup, options, and side
panel surfaces. Because this artifact is ignored and not durable repository
evidence, those counts are illustrative only and are not a required premise of
the proposal.

The bounded source records clicks, fills, selections, state transitions, and
scroll offsets but does not presently represent hover, focus, pointer,
keyboard, contrast, target-size, or similar transient/accessibility conditions
as first-class trace evidence. A literal search of the capture script, its test,
and its documentation found no hover or focus capture contract. This is a
bounded observation of those files, not a claim that Site2JSON contains no such
behavior elsewhere.

## Formation inference

The visual tour is a product-specific prototype of an experience trace, not a
generic UI evidence owner. The smallest credible next candidate is therefore a
domain-specific façade that preserves view identity, reachability evidence,
multimodal observations, derived-analysis caching, explicit probe coverage,
exact reliance, and selective refresh while retaining the owners above.

Real dogfood should test that inference. Schema validity, durable storage, one
successful provider response, or four agreeing reviewers would not establish
that the boundary is correct.
