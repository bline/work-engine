# External organization formation: proposed implementation plan

Status: accepted for bounded implementation by the external root coordinator on 2026-10-28 under the user's instruction to build a plan and start implementation. This is coordinator plan acceptance, not a claim of human approval of an exact generated artifact. This plan adds an organization-formation affordance to the external manual-orchestration skill used to build Work Engine. It does not change the internal Rust product.

The coordinator should be able to present a discovered, reviewable organization before dispatching implementation: explicit bounded outcomes, logical reporting relationships, requested model/effort/tier and launch choices, branch findings, uncertainty, and the useful discovery sessions proposed for continuation. Formation remains model judgment. A candidate artifact records a proposal; implementation and launch authority still come from the task's authority map.

## Smallest coherent change

Add `references/organization-formation.md`, conditionally loaded by coordinators and supervisors forming or revising an organization. Put role responsibilities and their reasons there: coordinator assembles the whole candidate and owns revision decisions; supervisors explore their branch and propose justified children; an implementer assigned discovery reports its bounded findings and whether it can own a coherent implementation outcome. A branch may remain a leaf. Do not prescribe depth, fan-out, a fixed sequence, or a role taxonomy beyond the existing manual roles.

Add `assets/organization-candidate.example.json`, a valid JSON example of the existing JSON-state plus Markdown-packet convention. This is a reviewable data form, without a new executable consumer, compiler, scheduler, or general team manager. JSON and Markdown fit the Rust direction's explicit data/document exception; no new executable mechanics are needed.

Add short, conditional links in `SKILL.md`, `references/coordinator.md`, `references/supervisor.md`, and `references/worker.md`. Add the new form to `references/demonstrated-forms.md` as an illustrative organization form, clearly distinct from that file's observed historical examples. Keep host, context, runtime scripts, pilot/profile paragraphs, model defaults, and the concurrent context-continuity text intact.

## Distinctions the reference and example must preserve

- **Candidate versus realization.** Give the organization and each logical role stable identities and a revision. Record the objective/authority references, bounded outcome and ownership, logical `reports_to`, evidence/uncertainty, proposed children, requested exact model/effort/tier and launch mechanism, and a proposed retain-or-fresh choice. A proposed role can be unlaunched. Link to existing assignment/continuation records for launched discovery roles; keep observed session IDs, rollout, configuration, runtime ancestry, and active collection ownership with that assignment record.
- **Discovery versus implementation.** Read-only formation launches need their own currently authorized discovery packets and explicit configuration. Each level explores its branch and proposes children only where the result justifies them. Discovery stops when the branch is defensibly assignable as a coherent outcome, more exploration has no credible decision value, or an explicit limit/authority boundary prevents further work; unresolved matters retain an owner. A reviewable candidate can disclose bounded uncertainty. It is not implementation permission.
- **Selection timing.** Select discovery configurations before discovery launches. Include explicit proposed implementation and review selections in the candidate, then check the active user/contract selection before dispatch. Preserve requested versus observed identity and tier. An unavailable or mismatched realization is unknown/unmet as applicable; organization formation supplies no silent model fallback. Refer to the model guide and host reference rather than copying their defaults into permanent instruction.
- **Useful continuity.** A discovery supervisor or implementer can continue into implementation when authorized and useful, after the existing scoped context check. Durable findings make the candidate independently reviewable; retained context adds operational value but is not the semantic owner. A fresh, separately launched reviewer remains separate from the discovery/designer/builder context.
- **Revisable organization.** Coordinator judgment classifies a proposed change as small or large and records its consequence and reason. Small changes require coordinator approval; large changes require human approval. These are the user's organization boundaries, not a closed change taxonomy. Existing provider, budget, mutation, review, and other explicit approval boundaries still apply even to a small change. Initial acceptance uses the active task's named approval owner rather than introducing a blanket human gate.
- **Exact approval provenance.** Freeze each candidate revision. In the existing assignment/decision record, bind a decision to the exact candidate path and digest, prior revision, change classification/basis, decision owner, and source approval/authority evidence. Keep the approval outside the candidate to avoid a self-referential digest. A later revision cannot silently consume the previous revision's approval; unchanged accepted portions may be carried with an explicit applicability reason. Candidate acceptance does not grant new task authority.
- **Logical versus runtime relationships.** A logical reporting change does not rewrite runtime ancestry or move a notification route. Any ongoing result/process collection retains a verified owner and delivery path. The existing host mechanism and context contracts govern any actual replacement or transfer.

