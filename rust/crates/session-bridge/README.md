# Optional session target provisioning

This component owns local provisioning state and a held native connection. It
is disabled by default. This source requires separate code review and a
profile-specific operational qualification before use. Nothing here changes
Codex trust or configuration, starts a service from an MCP request, establishes
human approval, or authorizes a live deployment.

A trusted operator initializes provisioning in the existing private bridge
store, supplies a private policy file, issues a private bearer file, and starts
one owner process explicitly. The store retains its identity and the existing
message-grant tables. An owner holds at most one target at a time. Its private
socket is under the bridge lock directory and its exclusive lock prevents two
cooperating owners. Restart removes a refused stale socket only when its
persisted device/inode marker matches; an unproven or live socket is left in
place for operator reconciliation. Between requests the owner serially consumes
native notifications and responds to WebSocket Ping control frames. A stalled partial or fragmented message has a
fixed deadline; failure fences custody. The native target is not fenced against
unrelated TUI or other native clients.

The policy is strict JSON (`schema_version` 1); unknown fields are rejected.
The following is a shape, not an operational policy. Replace every placeholder,
use absolute paths and Unix-second UTC deadlines, and keep the policy and
credential in an operator-only directory:

```json
{
  "schema_version": 1,
  "task_revision": "accepted-task-revision",
  "profile_name": "diagnostic-read-only-v1",
  "workspace": "/absolute/existing/git/workspace",
  "checkpoint": "40-or-64-lowercase-hex-git-commit",
  "model": "gpt-6.1-terra",
  "reasoning_effort": "medium",
  "service_tier": "default",
  "sandbox": "read-only",
  "network_access": false,
  "approval_policy": "never",
  "global_config": "/absolute/operator-config.toml",
  "global_config_sha256": "64-lowercase-hex-digest",
  "workspace_config": "/absolute/existing/git/workspace/.codex/config.toml",
  "workspace_config_sha256": "64-lowercase-hex-digest",
  "request_id": "canonical-uuid",
  "payload_sha256": "64-lowercase-hex-digest",
  "admit_until": 0,
  "close_by": 0,
  "budget_reference": "operator-budget-record",
  "budget_units": "approved-units",
  "budget_amount": "approved-amount",
  "reserve_controller": "operator-controlled-reserve-owner",
  "reserve_amount": "separate-reserve-amount",
  "first_funded_clock": "supervisor-owned-clock-reference"
}
```

Admission must be in the future and at least 15 seconds before close-by. The
current implementation qualifies only the named read-only model/control shape
above; workspace-write profiles need separate implementation and qualification.
The checkpoint must be the clean Git HEAD, including no untracked or ignored
files other than the pinned workspace config. That config may be tracked or
ignored, but must match its pinned hash. Checkpoint verification invokes only fixed
`/usr/bin/git` arguments in a cleared environment, disables fsmonitor and
hooks, rejects configured filters, applies a deadline, and kills the owned
process group on timeout. The owner checks config files, effective project layer, disabled controls and
native ACKs. It does not itself write those files or confer trust. Because an
external actor can change a file between observations, qualification must bind
operational control of those files and the native socket as well as their
hashes.

With `<bridge>`, `<config>`, `<policy>`, and `<new-private-credential>` denoting
operator-selected private paths, the explicit local lifecycle is:

```text
<bridge> --config <config> provisioning init
<bridge> --config <config> provisioning issue --policy-file <policy> --credential-out <new-private-credential>
<bridge> --config <config> provisioning owner serve
<bridge> --config <config> provisioning revoke --permission-id <permission-id>
```

`owner serve` is a foreground process owned and collected by the operator. Set
`SESSION_BRIDGE_PROVISIONING_ENABLED=1` only in a separately qualified
session-events profile. Deliver the parent credential over an authorized
private channel; do not put it in logs, shell arguments, board messages, or
native input. MCP creates using that credential and a canonical UUID
idempotency key, reads target state, supplies the preapproved exact initial text
to issue one child handle, submits with parent credential plus handle through
the existing runtime envelope, reads operation/result, and closes. The handle
is not a bearer credential. Other task permissions cannot use it.

Allocation and synthetic-initialization uncertainty occupy the slot and are
never blindly retried. The store records known target identity, generation,
setup, child handle, and closure observations. After a restart, a ready target
becomes `custody_lost`; no new submission is admitted merely from a saved ID.
A close may reconnect only to check the known target and workspace. The owner
also attempts bounded cleanup near close-by for durable unresolved reservations
after restart; loss of the owner can still leave cleanup uncertain. An
acknowledged archive can be followed by a read to observe `notLoaded`.
A lost archive acknowledgement remains uncertain even if a later read reports
`notLoaded`; the owner records that observed status and timestamp separately
and never replays the archive. Unknown allocation cannot be
called closed. If the owner is absent, target read reports
`owner_health: unavailable`; it cannot establish connection health. Reconcile
uncertainty with the operator rather than resetting or cloning the store.

The operator supplies budget and clock references; this component does not
measure provider spend, settle costs, release reserves, or start/reset a UI
clock. Provider usage coverage and the first-funded time must be observed by
the supervising owner. Native operations and cleanup use bounded deadlines;
process loss can prevent observed closure and is reported as uncertainty.

A proposed first qualification is a separately authorized disposable,
read-only workspace and checkpoint, one harmless exact message, and the path
create → issue → submit → correlated completion → close, with native and
callback evidence, a fresh short-lived capability, and complete cleanup. Bind
the actual profile, payload digest, expiry, owner, and observation plan before
that test. Fake-peer tests establish only source behavior; they do not establish
native API compatibility, Dot behavior, callback delivery, or deployment.
