---
name: review-bench
description: Build and run evidence-calibrated comparisons of code-review providers, models, and harnesses using immutable or confidence-qualified repository cases. Use when evaluating review fallback quality or same-model harness diversity; do not use a benchmark score as production review approval.
---

# Review Bench

Compare reviewer behavior without treating disagreement, historical provider
claims, or aggregate call counts as ground truth.

## Boundaries

- Bind every case and result to one snapshot digest.
- Keep case artifacts separate from adjudicated truth.
- Give reviewers the contract and evidence, not prior findings or builder
  rationale.
- Record provider, model, harness, inference family, fresh-context state, and
  tool access separately.
- Treat reconstructed snapshots according to their recorded confidence and
  limitations.
- Use human or executable adjudication to map reported findings to truth.
- Keep reports descriptive until sample size and calibration justify stronger
  statistical claims.
- Never convert a bench result into acceptance of the reviewed production
  slice.

Read [artifact-contracts.md](references/artifact-contracts.md) when authoring
or adjudicating corpus, truth, result, or scoring artifacts.
Read [experiment-contract-v1.2.md](references/experiment-contract-v1.2.md) when
selecting new cases or running new reviews. The frozen
[1.0](references/experiment-contract.md) and
[1.1](references/experiment-contract-v1.1.md) contracts remain authoritative
for results that identify those protocols. A contract change requires a new
protocol version; do not reinterpret existing reviewer outputs under revised
rules.

## Modes

### Inventory historical candidates

Use `inventory-history` to find receipts with review or repair evidence and to
identify missing snapshot information. Session matches are forensic candidates,
not automatically valid cases.

```bash
python3 skills/review-bench/scripts/review_bench.py inventory-history \
  --metrics metrics/roadmap.jsonl metrics/work-engine-roadmap.jsonl \
  --codex-sessions /home/bline/.codex/sessions
```

### Validate artifacts

```bash
python3 skills/review-bench/scripts/review_bench.py validate \
  --corpus <corpus.json> \
  --truth <truth.json> \
  --result <result.json> \
  --scoring <scoring.json>
```

### Export a blinded manual-review packet

```bash
python3 skills/review-bench/scripts/review_bench.py export-case \
  --corpus <corpus.json> \
  --case-id <case-id> \
  --output-dir <packet-dir>
```

For a committed snapshot, add `--repository-root <repo> --archive-git` to
produce an attachable `snapshot.tar.gz`. This is an explicit export; inspect
repository content and publication authority before sending it externally.

### Compare adjudicated results

```bash
python3 skills/review-bench/scripts/review_bench.py compare \
  --corpus <corpus.json> \
  --truth <truth.json> \
  --results-dir <results-dir> \
  --scoring-dir <scoring-dir> \
  --output <report.json>
```

Interpret pairwise complementarity as evidence about the tested cases, not as
proof of model independence. Compare complete configurations, including
reasoning effort, protocol, evidence access, pass count, and aggregation.
Prefer a route-class policy when results differ materially across deterministic,
cross-boundary, persistent-state, or novel/high-consequence changes.
