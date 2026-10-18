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

`historical_s5.graph.v1` is an additive private operation on the same socket
and version-2 grant. After one admitted v2 read, the host derives a disposable,
case-scoped owner graph. The response includes the full validated v2 summary,
a finite registry of owner/source classes and legal outgoing relations, sorted
nodes and edges, source limits, and explicit gaps. Its deterministic historical
IDs use checkpoint OIDs, gate digests, review revisions, captured result UUIDs
and capture hashes, scheduler row digest, or frozen metrics row digest. They do
not use the grant ID, executable-byte anchor or read time. The client
reconstructs the graph from the validated base and refuses a changed owner,
class, endpoint, identity or edge. All three operations keep the 131,072-byte
frame limit; graph and legacy owner work share the 30-second host deadline and
one graph→v2→v1 attempt shares the client's 35-second absolute deadline. A
refused newer operation may fall back to an older admitted operation, while an
unavailable or corrupt graph cannot be silently downgraded.

The graph registers selected checkpoints, five gate captures, 14 linked review
revisions and their finding states, eight captured CLI results, one current
acknowledged scheduler row and an optional frozen metrics row. The replacement
authority edge originates only at the candidate-5 review head; candidate 3
reaches its own exact review revision and the recorded revision chain. The
terminal labels jumps to the head, captured result or current scheduler row
as **case navigation** because no causal edge is admitted for those jumps.
The scheduler's outgoing edge binds its exact candidate-4 review result;
the captured 429 does not point causally to the scheduler. The derived cost sum
uses eight distinct captured UUIDs, not billing authority. The five-check and
eight-stage facts keep separate units and unresolved semantics. The graph
v1 does not add repair-contract, selected-review/acceptance, full scheduler
mutation, presentation or provider-billing evidence.

`historical_s5.graph.v2` adds one mandatory, case-fixed repair-contract source
under a private version-3 grant. The Rust helper verifies the exact parent and
repair Git commits, trees, script and SKILL blobs as raw objects, with Git
replacement disabled and SHA-256 checks on selected file bytes. The grant adds
no arbitrary commit or path selector. The existing gate and review owner
readers accept only this exact v3 descriptor; they keep their v1/v2 selectors
and validation. The host rechecks the grant, helper and other source bytes
across the whole 30-second read. The v3 graph response uses a checked table of
edge provenance to retain the 131,072-byte frame bound; the client reconstructs
and validates every edge from the admitted base and repair facts.

The graph has two immutable contract-revision nodes and an observed Git
revision edge. A separate semantic-interpretation edge relates the new rule to
the original generic episode's exact active/reported revision. That edge names
`s5-contract-applicability-interpreter` and authored rule
`v1@historical_s5.graph.v2`, with distinct Git and review support bindings.
Its edge ID includes that authored rule revision, so a later interpretation
cannot silently reuse the v1 identity over the same historical subjects.
It is an attributed case-scoped interpretation, not an Independent Review
State assertion that the repair was deployed in S5. The terminal shows both
supports, confidentiality, completeness, cutoff and the limit: there is no
proved S5 adoption, remediation event, cause of replacement or savings.
Graph.v2 refusal can fall back to graph.v1 and then inspect.v2/v1 under one
35-second client deadline; graph.v2 corruption or unavailability cannot.
Selected-review/acceptance consequences, complete scheduler history and
provider billing remain open; full UI-0 acceptance is not claimed.
