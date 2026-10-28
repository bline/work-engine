# W1 — offline canonical publication implementation result

W1 implements the accepted `canonical-publication-local-v1` profile in three
Rust owners. `slice-campaign` owns checkpoint inspection, fixture-operator
acceptance, exact offers, decisions, consumption and result references. The new
`workspace-publication` crate owns attributed Git deltas, isolated integration,
tree validation, a durable operation ledger, the guarded expected-old ref
transaction and read-only reconciliation. The new
`work-engine-completion-host` package composes those APIs behind a Unix socket
and supplies `work-engine-completionctl` for explicit operator commands.

The implementation changes no existing review result meaning. Inspection calls
`Campaign::recover_initial_outcome` with real installed claim and episode owner
handles. Only an owner-checked `Reported` outcome can become an inspection
package. Acceptance and `create` are separate records made after the campaign
owner checks the connected peer UID against its installed fixture operator UID.
This profile binds every manifest attribution reference to an exact selected
claim checked by those owners, including its kind, version, campaign root, ID
and digest. It rejects unsupported attribution kinds and foreign roots; it does
not provide a general claim-attribution registry. The operator still decides
the semantic acceptability of the attributed work.
The request schema rejects caller-supplied actor and approval fields. The
trusted installation pins the authority source, UID and fixture mode in the
completion root, and pins the Git repository identity, ref allowlist, validator
and policy in the workspace root. Acceptance, revocation, offer decisions,
consumption and result settlement commit their immutable record, state change
and replay entry in one immediate SQLite transaction. Cancellation competes
with consumption from Open and CreateAuthorized; cancellation or revocation
after consumption records a durable too-late decision. Exact Create replay
reconciles the operation without a second physical ref attempt. A production
authority mode cannot enter the
physical publication path without a future custody installation.

Preparation allocates the operation ID before integration, validates the
complete checkpoint-parent-to-tree manifest, integrates in an operation-private
worktree, runs the installed validator against the integrated tree and writes a
one-parent commit. Its message carries an acyclic preparation digest. The later
admission digest binds the exact offer, authenticated create decision, campaign
consumption, resources and generation. The ref update uses `gix` 0.84.0's
in-process expected-old transaction under a continuous `rustix` file lock in
the Git common directory. This uses local SHA-1 repositories and Unix flock;
it does not claim generic Git algorithms or unmanaged-writer exclusion. The Git
common directory pins one authoritative workspace ledger root; every guarded
operation checks it. A managed checkout child inherits that flock through its
Git process lifetime if its caller dies. Git and validator children start with
a controlled environment, excluding inherited index and repository overrides.
The in-process `gix` ref transaction opens the explicit repository with
isolated permissions and clears its ref namespace, so inherited
`GIT_NAMESPACE` cannot redirect the canonical ref.
The wire request and publication request contain no holder or generation
token, so a caller cannot submit an old generation as authority. A process
test holds the guard beyond a nominal lease interval, observes the same
generation, and refuses a spent request after release with the canonical ref
unchanged. This is a no-TTL guard model, not a token-expiry measurement.

Workspace state distinguishes `Prepared`, `Admitted`, `AttemptStarted`,
`ClosedNoEffect`, `PublishedObserved`, `PublishedUnconfirmed` and `Unresolved`.
`ClosedNoEffect` is durable under the same guard as admission, so a delayed
publisher cannot later mutate the ref. A crash after `AttemptStarted` never
causes a second physical attempt. Reconciliation inspects owner records and
Git without writing the ref. The supported uncertain outcomes retain exact
operation and observation references for later human handling.

