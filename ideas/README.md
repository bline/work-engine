# Reconciled Work Engine Ideas

## Purpose

This directory is a **clean active-idea projection** produced from the current `work-engine(10).tbz2` repository snapshot.

The reconciliation used current implementation and `ARCHITECTURE.md` as evidence of what exists now, `proposals/` as the owner of promoted candidate changes, and `ideas/` as speculative source material.

The goal is not to rewrite history. Original idea files remain useful provenance. This set answers a different question:

> **Which unresolved ideas still exist now, with one semantic idea per file and without re-owning implemented or proposal-owned meaning?**

## Rules used

1. Implemented ideas do not remain active ideas merely because their original brainstorm document still exists.
2. A formed proposal owns its candidate meaning; the originating idea becomes provenance unless it contains a distinct unpromoted consequence.
3. Large ancestor documents are decomposed when later architecture gave their parts different owners.
4. Each retained idea has an explicit `Does not own` or equivalent boundary.
5. Current implementation remains ground truth for current machinery.
6. This clean set does not authorize implementation or alter proposal lifecycle state.

## Active clean idea set

| Idea | Semantic owner of the idea |
| --- | --- |
| `architectural-review.md` | Diagnostic architectural review capability |
| `closed-loop-engineering-learning.md` | Post-implementation calibration/learning |
| `organizational-execution-envelopes.md` | Problem-specific organization assembly |
| `review-scope-coordination.md` | Review applicability vs mutable scope |
| `evidence-backed-proposal-evaluation.md` | Proposal evaluation evidence |
| `proposal-backed-portfolio-selection.md` | Proposal-backed roadmap/portfolio selection |
| `proposal-research-maturity-and-freshness.md` | Research maturity/readiness/freshness |
| `role-decision-trace.md` | Durable semantic decision lineage |
| `cross-cutting-seam-review.md` | Cross-boundary design/contract coherence review |
| `ui-review-capability.md` | Project UI design-review role/capability |
| `environment-adapter-and-host-provided-runtime-services.md` | Provider-neutral host-service integration |
| `runtime-adapter.md` | Provider-neutral runtime execution/binding |
| `control-plane-and-client-protocol.md` | Coordination/control/client protocol |
| `work-engine-studio.md` | Human design/control/forensics surface |

## Original-file disposition

