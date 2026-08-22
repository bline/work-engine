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
