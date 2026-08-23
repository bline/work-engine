# Placement: Agent-Instruction Structure and Placement Review

## Probable semantic owner

A distinct specialized reviewer owns diagnosis of the structure and placement
of materially normative agent-facing text. The likely first adapter is a
repository-local skill, provisionally `skills/agent-instruction-review/`, with a
bounded role contract and on-demand references only if dogfood shows they are
needed.

Implementation review is the primary consumer because it supplies actual
instruction artifacts. Proposal review consumes the specialist conditionally
when the proposal itself contains or changes normative agent instructions,
loading behavior, precedence, or an exact mandatory route.

## Why placement context belongs here

Instruction classification depends on the text's semantic owner, consumer,
authority, scope, and loading layer. A general rule in `AGENTS.md`, an
applicability instruction in a skill entrypoint, a mechanism in an adapter
reference, and an example in supporting documentation have different reach and
authority even when their wording is similar.

The reviewer therefore needs bounded placement evidence but does not acquire
architectural placement authority. When the underlying capability owner is
uncertain or stale, it emits a consequence for architectural diagnosis or the
owning decision role.

## Rejected placements

### Architectural diagnostic review

Rejected as the sole owner. Architectural review diagnoses system ownership,
decomposition, and component placement. Instruction placement has a distinct
subject, audience topology, context-loading cost, and applicability boundary.
Both reviewers may be selected when a new skill or role changes both kinds of
placement.

### Doctrine and authority review

Rejected as the sole owner. Doctrine review can identify binding conflicts and
authority inflation, but this specialist also distinguishes local mechanisms,
defaults, examples, loading reach, and misplaced text that may not violate a
binding contract.

### Skill authoring

Rejected. The author owns mutation and may use the same design questions, but
cannot provide an independent review of its own instruction choices.

### Adaptive panel coordinator

Rejected. Coordination decides applicability and preserves synthesis; it does
not absorb the specialist's diagnosis. The reviewer can also operate before the
coordinator candidate is implemented or accepted.

### Mechanical linter

Rejected. Whether a route is causally required and which distinction it
preserves are semantic judgments. Deterministic checks may later validate exact
references or closed output shape without deciding philosophical alignment.

## Reopening conditions

Reopen placement when dogfood shows no distinct findings, excessive context
requirements, incompatible proposal and implementation roles, a valid shared
owner in another review capability, or consumers requiring a different
artifact/state boundary.

