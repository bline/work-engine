---
name: work-engine-mcp
description: Expose bounded Work Engine state and production claim-evidence reads to MCP consumers. Do not use it to publish claims, authorize workflow transitions, or infer claim applicability.
---

# Work Engine MCP

Use this repository-local MCP adapter when an external model needs to inspect
already-owned Work Engine state without receiving filesystem or mutation
authority.

The adapter is a consumer projection. It does not own slice state, claim
meaning, transition validity, authority, or completeness. It exposes only:

- the current or an exact retained revision of one explicitly identified
  active-slice attempt;
- bounded, newest-first history for that same exact attempt identity; and
- when the trusted launcher selects one production claim root, bounded
  discovery, exact resolution, lineage traversal, and direct or reverse
  exact-revision reliance from the shared claim-evidence owner.

Callers must supply the full active-slice identity. The adapter does not
discover campaigns or infer which attempt is current. Claim results retain the
claim-evidence projection's exact identity, schema and build versions,
freshness, completeness, actual content set, exclusions, failures, and
unresolved references. Discovery returns candidates with applicability not
assessed; the adapter does not select a newest revision or reinterpret domain
judgment.

The launcher may bind a production root with `--claim-root`. Tool callers
cannot select or change that root. Without that launch binding the production
claim tools are absent. The adapter invokes the claim-evidence read owner and
does not implement filtering, projection enrichment, publication, authority
admission, initialization, or rebuild semantics itself. A claim read must not
create canonical files, projections, locks, or directories, including when it
fails.

Without an episode authority manifest, all tools are read-only. When the server
is launched with `--review-authority-file`, it additionally exposes the narrow
`independent-adversarial-review-episode-v1` transition surface defined by
`skills/independent-review-state/SKILL.md`. That surface is fixed to one review
episode and does not accept arbitrary durable keys or payloads.

For a retained review, provision this authority-bound surface in the initial
review call when the provider supports the MCP transport, so the reviewer can
publish its own attributed result before returning and later resume from the
same episode. A retained Codex reviewer receives the same semantic profile
through its available authority-bound adapter; transport choice does not change
state meaning or evidence class. Do not add mutable episode state to disposable
reconnaissance, placement, diagnosis, or falsification calls.

The launch manifest verifies profile scope, authority provenance, and writer
generation; it does not authenticate the MCP peer as its declared provider.
Caller admission belongs to the trusted launcher, operating-system permissions,
and MCP tool configuration. Review-state tools cannot publish supervisor or
implementation state, accept a slice, revise claims, or mutate repository
content.
