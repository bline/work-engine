# Requirements revision `hosted-route-qualification-r1`

**Standing: an explicitly versioned requirements revision for one bounded hosted-route
qualification. Not the frozen requirements, not a qualified organization, and no source of an
established condition or a consumable result.**

## Authority

Operator decision, 2026-11-17 (D2 / OQ-1, option D), quoted:

> conditional on a distinct, explicitly versioned requirements revision. Preserve the original
> obligation, semantic scope, and normative requirements. Remove the two realization-class
> constraints only for this bounded hosted-route qualification. Use a new run identity and
> truthful proposition. Keep the original frozen candidate unchanged and its original conditions
> unmet. No qualification outcome establishes a review condition or becomes consumable.

## Derivation

Source: `rust/crates/work-engine-organization/tests/fixtures/frozen/requirements.json`
(unchanged; it remains the frozen world, refused on this route by known realization
incompatibility). Applied, and nothing else:

```text
value = parse(frozen/requirements.json)
value.problemScope.runId = "hosted-route-qualification-20261117"
for condition in value.acceptanceConditions:
    delete condition.realizationConstraints          # provider_family, execution_confinement
    condition.proposition = <route text naming its own boundary>
assert parse(hosted-route-qualification-r1/requirements.json) == value
```

Checked by `tests/revision.rs::revision_is_the_frozen_requirements_with_exactly_the_authorized_changes`
(semantic JSON equality, plus an exact list of differing JSON paths), and
`revision_compiles_with_the_unchanged_compiler` (`derive`, profile `fixed-slice-review-v1`, one
candidate, nothing unresolved).

Unchanged, by the operator's text: the obligation (`rp3-initial-implementation-review`, result
contract `implementation-review` v1, `scope.digest` = `5212141d…c51`, the frozen subject's
`patchIdentity`), plan lineage and slice number, every normative value (including
`mutation: none`, `capabilities: []`, `evidenceCustody: execution_evidence_custody`,
`observerClass: independent_execution_evidence`), consumption boundaries, consumer classes,
covered state, `unestablishedRoute: hold-consumption-for-owner-disposition`, acceptor, required
authorities and organization policy. The acceptor and authority principal still name
`operator-production-qualification-20261107`; the revision does not rename them.

The propositions describe this route truthfully: route qualification only; the hosted Pi analysis
profile `work-engine.pi-analysis-hosted.v1`; the requested model `gpt-6-luna` via provider
`openai-codex`; Host-served read-only snapshot tools `snapshot_list` and `snapshot_read`; not a
guarded Claude reviewer; no condition established; not consumable.

## What this revision does not do

- It does not change the frozen requirements or their candidate, and does not meet their
  conditions.
- Removing the two constraints removes their `realization:*:provider_family` and
  `realization:*:execution_confinement` checks only. Every other preparation blocker of this world
  stays undischarged (the adapter discharges nothing), and the hosted claim names two tools, so
  mutation and capabilities stay `unestablished` (`work-engine-review-preparation/src/realization.rs:134-138`, `:166-176`).
- The normative values it keeps (fresh continuity, no mutation, verified artifacts, execution
  evidence custody, independent observer) are requirements, not properties this route has been
  shown to have.
