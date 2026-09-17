# Idea: Builder-Side Candidate Trajectory Consumption

**Status:** Conceptual direction; not accepted, prioritized, or authorized for
implementation. Depends on [Candidate Trajectory — Durable Foundation](candidate-trajectory-durable-foundation.md)
and is a larger, more speculative surface than
[the reviewer-side consumer idea](candidate-trajectory-remediation-delta-for-native-review.md).

**Scope:** How a builder, during remediation and later gate cycles within the
same slice/attempt, could consume Candidate Trajectory facts instead of
reconstructing them from its own conversational memory, and how that might
eventually shrink the semantic-vs-mechanical mix in the builder's
candidate-ready projection.

**Authority:** Exploratory only. This document does not amend the migration
roadmap, admit an implementation, select a builder contract change, or
authorize implementation.

```yaml
idea_status:
  architectural_supersession: not_applicable
  architectural_supersession_note: "Checked 2026-09-17, per review: no plausible canonical owner exists to compare against, so this needs no additional semantic-coverage check to earn none rather than unknown -- Candidate Trajectory (and this consumer of it) is App Server slice-campaign implementation detail, an unaccepted proposal for a checkpoint-diffing primitive internal to that service, not the kind of content any of the 13 dimensions, 5 mechanisms, or 2 substrates tracks. Consistent with how the rest of the Candidate Trajectory family and similar migration-tooling ideas were classified this session."
  residue: none
  backlog: none
  backlog_note: "SS3 states a general compare-before-retiring discipline, not a dedicated staged plan with named stages; no PLAN disposition applies."
  audit_scope:
    - keyword-scan: full_document
    - close-read: "SS1-6"
  audit_scope_completeness: complete
  status_as_of: 2026-09-16
```

```yaml
idea_provenance:
  origin: direct_capture
  related_reconciliations:
    - candidate-trajectory-durable-foundation.md (upstream dependency, itself architectural_supersession: none)
```

## Summary

Candidate Trajectory (once it exists) is bound from an immutable candidate
checkpoint, so it cannot help the very first implementation pass before C1
exists. But once C1 exists, every later remediation turn has a builder that
just produced a change, is about to be told what a reviewer found, and will
have to reason about what its own repair actually did structurally. That
reasoning currently happens entirely inside the builder's own context. This
idea explores handing the builder deterministic, host-owned facts about its
own candidate-to-candidate trajectory instead, so the builder's context is
spent on semantic judgment rather than remembering and re-describing the
physical consequences of its own edits.

This is explicitly framed as more speculative and higher-risk than the
reviewer-side idea: it touches the shape of the builder contract itself, not
just an additive context section, and several of its uses require empirical
validation before any of them could reasonably replace something the builder
currently reports.

## 1. Candidate uses

Each of these is a separate, independently useful application of the same
underlying trajectory facts. None require adopting the others.

- **Deterministic remediation context.** When a builder receives C1 back for
  repair, give it the C1 physical profile; once C2 exists, give it the C1→C2
  trajectory delta. The builder no longer reconstructs "what did my repair
  actually spread into?" from memory.
- **Boundary-drift detection.** Compare the accepted slice boundary /
  attributed manifest against the physical profile and later candidate deltas.
  If remediation crosses new modules or introduces new generated/validation
  dependencies, surface that mechanically as an observation (e.g.
  `observed_boundary_change: {new_modules: [...], new_validation_dependency: ...}`)
  — not as a conclusion like "scope violation," which stays semantic/policy
  territory for the builder or supervisor to judge.
- **Targeted validation selection (Candidate Trajectory + validation/dependency
  mapping — not Trajectory alone).** Some of what a builder currently reasons
  about when choosing focused checks could become deterministic from changed
  file categories, modules, and trajectory deltas, but file categories and
  modules alone do not establish which tests are implicated by a given path.
  This use needs an additional deterministic test/dependency relationship map
  (existing or newly built) as a second input; Candidate Trajectory supplies
  the "what changed" half, not the "what does that imply for validation" half.
  Code can propose or require the obviously-implied checks once both inputs
  exist; the builder still judges semantic sufficiency.
- **Remediation amplification warning.** If fixing one finding caused C1→C2 to
  add several modules, paths, and repeated symbol changes, hand the builder
  that fact directly instead of it discovering the amplification the hard way.
