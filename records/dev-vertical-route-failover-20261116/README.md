# Dev vertical: bounded route selection and authorized failover, 2026-11-16

Standing: bounded offline phase-3 implementation result. Successor to
[streaming revision 2](../dev-vertical-streaming-bridge-r2-20261116/README.md).
Not acceptance, not the required Astra/xhigh review, not live qualification.
Both routes are synthetic loopback endpoints. They share one host-held synthetic
credential, so per-route credential custody is not exercised here.

## Operator decisions recorded

- **Lease recheck window approved (2026-11-16):** while relaying one admitted
  provider response, `EffectLease` currentness is rechecked at most every 5 s,
  and always immediately before the request. Revocation stops delivery within
  that window. The probe constant is `LEASE_RECHECK`.
- The operator said "proceed" with the proposed phase-3 work. Session budget is
  unchanged ($40 of API credit for this session's own inference; about $5.67
  spent at the time of approval). No live funding was used.

## Design (`src/pi_route.rs`)

- A pinned `Policy` (digest recorded with every attempt) lists routes in
  preference order. Each route has a nonsecret connection id, model, payer, an
  `authorized` flag, a `revoked` flag and a per-route attempt limit. The policy
  also sets a per-request attempt limit, an allowance, a reservation per
  attempt, and an expiry.
- `Router::next` makes mechanical decisions only. It refuses with a structured
  reason: `policy_stale`, `model_incompatible` (no substitution),
  `attempt_unsettled`, `uncertain_predecessor`, `request_completed`,
  `attempt_limit`, `no_permitted_route` (routes that are unauthorized, revoked,
  incompatible, exhausted or at their limit) or `allowance_exhausted`.
- A switch is permitted only after a **definitive refusal at the response head**
  (401/402/403/429) when nothing was relayed. The refusal releases that
  attempt's reservation and marks the route unavailable for the rest of the job.
  This is stable routing: later requests go straight to the working route.
- Completed and uncertain attempts keep their reservations; HTTP completion is
  not treated as financial settlement.
- A provider failure event (`response.failed`/`error`) inside a relayed stream
  is `Uncertain(provider-failed-after-output:<code>)` and never switches.
- Each attempt records its predecessor, reason, route, connection, model,
  payer, policy digest, reservation, disposition and normalized usage.
  `route-NNN` (intent, before any connect) and `settle-NNN` controls are
  retained. Provider effects use `Effect::Provider(attempt)`, so every
  attempt has its own unknown-before-I/O lease.
- With no permitted route, the host sends `response-unavailable{code}` and the
  native fetch fails with `route_unavailable_<code>`. Nothing is fabricated as a
  provider response.

## Evidence (one subject: [inputs.sha256](inputs.sha256); [evidence.json](evidence.json); runner [run2.sh](run2.sh))

- Unit: `tests/pi_route.rs` 4/4 covers failover with provenance, stable
  routing, conservative reservations, uncertain/unsettled no-switch, and
  unauthorized, revoked, incompatible-model, stale, allowance, per-request and
  per-route refusals. `tests/pi_stream.rs` is 11/11 (adds the failed-terminal
  event).
- `failover2` (Docker, actual native Pi): the primary returns 429
  `insufficient_quota` and the secondary is normal. The full workflow passes:
  tools, build plus 3 tests, export, three snapshots and sealed readers.
  Attempt 1 is primary/refused (reservation released). Attempt 2 is secondary
  `failover-after-429-insufficient_quota` with predecessor 1. Requests 2–5 went
  directly to the secondary. Primary saw 1 receipt; secondary saw 6 receipts
  and 5 effects.
- `midquota2`: the primary sends two events, then `response.failed`
  `insufficient_quota`. Disposition is uncertain; the secondary received
  **0** requests; Pi recorded the provider error; `provider:1` stays `unknown`.
- `unavailable2`: both routes refuse. Both attempts are refused (reservations
  released, both effects observed). The third decision is
  `unavailable/attempt_limit`. Pi recorded `route_unavailable_attempt_limit`.
- `stream10` (single route), `fault-stall3` and `fault-truncate5` repeat the
  revision-2 controls on this subject with unchanged outcomes.
- Regression on the touched crates: only the four known `claude_boundary`
  ProgramDrift failures. Node: the two pre-existing `pi-memory-guard`-related
  failures.

## Open

- Restart during a switch: attempt intents are durable controls and effects
  are durable leases, but a fresh-process reconciliation that resumes or
  closes a job mid-switch has not been implemented or tested.
- Per-route credential custody: a second credential, exclusion screening for
  every route secret, and real account/model eligibility are phase 6 work.
- Refusal classification is status-based. Provider-specific error codes, and
  whether a quota-limited plan is app-scoped, need real-route qualification.
- Cache-affinity evidence across a switch has not been measured.
- Still open from earlier slices: head-phase cancel, frame/entry retention,
  raw screened SSE custody, phase 4 (replacing the old credential-mount path),
  phase 5 (13-class matrix, broad gate, Astra/xhigh review) and phase 6.
