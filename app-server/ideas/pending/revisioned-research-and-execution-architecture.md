# Revisioned Research and Execution Architecture

## Status

Conceptual reference document.

This document describes an architectural consequence emerging from several Work Engine directions taken together. It is not a proposal, accepted architecture direction, implementation plan, runtime contract, or claim that the described system already exists in complete form.

Its purpose is to make the emerging concept explicit enough to reason about, test, and relate to the rest of Work Engine without prematurely turning it into normative product structure.

The central observation is:

> **If the state required for model judgment is externally owned, revisioned, and reconstructable, then a meaningful point in an agent workflow can become a recovery coordinate and a candidate fixture for reproducible experiments.**

The second consequence requires more than reconstruction. A coordinate can own
the starting world; a separately governed experiment specification must own the
comparison, controls, outcomes, and authority that make a research conclusion
valid.

## Contributing directions and distinct owners

The concept draws consequences from several pending directions without
accepting them or collapsing their semantic ownership:

| Contribution | Pending sources | Relationship to this concept |
| --- | --- | --- |
| Revisioned product meaning and authority | [Architecture directions](../ideas/pending/authority-backed-architecture-directions-as-workflow-inputs.md), [decision-gated implementation compilation](../ideas/pending/proposal-decision-gated-implementation-compilation.md), and [architecture intake](../ideas/pending/incremental-architecture-intake-and-seam-reconciliation.md) | Supply attributable semantic, decision, adoption, transition, and prospective-architecture revisions. |
| Role-local projections and execution structure | [Hierarchical orchestration](../ideas/pending/hierarchical-planning-and-multi-supervisor-orchestration.md) and [Structural Plan IR](../ideas/pending/structural-plan-ir-for-capability-aware-multi-model-execution.md) | Supply role-specific causal context and multiple projections over one canonical implementation contract. |
| Exact executable realization | [Provider/harness/projection ports](../ideas/pending/provider-turn-harness-runtime-and-operator-projection.md) and [materialized runtime realization](../ideas/pending/pre-indexed-capability-resolution-and-frozen-runtime-realization.md) | Supply attributable runtime composition, capability grants, ownership, invalidation, and admission boundaries. |
| Recovery pressure and effect custody | [Context-replacement emergency recovery](../ideas/pending/context-replacement-emergency-and-writable-operator-ingress.md) | Exposes why context reconstruction requires fencing, queued-input custody, effect reconciliation, and truthful failure. |
| Experimental governance | [Governed-agent pilot specifications](../ideas/pending/pilots/adapter-representation-effect-pilot.md) and their [research protocol](../ideas/pending/pilots/prior-art-research-round-2-plan.md) | Supply the controls that turn a reconstructable fixture into an interpretable experiment. |
| Revisioned external-world evidence | [AI-accessible browser](../ideas/pending/AI_ACCESSIBLE_BROWSER_DESIGN.md) | Supplies a precedent for environment branches, evidence revisions, projections, coverage, and non-reproducible external state. |

The pending Claude runtime and OpenCode substrate work supply candidate
realization evidence. They do not own the architecture above them.

Work Engine currently develops this architecture through software engineering workflows. The underlying concept may prove more general, but coding and engineering remain the present proving ground.

---

# 1. From Agent Session to Revisioned Execution Environment

Conventional agent systems often treat the active model conversation as the practical owner of substantial execution state.

The model context may implicitly carry:

- what the task means;
- what has already been decided;
- which evidence matters;
- what the agent is allowed to do;
- which tools are available;
- what prior attempts failed;
- what assumptions remain active;
- what role the model is currently occupying; and
- what should happen next.

This makes the conversation unusually important.

If the context is lost, corrupted, truncated, replaced, or bound too tightly to one provider runtime, substantial reasoning may have to be reconstructed.

Work Engine has been moving in the opposite direction.

Increasingly, durable system state owns distinctions such as:

- logical role identity;
- role contracts and authority;
- accepted objectives and invariants;
- capability ceilings and admitted capability grants;
- provider and harness realization;
- proposal and decision revisions;
- implementation-plan revisions;
- evidence and provenance;
- workflow state;
- review findings;
- continuation and reconciliation state; and
- historical execution records.

The model context then becomes a **working set over owned state**, not the sole
owner of that state. Part of the working set is a role-scoped projection. Part
may be an uncommitted write buffer containing in-flight judgment, candidate
evidence, and intended actions that have not crossed an admission boundary and
cannot necessarily be regenerated from the durable store.

That distinction matters most during recovery. A tool effect may already be
committed to the external world while its meaning remains uncommitted to Work
Engine state. Treating all context as a derivable view would hide precisely the
frontier that recovery must reconcile.

Conceptually:

```text
durable Work Engine state
        |
        v
role-scoped projection
        |
        v
context working set
        |
        v
judgment / execution
        |
        v
candidate evidence and state
        |
        v
owned admission / reconciliation
        |
        +------------------> durable Work Engine state
```

This changes the meaning of an agent session.

