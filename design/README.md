# Design

Current design: what Work Engine is meant to be. Files here are revised in place
to stay true, and are marked stale when superseded. See
[repository layout](../docs/repository-layout.md) for how this directory relates
to the others.

## Placement is not acceptance

Being in this directory identifies a document as part of the design corpus. It
does not accept, authorize or settle anything inside it. Each document's own
status stays authoritative; the architecture views, for example, deliberately
mix accepted design, reconciled-but-unaccepted candidates and open hypotheses
under [`status-grammar.md`](architecture/status-grammar.md).

## What belongs here

System-level design: architecture views, ideas being reconciled into design,
and design-bearing research.

Not here:

- binding doctrine (`DESIGN.md`, `PHILOSOPHY.md` at the root);
- operating references (`docs/`);
- a component's own documentation (with the component);
- records of design work, such as plans, reviews and reconciliation waves
  (`records/`, `planning/`, `reviews/`).

## What is here

Phase 5 of the [layout plan](../planning/repository-layout-plan.md) moved 87
files into this directory on 2026-10-27. 33 moved byte-identical; 54 had their
references to old paths rewritten. By count:

| Location | Files | Came from |
| --- | --- | --- |
| [`work-engine-planned-architecture.md`](work-engine-planned-architecture.md) | 1 | `app-server/docs/` (the capstone) |
| [`architecture/`](architecture/) | 23 (16 views, 5 mechanisms, 2 substrates) | `app-server/docs/architecture/` |
| [`ideas/pending/`](ideas/pending/) | 57 (49 ideas, 8 pilots) | `app-server/ideas/pending/` |
| [`ideas/history/2026-09-15-pre-synthesis/`](ideas/history/2026-09-15-pre-synthesis/) | 3 | `app-server/ideas/history/` |
| [`ideas/retired/`](ideas/retired/) | 1 | `app-server/ideas/retired/` |
| [`research/`](research/) | 2 | `app-server/docs/research/` |

Eleven of these files are cited by a full-length SHA-256 or git blob id, so they
moved byte-identical and their citations still match. The idea-status validator,
[`app-server/scripts/validate-idea-status.mjs`](../app-server/scripts/validate-idea-status.mjs),
now scans `design/ideas/pending` by default.

## What stayed, and why

- [`experiments/context-lifecycle-rust/DESIGN.md`](../experiments/context-lifecycle-rust/DESIGN.md)
  and `DESIGN_EVIDENCE.md`: the experiment is a digest-bound unit, so its design
  stays with it.
- [`ideas/`](../ideas/README.md): the earlier reconciled projection of the root
  ideas, kept as a separate set. The current ideas pipeline is
  [`design/ideas/`](ideas/); the projection is reconciled in
  [`app-server/docs/root-ideas-reconciliation.md`](../app-server/docs/root-ideas-reconciliation.md).
- The 16 reconciliation documents and their companions under `app-server/` and
  `app-server/docs/`:
  records of reconciliation work, left as written. Their references to the moved
  files are listed under Unresolved references below.
- The other 19 research notes in
  [`app-server/docs/research/`](../app-server/docs/research/) and the component
  documents in `app-server/docs/`: not part of the moved set.

New system-level design documents go in this directory and may link to the
locations above.

## Unresolved references

These references still name an old path and were left as written. Old-to-new
mapping is the table below. Every reference and structured field that phase 5 found, with
its class, resolution before and after, treatment and basis, is in
[`records/repository-layout-phase5-resolution-report.json`](../records/repository-layout-phase5-resolution-report.json).

**Byte-locked or another session's file (24 reference records).** The file is
digest-cited, or untracked and owned by another session, so its text was not
rewritten:

