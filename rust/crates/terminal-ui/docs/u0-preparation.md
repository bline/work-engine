# Lifecycle observation: U0 result and U1 connection

Status: S3 U0 contract integrated on 2026-10-07; S4 U1 explicit read-only
socket connection integrated on 2026-10-08. The UI consumes published contract
examples and the passive client API. [The README](../README.md#run) documents
the selected endpoint/subject and interactive controls.

## Exact consumed boundary

S3 publication `634161c2f54dc4fcf9dcb53936951ecefe457235` supplies
`lifecycle-wire` v0.1.0, `lifecycle-client` v0.1.0, Draft 2020-12 JSON schemas,
and versioned fixtures. The exact manifest SHA-256 is
`f4c73a45602284989fc352f5529f421a5e2c73cdaece2c4678035825032727be`.
The UI's standalone manifest and lockfile use path dependencies on the public
crates. `scripts/check_s3_pin.py` verifies the historical S3 source hashes at
that commit, current stable v1 schema/fixture bytes, and the current public S4
source receipt in [s4-client-pin.json](s4-client-pin.json), bound to published
commit `41992a22d86b041d2db357d16345c477804af63a`. It writes no owner artifact. A changed hash
requires owner reconciliation before relying on these displayed examples.

`src/contract.rs` uses `lifecycle_wire::parse_snapshot` for the snapshot cases
and the owner's strict JSON parser plus published wait-result DTO for the
unavailable case. Unsupported observation variants and versions fail closed.
The fixture modes are `contract-failed`, `contract-historical`,
`contract-unavailable`, and `contract-unsupported`, also reachable through
Ctrl-P. Every mode says **S3 CONTRACT EXAMPLE**, separate from the UI's invented
work/chat examples and retained historical replay. F3 expands exact details;
the same presentation feeds source copying and palette event inspection.

## Supported interpretation

| Public S3 fact | UI interpretation |
| --- | --- |
| Subject, context, cursor | Displays exact subject ID, context generation, store ID, stream ID and commit sequence. These are separate scopes; a changed source invalidates inspection anchors. |
| Delivery | Displays delivery, effect and attempt IDs, execution outcome, settlement and provenance separately. `failed` with `unresolved` remains unsettled; `conflict` remains conflict. |
| Transition and admission | Displays historical/target transition stage and completed flag separately from current admission and blocking reason. A completed prior transition cannot imply current admission. |
| Runtime availability/provenance | Displays `available`, `unavailable`, or `unknown` and the provenance field state. Known provenance retains source reference and Unix-millisecond observation time. |
| FieldV1 and references | `unknown`, `not applicable`, and `unavailable` stay distinct. Authority/evidence references are displayed as supplied, without conferring authority. |
| Passive client health | Controlled transport tests use `ReadClient::snapshot`, `wait`, and `reconnect`. Targeted reread on explicit cursor gap, deadline with last-known snapshot, unavailable observation, wrong store/stream and sequence rollback project without fabricated progress. The client owns wait/reconnect semantics. |
| Input boundary | Contract/client inspection retains recipient drafts and disables simulated send and retargeting. The normal live work fixture still permits its clearly labeled local simulation. |

U0's `measurement: FieldV1<String>` is retained as a raw value or explicit field
state. It has no measurement kind, unit, capacity, context binding, or validity
period. The UI therefore labels context usage unavailable and computes no
percentage. S3 supplies no agent transcript, source text stream, branch history,
or command authority; none is invented here. Snapshot cursor is a committed
observation position, not a timestamp or proof of complete history.

## Validation and limits

The UI has 47 Rust tests, including published fixture interpretation and a
controlled in-memory `ObservationTransport` test of actual `ReadClient`
snapshot/wait/reconnect results. U1 adds tests of passive request framing,
bounded stalled reads, generation fencing, cursor identity/rollback rejection,
and source-anchor retention on unchanged reads. The in-memory transport is a
test adapter; U1 separately joins the actual S4 Unix socket service through a
PTY smoke. The UI's path-dependency lockfile is local to its excluded
workspace. Run from the repository root with Rust 1.92.0:

```sh
python3 rust/crates/terminal-ui/scripts/check_s3_pin.py
cargo +1.92.0 fmt --manifest-path rust/crates/terminal-ui/Cargo.toml --check
cargo +1.92.0 test --locked --manifest-path rust/crates/terminal-ui/Cargo.toml
cargo +1.92.0 clippy --locked --manifest-path rust/crates/terminal-ui/Cargo.toml --all-targets -- -D warnings
cargo +1.92.0 build --locked --manifest-path rust/crates/terminal-ui/Cargo.toml
python3 rust/crates/terminal-ui/scripts/pty_smoke.py \
  rust/crates/terminal-ui/target/debug/work-engine-terminal-ui
cargo +1.92.0 build --locked --manifest-path rust/Cargo.toml -p lifecycle-service
python3 rust/crates/terminal-ui/scripts/connection_smoke.py \
  rust/crates/terminal-ui/target/debug/work-engine-terminal-ui \
  rust/target/debug/lifecycle-service
```

The UI is intentionally outside the parent Cargo workspace. Its dependency
tree should contain public wire/client/types support only, without lifecycle
store, core, runtime, service, or provider packages. U0 tests establish mapping
fidelity and client behavior at the public port. U1 joins the separately built
service only in a disposable test or an operator-selected session.

## U1 result and remaining boundaries

U1 connects with explicit `--connect` and `--subject` arguments. One background
worker owns bounded `ReadClient<UnixSocketTransport>` snapshot and reconnect
requests; the UI thread never performs socket I/O. Ctrl-P offers refresh,
reconnect, and explicit disconnect. Last-known observations stay bound to the
selected endpoint, subject, store, stream, and cursor. Changed identity,
rollback, stale completion after navigation, and stale completion from an old
subject cannot replace the current view. Refresh leaves unchanged inspection
anchors intact; changed source data clears them. Input remains disabled while
inspecting the connection even before a result arrives.

The service PTY smoke starts only a disposable, empty `read_only` profile for
one configured subject in an owned 0700 directory. It verifies the initial
snapshot, refresh, outage with last-known labeling, same-store service restart
and reconnect, explicit disconnect, and clean process exit. It does not prove
controlled-turn history, cursor-gap behavior in a running service, or a
provider path. U0's controlled client tests cover cursor-gap mapping; S4 owns
the source proof and targeted durable-read semantics. Service authorization and
redaction remain the service's boundary, with no UI-side grant or command path.

Agent output, qualified context usage, branch/history completeness and
cross-domain work state require future owning public projections; the
lifecycle snapshot does not claim to provide them.

## S5 populated observation validation

The S5 join uses the actual packaged service and pinned installed native CLI
against a local simulated inference server, with external networking disabled
by Bubblewrap. `scripts/s5_populated_smoke.py` derives its test route from S5's
retained `proof_harness/generation_switch_proof.py`; it does not modify that
historical evidence. The driver supplies synthetic immutable snapshot bytes,
temporary grants and one replacement request. The UI only issues passive reads.
Two disposable scenarios exercise actual rendered terminal frames:

- **Generation handoff:** predecessor domain turn and child snapshot read,
  `Switching` with current admission blocked, outage with last-known labeling,
  successor rehydration and domain turn, then context 2 with `Reconciled` and
  completed true. Four simulated requests, no authorization headers, distinct
  native threads, and two clean service exits. A stale predecessor-context
  replacement is rejected without changing admission, context or transition;
  the journal cursor may advance when recording the rejection.
- **Missing verifier source:** predecessor and child complete, but the
  verifier's exact retained source is absent. The actual service exposes
  `RecoveryRequired`, completed false, and **fenced / durable_recovery_required**
  in the UI. Two simulated requests and one clean service exit. This is a
  specific verifier failure cut, not proof of every native failure mode.

The join revealed an important public-interface limit. S5's native parent and
child results are durable backend facts, but this route's v1 snapshot returns
`delivery: not_applicable`; it supplies no native operation collection or
native failure cause. Driver-only read-only SQL checks establish the native
`completed_settled` states and verifier rejection. Those checks are **not UI
observations** and are never fed to the UI. The UI can currently display the
handoff's current state and recovery fence, but cannot display the native
operation outcomes, explain the verifier failure, or reconstruct a populated
historical event timeline from this contract. Those capabilities require an
owning public projection; no backend or schema was changed here.

Live observation rows now identify themselves as lifecycle observations and
the banner says **current snapshot**. Details state the history, operation and
transcript limits. Neither a cursor nor a sequence of manual refreshes is
presented as a complete event history. Usage remains unavailable.

Reproduce from the repository root on Linux, with the S5 pinned native binary,
Bubblewrap, Python, and explicit built binary paths. Use an external writable
scratch directory with adequate space for disposable native state:

```sh
export CARGO_TARGET_DIR=/absolute/path/to/cargo-target
export TMPDIR=/absolute/path/to/disposable-scratch
cargo +1.92.0 build --locked --manifest-path rust/Cargo.toml \
  -p lifecycle-service --features native-sim-proof,controlled-proof \
  -p lifecycle-test-support --bin lifecycle-service --bin controlled_peer
cargo +1.92.0 build --locked --manifest-path rust/crates/terminal-ui/Cargo.toml
python3 rust/crates/terminal-ui/scripts/s5_populated_smoke.py \
  "$CARGO_TARGET_DIR/debug/work-engine-terminal-ui" \
  "$CARGO_TARGET_DIR/debug/lifecycle-service" \
  "$CARGO_TARGET_DIR/debug/controlled_peer" \
  --evidence /absolute/path/to/new-evidence-directory
```

The harness enforces the S5 native executable SHA-256, records all executable
hashes, and retains selected wire snapshots, raw ANSI frames, plain extracted
text and receipts when `--evidence` is given. The evidence destination must be
new. It bounds the joined run to 100 seconds, isolates the process namespace,
reaps owned processes, checks UI terminal restoration, and removes temporary
service/native state. It uses read-only executable mounts rather than copying
large debug binaries. This validation enables neither production activation,
live inference nor UI commands.
