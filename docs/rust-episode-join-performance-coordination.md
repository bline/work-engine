# Episode integration and campaign performance sequencing

Prepared against `d4fc06027dc2dc11d91187da11a096c857148c25`, the published
CE3/SC2 core commit. The user authorized this bounded planning after publication.
The user subsequently authorized parallel ER1/P1 implementation. That authority
does not extend to J1/J2, providers, migration, activation or another commit.

## Recommended next pair

Run **ER1, the permanent read-only episode API**, in parallel with **P1,
campaign history measurement and bounded read-path optimization**, after plan
acceptance. Implementations use `gpt-6-sol`; these two planners used
`gpt-6-astra/high`.

- The [episode/owner join plan](rust-episode-owner-join-implementation-plan.md)
  specifies ER1 and the later dependent J1/J2 integration stages.
- The [history performance plan](rust-campaign-history-performance-plan.md)
  specifies the measurement contract and the smallest proposed optimization.

ER1 changes episode-owned code; P1 changes campaign-owned code. Neither needs
the other's unfinished product API. One integration owner serializes shared
Cargo/lock changes. Lifecycle and terminal UI retain their current ownership.

The later campaign join changes campaign application/store behavior and must
wait for the performance lane's stable handoff. Do not authorize simultaneous
edits to those files merely because their objectives differ. The episode-only
prerequisite provides useful parallel work without manufacturing a temporary
integration layer.

## Boundaries and proposed choices

ER1 should reuse the existing selected native root/profile and a typed startup
scope, with private checked readback. It adds an actual read-only store opening
route where the existing native opener writes SQLite configuration. It grants
no episode write operation. The selected executable identity remains meaningful:
joined fixture setup uses the same selected binary for its existing native write
route and new read route. This simplifies the earlier proposed separate scope/
source document schemas; it does not turn a root marker into episode authority.

P1 first measures the exact committed code and a reproducible bounded corpus.
Static source shows repeated validation within a read, but that is a hypothesis
about cost rather than a measured profile. The initial optimization should remove
duplicate work inside one operation while preserving fresh root, receipt, state,
revision, history and per-obligation checks. No persistent cache, database schema
redesign, history pruning or weaker integrity contract is proposed for this pair.

The earlier interrupted 129-attempt run is a diagnostic lead. Its completion
receipt reports 40 persisted slots after more than two minutes; retained raw
output does not provide a complete granular timing trace or reproducible stress
source. It cannot establish a supported workload, bottleneck or performance SLO.
The performance plan separates these facts and defines future measurement.

J1/J2 are subsequent integration stages, not simple wrappers around the existing
core. They must persist exact claim/evidence/consumption and finding bindings,
provide actual CE3 campaign leases and checked episode reads, and admit the
appropriate later projection/evaluation operations. The lane plan proposes an
explicit new private campaign schema/profile version for these added semantics;
that is a future acceptance decision, not permission to upgrade existing roots.

Actual execution/transport custody and positive provider-backed retry remain
HP3-owned. Controlled immutable evidence in real-store tests proves the local
owner composition only. It does not replace that runtime owner. No Node bridge,
generic broker, new domain daemon or availability infrastructure is needed.

## Handoff and validation

ER1 hands off its exact checked API, read-only refusal behavior, selected-binary
constraints and real-store tests. P1 hands off raw reproducible measurements,
the exact source/profile tested, preserved integrity gates and any remaining
performance limitation. A measured regression or unresolved correctness failure
is not a successful optimization.

Before J1/J2 begins, reconcile those two handoffs and freeze the campaign changes
against the actual accepted performance candidate. The joined workflow then
needs its own real-owner tests and partial-commit recovery cuts. Neither lane's
local pass qualifies the join or the full host.

## Planning evidence

Evidence is retained under
`/home/bline/.local/state/work-engine/rust-episode-join-performance-planning-20261008/`.
This is parent coordination and source-backed planning, not an independent
adversarial review. No product tests, builds or benchmarks ran during planning.
The preceding publication separately reran the CE3/SC2 package tests against its
isolated commit tree before fast-forwarding the shared branch.

Status: ER1 and P1 are implemented; their validation and acceptance dispositions
are recorded in [the implementation handoff](rust-er1-p1-validation.md). ER1
provides checked read-only episode access. P1 substantially reduces measured
current-read cost. The subsequent [focused replay comparison](rust-p1-replay-regression-validation.md)
closes the specific suspected slowdown with scoped measured acceptance; it does
not qualify every workload. Freeze this accepted campaign candidate before
J1/J2 edits begin. Timing windows were
serialized with other builds; background desktop load remained and is recorded.
J1/J2 and the proposed campaign version-3 policy remain later, separately accepted
work.
