---
name: claim-evidence
description: Publish, validate, discover, resolve, traverse, and rely on exact production evidence-backed statements while preserving authority, limitations, lineage, and projection completeness. Use for proposal-research claims or revision-bound review findings; do not use it to infer applicability, freshness maintenance, proposal decisions, or review outcomes.
---

# Claim Evidence

Use the shared claim-evidence capability when a role needs a durable, exact-revision statement rather than unaddressable prose or an implicit latest result.

The capability owns stable identity, immutable revisions, reference and lineage integrity, validation of admitted authority, reliance mechanics, and rebuildable query projections. The trusted launcher or another separately authorized lifecycle owner owns when grants are admitted. Proposal-research and review roles retain their domain judgments. A visible or discoverable candidate is not thereby true, current, applicable, sufficient, accepted, or authoritative.

## Operations and exact-revision reliance

Choose operations according to the task rather than following a fixed sequence:

- discover with bounded subject, profile, producer, support, sensitivity, or consumer criteria;
- resolve an exact revision when a decision names one;
- inspect lineage and direct or reverse exact-revision reliance;
- publish only with a trusted, verified authority manifest whose profile, actor, scope, and permission admit the requested transition; or
- rebuild or verify the projection when its freshness or completeness matters.

Never silently select the newest revision when several heads or branches may govern a decision. Keep candidate identity, evidence baseline, limitations, unresolved reference status, freshness, completeness, exclusions, and failures together. Record reliance only against an exact claim revision, consumer revision, and decision scope. The named consumer owns the reliance decision; a v1 reliance record is the admitted producer's attributed record of that decision, not proof that the consumer directly attested it or delegated to that producer.

The deterministic mechanics are in `scripts/claim_evidence.py`. Read [the contract](references/claim-evidence-contract.md) before defining a domain publication, authority grant, lineage transition, or reliance. The schemas under `schemas/` define the closed v1 wire records.

## Authority and lifecycle boundaries

This phase does not nominate change impact, orchestrate refresh, advance canonical support, reopen downstream work, deliver obligations, monitor sources, accept proposals or reviews, or grant authority from possession of this skill.

Publication cannot self-admit a grant; access to the script or skill is insufficient authority. The current filesystem adapter accepts launcher-supplied grants only while initializing a canonical root. That bootstrap limitation is adapter behavior, not a shared requirement that all authority lifecycles freeze grants at root creation. Any other admission lifecycle requires its own trusted owner and must remain causally separate from publication.