The session becomes one realization of a logical role at a particular point in a durable execution history.

Model output acquires authority only through the boundary owned by the state it
would change. Before admission it may re-enter later reasoning as revisable
evidence or remain uncommitted, but it does not become authoritative merely by
appearing in context. Conversely, state already owned by the system should be
read from its owner rather than reconstructed from a transcript, model report,
or other replica.

---

# 2. Reproducible Reasoning Coordinates

The emerging abstraction is a **reproducible reasoning coordinate**.

The term does not mean that Work Engine can reproduce a model's private internal reasoning or hidden chain of thought.

It means that Work Engine may be able to reconstruct the externally owned state from which the model's next judgment is made.

A coordinate may eventually identify enough state to reconstruct:

```text
workflow lineage
+
branch identity
+
historical position
+
logical role identity
+
role contract revision
+
authority state
+
applicable architecture direction revisions
+
semantic artifact revisions
+
accepted decisions
+
evidence cutoff
+
world-state basis and coverage
+
instruction and configuration revisions
+
capability grant
+
runtime realization
+
context / projection revision
+
effects and reconciliation frontier
```

The exact coordinate representation remains an implementation question.

The important property is that a coordinate identifies a **decision
environment** sufficiently complete for the recovery, forensic, or research
claim that will rely on it, including what is known not to have been captured
or reconstructed. Coordinate adequacy is therefore claim-relative and belongs
in admission, not in an intrinsic fidelity label attached to the coordinate.

Conceptually:

```text
coordinate C
    |
    v
project(role R, C)
    |
    v
bounded decision environment
    |
    v
model / harness realization
    |
    v
next judgment
```

If that projection is sufficiently faithful, a role need not preserve one uninterrupted model context forever in order to retain logical continuity.

The durable system preserves the relevant world.

The model inhabits a temporary projection of that world.

---

# 3. Reconstructing the World, Not the Model

This distinction is fundamental.

Work Engine should not claim:

```text
reconstruct coordinate C
    =
reconstruct exactly what the prior model was thinking
```

That is neither required nor generally available.

The intended claim is narrower:

```text
reconstruct coordinate C
    =
reconstruct the externally owned conditions
under which the next authorized judgment occurred
```

Those conditions may include:

- the objective;
- the role;
- the authority envelope;
- relevant accepted architecture;
- plan or topology revision;
- current evidence;
- unresolved judgments;
- prior durable decisions;
- rejected or invalidated routes;
- admitted capabilities;
- available runtime mechanisms; and
- required downstream consequences.

A successor model may reason differently.

That difference is not necessarily reconstruction failure.

It may be precisely the phenomenon being investigated.

---

# 4. Durable History as the Basis of Reconstruction

Reconstruction requires a durable history that does not depend upon one model context remaining alive.

A useful conceptual model is:

```text
event / artifact history
        |
        +-- accepted decisions
        +-- evidence
        +-- plan revisions
        +-- realizations
        +-- findings
        +-- invalidations
        +-- transitions
        +-- checkpoints
        |
        v
historical coordinate
        |
        v
derived role projection
```

The durable record should preserve the consequences needed to reconstruct the relevant state.

It does not require preserving every reasoning transcript as canonical truth.

This follows the existing Work Engine principle:

> Preserve the consequence of reasoning, not necessarily the reasoning transcript.

For reconstruction, the important durable facts are things such as:

```text
route X was attempted
because premise Y made it admissible
observation Z invalidated Y
therefore X is stale under the established conditions
```

Later reasoning should not need to reconstruct the entire original investigation merely to understand that consequence.

---

# 5. Branching Instead of Rewriting History

Reconstruction from an earlier coordinate introduces a historical problem.

If execution resumes from a prior point, the system must not pretend that the original continuation never occurred.

The natural consequence is branching.

```text
C0 -> C1 -> C2 -> C3 -> C4
              \
               C3b -> C4b -> C5b
```

The original branch remains historical truth.

The successor branch records another possible continuation from the same predecessor coordinate.

This preserves several important properties:

- historical evidence is not rewritten;
- failed execution remains inspectable;
- replacement roles do not erase predecessor behavior;
- alternative decisions can coexist;
- recovery remains attributable; and
- experimental continuations do not contaminate canonical execution history.

Branching therefore supports both operational recovery and research.

---

# 6. Recovery as the First Consequence

The most immediate use of reproducible coordinates is role recovery.

Suppose a builder becomes unusable because of:

- context corruption;
- runtime failure;
- provider loss;
- model degradation;
- context replacement failure;
- operator-directed restart; or
- another condition that makes continued execution unreliable.

A successor does not need an improvised summary of the entire session. It does,
however, need proof that effects after the chosen coordinate have been
reconciled and that the predecessor can no longer act under the same logical
authority.

Conceptually:

```text
last trusted coordinate C
        |
        v
reconcile later and uncertain effects
        |
        v
fence or establish predecessor termination
        |
        v
preserve queued input and unresolved authority
        |
        v
derive successor projection
        |
        v
admit successor realization
        |
        v
instantiate replacement builder
        |
        v
continue on successor branch
```

