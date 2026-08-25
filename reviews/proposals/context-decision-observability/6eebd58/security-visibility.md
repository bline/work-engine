# Security and Visibility Review

Disposition: do not accept before revision. Durable episode revision:
`ee90352cbb136706806a9431066a660b17001fa4`.

- `SVO-001` (high): the effect mechanism must independently verify an
  authenticated, exact-subject safety and authority attestation. The projection
  cannot mint it.
- `SVO-002` (high): serve authenticated role/episode/purpose-scoped views with
  field attenuation, explicit denial, and non-bearer references.
- `SVO-003` (high): fresh reviewers receive no builder recovery content;
  retained or reconstructed reviewers receive only their own episode and
  explicitly permitted immutable evidence.
- `SVO-004` (high): source labels and hashes are not authentication. Require a
  verifier, principal, subject and generation binding, anti-replay/freshness
  rule, authorized use, and fail-closed verification behavior.
- `SVO-005` (high): expose mechanically checkable snapshot coherence and
  `decision_suitable: false` when required evidence is stale, conflicting,
  cross-generation, denied, missing, or unauthenticated.
- `SVO-006` (high): rehydration entries need owner-authorized semantic classes
  and loading modes. Only governing instruction owners may supply directives;
  durable data and judgments remain non-authoritative evidence.
- `SVO-007` (medium): separate restricted audit provenance from the minimum
  model-facing view; scope identifiers, reduce precision, compute aggregates
  inside visibility boundaries, and constrain live-packet retention and
  correlation.

Exact transport and isolation mechanics remain placement-dependent. The
protected authorization and least-disclosure consequences do not.