| File (now) | Lines | Why left |
| --- | --- | --- |
| `app-server/docs/context-lifecycle-transition-protocol.md` | 770, 772 | untracked; cited by `experiments/context-lifecycle-rust/design-sources.json:118` |
| `app-server/docs/prior-art.md` | 21 | untracked; owner-held |
| `experiments/context-lifecycle-rust/BOUNDED_INFERENCE_IMPLEMENTATION_PLAN.md` | 61, 66, 72, 75, 79, 83, 104, 334, 336, 337 | untracked; cited by `BOUNDED_INFERENCE_DECOMPOSITION.md:11` |
| `rust/crates/terminal-ui/docs/implementation-roadmap.md` | 273 | cited by a digest in `planning/s10-live4-admission-artifacts-20261021/engineering/source-manifest-v1.json:3211` |
| [`ideas/pending/execution-envelope-live-effect-admission.md`](ideas/pending/execution-envelope-live-effect-admission.md) | 17, 1832, 1951 | byte-locked (digest cited by `authority-commit-gate-and-effect-admission.md:45`) |
| [`ideas/pending/provider-turn-harness-runtime-and-operator-projection.md`](ideas/pending/provider-turn-harness-runtime-and-operator-projection.md) | 16, 18, 474 | byte-locked (`design-sources.json:976`) |
| [`ideas/pending/service-plane-and-kernel-domain-boundary.md`](ideas/pending/service-plane-and-kernel-domain-boundary.md) | 20, 503 | byte-locked (`design-sources.json:960`) |

**Reconciliation and seam records (32 records in 17 files).** Left
as written; each is a dated record of the state when it was written:
`app-server/claims-service-migration.md` (95), and under `app-server/docs/`:
`agent-instruction-structure-and-placement-review-reconciliation.md` (10),
`ai-accessible-browser-seam-reconciliation.md` (7, 18, 266, 297),
`architecture-direction-seam-map.md` (6),
`claim-maintenance-and-reliance-propagation-reconciliation.md` (119),
`closed-loop-engineering-learning-reconciliation.md` (6, 66, 74),
`cross-cutting-seam-review-and-architectural-review-reconciliation.md` (15),
`evidence-backed-proposal-evaluation-reconciliation.md` (10, 198),
`organizational-execution-envelopes-reconciliation.md` (508),
`proposal-backed-portfolio-selection-reconciliation.md` (10),
`proposal-research-maturity-and-freshness-reconciliation.md` (8, 188, 255),
`review-scope-coordination-reconciliation.md` (177),
`role-decision-trace-reconciliation.md` (43),
`root-ideas-reconciliation.md` (219, 231, 362, 371, 401, 403),
`service-plane-reconciliation.md` (9, 164, 453),
`ui-review-capability-reconciliation.md` (6),
`work-engine-studio-reconciliation.md` (6). Six of the 32 are markdown links
(`ai-accessible-browser-seam-reconciliation.md` lines 7, 18, 266 and
`service-plane-reconciliation.md` lines 9, 164, 453), which now do not resolve.

**Other named cases.**

- `app-server/docs/architecture/status-grammar.md` lines 293, 330, 362 (now
  [`architecture/status-grammar.md`](architecture/status-grammar.md)): worked
  examples in section 10 that illustrate a status block as it was; left as
  written.
- [`ideas/pending/execution-strategy-selection.md`](ideas/pending/execution-strategy-selection.md)
  lines 45-54: ten provenance locators left as written, so the file moved
  byte-identical.
- `experiments/context-lifecycle-rust/design-sources.json:950`: names
  `app-server/ideas/pending/authority-commit-gate-and-effect-admission.md`; it
  is a manifest line naming the moved file, left as written because its
  historical basis is unresolved.
- The frozen readiness profiles
  `skills/manual-orchestration/readiness/integration-readiness-v1.json` (line 16)
  and `skills/manual-orchestration/readiness/test.campaign-acceptance-sketch-v1.json`
  (lines 15, 16): their `path` fields name
  `app-server/docs/architecture/decision-specific-readiness.md`. They are
  byte-locked, so they were left as written. The Rust fixture copy,
  `rust/crates/decision-readiness/tests/fixtures/integration-readiness-v1.json`
  (line 16), is a historical fixture.
- The layout plan's own mentions of the old paths, `HISTORY.md` and dated
  records that name the old locations: historical, not rewritten.

## Moved files

Old paths are code spans because they no longer exist.

