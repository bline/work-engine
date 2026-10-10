---
name: proposal-packets
description: Discover and mechanically validate durable Git-backed proposal packets without deciding their semantic quality, priority, or implementation authority.
---

# Proposal Packets

Use this capability when a human or explicitly authorized formation role has
created or revised proposal packet manifests and their human-readable
narratives. Read [references/packet-contract.md](references/packet-contract.md)
before interpreting a packet.

The capability owns stable proposal identity, the versioned manifest and
decision-record contracts, repository discovery, authority-input transition
mechanics, and deterministic validation. The named decision owner supplies the
semantic disposition; this capability only records and checks it. It does not
own proposal meaning, portfolio decisions, roadmap mutation, implementation
authorization, interactive formation, or runtime campaign state.

Validate a bounded packet repository with:

```bash
python3 skills/proposal-packets/scripts/proposal_packets.py validate <packet-root>
```

A valid result establishes only the mechanically decidable contract described
in the reference. Preserve semantic uncertainty and the named human or
delegated decision owner; never promote validation success into acceptance.

Apply an authority-authored decision record with:

```bash
python3 skills/proposal-packets/scripts/proposal_packets.py transition \
  <packet.json> <decision-input.json> --repository <packet-root>
```

The command validates the packet's source state and authority attribution,
writes `decision.json` beside it, and updates only the lifecycle, placement,
and reopening fields carried by that decision. Git review remains the authority
boundary; the command cannot establish that a claimed actor was authorized.

A later placement-only revision of a decided packet uses the same command with
a version 2 `revise_placement` record. The command preserves the superseded
`decision.json` byte-exact under `decision-history/`; see the reference for the
supersession rules.

Transitions are locked per packet and journaled before any file changes. If
`validate` reports an interrupted transition, run `recover <packet.json>
--repository <root>` to finish it; do not repair the files by hand.
