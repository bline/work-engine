# E1a DeepSeek battery arm — Host widening design (2026-11-20)

Authority: operator decision 2026-11-20, option A ("pinned battery widening to the
hosted coding lane, offline-qualified, then 1 arm, 3 if clean"). Owner: this
session (OpenRouter lane). Branch: `rescue-detached-20260911`. Baseline for the
arm: `1e21e71c` (E1a packet, `~/cbm-work/e1a`).

## Problem

The merged lane (live-qualified in `records/openrouter-lane-live-20261120`) cannot
execute a real builder arm. Three independent blockers, discovered 2026-11-20:

1. **Admission caps** (`work-engine-manual-host/src/pi_docker.rs check_job`):
   every non-relay fresh job must have `deadline ≤ now+300s`;
   `command_timeout_seconds` is restricted to `1..=30`. A builder arm needs
   hours and multi-minute cargo builds. The only 3600s exception is the relay
   profile, which has no workspace effect.
2. **The gate is a fixed choreography** (`pi-pressure-context-host/src/coding.rs`,
   `CodingContinuity`): phases are hard-wired — one task turn (phase 0), one tool
   call, one physical reset + rehydrate (phases 1–2), one continuation prompt
   ("compute 6*7", phase 3), then the report closes (phase 4). The budget check
   hard-codes `max_provider_requests == 4`, `max_tool_calls == 1`; the producer
   (`coding-qualification.rs`) hard-codes `max_tool_calls=1`, boundary
   `src_lib_rs`, fixed packet bounds. A builder arm needs a bounded multi-turn
   domain-work loop with replacement-on-pressure.
3. **Bridge protocol caps context at ~32 KiB** (`receiver/pi-coding.mjs`): every
   `coding_provider_request` frame carries the whole SDK context twice
   (`messages` + `agentMessages`, the known D4 duplication) under a fixed
   `FRAME_BYTES=65536`; the hosted request body is inline base64 under
   `MAX_BODY=45000`; `MAX_REQUESTS=4` is a shim constant. A real E1a arm needs
   hundreds of KiB of context. Numbers cannot simply be raised on the existing
   inline protocol; large bodies need a spooled-file + digest-reference channel.

## Design (option A)

### New profile: `work-engine.pi-coding-hosted-battery.v1`

A hosted live profile identical in custody (Host-held access, network-none
inference container, hosted routes) to `pi-coding-hosted.v1`, but admitting a
bounded builder run. Selected only by its exact schema string; no field toggles
the semantics of the existing profile. New profile pin constants in
`pi_coding.rs`; `is_hosted`/`is_live_bridge` extended; `tools_match`/`ready_tools`
treat it as live-with-shell.

Battery profile bounds (all pinned in the job + admission, exact-bound like the
lane's other pins):

| bound | battery value | rationale |
|---|---|---|
| deadline | ≤ 4 h (14400 s) | a Claude E1a arm historically took 1–3 h |
| command_timeout_seconds | ≤ 600 | offline cargo builds take minutes |
| max_tool_calls | ≤ 512, admission-bound | builder edits/builds/tests |
| workspace_write_boundary | `legacy_workspace` (whole repo) | builder owns the crate + docs |
| max_requests | ≤ 512, admission-bound | bounded by budget, not count |
| max_context_bytes | stage-2 bound | see protocol below |
| toolchain/acceptance | pinned as today | toolchain = offline rust toolchain; acceptance optional (see below) |

Acceptance: for the battery profile the acceptance artifact becomes **optional**;
the E1a arm's real acceptance is the F3.1 scorer run by the coordinator after the
run (`~/cbm-work/f3-battery-scorer`, counts-only). If an acceptance artifact IS
pinned, it runs exactly as today. Default profiles unchanged.

### New gate mode: battery continuity (`CodingBattery` in `pi-pressure-context-host`)

Resolved design detail (2026-11-20, second design pass): the replacement
machinery (`prepare_reset`/`replaced`/`finish_rehydration`/`replacement_admission`)
is hard-wired to ONE cycle through fixed ids — `session:new`, `context:new`,
`command:replace`, `grant:replace`/`grant:new`/`grant:successor`, fixed
`EvidenceId` strings. The subject's successor generation is also fixed in the
command basis. A battery arm WILL hit the model's context window (the receiver
disables auto-compaction; without replacement the run dies at the provider
window or the context gate), so multi-cycle replacement is required, not
optional.

Therefore the battery mode parametrizes the choreography by **cycle index `i`**
(0-based; the producer installs up-front, per the packet's pinned `max_resets`):

- grants: `grant:old` (context:old, cycle-0 turns), and per cycle `i ≥ 1`:
  `grant:replace-i` (bound context:new(i-1), scope RequestReplacement),
  `grant:new-i` (bound context:new(i), NativeTurn), `grant:successor-i`
  (bound context:new(i), EnqueueInput);
- replacement command per cycle: `command:replace-i`, context generation
  `context:new(i-1)`, successor `context:new-i`, session `session:new-i`,
  evidence ids suffixed `-i` (e.g. `coding:host-writers-absent-i`);
- the rehydrate ack per cycle uses purpose `SuccessorRehydrate` with the v4
  owner-derived checkpoint read from the previous settled turn (existing
  machinery; the reasoning-only-recovery waiver from 4443b7bd applies per cycle);
- battery turn loop: after each settled DomainWork turn, the gate decides:
  (a) workspace completion signal (see below) → report + close;
  (b) context pressure (last request's `contextBytes` ≥ the packet's pinned
  `replacement_basis_points` of `max_context_bytes`, or the provider window is
  near) → replacement cycle → rehydrate → continuation prompt;
  (c) else → continuation prompt on the same session (new native turn,
  `DomainWork`), prompt text "continue";
- completion signal: `evidence/completion.json` in the Host's
  `state_dir/coding-workspace` (the tool container's writable /workspace for the
  LegacyWorkspace boundary) with status `complete`/`handoff_ready`; the gate
  reads the durable file — the model's final text is provenance only;
- bounds enforcement: finite counters (requests, tool calls, resets) pinned in
  the packet; exhaustion = fail-closed with retained custody;
- close: report family as today plus battery counters (turns, resets, cycles);
  `continued_parent` semantics preserved.

Reuses `CodingLifecycleGate` wholesale for tool custody (`execute`/`observed`
are per-call and mode-independent).

### Stage 2 — large-context bridge protocol (required before any live arm)

The E1a arm's context will exceed the current inline frame protocol within the
first few turns. Design:

- The receiver spools large outbound provider bodies to files under a
  Host-provided spool directory (new job input, 0700, outside the workspace
  inventory) and emits only `{path, sha256, bytes}` in `provider_fetch`; the
  Host reads the file, verifies the digest, and forwards. Inlining stays for
  small bodies (threshold pinned).
- `coding_provider_request` frames carry the context **once** (drop the
  `agentMessages` duplicate; the Host reads the spooled context from the request
  body it forwards — the duplicate was only for settlement evidence; retain
  digests in the frame for provenance). `FRAME_BYTES` for battery frames raised
  via config (analysis precedent: `config.frameBytes`).
- `MAX_REQUESTS` and `MAX_BODY` become config-driven for the battery profile
  (analysis precedent: `config.maxRequests`/`config.maxBodyBytes`).
- Offline fixture must include a large-context case (spool path, digest
  verification, refusal on digest mismatch) before live.

### Offline qualification before live

- `coding-qualification` gains a battery branch: battery packet schema
  (`work-engine.coding-battery-packet.v1`), battery bounds, profile
  `pi-coding-hosted-battery.v1`; offline path uses `LoopbackTransport` + a
  scripted fake provider driving ≥ 2 domain-work turns, ≥ 3 tool calls, one
  replacement cycle, and the completion-signal close.
- Existing qualification choreography untouched (same suite, 303 tests must stay
  green).

### Live E1a composition (after offline qualification)

- `arm_run.py prepare` already produced arm `e1a-D1` (baseline `1e21e71c`,
  env verified 0 mismatches). The hosted arm's task text adapts the frozen
  builder prompt: same contract/interface/scope/commit/completion obligations,
  transport-neutral instructions (no claude_transport/child_context), Host-pinned
  model statement (`~deepseek/deepseek-v4-flash-latest:floor`, effort high via
  shim policy).
- Workspace = the arm's repo clone (65 MB, whole repo); toolchain = the pinned
  offline toolchain; cargo registry exposure for offline builds must be resolved
  in stage 1 (how `TOOLCHAIN` is mounted for `workspace_exec` — check exec
  composition; the arm's `cargo-home/registry` symlink may need a pinned mount
  or the toolchain root must embed a vendored registry).
- Budget: admission ceiling ~$5 (lane plan proposal); floor pricing; operator
  monitors spend.
- After the run: F3.1 scorer `battery_score.py score --interface e1a-v1`
  counts-only on the arm's HEAD commit; report counts (pass / confirmed_failure
  / nonblocking_difference / indeterminate).

## Status / ledger

- 2026-11-20: design written.
- 2026-11-20 Stage A COMMITTED `3353c34b`: hosted battery profile admitted
  (deadline 4 h, command timeout 600 s, tool calls ≤ 512, legacy_workspace
  write boundary, plan bounds 512 requests / 2 MiB context, acceptance
  optional; receiver staging + shim battery-aware; suites green). Remaining:
  (B) producer battery branch + parametrized-cycle gate loop (see the battery
  gate section above for the exact per-cycle id scheme); (C) stage-2 spool
  protocol (receiver spools large context/body to `state_dir/coding-spool`
  mounted rw at /spool in the battery inference container; frames carry
  `{path, sha256, bytes}` + scalars; Host reads + digest-verifies + forwards;
  battery mounts + `verify_inference` expected-mount update); (D) offline
  fixture end-to-end (loopback transport, ≥ 2 domain-work turns, ≥ 3 tool
  calls, 1 replacement cycle, completion-signal close); (E) live composition
  (arm e1a-D1: workspace = arm repo, battery toolchain Tree composed from
  ~/.rustup/toolchains/1.92.0 + registry hardlinks, task = adapted builder
  prompt, model ~deepseek/deepseek-v4-flash-latest:floor, effort high via
  shim) + run + F3.1 scoring.
- Resolved (was open item 1): the tool container mounts `toolchain` at
  `/toolchain` (read-only), acceptance at `/acceptance.rs`, and the workspace at
  `/workspace` — **`LegacyWorkspace` boundary already gives a writable
  whole-repo tool container** (RW=true on the workspace bind); `PATH` already
  includes `/toolchain/bin`. The pinned offline-rustc toolchain has only
  `rustc`; the battery arm therefore pins a **composed battery toolchain Tree**:
  a hardlinked copy of `~/.rustup/toolchains/1.92.0-x86_64-unknown-linux-gnu`
  (cargo, rustc, clippy, fmt) plus `cargo-home/registry` (hardlink of
  `~/.cargo/registry`, 1.4 GB, read-only inside the read-only mount). No new
  profile field, mount or env change: the builder invokes cargo per-command with
  `CARGO_HOME=/toolchain/cargo-home` (instructions in the battery task text and
  the battery `workspace_exec` description). The E1a crate is pure Rust (serde,
  serde_json) — no `cc` needed. Container limits (512 MiB, 1 CPU, 64 pids) are
  tight for cargo; the builder should scope builds to the target crate.
- Remaining open items: (2) `max_context_bytes` exact stage-2 bound vs receiver
  frame math; (3) shim `workspace_exec` description is hard-coded to the tiny
  fixture ("Only edit src/lib.rs") — battery needs repo-oriented text; (4) Fable
  review of the widening before the live run (claude-sdk quota resets 04:40 MST).
- The qualified producer binary `d5575836…` stays valid until this work lands;
  rebuild + re-pin after any source change.
