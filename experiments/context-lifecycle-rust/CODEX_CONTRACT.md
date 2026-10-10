# Codex observation contract experiment

This is the historical observation-only contract, **not a complete live-provider lifecycle**.
The later, separate text-turn completion domain and controlled native vertical
are documented in [NATIVE_VERTICAL.md](NATIVE_VERTICAL.md); old observation
records are never retroactively upgraded.
The existing coordinator remains the sole lifecycle transition owner. Its owned
executor commits effect entry before calling the common provider boundary.
`ProviderReply::Codex` carries a durable native observation, never a fake-provider
seal. Completed, failed, interrupted, pending, unknown, and conflicted native
observations are distinct from strict delivery outcome. The latter stays Unknown,
with no settlement evidence and no permission to advance FIFO.

## Bounded protocol

`src/codex.rs` owns a stdio app-server child and a separate SQLite ledger. It
records the exact effect/request digest and entered command receipt before
dispatch, then thread identity and transient JSON-RPC correlation before
`turn/start`. The response supplies the native turn identity. Early notifications
are buffered and associated only after that exact identity is known. They are
not guessed from `clientUserMessageId`, which has no assumed deduplication or
replay contract. Identical command keys replay receipts; changed content,
including changed rejected commands, is rejected.

An adapter restart never resends an entered `turn/start`. Missing native identity
stays Unknown. Known exact IDs can acquire positive observations through
`thread/read` with `includeTurns:true`; this adapter explicitly requests persistent
legacy history. Missing/unsupported history is uncertainty, not absence. It does
not implement paginated history, resume, arbitrary recovery, or a general SDK.

`turn/interrupt` is an explicit experimental provider-control action. Its entered
intent is durable before send; acknowledgment is not completion. The adapter
never automatically retries an uncertain interrupt. Observer disappearance never
interrupts the turn. Native process identities and actually observed exit status
are retained independently; an unobserved death is not given an invented status.
Neither exit nor terminal turn status proves tools, descendants, or remote effects
have settled. Async tools/background-terminal APIs preclude inferring a seal from
the inspected terminal notification contract.

The narrow Serde DTOs are selected projections, not a full Codex SDK. Unit tests
derive small JSON Schemas for `NativeStatus`, `Turn`, and `TurnNotice` from those
DTOs and validate representative fixtures with offline reference resolution.
Schema generation and validation are test-only; the adapter parses with Serde.
The schemas check required fields, field types, and status values, while item
content stays opaque and never becomes closure evidence. Shape and semantics
are different: `inProgress` is schema-valid but rejected as a terminal
notification. Wrong native IDs do not associate; identical terminal records are
idempotent; contradictory terminal statuses are preserved as Conflicted. EOF,
malformed frames, missing responses, and unavailable readback remain uncertainty.

Input is at most 2048 UTF-8 bytes; frames are capped at 1 MiB with 128 buffered
notifications. Response waiting is bounded, but blocking pipe/socket operations
and coordinator observation budgets are separate. No hard end-to-end timing,
production backpressure, shutdown protocol, or crash-proof arbitrary-descendant
cleanup guarantee is claimed. The local child has its own process group and Linux
parent-death signal; ordinary owned process cleanup kills and reaps it. Stderr is
discarded in this bounded harness.

## Version and sources

Inspected installed Codex version: **0.160.1**. The loopback launch verifies the
exact native executable and its SHA-256 before starting it:

```text
/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex
f34a4d2301892ae96c90097786bfe5dc269f187b6f69faf42a7b357b8c081e35
```

The local generated experimental schemas were inspected, not copied wholesale.
Selected source hashes (SHA-256) are:

| Generated schema | SHA-256 |
| --- | --- |
| v2/TurnCompletedNotification.json | `016870158603b0f84bd9f8f65f927161c9fd5128e5ec632087616462dc44e085` |
| v2/ThreadStartParams.json | `80a40a7fac15b4bf70efb7f893fb353acc0a0d30c68f54aee4f01923deca85de` |
| v2/TurnStartParams.json | `07771223642e1b61bd9aac0069fc0f98143a1c047724ca02c7ceb13653442738` |

