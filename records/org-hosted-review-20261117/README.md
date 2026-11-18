# Org step 3: hosted review route qualification (2026-11-17/18)

Operator objective step 3 (see `records/org-hardening-owners-20261117/` for steps 1-2): hosted review job inputs and result checker for the Pi analysis profile, crate `rust/crates/work-engine-review-hosted`.

- H1 implementation, remediations 1-6, mutants, reviews: `h1/`. Fable review-2 `accept_with_changes`; P3-1/2/4/7 fixed in `h1/remediation-6/` (64 tests). Committed crate files match `h1/remediation-6/final-hashes.txt`.
- D3: `d3/`.
- End-to-end runs: `e2e/` (envelopes, `LIVE-RECEIPT.md`, job inputs, contract, requirements, frozen world, run `summary.json`). Rechecked with the remediation-6 checker: live `checks_passed`; offline fails only `strict_json`.
- Results are `owner_pending` and non-consumable.
- Budget: `assignment.json`, `cost.py`, `children.txt`.

Not retained: the Claude runtime copy (`claude-runtime/`), role-host state, lock files, the protected review subject snapshots and rendered task text (`e2e/*-inputs/snapshot`, `task.txt`), and the run directories (including `private/`) apart from their `summary.json`.

Merged into `rescue-detached-20260911` on operator request (2026-11-18).
