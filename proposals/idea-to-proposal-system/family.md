# Idea-to-Proposal System

## Status

Formed proposal family. Its three proposals use the canonical version-1 packet
manifest while retaining human-readable narrative and supporting artifacts.
Mechanical validity does not evaluate or accept any proposal and does not
authorize implementation.

## Origin

This family converts the brainstorming in the following documents into bounded
candidate changes:

- `ideas/proposal-packet.md`;
- `ideas/proposal-packet-workflow.md`;
- `ideas/proposal-packet-impl.md`; and
- `ideas/interactive-idea-to-proposal-workflow.md`; and
- `ideas/raw-idea-intake-and-claim-assessment.md`.

Those documents remain source material. They are not contracts, evidence, or
implementation authority.

## Shared problem

Work Engine can execute bounded objectives and preserve execution receipts, but
it does not yet own a durable decision object between speculative ideas and
accepted roadmap work. Proposal meaning, placement, alternatives, and
relationships can therefore be reconstructed repeatedly or compressed into a
roadmap entry.

## Family decomposition

This family contains three proposals:

1. `raw-idea-intake` defines the source-bound assessment that can reconcile a
   raw idea into zero, one, or several candidate handoffs without rewriting the
   source or forcing proposal creation.
2. `durable-proposal-packets` defines the durable proposal object and its
   ownership.
3. `interactive-proposal-formation` adds a model-centered capability that
   creates and revises those objects with a human.

They are separate because raw-source assessment may yield no proposal, the
packet contract is useful without either authoring capability, and formation
owns candidate meaning rather than intake interpretation. They may be exercised
or delivered together when that preserves the same ownership boundaries; this
decomposition does not impose one mandatory runtime sequence.

Evidence-backed evaluation, proposal-backed roadmap selection, and calibration
from implementation outcomes remain later proposals. They are consumers of
this family, not hidden scope inside it.

## Authority boundary

The model may form, split, merge, challenge, and revise candidate proposals
within granted repository authority. A proposal artifact does not authorize
implementation, change roadmap priority, approve product value, or override a
human decision. Those transitions remain owned by the user or an explicitly
authorized planning contract.

## Family relationship

```text
raw-idea-intake
        enables
interactive-proposal-formation
        writes
durable-proposal-packets
        enables
evidence-backed evaluation
        informs
strategic planning / roadmap selection
        supplies bounded objectives to
campaign supervision and execution
```

Only the need for an intelligible durable target creates a causal relationship
between the first two nodes. Separate acceptance or deployment order is not
required.