The replacement role may use:

- the same model and harness;
- a different model;
- a different provider;
- a different harness;
- a revised capability realization; or
- a different context projection,

subject to the relevant authority and admission rules.

Logical role continuity therefore becomes separable from provider-session continuity.

The coordinate and the recovery frontier are related but distinct. A trusted
coordinate identifies the state from which meaning can be reconstructed. The
recovery frontier identifies later tool effects, messages, approvals,
cancellations, leases, and observations that must be reconciled before a
successor may safely continue. Replaying from the coordinate without the
frontier could duplicate an external effect or leave two realizations acting as
one logical role.

The same frontier also identifies consumption. Replacement restores a required
production-path claim only when invalid or uncertain outputs from the discarded
subject can still be fenced, discarded, repaired, or replaced before they cross
into the state covered by that claim. Which crossing matters depends on the
claim's exact subject, covered state, consumption boundary, and consumer.

---

# 7. The Larger Consequence: Historical Coordinates Become Experimental Fixtures

Once a historical coordinate can be reconstructed for recovery, the same
mechanism can supply a fixture for controlled experimentation.

A coordinate from real engineering work may be replayed into several isolated branches.

For example:

```text
                         coordinate C
                              |
                    reproducible projection
                              |
          +-------------------+-------------------+
          |                   |                   |
          v                   v                   v
     Codex + Sol         Codex + Luna       Claude + model X
          |                   |                   |
          v                   v                   v
       branch A            branch B            branch C
```

The original history remains unchanged.

Each experimental branch records its own realization and outcome.

This makes a real workflow decision point eligible to become a repeatable
evaluation condition. It does not by itself establish that the point is a valid
or representative experiment.

## Coordinate and experiment specification are separate owners

A historical coordinate owns the starting decision environment. A research
experiment requires another revisioned object that owns at least:

- the research question and bounded claim;
- fixture selection and exclusion criteria;
- the manipulated and controlled variables;
- treatment and projection fidelity requirements;
- model, provider, harness, capability, and sampling bindings;
- repetitions, randomization, and stopping conditions;
- outcome definitions, acceptance or scoring oracles, and blinding limits;
- required production-path claims, their covered state and consumption
  boundaries, admissible evidence sources, and owning consumers;
- research execution, cost, credential, network, and mutation authority; and
- declared non-claims and generalization limits.

Conceptually:

```text
historical coordinate
        +
experiment specification
        +
admitted research realization
        |
        v
research run and attributable result
```

The coordinate makes the world addressable. The experiment specification makes
the comparison interpretable.

---

# 8. Comparative Branch Graphs

The result is more expressive than a conventional benchmark table.

Different realizations can produce different decision trajectories.

For example:

```text
coordinate C47

Sol / projection P1
    -> route R1
    -> escalation J3
    -> accepted result

Luna / projection P1
    -> route R2
    -> misses J3
    -> violates invariant I4

Luna / projection P2
    -> route R1
    -> escalation J3
    -> accepted result

Claude / projection P1
    -> requests additional evidence
    -> route R3
    -> accepted result
```

The system can therefore compare not only terminal quality, but where consequential behavior diverged.

A branch graph might later show:

```text
C47
├── Sol / P1 -> R1 -> C53
├── Luna / P1 -> R2 -> C54
├── Luna / P2 -> R1 -> C57
└── Claude / P1 -> R3 -> C61
```

The differences themselves become evidence.

---

# 9. Controlled Variation

The value of a reproducible coordinate is strongest when an experiment
specification changes one variable while holding materially relevant others
stable or recording why exact control is unavailable.

Candidate experimental dimensions include:

### Model

```text
same coordinate
same projection
same capabilities
same harness
different model
```

### Harness

```text
same coordinate
same model
same semantic projection
equivalent admitted capabilities
different harness
```

### Plan resolution

```text
same coordinate
same model
same harness
same canonical plan
different executor projection
```

### Capability grant

```text
same coordinate
same role
same semantic work
different admitted capability subset
```

### Context projection

```text
same coordinate
same role
same model
different amount or form of reconstructive context
```

### Reasoning or execution strategy

Where provider controls permit it, the same historical state may support comparisons of execution configuration without changing the underlying work meaning.

This creates a basis for controlled experiments using real workflow conditions.
Similarity of labels such as `same capabilities` or `equivalent harness` is not
enough; equivalence must be established for the causal contrast being tested,
and every material mismatch must remain visible.

---

# 10. Repeated Branches and Behavioral Variance

One branch does not establish deterministic model behavior.

Model execution may vary across repeated realizations even when the starting coordinate is held constant.

A research run may therefore create repeated descendants:

```text
C47
├── Luna / P2 / run 1
├── Luna / P2 / run 2
├── Luna / P2 / run 3
├── Sol / P1 / run 1
├── Sol / P1 / run 2
└── Sol / P1 / run 3
```

With an adequate experimental design, this may allow Work Engine to distinguish:

