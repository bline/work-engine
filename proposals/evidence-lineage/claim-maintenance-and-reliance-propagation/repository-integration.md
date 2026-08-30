# Repository Integration: Claim Maintenance and Reliance Propagation

This document elaborates the initial bidirectional code-and-documentation
integration summarized in [`proposal.md`](proposal.md). Canonical claim meaning
and authority remain owned by the proposal narrative and its production
dependency.

## Integration consequence

```text
immutable repository subject + accepted plan
                    ↓
physical and structural change profile
                    ↓
current claim-maintenance projection
                    ↓
bounded related-claim and unmapped candidates
                    ↓
canonical may_affect nomination
                    ↓
authorized refresh judgment
```

The route is symmetric. A code change may nominate documentation, claim
support, tests, schemas, configuration, and consumer reliance. A normative
documentation change may nominate implementation and verification surfaces
that may no longer realize its meaning.

## Planned and actual inputs

The slice lifecycle supplies two distinct inputs rather than one reconstructed
scope:

- the accepted plan references an integrity-bound expected semantic-impact
  artifact, including semantic targets believed relevant, expected
  documentation effect, material uncertainty, and the plan or scope revision;
  and
- the immutable candidate or accepted checkpoint supplies the actual
  attributed manifest, before and after trees, task-patch identity, and
  observed change profile.

Planned-versus-actual comparison is evidence of implementation drift or an
omitted expectation. It does not decide that a claim changed. The full
checkpoint lifecycle receipt or a lossless derived subject must have a durable
owner before a terminal projection discards path attribution or before model
context becomes its only copy.

Mechanical nomination is bound to the immutable review candidate when its
output is needed to detect omitted scope before acceptance. A refresh that is
itself an acceptance condition completes before the slice is accepted. Other
refresh and downstream-reconsideration consequences may begin from the
accepted implementation-completion event and remain durable post-slice
obligations. "Post-slice" timing never permits a required documentation
consequence to be silently deferred past its owning gate.

## Historical structural identity

A replaceable change-profile analyzer derives physical and structural entity
lineage across two immutable repository worlds. Candidate observations include
unchanged identity, rename or move, split, merge, modified structure, addition,
deletion, and unresolved matching for code symbols, tests, schemas,
configuration, and documentation anchors.

Every observation retains analyzer identity, evidence cutoff, coverage,
matching method, confidence or ambiguity, and exact repository subjects. A
current-only graph, raw touched-file list, or line-number comparison is
insufficient evidence for a historical identity delta.

Stable claim identity survives ordinary refactors. Changed support creates
immutable successor revisions; removals retain non-destructive history rather
than silently deleting meaning. Structural matching may nominate possible
impact but cannot decide claim identity, causality, applicability, or
retraction.

## Current-world projection

The projection compiler joins observed or current structural entities to exact
claim revisions, authored content references, sensitivities, and active
reliance. It produces a cacheable projection specification whose loss requires
deterministic recompilation from canonical records, not human reconstruction.

Projected semantic objects point back to their exact canonical record and
revision. Projection mutations never create or revise canonical claims,
authored relationships, nominations, judgments, reliance, or obligations.
Derived nodes and incident edges may be rebuilt without acquiring authority to
replace or tombstone their canonical sources.

The projection can report newly observed behavior or normative documentation
meaning with no mapped claim or relationship as an explicit unmapped
candidate. A clean result means only that no candidate was found within the
declared generation, coverage, exclusions, and traversal bounds.

## Probable Codebase Memory placement

Direct inspection establishes Codebase Memory as the probable default
current-world projection and traversal provider after a bounded extension. Its
store already supports generic labels, edge types, qualified-name identity,
JSON properties, read traversal, staging publication, and conservative
incremental indexing. It does not currently expose generic projection
ingestion or preserve arbitrary externally materialized graph records through
full publication.

The missing boundary is provisionally called `sync_projection`. The name and
wire shape are implementation choices; the protected consequences are not.

### Namespace ownership

A projection owner can replace only its own nodes and edges. It cannot
overwrite parser-produced structural data or another projection. Ownership
must participate structurally in identity or storage isolation; a JSON property
alone cannot prevent collision under the current node and edge uniqueness
rules.

### Generation binding

Synchronization binds at least:

```yaml
expected_index_generation: graphgen:...
repository_subject: commit-or-checkpoint:...
structural_manifest_digest: sha256:...
projection_revision: ...
```

The exact fields may vary, but the binding distinguishes the immutable
repository lineage from the precise structural generation, including indexed
dirty-worktree content. A stale preparation fails visibly rather than
attaching canonical references to the wrong repository world.

### Publication completeness

Projection evaluation occurs in the staging generation before atomic
publication:

```text
build structural graph
→ load compiled projection specifications
→ resolve endpoints against that generation
→ retain resolved and unresolved outcomes
→ validate owner and generation invariants
→ atomically publish the complete evaluated generation
```

"Complete" means evaluation completed truthfully. It does not mean every
endpoint resolved. Readers never observe a newly published structural
generation temporarily missing the projection state that was evaluated for it.

### Truthful unresolved state

An authored relationship whose old qualified-name endpoint no longer resolves
does not silently disappear. The projection retains an unresolved record with
its canonical source, expected endpoint, generation, and reason. The
code-change profile may later supply relocation evidence; only the authorized
claim-maintenance boundary can publish the resulting canonical disposition.

### Projection-only authority

Codebase Memory may materialize claim, claim-revision, relationship, and
derived-impact nodes for traversal. It never authors their canonical meaning.
A projected `MAY_AFFECT` edge is marked as derived or points to an already
canonical nomination; graph reachability alone cannot manufacture nomination.

## Role-facing retrieval

Roles consume this capability through `repo-search`, not provider-specific
Cypher or synchronization behavior. A claim-aware impact request may specify
changed entities, direction, and a relationship profile. Its result includes:

- the bounded related structural and projected entities;
- traversal paths and relationship types;
- exact projection revision and structural generation;
- repository subject and freshness;
- coverage, exclusions, failed inputs, and truncation; and
- unresolved endpoints and unmapped candidates.

`repo-search` owns evidence retrieval, not canonical nomination. Claim
maintenance decides whether retrieved candidates justify a durable
`may_affect` record.

## Reopening conditions

Reconsider Codebase Memory as the probable provider if it cannot preserve
owner isolation, content-bound generation identity, atomic publication
completeness, and explicit unresolved state through a bounded extension.
Reconsider a generic projection-sync capability as merely enabling machinery
if independent consumers establish a separate product consequence for it.