| Original | Disposition | Current owner / clean successor | Reason |
| --- | --- | --- | --- |
| `adaptive-specialized-review-panels.md` | Promoted | `proposals/adaptive-specialized-review/` | No active clean idea; proposal family owns remaining candidate meaning. |
| `architectural-review.md` | Retained / narrowed | `architectural-review.md` | Diagnostic capability retained; historical blocking consequence remains an explicit unresolved authority/policy question. |
| `closed-loop-engineering-learning.md` | Retained / narrowed | `closed-loop-engineering-learning.md` | Outcome calibration only. |
| `context-compaction-as-role-resume-event.md` | Promoted / absorbed | agent-state proposal + `organizational-execution-envelopes.md` durable-role profile | The proposal owns recovery semantics; the clean idea retains profile binding/composition only. |
| `context-derived-role-execution-envelopes.md` | Retained / narrowed | `organizational-execution-envelopes.md` | Organization assembly and system/profile/role definition retained; control/runtime/UI/research details moved to their owners. |
| `durable-review-queue-and-scope-coordination.md` | Retained / narrowed | `review-scope-coordination.md` | Only applicability/mutation coordination; checkpoint and review-artifact semantics moved to implemented/proposed owners. |
| `environment-adapter-and-host-provided-runtime-services.md` | Retained / narrowed | `environment-adapter-and-host-provided-runtime-services.md` | Host-service integration remains unimplemented; duplicate discussion was compressed. |
| `evidence-backed-proposal-evaluation.md` | Split | `evidence-backed-proposal-evaluation.md; proposal-backed-portfolio-selection.md` | Evaluation and portfolio selection are independently decidable; typed evaluation and conditional comparison remain with evaluation. |
| `git-backed-slice-checkpoints.md` | Implemented | `skills/slice-checkpoint + slice-supervisor checkpoint lifecycle` | Remove from active ideas. |
| `interactive-idea-to-proposal-workflow.md` | Implemented foundation / promoted | idea-to-proposal proposal family + `skills/proposal-former` | Skill is an active-construction foundation; the proposal family owns remaining meaning. |
| `persistent-agent-state-and-runtime-introspection.md` | Superseded / decomposed | agent-state proposal + control/runtime/Studio/decision-trace/review/learning ideas | Broad ancestor no longer useful as one idea; general resource claims remain explicit in control-plane direction. |
| `persistent-agent-state.md` | Promoted | `proposals/agent-state/role-owned-durable-operational-state` | Remove from active ideas. |
| `persistent-strategic-planner.md` | Implemented role; persistence generalized elsewhere | `skills/strategic-planner + agent-state proposal` | Remove from active ideas. |
| `proposal-packet-impl.md` | Implemented foundation / promoted | idea-to-proposal proposal family + `skills/proposal-former` | Proposal family owns remaining candidate meaning. |
| `proposal-packet-workflow.md` | Implemented foundation / promoted | idea-to-proposal proposal family + `skills/proposal-former` | Proposal family owns remaining candidate meaning. |
| `proposal-packet.md` | Implemented foundation / promoted | durable-proposal-packets proposal + `skills/proposal-packets` | Proposal family owns remaining candidate meaning. |
| `research-maturity-evidence-snapshots-and-staleness.md` | Retained / narrowed | `proposal-research-maturity-and-freshness.md` | Shared claim-lineage semantics are now a formed proposal and are referenced, not redefined. |
| `role-aware-agent-scheduler.md` | Partially implemented / remaining scope absorbed | `skills/role-scheduler + control-plane-and-client-protocol.md` | Scheduler prototype is current machinery; broader activation/client issues belong to control plane. |
| `role-decision-trace-and-work-dossier-v2.md` | Retained / narrowed | `role-decision-trace.md` | Decision lineage retained; dossier demoted to a possible Studio projection. |
| `seams-principles-review.md` | Retained / narrowed | `cross-cutting-seam-review.md` | Cross-boundary coherence only. |
| `slice-completion-commit-prompt.md` | Implemented / superseded | `skills/slice-completion-commit + supervisor completion-offer lifecycle` | Remove from active ideas. |
| `ui-review.md` | Retained / scoped | `ui-review-capability.md` | Project-facing UI review capability, separate from Work Engine Studio. |
| `work-engine-codex-runtime-adapter-boundary-analysis.md` | Retained / narrowed | `runtime-adapter.md` | Runtime binding/execution only. |
| `work-engine-control-protocol-and-environment-affordances.md` | Retained / narrowed | `control-plane-and-client-protocol.md` | Control-plane/client protocol only. |
| `work-engine-studio-design-control-forensics.md` | Retained / narrowed | `work-engine-studio.md` | Human-facing design/control/forensics surface only. |
| `ui-design-principles-structure.yaml` | Not an active Work Engine idea | `skills/ui-design-principles / project-specific source` | Structural draft for Site2JSON design principles; keep out of active Work Engine idea set. |
| `workflow.diff` | Not an idea | `none` | Historical diff between proposal-workflow drafts; remove from active idea set. |

## Important promoted/implemented boundaries

### Proposal system

The four proposal-packet / proposal-formation ancestor documents are no longer active future ideas. Current machinery exists in:

- `skills/proposal-packets`;
- `skills/proposal-former`;
- `proposals/idea-to-proposal-system`.

Future proposal research/evaluation/portfolio ideas remain separate consumers.

### Durable operational state

The broad persistent-agent-state direction has been promoted into:

- `proposals/agent-state/role-owned-durable-operational-state`.

Context replacement and reconstruction consequences remain owned by that formed
proposal. The clean organizational idea retains the distinct unresolved question
of how a reusable durable-role profile binds those consequences to applicable
roles without redefining them.

### Review system

Adaptive specialized review and revision-bound review artifact semantics are represented by:

- `proposals/adaptive-specialized-review/`.

The clean review-scope idea therefore owns only mutable-scope/applicability coordination.

### Scheduler / control plane

Durable scheduling is real current machinery in `skills/role-scheduler`.

The clean control-plane idea begins where that implementation stops: activation/leases, generalized routing, control packets, reconciliation, and client/environment interaction.

### Checkpoints and publication

Git-backed slice checkpoints and the completion-commit/prompt direction have current implementations in:

- `skills/slice-checkpoint`;
- `skills/slice-completion-commit`;
- supervisor-owned checkpoint/completion lifecycles.

They are not active future ideas.

## Recommended repository workflow

If this projection is adopted:

1. keep the original idea files in Git history or move them to an explicit historical/archive area;
2. replace the active `ideas/` surface with the clean set only after human review;
3. link promoted ideas to their proposal families instead of keeping duplicate current descriptions;
4. treat this reconciliation as semantic cleanup, not implementation authority;
5. repeat the reconciliation periodically as proposals are promoted and ideas become implemented.

The result should make `ideas/` answer one simple question:

> **What architecture or product possibilities are still unresolved?**