- systematic model differences;
- projection effects;
- harness effects;
- capability effects; and
- ordinary run-to-run behavioral variation.

The coordinate provides control over the reconstructed starting environment.

Repeated execution estimates within-fixture behavioral variation. It does not
increase task or domain coverage, and it cannot by itself identify which
changed variable caused a difference.

---

# 11. Relationship to Structural Plan IR

Structural Plan IR makes this architecture substantially more interesting.

The broader decision-gated implementation-compilation proposal supplies a
candidate revision chain from accepted proposal meaning through sealed material
decisions, evidence claims, repository binding, and implementation contract.
Plan IR is a possible representation and projection mechanism for that contract;
it is not the owner of the full historical coordinate or its authority.

A canonical plan may preserve one implementation contract while different executor projections expose different amounts of structure.

Conceptually:

```text
canonical Plan IR
       |
       +--> projection P1 -> Sol
       |
       +--> projection P2 -> Luna
       |
       +--> projection P3 -> another executor
```

A reproducible coordinate allows those projections to be tested against the same real decision state.

This creates empirical questions such as:

> At what Plan IR resolution does an executor preserve the canonical plan's
> material decisions, remain within delegated judgment, and satisfy independently
> owned acceptance consequences?

Possible evidence may look like:

```text
P0
    high material-decision divergence

P1
    architecture preserved
    significant local reconstruction failures

P2
    material decisions converge
    accepted implementation

P3
    little additional quality
    higher planning and projection cost
```

Agreement with a stronger executor may be useful diagnostic evidence, but the
stronger executor is not the semantic or acceptance authority. Plan quality,
projection fidelity, executor capability, and implementation outcome must be
measured separately.

The result would be an empirical resolution frontier rather than an assumed model hierarchy.

This directly supports capability-aware planning and execution strategy selection.

---

# 12. Relationship to Execution Characterization and Realization Capability Evidence

Work Engine is considering execution characterization as part of planning.

A planner may characterize properties such as:

- semantic novelty;
- remaining implementation discretion;
- repository breadth;
- dependency complexity;
- oracle strength;
- reversibility;
- discovery burden; and
- integration consequence.

Historical branch experiments can then test whether those dimensions actually predict execution outcomes.

Over time:

```text
planner characterization
        |
        v
selected execution strategy
        |
        v
observed branch outcomes
        |
        v
empirical capability profile
        |
        v
better future strategy selection
```

A capability profile therefore need not be based primarily on model reputation or generic benchmarks.

It can increasingly be based on observed behavior at Work Engine's actual decision coordinates.
The authoritative evidence should bind to an execution realization, not only a
model name. At minimum, that identity may include model and version, provider,
harness revision, role, context and Plan IR projection, capability grant,
execution-profile or task class, reasoning and sampling controls, and evidence
time. A model-level capability profile is a derived aggregation over those
observations and may become stale as any material dependency changes.

---

# 13. Production Work as Research Corpus

The architecture creates a potentially valuable feedback loop.

Real engineering work naturally produces difficult and discriminative states:

```text
architecture ambiguity
review finding
stale planning premise
runtime failure
recovery decision
topology conflict
migration consequence
weak verification condition
capability limitation
```

These states can become candidate evaluation coordinates.

Instead of constructing an entirely synthetic benchmark suite, Work Engine may accumulate a corpus of real historical decision points.

For example:

```text
evaluation-coordinate library

architecture judgment
    C42
    C118
    C391

bounded implementation
    C77
    C209
    C488

review reasoning
    C101
    C355

recovery / reconciliation
    C144
    C612
```

A separately governed curation process may admit some candidates into an
evaluation-coordinate library. Selection must preserve ordinary, failed, null,
boundary, and difficult cases rather than choosing only interesting states after
their outcomes are known. Only then can a newly available model or harness be
evaluated against a bounded claim of representativeness before being trusted
broadly in production workflows.

---

# 14. Production and Research Form a Closed Loop

The resulting system may support a continuous cycle:

```text
real engineering execution
        |
        v
revisioned workflow history
        |
        v
interesting decision coordinates
        |
        v
controlled branch experiments
        |
        v
empirical model / harness / strategy evidence
        |
        v
updated capability profiles and routing knowledge
        |
        v
better engineering execution
```

This is not automatic self-improvement in the sense of models modifying themselves.

It is a system becoming better informed about:

- which models are capable of which work;
- which plan resolutions are sufficient;
- which harnesses preserve required behavior;
- which capabilities improve outcomes;
- which context projections are sufficient;
- which strategies are economically useful; and
- where stronger judgment remains necessary.

---

# 15. Relationship to Runtime Realization and Isolation

Controlled experimentation requires the actual execution environment to match the declared realization closely enough to make comparisons meaningful.

Work Engine's runtime direction separates:

- provider turns;
- harness execution; and
- operator projection.

Capability realization additionally aims to bind an exact admitted runtime composition for execution.

Role manifests define capability requirements and ceilings, while parent roles may request bounded child capability subsets.

