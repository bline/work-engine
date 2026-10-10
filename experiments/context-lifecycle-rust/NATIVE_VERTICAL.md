# Native text lifecycle: bounded controlled result (2026-10-07 UTC)

The parent subsequently passed the isolated live three-turn proof. See
`PARENT_VALIDATION.md` and `evidence/live-text-vertical-2026-10-07/` for that
separate result, exact identities, coherent state backups, and limitations.
The three-turn authorization is fully consumed and the disposable profile has
been logged out. The preparation recipe below is documentation, not new
execution authority.

This standalone crate now has a separate `CodexTextTurnCompletion` evidence
domain. The fake seal and older observation-only Codex records are unchanged.
The coordinator alone admits and enters text operations, validates a bound
receipt from an exact full `thread/read` turn, and changes effect/delivery state.
Native `Completed` alone never does so. The receipt binds context and epoch,
effect and delivery, native thread and turn, request and pinned profile digests,
the final item and output digests, and (for readiness) continuation package
revision and digest. A contradicted finalized item fences the context. Missing
identity, final text, or exact readback remains uncertain; entered work is not
automatically resent. `Failed`, `Interrupted`, EOF, unsupported item activity,
or a native user-input request cannot satisfy successful text completion.
The pinned protocol's pure reasoning items and exact-ID answer/reasoning
stream deltas are allowed as non-effectful activity only when their lifecycle
item identities and final full readback agree; unknown item methods or kinds
still fail closed. Deltas do not substitute for authoritative final content.

Generation-to-thread creation intent is durable before `thread/start`; an
ambiguous creation cannot authorize an inference retry. The native thread ID is
durable before `turn/start`. A successor generation gets a distinct thread,
while readiness and its following input reuse that successor thread. Readiness
is a coordinator-owned entered effect with the same receipt path, not a harness
tail. It sends the exact frozen package and requires a mechanically checked
acknowledgment. `ActivateText` also requires the caller's explicit, limited
package-adequacy attestation; this is not a broad model-understanding claim.

The shared `run_text_vertical` driver (`src/vertical.rs`) invokes public
coordinator commands and expects exactly three entered native turns: A input 1,
B readiness, B input 2. It has no inference retry. The explicit
`native-text-vertical` client binary connects to a caller-supplied local
coordinator; it does not launch a provider, source credentials, or run inference
by default. The explicit `PreparedLive` endpoint and `CodexTextLive` mode use
the same driver with a distinct profile digest. `prepare-live-profile` creates
only empty isolated config/state; `native-live-preflight` issues read-only
config, account, and model-catalog RPCs. Neither logs in or sends a turn.

## Profile and controlled execution

The installed Codex binary is version 0.160.1, SHA-256
`f34a4d2301892ae96c90097786bfe5dc269f187b6f69faf42a7b357b8c081e35`.
The isolated, auth-free loopback profile selects `gpt-6-sol`, read-only
permissions, `features.goals=false`, `features.code_mode_host=false`, and no
configured shell, agents, hooks, web search, apps, dynamic tools, or MCP
endpoints. The adapter verifies pinned binary/effective config and uses a
local Responses simulator with no authorization header. The first Responses
request has `tools:null`; the developer message at `input[0].tools` describes
`functions` and `clock` namespaces, including `exec`, `wait`, and user-input
tools. A namespaced `functions.exec` call is internally rejected with exact
output `code-mode host is disabled`. Synchronous user input is unavailable in
Default mode. App-server user-input requests carrying an ID receive deterministic
`-32601` denial both during RPC wait and ordinary draining.

Namespaced asynchronous user input instead returns `{"accepted":true}`. The
native `item/started`, `item/completed`, and exact full `thread/read` all show
an `agentMessage` with `delivery:"async"` and nonnull `questions`, bound to the
same thread/turn/item. That is a durable `UserInputRequested` violation, even
if a later final message and native `Completed` arrive. The adapter neither
forwards the question to a human nor invents an answer. A denied, disabled-host
exec attempt is different: the simulator's exact call ID receives the internal
disabled-host rejection before any code execution; this pinned profile can
still accept a later complete text answer. This distinction is not a claim
about enabled hosts, other tools, or after-entry failures.

The opt-in actual-CLI controlled vertical passed with incremental Responses SSE
text and a reasoning summary on each turn. Exact readback contained
`userMessage, reasoning, agentMessage`, and native notifications included
`item/agentMessage/delta` and `item/reasoning/summaryTextDelta` tied to
completed lifecycle items. An exact-ID unit case rejects an orphan delta and
a mismatched turn. One coordinator PID survived
all commands; A and B used distinct native thread IDs; readiness and input 2
used B's one thread. The local simulator saw exactly three inference requests
and zero authorization headers. Outputs were `FACT:ORION-7`, an exact
`READY:<package digest>:FACT:ORION-7`, and `SECOND:ORION-7`. The second input
was delivered once on B. Input 2's prompt does not contain the fact value;
only B's carried context supplies it. Old A authority was rejected. The claim is local
single dispatch/no resend, not network-wide exactly-once inference. The
positive receipt, snapshot, request identities, model/profile facts, and actual
PID/cleanup statuses are retained in `evidence/controlled-text-vertical/`.
The controlled adapter and coordinator children were explicitly killed and
reaped by the test (`SIGKILL`); their exit is cleanup evidence, **not** remote
effect settlement.

Additional opt-in native evidence is retained under
`evidence/async-user-input-observability/`, `evidence/async-text-violation/`,
and `evidence/disabled-exec-text-rejection/`. Each contains bounded auth-free
native stdio and/or exact readback plus simulator exchanges; no user auth was
read or copied. The async text turn remains strict `Unknown` despite native
`Completed`. Default fixture tests cover early/delayed terminal, finalized
same-status content conflict, lost start response, incomplete final readback,
restart/no resend, stale authority/package, disconnected observer, and FIFO
uncertainty. Fixture control changes only its simulated provider world; it
does not edit lifecycle database rows.

