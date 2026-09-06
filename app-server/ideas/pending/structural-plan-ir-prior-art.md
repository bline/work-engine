# Structural Plan IR Prior Art

## Status and scope

Research note supporting
[Structural Plan IR for Capability-Aware Multi-Model Execution](structural-plan-ir-for-capability-aware-multi-model-execution.md)
and its relationship to
[Decision-Gated Implementation Compilation](proposal-decision-gated-implementation-compilation.md).

- Evidence cutoff: 2026-09-05
- State: prior-art observations and Work Engine interpretations; not an accepted
  Plan IR design, schema, ownership decision, implementation plan, or migration
  obligation
- Intended use: preserve representation lessons for post-migration proposal
  formation and experimental design

The source systems below solve different problems. Their documented behavior is
evidence; correspondence to Work Engine is an interpretation. No analogy grants
authority, establishes semantic preservation, or makes a candidate structure
canonical.

## Research question

What design patterns do mature program representations use for:

- semantic identity;
- immutable revisions;
- cross-revision correspondence;
- context-free cores and contextual views;
- extensible layering;
- structural verification;
- transformation and lowering;
- loss-aware projection;
- canonical serialization;
- schema evolution; and
- compatibility across producers and consumers?

The purpose is not to select one external representation as a template. It is to
identify which mature pattern, if any, establishes each property that a future
Plan IR may need.

## Summary

| System | Strongest lesson for Plan IR | Important limit |
|---|---|---|
| Tree-sitter | Reused object identity can conservatively prove unchanged structure | Node identity is not durable semantic identity across arbitrary revisions |
| Roslyn red/green trees | Separate immutable context-free structure from contextual navigation | Red wrappers are not executor-specific lowerings |
| SCIP | Use hierarchical semantic locators with explicit package or basis context | A readable locator is not cross-revision semantic lineage |
| Code Property Graph | Layer typed nodes and relationships over a small shared graph | An overlay is not automatically optional, authoritative, or semantically sound |
| MLIR | Use typed extensibility, traits/interfaces, verifiers, and explicit conversion legality | A legal conversion does not by itself prove preservation of Work Engine meaning |
| GumTree | Compute explicit correspondence between structures from different revisions | A structural match is evidence for lineage, not authority to declare sameness |

## Tree-sitter: conservative reuse identity

### Source observation

Tree-sitter incrementally reparses source and may reuse unchanged subtrees. When a
node is reused, its node ID remains stable. An unchanged node near an edit may
nevertheless be recreated with a different ID. The maintainer describes node IDs
as a reliable but conservative indication that a subtree did not change.