The harness should realize those grants rather than expose uncontrolled ambient facilities.

This matters for research because:

```text
declared capability set
must approximately correspond to
actual operational affordances
```

Otherwise an experiment claiming:

```text
reviewer with repository-read only
```

would be invalid if the underlying harness silently exposed mutation tools, ambient instructions, unmanaged subagents, or unrelated capabilities.

Runtime isolation is therefore not merely a safety property.

It is also an experimental-control property.

The closer the runtime approaches:

```text
required role projection
+
admitted capabilities
+
harness mechanics
+
model
```

the cleaner comparisons among realizations can become.

The pending Claude runtime and OpenCode substrate work are candidate sources of
realization evidence. They do not define the coordinate or acquire Work Engine
context, orchestration, or research authority merely because they can execute a
turn, preserve a session, or expose a fork operation.

---

# 16. Relationship to Hierarchical Orchestration

Hierarchical planning introduces additional reproducible coordinates at different levels of judgment.

Potential coordinates include:

```text
preplanner coordinate
    topology judgment

orchestrator coordinate
    dependency / execution-state judgment

branch planner coordinate
    implementation architecture judgment

supervisor coordinate
    slice progression / recovery judgment

builder coordinate
    local implementation judgment

reviewer coordinate
    falsification judgment
```

Each role sees a different causal context because each owns a different kind of decision.

Research comparisons should preserve those role boundaries.

A model performing well as a builder does not establish capability as a preplanner.

A harness that is convenient for implementation does not automatically satisfy reviewer-independence requirements.

The coordinate is therefore role-specific, not simply task-specific.

---

# 17. Relationship to Architecture Directions

Accepted architecture directions provide another important revision-bound input to a reproducible decision environment.

A historical coordinate should not merely reconstruct current repository state.

It should identify the architectural direction revision that governed the work at that time.

This matters because later architectural decisions may change what future work should do without rewriting whether an earlier judgment was valid under its original authority and evidence.

Conceptually:

```text
coordinate C
    |
    +-- architecture direction D3
    +-- adoption state A7
    +-- transition contract T2
    +-- plan P14
    +-- realization R9
```

A later direction revision D4 may change current routing or interpretation.

It does not silently rewrite the historical basis of C.

This supports truthful replay and comparison.

---

# 18. Relationship to Architecture Intake and Seam Reconciliation

Architecture intake may itself eventually produce research-worthy coordinates.

An intake session incrementally integrates new ideas against:

- one implemented baseline;
- accepted architecture direction;
- and the current prospective seam map.

A seam reconciliation point may therefore be replayable:

```text
same prospective architecture
same exact idea revisions and admission order
same seam-map and reconciliation revision
same implemented baseline and evidence cutoff
same accepted direction, adoption, and transition basis
different architecture model
```

This could eventually support investigation of:

- architecture synthesis quality;
- conflict detection;
- ownership assignment;
- proposal decomposition; and
- sensitivity to model choice.

This remains a downstream possibility rather than a current requirement of the intake architecture.

---

# 19. Relationship to Forensics

Forensics and experimentation can become two projections over the same historical substrate.

Forensics asks:

> What happened, why did it happen, and what environment did the role inhabit?

Experimentation asks:

> Reconstruct that environment and observe what happens under a controlled alternative realization.

Conceptually:

```text
historical coordinate
        |
        +--> forensic projection
        |       inspect original execution
        |
        +--> executable projection
                create successor branch
```

This makes execution history useful for both explanation and experimental replay.

---

# 20. Coding as the Current Proving Ground

## Current research pilots as experimental-governance precedent

The governed-agent pilot specifications already demonstrate that a repeatable
runtime is not a complete experiment. They separately bind causal contrasts,
admission sets, frozen treatments, fidelity review, repetition, scoring,
stopping rules, evidence packages, and research-only authority. A future
coordinate-based research facility should reuse or generalize those distinctions
rather than moving them into coordinate identity or discarding them because the
execution substrate is shared.

## AI-accessible browser as environment-state precedent

The pending AI-accessible browser design is a related example of the same
architecture at a different boundary. It treats browser facts, derived
representations, claims, environment branches, validity intervals, and
epistemic coverage as distinct revisioned state. Its requirement that every
operation bind to an exact scene, state, environment, and evidence revision is a
useful precedent for reconstructing a Work Engine decision world.

The browser also exposes an important limit: an external world may be observed
and projected without being perfectly reproducible. A coordinate should
therefore carry a reconstruction-coverage account such as:

```text
materialized
referenced and retrievable
observed but not reproducible
stale
inaccessible
unknown
```

Research admission can then decide whether the known closure is sufficient for
the particular causal comparison.

## Software engineering remains the current proving ground

The architecture described here is not inherently limited to software engineering.

In principle, reproducible role projections, bounded capability realization, branchable history, and controlled model comparison could apply to other forms of structured knowledge work.

Possible domains might include:

- scientific research;
- technical investigation;
- policy analysis;
- legal research;
- operational planning;
- security analysis; or
- other multi-role reasoning systems.

