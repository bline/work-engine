
### `evidence-backed-proposal-evaluation.md`

**Idea:** Research proposal characteristics using specialized agents and produce evidence-backed estimates rather than intuitive scores.

**Problem:** Roadmap prioritization currently lacks comparable information about expected impact, complexity, risk, fan-out, maintenance effects, and evidence confidence.

**Proposed consequence:** Every evaluated proposal can expose dimensions such as:

```text
expected impact
implementation complexity
design alignment
fan-out
regression risk
reversibility
maintenance benefit
validation burden
expected cost
evidence strength
confidence
```

Each estimate carries its evidence basis, limitations, provenance, and confidence.

No single ranking formula becomes canonical. Different portfolio views may weight the dimensions differently.

**Possible research roles:**

```text
repo / ownership analyst
design-alignment reviewer
metrics analyst
risk / reversibility analyst
proposal synthesizer
```

**Key design question:** Which dimensions can be meaningfully normalized across unlike proposals without manufacturing false precision?

---

### `proposal-backed-roadmap.md`

**Idea:** Reduce the roadmap to a portfolio/index of accepted proposal packets.

**Problem:** Rich proposal reasoning becomes duplicated or lost when copied into roadmap prose.

**Proposed consequence:** The roadmap contains only enough information to identify, order, and select work:

```text
title
short description
status / priority
proposal packet pointer
```

When a campaign begins, the supervisor and builder receive appropriate projections from the underlying packet rather than reconstructing the proposal.

**Additional consequence:** A rejected, deferred, or unselected proposal retains its research and can later be refreshed rather than rediscovered.

**Key design question:** What packet material should the supervisor receive versus the builder, reviewer, and evidence agents?


