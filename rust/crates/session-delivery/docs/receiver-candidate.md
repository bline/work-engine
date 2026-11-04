# S10 receiver candidate — Root disposition required

Read-only inspection of accepted S10 five-file source and pinned Pi 1.5.0.
Exact source hashes: `.delivery-evidence/receiver-source-evidence.json` in the
private delivery worktree. No receiver code or runtime executed.

The accepted host `create_args` explicitly disables extensions. Its `rpc_input`
permits only four controls. Pi RPC `prompt` delegates to session.prompt; steer
and follow_up enqueue model input. None is admitted as observation-only ingress
by the accepted S10 profile. Do not use prompt, naming, get_state errors or a
pipe write as an event receipt.

Concrete candidate: explicitly load one pinned first-party Pi extension, with
all other extension discovery disabled, inside the existing network-none,
credential-free, read-only runtime/workspace containment. On session_start it
reads one immutable, separately read-only mounted envelope and expected runtime
binding. After validating full delivery/attempt/payload/binding digests and the
native session identity, call `pi.sendMessage` with customType
`work-engine.delivery-observation.v1`, the exact envelope in details,
`display:false`, and `{triggerTurn:false}` while the session is idle. Pi's
agent-session.js sendCustomMessage idle branch calls _appendCustomMessage;
that updates the native session context/history and emits message_start/end.
The extension API is void/catches errors, so return from sendMessage alone is
not an ACK. Host must independently correlate emitted message_end AND
get_messages content with native get_state session identity and the exact
attempt. Export native session/history, raw RPC stdout, container inspection,
wait/exit/removal and receipt before cleanup. Durable native file existence
must be checked: current zero-message profile has no session-file proof.
A streaming/busy target holds; unknown ACK never retries. No application,
completion, semantic recovery or inference qualification follows.

Dependencies requiring Root/S10 disposition before implementation or execution:

1. S10-owned source/profile change: permit only this exact extension and
   immutable envelope mount; extend strict RPC verifier beyond its four-control
   profile to correlate get_messages/native event proof without changing Pi.
2. Explicit first-party JavaScript/TypeScript exception for the small Pi-native
   extension (Pi's supported extension language); Rust keeps persistence and
   host custody. No third-party runtime edits.
3. Root-reviewed exact source and immutable disposable probe grant binding
   extension, host, runtime, envelope, image, identities, mounts, deadlines,
   export locations and provider-disabled containment. Existing accepted smoke
   and fixture_only do not authorize this.

This is a feasible hook candidate inferred from exact installed source, not a
reviewed or supported receiving contract and not a successful real receiver.
Pressure enrollment can proceed independently using the existing reader.

Native-identity barrier still needs an S10 API decision: the current runtime
allocates its session at startup. A prewritten startup envelope cannot assume
that generated identity. A containing host must first observe/bind the actual
native session, then release exactly one immutable envelope to the idle hook
(for example a bounded one-shot data-file ingress after a ready barrier).
`session_start` alone is therefore insufficient as a complete receiver API.
The host must preserve the prepared attempt across this barrier and collect
native message/session proof before acknowledging; a file publication is
transport evidence only. This two-phase receiving hook is not implemented by
the accepted four-control host. Root/S10 must select and review it before a
probe; this worker makes no claim that an already supported ingress exists.
