
### `closed-loop-engineering-learning.md`

**Idea:** Connect proposal predictions with Work Engine execution metrics and explicit implementation failure modes.

**Problem:** Work Engine already measures execution, but those observations do not yet systematically improve future proposal evaluation and implementation decisions.

**Proposed consequence:** Completed work produces calibration evidence:

```text
predicted cost        ↔ observed cost
predicted complexity  ↔ observed effort
predicted fan-out     ↔ observed fan-out
predicted risk        ↔ repairs / regressions
predicted validation  ↔ observed validation cost
predicted impact      ↔ later measured consequence
confidence            ↔ prediction accuracy
```

The execution record should additionally distinguish actionable failure modes such as:

```text
placement failure
skill-understanding failure
capability/tool-use failure
context-bloat failure
validation failure
route failure
authority failure
provenance failure
```

Those observations become empirical priors for later decisions rather than new mandatory rules.

For example:

```text
proposal class
→ historically high placement-error rate
→ future evaluator receives stronger evidence that
   placement uncertainty deserves attention
```

not:

```text
proposal class
→ placement error happened before
→ always run procedure X
```

**Key design question:** How do we attribute failure modes strongly enough to learn from them without pretending causal certainty that the receipts cannot establish?

---

I like this split because there’s a natural dependency graph without making it one monolithic feature:

```text
proposal packets
      ↓
proposal formation + placement
      ↓
evidence-backed evaluation
      ↓
proposal-backed roadmap
      ↓
Work Engine execution
      ↓
closed-loop learning
      └──────────────→ improves future evaluation

