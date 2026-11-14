# Supervisor assignment packets

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: forming, sizing or revising a worker or review assignment.

## Packets

When forming or revising an organization, including during ongoing implementation with no new dispatch, launch or discovery assignment, use [organization formation](organization-formation.md) to assemble the candidate and bind revision decisions to their authority.

A useful packet conveys the exact accepted authority, source/subject identity, **applicable settled facts**, outcome, checks, limits and escalation owner. Settled facts are constraints already fixed by accepted slices, contracts or committed code - a schema's version gates, an actor's concurrency model, a published wire contract. In S4-S8 about 42% of independent findings reopened such a fact and about 36% more were arguably preventable by better contract formation. Include only the facts that apply; loading everything recreates the context cost. For an integration or replay assignment, include known predecessor attribution and likely derived dependencies such as a lockfile; mark unknown lineage and resolve it before accepting the result, rather than making a final patch stand for its history. Treat admission HEAD as a reference until current affected bytes are bound at execution. Give a child only its packet, its identity binding and mandatory repository context; it loads its own role reference from this skill.

**Write packets in plain sentences.** State each settled fact once, in words, and refer back to it by name; put numbers a reader must compare in a small table. Shorthand such as `original68/24/36/8 and actual0/34/0 inclC7breach` (an S10 supervisor packet, 2026-10-29) saves characters, but every reader must decode it and a misread fact costs more than the words saved. The 27 packets and handoffs of that session came to about 19,700 words.

**Ask for custody records, not hand-written custody.** Builds, tests and frozen subjects are recorded by [`scripts/custody.py`](../scripts/custody.py) as the [worker reference](worker.md#builds-tests-and-frozen-subjects) describes. Do not ask children to copy PIDs, exit codes or tool-session output into separate records.

**Size a packet by what it must read, not only what it must write.** Reading the existing code and contract counts against the same context budget. On Claude Code (2026-10-22) a subagent opened at about 35K; agents told to read a 2,700-line crate plus contract sections spent about 100K before writing, and three in a row handed off with the work unfinished. A successor given an explicit read list (handoff, test support, function signatures via `grep`, and only the contract line ranges each test checks) read 46K and finished at 135K (n=1); a longer read list read 92K. Name the read set as precisely as the write scope, ask for signatures before bodies, and have builders commit after each independently passing unit so a handoff loses nothing.

Size a review the same way: count the reviewer's instructions, the subject and the evidence it must check against its window. On 2026-10-29 an Astra reviewer ran out of context at 225,732 of 258,400 tokens before finishing (n=1). If the necessary reading cannot fit, divide the review into named obligations with a reviewer each, and record any obligation left unreviewed. A smaller packet must not quietly narrow what the review claims to cover.

Before dispatch, compare the packet to the owning contract and current candidate. If the run is meant to be compared with another, freeze the subject, controls and scoring before either arm launches ([measurement](measurement.md#accounting-rules)); a run already started cannot be relabelled prospective.


For an assignment governed by the task-profile comparison, load [the pilot contract](task-profile.md) before screening or choosing its model.
