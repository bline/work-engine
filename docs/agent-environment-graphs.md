# Agent Environment Graphs

This document renders the first three role projections from
[`agent-environments.yaml`](agent-environments.yaml). The YAML file is the
automation-oriented projection source; invariant definitions and machinery
definitions remain owned by [`workflow-invariants.md`](workflow-invariants.md).

These diagrams describe each role's **declared operational world**. They are a
static verified baseline, not live runtime state and not a claim that every
platform or conversational influence is modeled.

## Slice Supervisor

```mermaid
flowchart TB
  R[Slice Supervisor]

  subgraph P[Pinned structure]
    I1[Objective and effective config]
    I2[Approval and authority boundaries]
    I3[Truthful lifecycle and terminal history]
    I4[Sequential accepted slices]
    I5[Limits, stops, and notifications]
  end

  subgraph C[Controls and mediated capabilities]
    C1[Campaign preflight]
    C2[Builder lifecycle control]
    C3[Receipt finalization]
    C4[Private checkpoint lifecycle]
    C5[Completion offer and publication adapter]
    C6[Validated resume]
    C7[Strategic reconciliation]
  end

  subgraph A[Owned state and artifacts]
    A1[Effective configuration]
    A2[Campaign lifecycle]
    A3[Durable audit history]
    A4[Live completion offer]
  end

  subgraph N[Explicit non-authority]
    N1[No repository-domain implementation]
    N2[No inferred validation]
    N3[No silent campaign amendment]
    N4[No commit without user decision]
  end

  R -->|BOUND_BY| P
  R -->|MAY_INVOKE| C
  R -->|OWNS| A
  R -->|FORBIDDEN_FROM| N
  C2 -->|repository work mediated by| B[Slice Builder]
  C4 -->|private Git mutation mediated by| CP[Checkpoint adapter]
  C5 -->|user-history mutation mediated by| CC[Completion adapter]

  classDef role fill:#dbeafe,color:#172033,stroke:#1d4ed8,stroke-width:2px;
  classDef pinned fill:#fff3cd,color:#1f2937,stroke:#8a6d00,stroke-width:2px;
  classDef control fill:#e8f1fb,color:#1f2937,stroke:#315b7d,stroke-dasharray:5 4;
  classDef artifact fill:#dcfce7,color:#172033,stroke:#15803d;
  classDef denied fill:#fee2e2,color:#172033,stroke:#b91c1c;
  class R role;
  class I1,I2,I3,I4,I5 pinned;
  class C1,C2,C3,C4,C5,C6,C7,B,CP,CC control;
  class A1,A2,A3,A4 artifact;
  class N1,N2,N3,N4 denied;
```

The supervisor has broad lifecycle authority but deliberately weak repository
agency. Repository work, private Git mutation, publication, and strategic
judgment are available through owners rather than absorbed into the supervisor.

## Slice Builder

```mermaid
flowchart TB
  R[Slice Builder]

  subgraph P[Pinned structure]
    I1[Accepted objective and scope]
    I2[Planning is read-only]
    I3[Preserve user-owned work]
    I4[Confirmed semantic path]
    I5[Configured gates and truthful provenance]
    I6[Independent evidence remains independent]
  end

  subgraph C[Manipulable interior]
    C1[Repository evidence]
    C2[Targeted direct-source observation]
    C3[Mutation inside accepted task paths]
    C4[Deterministic gate]
    C5[Independent reviewer]
    C6[Recorded route revision]
  end

  subgraph S[Observable and mutable state]
    S1[Repository and worktree]
    S2[Accepted scope]
    S3[Task-owned changes]
    S4[Gate state]
    S5[Review findings]
  end

  subgraph O[Outputs]
    O1[Plan and placement certificate]
    O2[Implementation and gate receipts]
    O3[Audit and handoff receipts]
    O4[Attributed checkpoint manifest]
    O5[Bound completion proposal]
  end

  subgraph N[Explicit non-authority]
    N1[Cannot accept its own plan or slice]
    N2[Cannot control checkpoint refs]
    N3[Cannot publish user-visible history]
    N4[Cannot silently expand scope]
  end

  R -->|BOUND_BY| P
  R -->|MAY_INVOKE| C
  R -->|OBSERVES / MUTATES WITHIN BOUNDARY| S
  R -->|EMITS| O
  R -->|FORBIDDEN_FROM| N
  O4 -->|consumed by| SP[Supervisor + checkpoint adapter]
  O5 -->|requires user authorization through| PUB[Supervisor + completion adapter]

  classDef role fill:#dbeafe,color:#172033,stroke:#1d4ed8,stroke-width:2px;
  classDef pinned fill:#fff3cd,color:#1f2937,stroke:#8a6d00,stroke-width:2px;
  classDef control fill:#e8f1fb,color:#1f2937,stroke:#315b7d,stroke-dasharray:5 4;
  classDef artifact fill:#dcfce7,color:#172033,stroke:#15803d;
  classDef denied fill:#fee2e2,color:#172033,stroke:#b91c1c;
  class R role;
  class I1,I2,I3,I4,I5,I6 pinned;
  class C1,C2,C3,C4,C5,C6 control;
  class S1,S2,S3,S4,S5,O1,O2,O3,O4,O5 artifact;
  class N1,N2,N3,N4 denied;
  class SP,PUB control;
```

