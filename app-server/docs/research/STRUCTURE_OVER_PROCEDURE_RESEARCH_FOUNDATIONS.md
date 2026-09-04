# Structure Over Procedure: Research Foundations

## Status

Explanatory research note. This document is **not normative** and does not define Work Engine behavior.

### Provenance

The structural design stance described here—pin invariants, authority, and evidence; leave route and sequencing to judgment—was derived independently from direct engineering experience building Work Engine. The cited literature was located afterward to compare the resulting architecture against existing systems-design traditions and contemporary agent research, and to identify convergence, disagreement, and limits.

This document is therefore a **retrospective research foundation**, not a claim that the architecture was derived from mechanism/policy separation, least-privilege, design-by-contract, or later agent literature. It is not a systematic literature review.

Its purpose is to explain the research traditions and direct agent evidence that converge with a structural rather than unnecessarily procedural approach to agent-system design.

**Companion note:** `BEHAVIORAL_ENVIRONMENTS_RESEARCH_FOUNDATIONS_REVISED.md` examines the independent empirical question of how interaction context, social framing, and agent scaffolding affect expressed model behavior.

## Thesis

A recurring systems-design principle is to specify the distinctions that must remain true while avoiding unnecessary commitment to one implementation policy or execution path.

For agent systems, this motivates a design stance in which the system strongly defines:

- desired and invalid outcomes;
- invariants and acceptance conditions;
- authority and ownership boundaries;
- observable evidence and provenance;
- available mechanisms and capabilities;

while leaving route, sequence, trigger choice, and tuning to runtime judgment unless correctness depends on them.

This is not a claim that procedure is inherently bad. Procedure is appropriate when ordering or route is itself causal to correctness. The concern is **unnecessary procedural commitment**.

## Research lineage

### 1. Mechanism versus policy

One of the clearest ancestors is the operating-systems distinction between **mechanism** and **policy**.

Per Brinch Hansen's RC 4000 work sought a small nucleus that supplied fundamental mechanisms while allowing higher-level systems to choose scheduling, resource-allocation, and other policies. His 1970 paper, *The Nucleus of a Multiprogramming System*, describes a system designed to be extended for diverse requirements rather than hard-wiring one operating model.

The HYDRA work at Carnegie Mellon made the principle explicit. Wulf and colleagues argued that failure to separate mechanisms from policies restricts experimentation and adaptation, and that mechanisms should preserve flexibility for higher-level policy decisions.

This maps closely to the agent-system distinction between:

- **what machinery exists**, and
- **which route a reasoning component should choose with that machinery**.

The connection does not imply that an LLM agent is an operating-system scheduler. It shows that preserving decision freedom above a stable mechanism layer is an old and durable systems principle.

### 2. Protection, authority, and least privilege

Saltzer and Schroeder's 1975 *The Protection of Information in Computer Systems* described architectural principles including fail-safe defaults, complete mediation, separation of privilege, least privilege, least common mechanism, and economy of mechanism.

These principles support a second part of the structural model: freedom of judgment does not mean unconstrained authority.

An agent can be given broad latitude over **how** to pursue an objective while still operating inside explicit limits on:

- what it may observe;
- what it may mutate;
- which authority it possesses;
- which transitions require mediation;
- what evidence must exist before an outcome is accepted.

This separates **decision freedom** from **authority freedom**. They are not the same thing.

### 3. Preconditions, postconditions, and invariants

C. A. R. Hoare's 1969 work on axiomatic program reasoning formalized correctness in terms of assertions surrounding computation: if a precondition holds before a program executes, a specified postcondition should hold afterward.

Bertrand Meyer's later *Design by Contract* work made preconditions, postconditions, and invariants a practical software-design methodology.

The relevance to agent systems is structural rather than literal. When the exact route cannot or should not be prescribed, correctness can instead be anchored in properties that must hold:

- before action;
- throughout action;
- at completion.

This supports outcome-oriented specification without requiring the system designer to encode every valid route to the outcome.

### 4. Flexibility through bounded structure

Across these traditions, a common pattern appears:

1. identify the distinctions correctness depends on;
2. encode those distinctions in stable structures or contracts;
3. expose mechanisms capable of realizing multiple valid policies;
4. keep higher-level choice outside the mechanism when the choice does not belong there.

The historical systems literature generally places that higher-level choice in another program, administrator, or system designer.

Agent systems introduce an unusual case: a **general-purpose semantic reasoner can itself be a runtime component**. This changes who may exercise the higher-level choice, but it does not erase the older structural principle.

## Direct evidence from agent orchestration

The historical systems literature establishes structural precedent, but it was not written for modern language-model agents. A more direct contemporary result comes from Lim et al. (2026), *Declarative Skills for AI Agents in Knowledge-Grounded Tool-Use Workflows*.

