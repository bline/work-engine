---
name: repo-search
description: Retrieve verifiable repository evidence across source, documentation, configuration, tests, skills, packages, and architecture. Use for repository orientation, implementation discovery, caller or dependency tracing, change-impact analysis, structural audits, and exhaustive or negative searches that require coverage and fallback provenance.
---

# Repository Search

Retrieve evidence; leave architectural and product judgment to the caller. Keep
observations, relationships, coverage, inference, fallbacks, and limitations
distinguishable. Never turn a candidate match into proof.

## Choose the evidence tier

- **Scout:** Find a likely positive answer quickly. Mark it provisional. Do not
  make absence, exhaustive, dead-code, or complete-impact claims.
- **Verify:** Default. Confirm material claims with exact source or structural
  relationships, exhaust relevant result pages, and check coverage for every
  relied-on path.
- **Auditor:** Verify a bounded scope against a recorded index generation. Cover
  all relevant pages, both relationship directions when material, and every
  reported gap with direct source. State unresolved limits explicitly.

## Express repository intent

Select the durable operation before selecting a backend:

- `orient`: identify packages, boundaries, entry points, languages, and likely
  ownership areas.
- `find`: locate definitions, implementations, literals, documents,
  configuration, tests, skills, or packages.
- `trace`: follow callers, callees, imports, dependencies, data flow, or other
  explicit relationships.
- `impact`: identify changed symbols and bounded downstream consumers, with the
  direction and traversal depth visible.
- `audit`: test a bounded structural, exhaustive, negative, unused-code, or
  high-connectivity claim with coverage-aware verification.

Do not expose one backend's command names as these operations. Read
[backend-capabilities.md](references/backend-capabilities.md) when selecting or
combining current capabilities.

## Route the claim

1. Use exact-name or qualified-name lookup for a known symbol.
2. Use lexical or BM25-style search for natural-language implementation
   discovery.
3. Use vector similarity only to discover candidates across vocabulary. Verify
   every material candidate structurally or from source.
4. Use relationship-aware retrieval for callers, callees, imports,
   dependencies, data flow, and impact. Do not infer these from text matches.
5. Use literal/direct-source search for prose, configuration, fixtures, skill
   instructions, generated text, and content the structural index does not
   represent reliably.
6. For negative or exhaustive claims, bound the scope, consume every relevant
   page, inspect coverage for both returned paths and the searched scope, and
   read every reported missed range or excluded target directly.
7. Fall back to targeted source when the index is unavailable, stale, partial,
   ambiguous, or incapable of establishing the claim. Record why.

Prefer the smallest set of capabilities that can establish the requested
claim. Additional retrieval without credible decision value is noise.

## Return an evidence packet

Report:

- intent operation and evidence tier;
- observed evidence, with repository-relative paths and exact symbols,
  relationships, or source locations when available;
- evidence mode for each material claim, such as indexed structure,
  direct-source observation, or vector candidate discovery;
- index/project identity, generation or freshness state, and query pagination;
- coverage result for every relied-on path and relevant negative/exhaustive
  scope;
- fallback transitions and reasons;
- limitations, unresolved ambiguity, and what would be required to strengthen
  the result.

Keep inference in a separately labeled section if the caller asks for it.
Never claim that clean coverage proves completeness, that static structure
proves runtime behavior, or that a similarity score proves correspondence.

## Preserve role boundaries

Repository retrieval does not select architecture, approve a placement
certificate, validate runtime behavior, or perform adversarial review. A fresh
reviewer may consume the evidence packet, but review-provider identity and
freshness remain separate provenance from the retrieval provider.
