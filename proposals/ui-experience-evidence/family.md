# UI Experience Evidence

## Family identity

- Family ID: `work-engine.ui-experience-evidence`
- Origin:
  [`ideas/ui-experience-evidence.md`](../../ideas/ui-experience-evidence.md)
- Related consumer idea:
  [`ideas/ui-review-capability.md`](../../ideas/ui-review-capability.md)
- State: one formed candidate with placement uncertainty; not reviewed,
  evaluated, accepted, prioritized, or authorized for implementation
- Decision owner: user or future explicitly authorized portfolio owner

This family currently contains one independently decidable candidate:

1. [`ui-experience-evidence-interface`](ui-experience-evidence-interface/proposal.md)
   defines a shared, revisioned evidence façade for exact human-facing views,
   interaction traces, multimodal observations, accessibility probes, cached
   provider analysis, claim reliance, and selective refresh.

The related UI-review idea describes four co-equal review lenses, and the
experience-evidence idea describes a possible human visual-replay surface.
Those are consumers and projections, not silently accepted parts of this
candidate. A later formation decision may split provider analysis,
accessibility-probe profiles, reviewer roles, or a production query service if
real evidence shows they have independently decidable consequences.

## Why this is a separate family

Chrome Vision owns generic bounded Chrome observation and interaction packets.
The claim-centered evidence-lineage candidate owns shared epistemic identity,
revision, impact, refresh, and reliance semantics if that placement survives
dogfooding. Review artifacts own reviewer episodes and findings. Product
adapters own the meaning and safe reachability of their own views.

The proposed UI experience boundary composes those owners around a distinct
problem: several consumers need reusable evidence about the same exact rendered
and interactive human experience without independently reconstructing it or
turning browser/provider output into authoritative claims. Keeping this family
separate allows that consequence to be accepted, revised, split, or rejected
without broadening Chrome transport, the generic claim ontology, or UI-review
authority by implication.

## Authority boundary

These are formation artifacts. They do not establish a production interface,
approve a storage or provider strategy, define a product's intended human access
surface, perform UI review, accept the claim-lineage placement hypothesis,
change roadmap priority, or authorize implementation. Durability preserves the
candidate and its provenance; it does not strengthen its truth or authority.
