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
