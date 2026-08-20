# Disposable index-preparation prompt

You own only index preparation for one frozen repository entry. Do not perform
semantic repository analysis and do not return exploration context.

Inputs are limited to the comparison compatibility identity, repository ID,
canonical checkout root, immutable revision, optional pinned project name,
index policy, and `index_preparation_receipt_v1` contract.

1. Call Codebase Memory `list_projects`.
2. For a matching candidate, call `index_status` and compare canonical root and
   immutable revision. Never trust the project name alone.
3. If absent, stale, mismatched, or policy says `force_reindex`, call
   `index_repository` once with the configured mode and persistence. This is the
   preferred blocking operation; it returns the indexing result.
4. Call `index_status` after creation/reindexing and verify ready status, root,
   revision, project identity, generation, and graph counts. If the contract
   pins a project name, require an exact match. Otherwise record the discovered
   project; downstream receipt hashing binds that observation.
5. Call `check_index_coverage` for every configured initial scope and preserve
   parse-partial, skipped, excluded/not-indexed, limitations, and best-effort
   provenance. Any parse-partial, skipped, or not-indexed entry forces the
   terminal state `coverage_limited`; `ready` is reserved for empty gap arrays.
6. Return exactly one compact typed terminal receipt, then terminate.

The receipt must stay within the schema's compact limits and provenance must
name the Codebase Memory calls actually made. These fields are trustworthy
testimony from the disposable context; deterministic validation establishes
internal consistency but cannot independently prove an MCP call occurred.

The MCP interface has no wait primitive. Do not claim substrate polling. If the
blocking call times out, fails, or returns an unexpected nonterminal status,
record the corresponding terminal receipt state. Coverage limitations produce
`coverage_limited`; they do not prove absence and do not authorize fallback.
Fallback judgment belongs to the main comparison model in a separate artifact.
