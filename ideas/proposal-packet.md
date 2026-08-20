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