The builder has the richest manipulable interior, but its mutation authority is
bounded by accepted scope and attribution. It contributes checkpoint and commit
inputs without owning either Git lifecycle.

## Independent Reviewer

```mermaid
flowchart TB
  R[Independent Reviewer]

  subgraph P[Pinned structure]
    I1[Fresh independent entry context]
    I2[Read-only evidence boundary]
    I3[Task-scoped causal review]
    I4[Truthful observed vs inferred findings]
    I5[No retrieval-as-review substitution]
  end

  subgraph C[Evidence capabilities]
    C1[Indexed repository structure]
    C2[Bounded Read / Glob / Grep]
  end

  subgraph S[Observable inputs]
    S1[Placement certificate]
    S2[Task-owned manifest and changes]
    S3[Compact deterministic gate result]
    S4[Necessary callers and dependencies]
  end

  subgraph O[Owned output]
    O1[Independent review result]
    O2[Findings with evidence and invariant impact]
  end

  subgraph N[Explicit non-authority]
    N1[No repository or Git mutation]
    N2[No tests or gate replacement]
    N3[No implementation or patches]
    N4[No plan or slice acceptance]
    N5[No raw builder reasoning context]
  end

  R -->|BOUND_BY| P
  R -->|MAY_INVOKE| C
  R -->|MAY_OBSERVE| S
  R -->|OWNS / EMITS| O
  R -->|FORBIDDEN_FROM| N
  BC[Builder context] -. INDEPENDENT_OF .-> R
  O -->|consumed and judged by| B[Slice Builder]
  O -->|acceptance consequence owned by| SP[Slice Supervisor]

  classDef role fill:#dbeafe,color:#172033,stroke:#1d4ed8,stroke-width:2px;
  classDef pinned fill:#fff3cd,color:#1f2937,stroke:#8a6d00,stroke-width:2px;
  classDef control fill:#e8f1fb,color:#1f2937,stroke:#315b7d,stroke-dasharray:5 4;
  classDef artifact fill:#dcfce7,color:#172033,stroke:#15803d;
  classDef denied fill:#fee2e2,color:#172033,stroke:#b91c1c;
  class R role;
  class I1,I2,I3,I4,I5 pinned;
  class C1,C2 control;
  class S1,S2,S3,S4,O1,O2 artifact;
  class N1,N2,N3,N4,N5 denied;
  class BC,B,SP control;
```

The reviewer has intentionally narrow agency: rich enough to independently
observe the bounded claim, but with no mutation, deterministic-gate, acceptance,
or campaign authority.

## Initial design observations

1. The supervisor's power is primarily lifecycle authority, not repository
   manipulation. Mediated capability edges are therefore central to its graph.
2. The builder's environment is intentionally the richest, but acceptance,
   checkpoint identity, durable append, and publication remain outside it.
3. The reviewer is not simply a builder with mutation disabled. It has a fresh
   context boundary, a narrower evidence objective, no deterministic-gate role,
   and an output that another role must judge.
4. `artifact.checkpoint_manifest` and `artifact.commit_proposal` are important
   ownership tests: builder emission does not imply lifecycle or mutation
   authority.
5. Negative authority is modeled explicitly. Missing `MAY_MUTATE` edges alone
   are insufficient because an incomplete projection could otherwise look like
   a prohibition.

## Automation path

The YAML already separates stable references from rendered presentation. A
future builder should:

1. validate invariant and mechanism references against
   `workflow-invariants.md` or its eventual structured successor;
2. validate entity references and role relation shapes;
3. expand role fields into typed edges;
4. render one Mermaid, Graphviz, or interactive projection per role;
5. run the declared analysis queries for obligation coverage, observability,
   authority, ownership, redundancy, procedural drift, and independence; and
6. optionally overlay effective campaign configuration and live runtime state
   without rewriting the role contract layer.

The rendered diagrams in this file are currently maintained manually. They
should be treated as views; `agent-environments.yaml` owns the role projection
data, and `workflow-invariants.md` owns invariant and machinery definitions.

