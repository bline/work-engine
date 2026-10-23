# Filesystem experiment control

The first implementation of the
[exclusion contract](agent-experiment-exclusion-contract.md) is a Rust reducer
and a local CLI adapter. It stores experiment state in private Git refs, with
no daemon. This is the first, unreleased schema version; no earlier prototype
aggregate was enrolled in a live experiment during this implementation. There is
no compatibility or migration promise for intermediate development fixtures.
This is a managed input and ownership boundary for cooperative
experiments, not an OS sandbox or a replacement agent launcher.

## What it actually controls

| Surface | Current consequence |
| --- | --- |
| Pinned startup and artifact bundles | Returns the captured bytes only after checking the binding/policy and durably issuing a receipt |
| Experiment mailbox | Checks sender and receiver permissions; checks the receiver again before output; preserves branch replay position across handoff |
| Membership and handoff | Explicit preparation, identity attachment, verifier attestation and activation; generation checks reject stale owners |
| Phase closure | Rejects closure while bindings, prepared transfers or output receipts remain unresolved; execution and scoring can close independently |
| Identity and exposure evidence | Records attributed assertions and evidence references; does not independently read or attest provider runtime state |
| Native Codex/Claude startup, global skill discovery and direct messages | Not intercepted; the launcher must arrange and verify them under the experiment's rules |
| Existing shared chatboard and context-control listener | Unchanged; they are not automatically scoped by this registry |
| Other filesystem reads, tools, process/child ownership, quota and caches | Not fenced by this adapter; declare cooperative controls or uncontrolled limits |
| Multiple simultaneous experiments governing one actor | No cross-registry reconciliation; unsupported as an enforced claim |

Do not claim full branch isolation merely because registration succeeded. The
specification must declare limitations for `host_startup`, `direct_messages`,
`filesystem_access` and `shared_resources`. `managed` coverage is accepted only
for `artifact_delivery`, `mailbox_delivery` and `membership_transitions`.

Caller identities and evidence references are **asserted**, not authenticated.
The registered owner and its delegates are checked against those assertions.
Anyone with unrestricted filesystem/CLI access can impersonate an actor or read
the underlying aggregate, including all arms. This is useful for preventing
accidental exposure through cooperating tooling; it does not establish hostile
actor separation, independent attestation or complete blinding. Identity matching
is exact over the asserted host/session/agent strings. The adapter cannot detect
aliases, differently cased identifiers or an invented identity for the same host
session; the launcher/verifier owns that check.

Artifact IDs, resource labels, handles and content remain visible. For a blinded
role, the owner must supply neutral labels and suitably blinded content. Tests
of omitted arm/profile fields establish structural projection only. Neither this
adapter nor its hashes establish semantic blinding, allocation balance or causal
validity. Experimental units are recorded labels; assignment/randomization and
whether a unit may appear in multiple arms belong to the experiment design.

## Build and storage

```sh
cargo build --locked --offline --manifest-path rust/Cargo.toml \
  -p experiment-control --target-dir /absolute/private/target
sha256sum /absolute/private/target/debug/experiment-control
```

Use an immutable provisioned binary and its SHA-256 in subsequent calls. Linux
is required by the adapter's filesystem lock and opened-file execution binding.
It checks the digest and executes the same opened inode; concurrent in-place
modification of a provisioned binary is unsupported.

The adapter is
`skills/durable-state/scripts/experiment_control.py`. It uses the existing
`GitRefDurableStateStore` with namespace `experiment-control-v1` and the
experiment ID as its key. This does not move HEAD or modify the index/worktree.
Linked worktrees share that store and its lock. A separate repository can also
be used as the experiment's storage location.

One aggregate owns spec revisions, inline artifact bytes, bindings, generations,
receipts, mailbox positions and audit entries. Spec revisions, captured artifacts
and appended audit entries are immutable. Bindings, receipt statuses and mailbox
positions change through commands; their earlier values remain in store history. The existing store retains prior revisions and
their integrity metadata. Exported JSON is a view, not another writable owner.