Work Engine has not established this generality.

Its current contracts, artifacts, role structures, and evidence mechanisms have been developed around software engineering.

Coding remains an unusually useful proving ground because it produces strong observable consequences:

- source changes;
- tests;
- review findings;
- repository state;
- failures;
- migrations;
- dependency structure;
- measurable cost;
- and durable artifacts.

Broader applicability should therefore remain a hypothesis until demonstrated.

---

# 21. Why This Is More Than an Agent Environment

A conventional agent environment primarily provides a place for a model to work.

A revisioned research-and-execution architecture additionally preserves the identity and history of the world in which that work occurred.

The distinction is approximately:

```text
agent environment
    provides execution context

revisioned execution architecture
    owns and reconstructs execution context

research architecture
    can branch that reconstructed context
    and compare alternative realizations
```

The model becomes a replaceable reasoning engine inside a durable system.

The harness becomes a replaceable realization mechanism.

The active context becomes a projection.

The history becomes the persistent substrate.

---

# 22. Potential Research Questions

If reproducible coordinates become sufficiently faithful, Work Engine could investigate questions such as:

### Model capability

- Which models preserve material architectural decisions?
- Which models are reliable for bounded implementation?
- Which roles require high-capability judgment?

### Plan resolution

- What is the minimum Plan IR resolution required by a given executor?
- When does additional planning resolution stop improving outcomes?
- When does planner inference cost exceed downstream savings?

### Context sufficiency

- Which information is causally necessary to preserve execution quality?
- How aggressively can a role projection be compressed?
- Which context can be safely reconstructed on demand?

### Harness effects

- Does the same model behave differently under different harness realizations?
- Which harness affordances materially affect success?
- Which ambient features introduce unwanted behavioral variation?

### Capability effects

- Which capabilities improve decision quality?
- Which merely enlarge the action space?
- Which capabilities create correlated failure modes?

### Review strategy

- Which reviewer realizations detect which defect classes?
- When does reviewer persistence improve remediation?
- When is fresh independence necessary?

### Execution economics

- Which model / plan / verification combinations minimize total accepted-work cost?
- Which task properties predict cheap-model success?
- Where does repair cost erase inference savings?

These questions arise naturally from the architecture without requiring a
separate execution substrate. They still require separately owned benchmark,
experiment, and evidence-admission contracts.

---

# 23. Reproducibility Requirements

The concept only becomes meaningful if reconstruction fidelity is strong enough.

A useful research coordinate should eventually make explicit:

- exact historical lineage;
- role identity;
- authority basis;
- applicable architecture revisions;
- semantic artifact revisions;
- exact repository revision and any workspace overlay;
- required generated, indexed, fixture, and external-system state;
- governing instruction and configuration revisions;
- evidence cutoff;
- reconstruction coverage and material omissions;
- unresolved state;
- queued operator input and the effects/reconciliation frontier;
- capability grant;
- runtime realization;
- context projection identity;
- the identity, covered state, consumption boundary, consumer, and establishment
  status of any production-path claim relied upon;
- known omissions;
- and experimental variation.

A branch result should identify the coordinate and realization from which it descended.

An experimental result should additionally identify the exact experiment
specification, treatment or condition, assignment, repetition, scorer or oracle,
and research authority under which it was produced.

Every admitted realization must also be able to obtain the exact subject it
claims to execute or review. A subject may be materialized into the realization
or held behind a shared, integrity-checked reference whose retrievability is
part of admission. A local-only ref or ambient working-tree path is not a common
subject across realizations that cannot read it.

Derived comparison artifacts such as diffs are projections, not subject
identity. Prefer exact endpoint content identities. When a projected delta is
relied upon, bind its endpoint identities and a canonical projection format, or
record the generator and every option or configuration input that can change
its bytes.

Without these properties, apparent comparisons may actually compare different worlds.

---

# 24. Reproduction Fidelity Is Not Outcome Identity

Even a perfect external reconstruction does not imply identical model behavior.

Different models may:

- interpret evidence differently;
- choose different valid routes;
- request different information;
- escalate at different thresholds;
- make different mistakes; or
- reach different but equally valid implementations.

Repeated runs of the same model may also diverge.

The purpose of reproducibility is therefore not:

> produce the same output.

It is:

> **make the relevant starting conditions attributable enough that differences in continuation become meaningful evidence.**

Projection fidelity is adequate only relative to the claim made from the
projection. Mechanical reconstruction coverage can expose omissions, but no
general score establishes semantic sufficiency. Fidelity evidence should
therefore accumulate as claim-bound falsifications: a judgment that required
omitted state, a recovery that had to reacquire missing evidence, or another
attributable failure of the projection at a coordinate.

Successful recovery is weak evidence that a projection sufficed for that role
and coordinate; a recovery that required omitted information is a specific
falsification. Both observations carry their sampling condition. Recovery
exercises failures, restarts, and degraded environments rather than an unbiased
sample of ordinary coordinates, so its record must not be treated as an
estimate of general projection sufficiency.

