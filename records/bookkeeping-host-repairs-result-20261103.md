# Bookkeeping host repairs: accepted and integrated, 2026-11-03

Source `bdf564f9bbe4568b740323fc74c49bd42bf7150e` is integrated on the shared
branch. Six task-owned files changed; 16 unrelated modified tracked files were
preserved. The source is release-ready and has not been globally activated.

The host now provides:

- `ack --state DIR --ack-receipt ID`: acknowledges exactly that receipt without
  polling, effects or a new receipt. Its summary distinguishes an old consumed
  receipt from the latest task receipt's delivery.
- `inspect --state DIR --summary [--operation ID]`: one read-only view of recorded
  identity, requested/observed settings, sandbox evidence, context, configured
  canonical budget controls, result, failures, processes and delivery. Missing
  evidence stays explicit; it makes no fresh-observation/liveness claim.
- Automatic 0700 preparation of authorized output parents. Authority/objective
  validation precedes creation; canonicalized existing outputs, dangling links,
  path reservations and conflicting operation identities remain protected.
- An unavailable receipt after selected-turn reader loss, without further turn
  polling or automatic resume/permission changes. A later collection can recover
  the same known operation. Workflow phase and native outcome remain distinct.

Initial review caught MHR-001: making a missing directory could expose an old
output via `new/../result.json`. The correction rechecks the canonical output
slot before admission. The bounded correction and final unavailable-read delta
were accepted by separate fresh Astra/high/default reviewers. Reports preserve
exact subjects and reconstructed-context limits. Changed guidance received
applicable/retain instruction review.

All 44 lifecycle tests pass on both private and integrated source. Formatting,
Clippy with warnings denied, skill catalog/synopsis checks and immutable release
provisioning pass. The fresh Sol6.1/low/default smoke task exercised missing
output parents and returned the expected JSON. Its session unloaded before
later collection, which produced the new explicit unavailable receipt. Root
manually loaded that same known session with explicit read-only/never/default
settings; the returned policy was readOnly/networkfalse. No new turn started;
complete captured history contains one turn. Collection then succeeded.
ACK-only preserved operation/observation state, receipt count and identity.
Final task and accounting receipts are acknowledged without a successor pending.

A historical operation's raw JSON was 897386 bytes; the compact operation view
was 6243 bytes. This is a representation-size comparison, not measured root-time,
turn-count or cost savings. The historical source/pins/receipts were not rewritten.

[Reviewer-budget diagnosis](bookkeeping-host-budget-diagnosis-20261103.md)
records that the first node limit was configured after completion and the delta
limit after launch. Inspection exposes those controls. Automatic per-role billing
caps and native interruption policy were not added.

The new tested bundle is `68736703ade77d6cd125fec73e15548d64728f3801839c040353ae0cdd40ae22`;
binary SHA256 `9a8536e165cb6d56a6ef773db221fe7527e75e62964c5213600e4e4e40a748a7`. Full absolute paths, build custody and
exact release comparison are in [the structured result](bookkeeping-host-repairs-result-20261103.json).
Existing states retain their exact original executor; use a new state for this
binary. Global skill export alone does not install it. The prepared source
release remains subject to the release owner's user-decision boundary.

Observed spend is **$10.221724 / $25** at 2026-11-03T04:24:28.954137+00:00, complete recorded
coverage and zero unpriced calls, with subsequent reporting usage unsettled.
The supervisor's two-hour elapsed guard was exceeded: source/review/native turns
were admitted before 03:36:13Z; final collection/integration/closure followed the
user's 04:16:15.295Z continuation. The clock was not reset and no numeric budget
extension was inferred. The funded objective is now closed, every required role
and process is collected, and publication/coordination release are the only
remaining closure actions. No new features or paid child work are admitted.

Evidence root:
`/home/bline/.local/state/work-engine/bookkeeping-host-repairs-20261103/`.
Its assignment, continuation, exact review reports, custody, native state,
accounting receipts and integration plan retain provenance and limitations.
Automatic load recovery and retained reviewer grant qualification remain future
work; the recorded manual collection-only load is not an automated recovery claim.
