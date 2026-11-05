---
name: claim-evidence
description: Publish, validate, discover, resolve, and rely on exact evidence-backed repository facts, proposal-research claims, or revision-bound review findings while preserving authority, limitations, and provenance. Do not infer applicability, freshness maintenance, or workflow outcomes.
---

# Claim Evidence

Use the shared claim-evidence capability when a role needs a durable, exact-revision statement rather than unaddressable prose or an implicit latest result.

The capability owns stable identity, immutable revisions, reference and lineage integrity, validation of admitted authority, reliance mechanics, and rebuildable query projections. The trusted launcher or another separately authorized lifecycle owner owns when grants are admitted. Repository-fact, proposal-research and review roles retain their domain judgments. A visible or discoverable candidate is not thereby true, current, applicable, sufficient, accepted, or authoritative.

## Operations and exact-revision reliance

Choose operations according to the task rather than following a fixed sequence:

- discover with bounded subject, profile, producer, support, sensitivity, or consumer criteria;
- resolve an exact revision when a decision names one;
- inspect lineage and direct or reverse exact-revision reliance;
- publish only with a trusted, verified authority manifest whose profile, actor, scope, and permission admit the requested transition; or
- rebuild or verify the projection when its freshness or completeness matters.

Never silently select the newest revision when several heads or branches may govern a decision. Keep candidate identity, evidence baseline, limitations, unresolved reference status, freshness, completeness, exclusions, and failures together. Record reliance only against an exact claim revision, consumer revision, and decision scope. The named consumer owns the reliance decision; a v1 reliance record is the admitted producer's attributed record of that decision, not proof that the consumer directly attested it or delegated to that producer.

The filesystem adapter for proposal research and review findings uses
`scripts/claim_evidence.py`. Read [its contract](references/claim-evidence-contract.md)
before defining a publication, authority grant, lineage transition, or reliance
through that adapter; `schemas/` defines its closed v1 wire records.

Native reusable repository facts use the Rust `repository-fact-v1` profile,
the out-of-band `work-engine-claims-host`, and startup-selected MCP reads.
[Repository facts](references/repository-facts.md) describes that profile's
capabilities, exact-reference encoding, and narrower operation set. Its native
startup and projection shapes are owned by the Rust implementation; the
filesystem CLI and schemas do not configure a native repository root.

## Authority and lifecycle boundaries

This phase does not nominate change impact, orchestrate refresh, advance canonical support, reopen downstream work, deliver obligations, monitor sources, accept proposals or reviews, or grant authority from possession of this skill.

Publication cannot self-admit a grant; access to the script or skill is insufficient authority. The current filesystem adapter accepts launcher-supplied grants only while initializing a canonical root. The native repository host likewise uses a fixed externally provisioned bootstrap grant set. These are adapter limitations, not a shared requirement that all authority lifecycles freeze grants at root creation. Any other admission lifecycle requires its own trusted owner and must remain causally separate from publication.
