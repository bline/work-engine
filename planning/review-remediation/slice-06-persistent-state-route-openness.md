# Slice 6 — Keep Persistent-State Architecture Route-Open

## Protected consequence

After context loss or runtime replacement, an authorized agent can reconstruct
its truthful operational position from durable state and authoritative evidence
without replaying a transcript, resurrecting handled work, or confusing
terminal history with live state.

Model context is ephemeral working memory, not a safe state owner. Compaction is
an expected and inherently lossy runtime boundary in this environment; useful
context should be retained while it improves judgment, but correctness must not
depend on its survival.

## Observed doctrine conflict

The reviewed detailed persistent-state proposal required every meaningful state
mutation to produce an append-only semantic transition, and the roadmap placed
transition history, idempotent transitions, and a current-state projection in
the immediate slice. Those choices describe an event-sourced implementation
before placement evidence has established that event sourcing is required.

Append-only transitions may be useful, but transactional snapshots, database
journals, compare-and-swap documents, or another state-owner design could
preserve the same reconstructability, ordering, audit, and crash-safety
consequences.

## Acceptance consequences

- One authoritative live-state owner and its consumers are established through
  repository evidence before storage mechanics become binding.
- Stable logical identity remains separate from provider runtime identity.
- State distinguishes observed, inferred, unresolved, stale, and unavailable
  facts and references stronger owners rather than copying them.
- Updates are crash-safe and have enough identity/order semantics to make retry
  and reconstruction truthful.
- A handled event is not resurrected after context replacement.
- Every consequence needed for correct continuation survives context compaction
  in a named durable owner; retained context is never the only copy.
- The design can explain or audit consequential state changes to the degree an
  identified consumer requires.
- Append-only event history, materialized projections, hydration sequencing, and
  idempotency mechanisms remain implementation decisions unless a concrete
  failure mode makes one irreducible.

## Proof for this remediation slice

Verify across the current proposal, roadmap, planning lineage, and runnable
campaign that the recovery consequences above remain protected while
append-only history, materialized projections, hydration sequencing,
idempotency mechanisms, and storage technology are represented as candidate
routes rather than requirements. The artifacts must also name the durable owner
and consumer questions that implementation evidence still needs to resolve.

This is documentation and configuration evidence only. It does not establish
working runtime recovery. A later implementation slice must replace or
reconstruct an active context and observe recovery of the correct active work,
handled-event disposition, unresolved decisions, and authoritative artifact
references. That slice must also repeat a relevant write/recovery boundary and
observe no duplicated consequence or stale-state resurrection. This remediation
slice explicitly defers that executable proof and does not claim it passed.

## Insufficient substitutes

- Successful schema parsing without restored operational behavior.
- A transcript summary labeled as durable state.
- An event log with no authoritative current-state consumer.
- A current snapshot that cannot distinguish stale or already-handled input.
- Selecting SQLite, JSONL, or event sourcing solely because it is convenient.
