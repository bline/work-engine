# Agent Instruction Review

Applicability: applicable. The formed proposal already contains normative
future-agent contracts. Durable episode revision:
`784920ce9528b93b88947e2bcbd23bde5fc59d4a`.

- `AIR-CDO-001-decision-owner` (medium): split and restate ownership. Wind
  Walker owns the continuation-safety contract; the authorized role/agent owns
  the benefit decision; the projection only composes evidence.
- `AIR-CDO-002-safety-gate-sequence` (medium): safety must precede effectful
  replacement, not observation or benefit analysis. Benefit evidence may never
  prove, relax, or substitute for safety.
- `AIR-CDO-003-provenance-dimensions` (medium): split the exclusive enum into
  orthogonal origin/method, availability/access, freshness/applicability,
  conflict, uncertainty, and ownership dimensions, including referenced facts.
- `AIR-CDO-004-source-ban-overbreadth` (medium): preserve semantic
  non-substitution, but state it by authoritative source contract. Treat current
  source-form rejections as non-exhaustive examples rather than permanent bans.

The no-threshold boundary and consequence-oriented judgment factors should be
retained. The review does not decide loading placement or proposal acceptance.
