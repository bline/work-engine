# Claude native new_context probe — 2026-11-15

Scope: the development-agent Mods adapter and shared skill guidance. No Rust
runtime changes. These observations qualify native control behavior separately
from semantic recovery, context-pressure enrollment or production review custody.

Claude Code **2.1.325** loaded the local `work-engine-context` plugin, exposing
`mcp__work-engine-context__new_context` with `{}`. The adapter's SHA-256 was
`766dbbf30647cfeab04d08b2ddc1f0d9431ba8918f5491cb88de96e85d2d645f`.
The [structured observations](claude-native-new-context-20261115.json) retain
the session IDs, checks and control records. The
[plugin README](../../skills/manual-orchestration/claude-context-plugin/README.md)
documents launch and operation; the
[scripted driver](../../skills/manual-orchestration/claude-context-plugin/tests/native-driver/README.md)
documents reproduction.

## Observed results

| Case | Observation | Claim supported |
| --- | --- | --- |
| Offline validator | Strict validation passed; optional catalog icon warning | Installed host accepts manifest and hook registration |
| Offline tests | 19 passed, 0 failed across two files | Deferred scheduling, duplicate suppression, permission/subagent boundaries, cancellation, clear/prompt failures and persistence reporting under modeled host conditions |
| Open stream-json input | Two successive reset cycles in one actual CLI process | Native clear, changed session ID, automatic recovery turn and continuing tool registration |
| Interactive terminal | One reset and recovery cycle | Same native control behavior on the interactive surface |
| Closed text input | Request acknowledged; process exited before native clear | A pending acknowledgement does not establish reset or continuation |
| Closed stream-json input | Same exit-before-clear behavior | Autonomous headless continuation requires input to remain open on this host |
| Live API, previous environment alias | HTTP 401 `invalid x-api-key`; reported cost $0 | No model inference or semantic recovery established |
| Live API, newly minted key sourced from operator-updated `.bashrc` | HTTP 400: specified API usage limits reached until 2026-12-01 00:00 UTC; zero model tokens and reported cost $0 | Credential change removed the invalid-key response; live semantic recovery remains unverified |
| Live API, operator-requested retry after sourcing `.bashrc` again | Nonce recovered and exact expected `RESULT.json` produced in the new session | One bounded autonomous semantic recovery succeeded |

The open-input and interactive cases used a **scripted model**, intercepting
every `turn.step`; native tool execution, `/clear`, lifecycle events, store and
prompt submission remained real. In each recovery turn the old
`PRE_CLEAR_ONLY_MARKER` was absent, the predecessor recovery-topic pointer and
Wind Walker instruction were present, and status recorded `reset: confirmed`,
`continuation: submitted`, `persistence: recorded`. This is three observed
cycles across two processes, including two cycles in one process, rather than
three independent repeated trials. Native `/clear` emitted `session.end`
with reason `clear`, changed the provider session ID and did not rerun
`session.start`.

The live attempt used **Sonnet 5.5, medium effort**, a $2 CLI cap and 30-turn
limit, with the production plugin only. It kept streaming input open in an
isolated repository. The initial prompt supplied a unique nonce and asked the
model to preserve it through a board note and continuation, call the tool, then
recover and write `RESULT.json` after reset. The first two API failures occurred
before that work, creating no result file. The controller stopped and collected
the idle second CLI after its terminal API error (native exit 143; harness exit
0). Credentials were confined to the child environment and were never recorded.

The operator then requested another attempt. It succeeded in **49.39 seconds**:
source `86dcecbd-8432-4089-99c0-e3a1cbc7dbde` became
`8c2a091d-efd6-4fc0-b1c2-7230decdb5a5`, with observed model
`claude-sonnet-5-5`. The new transcript starts with native clear and the plugin's
recovery prompt, contains no original task message or access to the old
transcript, and receives no nonce from the adapter. Claude read the durable
continuation and predecessor board note, then wrote the exact expected nonce,
objective and completion fields. Control state recorded reset confirmed and
continuation submitted. The process and harness both exited 0 after the final
recovery result and stdin closure. CLI-reported costs were $0.0825868 before
clear and $0.0517542 after clear, totaling **$0.134341** across the two native
session IDs. The native reset starts a new cost counter; the post-clear value
alone understates this fixture's cost.

This was one bounded semantic fixture. Claude read `CONTINUATION.md` before
reading the board pointer, then reconciled both; no exact recovery-order or full
doctrine-compliance claim is made. Pressure delivery, review retention and
process custody remain outside this fixture's qualification.

## Failure and ownership boundaries

The tool acknowledges `checkpoint: agent_owned_unverified`; it cannot establish
that task meaning has been preserved. Native replacement requires both the
matching clear lifecycle event and a changed provider session ID. The adapter
then submits a recovery prompt bound to the predecessor's existing board note.
Prompt submission and actual semantic recovery remain separate observations.

The installed Mods host skips a plugin's own hooks for API calls that plugin
raises. The adapter therefore fences incoming prompts while its native command
and recovery submission are in progress, instead of relying on its own
`command.run` hook to cancel that command. Before native submission, new input
cancels the request; afterwards it receives an explicit retry message. Direct
host-failure tests cover native promise rejection because the test kit's event
answer stub does not reproduce that failure.

Main-conversation reset includes separately launched role processes. Native
`Agent` subagent requests are refused because `/clear` would target the parent.
No reset, lower pressure reading or changed provider ID transfers authority,
settles process collection, or proves semantic recovery. Production pressure
delivery and retained review/child custody were outside these fixture cases.

## Review and provenance

A bounded same-author instruction review inspected the six changed guidance
files, plugin README and registered tool/recovery instructions against the
core, instruction-design, evidence and context-lifetime contracts. Subject
manifest SHA-256:
`bb21af8fe113a3cdee501d344d522313615e7b339f9e0e298778476b2b779527`.
No material instruction finding remained in that subject. Exact host commands
(end the requesting turn, publish the predecessor topic and exclude native
subagents) have exposed causal reasons at their loading boundary. This is
advisory author review, with no independence or general correctness claim.

Graph coverage reported unresolved dynamic Mods/test-kit calls; the complete
plugin, test and driver source was read directly, including flagged call sites,
and behavior was checked on the installed host. No graph-completeness claim is
used. An optional earlier Claude reconnaissance call failed on exhausted
subscription quota, so direct source, official API documentation and local
host tests supplied the evidence. No provider review occurred.

Full local artifacts, failed attempts, immutable review subject and original
baseline copies are retained under
`/home/bline/.local/state/work-engine/claude-new-context-20261115/`.
