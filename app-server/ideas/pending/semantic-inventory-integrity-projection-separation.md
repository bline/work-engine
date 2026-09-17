# Idea: Separate Semantic Inventory from Integrity Projection

**Status:** Post-migration proposal input; deterministic attribution hot fix implemented

```yaml
idea_status:
  architectural_supersession: none
  architectural_supersession_note: "classifySkillsMigrationIntegrity (attribution.mjs) is real evidence the underlying idea is sound, structurally close to substrates/evidence-anchor.md's AnchorObservation shape -- but confirmed 2026-09-16 as a parallel, independently-built instance, not this document's provenance for that substrate. Nothing has been absorbed into any canonical view from this document itself."
  residue: present
  residue_ledger: "Corrected 2026-09-17, per review: two free-form live questions found in prose (outside any formal ledger, which is why the original pass missed them) and now tagged inline -- (1) whether the classifier's comparator should migrate onto substrates/evidence-anchor.md if that substrate is ever built; (2) whether a skills-migration-integrity-v1 domain profile should be formed under evidence-and-claims.md. Both KIND: RESIDUE, confirmed OPEN (each explicitly 'not decided now' in the document's own words, which is itself confirmation the question remains open, not evidence it doesn't exist)."
  backlog: none
  backlog_note: "The described 'bounded hot fix' is already real and shipped (attribution.mjs), so nothing here is pending construction as a staged plan -- no PLAN disposition applies. The two open placement questions above are RESIDUE-kind (architectural placement), not BACKLOG-kind (operative construction), so they do not change this axis."
  audit_scope:
    - keyword-scan: full_document
    - close-read: "full document, extensively investigated this session"
  audit_scope_completeness: complete
  status_as_of: 2026-09-16
```

```yaml
idea_provenance:
  origin: direct_capture
```

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

## Real-code findings (2026-09-16 investigation)

The "hot fix implemented" status line is confirmed directly, not assumed:
`classifySkillsMigrationIntegrity` (`app-server/src/services/skills-migration-
integrity/attribution.mjs:76-151`) resolves a `baselineRevision` and
`candidateRevision` to exact commits, reads
`app-server/migrations/skills/portfolio-inventory-v1.json` (4,914 lines, real
and live) at each, and flattens every package's `source`/`artifacts`/`tests`
entries into one `{path -> sha256}` map. For every path that changed or was
explicitly accepted, it computes the actual git-blob SHA-256 at both
revisions and classifies the path into exactly the five dispositions this
document names, plus `unchanged_current_binding`. It separately compares the
whole inventory with every `sha256` field stripped (`withoutIntegrity`,
line 67) to produce `semanticProjectionEqual` — the document's own "separately
detects semantic inventory drift" is this exact boolean, not a future
aspiration. Verdict is `mechanically_attributed` only when zero violations
remain; the whole result is wrapped in a `receiptDigest` (canonical-JSON
SHA-256 over the frozen body), giving it exactly the immutable-receipt shape
this document requires. Covered by
`app-server/tests/skills-migration-integrity-attribution.test.mjs` (68 lines,
real git fixtures, not mocked).

The single inventory document this idea wants split still exists as one file
today (`portfolio-inventory-v1.json` itself interleaves each package's
authored `source`/`artifacts`/`tests` declarations with their `sha256`
bindings) — the classifier is a derived comparator layered *on top of* the
undivided document, not evidence that the split already happened. The
document's own framing is accurate: this is the bounded hot fix, not the
final storage architecture.

## Relationship to the architecture views (2026-09-16)

**`substrates/evidence-anchor.md` — a structural precedent, not an instance.**
The classifier's core move — bind a declared subject to an exact SHA-256 at
declaration time, later compare it against the actual observed bytes at a
requested revision, and only emit a mechanical outcome when they diverge — is
the same shape that substrate's `AnchorObservation`
(`bound_observation.subject_revision` vs. `observed_observation.observed_
subject`) and its `TextAnchor` kind (`file, range, digest`) describe in
general. This is real, working, independently-built evidence that the
substrate's general shape is sound in practice — but it predates that
substrate being named (2026-09-16, this session) and does not use its
vocabulary or a shared observer, so it is a parallel, domain-specific
instance of the same idea, not an implementation of `EvidenceAnchorObserver`.
`evidence-anchor.md`'s own `implementation: none` status is unaffected; this
finding does not license reclassifying it. **[KIND: RESIDUE] [OPEN — found 2026-09-17 by review, previously recorded as prose without a formal tag: a genuine, if conditional, future-placement question — should this classifier's own comparator migrate onto substrates/evidence-anchor.md if that substrate is ever built?]** If that substrate is ever built,
this classifier's per-path digest comparison is the more natural candidate
to migrate onto it than to keep reinventing — noted here for that future
reconciliation, not decided now.

**`evidence-and-claims.md` — the natural home for "candidate and review
projections," if this is ever accepted.** The three real domain profiles
(`proposal-research-v1`, `revision-bound-review-finding-v1`, `production-
path-v1`) and the two named-but-unbuilt ones (§7 `planning-facts-v1`, §8
`organizational-facts-v1`) are exactly the pattern this idea's own "candidate
and review projections that distinguish derived integrity changes from
authored semantic changes" is asking for — **[KIND: RESIDUE] [OPEN — found 2026-09-17 by review, previously recorded as prose without a formal tag: a genuine placement question -- should a skills-migration-integrity-v1 domain profile be formed under evidence-and-claims.md?]** a `skills-migration-integrity-v1`
domain profile materializing `classifySkillsMigrationIntegrity`'s receipt as
a durable claim would follow the identical shape, with the dimension's own
non-authority boundary (§9) applying unchanged: the claim would materialize
the receipt, never judge which `requires_disposition` violation is
acceptable. Not proposed as an accepted profile here — named only so a future
author does not have to rediscover that the pattern already exists three
times over.

**No dimension, mechanism, or substrate ownership conflict.** This remains
software-engineering domain detail internal to the skills-migration
portfolio, consistent with how the rest of the Candidate Trajectory family
and similar migration-tooling ideas were classified this session — real code,
no architectural residue, nothing here that any of the 13 dimensions, 5
mechanisms, or 2 substrates needs to absorb or correct for.
