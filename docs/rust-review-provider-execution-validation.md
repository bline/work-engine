# RP0 provider-disabled boundary validation

This receipt covers the Rust RP0 boundary only. The human-authorized subject is
the provider plan at SHA-256
`9404fd77e88e2e1ed811852be623143d9dd4e5cc0301cf94aa941bf29b8eb1e4`
and the authorization record is
`/home/bline/.local/state/work-engine/rp0-sc3d-implementation-20261013/authorization.json`.
The three nonblocking plan clarifications were added after acceptance and have
their own hash in the external `environment.json`. RP0 makes no provider call,
reads no real credential, creates no qualified E1 evidence, and grants no SC3
or RP1–RP3 admission.

## Realized boundary

`CandidateBoundary::prepare` hashes an exact candidate tree, builds a new private
CBM project in a network-denied Linux namespace, disables watcher/automatic
indexing and UI, waits for the build process to quiesce, rejects SQLite
WAL/SHM/journal sidecars, checks integrity, and copies a self-contained store.
It reads project/root/generation and every fixture path's coverage through the
mounted review copy. The candidate, store, configuration and coverage manifest
are pinned by SHA-256; their drift refuses the probe. The coverage signal is
best effort and cannot establish behavioral accuracy or a complete graph by
itself. A later reviewer must use source fallback for flagged or disputed paths.

The reviewer process has a read-only `/repo/candidate`, read-only MCP
configuration and client, private writable session/scratch, and only a Unix
socket to the candidate's CBM service. It receives neither `/cbm/bin`, `/cache`
nor `/runtime`. A narrow trusted relay accepts MCP initialization and exactly
seven read query tools, checks explicit project arguments against the frozen
candidate project, and returns a filtered `tools/list`. The relay starts the
real CBM MCP process and its detached daemon in a separate first-level
Bubblewrap namespace with `--unshare-net`; the pinned CBM launcher adds
`no_new_privs`, a seccomp filter that allows only AF_UNIX sockets and denies
`io_uring_setup`, and closes inherited descriptors. The host observes the
actual two CBM processes, mount access, network namespace, socket state and
post-run store. The private daemon is task-owned and exits with its namespace.

This relay is a material deviation from the accepted plan's “no new broker”
expectation. The first otherwise functional realization mounted a writable
private CBM cache into the reviewer namespace. A hostile process bypassed the
MCP tool list by directly executing `/cbm/bin cli index_repository` and created
`rp0_escape.db`. A read-only cache trial failed at CBM startup: its private
directory validation calls `fchmod(0700)`, and `_config.db` is opened for
read-write schema initialization. Splitting the reviewer and CBM namespaces is
the smallest implemented remedy; it needs explicit fresh review before a
later slice treats this profile as accepted production infrastructure.

## Evidence and gates

Raw logs and captured fixtures are under
`/home/bline/code/.work-engine-tmp/rp0-20261013/final/`. The fixture source
tree SHA-256 is
`53d42ed789ab4fc9cf13dabd4a5cda0689d1e84da4dd36ac40d9031df366e2b2`.
The new `rp0_fixture` project was indexed at `2026-10-13T07:57:23Z`, with 5
nodes and 4 edges. Its complete, quiescent store SHA-256 is
`224894381bc05f6fee017854b0cbc9a9963bddcbbc7d969f07b80e31c45afe89`;
the coverage manifest SHA-256 is
`4bcc8dba5e9991ab541538500b24dae14a3eb5e2d19b212631bc9fbc83dfe82f`.
`fixture/index-binding.json`, `coverage.json`, the SQLite store and
`environment.json` preserve those bindings and the pinned program/toolchain
versions. The CBM build is
`0.11.0-945-g3d228b96-integ`, SHA-256
`e1adfa5c41f2e64c9d05620f660ebf9975f91deabc6fd72d4289da3e051a4872`.

The serial command recorded in `cargo-test.log` was:

