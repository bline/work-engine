# Unified operator interface — first increment

User authorization: this conversation, 2026-11-12, taking over development after
the interface exploration. Browser and terminal access are equally important.

Status: the first increment is implemented locally and qualified as recorded in
[the implementation result](../records/unified-operator-interface-first-increment-20261113.md).
This is development evidence, not deployment or formal workflow UI acceptance.

The [next delivery plan](unified-operator-interface-next-plan-20261113.md)
proposes the broader shared operator outcome, delivery order and manual
orchestration after the user's request to plan before further development.

The first increment connects explicit native sessions to a shared, read-only
activity observation. It does not equate a session with a task or reconstruct
task/campaign state from prose. The Events server's existing pinned compositor
owns transcript parsing and redaction. A new Rust consumer owns only a bounded
presentation snapshot. CLI, TUI and authenticated web consume the same version.

Placement: `rust/crates/operator-observation` owns the projection DTO, validation,
collector and `work-engine-observe` binary. The existing terminal UI adds an
explicit observation mode. `../devwits-auth` adds a dashboard projection and an
optional file transport through its existing authenticated mailbox sync. The
default sync and approvals retain their existing owners. No task-host or
research-admission source is in this change's ownership.

Consequence: select sessions once; observe the same attributed recent activity,
source unavailability and receipt age in the terminal and browser. Missing data
cannot become empty success, running/completed work, settled spend or authority.
Scope changes and source failure remain visible. Snapshot collection is polling,
not an event subscription or complete history. The web retains its 60-second
mailbox refresh; the TUI refreshes the local snapshot. Deployment and persistent
service activation are separate operational actions.

Qualification: a controlled pinned Events owner reads disposable transcripts;
the resulting Rust snapshot passes both actual consumers. Test wrong identity,
unsupported schema, malformed and unavailable records, empty observations,
refresh failure, scope changes and opt-in sync cache invalidation. Run focused
Rust/Node/Python checks and browser/terminal rendering evidence. No live provider
execution, approval signing or production write is needed.

Next work: owner-backed task/campaign joins; event notifications/replay and faster
browser transport; budget/closure observations; owner-admitted actions. These
remain open and are not implied by this first increment's acceptance.

The current working-tree Events owner differs from the two source digests
accepted by `session-query`. This increment preserves those checks and uses an
exact retained accepted owner for qualification and local session reads. Qualify
the current Events revision with the query owner before switching the monitor's
source profile to it. Do not make pin acceptance a UI concern.
