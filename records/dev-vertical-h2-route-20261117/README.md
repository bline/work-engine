# Dev vertical: Pi-route capabilities for H2, qualification record

Standing: qualification record for the slices of the
[H2 Pi-route plan](../../planning/dev-vertical-h2-pi-route-20261117.md). It is
not owner acceptance. Nothing was pushed or deployed by this work.

The harness is a copy of the closed milestone's
([qualify.py](../dev-vertical-hosted-qualification-20261117/qualify.py),
[check.py](../dev-vertical-hosted-qualification-20261117/check.py)), with
`check.py` extended by the `long-answer` scenario.
[matrix-s1.sh](matrix-s1.sh) runs the r8 scenarios plus `long-answer`, copying
the binaries it uses into the run set's `bin/`.

## Slices 1 and 2: response ceiling (D4) and incremental output (`da0a54f8`)

Changes: the receiver no longer forwards `message_update` events on the live
bridge, and the live wire's protocol digest names that
(`raw-agent-events-except-message-update`; the v4 grammar is unchanged). The
Host reads the inference container's stdout from the offset just past the
last applied frame instead of re-reading and re-splitting the whole file every
20 ms. New observer scenario `long-answer`: the second served response is one
text answer of 3 000 deltas (6 000 characters), sent without pacing.