```text
RP0_EVIDENCE_DIR=/home/bline/code/.work-engine-tmp/rp0-20261013/final/fixture cargo test --manifest-path rust/Cargo.toml -p review-execution-evidence --locked --target-dir /home/bline/code/.work-engine-tmp/rp0-20261013/target -- --test-threads=1 --nocapture
```

It exited 0: 20 tests passed, 0 failed, 0 ignored, including the real hostile
process, the real private CBM MCP/daemon, drift and synthetic credential
canaries. `cargo fmt --manifest-path rust/Cargo.toml --package
review-execution-evidence -- --check` and `cargo clippy --manifest-path
rust/Cargo.toml -p review-execution-evidence --all-targets --locked --target-dir
/home/bline/code/.work-engine-tmp/rp0-20261013/target -- -D warnings` also exited
0; their exact logs are `cargo-fmt.log` and `cargo-clippy.log`.

`fixture/peer-report.json` records refusal of candidate and MCP-config writes,
direct CLI execution, mutation and hostile cross-project/Cypher requests. It
observes read-only candidate/socket/client/MCP mounts, writable scratch, no
host-source or credential path, no inherited host file descriptor, and exactly
the seven selected CBM tools. `fixture/cbm-observation.json` records both CBM
processes with `NoNewPrivs=1`, `Seccomp=2`, a distinct network namespace with
only loopback and zero TCP listeners immediately after `index_status` and after
three seconds. The installed CBM source polls UI configuration every one second
(`src/daemon/host.c`, `HOST_HTTP_CONFIG_POLL_MS=1000`), so the second observation
crosses that interval. `cleanup.json` confirms the observed task-owned PIDs
exited. The shared account CBM daemon was not stopped or mutated.

The positive observation is limited to this one-file synthetic candidate and
one provider-disabled MCP connection. It does not establish the eventual
production candidate's coverage, a live Claude tool/configuration fit, a real
credential horizon, model identity, inference routing, result custody, or a
retained same-session review. Those require the separately owned RP1, RP2 and
RP3 gates and SC3-H. No deployment or production claim was made.

## Retained RP0 remediation after fresh review

The original archive above remains immutable at SHA-256
`4e7a1b1d3e4ad05fe4f9b02cd15a91f553b3bb9306f31631a52ec502aac6b67e`.
The fresh reviewer report at
`/home/bline/code/.work-engine-tmp/rp0-astra-review-20261013/review-report.json`
(SHA-256 `da7892901a267ce4072d7a1e0d8c670110c3fddae0910c7d98b18480c0943d64`)
blocked RP0 on R1–R3. The remediation source overlay is
`/home/bline/code/.work-engine-tmp/rp0-remediation-20261013/final/remediation-source.tar.gz`
(SHA-256 `ea0622b4314f428af44dc37e7c42edc308d787353ef4f9b9be6abb1fcfb9c3ac`).
It contains the two changed Claude modules, the boundary test, the amended
hostile CBM probe, and the new adverse-frame probe. The original archive plus
this overlay identify the new review subject; neither archive is an activation.

R1: the live CBM launch now mounts the frozen project DB, `config.json`, and
`_config.db` read-only within the CBM namespace. The cache directory remains
writable only for required CBM housekeeping. In the real installed CBM MCP and
daemon, `/proc/<pid>/mountinfo` showed each protected file read-only and an
actual write-open through each process root failed with `EROFS` (errno 30).
Both observations held immediately after `index_status` and after the UI
configuration reload interval. The reviewer still had no `/cache` mount or CBM
binary. The frozen-store readback passed after each adversarial run; the
observed task-owned MCP/daemon PIDs exited. This is a measured installed-CBM
result, not an inference from Bubblewrap flags.

R2: credential access now walks every absolute ancestor with descriptor-relative
`openat` and `O_NOFOLLOW`, checks owner and unsafe modes, and pins the parent
directory device/inode at construction. Access-time rewalk refuses a symlink
ancestor or different real parent. The final regular source is opened relative
to that checked parent with `O_NOFOLLOW` and must be owned by the current user,
mode 0600 or stricter, and at most 64 KiB. A same-directory replacement of the
final credential file is deliberately allowed for owner-managed rotation;
synthetic tests prove that path and reject both symlink and real-directory
ancestor replacement. No real credential was read.

