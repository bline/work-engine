# Placement: Raw Idea Intake and Claim Assessment

## Probable semantic owner

A dedicated Work Engine planning-layer intake capability owns attributed
interpretation and claim-level reconciliation of exact raw-idea revisions. It
ends before proposal identity is created and before repository cleanup is
applied.

```text
raw idea source
      │ referenced, never rewritten by assessment
      ▼
intake record and assessment
      │ surviving candidate projection
      ▼
proposal former
      │ formed meaning
      ▼
proposal packet
```

The arrows describe semantic consumption, not a mandatory universal runtime
sequence. A user may form an already-bounded proposal directly, and one intake
assessment may produce no proposal or several candidate handoffs.

## Candidate physical placement

A repository-local validated surface separate from cheap raw capture is the
probable first adapter. A top-level `idea-intake/` directory would make this
boundary visible, while an explicit `ideas/intake/` hierarchy could preserve the
same ownership with less top-level structure. Exercise both consequences before
settling the physical layout.

Git owns source and assessment history. The intake record must bind immutable
source identity strongly enough that later movement does not silently change
meaning or leave proposal origins dangling.

## Rejected placements

### Proposal former

Rejected as the semantic owner of raw intake. Formation decides whether
surviving candidate meaning becomes a proposal; it should not thereby own raw
capture, prior assessment history, or cleanup authority. It remains the primary
consumer.

### Proposal packet

Rejected. Intake can truthfully yield no proposal, and source-level claims may
remain unresolved or partly represented by several existing owners. Putting
that history in `packet.json` would make proposal identity own pre-proposal
meaning.

### Claim-lineage subsystem

Rejected as a decided placement. Claim-lineage semantics are a plausible shared
dependency, but the current implementation is bounded dogfood and does not own
intake disposition, source movement, or proposal readiness.

### Ideas directory alone

Insufficient without an explicit validated ownership distinction. Physical
adjacency is acceptable only when raw source and attributed assessment cannot be
mistaken for each other by people or tooling.

## Reopening conditions

Reopen placement when exercised intake shows that the proposed record cannot
survive source movement, that formation must reconstruct material assessment,
that the physical split adds no decision value, or that concurrent/external
consumers require a different durability boundary.