The adapter locks a per-experiment file under the common Git directory, reads,
reduces, CAS-publishes, emits output and flushes before releasing the lock. The
sidecar is never unlinked during normal operation. All cooperating adapters use
the same lock. A CAS conflict returns an error and emits no result; delivery is
never automatically retried. CLI stdout backpressure has a default 10-second
deadline; lock acquisition defaults to 30 seconds. Set
`--output-timeout-seconds` and `--lock-timeout-seconds` to finite positive values
up to 300. A timeout releases the lock but may leave partial output and an issued
receipt. These deadlines bound pipe/lock waiting, not filesystem hangs or an
arbitrary callback supplied by Python code. The host still owns process
collection/cancellation. Give each CLI invocation its own output pipe: temporarily
setting nonblocking mode affects any writer sharing that open file description.
An emergency native stop need not wait for this CLI.

Storage is deliberately bounded: 512 KiB per artifact, 2 MiB initial snapshot
per specification, 16 spec revisions, 512 bindings and 2,048 output receipts.
Ordinary mutations stop at 8 MiB or 4,096 audit entries. One-way recovery commands
(`pause`, `retire`, `acknowledge`, `dispose`, `close_phase`, `seal_admissions`,
`revoke_delegate`) have reserved headroom up to 32 MiB and 12,288 entries;
repeating a completed transition is refused. Each spec permits at most 32 phases
and 64 distinct delegates; evidence references and dispositions are bounded to
512 bytes. Payload bytes are not duplicated in audit entries. `resume` and
`inspect` use ordinary capacity. This reserve supports settling a full registry;
it is not immunity to storage faults or an unbounded recovery guarantee. The
receipt, binding and revision caps have no startup reserve. Exhausting them, or
entering the byte/audit reserve, can permanently prevent ordinary continuation
and handoffs in this registry. Settle and close it; any later experiment needs
explicit new admission with prior exposure retained, never a silent fresh retry. A rejected command appends nothing. These
are implementation capacity limits, not automatic experiment end conditions.
Retaining each aggregate revision has storage cost proportional to its encoded
size; this first adapter is intended for small experiments and input bundles,
not whole repository copies or transcript archives. Limits fail explicitly;
there is no automatic pruning or history deletion. Invalid/corrupted retained
state fails closed; no automatic repair is attempted. The owner can inspect
retained Git state directly for recovery when managed inspection is unavailable;
that bypass is privileged and unblinded, not a participant route.

## Specification and requests

[The synthetic specification](../rust/crates/experiment-control/examples/specification.json)
shows control/treatment participants, a blinded reviewer and an observer.
Its identities, evidence and instructions are fixtures, not authority to run an
actual experiment. Replace them with the experiment owner's accepted design.

The public [Input schema](../rust/crates/experiment-control/schema/input-v1.json)
defines `Specification`, `Request` and every command. The Rust CLI can regenerate
it with `--schema`. Unknown fields and duplicate JSON keys are rejected,
including duplicates inside nested policy maps.

Each profile selects an arm (or `null` for an outside evaluator), an environment,
a required role-instruction artifact, exact grants, carryover policy and whether
observed-exposure evidence is required. Grants use `read`, `publish`, `send`,
`receive` or `dispatch` plus an exact resource identifier. There are no wildcard
or prefix permissions. Sharing a resource between profiles is an explicit
permission to share that class of input; different arm labels do not override
the grants. A `dispatch` grant names the permitted child profile.

Artifacts contain `data_base64`, `sha256`, `resource` and source `provenance`.
They are captured bytes, not paths or unretained Git object references. To
capture a bounded regular file for inclusion in a specification or import:

```sh
/absolute/pinned/experiment-control snapshot /absolute/instructions.md
```

That returns `data_base64` and `sha256`. Supply the resource and provenance when
constructing the artifact record. The source changing later cannot retarget
these bytes. Capturing bytes does not establish that an agent received them.
Use worktree/source revisions separately in the experiment's declared evidence;
the adapter does not create or enforce a repository checkout.

Write a Request JSON file with `id` (operation ID unique per asserted actor), `actor` (explicit
`host`, `session`, optional `agent`), `observed_at` (attributed observation text),
and `command`. For example, a participant status request is:

```json
{
  "id": "status-001",
  "actor": {"host": "codex", "session": "actual-thread-id", "agent": null},
  "observed_at": "2026-10-23T18:30:00Z",
  "command": {"op": "status", "handle": "participant-007"}
}
```

```sh
python3 skills/durable-state/scripts/experiment_control.py \
  --repository /absolute/experiment-store \
  --experiment example \
  --binary /absolute/pinned/experiment-control --sha256 '<sha256>' \
  --request /absolute/request.json
```

