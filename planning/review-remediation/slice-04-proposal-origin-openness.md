# Slice 4 — Remove Completing-Builder Identity from Product Validity

## Protected consequence

A commit proposal accurately describes the authorized accepted change and is
bound to its immutable evidence. Useful builder context is retained while it
improves that proposal, but loss or replacement of that context does not make an
otherwise valid proposal structurally impossible.

## Observed doctrine conflict

Runtime instructions require the same completing builder to compose the
proposal, and the durable validator requires `origin: completing_builder`.
Builder continuity is valuable because it avoids reconstruction, but model
identity is a method, not the protected product property. A replacement could
produce an equally accurate proposal from the accepted checkpoint, compact
semantic receipt, and attributed manifest.

This must not be read as permission to keep resume-critical state only in the
builder's context. In this environment, compaction can silently lose or distort
that context. Retaining the completing builder is a useful optimization only
after the proposal-relevant consequences have a durable owner from which a
replacement can reconstruct them.

## Acceptance consequences

- Durable validity depends on proposal content, evidence binding, authority,
  and provenance—not one provider/model context identity.
- The completing builder remains an available and preferred source while its
  context is useful.
- Loss or compaction of that context cannot erase the accepted change's
  proposal-relevant semantics or make recovery depend on transcript replay.
- A reconstructed or replacement context can produce a proposal when sufficient
  durable evidence exists, with truthful origin/provenance.
- The product does not claim equivalent semantic quality when required context
  is unavailable; it may refuse or surface uncertainty.
- Runtime instructions explain the context-value consequence without turning it
  into an unconditional sequence.

## Required vertical proof

Validate proposals with identical authorized content and checkpoint binding but
different truthful production provenance. Acceptance must turn on the protected
semantic and authority properties. A malformed, weakly supported, or
mis-bound replacement proposal must still fail.

## Insufficient substitutes

- Renaming `completing_builder` while retaining a closed origin enum.
- Dropping all origin provenance.
- Always discarding the completing builder before proposal generation.
- Assuming reconstruction is equivalent without evidence.
