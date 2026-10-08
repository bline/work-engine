# CE3 and SC2 bounded implementation integration

Prepared 2026-10-08 against shared commit
`41992a22d86b041d2db357d16345c477804af63a`. This is a planning proposal,
not implementation, publication, provider execution or activation authority.
The two bounded planners use `gpt-6-astra` at high reasoning effort; subsequent
implementation and remediation use the user's selected `gpt-6-sol`.

The [CE3 plan](rust-claims-ce3-implementation-plan.md) owns observation custody,
establishment and exact claim evidence readback. The
[SC2 plan](rust-campaign-sc2-implementation-plan.md) owns campaign dispatch,
outcome, retry/evaluation state and recovery. Both refine the existing
[HP2 ownership contract](rust-host-hp2-contract.md); neither transfers claim
truth to campaigns or campaign selection to claims.

## Recommendation and dependencies

Start CE3 and the independent SC2 core in parallel after accepting their bounded
plans. Complete the real-store outcome/evaluation join after CE3 and the small
episode-owner prerequisite are available. Keep runtime-backed positive retry
qualification with HP3's provider evidence owner. This is an explicit staging
adjustment: SC2 core completion must not be reported as fully qualified retry.

Work Engine is unused during development. No temporary Node bridge, provider
daemon, generic broker or replacement evidence service is needed to make these
lanes appear independent. Lifecycle and terminal UI work retain their own owners.

| Work | Owner | Dependency and completion meaning |
| --- | --- | --- |
| Observation schema, custody, establishment, exact checked readback | CE3 worker | Claims-local implementation and qualification; preserves CE1/CE2 finding behavior |
| Campaign state, durable slots, owner fencing, leases, dispatch and recovery | SC2 worker | Independent core can proceed against the agreed API contract; local tests do not qualify the join |
| Permanent scoped read-only episode admission/readback prerequisite | Integration-assigned episode worker | Required before SC2's real episode-store completion gate; exact bounded surface in the SC2 plan; grants no episode writes |
| Claims/episode/campaign outcome and evaluation join | SC2 worker with integration owner | Real owner applications and stores; no fixture admission substitutes |
| Provider failure custody and positive runtime-backed retry | HP3 provider owner | Pending; pure retry validation and refusal tests do not establish this capability |
| Shared Cargo/dependency changes and combined validation | Integration owner | Serialized with current workspace owner; no worker independently rewrites the shared lockfile |

## Shared boundary

The existing CE2 API is `AdmissionBinding` plus
`FindingAdmissionPort::acquire`, returning a `FindingLease`. CE3 adds a separate
production-path admission binding and port; it does not pretend a generic
admission API already exists. The campaign adapter implements that port over
campaign-owned current state and a private, bounded local lease.

The proposed exports are `ProductionPathAdmissionBinding`,
`ProductionPathAdmissionPort::acquire_production_path`, `ProductionPathLease`
and `CheckedClaimEvidence`. CE3 supplies observation/establishment locators,
`record_observation`, `establish_claim`, `read_claim_admission` and separate
stage reconciliation. SC2 supplies `CampaignClaimsAdmission`; CE3 has no
campaign dependency.

The lease protects the exact campaign revision, selected candidate/profile,
prepared request and durable slot through the claims commit or checked read.
It does not keep a SQLite transaction open across stores. Reopening a campaign
root changes the owner epoch; an exact recovery capability cannot grant provider
entry. Lock loss, timeout and absent local receipts cannot settle an external
effect.

Campaigns supply the exact full selected claim document. Claims checks its own
stored observation and establishment and recomputes the binding. The checked
owner result is privately constructed and cannot be deserialized into a grant.
A serialized reference or admission DTO remains descriptive data. The two
consumption boundaries and consumers remain distinct.

