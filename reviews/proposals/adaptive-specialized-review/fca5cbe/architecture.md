# Architecture and Placement Review

Role: continuing read-only architectural reviewer. This preserves the prior
reviewer's context; it is not a new fresh-independence claim. Consequence:
diagnostic and advisory only; no architecture mutation, proposal decision,
scope reopening, or implementation authority.

## Subject and provenance

- Current commit: `fca5cbe6efee93bf32354a05585ea9012dce7271`
- Current tree: `5c8f711989cf7d0b75a57e2f33755d7a233b1d86`
- Prior reviewed commit: `5367d9a12ed5ad72fd130e1ca918eadacd129eca`
- Bound packets, verified by SHA-256:
  - `work-engine.claim-centered-evidence-lineage`:
    `38fd6ff11473d2bee029bf263ef79bce1d2bfe8b08730688301847d397a3dad1`
  - `work-engine.revision-bound-review-artifacts`:
    `878d98c6f5fc74b3a08cf0b328aaebe30901d82dd32a9bdd6bba369bf95b1e1b`
  - `work-engine.adaptive-review-panel-coordination`:
    `eaec9664ee062fbfe8b714a6e0be07328106372e5ffbe317dac4e47591e7255b`

Evidence used: the bound subject and proposal paths, this reviewer's prior
durable review, `DESIGN.md`, and `PHILOSOPHY.md`. Tier-2 Codebase Memory
generation `2026-08-22T20:51:52Z` was full and matched current HEAD; all
relied-on paths and both proposal scopes had no recorded coverage issue. Direct
source remained authoritative for proposal prose. Graph inspection confirmed
that the current packet validator mechanically checks closed manifest fields,
references, stable IDs, and relationship-target existence, but does not
establish semantic causality, placement, or authority correctness.

## Prior-finding applicability

1. **Final placement lacks consumer evidence — remains applicable and
   unresolved.** The revision names sharper dogfood and reopening conditions,
   but no research/review consumer pair, proposal revision cycle, or
   implementation-review consumer has yet supplied the discriminating
   evidence. The finding remains a truthful limitation, not an unaddressed
   framing defect.

2. **Semantic ownership and initial adapter placement need sharper separation
   — closed.** The revised artifacts explicitly distinguish a possible shared
   semantic owner from a proposal-local first Git adapter and repeat that
   distinction in the placement certificate and review-profile boundary.

3. **Synthesis has two ownership senses — closed.** The revision now states
   that the review profile owns representation and reference integrity while
   the coordinator owns synthesis judgment and any introduced inference
   remains coordinator-attributed.

## New findings

1. **Medium, inferred — applicability authority is over-concentrated in the
   downstream decision owner.** The shared candidate requires the downstream
   decision owner to judge both applicability and readiness, and that rule
   propagates into the review profile and coordinator. Final readiness,
   reliance on residual uncertainty, and the authority transition clearly
   belong to that owner. But an authorized evaluator, review maintainer, or
   specialist can validly produce an advisory applicability judgment without
   acquiring decision authority. No demonstrated failure mode makes every such
   delegation invalid across all domains.

   **Consequence:** the proposed invariant removes a legitimate degree of
   delegation and risks forcing decision owners to redo semantic evidence
   work. Separate who may produce an applicability assessment from who may
   rely on it and execute the downstream transition.

2. **Low, inferred — semantic freshness is not yet shown to require a distinct
   maintenance owner for every evidence-backed statement.** An immutable
   finding remains truthful about its bound evidence world; later-world use
   primarily raises applicability. Canonical live research claims may
   additionally need freshness maintenance, but the proposal generalizes that
   ownership split before the research-maintenance consumer is formed.

   **Consequence:** implementation could manufacture a durable maintenance
   role and state transition for historical review evidence that only needs
   immutable provenance plus decision-specific applicability. This should
   remain profile-specific unless dogfooding proves the shared need.

No high-severity finding or binding-contract conflict was observed.

## Supported conclusions

- The new claim-centered evidence-lineage candidate is a coherent
  architectural hypothesis: bounded subject, evidence baseline, per-statement
  identity, provenance, limitations, sensitivity, and non-destructive lineage
  plausibly form a reusable core.
- Moving reviewer episodes, truthful omission/failure outcomes, conflicts,
  synthesis references, and review applicability into a review-specific
  profile materially improves ownership clarity.
- The dependency direction is coherent at the semantic level: the review
  profile consumes shared statement semantics, and coordination consumes
  review-specific representation. Neither relation proves delivery order or
  final placement.
- The dependency remains conditional on dogfooding. Review artifacts need the
  named semantics, but current evidence does not yet prove that a separate
  shared owner is preferable to review-local ownership.
- Selection and synthesis remain one defensible candidate capability while
  their context and durable-state lifetimes remain an explicit reopening
  condition.
- Implementation-review reuse remains appropriately prospective and
  non-causal.
- Truthful episode identity, absence-versus-success outcomes, finding-linked
  synthesis, and the distinction between binding doctrine and non-normative
  philosophy are supported improvements.

## Preserved conflicts and uncertainty

- Final shared-versus-domain placement remains unresolved until research and
  review examples expose a genuine common minimum.
- `causal: true` still denotes a semantic dependency, not mandatory acceptance
  or construction order.
- The proposed lineage vocabulary combines revision relations, epistemic
  changes, and decision-specific applicability. Its distinctions are stated,
  but typed targets and allowed profile-specific relations remain to be
  proven.
- External evidence requires a truthful immutable-baseline representation; the
  proposal does not yet establish whether capture, digest, version, or another
  identity is sufficient.
- Which fields are mechanically closed, whether Git-backed reconciliation is
  sufficient, and whether concurrency requires runtime state remain
  unresolved.
- Prospective implementation-review similarity remains evidence to seek, not
  evidence already establishing a universal substrate.

## Limitations

No schema, evidence-lineage validator, exercised research claim,
revised-proposal applicability cycle, implementation-review consumer, or
concurrency behavior was in the subject. Mechanical packet validity proves
only shape and reference integrity. Other specialists' reviews and the prior
synthesis were not used as evidence.

Readiness consequence: `revision_warranted`.

The revision closes the prior ownership-framing defects and substantially
strengthens the proposal family. Before an authority decision, the shared
contract should stop assigning all applicability judgment exclusively to the
downstream decision owner and should keep semantic-freshness maintenance
profile-specific until a real consumer proves it is shared.
