# Context lifecycle wake policy

Recorded 2026-10-09. A standalone addendum to the
[Rust lifecycle design](../experiments/context-lifecycle-rust/DESIGN.md)'s
"Time and efficiency policy". It lives in its own file because that design and
the [efficiency proposal](../ideas/01-semantic-durability-and-context-lifetime-judgment.md)
are bound by hash in active S8 planning evidence; editing them would invalidate
that binding. Fold this text into the design when its documents receive their
permanent home.

This states a policy direction. It changes no accepted S0–S8 acceptance scope
and confers no transition, delivery or scheduling authority.

## Policy

Waiting consumes no inference. A role awaiting subordinate work, a gate result or
input holds no model turn; the observer and scheduler establish completion from
durable state, never by model polling.

Only a delivery whose consequence needs the role's judgment is wake-eligible: a
terminal result, decision or authority request, operator input or recovery
obligation. Progress and liveness evidence is recorded without waking the role.

Pending wake-eligible deliveries may share one turn when their delivery contracts
permit. Each keeps its own input, delivery and custody identity; batching never
merges, reorders or settles deliveries on another's behalf.

Deadlines and leases detect stalled subordinate work, not periodic model
check-ins.

Usage accounting records each turn's wake reason, so supervision cost remains
observable alongside the cache-aware accounting and conditional payback in the
design (R28/T30).

## Rationale

Every wake re-sends the role's whole context. In the
[pure-Codex baseline](../metrics/baselines/pure-codex-20261009/README.md)
(2026-10-05 to 10-09), calls whose only action was waiting on subordinate agents
were about 11% of all spend. They were 31–32% of the spend of root supervisors
running at high and medium effort, whose contexts averaged about 144K tokens per
call. Messaging between agents was a further ~13%. Work Engine's deterministic
scheduler, observers and campaign owner can remove that cost structurally; this
policy keeps an otherwise deterministic organization from reintroducing it
through chatty wakes.

This applies the [determinism principle](../DESIGN.md#9-determinism-and-model-judgment):
whether subordinate work has finished is fully determined by available state, so
it should not spend model reasoning.