Without `--request`, the adapter reads stdin. Output is one JSON object with
`revision` and `result`; exit 1 reports failure on stderr, and exit 130 is
cancellation. Policy/CAS refusals emit no result. An emission error or timeout
can leave partial JSON after a successful publication; discard that output and
recover the retained receipt rather than assuming the operation did not occur. `--expected-revision <revision>` adds a caller's
optimistic concurrency precondition; `absent` requires initial registration.
The raw Rust reducer's stdout includes private next-state data: **do not use it
as a participant-facing tool**. Only the adapter publishes the state and emits
the selected result.

## Admission and a first delivery

1. **Register:** the owner sends `register` with the complete specification.
   Retain the returned spec digest. Bootstrap authority is caller-asserted.
2. **Prepare:** an admission authority supplies a neutral handle, spec digest,
   phase, profile, experimental unit, branch, optional parent/replacement and
   continuation artifact IDs. A parent needs the declared dispatch permission;
   its child cannot escape the parent's spec, phase or arm through that route.
3. **Startup:** the launcher requests the prepared handle's startup bundle with
   a new local receipt ID. A nonadministrative parent also needs `read` grants
   for every startup/continuation resource; dispatch alone cannot disclose them.
   An `admit` delegate, like the owner, is deliberately an unblinded launcher.
   The result contains assigned artifact IDs, bytes and digests, without added
   arm/profile labels or source paths. Collect it and acknowledge the **returned
   opaque receipt token** with evidence, not the input local ID. This prepares the bundle; no provider is launched.
4. **Bind:** attach the actual identity and evidence references, the supplied
   and observed artifact sets, known prior bindings and inherited-context
   evidence. Required observed exposure cannot be replaced by an empty record.
5. **Verify:** a spec-named verifier records its identity/exposure attestation.
   This is a separate attributed decision, not independent host verification.
6. **Activate:** an admission authority names the expected branch generation.
   The returned generation is required for subsequent participant operations.
7. **Deliver:** the active participant requests permitted artifact IDs and a
   new local receipt ID. Missing identity, wrong generation, pause, closed phase or
   absent grant refuses the managed output without printing withheld bytes.

The launcher must arrange the frozen instructions and global discovery surfaces
before provider startup can expose them. If that host route cannot satisfy the
experiment, stop that admission or obtain the owner's explicit disposition;
registering after launch does not undo earlier exposure. The included synthetic
tests exercise the whole local sequence without launching real agents.

`evaluate` reports whether a grant matches, with `not_a_permit: true`; it is
never a reusable authorization for later delivery. `status` is a small neutral
participant projection. `inspect` records its issuance in the audit log. The
owner can inspect the full private aggregate; a delegated inspector receives
only the requested revision's specification, bindings, receipts, imports and
branches, after all phases in `inspect_after` have closed. An empty set allows
immediate delegated inspection. The owner remains unblinded recovery authority;
never assign that identity to a blinded participant or pass its output to one.
Inspection issuance does not prove collection or consumption.

## Messages, imports and evidence

`send` publishes an attributed message to named branch inboxes. It requires the
sender's exact `send` grant and each current receiver's `receive` grant, within
the same spec revision and phase. Message IDs are local to the sending binding. `receive` checks the active receiver again,
returns at most the requested limit (1–100), and issues a receipt. A collected
receipt advances the branch position. An unresolved mailbox receipt blocks a
second receive, avoiding silent cursor advancement or duplicate consumption.
An empty receive is read-only. There is no continuous watcher in this adapter;
the host owns when it calls and collects results.

`import` admits a new immutable artifact, such as a handoff or blinded result.
A producing participant needs a `publish` grant for its resource, or an
intervention authority records the outside import. It records provenance and
evidence, refuses replacement of any existing artifact ID, and does not change
the frozen environment. Receivers still need their own `read` grants.
Source/digest checks cannot prove that prose is free from another arm's influence.

Input receipt IDs are local to the binding; returned opaque tokens identify the
stored receipts used by `acknowledge` and `dispose`. If output is lost, `status`
reports pending tokens and owner inspection supplies their local IDs. This
avoids a shared receipt-ID collision namespace, not all possible timing or
capacity side channels in the shared store.

