# Requirements revision `hosted-route-qualification-r2`

**Standing: an explicitly versioned requirements revision for one bounded hosted-route
qualification on the `claude-sdk` profile. Not the frozen requirements, not a qualified
organization, and no source of an established condition or a consumable result.**

## Authority

Operator decision D, 2026-11-18, quoted (it supersedes, for this revision, the earlier D2 removal
of both constraints):

> Requirements revision keeps `provider_family: anthropic` and removes only
> `execution_confinement`.

Still binding from operator decision D2 / OQ-1 (D), 2026-11-17 (quoted in full in
`../hosted-route-qualification-r1/DERIVATION.md`): a distinct, explicitly versioned revision;
the original obligation, semantic scope and normative requirements preserved; a new run identity
and truthful proposition; the original frozen candidate unchanged and its conditions unmet; no
qualification outcome establishes a review condition or becomes consumable.

Revision r1 stays in the tree unchanged as history and as the revision of the `openai-codex`
profile. This revision is the one the `claude-sdk` profile uses.

## Derivation

Source: `rust/crates/work-engine-organization/tests/fixtures/frozen/requirements.json`
(unchanged). Applied, and nothing else (the script is retained with the objective evidence as
`h1/remediation-2/make_r2.py`):

```text
value = parse(frozen/requirements.json)
value.problemScope.runId = "hosted-route-qualification-r2-20261118"
for condition in value.acceptanceConditions:
    assert [c.dimension for c in condition.realizationConstraints] == [provider_family, execution_confinement]
    condition.realizationConstraints = [c for c in condition.realizationConstraints
                                        if c.dimension != execution_confinement]
    condition.proposition = <route text naming its own boundary>
assert parse(hosted-route-qualification-r2/requirements.json) == value
```

Each condition keeps `{"dimension": "provider_family", "allowed": ["anthropic"]}` exactly as frozen.

Checked by `tests/revision.rs::revision_r2_is_the_frozen_requirements_without_only_execution_confinement`
(semantic JSON equality, the exact list of differing JSON paths, and the kept constraint equal to
the frozen one) and `revision_r2_compiles_and_the_claude_sdk_claim_has_no_known_incompatibility`.

Unchanged: everything r1's DERIVATION.md lists as unchanged (obligation, result contract,
`scope.digest` = the frozen subject's `patchIdentity`, plan lineage, every normative value,
consumption boundaries, consumer classes, covered state, unestablished route, acceptor, required
authorities, organization policy), and in addition the `provider_family` constraint.

The propositions describe this route truthfully: route qualification only; the hosted Pi analysis
profile `work-engine.pi-analysis-hosted.v1`; the requested model `claude-haiku-5-5` via Pi provider
`claude-sdk` (Claude Code in the inference container, Anthropic Messages family, provider family
`anthropic`); the Host-served read-only snapshot tools, presented as `mcp__pi__snapshot_list` and
`mcp__pi__snapshot_read`; not the guarded Claude reviewer the frozen requirements name, because
execution confinement is not established; no condition established; not consumable.

## Preparation on this revision

With the `claude-sdk` claim (provider family `anthropic`, confinement unstated), the unchanged
`prepare` evaluates `realization:provider_family` as `claimed_compatible` (a claim, not admission
evidence; `work-engine-review-preparation/src/realization.rs:88-92`) and produces no
`KnownIncompatibility` blocker. Every other blocker stays undischarged; mutation and capabilities
stay `unestablished` because the claim names tools (`realization.rs:134-138`, `:166-176`).

## What this revision does not do

- It does not change the frozen requirements or their candidate, and does not meet their
  conditions. On the `claude-sdk` profile the frozen requirements are no longer refused by a known
  provider-family incompatibility (`anthropic` is allowed there too); they stay unmet because their
  `execution_confinement` constraint is unestablished (recorded as `frozen_world.outcome:
  prepared_conditions_unmet`), and no job inputs are produced from them.
- Keeping `provider_family` keeps a check that the claim satisfies only as a claim: the Host
  enforces the Anthropic catalog entry and Anthropic origins, but which endpoint served each
  attempt, and who paid, is Host-reported, not established here.
- The normative values it keeps are requirements, not properties this route has been shown to
  have.
