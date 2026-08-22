---
name: proposal-packets
description: Discover and mechanically validate durable Git-backed proposal packets without deciding their semantic quality, priority, or implementation authority.
---

# Proposal Packets

Use this capability when a human or explicitly authorized formation role has
created or revised proposal packet manifests and their human-readable
narratives. Read [references/packet-contract.md](references/packet-contract.md)
before interpreting a packet.

The capability owns stable proposal identity, the versioned manifest contract,
repository discovery, and deterministic validation. It does not own proposal
meaning, portfolio decisions, roadmap mutation, implementation authorization,
interactive formation, or runtime campaign state.

Validate a bounded packet repository with:

```bash
python3 skills/proposal-packets/scripts/proposal_packets.py validate <packet-root>
```

A valid result establishes only the mechanically decidable contract described
in the reference. Preserve semantic uncertainty and the named human or
delegated decision owner; never promote validation success into acceptance.
