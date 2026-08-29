# Slice-completion commit receipt

Schema version 1 records `state` (`pending`, `declined`, `created`, or `refused`), repository/run/slice identity, the proposal and its digest, expected branch and parent, optional created commit OID, and an optional reason. A `created` receipt requires a commit OID; its reason is normally null but truthfully records a post-publication verification failure if one occurs. `pending` has neither; `declined` and `refused` require a reason and no commit OID.

New proposals use schema version 2. They contain a one-line subject, body,
exact repository-relative path set, accepted checkpoint commit/tree IDs,
task-patch digest, and structured production provenance. Provenance schema
version 1 contains a nonempty producer description and one or more durable
evidence references, each with an open nonempty kind and lowercase SHA-256
digest. The proposal's separate task-patch field remains the authoritative
content binding. Provider/model identity is descriptive rather than authority. Historical
schema-version-1 proposals with `origin: completing_builder` remain readable;
new producers do not use that closed identity contract. This receipt records an
authorized user-history projection; it does not replace the accepted private
checkpoint or its resume authority.

The completing builder remains the preferred producer while its context is
useful. A replacement may reconstruct only when durable accepted-checkpoint,
manifest, and compact semantic evidence supports an accurate proposal, and must
record that production route truthfully. Structural provenance validation does
not prove semantic equivalence; insufficient support requires refusal or
surfaced uncertainty.

Before a terminal consumer persists `created`, the adapter's read-only
verification re-establishes the named repository, proposal digest and
checkpoint binding, commit object, parent, tree, message, publication ref, and
resulting branch state. The expected branch must remain attached and its tip
must either equal the recorded commit or descend from it. A missing expected
branch, a different attached branch, or a divergent branch tip does not prove
the recorded publication and is rejected. The other lifecycle states do not
claim that Git evidence.

New delayed interactions use the supervisor-owned completion-offer lifecycle
rather than persisting `pending` in immutable terminal history. Historical
pending receipts remain shape-valid compatibility evidence.
