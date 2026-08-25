# Claim Evidence Contract

## Ownership

The shared capability owns record identity, closed schemas, immutable revision mechanics, typed lineage integrity, exact-revision reliance, authority-envelope validation, and projection provenance. A configured storage or transport adapter does not own those semantics.

The `proposal-research-v1` profile owns research materiality and support qualification. The `revision-bound-review-finding-v1` profile owns finding identity, severity, episode, and outcome. Their payloads are versioned extensions; shared code must not interpret those fields into proposal or review decisions.

## Publication admission

Every mutation binds a stable operation identity, one exact payload, a named profile, expected predecessor or state, producer actor, and verified authority reference. Repeating the same operation identity and payload is idempotent. Reusing it for different content fails. Profile or permission mismatch, unverifiable authority, and conflicting predecessors fail closed.

Authority admission is causally separate from publication. Publication accepts only an exactly matching grant admitted by a trusted external lifecycle; an operation payload cannot add its own grant. The current filesystem adapter exposes one bootstrap mechanism, `init --authority`, and therefore cannot add grants after root creation. That mechanism and limitation do not define the shared authority lifecycle. Authentication of a launcher, authority to admit or later change grants, and protection of the canonical root belong to their external lifecycle and operating-system boundaries, not to a self-declared role inside the payload.

Stable claim identity is derived from namespace, subject kind, and stable subject ID. Immutable revision identity is the stable claim identity plus the SHA-256 of the canonical closed revision. Corrections, supersessions, derivations, compositions, identity forks, refreshes, retractions, and reliance retirement add records; they do not overwrite history.

The filesystem adapter serializes canonical publication with an exclusive store lock. It reloads and validates current state inside that transaction, applies predecessor and operation-envelope checks, atomically replaces the human-readable store, then rebuilds the projection against the committed store digest. Readers take a shared lock and reject a projection whose canonical digest differs from current state. The lock is adapter machinery, not semantic authority.

## References and projection

Evidence, sensitivity, and authority references name an owner, reference, exact revision, SHA-256 integrity, freshness rule, and one status: `verified`, `unavailable`, `moved_resolvable`, `excluded`, or `integrity_mismatch`. Only a verified authority reference can admit publication. Non-verified evidence may be truthfully recorded, but it remains visibly unresolved and never becomes verified by inclusion.

The canonical store is human-readable. The query projection is deterministic and non-owning. It reports its build version, canonical input digest and watermark, actual content set, freshness, completeness, exclusions, failures, unresolved references, branches, and conflicts. `unavailable` completeness is an error for discovery, not an empty candidate set.

Discovery returns candidates with `applicability: not_assessed`. A named consumer owns the decision whether a candidate supports its bounded work. A v1 reliance mutation records the admitted producer's attributed assertion of that decision against the exact chosen revision, consumer revision, and decision scope. The `consumer` field does not authenticate direct consumer attestation or encode delegation, so grant issuers and downstream consumers must not infer either relationship from the record. A lifecycle that requires machine-verifiable consumer attestation or delegation needs a separately owned contract and record relation.

Query mechanics include claim or revision resolution, discovery by bounded subject/evidence/content/profile criteria, typed predecessor or successor traversal, and direct revision or reverse consumer reliance. Every result carries exact projection identity, freshness, and completeness; resolution also carries the admitted authority envelope and unresolved reference status.

## Closed phase boundary

This contract does not authorize impact nomination, refresh episodes, source watching, support advancement, selective reopening, obligation delivery, proposal acceptance, review acceptance, readiness, or execution transitions. Those consequences require their separately owned contracts.
