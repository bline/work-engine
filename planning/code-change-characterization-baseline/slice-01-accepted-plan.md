# Accepted Plan: Code Change Characterization Baseline, Slice 1

- Run: `code-change-profile-baseline-20260823`
- Baseline: `0da2d771ada490a9ac8a9a02aea295433dbb8b4d`
- Acceptance: `procedural_auto_approval` after explicit user authorization
- Route: `falsified-placement`
- Placement risk: low for this bounded baseline
- Placement verdict: accepted at `skills/code-change-profile/`
- Review evidence: accepted same-model review, `gpt-5.6-sol` low,
  `independence_claimed: false`

## Bounded slice

Create the first shared, recomputable deterministic Code Change Profile
vertical. Bind a small, truthful set of existing immutable Work Engine
checkpoint subjects; validate their provenance through the checkpoint owner;
and derive versioned canonical observations for physical diff shape, file
categories, tests, documentation, configuration, bounded changed symbols, and
module distribution with explicit measurement states.

The slice does not implement semantic classification, architecture or claim
ownership, Review Bench admission/truth/scoring/routing, raw execution evidence
archival, entity lineage, historical graph comparison, profile aggregation,
correlation analysis, causal claims, or a universal complexity score.

The representative matrix in the proposal is an eventual target. This slice
admits only subjects whose immutable base/result identities and attributed
task-owned manifests can be established. Missing UI/browser and paired
remediation subjects remain explicit representation gaps.

## Accepted semantic-path certificate

The trigger is deliberate admission of an exact immutable Work Engine
checkpoint subject. The analyzer consumes the full checkpoint lifecycle
receipt, including baseline identity and attributed paths, and delegates
checkpoint-binding validation to `slice-checkpoint`; the supervisor's compact
metrics projection is not a sufficient derivation subject.

`skills/code-change-profile/` produces and owns the subject/profile schemas,
analyzer-version semantics, canonical derivation, measurement-state vocabulary,
derived profile identity, and profile validation. It does not own checkpoint
ancestry, refs, acceptance, or attribution semantics.

A fresh repository consumer loads canonical profile JSON without reading the
campaign transcript or current working tree. The lifecycle is admitted subject,
validated immutable binding, derived profile revision, and retained or
recomputed output. Revised analyzers create new addressable derived identities
rather than overwrite subject identity or prior derived revisions.

The observable consequence is a recomputable, evidence-qualified physical
baseline for exactly the admitted change, with observed zero, unknown,
unsupported, failed, and not applicable remaining distinct.

A real accepted checkpoint must validate against immutable Git ancestry and
patch digest, produce byte-identical canonical output across two runs, and
yield expected observations plus explicit non-observed states. A current
worktree diff, terminal metrics alone, checkpoint schema extension, prose
inventory, semantic classifier, Review Bench case, or raw transcript archive
is an insufficient substitute.

## Invariants

- Subject identity binds repository, immutable base/result commits and trees,
  checkpoint disposition, run/slice/attempt/plan/scope, attributed manifest and
  digest, patch digest, construction method, evidence cutoff, and limitations.
- Analysis reads immutable Git objects and never infers task scope from the
  current branch, index, or working tree.
- Checkpoint, proposal, architecture, claim, review, and runtime owners retain
  their authority; the profile only references them.
- Canonical output is independent of locale, traversal order, recomputation
  time, current branch, index, and dirty worktree.
- Every measurement has an explicit state. Zero is valid only after successful
  measurement of an empty result.
- Unsupported, failed, unknown, and not applicable remain distinct and retain
  limitations or failure identity without fabricated values.
- Analyzer version and exact source/subject hashes are retained; analyzer
  revisions produce new profile identities.
- Analysis mutates no branch, real index, user file, checkpoint ref, or metrics
  history.

## Expected task boundary and baseline overlaps

Expected additions are confined to `skills/code-change-profile/`: its
`SKILL.md`, contract reference, subject/profile schemas, analyzer, focused
tests, and immutable subject fixtures. Checked canonical profile examples are
allowed only when they materially improve the fresh-consumer proof.

The implementation must not modify `skills/slice-checkpoint/**`,
`skills/slice-supervisor/**`, `skills/review-bench/**`,
`docs/ai-workflow-metrics.jsonl`, or checkpoint private refs.

Supervisor-owned baseline paths remain read-only and excluded from the
implementation checkpoint:

- `proposals/empirical-agent-research/code-change-characterization-profile/packet.json`
- `proposals/empirical-agent-research/code-change-characterization-profile/decision.json`
- `campaigns/code-change-characterization-baseline.yaml`
- `planning/code-change-characterization-baseline/slice-01-accepted-plan.md`

## Validation mapping

- `semantic_proof`: a real accepted checkpoint fixture validates immutable
  identity and patch digest, profiles twice byte-identically, and is consumable
  without current-worktree or campaign context.
- `risk_proportional_checks`: focused profile tests, skill validation, tamper
  and measurement-state tests, and the existing slice-checkpoint regression.
- `workspace_integrity`: exact task manifest, baseline-overlap exclusion,
  current-worktree independence, `git diff --check`, and `git status --short`.
- Final adversarial review uses fresh-process `gpt-5.6-sol` at low reasoning and
  is labeled `accepted_same_model_review`, never independent review.

Implementation must return to planning if the checkpoint owner cannot validate
the full lifecycle receipt without modification, a real subject cannot be
truthfully admitted, or a consumer requires semantic or policy ownership.
