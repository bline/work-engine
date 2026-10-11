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
discovered packets, never by title or directory name.

## Validation scope and failure reports

Validation has two separate boundaries. The packet root bounds discovery:
every `packet.json` under it is a canonical packet, and no file outside it is
considered. The repository root bounds reference resolution: a packet-relative
path may leave the packet root, for example `../family.md` or
`../../../ideas/x.md`, provided it resolves to a file inside the repository.
Paths are never reinterpreted as repository-root-relative.
`validate --repository . proposals` is the repository's canonical invocation;
with one positional argument, that directory serves as both boundaries.

The invariant is: every canonical packet in scope is discovered and validated,
every failure identifies the violated contract, and no unrelated artifact can
prevent valid packet discovery. Validation therefore collects every failure in
one run instead of stopping at the first. Each failure names its packet and
owning file, plus a category:

- `malformed_packet`: the manifest cannot be read or violates the closed schema;
- `missing_reference`: a narrative or origin path does not resolve inside the
  repository;
- `malformed_decision`, `decision_presence`, `decision_binding`, or
  `decision_history`: the decision record is invalid, absent or unexpected,
  disagrees with the packet (reported against `packet.json` with the
  decision's expected value and the packet's observed value), or its history
  chain is broken;
- `transition_pending`: an interrupted transition awaits `recover`;
- `duplicate_identity` or `relationship`: proposal IDs or relationship targets
  do not resolve uniquely; and
- `duplicated_state`: a narrative restates lifecycle state the manifest owns.

A packet that fails to load is still reported, and relationships that target
it are not misreported as dangling.

### Enforcement

`.github/workflows/proposal-packets.yml` runs the tests and the canonical
validation on every push and pull request, with no path filter, so a change to
a referenced file outside `proposals/` is caught too. A local hook gives
earlier feedback but can be omitted or bypassed, so CI is the enforcement
boundary. To install the hook:

```bash
ln -s "$(git rev-parse --show-toplevel)/skills/proposal-packets/scripts/pre-commit" \
  "$(git rev-parse --git-path hooks)/pre-commit"
```

The hook validates the working tree, not the staged snapshot.

### Narratives and lifecycle state

`packet.json` and, once decided, `decision.json` are the only representation
of lifecycle, placement state, and authorization status. A narrative restating
them is a second copy, and copies drift. Narratives therefore carry no
`State:`, `Status:`, or `Lifecycle:` line. Prose that explains a decision, its
limitations, or what remains unauthorized belongs in the narrative body.

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
or version 2 proposal-decision schema. The record binds the proposal ID and prior lifecycle
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

### Placement revision of a decided proposal

Later evidence can change a decided proposal's conditional placement without
changing its approved meaning. A version 2 record with disposition
`revise_placement` is the only decision that may start from source lifecycle
`decided`. It replaces only the current placement state, placement claim, and
reopening conditions. The decided disposition, implementation authority,
roadmap priority, and permanent-architecture status remain unchanged.

The superseded decision is retained, not overwritten. The revision names a
`decision-history/` path and the SHA-256 of the current `decision.json` bytes.
The transition refuses a stale digest and preserves those exact bytes at that
path in the same transaction that writes the new record. Validation walks the `supersedes` chain from the
current decision, checks each archived record's digest, proposal identity, and
placement continuity, and rejects any history record the chain does not reach.
Version 2 first decisions carry `supersedes: null`; version 1 records remain
valid unchanged.

### Transition atomicity and recovery

A transition changes up to three files: the archived prior decision, the new
`decision.json`, and `packet.json`. It must never leave them in a mixed state
that validation cannot distinguish from a deliberate edit. The command
therefore:

- accepts only a `packet.json` inside the supplied repository whose every
  path component below the repository root is not a symbolic link, checked
  before anything is resolved (`validate` and `recover` apply the same check);
- holds an exclusive per-packet lock for the whole read, validate, and write
  sequence, refusing rather than waiting when another transition holds it;
- performs every check before writing, then journals the complete
  post-state with each file's prior digest;
- replaces each file by atomic rename, never following a symbolic link out of
  the packet; and
- removes the journal only after the result validates.

The lock and journal are local coordination state in the per-worktree Git
directory under `proposal-packets/transitions/`, never in the working tree.
While a journal is pending, `validate` reports an interrupted transition and
`transition` refuses. `recover <packet.json> --repository <root>` rolls the
journaled transition forward. Recovery re-decides nothing and refuses any
file that changed after journaling to content other than its recorded prior or
journaled result; such a conflict needs human resolution. The lock serializes
only writers that use this command, and Git review remains the publication
boundary.
