# Local historical S5 read

`historical-evidence-read` is a bounded local inspection host for
`skills-migration-app-server-20260828`, slice 6. It calls the legacy
Slice Checkpoint, slice-builder gate-capture and Independent Review State
readers. It returns a versioned, allowlisted summary through a Unix socket.
It does not create workflow state, run gates, resume reviewers or establish
test-execution attestation. The current Rust campaign/review stores are not
substitutes for these historical Git records.

The trusted local launcher supplies `--inspection-grant ABSOLUTE_PATH --socket
ABSOLUTE_PATH`. The grant is a current read admission, separate from the three
historical writer manifests. The launcher places its JSON in a directory owned
by the process UID with mode `0700`; the opened regular file has mode `0600`.
The socket is created in that directory with mode `0600`, and the host checks
the connecting peer UID.
The directory's execute bit lets its non-root owner traverse to the grant;
mode `0600` on a directory would make the read path unusable. The file and
socket modes prevent other UIDs from opening this local inspection boundary.
The grant fixes repository identity, campaign/slice,
five exact gate source descriptors and capture hashes (including candidate 5's
coordinator event/output pair), six checkpoint refs/OIDs, three
review keys/heads, read audience, field allowlist, issuer provenance and a
time window. The host refuses an altered grant or source files during a read.
The review and gate readers independently check the opened grant file and
directory modes. JSON `issuer_provenance` is attribution; OS admission and a
trusted launcher, rather than that string, establish who could issue the read.

The client operation is `historical_s5.inspect.v1`. Candidate and review
selectors are confined to the admitted case. Each response names exact source
revisions/digests, source limits and gaps. The candidate 1–4 runner captures
carry `check.state` and no native campaign identity; checkpoint metadata,
gate digest and the trusted descriptor bind them. The candidate 5 coordinator
receipt has native identity and `check.result`. Both retain their original
canonical receipt digest. Review history is traversed to the null predecessor;
only finding IDs, statuses and severities leave the host. The replacement edge
requires the recorded authority reference to the original reported revision.
Historical checkpoint commits and review blobs are read with Git replacement
refs disabled and their raw bytes checked against the admitted object IDs.
The host bounds capture lines and prefixes, owner stdout/stderr, child process
time, and whole request frames. An unavailable owner produces an unavailable
read; it does not turn the local evidence map into a fallback authority.
SIGTERM and SIGINT stop the active owner process group, reap its children and
remove the host-owned socket so a fresh host can use the same path.

The UI uses `--history-connect SOCKET`; its `--history-snapshot WIDTH HEIGHT
[stage]` mode renders the same admitted projection for local qualification.
The frozen metrics row, provider attempts, quota and scheduler item have no
public reader in this increment. The UI labels the five receipt checks and
leaves the eight-stage historical metric unverified through this read.
