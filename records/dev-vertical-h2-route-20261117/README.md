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
