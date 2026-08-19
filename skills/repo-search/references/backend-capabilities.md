# Backend capability routing

Use this reference only after choosing an intent operation and evidence tier.
Capability availability is observed runtime state, not a permanent part of the
repository-evidence contract.

## Indexed structural backends

Map available graph capabilities by what they establish:

| Need | Capability class |
| --- | --- |
| Repository orientation | Architecture/package/boundary inventory |
| Exact symbol discovery | Structural name or qualified-name search |
| Natural-language discovery | Lexical/BM25 structural search |
| Cross-vocabulary candidates | Vector similarity search |
| Call/dependency relationships | Directional relationship trace |
| Change impact | Diff-to-symbol and inbound/outbound traversal |
| Exact implementation evidence | Symbol source/snippet retrieval |
| Coverage and freshness | Index identity/status and path/scope coverage |

Discover the current backend interface at runtime. Follow its pagination tokens
or offsets until the relevant result set is complete. Preserve its project
identity and generation in the evidence packet.

Direct Codebase Memory is Work Engine's default indexed provider. Its interface
may evolve; select current capabilities dynamically rather than copying a
fixed tool sequence into the durable skill contract.

## Direct filesystem/source backends

Use targeted file enumeration and literal search for non-code files, exact text,
unindexed paths, and coverage fallback. Prefer fast recursive search tools when
available. Read the smallest ranges that establish the claim, but broaden to the
entire bounded scope for an exhaustive or negative claim.

Filesystem observation is a supported evidence mode, not an embarrassment or
an invisible fallback. Record `index_unavailable`, `coverage_gap`,
`graph_ambiguity`, or `provider_failure` as the transition reason when one
applies.

## Claude-backed retrieval

Claude-backed Codebase Memory and Claude filesystem retrieval remain optional
providers for compatibility, benchmark control, or disposable context. Their
model identity does not change the evidence packet contract. Do not treat a
Claude retrieval process as independent review merely because it uses a
different context; review is a separately selected role and stage.

## Negative evidence checklist

Before reporting absence or exhaustive coverage:

1. State the repository revision and bounded path/symbol scope.
2. Use the search mode appropriate to every relevant artifact class.
3. Consume all relevant result pages.
4. Check index coverage for returned paths and the searched scope.
5. Search reported skipped, partial, excluded, stale, or unknown areas directly.
6. State that best-effort coverage metadata is not proof of completeness.