---

# 25. Research Branches Must Not Acquire Production Authority

Experimental execution should remain distinct from canonical workflow execution.

A research branch must not become accepted work merely because it performs well.

Conceptually:

```text
production coordinate C
        |
        +--> canonical successor
        |
        +--> research branch A
        +--> research branch B
        +--> research branch C
```

Research descendants may produce:

- evidence;
- model capability observations;
- routing evidence;
- candidate findings; or
- new hypotheses.

They do not silently acquire:

- implementation acceptance;
- publication authority;
- architecture authority;
- spending authority;
- or workflow-state authority.

Any research result that should affect production must pass through the owning decision boundary.

Research execution nevertheless requires affirmative authority of its own. A
research branch may spend money, access credentials or networks, retain
sensitive artifacts, or create externally visible effects only within an exact
research grant. Isolation from production authority is not permission to run an
experiment.

---

# 26. Historical Replay Must Preserve Original Meaning

Later systems may understand more than earlier systems.

That does not permit historical reinterpretation to replace original meaning.

A historical coordinate should remain bound to:

- the schema revisions;
- architecture directions;
- capability semantics;
- plan revisions;
- and authority state

that governed its original execution.

A modern projection may add explanatory metadata or compatibility interpretation.

It must not silently claim that the historical role possessed semantics that were not present at the time.

This distinction is important for both audit and research validity.

---

# 27. Coordinate Adequacy Is Claim-Relative

A historical point does not possess one intrinsic reconstruction-fidelity
class. The same coordinate may be adequate for forensic inspection and
inadequate for role recovery; adequate for one bounded comparison and
inadequate for another whose claim depends on omitted state.

Recovery, comparative, forensic, benchmark, and architecture use may still
define useful admission profiles. They should remain named consumers with
explicit claims and coverage requirements rather than becoming a taxonomy that
asserts what every coordinate of a class contains.

Admission evaluates the coordinate's reconstruction-coverage account against
the particular claim, covered state, consumption boundary, realization, and
consequence. An inaccessible or unknown input leaves the dependent claim
unestablished unless its owner has already authorized an acceptance condition
that does not rely on that input.

The admission decision should expose both declared reconstruction coverage and
the accumulated claim-relevant falsification record. A mechanically convenient
coverage number must not displace evidence that prior judgments required state
the projection omitted.

---

# 28. Open Architectural Questions

Several important questions remain unresolved.

1. What exact durable object defines a historical coordinate?
2. Is a coordinate an explicit artifact or a derived tuple over other revision identities?
3. Which workflow events are sufficiently meaningful to become reconstructable coordinates?
4. What minimum state is required for safe role recovery?
5. What additional state is required for controlled experimentation?
6. How should claim-relative projection falsifications be attached to projection
   revisions and made available to later admission decisions?
7. Which context elements must be materialized versus referenced?
8. How should unavailable historical capabilities be represented during replay?
9. How should experiments handle provider features that cannot be reproduced exactly?
10. How should stochastic variation be separated from model, harness, or projection effects?
11. What execution branches should be retained permanently?
12. Which experimental results should become durable model-capability evidence?
13. How are research branches isolated from production authority and mutation?
14. How should model capability evidence expire when model versions or harnesses change?
15. Which historical coordinates are suitable for benchmark reuse?
16. How should privacy, credential, cost, and external-network authority be handled during replay?
17. Can a coordinate remain useful when repository state has advanced substantially?
18. Should historical source state be reconstructed in an isolated worktree or another content-addressed environment?
19. How are Plan IR revisions and historical repository state bound together?
20. Which aspects of this architecture are genuinely domain-general versus artifacts of software engineering?
21. What artifact owns experiment specification, preregistration, and amendment?
22. How are coordinate candidates sampled without post-outcome selection bias?
23. Which acceptance or scoring oracle is valid for each admission profile and
    bounded claim?
24. What effects and messages belong to the recovery frontier after a coordinate?
25. Which realization owns admissible observation for each required
    production-path claim, and how is `unestablished` routed to its consumer?

---

# 29. Failure Modes

The concept should be treated cautiously if implementation begins to produce any of the following.

### False reproducibility

The system claims two branches began from equivalent conditions when material context, capabilities, or repository state differed.

### Unreconciled replay

A successor or experimental branch begins from an earlier coordinate while
later tool effects, queued inputs, leases, or predecessor activity remain
unresolved.

### Transcript worship

Reconstruction degenerates into preserving enormous conversation histories rather than durable semantic consequences.

### Hidden ambient state

Harness configuration, filesystem state, project instructions, credentials, or provider behavior influence experiments without being part of the declared realization.

### Historical reinterpretation

New schemas or architecture understanding silently rewrite what an old coordinate meant.

### Benchmark overfitting

Routing or model selection becomes optimized against a narrow coordinate corpus rather than real production outcomes.

### Post-outcome experiment construction

