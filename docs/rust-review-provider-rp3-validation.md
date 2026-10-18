# RP3 installed profile: offline qualification

RP3's approved first step qualifies an installed Claude executable only through a
non-authorizing, no-egress local probe. It does not install campaign acceptance,
consume an Applied permit, read a production credential, invoke a provider, or
qualify a live initial review. The separate live envelope in
`rust-review-provider-rp3-implementation-plan.md` remains unapproved.

The probe uses the E1 selected review argv, pinned program and guard, Bubblewrap
mounts, READY framing, private session, cgroup and zero-swap policy. Its
test-support-only launcher adds `--unshare-net`; the guard checks the realized
network namespace and routes before READY and before any child execution. Only
after READY does the harness send a synthetic canary and prompt. The guard
emits a post-`spawn` marker with the selected executable's device and inode.
The harness verifies that marker, bounded capture, cgroup memory peak, OOM
events and swap state. The probe root has no production writer or campaign
authority. Neither a checked READY line nor the CLI's own output alone proves
the installed program was spawned.

The synthetic installed-like peer passed this mechanism under the original
combined 64 MiB cgroup and the new versioned combined 1 GiB trial policy. A
local compatibility candidate, Claude executable 2.1.300 at SHA-256
`674deba0bba9aee479f4503c438b49303b6fdd304b9a471b14977aaec5215453`,
with `opus`/`high` review flags, reached READY and the post-READY spawn marker
inside the denied-network namespace under the original policy. It then hit that cgroup limit:
`memory.max=67100672`, `memory.peak=67100672`, `memory.swap.max=0`,
`memory.swap.current=0`, `oom_kill=1`, guard exit 69, no stdout, and no local
milestone. This is preserved evidence of the original policy, not a current
viability verdict.

The user-authorized successor `session64mib_combined1g_zero_swap_v2` bounds the
combined process/tmpfs/kernel cgroup plus the checked host session-custody
charge to 1 GiB. For the same local candidate, the exact realized
`memory.max` was 1,073,733,632 bytes after subtracting a 4,181-byte host
charge and rounding down to a 4,096-byte page. The selected executable again
spawned after READY with external egress denied and no production credential; its measured peak was
71,741,440 bytes initially and 71,835,648 bytes on the final repeat, with zero
swap and zero OOM kills. The guard exited 1 with no
stdout after a bounded 231-byte stderr. This satisfies only the narrow local
selected-argv spawn milestone; no authentication, MCP exchange or structured
review result was observed. `opus`/`high` remains a **non-authoritative local
compatibility candidate**, not a production owner selection. The old physical
combined 64 MiB limit is not claimed under v2. The separate 16 MiB active
content/export bound, per-scratch data bounds and checked 64 MiB host
live/retained/import/snapshot accounting remain in force; they are not an
independent 64 MiB physical tmpfs/kernel-memory limit.

Credential format, authentication, network reachability, complete review
memory, remote model identity, billing and hard spend control remain
unobserved. A future live authorization requires exact protected model,
credential, route and spend-owner selections and a separately reviewed
envelope; this offline result grants none of them.

Builds used task-owned `/dev/shm` Cargo target after the repository filesystem
hit quota. Credential tests used a private disk-backed `TMPDIR` because their
same-core guard rejects `/dev/shm`'s world-writable ancestor. These host build
and test locations are outside the guarded child and do not increase its
`/session` writable mount or cgroup limit. Raw logs and frozen receipts are
persisted under `/home/bline/.local/state/work-engine/rp3-implementation-20261017`,
not in the temporary build cache.
