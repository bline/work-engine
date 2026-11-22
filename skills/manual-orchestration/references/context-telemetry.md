# Context telemetry identity and limits

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: provisioning or interpreting a context observation, including unavailable measurements.

## Telemetry rules

Choose and verify an owned [session control route](context-control.md) at
startup. A bound CLI wait can collect control observations; Pi can also
project warnings at its next natural request through the separate
context-pressure extension. Neither availability nor a quiet observation
proves timely delivery. Name who checks the binding, collects output and
delivers it to the deciding agent; record the explicit-check fallback when
needed. Telemetry does not automate handoffs.

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