## Implementation ownership and dispatch

Use one worker for the new reference/example and all five existing-file links; the semantics and role loading are one coherent outcome. The retained supervisor owns packet preparation, current-byte binding, validation, result collection, and ordinary remediation. Root owns acceptance/integration and the separate top-level Astra review.

Recommended worker selection: `gpt-6.1-sol` at `medium`, fresh `spawn_agent(fork_turns="none")`. The authority-sensitive cross-role instruction boundary makes the Sol builder recommendation appropriate. Proposed tier is the current root tree's observed `default`; recheck the active root tier before dispatch and record any deliberately selected change. No configuration toggle is part of this edit. The retained supervisor is verified Sol6.1/xhigh/default. The root-owned review is separate top-level `gpt-6-astra` at `xhigh`, explicitly `service_tier="default"`, because the subject changes an ownership/instruction contract.

Before writing, bind the current affected bytes and reconcile the overlapping SKILL/coordinator additions with the context-continuity owner. Do not overwrite a whole file from HEAD or from the released skill. No source changes or child launches occurred during this reconnaissance.

## Validation and acceptance evidence

Parse the JSON example, check changed Markdown links and skill packaging using existing tooling, and inspect the exact diff for concurrent-byte preservation. Do not add tests that assert prose wording. Exercise observable decisions against the finished form and reference:

1. A proposed, unlaunched role has explicit requested selection and no invented observed session; discovery acceptance alone cannot authorize source changes.
2. A branch can remain a justified leaf or propose children; neither a fixed depth nor automatic expansion is required.
3. A useful discovery role continues with its verified binding and existing context check, while independent review starts separately.
4. A small revision is coordinator-approved against its exact revision; a large revision remains pending human approval. A small label cannot waive an existing authority boundary.
5. An old approval cannot approve changed scope/model/organization bytes; an unavailable model does not trigger silent substitution.
6. Logical reparenting preserves actual runtime ancestry and active collection ownership; no child or process is stranded.
7. The reviewer has an explicit default-tier request and separately observed realization; proposal data alone cannot establish the latter.

Use this task as one bounded actual example: root assembles its candidate from this discovery result, links this real supervisor's assignment, and leaves the implementer/reviewer proposed until launched. After plan acceptance and the pre-dispatch check, retain this supervisor for implementation supervision. The worker's result and the root's actual bindings demonstrate realization; scenario evaluations establish only their stated decision cases.

Run the final instruction subject through the agent-instruction-review contract in the root's separate Astra review, covering authority scope, loading, semantic ownership, causal reasons, and whether any route has been promoted into an unjustified invariant. Source validation is not a global skill release; root retains any release decision.

No new architectural owner or executable consumer remains unresolved. Root acceptance, the dispatch-time tier choice, and the concurrent owner's file-level integration response are the remaining pre-implementation decisions.

## Coordinator acceptance

Root `01a18339-409a-7d72-be0f-4d96cd7fe4a6` accepts this skills-and-data scope. The seven source paths are task-owned additions only; pre-existing context-continuity edits remain owned by their session. Shared-file links must be additive and preserve current bytes. The supervisor may start the new reference/example and non-overlapping role sections while root continues overlap coordination. No new executable code, Rust runtime work, campaign transition, commit, or deployment is authorized by this plan. Review must include a separate Astra session and actual useful supervisor continuity from reconnaissance through implementation supervision.