**Baseline** (closed-milestone binary `cb73a44a…`, the r8 and `c3` binary;
`~/.local/state/work-engine/dev-vertical-h2-route-20261117/baseline`):
`long-answer` fails closed. The container's stdout reached 8 390 666 bytes,
"process output exceeded bound", request 2 Uncertain ("run loop ended with the
stream open"). This confirms open item D4 as measured.

**Run set s12** on `da0a54f8` (`…/dev-vertical-h2-route-20261117/s12`,
binaries `coding-qualification` `d131afe8…`, observer `aba0eef0…`): all 20
scenarios pass ([matrix-s12.json](matrix-s12.json), the output of
`check.py <s12> da0a54f8`). `long-answer` settles all four requests; request 2
completed with all 3 000 deltas in its retained `stream.sse`; the container
forwarded no `message_update`, and its `message_end` carries the whole answer.
The job's container stdout was 1 214 096 bytes.

Unit tests: `work-engine-manual-host` (new
`incremental_output_applies_each_complete_frame_once`), `lifecycle-store` and
`pi-pressure-context-host` pass, except the known inherited timing failure
`tests/lifecycle.rs` `mutation_during_snapshot_collection_is_a_conflict`
(recorded in the closed milestone's record).

What remains of the ceiling: frames. Each container frame is bounded at
64 KiB, and the history frame carries the conversation twice, so a coding
turn's answer is now limited by the frame bound (several thousand tokens at
the coding profile's 24 000-byte context), not by the job's output bound.
The analysis profile (slice 3) raises the frame bound.

## Slice 3: hosted analysis profile (`1051e660`)

Changes: the analysis profile `work-engine.pi-analysis-hosted.v1` (Host-served
read-only snapshot tools, no shell, tool container or validator), its
lifecycle grammar and gate, the `analysis-qualification` producer, request
body retention for both hosted profiles, the tools policy over Codex
`additional_tools` items, and relay coalescing (see the
[plan](../../planning/dev-vertical-h2-pi-route-20261117.md), revision 2).

Two findings during development changed the design:

- **Tools are not in the top-level `tools` array.** Pi 1.5 declares the Codex
  tools as an `additional_tools` input item. The coding profile's body policy
  ("only `type: function` tools") had checked only the top-level array, so it
  never saw the declared tools. Both profiles now check every declared tool.
  The analysis profile also pins the canonical digest of the declared tool
  specifications (`TOOLS_SHA256`, taken from the first offline run's
  refusal message).
- **Relay throughput.** A response streamed as one HTTP chunk per event (as
  real backends do) was relayed as one chunk per event, each costing a
  container round trip and a durable append: 8 000 deltas took about 260 s.
  The upstream reader now appends bytes it has already received, up to
  16 KiB, without ever blocking (`Upstream::next_buffered`); the relayed bytes
  are unchanged. The same answer then took 38 s in a debug build.

Harness: [analyze.py](analyze.py) prepares and runs one analysis job (offline
observer `examples/pi_hosted_analysis_observer.rs`, or `live`);
[h2_snapshot.py](h2_snapshot.py) materializes an arm of the draft S2 H2
snapshot from git and checks every member's size and SHA-256 against the
draft manifest (`0fe767fd…`); [check_analysis.py](check_analysis.py) checks a
run set; [matrix-a.sh](matrix-a.sh) runs one.

**Run set a3** on `1051e660`
(`~/.local/state/work-engine/dev-vertical-h2-route-20261117/a3`,
`analysis-qualification` `21de2d53…`, observer `67d69996…`): all four runs
pass ([matrix-a3.json](matrix-a3.json)). Every run: exit 0, Host receipt
passed, analysis turn `CompletedSettled`; request 1 refused on the primary
and completed on the secondary; each request's body retained create-only,
every attempt's `request_body_sha256` equal to it and to the body the
synthetic provider received; every Host delivery retained and matched
natively; gate log equal to the attempt records; no container left, no access
token in any file, `private-access` empty; the inference container had
network `none`, no `/auth` and no snapshot or `/workspace` mount.

| Run | Requests | Deliveries | Notes |
| --- | --- | --- | --- |
| `ok` | 3 | 2 | The model's `workspace_exec` call was refused by the SDK ("not found") and never reached the Host. |
| `long-answer` | 3 | 2 | A 16 000-character answer in 8 000 deltas settled. |
| `read-all-treatment` | 6 | 16 | The draft H2 treatment arm (12 members, 158 961 bytes): every byte delivered; largest request context 191 527 bytes. |
| `read-all-baseline` | 4 | 6 | The baseline arm (5 members, 45 812 bytes): every byte delivered; largest context 57 184 bytes. |

**Run set s3** (coding regression) on `1051e660`: all 20 scenarios pass
([matrix-s3.json](matrix-s3.json), `coding-qualification` `29b98861…`),
including the relay fault and tail-rule scenarios (`trailing`,
`trailing-data`, `echo-trailing`, `late-close-long`, `crash`, cancels) under
coalescing. Scenario times fell (for example `long-answer` 90 s in s12, 12 s
here).

Not covered offline for the analysis profile: crash and cancel (the transport
is shared with the coding profile, whose matrix covers them), and the
tools-digest refusal end to end (unit-tested).

## Live analysis run `live-1` (`1051e660`)

Real routes (primary `https:chatgpt-codex` on the ChatGPT subscription,
secondary `https:openai-responses` on the API key), Host-held access,
`gpt-6-luna`, the draft H2 **treatment** snapshot (12 members, 158 961 bytes),
and a qualification task that asks nothing about H2
([live-task-1.txt](live-task-1.txt): name each member and say in one
sentence what it is about, reading only first pages). It is not an H2 arm and
has no research standing. Run directory
`~/.local/state/work-engine/dev-vertical-h2-route-20261117/live/live-1`,
binary `21de2d53…` (the a3 binary).

Result: exit 0, receipt passed, analysis turn `CompletedSettled`, 25 s. Three
requests, all completed on the primary route (subscription-funded; settled
188, 589 and 10 252 µUSD at catalog rates): the list, then twelve
`snapshot_read` calls in one response, then the answer. 13 Host deliveries,
all matched natively; no SDK refusals. Each request body retained (3 170,
5 347 and 152 686 bytes) with the matching `request_body_sha256`. The answer
names all twelve members with a plausible one-line description each.
`live_check.py` (the closed milestone's) scanned 93 run files: no
subscription access or refresh token and no API key. Inference container:
network `none`, mounts `/pi`, the receiver, `/export`, the stream adapter,
`/config` and `/profile` only. The live observation log gained no entries
(no refusal, uncertain attempt or failed job).

Observed limits that matter for H2: the Host admits a job deadline of at most
300 s (the fresh-fixture deadline), so a slower model or many more requests
would need that bound revisited; and the subscription route served this run,
so on this evidence it is not exhausted for the Pi path (the route
comparison had inferred it might share the exhausted Codex quota).

## Review and remediation (`22487ec6`)

[Fable review](../../reviews/dev-vertical-h2-slices123-fable-20261117/REVIEW.md)
of slices 1–3 (Claude subscription quota): OK with notes, no P0 or P1. An
earlier Opus review of slices 1–2 alone stopped at the subscription's 5-hour
limit and was replaced by this one.

| Finding | Disposition |
| --- | --- |
| P2-1 `tool_request` reachable under the analysis profile (refused only by a Docker inspect failure) | Fixed: refused by policy before any intent is journaled (unit test `analysis_profile_refuses_workspace_tool_requests_by_policy`). |
| P3-a stream binding named the coding profile for analysis streams | Fixed: the binding names the job's profile. |
| P3-b request bounds disagree (Host 32, gate 64, grammar 64) | Gate now uses the Host bound (32). The store grammar's 64 is a looser structural bound by design; the Host plan governs. |
| P3-c gate did not validate its provider policy | Fixed. |
| P3-d answer bound only at settlement | Fixed: the Host refuses a final answer over 64 KiB when its `message_end` is observed (unit test). |
| P3-e an oversize encoded page failed the job | Fixed: answered to the model as an error ("request fewer bytes"). |
| P3-f `snapshot_list` sizes from file metadata | Kept: `snapshot_read` verifies every member's digest; sizes are informational. |
| P3-g `TOOLS_SHA256` derivation | Inputs: receiver `pi-coding.mjs` SHA-256 `3ec041c7…` (unchanged since `1051e660`), Pi SDK index pin `PI_SDK_INDEX_SHA256` (`lifecycle-store`), canonical `serde_json` serialization of all declared tools; the value was read from the first offline run's refusal message, and every later request body (for example a3 `ok/state/coding-hosted-1.body.json`) reproduces it. |
| P3-h, P3-i | Notes; no change. |
| P3-j coding coverage of the tightened tools policy | Closed by run sets s3 and s4. |

Tests on `22487ec6`: `lifecycle-store`, `work-engine-manual-host`,
`work-engine-server` and `pi-pressure-context-host` pass, except the known
inherited `tests/lifecycle.rs` timing failure. One integration test,
`tests/pi_stream.rs` `split_original_a_is_never_forwarded`, failed: its
upstream was a `Cursor`, so both chunks were already received and the new
coalescing merged them; the screen then saw the secret whole and forwarded
nothing (the safe direction) instead of the expected prefix. The test now
delivers the chunks in separate reads, as its name intends, and a new test
`coalesced_original_a_is_never_forwarded` covers the merged case (test-only
change after `22487ec6`).

**Run set a4** on `22487ec6` (`analysis-qualification` `150384ad…`): 4 of 4
pass ([matrix-a4.json](matrix-a4.json)). **Run set s4** (coding,
`coding-qualification` `a0cdd111…`): 20 of 20 pass
([matrix-s4.json](matrix-s4.json)) after one check correction: in `echo-body`
the echoed event arrived in the same relayed chunk as the events before it,
so the screen tripped before anything was forwarded (`forwardedBytes` 0,
434 held bytes dropped, `hosted_access_exclusion`). The old check required
some bytes to be forwarded first, which held only when each provider event
was relayed separately. The check now accepts either order of events and
still requires the exclusion interruption, an Uncertain attempt with its
reservation retained, and no access token in any run file. s3 passes under
the corrected check as well.

## W1-W4: Anthropic Messages family for the `claude-sdk` provider (`c4a8ecfc`)

Plan: [Pi Host realization](../../planning/dev-vertical-pi-host-realization-20261117.md)
W1-W4 (operator authorization 2026-11-18, model `claude-haiku-5-5`). Standing:
qualification record; not owner acceptance, not an H2 arm.

Changes (`work-engine-manual-host`, its receiver, harness):

- **Plan and admission.** `ProviderPlan.provider` (`openai-codex` by default,
  or `claude-sdk`), bound by the runtime admission. `claude-sdk` only on the
  analysis profile, model `claude-haiku-5-5`, priced from Pi's
  `anthropic.json`. Full reservation 945 001 µUSD per request (1M context,
  128k output, upper tier); Messages analysis budget ceiling 32 000 000 µUSD.
  Pi's own `anthropic` provider is not offered yet. Its client does accept a
  fetch override (`pi-ai` `api/anthropic-messages.js` passes `options.fetch`
  to the Anthropic SDK), so the receiver's existing fetch interception could
  serve it; it was left out because its API-key route bills API credits and
  its OAuth route would make Pi a third-party client (correction: an earlier
  revision of this record said the client takes no fetch override, which was
  checked in the wrong file). A `claude-sdk` plan may instead route to the
  API-key endpoint, because both endpoints receive the same body.
- **Endpoints.** `https:anthropic-oauth` (Claude Code's credentials file,
  read again before every connection; every token seen is screened) and
  `https:anthropic-api` (`x-api-key`). Loopback routes serve either family.
- **Body policy.** Strict JSON (duplicate keys refuse), top-level key
  allowlist (no `mcp_servers`, `container`, `service_tier`), `tool_choice`
  auto only, custom tools only, the analysis tool surface pinned by digest
  (`CLAUDE_SDK_TOOLS_SHA256`, from the first offline run's refusal message),
  and every request must extend the previous request's `messages` under the
  two canonical rules. That rule is copied from `native-realization`
  (branch `claude-realization-20261117`); it must become one implementation
  when that crate reaches the base.
- **Relay and settlement.** `message_stop` terminal under the same tail rule;
  usage from `message_start` and the last `message_delta`; settlement per
  token class at the catalog tier of all input tokens; the raw stop reason
  must agree with the container's.
- **Subscription billing guard.** On `https:anthropic-oauth`, a 200 head is
  relayed only with `anthropic-ratelimit-unified-status` `allowed` or
  `allowed_warning` and `...-overage-status` `rejected` or absent; anything
  else closes the attempt as uncertain and fails the job. Kept rate-limit,
  overage and `request-id` headers are retained in `relay.json`
  (`headHeaders`).
- **Receiver (W2).** Loads the pinned extension; a loopback forwarder is
  Claude Code's `ANTHROPIC_BASE_URL`, and each request becomes one
  `provider_fetch` under the current grant. At most one request per grant;
  anything outside an open grant is answered locally. A dummy credential
  sits in the tmpfs `HOME`. Completion is reported only after the relay has
  settled (Claude Code ends its turn at `message_stop`, before the stream's
  end). The process exits after close.
- **Retries (W3).** Claude Code's retries and fallbacks are off
  (`CLAUDE_CODE_MAX_RETRIES=0`, `..._DISABLE_NONSTREAMING_FALLBACK`,
  `..._NO_MODEL_FALLBACK`, `..._DISABLE_MODEL_ACCESS_FALLBACK`,
  `..._DISABLE_REFUSAL_FALLBACK`, `..._DISABLE_REFUSAL_RETRY`; switches
  present in the bundled binary). The forwarder refuses a second request per
  grant, and the Host refuses `x-stainless-retry-count` other than `0`.
- **Harness (W4).** [claude_runtime.py](claude_runtime.py) builds the runtime
  root: Pi's pinned runtime plus the extension (0.2.4), the Agent SDK, its
  linux-x64 `claude` binary and the MCP SDK; 19 014 entries, manifest
  `52ace834…`. `examples/pi_hosted_messages_observer.rs` is the offline
  Messages upstream; `analyze.py` runs it with `ANALYSIS_PROVIDER=claude-sdk`.

Unit tests: `work-engine-manual-host` passes (94 lib tests, 8 new Messages and
guard tests), except the known inherited timing failure in `tests/lifecycle.rs`
(`mutation_during_snapshot_collection_is_a_conflict`), which passed in two of
three runs. The receiver's node tests pass with `--experimental-vm-modules`
(their module sandbox gained `node:http` and `mkdirSync`).

Found during development and fixed before the commit: the receiver kept the
container alive after `complete` (forwarder listening); and `provider_complete`
raced the relay's end (Claude Code reports the turn before the forwarder
pulled `response-end`), which the Host refuses.

**Offline run set w4a** (`~/.local/state/work-engine/dev-vertical-h2-route-20261117/w4a`,
`analysis-qualification` `555ac071…`, observer `f34e1cc5…`), Pi Host container
with the real Claude Code against the synthetic upstream:

| Run | Result |
| --- | --- |
| `ok` | exit 0, 10 s, `CompletedSettled`. Request 1 refused (429) on the primary, then dispatched with the same retained body on the secondary; three requests, each settled at 450 µUSD (40 + 3 000 cache-write + 1 000 cache-read input, 120 output at base-tier rates); two Host deliveries; retry count 0 on every request; no container left. |
| `retry-head` | The secondary answers 500 (not a definitive refusal): attempt `Uncertain`, reservation retained, job failed closed; no further request reached upstream. |

**Live run `live-c1`** (`…/live/live-c1`, same binary): route
`https:anthropic-oauth` only (no API-key failover), the draft H2 treatment
snapshot (12 members, 158 961 bytes) and the qualification task
[live-task-1.txt](live-task-1.txt). This is not an H2 arm. Inference and
transport worked. Three requests, all HTTP 200 on the subscription, each head
`unified-status: allowed` and `overage-status: rejected`. They settled at
227, 603 and 7 960 µUSD at catalog rates (request 3: 58 868 cache-write input
tokens). There were 12 `snapshot_read` calls in one response, every Host
delivery was made, and the answer was produced. A scan of 91 run files found
neither the subscription access token nor the refresh token;
`private-access` is empty.

**The job failed at the lifecycle gate**: "entry claim conflicted with durable
state". The model's assistant messages carry `thinking` blocks (requests 2
and 3), and the analysis grammar (`lifecycle-store`
`pi_analysis_proof.rs`) accepts only `text` and `toolCall` content. The
receiver pins `thinkingLevel: off`, but Pi's catalog has no `off` level for
`claude-haiku-5-5` and marks the model `forceAdaptiveThinking`, so Pi sends
adaptive thinking at effort `low`. As far as these catalogs show, thinking
cannot be turned off for this model. Running Claude on this profile, for
qualification or H2, therefore needs the analysis grammar to accept
`thinking` blocks. That is a `lifecycle-store` change for the
research-lifecycle owner, not part of W1-W4. The offline runs pass because
the synthetic upstream sends no thinking.