Coordinates, treatments, exclusions, outcomes, or stopping rules are selected
after branch behavior is visible and then reported as if they were frozen in
advance.

### Oracle substitution

Agreement with a favored or stronger model is treated as correctness instead of
plan fidelity, authorized judgment, and independently owned acceptance evidence.

### Claim substitution

An actor responds to an unestablished claim by narrowing its subject, covered
state, consumption boundary, or consumer until available evidence can support a
weaker statement that the owning acceptance condition never authorized.

### Self-attested independence

An acting role's own route record is treated as independent observation of its
capability grant, exposure, custody, or external effects even though the
purported observer is under the same relevant authority.

### Research authority leakage

Experimental branches mutate production state or acquire acceptance authority.

### Coordinate explosion

Every event is retained as a first-class benchmark point without evidence that reconstruction has value.

### Excessive reconstruction cost

Recreating a coordinate becomes more expensive than rerunning the relevant workflow.

### Model-specific architecture

The system encodes today's model quirks into permanent semantics rather than keeping empirical capability evidence replaceable.

---

# 30. Expected Value if the Concept Holds

If sufficiently faithful reconstruction proves practical, several current Work Engine concerns converge.

Recovery becomes:

```text
reconstruct trusted coordinate
-> reconcile later and uncertain effects
-> fence predecessor and preserve queued authority/input
-> admit and instantiate successor
```

Model evaluation becomes:

```text
reconstruct coordinate
-> bind experiment specification and research authority
-> branch across admitted realizations
-> compare
```

Plan-resolution research becomes:

```text
reconstruct coordinate
-> vary projection resolution
-> measure convergence and cost
```

Harness research becomes:

```text
reconstruct coordinate
-> vary runtime realization
-> compare
```

Context optimization becomes:

```text
reconstruct coordinate
-> vary projection contents
-> measure outcome degradation
```

Capability research becomes:

```text
reconstruct coordinate
-> vary admitted capabilities
-> measure consequence
```

These would no longer require independent execution infrastructures.

They would become different uses of the same revisioned execution substrate.

---

# 31. Conceptual Architecture

The emerging architecture can be summarized as:

```text
                   DURABLE REVISIONED STATE
                            |
          +-----------------+------------------+
          |                 |                  |
          v                 v                  v
      authority         semantic state      evidence
          |                 |                  |
          +-----------------+------------------+
                            |
                            v
                 BOUND WORLD-STATE BASIS
                            |
                            v
                   HISTORICAL COORDINATE
                            |
                            v
                     ROLE PROJECTION
                            |
             +--------------+--------------+
             |                             |
             v                             v
       runtime realization           context projection
             |                             |
             +--------------+--------------+
                            |
                            v
                        MODEL
                            |
                            v
                 judgment / execution
                            |
                            v
                       NEW EVENTS
                            |
                            v
                   DURABLE REVISIONED STATE
```

Branching adds:

```text
                         coordinate C
                              |
                  experiment specification
                              |
              +---------------+---------------+
              |               |               |
              v               v               v
           branch A        branch B        branch C
              |               |               |
       realization A   realization B   realization C
              |               |               |
              +---------------+---------------+
                              |
                              v
                      comparative evidence
```

---

# 32. Relationship to Work Engine's Existing Philosophy

This concept is not a replacement for Work Engine's existing philosophy.

It appears to be a consequence of applying that philosophy consistently.

Work Engine already seeks to:

- make true constraints explicit;
- leave legitimate route choice to model judgment;
- preserve authority outside model convenience;
- retain truthful provenance;
- minimize unnecessary context;
- use deterministic machinery where judgment is unnecessary; and
- separate durable product meaning from replaceable mechanisms.

A reproducible coordinate extends those principles to model continuity itself.

Instead of asking the model to preserve the workflow by remembering it, Work Engine attempts to preserve enough of the world that another model realization can inhabit it.

---

# 33. Compact Interpretation

The concept can be stated in several equivalent ways.

### As execution architecture

> A model is a replaceable executor over revisioned external state.

### As recovery architecture

> A logical role can be reconstructed from durable state rather than depending permanently on one conversation.

### As research architecture

> A historical decision environment can be combined with a governed experiment
> specification and branched under controlled alternative realizations.

### As model evaluation architecture

> Model capability can be measured against real Work Engine decision states rather than inferred only from generic benchmarks.

### As context architecture

> The context seen by a model is a projection of owned state, not the canonical state itself.

---

# Core Concept

> **Make the world of model judgment revisioned and reconstructable.**

If that can be achieved with sufficient fidelity and safe reconciliation, a Work Engine execution coordinate becomes more than a point from which work can resume.

It becomes an addressable fixture from which separately governed alternative
reasoning and execution paths can be explored without rewriting history.

That would make Work Engine simultaneously:

- an engineering execution system;
- a recovery system;
- a forensic system;
- and a research architecture for comparing model, harness, context, capability, and planning strategies against real consequential work.

Software engineering is the current domain in which this architecture is being developed and tested.

Whether the abstraction generalizes beyond that domain remains an empirical question.
