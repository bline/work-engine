# Session task admission: disabled candidate

The optional local helper inspects an admission request against pinned board
and usage-ledger programs, a protected key policy, and this crate's current
registry. It first returns an exact existing admission, or a `held` view for
an unmatched request when those checks succeed; it cannot commit a new
admission or dispatch. The opt-in authenticated `task_admit` transport only
accepts those read-only outcomes. `task_snapshot` reads a
configured projection; authenticated-read-only HTTP continues to deny writes.
Paths, helper pins, caller identity and trust policy come from operator startup
configuration, never from MCP request fields. Unsupported key algorithms hold.

The private kernels include exact signed-reply matching, current task/proposal
heads with compare-and-swap generations, immutable request/decision identity,
an admission/outbox transaction, an effect-capacity claim, first-funded
observation preservation and conservative result projection. A separate local coordinator command requires a configured credential hash and
performs only CAS registry mutations; MCP never forwards that command. The
credential is a coordinator capability, not a human decision. Its entropy,
protection and exclusion from worker access remain unqualified. The local claim is neither
an upstream monetary reservation nor another spend ledger; an unknown delivery
retains its claim. Test fixtures construct trusted values only inside tests. No
production source currently constructs a qualified admission gate.

The board helper reuses the existing Git-ref reader and owner-answer request
codec. The source runner checks configured hashes, bounds output and execution,
and rechecks source pins. The actual protection of its interpreter, dependency
closure, paths, configuration, state and time against workers is unqualified.
The ledger observer checks fresh complete pricing and signed budget headroom;
that observation alone does not establish exclusive control, all funded paths,
unsettled work or a monetary reservation. Current mode/revocation, exclusive
objective/controller evidence, complete actual spend and action-time rechecks
must be established by protected adapters before admission or delivery.

The result parser can reject a missing, malformed or mismatched worker packet
and requires an exact selected-turn bridge observation, but its production
observation adapter is not wired. The optional bridge acknowledgement checker
is neither a write dispatcher nor proof of confinement. Actual OS grant
realization, path escapes and protection from same-UID workers remain
unestablished; existing bridge provisioning stays read-only. All tests are
synthetic local evidence; live activation and the separately approved exact
task remain outstanding.

The internal `admit_qualified` and `prepare_delivery_qualified` seam now
recollects the configured board, key and budget sources, checks the protected
current heads and atomically fences delivery before an external call. Its
qualification type has no production constructor. A synthetic test-only
constructor exercises the entire path with a fake custody adapter, exact retry,
lost ACK and selected-turn result; it cannot enable runtime behavior. The CLI/MCP path returns an exact existing admission before renewed proof,
current-state or budget checks; it exposes no new admission or delivery. An
unmatched request still yields only held inspection.

The separately versioned `dot.present.one-owned-session-disposable/v1` pilot
uses `scoped_pilot::ScopedOwner`. It verifies a distinct owner-signed activation
in the current canonical board, the current key policy, the independently signed
task and current task/proposal registry. It does not construct the older
`QualifiedContext` or establish its all-objective controller claim. That profile
remains Held. Accounting still uses the canonical scope-budget/usage-ledger
owners for all enrolled funded actors; it is separate from native custody.

A scoped control is an ordinary canonical board request whose
`data.scoped_control` contains `schema: "dot.scoped-control/v1"`, the exact
`profile_sha256`, a consecutive generation starting at 1, the previous request
SHA (null initially), and `state: "active"` or `"revoked"`. The existing
owner-answer/v1 closing signature covers the entire request. Current ingestion
re-verifies the whole matching signed chain, rejects rollback/conflicting heads,
and serializes its projection with the effect journal. A revoked profile cannot
be reactivated; a new reviewed revision needs a distinct signed activation.
The operator CLI is `session-task-admission --config ABS_CONFIG scoped-control`,
with stdin `{"schema":"dot.scoped-control-ingest/v1","profile":{"path":"ABS_PROFILE","sha256":"EXACT_SHA"}}`.
Neither that command, a database credential nor profile JSON can mint approval.

The profile's signed data binds task/proposal/mode/configuration, actor and cost,
parent prerequisite, exact session, immutable message wire digest, native
executable, bridge store, native custody generation, startup transport digest,
disposable root, one file's before/after bytes and digest, publication name and
expiry. Its explicit `one-first-owned-turn` rule permits one actual native ACK
to be durably claimed before a funded response or effect. Another turn/request
cannot replace a missing ACK. The registry extends the exact legacy eight-table
schema through user_version 1; unrelated or incompatible schemas are rejected.
The existing admission decision, capacity claim and outbox remain canonical.
The scoped tables contain control/custody/effect/publication observations, not
money balances or a second approval ledger.

The bridge performs the file write only in its private actual-native
conjunction. Admission exposes authority observations and a journal lease, not a
filesystem executor or native qualification factory. It commits an unknown fence
before I/O, then holds the registry ordering lock while the effect is consumed.
A committed earlier revoke rejects consumption. A crash or missing effect ACK
leaves unknown and never causes automatic resend. File and SQLite commits are
not one atomic transaction. The file is one signed component under the signed
root, opened relative to a held directory descriptor with no symlink following;
its exact prior bytes and single-link regular-file identity are checked.

These controls trust configured signed-board/key sources and source selection
by the same UID. Native descriptor/process checks are sampled. They establish
neither OS-wide nor same-UID isolation, continuous confinement, every funded
actor's exclusive routing, actual upstream billing, nor remote settlement.
Only a separately approved immutable activation/trust-policy/task packet can
supply live authority. Engineering fixtures use fake credentials and provider
responses. A provider token count or native terminal event never manufactures a
completed worker packet, billed usage, settled reservation or closed remote effect.
