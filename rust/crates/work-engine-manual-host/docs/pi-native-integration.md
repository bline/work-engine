# Offline native Pi integration

Status: unfinished offline development implementation; paused by the operator on
2026-11-14 after commit and merge. The operator permits unchecked temporary
candidates inside the isolated Docker scratch workspace. Original credentials
remain host-held; Work Engine exports and custody publications remain checked.
This prospective boundary change still needs qualification reconciliation. See
[the dated pause record](../../../../records/pi-native-development-pause-20261114/README.md)
for authority, bounded results, historical findings and remaining gates.

`pi_native` is a bounded offline transport implementation exercised by
`examples/pi_native_probe.rs`. It is separate from the earlier memory/FIFO
mechanism experiment. The human's trusted laptop OS, encrypted storage and
excluded host compromise are environment assumptions for this leaf; global
kernel capture/backing attestation is not a result of this implementation.

The actual Pi 1.5.0 SDK creates an `AgentSession`, `ModelRuntime`, in-memory
settings and in-memory native `SessionManager`. Native provider parsing and
session evolution run in the pinned network-none, read-only Docker image. The
receiver's only credential-shaped value is a nonauthenticating parser marker.
The host reconstructs a selected local synthetic HTTP request; workload URLs,
accounts and headers cannot choose host dispatch authority. Docker CLI's
non-TTY `start --attach --interactive` demultiplexes the Docker attach protocol;
the Rust adapter consumes the resulting length-prefixed stdout frames. There
is no TTY or second workload network path.

The distinct `review_execution_evidence::pi` branch retains original synthetic
A in a live owner. It checks native keys/values, parsed JSON escapes, nested
JSON/JSONL strings, whole-value base64, exact output envelopes and fixed names
before descriptor-relative create-only publication. Writer exclusion,
predecessor joins, native prefix evolution, quotas and real wall-clock expiry
are store invariants. A deserialized binding, hash or caller boolean is not a
publication permit. The custody parent issues an immutable inherited descriptor containing the exact
snapshot publication, store/root identity, admission/native context, and final
checked-registry image and metadata digests. The sealed reader has no live
acquisition, dispatch, mutation or descriptor-reissuance method. An editable
adjacent digest file is not an accepted expectation.

`Pending` owns retention sequencing only. The probe obtains allocation,
initialization and provider `PiOwner` / `EffectLease` authority from an explicitly
signed synthetic fixture. A is held before the new authoritative registry is
initialized or admission is persisted. The configured registry path is a
directory, so generic SQLite consumers refuse it. The opaque checked backend
serializes its fresh in-memory SQLite database, checks every text cell and the
exact final image, generated names and metadata, reserves shared capacity, then
retains create-only images. One checkout remains held through each lease; unknown
effects cannot authorize replay. No existing live registry is copied or migrated.
The transport is not integrated into the existing main Pi Docker execution path,
and this synthetic authority does not constitute production permission or C4. A bounded native tools/workspace/build/test/checked-export run passed on the
source12 subject. The later FT-02 repair has focused qualification only; the
complete crash, duplicate, cancellation, observer-loss and artifact matrix, broad
regression gate and whole-vertical review remain unfinished.

The native receiver records selected Node filesystem write API attempts and
network connects, including a final observation after native disposal, with observations checked
before publication. Scratch
contents and Docker diff are inspected before/after the actual run. Docker diff
includes mount-point placeholders, whose sorted entries are compared. These
observations are bounded behavior evidence, not an independent exhaustive
syscall audit or a universal absence-of-write claim. Failed create/attach and
Drop cleanup retain weaker historical disposition than a successful joined
closure; absence afterward alone cannot repair missing history.

The probe uses independent random synthetic access material, a separately
controlled host endpoint, two actual parsed assistant responses and two native
snapshots. Seven injected variants of the real native snapshot are rejected
before snapshot creation. A fresh-process `pi_native_read` exercises exact
sealed read/reconcile without an access source or transport capability. This
paragraph describes the earlier offline no-tools probe. The later tools probe
has bounded synthetic evidence in the pause record; neither establishes a
remote-model or accepted full-vertical result.
