# Usable-request regressions

`codex-reset.json` preserves the token-count information from B2 raw line 933
(ordinal 932), 2026-10-24 01:43:02.784 UTC: zero input/cache/output despite
`total_tokens = 28432`. Identity, ordinal and rate-limit fields are omitted;
tests bind a disposable session and add the compaction marker separately.
Source: private context-usage-audit-20261024/post-compaction-prefix.jsonl.

`claude-usable-requests.json` is the shared Python/Rust conformance table.
The synthetic-zero case reproduces the measurement-bearing fields of Claude
R2 raw line 168 (2026-10-23 15:18:38.416 UTC): `<synthetic>` with all three
input counters and output zero. Test prefix 91,579 represents the historical
real request before the check; the later 99,981 request is not used.
The other cases are generated edge cases, not observed production failures.
All three Claude input counters must be nonnegative u64 integers with a
positive sum representable as u64; cache-only requests remain usable.
Output counters do not measure context and retain their existing optionality.
