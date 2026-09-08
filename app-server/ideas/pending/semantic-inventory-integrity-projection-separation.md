# Idea: Separate Semantic Inventory from Integrity Projection

**Status:** Post-migration proposal input; deterministic attribution hot fix implemented

The skills migration portfolio currently combines authored semantic decisions
with exact-byte SHA-256 bindings in one repository-wide document. A change to
one skill can therefore make the shared inventory stale, and later repair of
that staleness appears to reviewers as an unrelated semantic scope expansion.
This repeatedly consumes builder and reviewer inference on a mechanically
decidable question.

The post-migration design should separate:

- a semantic inventory owning responsibility, destination, consumer,
  compatibility, and authority claims;
- an integrity projection generated from an immutable Git tree;
- preferably per-package projections, or tree-bound receipts, so one package's
  maintenance does not rewrite a shared cross-package artifact;
- generator identity, input-tree identity, and deterministic output identity;
- candidate and review projections that distinguish derived integrity changes
  from authored semantic changes.

The owning invariant is not that hashes remain checked into one JSON file. It
is that a consumer can verify which exact source bytes support the semantic
inventory at a named immutable revision. Accepted candidate, checkpoint,
review, and publication identities remain immutable and must never be silently
regenerated.

During migration, the bounded hot fix compares the accepted baseline tree,
candidate tree, accepted path set, and both inventory revisions. It classifies
each integrity change as:

- `in_scope_derived_update`;
- `inherited_baseline_debt_reconciliation`;
- `out_of_scope_source_change`;
- `stale_candidate_binding`;
- `inventory_or_source_membership_changed`.

It separately detects semantic inventory drift. Only the first two classes are
mechanically attributable; every other class remains an explicit disposition
boundary. This is transitional evidence machinery, not the final storage
architecture, and should preserve ProviderTurnPort, HarnessRuntimePort, and
OperatorProjection as the broader runtime evolves.
