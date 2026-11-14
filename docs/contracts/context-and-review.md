# Context and review lifetimes

Authority: Binding Work Engine doctrine; part of [DESIGN.md](../../DESIGN.md).
Scope: the product contracts and design rules stated below.

Load when: Coordinating role lifetimes, conducting review, or deciding whether to retain or replace context.

Section numbers retain their original citation identities. Links identify
related topics; they do not require reading every linked document.

## 7. Information lifetime is a decision dimension

Context is a resource with value and cost. Its lifetime should reflect how long the information remains useful to correct judgment. Preserve the consequence of reasoning, not necessarily the reasoning transcript.

Work Engine currently recognizes at least these semantic lifetimes:

### Supervisor lifetime

Keep durable campaign state, configuration, approvals, continuation state, and compact handoff information needed across slices.

### Builder lifetime

Preserve the builder's accumulating understanding for the bounded implementation task while that understanding continues to improve decisions.

### Investigation lifetime

Use disposable reconnaissance or diagnostic contexts when they need large temporary evidence that should not occupy the builder's durable context. Return only the conclusions, evidence references, uncertainties, and artifacts needed downstream.

### Reviewer lifetime

When independence from the builder is part of the evidence claim, establish the
isolation, exposure, capability, and custody properties that the owning claim
requires. A fresh reviewer is one possible mechanism when prior exposure would
violate that claim. Once the required independence has been established,
preserve the same isolated reviewer through a bounded remediation loop while
its accumulated understanding remains useful.

A typical useful shape is:

```text
fresh isolated reviewer
→ initial review
→ findings
→ builder repair
→ same reviewer evaluates the delta and prior findings
→ repeat as useful
→ acceptance
→ discard reviewer
```

Restarting the reviewer is a judgment call unless the owning claim requires
renewed independence that the current reviewer can no longer establish. Reasons
may include a material architectural or placement change, a changed review
premise, degraded or oversized context, or a need for a genuinely new
independent perspective. These are signals for judgment, not a deterministic
reset table.

Instruction freshness and reasoning-context lifetime are separate concerns. When instructions or skills change, refresh them in place when the runtime permits and the existing reasoning context remains valuable; do not destroy useful context merely to refresh instructions.

---

## 8. Independence is a property, not ritualized amnesia

Independent reasoning is valuable when correlated error is consequential. The required property is meaningful independence from the reasoning being challenged.

Freshness at the start of an independent review can establish that property. Repeatedly discarding the reviewer's own accumulated understanding during remediation does not inherently create more independence and may instead create repeated cost and inconsistent reconstruction.

When a placement certificate, adversarial semantic review, or other contract explicitly requires fresh independent falsification, preserve that requirement. Do not generalize it into compulsory freshness for unrelated retrieval, deterministic validation, or every repair iteration.