The study compares three agent configurations in the same task setting:

- an unscaffolded baseline;
- a **declarative** agent given natural-language skills while retaining control over its own action flow;
- an **imperative** agent constrained by an explicit phase-oriented orchestration structure.

Under high-quality retrieval, the declarative configuration improved over baseline on four of five tested models and tied on the fifth, while the imperative configuration underperformed the baseline on every tested model.

The authors frame the imperative design as restricting the available policy class by constraining which actions are available in each phase. Their empirical results are consistent with the proposition that externally forcing a route can reduce agent performance when that route restriction is not itself required for correctness.

This result does **not** establish the full Work Engine doctrine. It tests a narrower orchestration problem in a knowledge-grounded workflow domain, and retrieval quality materially affects the result. Under noisier retrieval, the declarative advantage weakens.

Its relevance is therefore specific:

> **Declarative guidance that preserves model control over route selection can outperform an externally phase-gated orchestration design using the same general task setting and model family.**

This is direct agent-specific convergence with the structural concern described in this document: when correctness does not require a particular route, unnecessary route restriction can remove useful decision freedom.

## Agent-system synthesis

A useful agent-system reduction is:

> Define the desired and invalid outcomes, authority boundaries, and observable evidence without embedding a preferred solution. Derive only the structure required to preserve those distinctions, then expose enough composable machinery to make valid outcomes reachable and observable. Leave route, sequence, trigger mechanisms, and tuning to judgment unless causality makes them part of the contract.

Short form:

> **Define the space, not the solution. Expose the machine, not the route.**

This formulation is a synthesis for agent systems, not a claim that the component ideas are novel.

## Why this matters for agents

A conventional deterministic component usually needs its decision procedure implemented in advance.

A reasoning agent is different: semantic route selection may be part of the capability being purchased from the model. If the surrounding architecture encodes every route decision procedurally, the system can accidentally remove the very degrees of freedom that make the reasoning component useful.

The design problem therefore becomes:

> Which decisions are part of the contract, and which decisions are legitimate runtime judgment?

A practical test is:

- If changing the route can violate correctness, the route may belong in the contract.
- If many routes preserve all protected outcomes and authority boundaries, choosing one route is likely judgment or policy rather than invariant structure.

## Relationship to Work Engine design provenance

The relevant lineage is:

```text
direct engineering observation
    -> produced structural design choices

structural design choices
    -> shaped Work Engine architecture

historical systems literature
    -> converges_with those choices

contemporary agent experiments
    -> support_part_of / complicate those choices

literature
    != derived_from relationship for the original design
```

The literature therefore serves as **retrospective corroboration, challenge, and context**, not as the historical source of the architecture.

## Limits of the claim

This research lineage does **not** establish that:

- open-ended agents always outperform procedural agents;
- every policy decision should be delegated to a model;
- semantic correctness can always be mechanically verified;
- structure can replace validation, review, or governance;
- older mechanism/policy literature directly anticipated LLM agents.

The narrower claim is that agent-system design inherits a long-standing systems problem: **how to preserve necessary constraints without prematurely fixing legitimate choices**.

## References

- Brinch Hansen, P. (1970). *The Nucleus of a Multiprogramming System*. Communications of the ACM, 13(4), 238–241, 250. https://doi.org/10.1145/362258.362278
- Wulf, W. A., Cohen, E., Corwin, W., Jones, A., Levin, R., Pierson, C., & Pollack, F. (1974). *HYDRA: The Kernel of a Multiprocessor Operating System*. Communications of the ACM, 17(6), 337–345. https://doi.org/10.1145/355616.364017
- Levin, R., Cohen, E., Corwin, W., Pollack, F., & Wulf, W. (1975). *Policy/Mechanism Separation in Hydra*. ACM SIGOPS Operating Systems Review, 9(5), 132–140. https://doi.org/10.1145/1067629.806531
- Saltzer, J. H., & Schroeder, M. D. (1975). *The Protection of Information in Computer Systems*. Proceedings of the IEEE, 63(9), 1278–1308. https://doi.org/10.1109/PROC.1975.9939
- Hoare, C. A. R. (1969). *An Axiomatic Basis for Computer Programming*. Communications of the ACM, 12(10), 576–580, 583. https://doi.org/10.1145/363235.363259
- Meyer, B. (1992). *Applying “Design by Contract”*. Computer, 25(10), 40–51. https://doi.org/10.1109/2.161279
- Lim, et al. (2026). *Declarative Skills for AI Agents in Knowledge-Grounded Tool-Use Workflows*. arXiv:2606.06923. https://arxiv.org/abs/2606.06923