Source:
[Tree-sitter discussion 800](https://github.com/tree-sitter/tree-sitter/discussions/800).
A related report demonstrates why callers cannot assume all unchanged nodes will
retain IDs:
[Tree-sitter issue 1969](https://github.com/tree-sitter/tree-sitter/issues/1969).

### Work Engine interpretation

Plan-node identity should not mean one raw identifier that survives arbitrary
plan revisions. A revision-local identifier can identify the exact node received
by an executor. Cross-revision continuity requires a separate, explicit
correspondence judgment.

Tree-sitter is therefore most useful as a counterexample to eternal node IDs and
as evidence for conservative structural reuse. It does not provide semantic
lineage for plan obligations.

## Roslyn: immutable cores and contextual wrappers

### Source observation

Roslyn uses parallel green and red representations. Green nodes are immutable and
do not store parent pointers or absolute positions. Red nodes wrap green nodes
with contextual navigation such as parents and positions. Green structure can be
shared because contextual location does not contaminate it.

Roslyn also exposes `TrackNodes` and `GetCurrentNode(s)` for following selected
syntax nodes through transformations rather than assuming object identity alone
will remain sufficient.

Sources:

- [Roslyn red/green tree design](https://github.com/dotnet/roslyn/blob/main/docs/compilers/Design/Red-Green%20Trees.md)
- [Roslyn `GetCurrentNodes` API](https://learn.microsoft.com/dotnet/api/microsoft.codeanalysis.syntaxnodeextensions.getcurrentnodes)

### Work Engine interpretation

The useful analogy is the ownership split:

```text
immutable, context-independent plan semantics
                    |
          contextual derived views
```

A future Plan IR core could exclude target-model explanation, display ordering,
repository navigation aids, and other projection context. Derived views could add
those elements without becoming independently editable semantic owners.

The analogy must stop there. A Roslyn red node is a contextual view of the same
syntax, whereas a Sol, Luna, supervisor, or reviewer representation may reorder,
elaborate, select a required closure, or introduce derived explanation. Those
representations are better modeled as versioned projections or compiler
lowerings than as literal red nodes.

## SCIP and SemanticDB: semantic addressing

### Source observation

SCIP symbols use a standardized hierarchical representation containing a scheme,
package manager, package name, version, and semantic descriptors such as
namespace, type, method, type parameter, or parameter. The descriptor chain
ordinarily forms a fully qualified name inside the package.

Source:
[SCIP reference](https://github.com/scip-code/scip/blob/main/docs/scip.md).

### Work Engine interpretation

Plan nodes benefit from human- and tool-readable semantic addresses in addition
to exact revision-local identity. For example:

```text
changes/mutation-boundary/token-validation
acceptance/stale-token/rejected
authority/workspace-mutation/coordinator
```

Such an address is a locator, not canonical identity. Renaming, reparenting, or
reclassification may change it without changing the underlying obligation, while
two similar obligations may need distinct identities despite similar addresses.

## Code Property Graphs: layered relations

### Source observation

The Code Property Graph specification defines a directed, edge-labeled,
attributed multigraph. Its schema is organized into layers that may depend on
other layers. Joern uses overlays to augment a shared representation with further
abstraction levels such as data-flow information.

Sources:

- [Code Property Graph specification](https://cpg.joern.io/)
- [Joern Code Property Graph documentation](https://docs.joern.io/code-property-graph/)

### Work Engine interpretation

Plan IR need not place every relation into every node. A small semantic nucleus
can be combined with named relationship layers. Each derived layer should bind
its producer, source revision, derivation version, limitations, and authority
ceiling.

Layering must not imply optionality. Authority, provenance, mutation boundaries,
and other load-bearing distinctions cannot disappear merely because they are
represented outside the core node record. A future plan profile must declare the
closed set of required layers and fail when that closure is unavailable.

## MLIR: typed extensibility, verification, and lowering

### Source observation

MLIR represents extensible typed operations organized into dialects. Traits and
interfaces let verifiers and transformations reason about shared properties
without hard-coding every operation. Dialect conversion defines legal and illegal
forms, rewrite patterns, and type conversions for lowering between abstraction
levels.

MLIR's bytecode format is versioned. Its compatibility guarantees assume stable
dialects unless a dialect supplies its own versioning and upgrade behavior. This
separates container-format compatibility from evolution of the semantics encoded
by an individual dialect.

Sources:

- [MLIR language reference](https://mlir.llvm.org/docs/LangRef/)
- [MLIR interfaces](https://mlir.llvm.org/docs/Interfaces/)
- [MLIR traits](https://mlir.llvm.org/docs/Traits/)
- [MLIR dialect conversion](https://mlir.llvm.org/docs/DialectConversion/)
- [MLIR bytecode format](https://mlir.llvm.org/docs/BytecodeFormat/)

### Work Engine interpretation

MLIR is the strongest precedent for separating:

- the typed semantic vocabulary;
- generic interfaces shared across node kinds;
- mechanically checkable structural validity;
- target-specific transformations; and
- legal target representations.

It does not establish that a lowering preserves Work Engine semantics. A Plan IR
projection pass must have an explicit preservation contract and verification
evidence. Conversion machinery may change representation but cannot acquire
authority to revise objectives, decisions, invariants, or accepted boundaries.

The bytecode distinction also applies directly: parsing an older Plan IR document
is different from understanding the older schema's meaning well enough to execute
it. Schema-version upgrades need explicit owners and must not silently reinterpret
historical plans.

## GumTree: correspondence across revisions

### Source observation

GumTree is a syntax-aware differencing system. It constructs mappings between two
trees and emits syntax-aligned edit actions, including updates and subtree moves,
rather than reducing every change to line insertion and deletion.

Sources:

- [GumTree project](https://github.com/GumTreeDiff/gumtree)
- [Fine-grained and Accurate Source Code Differencing](https://doi.org/10.1145/2642937.2642982)

### Work Engine interpretation

A successor Plan IR should record explicit correspondence to predecessor nodes
instead of pretending that raw node identity survives. Structural matching may
propose correspondence when plans are revised, including possible moves or
renames.

The match remains evidence. It cannot decide that two obligations retain the
same meaning, authority, applicability, or acceptance state. That determination
belongs to the authorized plan-revision workflow and may remain unresolved.

## Candidate identity and lineage model

The research supports four distinct concerns, but they should not all be encoded
as permanent IDs on every node.

```text
plan_revision
    Immutable identity of one exact Plan IR revision.

revision_node_id
    Immutable node identity inside that revision. The effective exact
    identity is (plan_revision, revision_node_id).

semantic_address
    Human- and tool-readable locator derived from type and structure.
    It is not authoritative identity and may change.

lineage_correspondence
    An explicit relationship between exact nodes in different revisions,
    with producer, basis, rationale, confidence, and authority visible.
```

The exact implementation basis should ordinarily be bound at the plan-revision
level. Individual nodes may cite narrower basis components or an explicit
applicability override when necessary.

Example:

```yaml
plan_revision: sha256:...
implementation_basis: sha256:...

nodes:
  - revision_node_id: change-23
    semantic_address: changes/mutation-boundary/token-validation
    kind: change
    statement: add fencing-token validation at the mutation boundary

lineage:
  - predecessor:
      plan_revision: sha256:...
      revision_node_id: change-19
    successor:
      plan_revision: sha256:...
      revision_node_id: change-23
    relation: continues
    producer: ...
    basis: ...
    rationale: ...
    confidence: ...
    authority: ...
```

The initial schema need not support every possible lineage form, but the model
should not make one-to-one `supersedes` correspondence the only form that can
ever be represented. Later needs may include split, merge, replacement,
reclassification, and unresolved correspondence.

## Candidate layering model

The prior art suggests a small nucleus with required and derived layers rather
than one flat field taxonomy.

```text
SEMANTIC NUCLEUS
  objective
  consequence
  decision
  invariant
  change
  acceptance

REQUIRED CONTRACT LAYERS
  ownership and authority
  provenance and implementation basis
  mutation boundary
  delegation and open judgment
  forbidden consequence and escalation

DERIVED RELATIONSHIP LAYERS
  dependency and required consequence
  execution ordering
  verification coverage
  evidence correspondence
  cross-revision lineage candidates

PROJECTIONS
  builder capability profile
  supervisor
  reviewer
  reconciliation and progress
```

This remains a research shape, not a schema decision. In particular, some
relations may prove load-bearing enough to belong in the semantic nucleus, and
some proposed core categories may be better represented as attributed edges.

## Projection identity and fidelity

Executor and workflow projections require their own exact identity. A projection
should bind at least:

```yaml
projection:
  source_plan_revision: sha256:...
  required_layer_revisions: [...]
  renderer_id: ...
  renderer_version: ...
  target_capability_profile: ...
  included_closure_digest: sha256:...
  declared_omissions: [...]
  projection_digest: sha256:...
```

This distinguishes:

- which exact canonical plan was projected;
- which renderer and capability assumptions were used;
- which required semantic closure reached the consumer; and
- which exact bytes or normalized structure the consumer received.

A projection may add derived explanation, navigation, grouping, and causally
required order. It must not invent semantic obligations, omit required meaning,
or silently convert capability assumptions into authority. Structural validation
can establish closure and internal consistency; behavioral evidence is still
required to evaluate whether a model actually uses the projection successfully.

## Canonical serialization and schema evolution

Before choosing JSON, YAML, another text form, or a custom encoding, research
should answer:

1. What normalized information is hashed: source bytes, a canonical data model,
   individual layers, the closed plan, or several separately identified forms?
2. Are mappings ordered, duplicate keys rejected, numbers and strings normalized,
   and references serialized deterministically?
3. Can unknown extensions be preserved without treating them as understood?
4. Which unknown required fields or layers make a consumer fail closed?
5. How are optional experimental extensions distinguished from load-bearing
   semantics?
6. Can older consumers read newer documents without silently dropping meaning?
7. Can newer consumers reconstruct the exact historical interpretation of older
   documents?
8. Who owns schema upgrades, and what evidence shows that an upgrade preserved
   meaning?
9. How are projection bytes, normalized projection structure, and renderer
   configuration bound together?
10. How are content hashes handled when graph edges, shared nodes, or cyclic
    references are introduced?

Canonical serialization is necessary for reproducible identity, but canonical
bytes do not establish semantic correctness or compatibility.

## Implications for Work Engine

The strongest combined direction is:

```text
accepted implementation basis and authority
                    |
                    v
       immutable typed plan revision
                    |
         verified required closure
                    |
       versioned projection/lowering
                    |
        exact consumer projection
                    |
      execution and attributed evidence
```

This supports the existing decision-gated implementation proposal while keeping
the following owners distinct:

- accepted objectives, proposals, decisions, claims, and authority remain owned
  by their existing sources;
- the compiled implementation contract derives from those sources;
- the Plan IR representation owns neither upstream meaning nor implementation
  acceptance;
- renderer configuration changes representation, not authority;
- campaign services own durable workflow state rather than plan semantics;
- context lifecycle may preserve or reference exact plan revisions but does not
  make context canonical state; and
- empirical model profiles describe demonstrated execution capability rather
  than permanent properties inferred from a model name.

## Unresolved questions

- Is the first Plan IR an implementation-contract representation, a slice-plan
  representation, or a shared substrate with separately closed dialects?
- Which node and edge kinds are necessary to distinguish valid from invalid plan
  states?
- Which relationships must be canonical and which may be rebuildable projections?
- What authority is required to establish or revise lineage correspondence?
- How should split and merge lineage affect downstream acceptance and evidence?
- Is semantic addressing deterministic, author-selected, or both?
- What constitutes lossless projection when explanation can be elaborated but not
  invented?
- How is a capability profile admitted, versioned, invalidated, and kept separate
  from a model slug?
- Can a compact projection safely use references, or must every causally required
  explanation be materialized at the effective loading boundary?
- Which historical accepted slices provide sufficiently strong truth for a
  representation-fidelity and builder-capability pilot?
- What serialization and compatibility format best matches the repository's
  existing canonical JSON, YAML authoring, digest, and migration practices?

## Explicit non-decisions

This research does not decide:

- that Plan IR will be implemented;
- that it will use a graph, tree, MLIR-like dialect, or any particular format;
- that the proposed node categories or layers are complete;
- that a structural match creates semantic lineage;
- that any model requires a particular projection resolution;
- that a compact representation may omit required causal meaning;
- that the current builder, a future planner, or a server service owns the final
  representation;
- that the idea belongs in the active skills-migration campaign; or
- that structural verification can replace semantic review or empirical testing.

## Recommended next research pass

Before schema formation, compare concrete mechanisms for:

- canonical JSON or binary serialization and content-addressed identity;
- unknown-field and extension preservation;
- dialect or schema version negotiation;
- immutable graph and Merkle-DAG storage;
- typed relationship constraints and closed required-layer declarations;
- one-to-one, split, merge, and uncertain lineage representation;
- projection equivalence and semantic-closure testing; and
- capability-profile admission using historical execution evidence.

The output should be a bounded design-options matrix with explicit rejected
alternatives and unresolved authority questions, not a prematurely selected Plan
IR schema.