The joined offline fixture uses the existing J2 campaign, claims and native
episode owners. A test-only hook exports the original controlled fixture
evidence fields and reference digest; the host reconstructs them through
`FixtureRecord::controlled` and refuses a changed record reference. It also
installs the exact existing completion selection on restart. A separate child
process runs the completion-host library's socket server, and another process
reopens the stores and reconciles the observed commit. The fixture test
executable must create and read the native episode root because the current
episode owner pins its reader to the root-creating executable. A separately
built `work-engine-completion-host` executable cannot reopen that test root:
it correctly fails `native selection executableSha256 differs`. An additive,
trusted read-only handoff from the episode owner, or a fixture created by the
completion executable itself, is required before claiming that exact
standalone-binary joined path. W1 has not weakened the episode pin. The campaign
root permits one full writer host at a time. Actual two-process decision races
use a test-faults-only completion-ledger opener bound to the original campaign
root ID and operator UID; it does not activate a second host.

Targeted qualification covers the J2 owner-to-socket-to-Git-to-campaign path;
restart; wrong manifest and displayed digests; forged actor/approval wire
fields; a changed fixture evidence reference; production-mode refusal;
completion-root UID/mode pinning; exact prepared commit and receipt reuse;
stale parent closure; delayed publisher versus durable closure; concurrent
publisher and managed checkout actors across a held guard; process kills after
admission, attempt start, physical CAS and before result finalization; and an
unmanaged different-parent writer causing expected-old CAS failure. Additional
proof covers pre-consumption cancellation, late cancel/revoke records, exact
Create replay, process cuts before acceptance/decision/revocation replay and
after decision commit, four actual-process revoke/cancel versus consumption
races in both start orders, a second ledger rejected after an uncertain first
attempt, a paused checkout child retaining the guard after parent death, and
poisoned inherited Git selection with byte-identical attached branch, index
and worktree after preparation, plus a full publication under poisoned
`GIT_NAMESPACE` that moves the canonical ref while preserving the namespaced
ref. Reopened-store tests independently reconstruct the preparation and
admission digests, refuse changed request references, and distinguish missing
or tampered admission from confirmed publication. An alternate, individually
readable create-decision reference is refused after reopening both publication
owners because it differs from the exact
decision bound by durable campaign consumption; the canonical commit stays
unchanged. The preparation proof reads the actual Git commit tree, parent and
message after reopening the workspace owner.

Joined host kills cover durable campaign consumption before workspace
admission and closure before campaign settlement. Joined process tests exercise
both admission-guard orders against actual campaign consumption. Owner-level
and joined negative tests distinguish
an unestablished owner outcome, missing or foreign selected-claim attribution,
wrong checkpoint, stale offer and changed validation policy. A full successful
publication also preserves the attached branch, index, worktree, unrelated
paths and accepted checkpoint ref. An unmanaged checkout and an external
A-to-B-to-A ref sequence demonstrate what the enrolled guard and OID CAS
cannot establish.

Durable qualification manifests and logs are under
`/home/bline/.local/state/work-engine/w1-20261018`; disposable test fixtures
and build output are under `/run/user/1000` due limited workspace filesystem
capacity. An aggregate v4 joined run returned an initial inspection refusal
before its guard-order actors started; a later aggregate retry failed during
fixture export with `ENOSPC` on the root filesystem. The focused guard-order
test passed, but the first aggregate refusal's cause is unproven, so neither
failed aggregate run is counted as successful qualification. The final v5
aggregate outcome belongs in its separate test receipt. The
selected fixture validator is `/usr/bin/true`; it does not mutate canonical
Git state. The tests do not establish independent operator accounts,
production filesystem custody, arbitrary validator confinement, a provider
review, push, deployment or HP4 dispatcher activation.

W1 uses `gix = 0.84.0`, `rustix = 1.1.5`, `rusqlite = 0.37.0` with bundled
SQLite and `serde_json_canonicalizer = 0.3.2`. New record payloads use the
named `workspace-publication-rfc8785-v1` digest domain; existing campaign,
claim and episode codecs are unchanged. The fixture export hook and its dev
dependencies are the only scope addition to
`rust/crates/review-workflow/tests/campaign_owner_join.rs` and its manifest;
there is no review-workflow production-semantic change.

No commit, push, production activation or provider action is part of this
result. The attached user branch, index and unrelated files were not used as
publication targets. The test repository uses an existing local target ref;
the accepted checkpoint commit remains unchanged.
