# Native Pi memory prerequisites

Status: bounded primitives and read-only diagnostic; native Pi runtime admission
and retention are not implemented by this module.

`pi_docker::native_memory::CandidateArena` owns a bounded private anonymous mmap.
It accepts input only after mlock, MADV_DONTDUMP and MADV_DONTFORK succeed; capacity
failure preserves existing contents. Clearing and dropping overwrite the mapping
before unmapping. It does not protect copies into ordinary heap, stack, parser,
SDK, subprocess or daemon storage. It supplies no publication capability.

`receiver/pi-memory-guard.rs` is minimal Rust Node N-API interoperability. Build
it as a cdylib with rustc and load the resulting `.node` before SDK import or
candidate acquisition. It sets PR_SET_DUMPABLE=0 in the final Node process and
exports a current-state check. An exec resets this protection; a launcher's
setting is insufficient. The module does not prevent later prctl changes or
qualify descendants. Native runtime acceptance must account for those paths.

`cargo run -p work-engine-manual-host --example pi_native_profile` observes the
controller's cgroup, local Docker socket peer, and fixed deployed Docker service
process without issuing Docker requests or creating containers. Socket activation
can make the socket peer PID1 rather than dockerd. Process backing observations
are data, not admission capabilities. Unknown identity/backing remains refusal.
The diagnostic always leaves runtime_qualified false; even swap-disabled relays
would still need dump-policy and descendant evidence.

A private transient user service with MemorySwapMax=0 can establish whole-process
swap backing without changing a shared group's policy. Actual current swap must
also be zero. These controls do not protect Docker attach relays outside the
service. Existing pi_docker command/output and export paths are unchanged and
must not be treated as this native retention boundary.
