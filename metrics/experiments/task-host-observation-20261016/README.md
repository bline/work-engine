# Driving the Rust task host offline, 2026-10-16

The S8 retained-task host was launched as a second, isolated instance and driven
through submit, observe and replacement with **no provider and no quota**. Every
reply in this directory is a verbatim capture from a running host.

## How it ran

The launch recipe came from the session that built the UI development host. It
reuses that host's pinned configuration, rewriting only the instance paths, so
the executable, source manifest, lock and toolchain digests still verify:

```bash
host_dir=$(mktemp -d /home/bline/.we-ui-host-XXXXXX)
# rewrite every occurrence of the original host dir to $host_dir in
# host-startup.json, and set lifecycle.proof_run_id to the new dir name
chmod 600 "$host_dir/host-startup.json"                 # see below
(umask 077; mkdir -p "$host_dir"/{barrier,native,lifecycle-store})  # see below
<pinned-binary> serve --startup "$host_dir/host-startup.json"
```

**Two steps the handed-over recipe omits**, each of which fails as the same
opaque `task host configuration is invalid`:

1. `validate_file` rejects any group or other permission bit on the startup file
   (`meta.permissions().mode() & 0o077 != 0`). Python's default umask writes
   0644/0664, so the file needs `chmod 600`. The original worked because it was
   written under a 077 umask.
2. The `barrier`, `native` and `lifecycle-store` directories must exist **and be
   0700**. Creating them under a default umask produces 0775 and still fails.
   Without them the error changes to `task owner failed: task input is invalid`.

Neither failure names the offending check, so the reason has to be found by
replicating `validate_startup` externally. That is a usability cost on a path
every future qualification run has to walk.

## No quota is at risk on this route

`validate_startup` requires `provider_identity == "native-sim-proof-v1"` and
refuses anything else for this profile, so the offline simulator is not a
workaround but the only accepted identity. The configuration points
`simulator_base_url` at `http://127.0.0.1:1/v1` — loopback port 1, where nothing
listens — and the `grants` array carries only `enqueue_input` and
`request_replacement`. The native-turn and semantic-control grants the native
block names (`grant:s8:native`, `grant:s8:semantic`) are **absent**, so no
inference can be authorized.

## What the host did

| operation | reply | result |
|---|---|---|
| `task.submit.v1` | 1,028 B | `admitted`, `outcome_kind: enqueued`, delivery ref |
| `task.observe.v2` (populated) | **5,222 B** | `observation`, 5 components |
| `task.observe.v2` (absent task) | 1,388 B | components `not_applicable` with `reason: task_absent` |
| `task.request_replacement.v1` | 906 B | **`replacement_requested`**, transition ref, revision 2 |
| `task.observe.v1` | 60 B | `unsupported_observation_version` |
| unknown op | 51 B | `invalid_or_unavailable` |

The largest reply is **4.0% of the 128 KiB wire bound**.

Populated observation, by component:

| component | state | bytes |
|---|---|---|
| `owner` | present | 3,116 |
| `lifecycle` | present | 864 |
| `native_delivery_status` | present | 333 |
| `command_result` | not_requested | 125 |
| `control_result` | not_requested | 125 |

## The replacement stops honestly

After `task.request_replacement.v1` was accepted, the transition reports
`stage: "quiescing"`, `completed: false`. With no semantic grant and no reachable
provider it cannot compile a continuation plan, and it **does not report a
completed replacement anyway**. `runtime_provenance` is `{"state": "unknown"}`
rather than a fabricated value.

So this route can qualify admission, observation and replacement *initiation*
end to end. It cannot yet produce a `ContinuationPlan`, which is what T30 needs
a measured size for. That requires a configuration carrying the semantic control
grant plus `lifecycle-proof` scripting the loopback provider's responses — a
separate build, not a variation of this recipe.

## Against the UI consumer's four gaps

[`rust/crates/terminal-ui/docs/task-observation.md`](../../../rust/crates/terminal-ui/docs/task-observation.md)
listed four gaps requiring owner reconciliation. The live v2 reply addresses
three of them explicitly:

| gap | what the live reply shows |
|---|---|
| partial reads lose their meaning | per-component `state` of `present` / `not_requested` / `not_applicable` / `unknown`, with `reason` (`task_absent`) |
| the composite is not atomic | top-level `consistency: "independent_components"` |
| native history has a capacity boundary | `completeness: {"kind": "complete_for_scope", "limit": 64}` |
| ordering and scope are limited | each component carries its own `scope` and, where available, a `revision` cursor (`store_id`, `stream_id`, `commit_sequence`) |

## One divergence from the consumer document

That document proposed `task.observe.v2` "with v1 behavior retained", and the S8
plan's pre-authorized route said to "add a versioned supported projection rather
than silently changing v1". **v1 is not retained.** It returns
`unsupported_observation_version`, pinned by a unit test at
`rust/crates/work-engine-task-host/src/lib.rs:1091` in both the qualified S8
snapshot and the repository working tree.

The refusal is explicit rather than silent, so it is truthful. It is still a
retirement, not a retention, and any client written to v1 breaks. Recorded here
as an observation for the S8 owner, not a defect finding — this session has no
authority over that decision.

## Provenance

Captures are verbatim socket replies, newline-terminated, SHA-256:

| file | bytes | sha256 |
|---|---|---|
| `observe-v2-populated.json` | 5,222 | `f0fa3627b72ea2b850f5c301362345d5…` |
| `observe-v2-absent.json` | 1,388 | `e2206aec246dfed3d53a303f53518f38…` |
| `observe-v1-refused.json` | 60 | `f9dcbc2ff1365286b281624a1ddd15ab…` |
| `unknown-op-refused.json` | 51 | `f1ee231d72ed4c407418f178888615fe…` |

Host identity in the replies: `provider_identity native-sim-proof-v1`,
`features ["controlled-proof"]`, `profile retained-repository-analysis-v1`,
executable `2703037b…`, source `4f446a2a…`. The instance was stopped and its
directory removed; the UI session's systemd host was never touched.
