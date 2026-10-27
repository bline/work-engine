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

## Explicit preservation mode

Requests may add exactly one closed `preservation` block:

- `mode`: `preserve_unrelated_v1`;
- `worktree`: canonical absolute repository root, equal to `repository`;
- `expected_index_sha256`: exact approved real-index bytes;
- `accepted_receipt`: external `{path,sha256}` for the existing checkpoint
  owner's accepted lifecycle receipt, matching repository/run/slice and subject;
- `approved_delta`: complete parent-to-accepted-tree list of
  `{path,action,before,after}`; entries are null or `{mode,oid}`, regular
  Git files only (100644/100755), with include/delete actions;
- `manifest`: external `{path,sha256}` for a schema1 preservation snapshot
  containing worktree, ordinary index_entries (mode/OID/tag), and nonignored
  tracked/untracked working paths (kind/mode/byte digest or symlink target,
  including missing paths). An untracked nested Git repository, which Git
  reports as a slash-terminated directory, appears once under its normalized
  root path with `kind: nested_repository`, root mode/device/inode, and a
  `git_marker` entry for its `.git` file or directory. That marker binds
  mode/device/inode and, for a file, its byte digest. This is an opaque
  boundary claim: nested working files, index and Git metadata contents are
  outside this snapshot, are never imported, and are not claimed unchanged.
  A nested boundary overlapping an approved or tracked path refuses. The
  read-only `preservation_snapshot` helper prepares this projection, not
  approval;
- `attempt_directory`: canonical external directory that does not yet exist.

Authority references cannot live in the attempt directory. This block cannot
coexist with operational exclusions. Complete request approval precedes create.
Approved-path partial staging conflicts refuse; unrelated partial staging
survives. Unmerged/split/sparse/extended-flag indexes, approved symlink/submodule
paths, directory/file collisions and content normalization refuse rather than
silently change meaning. Ignored payload is neither imported nor measured;
the restricted write footprint does not touch it. Nested Git boundaries are
also excluded from that write footprint. A missing, symlinked, or otherwise
unsupported `.git` marker refuses instead of turning an ordinary directory
into an opaque preservation claim.

Preservation receipts use schema2: all receipt1 fields plus `request_binding`
and `evidence`. Every state binds the whole validated request as
`{schema_version:1,canonical_contract:"completion-offer-request-v1",request,sha256}`.
The codec is sorted-key compact JSON, ensure_ascii=False, UTF-8, matching the
existing offer owner; historical proposal/receipt1 digest codecs do not change.

Non-created evidence is null. Created evidence is exactly
`{prepared,post,publication,index_transition,preservation}`; phase references
are external `{path,sha256}`, post may be null after interruption. Publication
is verified for the exact prepared commit on the approved attached branch,
possibly an ancestor of its later tip. Index transition and preservation are
separate verified/failed/unestablished outcomes; verified requires matching
durable post evidence. Created does not mean blanket success.

The unique attempt retains original.index, proposed.index, immutable
prepared.json (complete request, exact commit, index digests/projections,
snapshot and configuration digest), followed by post.json and result.json on
successful verification. Exclusive creation and fsync bind phases. Branch CAS
and atomic index replacement are not one transaction. An interrupted attempt
cannot be reused; original bytes and any incomplete owned lock remain for a
separate recovery decision. Read-only reconciliation follows the exact prepared
commit rather than guessing the tip; it repairs nothing.

`validate_receipt_shape` owns the pure supported-shape contract consumed by
terminal projections. `validate_receipt` also reestablishes Git objects,
accepted lifecycle, full delta and immutable evidence consistency. Actor records
are not independent attestation. The offer owner separately compares its whole
approved request with the result binding at transition and fresh read/reconcile;
self-consistency of a different request is insufficient.

Historical strict request/receipt1 remains readable without invented bindings.
Preservation requires receipt2 and cannot use offer1 to bypass human-decision
ownership. The legacy Node completion-publication route rejects this new block;
no host/runtime integration is provided.