| Old path | New path |
| --- | --- |
| `app-server/docs/architecture/architectural-kind-recognition.md` | [`design/architecture/architectural-kind-recognition.md`](architecture/architectural-kind-recognition.md) |
| `app-server/docs/architecture/authority-and-ownership.md` | [`design/architecture/authority-and-ownership.md`](architecture/authority-and-ownership.md) |
| `app-server/docs/architecture/context-lifecycle.md` | [`design/architecture/context-lifecycle.md`](architecture/context-lifecycle.md) |
| `app-server/docs/architecture/decision-specific-readiness.md` | [`design/architecture/decision-specific-readiness.md`](architecture/decision-specific-readiness.md) |
| `app-server/docs/architecture/evidence-and-claims.md` | [`design/architecture/evidence-and-claims.md`](architecture/evidence-and-claims.md) |
| `app-server/docs/architecture/execution-strategy-selection.md` | [`design/architecture/execution-strategy-selection.md`](architecture/execution-strategy-selection.md) |
| `app-server/docs/architecture/implementation-contract-compilation.md` | [`design/architecture/implementation-contract-compilation.md`](architecture/implementation-contract-compilation.md) |
| `app-server/docs/architecture/material-decision-selection.md` | [`design/architecture/material-decision-selection.md`](architecture/material-decision-selection.md) |
| `app-server/docs/architecture/mechanisms/authority-preserving-intent-projection.md` | [`design/architecture/mechanisms/authority-preserving-intent-projection.md`](architecture/mechanisms/authority-preserving-intent-projection.md) |
| `app-server/docs/architecture/mechanisms/candidate-resolution-and-admission.md` | [`design/architecture/mechanisms/candidate-resolution-and-admission.md`](architecture/mechanisms/candidate-resolution-and-admission.md) |
| `app-server/docs/architecture/mechanisms/resource-lease-and-fencing.md` | [`design/architecture/mechanisms/resource-lease-and-fencing.md`](architecture/mechanisms/resource-lease-and-fencing.md) |
| `app-server/docs/architecture/mechanisms/revision-cas-and-publication.md` | [`design/architecture/mechanisms/revision-cas-and-publication.md`](architecture/mechanisms/revision-cas-and-publication.md) |
| `app-server/docs/architecture/mechanisms/transition-fencing-and-leases.md` | [`design/architecture/mechanisms/transition-fencing-and-leases.md`](architecture/mechanisms/transition-fencing-and-leases.md) |
| `app-server/docs/architecture/organizational-compilation.md` | [`design/architecture/organizational-compilation.md`](architecture/organizational-compilation.md) |
| `app-server/docs/architecture/portfolio-selection.md` | [`design/architecture/portfolio-selection.md`](architecture/portfolio-selection.md) |
| `app-server/docs/architecture/proposal-evaluation.md` | [`design/architecture/proposal-evaluation.md`](architecture/proposal-evaluation.md) |
| `app-server/docs/architecture/review.md` | [`design/architecture/review.md`](architecture/review.md) |
| `app-server/docs/architecture/role-and-contract-structure.md` | [`design/architecture/role-and-contract-structure.md`](architecture/role-and-contract-structure.md) |
| `app-server/docs/architecture/runtime-realization.md` | [`design/architecture/runtime-realization.md`](architecture/runtime-realization.md) |
| `app-server/docs/architecture/semantic-planning-hierarchy.md` | [`design/architecture/semantic-planning-hierarchy.md`](architecture/semantic-planning-hierarchy.md) |
| `app-server/docs/architecture/status-grammar.md` | [`design/architecture/status-grammar.md`](architecture/status-grammar.md) |
| `app-server/docs/architecture/substrates/context-observer.md` | [`design/architecture/substrates/context-observer.md`](architecture/substrates/context-observer.md) |
| `app-server/docs/architecture/substrates/evidence-anchor.md` | [`design/architecture/substrates/evidence-anchor.md`](architecture/substrates/evidence-anchor.md) |
| `app-server/docs/work-engine-planned-architecture.md` | [`design/work-engine-planned-architecture.md`](work-engine-planned-architecture.md) |
| `app-server/ideas/history/2026-09-15-pre-synthesis/deterministic-authority-projection-into-logical-role-vantage-points-notes.md` | [`design/ideas/history/2026-09-15-pre-synthesis/deterministic-authority-projection-into-logical-role-vantage-points-notes.md`](ideas/history/2026-09-15-pre-synthesis/deterministic-authority-projection-into-logical-role-vantage-points-notes.md) |
| `app-server/ideas/history/2026-09-15-pre-synthesis/deterministic-authority-projection-into-logical-role-vantage-points-original.md` | [`design/ideas/history/2026-09-15-pre-synthesis/deterministic-authority-projection-into-logical-role-vantage-points-original.md`](ideas/history/2026-09-15-pre-synthesis/deterministic-authority-projection-into-logical-role-vantage-points-original.md) |
| `app-server/ideas/history/2026-09-15-pre-synthesis/deterministic-authority-projection-into-logical-role-vantage-points-transition.md` | [`design/ideas/history/2026-09-15-pre-synthesis/deterministic-authority-projection-into-logical-role-vantage-points-transition.md`](ideas/history/2026-09-15-pre-synthesis/deterministic-authority-projection-into-logical-role-vantage-points-transition.md) |
| `app-server/ideas/pending/AI_ACCESSIBLE_BROWSER_DESIGN.md` | [`design/ideas/pending/AI_ACCESSIBLE_BROWSER_DESIGN.md`](ideas/pending/AI_ACCESSIBLE_BROWSER_DESIGN.md) |
| `app-server/ideas/pending/active-evidence-acquisition-during-execution.md` | [`design/ideas/pending/active-evidence-acquisition-during-execution.md`](ideas/pending/active-evidence-acquisition-during-execution.md) |
| `app-server/ideas/pending/architecture-decision-discovery-from-concept-to-implementation.md` | [`design/ideas/pending/architecture-decision-discovery-from-concept-to-implementation.md`](ideas/pending/architecture-decision-discovery-from-concept-to-implementation.md) |
| `app-server/ideas/pending/authority-backed-architecture-directions-as-workflow-inputs.md` | [`design/ideas/pending/authority-backed-architecture-directions-as-workflow-inputs.md`](ideas/pending/authority-backed-architecture-directions-as-workflow-inputs.md) |
| `app-server/ideas/pending/authority-commit-gate-and-effect-admission.md` | [`design/ideas/pending/authority-commit-gate-and-effect-admission.md`](ideas/pending/authority-commit-gate-and-effect-admission.md) |
| `app-server/ideas/pending/candidate-trajectory-builder-side-consumption.md` | [`design/ideas/pending/candidate-trajectory-builder-side-consumption.md`](ideas/pending/candidate-trajectory-builder-side-consumption.md) |
| `app-server/ideas/pending/candidate-trajectory-durable-foundation.md` | [`design/ideas/pending/candidate-trajectory-durable-foundation.md`](ideas/pending/candidate-trajectory-durable-foundation.md) |
| `app-server/ideas/pending/candidate-trajectory-remediation-delta-for-native-review.md` | [`design/ideas/pending/candidate-trajectory-remediation-delta-for-native-review.md`](ideas/pending/candidate-trajectory-remediation-delta-for-native-review.md) |
| `app-server/ideas/pending/candidate-trajectory-upstream-amendments.md` | [`design/ideas/pending/candidate-trajectory-upstream-amendments.md`](ideas/pending/candidate-trajectory-upstream-amendments.md) |
| `app-server/ideas/pending/change-specific-review-concerns-and-cross-sections.md` | [`design/ideas/pending/change-specific-review-concerns-and-cross-sections.md`](ideas/pending/change-specific-review-concerns-and-cross-sections.md) |
| `app-server/ideas/pending/claim-relative-environment-projections.md` | [`design/ideas/pending/claim-relative-environment-projections.md`](ideas/pending/claim-relative-environment-projections.md) |
| `app-server/ideas/pending/claude-runtime-adapter-and-context-ownership-pilot.md` | [`design/ideas/pending/claude-runtime-adapter-and-context-ownership-pilot.md`](ideas/pending/claude-runtime-adapter-and-context-ownership-pilot.md) |
| `app-server/ideas/pending/context-replacement-emergency-and-writable-operator-ingress.md` | [`design/ideas/pending/context-replacement-emergency-and-writable-operator-ingress.md`](ideas/pending/context-replacement-emergency-and-writable-operator-ingress.md) |
| `app-server/ideas/pending/contract-kit-and-lifecycle-template-duplication.md` | [`design/ideas/pending/contract-kit-and-lifecycle-template-duplication.md`](ideas/pending/contract-kit-and-lifecycle-template-duplication.md) |
| `app-server/ideas/pending/deterministic-authority-projection-and-adaptive-organizational-topology.md` | [`design/ideas/pending/deterministic-authority-projection-and-adaptive-organizational-topology.md`](ideas/pending/deterministic-authority-projection-and-adaptive-organizational-topology.md) |
| `app-server/ideas/pending/deterministic-refactor-pressure-from-work-engine-evidence.md` | [`design/ideas/pending/deterministic-refactor-pressure-from-work-engine-evidence.md`](ideas/pending/deterministic-refactor-pressure-from-work-engine-evidence.md) |
| `app-server/ideas/pending/evidence-anchor-observation-and-impact-nomination.md` | [`design/ideas/pending/evidence-anchor-observation-and-impact-nomination.md`](ideas/pending/evidence-anchor-observation-and-impact-nomination.md) |
| `app-server/ideas/pending/evidence-basis-topology-for-bounded-inference.md` | [`design/ideas/pending/evidence-basis-topology-for-bounded-inference.md`](ideas/pending/evidence-basis-topology-for-bounded-inference.md) |
| `app-server/ideas/pending/evidence-calibrated-plan-resolution-and-continuous-capability-learning.md` | [`design/ideas/pending/evidence-calibrated-plan-resolution-and-continuous-capability-learning.md`](ideas/pending/evidence-calibrated-plan-resolution-and-continuous-capability-learning.md) |
| `app-server/ideas/pending/evidence-calibrated-plan-resolution-upstream-amendments.md` | [`design/ideas/pending/evidence-calibrated-plan-resolution-upstream-amendments.md`](ideas/pending/evidence-calibrated-plan-resolution-upstream-amendments.md) |
| `app-server/ideas/pending/evidence-topology-inference-fitness-pilot.md` | [`design/ideas/pending/evidence-topology-inference-fitness-pilot.md`](ideas/pending/evidence-topology-inference-fitness-pilot.md) |
| `app-server/ideas/pending/executable-generation-maintenance-rollover.md` | [`design/ideas/pending/executable-generation-maintenance-rollover.md`](ideas/pending/executable-generation-maintenance-rollover.md) |
| `app-server/ideas/pending/execution-envelope-live-effect-admission.md` | [`design/ideas/pending/execution-envelope-live-effect-admission.md`](ideas/pending/execution-envelope-live-effect-admission.md) |
| `app-server/ideas/pending/execution-strategy-selection.md` | [`design/ideas/pending/execution-strategy-selection.md`](ideas/pending/execution-strategy-selection.md) |
| `app-server/ideas/pending/fenced-oauth-credential-tip-broker.md` | [`design/ideas/pending/fenced-oauth-credential-tip-broker.md`](ideas/pending/fenced-oauth-credential-tip-broker.md) |
| `app-server/ideas/pending/governed-causal-diagnosis-and-epistemic-learning.md` | [`design/ideas/pending/governed-causal-diagnosis-and-epistemic-learning.md`](ideas/pending/governed-causal-diagnosis-and-epistemic-learning.md) |
| `app-server/ideas/pending/hierarchical-planning-and-multi-supervisor-orchestration.md` | [`design/ideas/pending/hierarchical-planning-and-multi-supervisor-orchestration.md`](ideas/pending/hierarchical-planning-and-multi-supervisor-orchestration.md) |
| `app-server/ideas/pending/idea-status-grammar.md` | [`design/ideas/pending/idea-status-grammar.md`](ideas/pending/idea-status-grammar.md) |
| `app-server/ideas/pending/incremental-architecture-intake-and-seam-reconciliation.md` | [`design/ideas/pending/incremental-architecture-intake-and-seam-reconciliation.md`](ideas/pending/incremental-architecture-intake-and-seam-reconciliation.md) |
| `app-server/ideas/pending/incremental-terminal-accounting-projection.md` | [`design/ideas/pending/incremental-terminal-accounting-projection.md`](ideas/pending/incremental-terminal-accounting-projection.md) |
| `app-server/ideas/pending/ir.md` | [`design/ideas/pending/ir.md`](ideas/pending/ir.md) |
| `app-server/ideas/pending/lifecycle-integrated-pluggable-agent-runtime.md` | [`design/ideas/pending/lifecycle-integrated-pluggable-agent-runtime.md`](ideas/pending/lifecycle-integrated-pluggable-agent-runtime.md) |
| `app-server/ideas/pending/observation-consequences-and-evidence-basis-evolution.md` | [`design/ideas/pending/observation-consequences-and-evidence-basis-evolution.md`](ideas/pending/observation-consequences-and-evidence-basis-evolution.md) |
| `app-server/ideas/pending/opencode-substrate-evaluation.md` | [`design/ideas/pending/opencode-substrate-evaluation.md`](ideas/pending/opencode-substrate-evaluation.md) |
| `app-server/ideas/pending/persistent-provider-runtime-managers-for-codex-and-claude.md` | [`design/ideas/pending/persistent-provider-runtime-managers-for-codex-and-claude.md`](ideas/pending/persistent-provider-runtime-managers-for-codex-and-claude.md) |
| `app-server/ideas/pending/pilots/adapter-representation-effect-pilot.md` | [`design/ideas/pending/pilots/adapter-representation-effect-pilot.md`](ideas/pending/pilots/adapter-representation-effect-pilot.md) |
| `app-server/ideas/pending/pilots/agent-constraint-pressure-pilot.md` | [`design/ideas/pending/pilots/agent-constraint-pressure-pilot.md`](ideas/pending/pilots/agent-constraint-pressure-pilot.md) |
| `app-server/ideas/pending/pilots/operator-input-adapter-fidelity-pilot.md` | [`design/ideas/pending/pilots/operator-input-adapter-fidelity-pilot.md`](ideas/pending/pilots/operator-input-adapter-fidelity-pilot.md) |
| `app-server/ideas/pending/pilots/operator-input-mediation-reconnaissance.md` | [`design/ideas/pending/pilots/operator-input-mediation-reconnaissance.md`](ideas/pending/pilots/operator-input-mediation-reconnaissance.md) |
| `app-server/ideas/pending/pilots/pilot-revision-candidates.md` | [`design/ideas/pending/pilots/pilot-revision-candidates.md`](ideas/pending/pilots/pilot-revision-candidates.md) |
| `app-server/ideas/pending/pilots/prior-art-matrix-governed-agent-input-and-constraint-pilots.md` | [`design/ideas/pending/pilots/prior-art-matrix-governed-agent-input-and-constraint-pilots.md`](ideas/pending/pilots/prior-art-matrix-governed-agent-input-and-constraint-pilots.md) |
| `app-server/ideas/pending/pilots/prior-art-research-round-2-log.md` | [`design/ideas/pending/pilots/prior-art-research-round-2-log.md`](ideas/pending/pilots/prior-art-research-round-2-log.md) |
| `app-server/ideas/pending/pilots/prior-art-research-round-2-plan.md` | [`design/ideas/pending/pilots/prior-art-research-round-2-plan.md`](ideas/pending/pilots/prior-art-research-round-2-plan.md) |
| `app-server/ideas/pending/pre-indexed-capability-resolution-and-frozen-runtime-realization.md` | [`design/ideas/pending/pre-indexed-capability-resolution-and-frozen-runtime-realization.md`](ideas/pending/pre-indexed-capability-resolution-and-frozen-runtime-realization.md) |
| `app-server/ideas/pending/proposal-decision-gated-implementation-compilation.md` | [`design/ideas/pending/proposal-decision-gated-implementation-compilation.md`](ideas/pending/proposal-decision-gated-implementation-compilation.md) |
| `app-server/ideas/pending/provider-turn-harness-runtime-and-operator-projection.md` | [`design/ideas/pending/provider-turn-harness-runtime-and-operator-projection.md`](ideas/pending/provider-turn-harness-runtime-and-operator-projection.md) |
| `app-server/ideas/pending/requirement-satisfaction-claims-for-reactive-semantic-propagation.md` | [`design/ideas/pending/requirement-satisfaction-claims-for-reactive-semantic-propagation.md`](ideas/pending/requirement-satisfaction-claims-for-reactive-semantic-propagation.md) |
| `app-server/ideas/pending/resolution-layered-semantic-refinement.md` | [`design/ideas/pending/resolution-layered-semantic-refinement.md`](ideas/pending/resolution-layered-semantic-refinement.md) |
| `app-server/ideas/pending/revisioned-research-and-execution-architecture.md` | [`design/ideas/pending/revisioned-research-and-execution-architecture.md`](ideas/pending/revisioned-research-and-execution-architecture.md) |
| `app-server/ideas/pending/scoped-workflow-event-surface.md` | [`design/ideas/pending/scoped-workflow-event-surface.md`](ideas/pending/scoped-workflow-event-surface.md) |
| `app-server/ideas/pending/semantic-inventory-integrity-projection-separation.md` | [`design/ideas/pending/semantic-inventory-integrity-projection-separation.md`](ideas/pending/semantic-inventory-integrity-projection-separation.md) |
| `app-server/ideas/pending/service-plane-and-kernel-domain-boundary.md` | [`design/ideas/pending/service-plane-and-kernel-domain-boundary.md`](ideas/pending/service-plane-and-kernel-domain-boundary.md) |
| `app-server/ideas/pending/structural-plan-ir-addendum-execution-profile-scoring-and-strategy-selection.md` | [`design/ideas/pending/structural-plan-ir-addendum-execution-profile-scoring-and-strategy-selection.md`](ideas/pending/structural-plan-ir-addendum-execution-profile-scoring-and-strategy-selection.md) |
| `app-server/ideas/pending/structural-plan-ir-addendum-planner-owned-execution-characterization.md` | [`design/ideas/pending/structural-plan-ir-addendum-planner-owned-execution-characterization.md`](ideas/pending/structural-plan-ir-addendum-planner-owned-execution-characterization.md) |
| `app-server/ideas/pending/structural-plan-ir-for-capability-aware-multi-model-execution.md` | [`design/ideas/pending/structural-plan-ir-for-capability-aware-multi-model-execution.md`](ideas/pending/structural-plan-ir-for-capability-aware-multi-model-execution.md) |
| `app-server/ideas/pending/structural-plan-ir-prior-art.md` | [`design/ideas/pending/structural-plan-ir-prior-art.md`](ideas/pending/structural-plan-ir-prior-art.md) |
| `app-server/ideas/pending/vantage-composition-quality-cost-and-lifetime.md` | [`design/ideas/pending/vantage-composition-quality-cost-and-lifetime.md`](ideas/pending/vantage-composition-quality-cost-and-lifetime.md) |
| `app-server/ideas/retired/production-path-claim-evidence-remediation.md` | [`design/ideas/retired/production-path-claim-evidence-remediation.md`](ideas/retired/production-path-claim-evidence-remediation.md) |
| `app-server/docs/research/BEHAVIORAL_ENVIRONMENTS_RESEARCH_FOUNDATIONS.md` | [`design/research/BEHAVIORAL_ENVIRONMENTS_RESEARCH_FOUNDATIONS.md`](research/BEHAVIORAL_ENVIRONMENTS_RESEARCH_FOUNDATIONS.md) |
| `app-server/docs/research/STRUCTURE_OVER_PROCEDURE_RESEARCH_FOUNDATIONS.md` | [`design/research/STRUCTURE_OVER_PROCEDURE_RESEARCH_FOUNDATIONS.md`](research/STRUCTURE_OVER_PROCEDURE_RESEARCH_FOUNDATIONS.md) |
