# Dev vertical: routing recovery after restart mid-switch, 2026-11-16

Standing: bounded offline phase-3 completion item. Successor to
[route failover r2](../dev-vertical-route-failover-r2-20261116/README.md). Not
acceptance, not the Astra/xhigh review, not live qualification.

## What changed

- The probe retains the pinned routing policy as `route-000`, before any
  attempt is made.
- `Router::recover(policy, intents, settlements)` rebuilds routing state in a
  fresh process:
  - It rejects a policy-digest mismatch, a gap in attempt numbering, or a
    settlement without a matching intent.
  - It replays settlements: refusals release their reservation and mark the
    route unavailable.
  - Any intent without a settlement becomes `Uncertain(restart-unsettled)`. Its
    reservation is kept and that request cannot be retried or switched.
- `examples/pi_route_recover.rs` is a read-only fresh-process recovery. It reads
  only retained route/settle controls; it has no access material, transport,
  registry mutation or dispatch. It reports what the router would now permit
  for each request it has seen.
- Probe crash injection (`PI_PROBE_CRASH=intent:N|connect:N`) exits the host
  process at those points, without any cleanup.

## Evidence (one subject: [inputs.sha256](inputs.sha256); [evidence.json](evidence.json); runner [run2.sh](run2.sh))

| Run | Crash point | Registry `provider:2` | Recovered attempts | Next for request 1 |
| --- | --- | --- | --- | --- |
| `crash-intent-2` | after the durable intent of failover attempt 2, before its lease | absent | 1 primary refused (released), 2 secondary `restart-unsettled` (1 unit kept) | `uncertain_predecessor` |
| `crash-connect-2` | after the request was sent to the secondary | `unknown` (secondary observer: 1 receipt) | same | `uncertain_predecessor` |
| `failover4` | none | `observed` | 6 attempts, all settled, 5 units kept | `request_completed` for all 5 requests |

- The unit test `fresh_router_recovers_mid_switch_without_retry_or_lost_attempts`
  checks that stable routing persists across restart: a new request goes
  directly to the secondary. It also checks that policy mismatch, intent gaps
  and orphan settlements are refused.
- `stream12` (single route) passes on the same subject.
- `work-engine-manual-host` regression: no failures in this run. The four
  `claude_boundary` ProgramDrift failures belong to the other crate and were
  not re-run here.

## Limits

- Recovery keeps an intent with no registry effect row conservatively
  uncertain, although the lease-before-I/O rule means no provider I/O occurred.
  Distinguishing `not-entered` safely is left to the continuation owner.
- The crashed host leaves its container running. The runner removed it
  (`leftover-containers.txt`). Host-level reconciliation of orphaned
  containers after a crash belongs to the existing acquisition/cleanup owners
  and was not re-qualified here.
- Recovery trusts the integrity of the retained create-only controls. It does
  not yet join to the sealed reader or registry in-process; the registry join
  above was made by the runner.
- Phase-3 limits that still apply: synthetic routes share one credential,
  refusal classification is status-based, cache affinity is unmeasured.
