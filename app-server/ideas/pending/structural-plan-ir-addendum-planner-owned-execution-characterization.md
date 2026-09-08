### Planner-owned execution characterization

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

A separate classification role should be introduced only if evidence shows that planner-produced characterization is systematically biased, unstable, or too costly.
