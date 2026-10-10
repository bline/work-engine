# Rust context-lifecycle experiment

This standalone experiment tests a narrow hypothesis: Rust ownership
and distinct identity types, combined with a small durable coordinator, can make
context-ownership and completion mistakes harder. It is not a production
implementation, migration, engine replacement, or performance benchmark. It does
not import or adapt the existing lifecycle implementation.

The proposed full replacement architecture is in [DESIGN.md](DESIGN.md), with
[requirements, legacy lessons and acceptance tests](DESIGN_EVIDENCE.md) and a
[source manifest](design-sources.json). That later assessment explicitly studies
the current schema-14 effort; it does not change this experiment's original scope
or make its bounded results production approval.

The [multi-slice implementation plan](IMPLEMENTATION_PLAN.md) proposes how to
build that replacement, including parallel ownership and UI integration points.
It follows the repository's [Rust development direction](../../docs/rust-development.md).
The plan is not implementation or live-provider authorization; this experiment
and its historical evidence remain separate and unchanged by that proposal.

The fake provider is synthetic. A separate, version-pinned Codex app-server
adapter can also record observation-only turns or settle one explicitly admitted
bounded text-turn obligation using exact full-turn evidence. Native terminal
status alone never settles a delivery. Its controlled installed-CLI vertical
uses only simulated loopback inference, without auth; it is not a live-provider
or general tool-effect seal. See [the native result and limits](NATIVE_VERTICAL.md)
and [the historical observation contract](CODEX_CONTRACT.md).

## Run

Validated with `rustc 1.92.0` and `cargo 1.92.0`, on Linux. `Cargo.lock` pins
dependencies; SQLite is bundled through `rusqlite`. This directory has its own
Cargo workspace. Commands below run from this directory:

```sh
cargo build --locked
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

The executable demonstration is the focused process integration test:

```sh
cargo test --locked --test lifecycle integrated_replacement_restart_and_fifo -- --exact --nocapture
```

It prints five `TRACE` lines with actual runtime/provider/coordinator PIDs,
observed child exit status, and independent provider submission counts. The test
owns, bounds, and reaps its child processes, uses ephemeral loopback ports, and
keeps temporary databases under the locally ignored `target/` directory. Timing
assertions use acknowledged barriers and bounded observation waits, not sleeps.

The binary also exposes the two small process modes used by the tests:

```text
context-lifecycle-experiment provider DATABASE
context-lifecycle-experiment coordinator DATABASE PROVIDER_ADDRESS [BARRIER]
context-lifecycle-experiment codex-provider LAUNCH_JSON
```

Each prints one JSON readiness message containing its actual PID and address.
`BARRIER` is an optional, one-shot executor pause: `before-entry`, `after-entry`,
`after-send`, or `after-result`. It pauses the real executor while observation and
command handling remain available. `after-result` withholds admission replies
until released, permitting a real crash after durable result publication and
before the caller receives its ticket. Barrier release is experiment harness
control; it grants no delivery authority.

## Small architecture

The trusted synthetic controller uses `Client`; model-context state lives in
`SyntheticRuntime`; read-only consumers receive `Observer`. The runtime carries
two model contexts in one process while the coordinator is killed and restarted.
The runtime context objects have no provider or shell/tool capability. The fake
provider is a separate process with a separate SQLite ledger.

`Coordinator` owns both admissions and executor responsibility. An actor services
commands, observation waits, and autonomous executor ticks. Closing the caller's
socket, dropping an observer, or expiring its wait does not remove an accepted
operation. Observation has no advance/retire/retry method. To keep the experiment
small, IPC is blocking with bounded timeouts and one thread per client connection;
this is not an async executor or a scalable server.

The server's observation budget begins when the actor accepts a waiter. After an
executor step, one captured monotonic time classifies pending replies: expiry
takes precedence even if the condition or admission ticket is then ready. A
timeout changes neither the durable result nor the idempotent command receipt;
both remain observable independently. Blocking provider IPC can delay this
classification. Socket operations and response channels have separate transport
timeouts, so a caller may instead see a transport error. These bounds are not a
hard end-to-end timing guarantee for a real provider and say nothing about
shutdown completion.

One SQLite row holds a serialized aggregate: contexts, immutable command identities
and replies (including rejections), queue bytes/digests/lineage, transitions,
deliveries, effects/evidence, and local-exit history. Every mutation is transactional
with WAL and `synchronous=FULL`. Rejected commands preserve their identity without
changing the semantic revision used for readiness. A single-host exclusive file
lock lasts for the coordinator's entire database lifetime. Canonical paths resolve
ordinary symlink aliases. No failover lease or distributed leader claim is made.

An admitted delivery first becomes `Prepared`. The executor commits `Entered`
before sending IPC. After an actual restart, entered work becomes `Unknown` and
is never automatically resent. Prepared work may enter if its original authority
has not expired. Thus a crash after entry but before send deliberately sacrifices
liveness: even a provider ledger with no record does not prove the request cannot
still arrive. No automatic retry or input retirement exists.

An exact provider seal binds effect identity, request digest, and outcome. The
fake provider consumes each delivery authorization only once, including attempts
to use another effect ID for the same delivery. Repeated identical records are
idempotent; provisional evidence cannot open the next input. The coordinator
atomically projects authoritative evidence into effect and delivery outcomes.
The executor has no remaining send work for a sealed delivery. A later deliberate
provider contradiction is preserved and fences the context; it cannot undo effects
already performed. `Contradict` is an explicit broken-provider test control, not a
valid normal settlement operation.

Replacement consumes a private, non-`Clone` `Owner`, revokes ordinary old-epoch
admission, and grants a transition/epoch/deadline-bound `CapturePermit`. Capture
can supply objective, authority, commitments, and references; it cannot deliver an
input or grant tools. Current unresolved effects and the exact queue are loaded
from current durable state for bootstrap, including after restart. The synthetic
runtime stores the exact package and returns an explicit identity/revision/digest
attestation. Activation commits successor authority only if the package is still
current. This checks byte custody, not model understanding or reference truth.
Capture failure and unknown capture are terminal for that transition; late text
cannot resurrect them. Waiting is valid and no replacement trigger is installed.

## What the tests demonstrate

The default suite has 7 unit tests, 17 process integration tests, and 5
compile-fail doctests; installed-CLI tests are explicitly opt-in.
All integration lifecycle changes use public command,
capture/readiness, provider, and observation paths; tests do not edit kernel
database rows.

The Codex unit test derives two small JSON Schemas from the selected Rust wire
DTOs, validates success and malformed fixtures offline, and separately checks
that a schema-valid `inProgress` turn is not terminal evidence. Schemars and the
JSON Schema validator are development dependencies only; no runtime validator
or complete Codex wire schema is claimed.

| Evidence | Demonstrated consequence |
| --- | --- |
| Integrated replacement/restart trace | Two queued inputs; held first effect; two contexts in one surviving runtime; successor carries unresolved work; stale commands and overtaking rejected; actual coordinator death/restart; no resend; exact reconciliation; separately authorized second delivery |
| Four executor crash barriers | Before entry: safe first dispatch after restart. After entry without send: Unknown and zero dispatches. After send: Unknown and one dispatch. After durable result before ticket: same durable success with one dispatch. |
| Immediate failure with disconnected caller | Correlation and responsibility exist before provider failure; effect, delivery, and evidence survive without any ticket observer. |
| Identity, readiness, idempotency cases | Stale epoch, wrong capture permit, wrong readiness identity/revision/digest, changed current state, input overtaking, and conflicting command keys are rejected. |
| Provisional/duplicate/conflicting evidence | Provisional evidence blocks the queue; identical evidence does not multiply results; authoritative contradiction survives and fences subsequent work. |
| Real normal worker exit plus later settlement | Actual child PID/status is observed separately from remote settlement; unsafe-at-time history remains after settlement and coordinator restart. |
| Manual-clock boundary tests | New entry and capture are rejected at the exact deadline; authoritative evidence arriving after entry permission expires is still accounted for without extending permission. |
| Exclusive process lock and observer timeout | A second coordinator cannot acquire the store; a bounded wait expiring does not abandon prepared work. |
| Pending reply deadline regression | Ready condition/ticket replies at or after expiry time out; readiness before expiry succeeds; a timed-out public wait leaves the completed effect and replayable ticket unchanged. |
| Compile-fail doctests | Reusing a moved owner, fabricating its private fields, swapping identity types, advancing via an observer, and using capture permission with the ordinary admission API do not compile. |

Replacement completion, capture readiness, delivery outcome, and local process
exit remain different facts. Successful replacement needs no worker shutdown.
`ObserveLocalExit` records an actual observer-supplied PID/status and whether
unresolved work existed at that observation. A normal exit is not evidence of
external settlement. That historical unsafe-close fact is never recomputed away
when later settlement leaves zero unresolved effects. No shutdown initiation or
cleanup-budget protocol is implemented; an expired observation wait cannot change
an already observed exit or a durable result.

## Limits and interpretation

`Unknown` means the system lacks final evidence. It does not mean success,
failure, absence, or permission to retry. Held work remains `Entered` while this
coordinator knows its live dispatch; a restart removes that local certainty.
Missing provider evidence never advances the queue. A fake sealed failure is a
terminal failure, distinct from pending and unknown.

Rust's affine owner prevents accidental reuse within a local ownership path.
It does not establish cross-process authority: trusted controller commands and
deserialized stamps still need durable epoch and readiness-revision checks.
Idempotent controller recovery can reconstruct a local owner wrapper, so the
wrapper is not a cryptographic or globally unique capability. Capture/authority
and readiness attestation are explicitly synthetic; network clients are trusted,
loopback IPC is unauthenticated, and the provider ledger belongs to the same
experimental trust domain. The separate process establishes bounded effect
history, not independent organizational attestation.

Epoch checks fence new stale admissions. They do not revoke already authorized
bytes in flight. This prototype cannot fence arbitrary external providers,
network-wide effects, shell commands, or tools run inside a real CLI agent. It
makes no arbitrary exactly-once claim. File replacement, hard-link aliases,
malicious same-user processes, unreliable/network filesystems, power-loss testing,
database corruption recovery, machine/clock rollback, and multi-host coordination
are outside the bounded model. Wall-clock operational deadlines assume the local
clock does not move backward.

There is no retention/compaction, scalable ledger, migrations, provider SDK,
authentication, generic workflow framework, model-understanding validator,
automatic unknown-effect disposition, retry policy, or efficiency/trigger policy.
The entire aggregate and command history are rewritten on mutations, and the
executor polls prior effects for contradictory evidence: appropriate to this
small experiment, not a measured production design. No speed, token, cost, or
operator-effort advantage has been established. Current evidence is test results,
process identity/exit observations, and provider-ledger counts.
