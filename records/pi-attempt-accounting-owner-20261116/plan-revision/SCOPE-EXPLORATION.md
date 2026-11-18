# S1 scope exploration against current Work Engine developments (2026-11-17)

Root: Pi session `01a1e569`. Operator direction: explore current developments for anything
that reduces S1 scope, then decide whether the plan needs revision before a Fable xhigh plan
review. Evidence is repository history and files at `rescue-detached-20260911` around
`3c1fc206`; no paid model work was used for this exploration.

## What S1 specifies

The S1 contract (`experiments/pi-attempt-accounting/s1/contract/CONTRACT.md`) is a finite
specification plus independent oracle for Pi spend accounting over 175 historical cases. Its
oracle README records 105 `current_producer` raw-selected cases and 32 `future_profile` cases
(plus controls). "Current producer" is the manual-orchestration Pi route: `role_host.py` with
the `pi_container.py` subscription proxy (release `92a03789`), whose spending facts (cost-proxy
stop, reservation constraint, unenforced declared cap, retired sessions resumable without
admission) are in CONTRACT §2.2–2.3. "Future profile" is the anticipated Rust vertical
(`ExecutionGate`, `BillingBound`, rules G-1..G-8, A-2..A-5). S2a (a production reducer from S1)
stays blocked on full S1 (`contract/S2A-ENTRY-CRITERIA.md`).

## Developments that bear on scope

1. **The Rust vertical exists, built independently of S1.** The dev-vertical hosted profile has
   its own per-attempt provider ledger (`pi_coding_provider.rs`: reservations, Refused /
   NotDispatched / Completed / Uncertain dispositions, raw-usage settlement), a lifecycle gate
   that binds route/payer policy and re-checks currentness per attempt
   (`pi-pressure-context-host/src/coding.rs`, gate contract
   `planning/dev-vertical-hosted-gate-contract-20261116.md`), and a sole admission owner with
   sequential `ProviderAttempt` effects (`session-task-admission`, `9d658689`). It has its own
   offline matrix, live qualification and several Fable reviews. Since `8fa71188` (operator
   decision 2026-11-16) it is the only live *coding* path; credential-mounted profiles are
   retired. It does not consume S1, and its semantics differ from S1's future-profile contract.
   The dev-vertical completion plan keeps S1 explicitly experimental and not a production ledger.
2. **The legacy producer is still live.** `role_host.py`/`pi_container.py` remain in
   `skills/manual-orchestration` (last changed 2026-11-11), and the role-host ledger shows Pi
   turns as recently as 2026-11-16. Its accounting already ships in Python
   (`usage_ledger.py` + `role_host.py`: retained reservations, objective `pi_api` admission,
   `f7a94781`, `375292f4`, `630240f2`). The dev-vertical plan notes this older path stages an
   access token in the container and has not been replaced, but no document decides to retire it.
3. **Organizational admission core (`a765298e`)** governs review-delivery admission, not Pi
   spend; no S1 scope change.
4. **Pricing.** The hosted path settles with pinned catalog rates (open item R-c: a proxy, not
   actual API spend); usage-ledger shared-prices plan is separate. No S1 scope change by itself.

## Scope-reduction candidates (operator decisions)

- **R1: drop or re-target the future-profile scope.** S1's future-profile cases and the
  ExecutionGate/BillingBound/G-rules sections specify a design that was not implemented as
  written. Under the pre-release compatibility policy, no consumer requires them. Options:
  remove them from the full-S1 universe (largest reduction: ~32 cases plus contract sections and
  their review partitions), or re-target them to the hosted ledger's actual semantics (that is
  new specification work, not a reduction).
- **R2: decide the legacy route's future.** If the operator retires or plans to replace the
  `role_host`/`pi_container` Pi route with the hosted path, S1's current-producer scope (~105
  cases) describes a producer with a bounded remaining life, and full S1 / S2a lose their
  consumer. Then S1 could stop at the bounded finding closure already achieved, with its
  invariants carried as regression properties for whichever ledger survives. If the legacy
  route stays, full S1 for current-producer cases keeps value for `usage_ledger`/`role_host`.
- **R3: use S1 as an invariant oracle, not a full spec.** Independently of R1/R2, the eleven
  closed findings express ledger-independent invariants (retained raw charge, malformed
  identity, no self-declared authority, raw-interval placement, distinct evidence, uncertainty
  is not release). The production-ledger cross-check (`ledger-crosscheck/`) tests whether the
  hosted ledger upholds them; its result informs whether S1's remaining value is as a
  specification or as an invariant source.

## Proposed next step

Ask the operator R2 (legacy route future) and R1 (future-profile scope). With those answers,
revise the full-realization plan's universe, staging and review partitions, then commission
the Fable xhigh plan review, which also covers the retroactive conformance of the option-3
decisions.
