# Docker retained-session feasibility, 2026-11-03

Dated bounded experiment, not accepted or integrated. Base `ab906b90`.
One-off Python probes compose the existing Docker/native Codex/role-host CLIs;
no reusable executable component or production script was introduced.

The explicit private app-server supports separate exec clients, retained native
SIDs, durable raw rollouts and existing metric readers in a cached Python image.
An offline OSS `codex exec` startup attempt produced no role-host socket; it
failed before inference and does not establish authenticated exec behavior.

Four fixed model turns used the same container and server: builder, reviewer,
retained builder, same reviewer. Both actors retained their private markers.
The artifact loop **failed**: the initial mount included `codex` but omitted its
matching `codex-code-mode-host`. Every model shell action failed closed, so
`answer.txt` never existed. A per-thread `features.code_mode_host=false` request
did not restore shell tools. The builder reported its correction was not written;
the reviewer left its finding unresolved. Native completed status alone was
insufficient to establish the fixture result.

The next bounded step is to mount the complete matching native distribution
(read-only, at its expected package layout), then qualify a fresh clean loop.
An offline `codex-code-mode-host --help` probe using that full distribution
already exits 0 in the cached image. No further model turns were attempted.

`offline_probe.py` and `qualification_turn.py` are historical one-off probes
with fixed owned paths and exact actors, not general launchers. They must not be
rerun against existing state or used as production orchestration. The result
records every failed setup as well as successful checks; credentials are absent
from source/evidence. The sole auth route was a narrow read-only native file
mount into the container. Both actors had dangerFullAccess inside the container;
this establishes no production reviewer independence, network allowlist, frozen
startup input set or pilot qualification.

See `result-20261103.json` for exact identities, private evidence paths and raw
rollout digests. Canonical raw records also remain under the explicitly owned
host sessions namespace, preserving unmodified session_meta and native tokens.
The usage ledger priced 8 actor calls at $0.1566072 API-equivalent (proxy, not
billing or quota). Both child-context reads and all four exact-turn settings
reads succeeded. All owned containers were collected and removed; the retained
app-server exited 0. No image, volume, daemon config or production file changed.