An output receipt starts as `issued`: publication succeeded, but emission and
model consumption remain unobserved. `acknowledge` records collection with
evidence; it does not prove an action's consequence. If the process crashes
after publication, the receipt remains issued even if no bytes reached stdout.
Do not blindly retry the same request ID or infer failure meant no state change.
Inspect the retained state and settle the receipt explicitly.

`dispose`, reserved for intervention authorities, records a stranded receipt's
evidence and measurement disposition. It does not claim collection and does not
advance a mailbox cursor. The owner must establish producer collection and
consider possible exposure before proceeding. A successor may see repeated
messages after such a disposition; it must not treat them as new instructions
merely because their delivery repeated.

## Handoff, amendments and closure

Publish a continuation artifact, then prepare a replacement on the same branch
with `replaces` naming its current owner, or its last owner after retirement. Its spec, phase, profile and unit stay
bound. Collect its startup, bind its actual identity and obtain verification.
Activation requires the current branch generation, all predecessor output
receipts settled, and process/child settlement evidence. One atomic update
retires the predecessor and advances ownership; competing or stale activations
fail. Pending replacement records remain inspectable after a crash.

Branch mailbox position survives; native process handles and telemetry bindings
do not transfer through this registry. The parent must follow its existing host
handoff rules and enroll telemetry against the successor's actual identity.
Retired participants retain only the explicit receipt-acknowledgment/status
paths, not artifact delivery or dispatch. Prior identity reuse must be declared
and permitted by the profile's carryover policy; a new label cannot remove it.
Inherited context from other sessions remains an attested exposure surface.

`amend` registers an additional immutable spec for explicitly selected future
admissions. Revisions **coexist**: existing bindings stay on their prior revision,
and its admissions and delegates remain valid until explicitly restricted.
Only the owner may change delegates, `inspect_after` or coverage declarations.
A non-owner `amend` delegate may revise experimental policy/inputs but must keep
those fields unchanged; revocations from the source revision are carried into
its new revision. This follows the chosen source revision only: revocation in
one sibling revision does not revoke the identity in another. For an
experiment-wide administrative withdrawal, revoke it in every revision that
still lists it. Owner-authored amendments start with no revocations; retaining
a delegate in that new specification reinstates it, so remove unwanted delegates
before publishing. Audit each new revision's authority and inputs, including
those authored by delegates. These powers do not migrate or cancel existing
bindings.
`seal_admissions` prevents all new preparations for a named revision and phase,
including replacement handoffs and resuming a retired branch; already prepared
bindings remain eligible. `revoke_delegate` is owner-only and
revokes that identity's administrative powers for the named revision, not other
revisions or its independent active-profile grants. Pause/retire such bindings
separately when intended. Owner identity is stable; rotation is unsupported.
Amendment is not a policy hot-swap or automatic supersession. New work under a new revision needs explicit admission,
with prior exposure retained; this first adapter does not implement live policy
migration or cross-spec mail delivery.

`pause` blocks subsequent managed participant operations without first resolving
output receipts. It does not stop a native process or recursively pause children.
`resume` records the owner's intervention disposition. `retire` settles an
individual binding, requiring all its output receipts resolved. Its branch keeps
its last owner and generation so an explicit replacement can resume it, with
continuation, verification, fresh generation and settlement evidence; pending starts
are cancelled rather than silently activated. Native quiescence is attested,
not observed by the reducer.

`close_phase` requires no live/prepared/bound/verified participants or unresolved
receipts in that spec and phase, and records the owner's end-condition evidence.
This is an explicit **local membership** check, not proof no external worker
exists. Pending replacement records must be retired/cancelled too. A scoring
phase can remain open after execution closes. Queued messages remain retained
history; closure does not publish or erase them. No timeout, board lease expiry,
or global skill release closes an experiment automatically.

## Validation

```sh
cargo test --locked --offline --manifest-path rust/Cargo.toml -p experiment-control
EXPERIMENT_CONTROL_TEST_BINARY=/absolute/target/debug/experiment-control \
  python3 -m unittest discover -s skills/durable-state/tests -p test_experiment_control.py
```

Tests use synthetic identities, inline fixtures and temporary repositories. They
exercise actual artifact output, refusal without publication, immutable versions,
pending receipt recovery, output/transition serialization, competing handoffs,
shared worktree state, phase separation and retained Git history. They do not
establish native host isolation, real agent exposure or experimental validity.
