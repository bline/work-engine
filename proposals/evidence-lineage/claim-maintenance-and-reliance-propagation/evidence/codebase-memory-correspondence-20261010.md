# Evidence: Code Correspondence Observation Against Codebase Memory (2026-10-10)

## Evidence status and cutoff

This is exploration evidence for
[`evidence-anchor`](../../../../app-server/docs/architecture/substrates/evidence-anchor.md)
and this proposal's [repository integration](../repository-integration.md).
It is not an implementation decision, provider contract, acceptance result,
or authority grant. The evidence-anchor core remains
`authorization: exploration_only`. The harness is the shadow, report-only
observation against explicit test anchors that
[the idea's §6](../../../../app-server/ideas/pending/evidence-anchor-observation-and-impact-nomination.md#6-the-sequencing-gap-what-can-be-built-now-whats-blocked)
already permits. It neither modifies accepted substrate semantics nor
implementation authorization.

- **Codebase Memory:** `DeusData/codebase-memory-mcp` at `ec4fbd3a`
  (2026-10-10), 3,624 commits after the
  [feasibility evidence](codebase-memory-feasibility.md) cutoff `51c48e2`,
  built from source. It ran with an isolated `HOME`, `CBM_CACHE_DIR` and
  `CBM_RUNTIME_DIR`; the installed 0.10.7 and its sessions were untouched.
- **Harness:** [`codebase-memory-correspondence-20261010/`](codebase-memory-correspondence-20261010/),
  with file hashes, raw `results.json`, and rerun instructions.
- **Interface discipline:** observations use only supported CLI tools
  (`query_graph`, `get_code_snippet`, `check_index_coverage`,
  `detect_changes`, `index_repository`, `delete_project`). Direct SQLite reads
  served only as a labelled oracle for judging those supported signals.
- **Fixtures:** a fresh Git repository per experiment containing small Python
  modules and Markdown documents, changed through uncommitted working-tree
  transitions. Canonical claims were stood in by a revisioned JSON file
  outside the repository.
- **Result:** 27 cases, all as predicted; 15 recorded findings, 6 negative.
  An earlier same-day manual probe (T1–T6 below) motivated the harness.

## Method

The observer records raw facts first: the anchored path's coverage and
freshness, exact-match hits, body digests, and successor candidates. A
separate step maps them onto the accepted five-state comparator. Where the
five states lose a distinction, the loss is recorded here for open question
#7. No vocabulary was added.

- **Code locator:** `(file path, owner-qualified symbol, node label)`. It
  resolves only when exactly one entity matches all three. There is no
  file-level fallback.
- **Declaration:** binds the entity's qualified name and a body digest. It is
  refused unless exactly one entity matches.
- **Text locator:** `(file, line range, digest)`, read directly from the
  document. It does not depend on graph section identity.
- **Successor discovery:** runs only when the anchor is absent. It lists
  same-kind entities with the same name anywhere, or same-kind entities in the
  declared file, flagging an equal body or an equal body excluding the
  signature line. It never rebinds and never changes the comparator outcome.
- **Consistency:** an observation is accepted only if the supported index
  generation signal and the canonical revision are identical before and after
  its dependent reads. Otherwise it retries, and after three attempts reports
  `unknown`.

## Results (all 27 cases)

| # | Exp | Case | Outcome | Cause recorded |
|---|---|---|---|---|
| 1 | E1 | function anchor | `matches` | same entity, same body |
| 2 | E1 | method anchor (`Account.close`) | `matches` | same entity, same body |
| 3 | E1 | anchor kind the adapter cannot observe | `unsupported` | anchor kind `config_key` |
| 4 | E2 | baseline | `matches` | |
| 5 | E2 | rename, before re-index | `unknown` | index stale for the file |
| 6 | E2 | rename, after re-index | `differs` | absence established; same-file candidate `clamp_value` (equal body excluding signature), not bound |
| 7 | E2 | directory move | `differs` | absence established; candidate `helpers/rounding.py::clamp` (identical body), not bound |
| 8 | E2 | parse damage inside the anchored function | `unknown` | `parse_partial` lines 6–7 reported by coverage |
| 9 | E2 | file newly excluded by `.gitignore` | `unknown` | on disk but not indexed |
| 10 | E3 | anchored function body changed | `differs` | same entity, body changed |
| 11 | E3 | caller of the changed function | `matches` | own body unchanged; reachable only as discovery (`CALLS` hop 1) |
| 12 | E3 | normative doc text anchor edited | `differs` | text changed |
| 13 | E3 | repeated heading, edited section | `differs` | text changed |
| 14 | E3 | repeated heading, other section | `matches` | same text |
| 15 | E4 | re-index between reads | `matches` | accepted on the first attempt: **concurrent change undetected** |
| 16 | E4 | canonical revision changes on every attempt | `unknown` | no consistent snapshot after retries |
| 17 | E4 | stale index (worktree edited, no re-index) | `unknown` | index stale for the file |
| 18 | E4 | same-size edit after an earlier restore | `unknown` | stale mtime record (negative finding 5), not content detection |
| 19 | E4b | fresh baseline | `matches` | |
| 20 | E4b | same-size, same-mtime edit on a fresh index | `differs` | freshness reported `metadata_match`; caught only because the snippet read disk (negative finding 2) |
| 21 | E5 | rebuild from the same canonical state | equal | 4 anchors gave identical outcomes and causes; only generation ids differed |
| 22 | E6 | baseline | `matches` | |
| 23 | E6 | entity gone, identical twin at a new path | `differs` | twin listed as candidate, never `matches` |
| 24 | E6 | two identical twins | `differs` | two unranked candidates |
| 25 | E6 | same path and name, different body | `differs` | body changed; a qualified name alone reports "present" |
| 26 | E6 | deleted (state) | `differs` | absence established |
| 27 | E6 | then recreated byte-identical | `matches` | state equivalence; continuity not observable |

Earlier manual probe of the same build on a single fixture:
- **T1:** exact `MENTIONS` from `path::Name`, `path:3-9` and links.
- **T2:** a stale member reference fell back to a file edge.
- **T3:** a directory move dropped 4 edges with no unresolved row.
- **T4:** `detect_changes` with `edge_types=[CALLS,MENTIONS]` reached doc and
  claim sections, with paths.
- **T5:** doc edits produce no seeds.
- **T6:** `file_hashes` stores working-tree content hashes.

On work-engine itself, the build produced 2,334 `MENTIONS` edges and these
unresolved rows: 116 `missing`, 1,538 `ambiguous`, 1,000 `below_bar_tier`.

## Validated behavior

- **No graph write path is needed.** Declared anchors resolve strictly in the
  adapter; canonical claims never enter the graph; rebuilding reproduces the
  observations (E5).
- **Strict resolution.** Missing members, wrong kinds and wrong owners are
  refused at declaration rather than degraded to a file (E1).
- **Conservative identity.** Absence, uncertain coverage and possible
  successors stay distinct (E2). Structurally identical entities elsewhere
  remain candidates and never satisfy the anchor (E6).
- **Five-state comparison.** Every outcome fits `matches` / `differs` /
  `unknown` / `unsupported` (no case produced `failed`). `unknown` was never
  promoted to `differs`.
- **Bidirectional impact.**
  - Code → docs and claims: anchor comparison, plus `detect_changes`
    discovery over `CALLS` and `MENTIONS`.
  - Docs → code: text anchors, plus the adapter's own section mapping.
  - Text anchors stay precise under repeated headings (E3).

## Observed upstream deficiencies

### Snapshot coherence (blocks a production adapter)

The three failures below share one missing contract: **a code observation
must be attributable to one identifiable, coherent, content-verified index
snapshot.** Graph results, snippets and coverage must refer to the same
snapshot rather than independently sampled filesystem states.

1. **No supported publication identity.** `check_index_coverage`
   `metadata.generation` stayed at the first full index's timestamp across
   incremental re-indexes, while the private `store_meta.mutation_gen` went
   1 → 2 → 3. A re-index between two reads went undetected (case 15).
   `recorded_at` changes per run but has one-second resolution. It was
   distinct here only because each CLI index took about a second.
2. **Snippets mix snapshots.** On a stale index, `get_code_snippet` returned
   current disk bytes alongside line numbers from the index. A same-size,
   same-mtime edit passed freshness, and the verdict was right only because
   the snippet read disk (case 20). A line-shifting edit could read another
   entity's bytes.
3. **Freshness is timestamps, not content** (`mcp.c:6829`). It accepts a
   same-size edit with a restored mtime.
4. **Freshness stays wrong after a successful re-index.** A file restored to
   identical bytes with a new mtime remained `metadata_changed` through two
   re-indexes, so its anchors read `unknown` until the content changes
   (case 18).

A token checked before and after the reads would suffice for concurrency if
publication identity were reliable and every dependent read sat between the
checks. It does not address findings 2–4. The upstream guarantees the
evidence supports asking for:
- a publication identity that changes whenever the published graph changes;
- a content manifest covering the indexed inputs, the relevant configuration,
  and the indexer build;
- graph queries and snippets readable against the same snapshot, or an
  explicit failure when that snapshot is gone;
- content-validated freshness;
- freshness records corrected by a successful re-index.

### Markdown discovery precision (does not affect declared anchors)

5. A `` `path::member` `` reference to a member that does not exist becomes a
   `MENTIONS` edge to the containing **file**, with no record of the requested
   member (`doc_links_md.c:1124`).
6. A reference whose directory no longer exists yields no edge and no
   unresolved row (T3).
7. Section qualified names are flat (`path.Heading-slug`). Repeated headings
   collapse, and their references are attributed to the wrong section or the
   file, even when all sources and generations are correct (E3). This is the
   most consequential discovery defect.
8. `detect_changes` never seeds Sections, so a doc-only edit yields 0 seeds.

## Comparison with evidence-anchor

| Contract | Observed |
|---|---|
| Inv. 1: observation and comparison only | Held. Nothing was written to the graph or the canonical stand-in. |
| Inv. 2: only established `differs` nominates; `unknown` never defaults | Held. Absence counted as `differs` only with clean coverage and freshness. |
| Inv. 4: never compare against an assumed revision | **Not enforceable through the supported interface today** (deficiencies 1–4). It is enforceable through the oracle, which is not an interface to design around. |
| Inv. 5: extensible anchor kinds | `code_structure` and `text` sufficed; an unknown kind gave `unsupported`. |
| §4: "does not ask codebase-memory-mcp to change" | Holds for correspondence. A production adapter does need the snapshot guarantees above from whichever backend it uses. |

**Open question #1 (`CodeStructureAnchor` locator).** A qualified name alone
is insufficient. It reports "present" for a different body at the same path
and name (case 25), and it cannot tell a rename from a deletion. The evidence
supports `(path, owner-qualified symbol, label)` plus a bound body digest,
with renames and moves reported as non-binding successor candidates.

Cases 26–27 separate **state equivalence**, which an anchor observation
answers, from **historical continuity**, which it cannot: deleted and
recreated byte-identical reads as `matches`. Continuity belongs to the
code-change profile and lineage. It should not be folded into anchor
identity through speculative rename tracking.

**Open question #7 (comparator richness).** No case required a sixth state.
The cause carried distinctions consumers would act on differently:
- `unknown` arose from five causes with different remedies: stale index
  (re-index), coverage gap (read source), exclusion (configuration),
  inconsistent snapshot (retry), ambiguous locator (re-declare).
- `differs` covered both "changed in place" and "absent, with successor
  candidates".

The evidence supports keeping the cause as structured observation metadata
beside an unchanged outcome. That matches #7's own decomposition into
availability/coverage versus temporal/correspondence questions. The "resolved
at reduced precision" state proposed during review never arose under strict
resolution. It is an artifact of resolving declared anchors through
`MENTIONS`, which this evidence argues against.

**Open question #2 (trigger model).** On-demand observation sufficed for
correctness. `detect_changes` gives cheap change-driven reach for code edits;
doc edits need the adapter's own section mapping.

## Not yet established

- production snapshot guarantees from any backend;
- watcher- or daemon-driven re-indexes, including sub-second windows;
- large-repository cost;
- languages other than Python and Markdown;
- precision of the 2,334 work-engine `MENTIONS` edges (not sampled);
- false-positive rate of the signature-excluded successor digest;
- text-anchor relocation after insertions above the anchored range.

## Consequences for this proposal

The proposal assumed a bounded `sync_projection` extension. This evidence
indicates that no externally writable projection is needed. Correspondence
belongs to evidence-anchor's adapters. Codebase Memory remains an observation
and discovery substrate. The remaining upstream dependency is snapshot
coherence. This evidence does not itself revise `packet.json`, `decision.json`
or `repository-integration.md`; that placement revision needs the proposal
owner's authority.

## Reopening conditions

Reinspect if:
- Codebase Memory adds or changes snapshot, generation, snippet or freshness
  behavior;
- the doc-link layer changes its fallback or section identity; or
- a production adapter targets a different backend.
