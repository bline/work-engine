# Context and instruction loading

Authority: Non-normative explanation; part of [PHILOSOPHY.md](../../PHILOSOPHY.md).
Scope: reasoning behind doctrine, without adding requirements.

Load when: Reasoning about context lifetime, documentation scope, or startup and recovery cost.

Section numbers retain their original citation identities. Links identify
related topics; they do not require reading every linked document.

## 14. Context has value and lifetime

> **Preserve the consequence of reasoning, not necessarily the reasoning transcript.**

Model context is not merely temporary storage. It contains accumulated interpretation.

Discarding useful context forces the system to reconstruct:

- architectural understanding;
- prior findings;
- accepted invariants;
- unresolved questions;
- evidence already evaluated;
- reasoning about why previous findings mattered.

This can be extremely expensive.

For that reason, context lifetime should be chosen according to the value of retained reasoning.

A reviewer is a good example.

A useful lifecycle is:

```text
fresh independent reviewer
→ initial review
→ findings
→ builder remediation
→ same reviewer evaluates the delta
→ repeat while context remains useful
→ discard
```

Freshness matters at entry because independence matters.

Persistence matters during remediation because the reviewer already understands the problem.

The choice to retain or reset that context should itself be model judgment unless a product contract imposes a boundary.

This follows the same philosophy: preserve useful structure, preserve useful knowledge, and avoid procedural lifecycle rules that destroy value without a causal reason.

---

## 17. The role of the design document

The design philosophy should not itself become workflow bloat.

That is why this file is separate from `DESIGN.md`.

`DESIGN.md` should contain the compressed product doctrine:

- the invariant structure;
- the first-class categories;
- the minimum design tests needed to keep new features aligned.

This file may contain:

- metaphors;
- examples;
- historical reasoning;
- failure modes;
- competing formulations;
- explanations of why the design evolved.

The distinction is deliberate:

> **Conversation and philosophy may be expansive. Product doctrine should be compressed.**

Runtime instructions form a narrower projection again. An agent should normally receive only the invariant structure, consequences, authority boundaries, and capability affordances relevant to its role and current machinery. Those instructions should remain traceable to the owning design rather than becoming an independent policy layer that can silently drift into new procedure.

A future author should be able to understand the "why" here without forcing every agent to ingest the entire reasoning history during ordinary execution. The same context-economy principle applies across the documentation stack: preserve the consequence and lineage needed for correct judgment, not every layer of explanatory history in every runtime context.
