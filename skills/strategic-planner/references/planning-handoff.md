# Strategic planning handoff

The planning handoff carries the durable consequence of a strategic review
back to a campaign supervisor or human decision owner. It is not a campaign
receipt, an implementation plan, a roadmap patch, or a reasoning transcript.

Use this compact version-1 shape. Omit empty optional collections only when the
consumer does not require them; never omit an unresolved authority need.

```yaml
schema_version: 1
strategic_objective: "Keep the standalone Work Engine route coherent"
evidence_cutoff:
  roadmap_revision: "git revision or content digest"
  repository_revision: "git revision"
  campaign_terminals:
    - {run_id: "...", slice_number: 14, status: accepted}
continuity: initialized | retained | reconstructed
verdict: continue | revise | pause | reorder | split_campaign | stop_campaign
current_rationale: "Why this verdict best serves the objective now"
assumptions:
  confirmed: []
  changed: []
  invalidated: []
route_changes:
  priorities: []
  dependencies: []
  newly_important: []
  deferred: []
recommended_campaign:
  disposition: continue_current | amend_current | start_new | none
  objective: null
  work_source: null
  reason: "..."
open_uncertainties: []
authority_required: []
revisit_when: []
```

Every nonempty assumption or route-change entry should name the evidence that
supports it and its strategic consequence. References may point to durable
receipts, revisions, roadmap sections, proposal packets, or observed product
state. Do not embed raw transcripts, logs, diffs, prompts, or source excerpts.

The verdict is advice within the planner's authority. A supervisor must not
silently convert it into a configuration amendment, new campaign, roadmap
mutation, or user approval. If the recommendation changes the active campaign
objective or work-source boundary, stop before launching another slice and use
the owning amendment or authorization path.

`continuity: initialized` means this is the first planning pass and no prior
planner context or handoff exists. `retained` means the same planner identity
preserved useful context. `reconstructed` means a later planner rehydrated from
a prior handoff and other durable sources. All three require a fresh evidence
cutoff; none makes remembered product state authoritative.
