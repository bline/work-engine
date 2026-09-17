### Planner-owned execution characterization

```yaml
idea_status:
  architectural_supersession: none
  architectural_supersession_note: "Checked 2026-09-17, per review: previously none with no note. Semantic check performed: searched semantic-planning-hierarchy.md, material-decision-selection.md, and implementation-contract-compilation.md for any execution-characterization or planner-owned-classification concept under any name -- none state it. Genuinely none, not unknown, since this 85-line document was fully close-read and the plausible-owner check was real."
  residue: present
  residue_ledger: "Corrected 2026-09-17, per review: a free-form live architectural question found in prose (outside any formal ledger) -- whether a separate execution-characterization classification role should ever be introduced, explicitly conditioned on future pilot evidence not yet gathered. Tagged inline [KIND: RESIDUE] [OPEN]."
  backlog: none
  backlog_note: "No dedicated staged-plan section or open-question ledger; this is a short (85-line) design-hypothesis addendum, fully consumed by structural-plan-ir-for-capability-aware-multi-model-execution.md's own pilot design. The one open item above is RESIDUE-kind (architectural placement, contingent on future evidence), not BACKLOG-kind (operative construction), so it does not change this axis."
  audit_scope:
    - keyword-scan: full_document
    - close-read: "full document (85 lines)"
  audit_scope_completeness: complete
  status_as_of: 2026-09-17
```

```yaml
idea_provenance:
  origin: direct_capture
  related_reconciliations:
    - structural-plan-ir-for-capability-aware-multi-model-execution.md
```

Execution-profile characterization should ordinarily be produced as a derived output of implementation planning rather than reconstructed later by a separate classification role.

The planner already possesses the repository evidence and semantic understanding required to judge properties such as:

- semantic novelty;
- remaining implementation discretion;
- repository and ownership breadth;
- unresolved material ambiguity;
- dependency complexity;
- discovery burden;
- oracle strength;
- reversibility; and
- integration or migration consequence.

Producing these characterizations during planning should therefore require only incremental inference over reasoning the planner has already performed.

The planner does **not** select the executor or assign an arbitrary aggregate difficulty score. It emits an attributable characterization of the planned work.

Conceptually:

```text
planning reasoning
      |
      +----> canonical Plan IR
      |
      +----> derived execution characterization
                     |
                     v
              deterministic scoring
                     |
          historical capability evidence
                     |
                     v
             strategy admission/routing
```

This preserves separate ownership:

- **Plan IR** owns the structural representation of implementation meaning.
- **The planner** owns semantic characterization derived from its planning judgment and evidence.
- **The scoring mechanism** deterministically transforms characterized properties according to a versioned scoring policy.
- **Capability evidence** records demonstrated executor performance.
- **Runtime admission** determines which currently available strategy can realize the selected execution requirements.
- **The appropriate authority owner** resolves any remaining material tradeoff not determined by policy.

Execution characterization should be treated as derived, revision-bound metadata rather than canonical implementation meaning. Its schema may evolve as experiments reveal which dimensions actually predict executor success, required Plan IR resolution, review burden, and total accepted-work cost.

This distinction allows historical plans to be re-evaluated under later scoring policies or model-capability evidence without rewriting the original plan or its original characterization.

For example:

```text
Plan revision P17
    |
    +-- original characterization E4
    |
    +-- scored under routing policy R3
    |       -> Sol / P1
    |
    +-- later rescored under routing policy R9
            + new Luna capability evidence
            -> Luna / P2 candidate
```

The historical planning judgment remains attributable. New evidence changes the derived routing conclusion rather than historical meaning.

### Pilot implication

The first pilot should measure the incremental inference required for the planner to emit the execution characterization separately from the cost of producing the Plan IR itself.

This tests an important economic premise:

> Execution characterization is useful only if capturing it while planning is substantially cheaper than reconstructing equivalent semantic understanding later.

The pilot should therefore record:

- planner tokens before execution-characterization output;
- incremental characterization tokens where observable;
- disagreement between planner characterization and observed execution difficulty;
- which dimensions required genuine additional investigation rather than reuse of existing planning understanding; and
- whether any characterization dimension materially improved strategy prediction.

**[KIND: RESIDUE] [OPEN — found 2026-09-17 by review, previously recorded as prose without a formal tag: whether this role should ever exist is explicitly gated on future pilot evidence, not decided now.]** A separate classification role should be introduced only if evidence shows that planner-produced characterization is systematically biased, unstable, or too costly.