R3: the hostile probe used the admitted `rp0_fixture` project for a mutation
shaped `query_graph` request, out-of-candidate coverage paths, and an external
qualified source request. The relay forwarded request IDs 8, 9, and 10 to the
real CBM backend. CBM returned “write operations not supported,”
`outside_project` for both paths, and “symbol not found,” respectively; no host
source text appeared. Cross-project and indexing calls remained relay refusals.
The test therefore separates relay scope enforcement from backend query/path
behavior. The earlier original receipt's cross-project query alone did not
prove this backend behavior.

The relay now has a visible 4096-request budget, no silent 64-response cap, a
one-response output queue, bounded nonblocking writes to CBM stdin, socket
write timeouts, and a deadline for joining the output reader after child kill.
The subsequent `child.wait` is synchronous and has no wall-clock deadline;
the exercised normal-kernel runs establish their observed cleanup, not a hard
reap bound under every failure. A real peer completed
91 requests in the normal boundary test. Separate oversize, flooding, and early
disconnect peers reached their attack branches: oversize returned `OutputLimit`,
flooding hit a bounded socket-write `WouldBlock`, and early disconnect returned
`IsolationFailed`; every run verified the frozen store and removed its socket.
The 4096-request threshold itself was not exercised, and this does not prove
fairness or availability under arbitrary concurrent peers. RP0 has one bounded
probe connection; production transport remains RP1 work.

The remediation serial crate gate in `final/cargo-test.log` exited 0: 22 tests
passed, 0 failed, 0 ignored (including six boundary tests); compile-fail doc
tests passed as designed. `final/cargo-fmt.log` and `final/cargo-clippy.log`
record zero-exit format and all-target Clippy checks. Raw CBM responses,
protected-mount observations, index binding, adverse-peer receipts and copied
fixture store/configuration are in `final/fixture/`. The new private index was
built from scratch with pinned CBM `0.11.0-945-g3d228b96-integ`; the shared
account CBM daemon was not stopped. These results are a remediation submission
for the retained reviewer, not a reviewer acceptance or RP1/production claim.

## Retained RP0-R4 credential follow-up

The retained reviewer verified R1–R3 resolved and identified R4 in
`/home/bline/code/.work-engine-tmp/rp0-astra-review-20261013/remediation-1/review-report.json`:
opening an as-yet-unchecked final credential path with blocking `O_RDONLY`
could hang forever on a FIFO without a writer. The two-file source/test overlay
`/home/bline/code/.work-engine-tmp/rp0-r4-20261013/final/r4-source-overlay.tar.gz`
has SHA-256 `16618a255bc533bc200b0a39f27e1c569a4dd1fd698b5da31606cc87596bf500`.
It layers on the original archive and the preceding five-file remediation
overlay; neither prior archive changed.

The final descriptor-relative, no-follow `openat` now adds `O_NONBLOCK` before
immediate `fstat`-based regular-file and owner/mode/size validation. Regular
file reads retain their normal behavior. The pinned parent directory and
same-parent final regular-file rotation policy remain in force. Two new
synthetic tests cover a FIFO at construction and a valid source replaced by a
FIFO before access, both with no writer. Against the pre-fix binary each test
timed out at three seconds under an external process deadline; the harness
killed that test process and confirmed no residual test process. With the fix,
the four focused credential tests passed in 0.01 seconds. Exact pre-fix and
post-fix logs are under `/home/bline/code/.work-engine-tmp/rp0-r4-20261013/raw/`.
Package format check and all-target Clippy with denied warnings also exited 0.
The CBM fixture and relay were unchanged, so the costly live CBM test was not
rerun. This follow-up is submitted to the same retained reviewer; no provider
call, real credential read, or RP1/production claim follows.
