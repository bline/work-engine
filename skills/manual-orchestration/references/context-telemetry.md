# Context telemetry identity and limits

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: provisioning or interpreting a context observation, including unavailable measurements.

## Telemetry rules

The [session control subscription](context-control.md) can supply pressure
observations through the board wait interface. The supervisor establishes an
observation route at startup; this subscription and explicit checks are
alternative mechanisms, not evidence that either is already active. It preserves these
decision owners and the explicit checks as fallback; enrollment, output
collection and model delivery have named owners. It does not automate handoffs.

- Provision the binary once per tool version with
  `scripts/provision_context_status.sh`; it builds outside the shared Cargo
  target, pins by digest under `~/.local/state/work-engine/tools/` and records
  the source commit. Send path and digest to every child in its identity
  binding. Never rebuild in the shared workspace to check context: build locks
  disable observation when work is busiest.
- Identify sessions by verified `session_meta` lineage, never by role name or
  an inherited `CODEX_THREAD_ID`, which can name the parent.
- Unavailable usage is `usage_unavailable` with its class - launch or tool
  failure, identity rejection, absent counters. Never substitute parent
  counters or treat it as zero.
- `epoch` also changes on model, window or availability changes; count
  compactions from lineage, not epoch, and never count an epoch as a handoff.
- The fraction is a proxy for request input, not live allocation. Record
  economics as `not estimated` unless a scenario supplies all five restart-cost
  inputs to the CLI ([guide](../../../docs/context-status-cli.md)).
