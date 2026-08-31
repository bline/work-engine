---
name: comparative-repository-analysis
description: Build compatible, evidence-backed repository profiles and separate Work Engine decisions under a frozen comparison-run contract.
---

# Comparative Repository Analysis

All commands below run from the repository root. The analysis agent's first
prompt must provide the path to the standalone human configuration, normally
`work-engine/campaigns/comparative-repository-analysis.yaml`.
Begin by preparing the durable contract:

```bash
python3 work-engine/skills/comparative-repository-analysis/scripts/comparison_artifacts.py prepare-run \
  --config work-engine/campaigns/comparative-repository-analysis.yaml \
  --output comparison-contract.json
```

Preparation reports `not_ready` and writes nothing while repository revisions
or the Work Engine snapshot remain unresolved. Do not begin repository analysis
until it reports `prepared`.

Before any semantic pass, launch one disposable index-preparation context per
repository using `references/index-preparation-prompt.md`. Give it only the
frozen run identity, repository entry, index policy, and receipt contract. The
main comparison context must receive only its compact terminal receipt.

`index_repository` is the preferred blocking Codebase Memory operation. The
current MCP interface exposes no wait primitive, so configuration fixes
`status_rechecks` at zero: an unexpected nonterminal result is recorded as
`unsupported_nonterminal_status`, not disguised as deterministic polling.
Its `persistence` flag controls writing the compressed
`.codebase-memory/graph.db.zst` team-sharing artifact. `false` does not make the
live index ephemeral or prevent later reuse.

Every dimension pass must carry its compatible `index_preparation_receipt_v1`.
Only `ready` proceeds directly. `coverage_limited` additionally requires a
separate, model-owned `index_fallback_decision_v1` permitted by the frozen
policy. All other terminal states block semantic analysis.

Use `scripts/comparison_artifacts.py` as the deterministic artifact boundary. It
validates comparison contracts and per-dimension findings, reconciles descriptive
profiles, checks profile compatibility, and validates separate decision artifacts.

Freeze the contract before accepting passes so artifacts from different corpus,
snapshot, ontology, or policy identities cannot be mixed. Analysts may choose evidence routes
and depth adaptively, but they must emit the typed states, claim stages,
correspondence states, provenance, and mechanism relationships required by the
contract. Every evidence item must bind to the repository's frozen commit and a
reopenable path, symbol or section, range, or content digest. Claim-to-evidence
links are reciprocal so an existing but disconnected evidence ID cannot satisfy a
claim. Every supported claim requires direct evidence. Supported interpretations
and comparison implications also require a supported parent at the preceding
claim stage; observations have no parents. Never infer absence from omission, flatten
conflicting claims, or put a KEEP/ADOPT/BORROW/AVOID/INVESTIGATE verdict in a
repository profile. `confirmed_absent` is valid only when the exact negative-search
scope has no receipt, parse, exclusion, fallback, or declared search limitation.

Reconciliation records the digest and evidence-route provenance of every source
pass so a profile cannot lose or substitute its retained-pass lineage. A capability identity conflict remains explicitly unresolved in the profile;
the first submitted definition does not become the retained truth. Every decision
must bind the exact profile digests and acknowledge the current state of each
referenced claim so a stale or substituted profile cannot silently support it. KEEP, ADOPT, BORROW, and AVOID may rely only on supported claims;
INVESTIGATE may retain acknowledged disputed or unresolved claims. Superseded
claims never support a decision.

Open discovery outside the frozen ontology belongs in a separate
`ontology_gap_proposal_v1`, conventionally stored beneath the run artifact root
at `ontology-gaps/<repository-id>/<gap-id>.json`. Validate it against its exact
source pass. Synthesis may recommend migration, selective re-analysis,
rejection, or a future run; a proposal never adds an active dimension.

Copy/pasteable vertical pilot (run exactly from the repository root):

<!-- executable-pilot:start -->
```bash
set -euo pipefail
comparison_cli=work-engine/skills/comparative-repository-analysis/scripts/comparison_artifacts.py
pilot_fixtures=work-engine/skills/comparative-repository-analysis/tests/fixtures/pilot
pilot_output="$(mktemp -d)"
trap 'rm -rf "$pilot_output"' EXIT
python3 "$comparison_cli" freeze-contract "$pilot_fixtures/contract.json" "$pilot_output/contract.json"
python3 "$comparison_cli" validate-pass "$pilot_output/contract.json" "$pilot_fixtures/pass-control.json"
python3 "$comparison_cli" validate-pass "$pilot_output/contract.json" "$pilot_fixtures/pass-review.json"
python3 "$comparison_cli" validate-ontology-gap "$pilot_output/contract.json" "$pilot_fixtures/gap.json" "$pilot_fixtures/pass-control.json"
python3 "$comparison_cli" reconcile "$pilot_output/contract.json" sample "$pilot_fixtures/pass-control.json" "$pilot_fixtures/pass-review.json" --output "$pilot_output/profile.json"
python3 "$comparison_cli" check-compatible "$pilot_output/contract.json" "$pilot_output/profile.json"
python3 "$comparison_cli" validate-decision "$pilot_output/contract.json" "$pilot_fixtures/decision.json" "$pilot_output/profile.json"
```
<!-- executable-pilot:end -->

Human-authored configuration is YAML; durable comparison artifacts are JSON.
YAML loading uses PyYAML safe loading and rejects anchors and aliases. The CLI
owns mechanical compatibility and referential integrity, not repository
checkout, indexing, analysis-agent invocation, semantic search order, corpus
selection, or adoption judgment.

**JSON Schemas validate wire shape only. Schema acceptance never establishes
semantic validity. `comparison_artifacts.py` is the authoritative semantic
validator.**
