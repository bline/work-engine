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
The UI labels the five receipt checks separately from the frozen eight-stage
metric; that discrepancy remains unresolved.

The `historical_s5.inspect.v2` increment adds a private Rust
`historical_s5_source_read` helper beside the host. The trusted launcher issues a
new version-2 grant with two exact rollout captures, eight result locations, one
Role Scheduler item descriptor, and one frozen supervisor metrics-row descriptor.
The host supervises the helper with the same process-group, byte and deadline
limits as its legacy owner calls; the host and helper are installed together in an owned mode-0700 directory;
the helper must be an owned, non-group-writable regular executable, and its
bytes join the read-source anchor. This prevents another group member from
replacing the helper between source checks through a writable build directory.
The host rechecks implementation bytes and the two captured files plus the
scheduler database/WAL companions across the complete v2 operation. One
30-second operation deadline covers legacy and new owner reads; the client
allows 35 seconds for the response. The v2 grant's nested source descriptors
have exact typed schemas and fixed case bindings, with unknown or duplicate
fields refused before any owner read. Scheduler inspection enters a Linux
Landlock no-filesystem-write domain before SQLite opens the DB; if this
protection or a side-effect-free WAL read is unavailable, the scheduler read
fails closed. Only an exactly missing frozen metrics commit is a typed optional
unavailable comparison, without stage or token comparisons. A failed source
fence, Git identity/digest mismatch, malformed row, unexpected helper failure
or deadline fails the whole read even if source bytes are restored before a
later check. Actual source bytes remain hashed before and after every v2 helper
call and before response publication; a larger read chunk and optimized SHA-2
dev profile change speed, not the ordered digest or admission contract. The
interactive terminal shows a neutral loading frame before admission is known,
keeps input responsive during refresh, and owns a nonblocking local socket with
one absolute 35-second attempt deadline covering explicit-refusal v1 fallback.
Leaving the view closes that socket; it does not cancel the host's bounded owner
work remotely. Snapshot reads remain a separate synchronous qualification path.
The v1 operation and its grant remain available for older clients. A v2 failure
does not fall back to the local evidence map or a partial v1 story.

The new source modules implement [captured provider-result read](../../../skills/slice-builder/references/historical-provider-capture-read.md),
[exact scheduler-item inspection](../../../skills/role-scheduler/references/historical-inspection.md),
and [frozen metrics comparison](../../../skills/slice-supervisor/references/historical-metrics-inspection.md).
Provider cost stays as a literal decimal string. Eight captured literals sum to
`15.7545494999999994`; the frozen row reports `15.7545495`, equal after rounding
to seven decimals. The failed 429 result has 27 thinking tokens, accounting
mechanically for the difference between captured `101224` and reported `101197`.
The scheduler displays only its current acknowledged row and exact review result
ref; `presented_at` is null and no mutation history or UUID-to-schedule causal
edge is supplied. The five-check receipt versus eight metric stages remains
unresolved. These limits remain visible in the terminal view; this local case
read is not full UI-0 acceptance.
