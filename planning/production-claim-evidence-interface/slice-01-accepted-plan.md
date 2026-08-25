# Accepted Plan: Production Claim Evidence Interface — Slice 01

## Identity and approval

- Run: `3876601b-c43d-44c6-94a4-e22646d64257`
- Slice: `1`
- Attempt: `attempt-1`
- Plan: `planning-v1`
- Planning obligation: `planning-1`
- Launch durable revision: `91c4a2100efe593ff67d18ce6fa1ce1603588f67`
- Campaign objective: implement the authorized Production Claim Evidence Interface proposal completely, using as many coherent slices as needed.
- Approval consequence: the supervisor procedurally auto-approved the planning-v1 receipt verbatim. This acceptance authorizes the bounded implementation attempt after the supervisor durably begins it; it does not itself begin implementation, expand the campaign into Claim Maintenance and Reliance Propagation, or waive any validation gate.

## Accepted boundary

Slice 01 implements the production semantic core and repository-local Codex consumer path, stopping before MCP integration:

- a closed v1 shared schema with explicitly named proposal-research and revision-bound-review profiles;
- deterministic stable and immutable revision identities, typed non-destructive lineage, exact-revision reliance, reference status, authority-manifest validation, idempotent operation identities, and predecessor-conflict handling;
- human-readable canonical records and a deterministic non-owning projection that exposes schema/build identity, watermarks, exclusions, failures, freshness, completeness, branches, and conflicts;
- transport-neutral validation, publication, discovery, resolution, traversal, reliance, projection rebuild, and verification mechanics;
- a repository-local skill that describes affordances and boundaries without prescribing a fixed query route; and
- two representative production records plus a vertical consumer test whose discovery begins from bounded criteria rather than a supplied claim identity.

## Placement route and limitation

Route: `falsified-placement-with-provider-quota-limitation`. Placement risk is medium/high because canonical history, authorization, two domain profiles, a Codex interface, and an external projection cross ownership and runtime boundaries.

The configured Claude placement call failed before evidence with HTTP 429 weekly-quota exhaustion. It is recorded as `provider_failure/quota`, not placement falsification, repository evidence, or independent review. Direct Tier-2 evidence nevertheless confirmed the provisional placement: the accepted proposal assigns the shared mechanics to a repository-local capability; the dogfood explicitly disclaims production ownership; `durable-state` owns only opaque CAS durability; and Work Engine MCP is a non-owning projection. The graph generation was full and ready at `2026-08-24T23:15:33Z`; relied-on paths had metadata-matching, no-recorded-issue coverage, with scoped exclusions limited to `__pycache__` trees. Claude availability remains required for any later configured independent-review gate.

## Placement certificate

When an authority-bound producer publishes or queries a production claim operation, the repository-local shared claim-evidence capability validates the closed profile and authority envelope, writes or resolves immutable claim and reliance history, and builds a provenance-bearing projection. A Codex consumer discovers a candidate without receiving its identity, resolves an exact revision, and records exact-scope reliance. A later MCP consumer observes equivalent identity, limitations, freshness, and completeness. The vertical tests prove publication through discovery and reliance; later MCP tests prove transport equivalence. This is not satisfied by copying dogfood files, storing opaque bytes alone, teaching a skill without canonical mechanics, or exposing the synthetic MCP query.

## Invariants

- Visibility never implies truth, freshness, applicability, sufficiency, acceptance, or authority.
- Reliance always names an exact claim revision, consumer revision, and decision scope.
- Shared mechanics do not absorb research materiality or review severity, episode, synthesis, or outcome semantics.
- Possession of a CLI, skill, file, validator, or MCP connection grants no publication authority.
- Publication is idempotent by stable operation identity and fails closed on unauthorized or conflicting predecessor state.
- Corrections, supersession, identity forks, reliance retirement, and tombstones preserve history.
- Projections remain rebuildable and truthfully report unavailable, excluded, failed, partial, or stale inputs; zero results cannot substitute for unavailable completeness.
- Phase-two impact nomination, refresh orchestration, reopening, delivery, monitoring, and recovery remain out of scope.

## Task-owned boundary and baseline

Expected task-owned paths are confined to a new `skills/claim-evidence/` tree:

- `SKILL.md`
- `agents/openai.yaml`
- `references/claim-evidence-contract.md`
- `schemas/*.schema.json`
- `scripts/claim_evidence.py`
- `tests/test_claim_evidence.py`
- narrow fixtures or representative canonical records under the same owner

The tree was absent at planning baseline, so it had no dirty overlap. The untracked user-owned proposal directory remains read-only. `skills/work-engine-mcp/` is deferred and was clean at baseline. All unrelated dirty files must remain untouched. If implementation evidence requires a different semantic owner or task boundary, the builder returns a boundary-change request rather than expanding silently.

## Vertical semantic proof

In a temporary canonical store:

1. admit authority manifests for both profiles;
2. publish one proposal-research claim and one revision-bound review finding;
3. publish an allowed successor and an exact-revision reliance;
4. rebuild the projection;
5. discover the research claim from bounded subject/profile criteria without its identity;
6. resolve the relied-on exact revision; and
7. verify that limitations, authority, evidence baseline, completeness, and branch/conflict state remain attached.

The intended consumer must observe the new state. A producer-only implementation or a projection that silently chooses the newest revision fails the proof.

## Checks

- `python3 -m unittest skills/claim-evidence/tests/test_claim_evidence.py`
- the capability's CLI `verify` or equivalent check-only freshness command against its representative records
- negative cases for unauthorized publication, duplicate idempotency, conflicting predecessor, identity collision, cyclic or dangling lineage, unavailable or integrity-mismatched evidence, partial projection, unsupported versions, retraction, and misleading newest-revision selection
- relevant existing dogfood and durable-state regression suites if their contracts are reused
- `git diff --check`
- baseline-aware task manifest and `git status --short` proving unrelated dirty files were preserved

These checks map `semantic_proof` to the vertical consumer path, `risk_proportional_checks` to focused positive and failure-mode suites plus relevant regressions, and `workspace_integrity` to diff and manifest checks.

## Deferred scope

Slice 02 replaces the synthetic claim query in Work Engine MCP with a read-only projection over the production owner and proves equivalent Codex/MCP state meaning. Final campaign acceptance exercises the external consumer and both representative domain records without ownership collapse. The separately authorized Claim Maintenance and Reliance Propagation proposal remains outside this campaign and is not mutable scope.