Pre-episode bindings use the actual result's claim-codec digest and exact episode
identity/attempt. An episode revision is included only when it actually exists;
no revision is synthesized to make an early operation look complete. Historical
unavailable evidence is an absent observation with the establishment's existing
unavailable reference/digest representation, not a fabricated stored observation.

The lane plans own the exact public signatures, closed fields, schemas, errors
and fixtures. Their shared signatures must agree before implementation begins.

The episode prerequisite exposes scoped `Read`/`Recover` through owner-validated
startup configuration and a privately constructed checked readback. It creates
no new episode writer grant. Joined test setup uses the existing actual native
descriptor admission route for episode writes; the new read API verifies exact
historical revisions and transition identities. This keeps the permanent
addition bounded and leaves runtime result production with HP3.

## Implementation ownership and gates

Each worker owns its domain crate and its lane's tests/fixtures. The SC2 plan
identifies the episode-owner files separately; editing those requires an explicit
integration assignment, rather than expansion of the campaign worker's ownership.
The parent owns this integration document. Concurrent terminal UI and lifecycle
files are excluded.

Use new private roots. Ordinary open must not silently upgrade historical roots,
extend their grants or decide saved-state disposition. Domain schema evolution
is explicit; live-root migration remains later work. Scratch and disposable test
roots belong under `/home/bline/code/.work-engine-tmp`, not `/tmp`.

Qualification has three distinct results:

1. Each lane's focused codec, admission, persistence, recovery and fault tests.
2. Joined real-store tests for exact owner readback, wrong-root/revision/scope
   refusal, lease fencing, partial commits and resume without provider re-entry.
3. Later HP3 external-provider evidence and responsiveness qualification.

The second result cannot be inferred from the first, and neither establishes the
third. Report blocked or deferred positive retry qualification explicitly.
Joined library tests may use a controlled immutable execution-evidence owner,
with real domain stores and admission. That qualifies the domain composition,
not runtime attestation: HP3 still supplies actual execution, transport and
artifact custody. A permissive callback or caller-provided verified flag does
not qualify even the library join.
Retained remediation, succession, campaign acceptance/terminalization, live
migration, host activation and UI work remain outside this pair.

## Decisions included in plan acceptance

Accepting these plans would accept the explicit new-private-root schema-2
policy, the bounded episode-owner prerequisite and staged HP3 retry qualification.
CE3 also proposes the enumerated legacy semantic changes in its lane plan:
nonempty event identity, independently checked custody/admission, exact execution
binding and owner-computed establishment fields. It preserves the historical
evaluator's empty-artifact behavior while requiring the trusted custody profile
to establish its actual evidence requirements. These are reviewable proposals,
not changes already authorized or implemented by this planning turn.

For final combined validation, include relevant existing domain regressions,
formatting and Clippy. The S4 handoff reported a process-global
`REVIEW_EPISODE_FAULT_CUT` test race: serialize relevant fault tests or separately
assign a narrowly scoped isolation fix. Do not count a parallel test-environment
race as proof of a production defect or conceal it with an unexplained rerun.

## Evidence and status

The planning baseline includes published S4 and the previously committed
compiler, episode, CE2 and SC1 work. Source and coverage receipts are retained in
`/home/bline/.local/state/work-engine/rust-ce3-sc2-planning-20261008/`.
Graph evidence uses Tier 2 verification with exact-source fallback for reported
coverage gaps. Coverage is best-effort, not a completeness guarantee.

Status: the user authorized **CE3 and SC2 core implementation in parallel using
gpt-6-sol** on 2026-10-08. That bounded work is implemented and validated in the
[CE3/SC2 core validation record](rust-ce3-sc2-core-validation.md). The episode-owner
prerequisite, final owner join and HP3 runtime work are not included in this
implementation assignment. No implementation qualification is claimed by this
planning document. Parent reconciliation is not an independent adversarial
review. Implementation evidence is retained separately under
`/home/bline/.local/state/work-engine/rust-ce3-sc2-implementation-20261008/`.
