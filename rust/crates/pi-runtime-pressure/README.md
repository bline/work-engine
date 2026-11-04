# Provider-disabled Pi runtime pressure qualification

This standalone Rust harness uses the existing Pi custody Host. Its fixed
`work-engine.pi-pressure-fixture.v2` packet is available only on the ordinary
provider-disabled coding fixture. The live, signed, and observation-only
profiles keep their separate authority and behavior.

After actual native readiness, the fixture asks the same SDK session for its
model/window, idle state and `getContextUsage`. The outside caller binds a
source and generation to that physical session, invokes the unchanged pinned
`pi-context-observer` executable, and preserves the canonical budget producer's
raw `usage_ledger.py control` output and cursor. Separate executables preserve
the accepted observer's `time 0.3.47` dependency and the Host's `0.3.55` pin.
The receiver inserts the exact composed text without a turn, then the existing
fixed fixture prompt supplies the next fake stream request. Native message,
raw request context, history, Host receipt, receipt ACK and owner acceptance
remain distinct observations.

SDK tokens are estimates. Undefined usage and post-compaction null tokens stay
separate in `sdk_usage`; unavailable evidence does not become zero. The source
owns generation, current identity, clock and sequence. One input is allowed per
fresh job. Wrong identity/model/window, stale/future readings, non-first sequence,
changed budget bytes, oversized or unknown wire fields, and deadline/cancel fences
are refused. No payload grants provider/task permission, reset or recovery.

`prepare OUT TEMPLATE BUDGET` inventories immutable inputs before assigning the
job deadline. `run JOB OBSERVER_PIN` admits a fresh job through Host. `recover
STATE` uses that same executable's existing Host recovery path and never creates
or starts work. Preserve the executable bytes for the journal's lifetime.
`--cancel-and-interrupt-at-ready` is a qualification-only fault: publish the
existing cancellation marker, exit 75, and collect through the same binary's
`recover` entry. A collected incomplete run remains `not_established`.

The original checkpoint operation and its emergency cleanup remain closed
history; a new successful receipt cannot settle that old unknown journal.
This package qualifies a local fixture boundary, not shared MCP/service delivery,
model comprehension, live inference, a lifecycle transition or full S10.