The official [app-server documentation](https://learn.chatgpt.com/docs/app-server)
describes initialization, turn notifications, interruption, and history/readback.
The [configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference)
describes the selected provider and capability controls. This experiment binds
its implementation to the installed schema/binary, not all future documentation
or CLI versions. It fetches no arbitrary schema references at runtime.

## Provider-disabled evidence

One focused unit test generates and validates the selected DTO schemas, including
malformed required fields/types/status, Serde correspondence, and schema-valid
`inProgress` rejected by the semantic terminal check. Five integration tests use
a separate version-bound stdio fixture through the
same adapter process and real lifecycle coordinator:

- Delayed interrupted terminal, uncertain tool activity, wrong IDs, rejected-key
  identity, separate interrupt acknowledgment, and blocked second input.
- Terminal notification before start response, duplicate and conflicting status.
- Adapter/coordinator restart, durable exact native identity and readback, one
  native start, no replay.
- Lost start response, EOF, malformed frame, and shape-valid nonterminal terminal
  notification; restart/repeated command cannot produce a fresh start.
- Disconnected caller/observer leaves accepted execution responsibility intact.

No integration test writes lifecycle database rows to advance state. The fixture
control endpoint mutates only its simulated provider world.

The explicit installed-CLI probe launches the real pinned native binary with
`app-server --strict-config --listen stdio://`, clean environment, disposable
HOME/CODEX_HOME/work directories, and no auth. It refuses pre-existing auth in
that state directory and refuses the presence of unreviewed system Codex config.
No user auth/session files are inspected or copied. It validates reported state
directory/version and selected effective provider/permission config. Project
documents are disabled and the isolated project root is marked.

The generated profile disables shell, agents/multi-agent, hooks, web search and
default apps. Fresh state supplies no MCP/plugin configuration; thread/turn
requests specify `environments:[]`, and thread start specifies `dynamicTools:[]`.
The only configured inference endpoint is a loopback simulator with retries
disabled and no authentication. This is configuration isolation plus observed
requests, **not an OS-level network sandbox or universal tool exclusion proof**.

Retained successful probe evidence from 2026-10-07 UTC:

```text
native PID: 3902349
thread: 01a1148c-d415-7280-8220-dbf309b6f05b
turn:   01a1148c-d422-7ef2-a864-2c9e77df600b
offered tools: request_user_input, get_goal, create_goal, update_goal
forced exec_command result: unsupported call: exec_command
native observation: Completed
strict effect: Unknown; sealed: false
simulated inference requests: 2; authorization headers: 0
adapter/native restart: exact-ID legacy readback; no extra inference request
real-provider turns: 0
```

The final explicit rerun also passed: PID `3921526`, thread
`01a11491-b693-7d83-a280-02c8aeca8a03`, turn
`01a11491-b6a3-7551-ad48-3674bda4a33a`, the same four offered tools and exact
unsupported-call result, two simulated requests, no authorization headers, and
no additional inference on restart/readback.

The backend forces a harmless `exec_command` attempt, rather than relying on a
voluntarily tool-free answer. The CLI explicitly rejects that attempted tool.
**The remaining goal/user-input tools mean this profile is not tool-free.** Their
presence is a blocking result for any stronger text-only/capability-exclusion
claim, not a reason to hide the inventory or infer that tool effects are sealed.
This inventory is specific to this version, profile, and locally simulated
`gpt-5.1-codex-mini` model configuration; it says nothing universal about other
models or authenticated live configurations. No live trial was performed.

The subsequent `gpt-6-sol` profile and controlled text vertical are recorded in
`NATIVE_VERTICAL.md`, `profile-probe-2026-10-07.json`, and retained evidence.
The older mini-model result above remains historical observation and is not
promoted into a completion seal.

## Reproduce

Run from this standalone crate:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --test codex_observation -- --nocapture
cargo test --locked --test codex_observation installed_cli_loopback_contract_probe -- --ignored --exact --nocapture
```

The final command is explicitly opt-in, requires the exact installed binary, and
performs only simulated local inference. It bounds/owns child processes and
disposable state; the backend rejects authorization headers and accepts only the
two intended inference requests. Its output prints identities, actual tool
inventory, attempted-tool result, and the distinct strict outcome.

The executable entrypoint is `context-lifecycle-experiment codex-provider
LAUNCH_JSON`. `Launch` accepts only a fixture or a loopback socket address. The
tests construct the owned launch JSON and are the runnable bounded profile.
There is intentionally no authenticated/live launch path. A later live phase
would need separate human authority, deliberate credential isolation, verified
capability constraints for that exact profile/model, and independently admissible
effect-settlement evidence before claiming FIFO completion. None is supplied by
this observation adapter.
