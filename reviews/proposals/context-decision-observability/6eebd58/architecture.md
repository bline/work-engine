# Architecture and Ownership Review

Disposition: revise before acceptance. Durable episode revision:
`0803957d7fd3ac90aab0382f1c19b6b23420565d`.

- `ARCH-CTX-001` (high): no durable owner joins the situated decision,
  authorization, replacement transition, old/new generations, manifest, and
  later measurement. Assign the role/workflow owner the attributed decision
  episode and protected consequence; leave execution facts with the runtime.
- `ARCH-CTX-002` (high): subtracting two occupancy observations is not
  post-checkpoint growth after compaction or a counting-basis change. Require a
  compatible measurement epoch and call the value a net occupancy delta unless
  the runtime supplies a monotonic growth counter.
- `ARCH-CTX-003` (high): packet-level time cannot make asynchronously acquired
  fields coherent. Bind material values and derivations to exact source
  observations, revisions, cutoffs, scopes, and applicability.
- `ARCH-CTX-004` (medium, unresolved): a causal dependency on the entire
  role-owned-state proposal overstates the minimal checkpoint interface and
  undermines independent decidability. Make the relationship non-prerequisite
  or identify a smaller accepted interface.
- `ARCH-CTX-005` (medium): one builder vertical establishes feasibility, not
  reusable placement or family ownership. Preserve placement uncertainty and
  define evidence that discriminates host, shared, and role-local seams.
- `ARCH-CTX-006` (medium): availability is not visibility authority. Source
  owners define grants and an access plane enforces their intersection for the
  authenticated consumer, including reviewer and replacement boundaries.

The review was read-only and based on the immutable subject. No host
implementation contract was available, so all placement conclusions remain
prospective.