- **Candidate convergence feedback.** Across several repairs, tell the builder
  the raw per-transition surface deltas — not a semantic label like
  "converging" or "diverging," which is itself a classification this document
  otherwise argues against making at the trajectory layer. Concretely:

  ```text
  candidate surface:
  C1: 2 modules / 7 paths
  C2: 4 modules / 11 paths
  C3: 3 modules / 9 paths

  new surface by transition:
  C1->C2: +2 modules / +4 paths
  C2->C3: -1 module / -2 paths
  ```

  Let another owner (builder judgment, or supervisor policy) decide whether
  that pattern constitutes convergence, divergence, or neither.
- **Replacement/escalation packets.** If a builder is replaced mid-slice, its
  state packet can include exact physical candidate and trajectory facts
  instead of prose reconstruction — these facts survive context replacement
  without interpretation.

## 2. The longer-term contract question

The builder's candidate-ready projection currently mixes mechanical facts
(paths, categories, structural surface, integrity identities) with genuine
semantic judgment (changed authority, ownership, lifecycle, interface
consequence, unresolved semantic uncertainty). Once Candidate Trajectory and
the underlying physical profile are durable and host-derived, the mechanical
half becomes something the host can establish independently of what the
builder reports. In principle this could narrow the projection over time
toward something closer to:

```text
Host-owned:
  checkpoint identity, exact paths/attribution, patch identity,
  file categories, modules/symbols where supported,
  validation/config/docs/test surface,
  candidate-to-candidate structural delta, repeated modifications,
  generated/validation dependency involvement

Builder-owned:
  changed semantic contract, ownership/authority/lifecycle/interface
  consequences, unresolved semantic uncertainty
```

This document does **not** propose adopting that narrower contract now. It
records the direction as a second-order consequence worth tracking, gated
entirely on the migration approach in the next section.

## 3. Required migration discipline: compare before retiring anything

None of section 1's uses, and none of section 2's contract narrowing, should
remove an existing builder reporting obligation on the strength of this
document alone. The applicable pattern, already used elsewhere in this
codebase's thinking about builder-vs-host fact ownership: run the host-derived
fact and the builder-reported fact side by side, measure agreement, and only
retire the builder's obligation once equivalence is demonstrated across a
real sample of slices. A single field disagreeing (e.g. the builder's own
manifest claim and the host-derived structural facts disagreeing about which
modules were touched) is itself useful migration evidence, not just noise —
it may indicate either a profiler gap or a builder misreport, and both are
worth knowing before either side is trusted exclusively.

This is materially different from, and larger than, the reviewer-delta idea's
evaluation step: that idea adds one additive context section and asks whether
it helps; this idea contemplates eventually removing a builder reporting
obligation, which is a much higher bar and should require production-scale
agreement evidence, not just a historical replay comparison.

## 4. Explicit anti-goal: no live mutable-worktree profiling

Do not continuously run the canonical profiler against the builder's mutable
worktree to give it a live "complexity score" mid-turn. That would blur the
clean checkpoint-bound evidence model Candidate Trajectory depends on (every
observation is bound to an immutable candidate) and could cause a metric to
steer implementation prematurely, before a candidate is even checkpointed. If
a cheap mutable-worktree advisory signal is ever wanted, it must be explicitly
non-authoritative and disposable, and is out of scope for this document.

## 5. Non-goals

- This does not define the exact schema of any host-owned observation beyond
  what [Candidate Trajectory](candidate-trajectory-durable-foundation.md)
  already proposes (structural delta fields, coverage/limitations discipline).
- This does not authorize any change to the builder contract, the candidate-
  ready projection schema, or supervisor routing.
- This does not claim any of section 1's uses reduces cost or improves quality
  without the comparison evidence in section 3.

## 6. Relationships

| Direction | Relationship |
| --- | --- |
| [Candidate Trajectory — Durable Foundation](candidate-trajectory-durable-foundation.md) | Owns the durable facts this idea proposes to surface to the builder. |
| [Remediation delta for native review](candidate-trajectory-remediation-delta-for-native-review.md) | Sibling consumer; independent implementation, shares only the underlying primitive. |
| Builder contract / candidate-ready projection (wherever currently specified) | Target of the longer-term narrowing discussed in section 2; this document proposes no change to it. |
| [Deterministic Refactor Pressure](deterministic-refactor-pressure-from-work-engine-evidence.md) | Independent longitudinal consumer of the same primitive; not a dependency of this idea. |

This document does not own the builder contract, supervisor routing, or
validation policy. It proposes candidate uses and the evaluation discipline
required before any of them could responsibly replace existing builder
obligations.