The custom loopback provider's `model:"gpt-6-sol"` request and effective
configuration are not proof of the authenticated OpenAI model catalog or
capability profile. A parent-run live trial requires supported fresh login
in a disposable prepared profile, read-only exact model/account/config
preflight, and at most three entered turns **including** readiness or uncertain
attempts. The first parent attempt was rejected before effect entry by a
seconds-versus-milliseconds deadline mismatch; see
[the durable diagnosis](evidence/live-admission-rejection-2026-10-07/diagnosis.json).
No live inference was entered in that attempt, and this crate does not extract
or reinject user auth. No OS-wide sandbox, production provider integration, or
arbitrary external-effect seal is claimed.

## Parent-only prepared live entry

Run from this crate. The first block creates only disposable state and then
invokes the pinned native binary's supported interactive login in that state;
the parent, not this implementation run, must perform that login. It never
copies the normal Codex home or extracts tokens.

```sh
live_root=$(mktemp -d /tmp/context-lifecycle-live.XXXXXX)
native=/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex
cargo run --locked --bin prepare-live-profile -- "$live_root"
(cd "$live_root/work" && env -i HOME="$live_root/user" CODEX_HOME="$live_root/codex" PATH=/usr/bin:/bin "$native" login)
cargo run --locked --bin native-live-preflight -- "$live_root" > "$live_root/preflight.json"
jq . "$live_root/preflight.json"
```

The preflight starts the pinned app-server, reads `config/read`,
`account/read` with `refreshToken:false`, and bounded `model/list` pages. It
requires a ChatGPT account and an unhidden exact `model:"gpt-6-sol"` catalog
entry; its printed result omits account email and has `inference_turns:0`.
The live adapter repeats these read-only checks before accepting provider
commands. The result binds the selected effective config and model metadata
digests, but does **not** prove the authenticated first inference request's
offered-tool inventory equals the loopback inventory. The parent must review
this limit and the effective capability exclusions before any turn. An absent
or contradictory catalog entry is a stop, not a reason to guess an alias.

Only after that review, create the explicit launch and three-turn config:

```sh
cargo build --locked --bins
# Coordinator deadlines use Unix epoch milliseconds, as SystemClock does.
live_deadline=$(($(date +%s%3N) + 600000))
jq -n --arg native "$native" --arg root "$live_root" \
  '{executable:$native,home:$root,endpoint:"PreparedLive"}' > "$live_root/launch.json"
jq -n --argjson deadline "$live_deadline" '{
  first_input:("Reply with exactly FACT:ORION-7 and nothing else."|explode),
  second_input:("Use the continuation fact already carried in this thread. Reply with SECOND: followed by its value, and nothing else."|explode),
  first_fact:"FACT:ORION-7", second_fact:"SECOND:ORION-7",
  continuation_objective:"Preserve FACT:ORION-7 for the queued second input",
  limited_package_adequacy:"Caller attests only that the exact package includes FACT:ORION-7",
  runtime:"prepared-live-coordinator",
  authority:{approval:"explicit parent-only three-turn live trial",deadline:$deadline,mode:"CodexTextLive"},
  transition_deadline:$deadline,observation_timeout_ms:120000
}' > "$live_root/vertical.json"
```

Start the local provider and coordinator separately; wait for each single
readiness JSON line before reading its address. These commands are deliberately
not a test:

```sh
target/debug/context-lifecycle-experiment codex-provider "$live_root/launch.json" > "$live_root/provider.ready" 2> "$live_root/provider.err" &
provider_pid=$!
# Wait for provider.ready to contain one JSON line and confirm provider_pid lives.
provider_addr=$(jq -er .address "$live_root/provider.ready")
target/debug/context-lifecycle-experiment coordinator "$live_root/coordinator.db" "$provider_addr" > "$live_root/coordinator.ready" 2> "$live_root/coordinator.err" &
coordinator_pid=$!
# Wait for coordinator.ready to contain one JSON line and confirm coordinator_pid lives.
coordinator_addr=$(jq -er .address "$live_root/coordinator.ready")
target/debug/native-text-vertical "$coordinator_addr" "$live_root/vertical.json"
```

The final command is the only inference-sending step, with at most three
coordinator-admitted turns in order. Every entered failure or uncertainty uses
that turn's budget; no driver retry is authorized. On either outcome, retain
the isolated directory for review and stop/reap exactly `coordinator_pid` and
`provider_pid`. A native child exit is cleanup evidence, not settlement.
The client rejects expired or seconds-scale deadlines before any coordinator
command. A failed prior run is not made reusable by editing its configuration:
command identities and rejection receipts persist in the coordinator state.

## Experimental UI handoff

`Observer::snapshot`, `result`, `wait`, `bootstrap`, and `readiness_package`
are read-only projections. `Client::register`, `admit`, `replace`,
`begin_readiness`, and `activate_text` expose the tested command path; `Owner`
and `CapturePermit` still delimit authority, with typed IDs and durable
rejections. A UI may render `Snapshot`/`Effect`/`Transition` and controlled
receipts experimentally. These DTOs and command/error contracts are not frozen
for production. The UI must not infer success from native terminal status,
rendered text, process exit, or a mere readiness package; only the coordinator's
domain-specific accepted outcome is a delivery fact. The separate UI remains
outside this crate.

`SyntheticAuthority.deadline` and `TextVerticalConfig.transition_deadline` are
absolute Unix epoch milliseconds; `observation_timeout_ms` is a duration in
milliseconds. The controlled actual-CLI vertical uses a bounded epoch-millisecond
deadline as well.
