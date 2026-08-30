# Evidence: Codebase Memory Projection Feasibility

## Evidence status and cutoff

This is repository-observation evidence used during proposal formation. It is
not an implementation decision, provider contract, acceptance result, or
authority grant.

- Repository: `DeusData/codebase-memory-mcp`, inspected from the local clone at
  `/home/bline/code/codebase-memory-mcp`
- Commit: `51c48e2093e9ad6c375b396e9624a0135cfb8351`
- Branch at inspection: `main`
- Worktree at inspection: clean
- Evidence tier: Codebase Memory Verify tier with direct-source fallback
- Index result: 25,215 nodes and 132,316 edges in full mode
- Coverage result: no skipped source files; 72 parse-partial files; every cited
  path checked and every reported missed range read directly

These observations describe that exact checkout. Later Codebase Memory changes
may make them stale.

## Repository observations

### Generic physical graph model

The SQLite store represents node labels, edge types, and properties as data.
Nodes contain a string label, qualified name, source location, and JSON
properties. Edges contain string type and JSON properties. The store therefore
can physically represent claim-projection labels and provenance-bearing typed
relationships without introducing another graph database.

Relevant source subjects:

- `src/store/store.c:232-328` — graph and metadata schema
- `src/store/store.c:1786-1817` — node upsert
- `src/store/store.c:2162-2192` — edge insertion
- `src/graph_buffer/graph_buffer.c:643-767` — in-memory node upsert
- `src/graph_buffer/graph_buffer.c:1083-1125` — in-memory edge insertion

### Current identity behavior

Stored node uniqueness is `(project, qualified_name)`. In-memory identity is
also keyed by qualified name. Code qualified names are derived from project,
path, and symbol name. An unchanged entity can retain identity across ordinary
content edits, but movement or rename normally changes the qualified name and
is not historical identity reconciliation.

Relevant source subjects:

- `src/store/store.c:247-278`
- `src/pipeline/fqn.c:113-148`
- `src/graph_buffer/graph_buffer.c:643-767`

Database integer IDs are storage identities. The incremental delta path
relinks captured edges by qualified name when both endpoints still resolve;
it does not adjudicate that two different qualified names are the same
historical entity.

### Documentation anchors

Markdown headings are extracted as `Section` nodes. In the inspected graph,
all 447 Section nodes had one incoming `DEFINES` relationship and no outgoing
relationship. Codebase Memory therefore exposes useful documentation anchors
but does not currently connect them semantically to code or claims.

Relevant source subject: `internal/cbm/extract_defs.c:3945-3971`.

### Change and incremental machinery

`detect_changes` combines committed changes against a merge base, unstaged
tracked changes, and staged or untracked files. It maps changed hunks to current
indexed symbols and computes a graph blast radius. Its default multi-source
traversal follows `CALLS`; custom claim relationships do not automatically
enter that result.

The watcher detects HEAD or dirty-state changes at repository granularity and
invokes indexing. Production incremental indexing compares semantic manifests,
attempts conservative closure repair for supported changes, and falls back to
a full rebuild when uncertainty prevents a safe partial repair. File-scoped
node deletion and edge cascade machinery exists inside that process.

Relevant source subjects:

- `src/mcp/mcp.c:10602-11100`
- `src/store/store.c:4907-5028`
- `src/watcher/watcher.c:1227-1274`
- `src/watcher/watcher.c:1348-1455`
- `src/pipeline/pipeline_incremental.c:1640-1902`
- `src/pipeline/pipeline_incremental.c:2354-2863`
- `src/graph_buffer/graph_buffer.c:857-916`

### Publication and preservation

A full publication dumps a fresh in-memory graph into a staging database and
atomically publishes the completed generation. Arbitrary records inserted into
the prior live graph are not automatically included. ADR content is explicitly
captured and restored, demonstrating an existing special-state preservation
pattern but not generic projection preservation.

Relevant source subjects:

- `src/pipeline/pipeline.c:1351-1382`
- `src/pipeline/pipeline.c:1695-1781`
- `src/graph_buffer/graph_buffer.c:1762-1830`
- `src/pipeline/pipeline_delta.c:451-563`

### Public mutation surface

The MCP tool registry contains structural queries, indexing, change detection,
ADR management, and `ingest_traces`. It does not expose generic node or edge
ingestion. At the inspected commit, `ingest_traces` counts accepted items and
states that runtime edge creation is not implemented.

Relevant source subjects:

- `src/mcp/mcp.c:367-703`
- `src/mcp/mcp.c:923-945`
- `src/mcp/mcp.c:11378-11410`
- `src/mcp/mcp.c:11414-11475`

## Formation inference

The observations support the following revisable placement judgment:

- another graph database is not presently necessary;
- Codebase Memory can probably serve as the default current-world projection
  and traversal provider;
- a bounded projection-synchronization and publication extension is required;
- the code-change profile remains necessary for immutable before/after identity
  reconciliation; and
- canonical semantic state must remain outside the replaceable graph.

This inference is not proof that the extension is correct or small in effort.
Its correctness contract must preserve owner isolation, exact-generation
binding, atomic publication completeness, explicit unresolved state, and
projection-only authority.

## Reopening conditions

Reinspect the placement if:

- the cited store, identity, publication, or MCP behavior changes materially;
- owner-aware projection isolation requires invasive changes that exceed a
  bounded extension;
- exact dirty-worktree generation identity cannot be exposed truthfully;
- projections cannot be evaluated before atomic publication;
- unresolved references cannot remain visible through rebuild; or
- a second current-world provider supplies a materially stronger correctness
  boundary without creating a second canonical semantic registry.
