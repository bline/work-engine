### `proposal-packets.md`

**Idea:** Introduce a durable proposal packet as the canonical decision-context artifact between ideation and implementation.

**Problem:** Valuable reasoning is currently compressed into roadmap entries and later reconstructed by supervisors/builders.

**Proposed consequence:** Expensive proposal context survives across planning, prioritization, and implementation while each role loads only the subset it needs.

**Likely packet structure:**

```text
proposal/
  idea.md
  proposal.md
  placement.md
  evidence/
    design.md
    repo.md
    metrics.md
    history.md
  scoring.md
  dependencies.md
  implementation-notes.md
  decision.md
```

**Key design question:** What information is durable decision consequence versus disposable reasoning history?

---

### `proposal-formation-and-placement.md`

**Idea:** Add a proposal-formation stage between raw ideas and expensive evaluation.

**Problem:** A raw idea may be incomplete, incorrectly placed, duplicate existing machinery, or solve the wrong consequence. Scoring it before resolving those questions wastes research.

**Proposed consequence:** Every proposal entering evaluation has a coherent objective, intended consequence, likely owner, architectural placement, affected contracts, alternatives, boundaries, and invalidation conditions.

**Conceptual states:**

```text
idea
→ formed proposal
→ placed proposal
→ evaluated proposal
→ roadmap candidate
```

These should describe epistemic state, not impose a mandatory procedural ritual.

**Key design question:** What evidence is sufficient to call placement established before detailed evaluation begins?

## Persistence and checkpoint semantics

The proposal packet is not only an end-of-stage artifact. It is the durable working memory for proposal understanding that has downstream value.

The system should distinguish two forms of persistence.

### Checkpoint persistence

During ongoing work, an agent may update the packet when newly acquired understanding has enough durable value that losing the current context would cause meaningful reconstruction cost.

Useful signals include:

- an important placement or ownership conclusion becomes supported;
- a credible alternative is ruled out for a durable reason;
- expensive repository or metrics exploration produces reusable evidence;
- a material assumption is falsified;
- proposal decomposition changes;
- a dependency, conflict, or enabling relationship is discovered;
- accumulated context is becoming large, fragile, or likely to be compacted;
- the agent is about to move into a substantially different investigation;
- a human decision materially changes the direction of the proposal.

These are signals for judgment, not mandatory procedural triggers.

> **Persist by value, not by phase.**

Checkpoint persistence should preserve the consequence of reasoning, not the full reasoning transcript. Incremental updates are preferred when they preserve the durable fact without rewriting unrelated packet state.

### Handoff persistence

Before control passes to another agent, role, stage, or process, all information required to resume the proposal lifecycle correctly must already be represented durably in the packet or another canonical owned artifact.

A handoff may summarize or project that state for the next role, but it must not become the only durable copy of resume-critical understanding.

> **No handoff may be the sole durable owner of information required to resume the proposal lifecycle correctly.**

This is stronger than ordinary checkpoint guidance because violating it can cause accepted work to disappear after a crash, restart, lost connection, or context reset.

### Resume behavior

After interruption, the next agent should reconstruct only what is not already durably represented.

Resume should normally begin by loading the packet sections relevant to the current task, checking whether decisive evidence or placement claims have become stale, and continuing from the last durable proposal state.

The goal is not to preserve every transient thought. It is to prevent expensive semantic work from being lost merely because the process ended before a formal handoff.

### Information lifetime

Packet sections should be updated according to the lifetime of the information they own.

Examples:

- `proposal.md` preserves the current formed proposal;
- `placement.md` preserves supported placement conclusions, alternatives, and reopening conditions;
- `dependencies.md` preserves causal and non-causal proposal relationships;
- `evidence/` preserves reusable evidence and its provenance;
- `implementation-notes.md` preserves downstream-useful constraints and observations;
- `decision.md` preserves accepted, deferred, rejected, or superseded proposal decisions.

Exploratory detail that has no downstream value should remain disposable.

The layered maturity, evidence-baseline, claim-sensitivity, and refresh model is
developed in
[Research Maturity, Evidence Snapshots, and Staleness](./research-maturity-evidence-snapshots-and-staleness.md).
