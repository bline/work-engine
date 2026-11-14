# Supervisor context capacity

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: admitting substantial work into this root, assessing pressure, or preparing continuation.

## Your own context

A root accepting substantial new work makes a scoped
[capacity decision](context-boundary.md#capacity-belongs-to-the-next-scope),
including the work and collection still owed. An earlier decision to finish
bounded work does not cover the new task, even when no new pressure alert arrives.

A root's context grows with every report; roots averaged about 144K per call in the baseline, and the S10 root of 2026-10-28/29 averaged 246K over 596 calls. Keep it small by delegating reads to children and reading their compact results. Check a builder's evidence through its deterministic records - `custody.py` outcomes and an evidence battery `--compare` - and open logs only for a failure; reading the evidence in depth belongs to the reviewer. When you take over from a predecessor, start from compact working context recovered from the durable state (assignment record, continuation and handoffs), not from a retained large context. Record your reading at each dispatch and integration boundary (a current [listener observation](context-control.md#what-to-do-with-an-observation) counts), and choose a response against the next scope's needs and available capabilities. Pressure alone does not require a root transfer. A root started before the 250,000 compaction limit (2026-11-05T04:58Z) keeps compacting near 450K and gives its children the same limit; at a quiescent boundary, propose replacing it to the user ([Roots](context-boundary.md#roots)). For same-session replacement, use the [continuity guidance](same-session-context.md); when transferring ownership, use the user-authorized quiescent procedure under [Roots](context-boundary.md#roots). Keep the continuation and assignment record current continuously, because recovery cannot depend on meaning held only in the old context.
