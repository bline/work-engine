# Proposal packet contract

The packet directory contains `packet.json` and human-readable narrative files.
The manifest owns stable identity and closed lifecycle metadata; the narrative
owns current proposal meaning. Git owns durable history and review.

The JSON Schema is the canonical owner of version 1's closed field and
vocabulary surface; the validator loads it directly and adds repository-level
reference and identity checks that JSON Schema cannot express alone. Version 1
requires identity, family, lifecycle, narrative and origin references,
placement, uncertainty, decision authority, and typed relationships. Paths are
relative to the packet directory and must resolve inside the repository passed
to the validator. Relationship targets resolve by stable proposal ID across the
discovered repository, never by title or directory name.

The validator proves only closed properties: shape, vocabulary, unique IDs,
local reference resolution, relationship targets, and the non-authorization of
implementation. It cannot prove proposal quality, placement correctness,
evidence sufficiency, value, priority, or human acceptance.

`formed` and `placement_uncertain` are the active v1 formation states exercised
by the initial vertical. Later states and lineage relationship types are
representable for compatibility, but this capability does not perform those
transitions or grant their authority.

## Authority-controlled decision transition

A decided packet has one adjacent `decision.json` conforming to the version 1
proposal-decision schema. The record binds the proposal ID and prior lifecycle
and placement states to an authority-authored disposition, resulting conditional
placement, rationale, reopening conditions, actor attribution, authority
evidence, and invariant non-effects. The packet duplicates only the current
lifecycle, placement, and reopening projection needed by existing consumers;
the decision record owns why and by whose authority those fields changed.

The transition tool validates closed shape, attribution equality with the
packet's named decision owner, prior-state freshness, resulting-state equality,
and that permanent architecture, roadmap priority, and implementation authority
remain unchanged. Attribution is not authentication: Git authorship and human
review establish whether the named authority was actually exercised. Mechanical
validation must never infer, manufacture, or strengthen a semantic disposition.
