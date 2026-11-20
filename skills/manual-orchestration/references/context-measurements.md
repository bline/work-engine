# Historical context measurements

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: interpreting H2 recommendations, changing telemetry thresholds or evaluating the registered pilot.

## Measured basis

Codex compacts a session automatically when its input nears a limit
([probe](../../../metrics/probes/compaction-points-20261105.md), 2026-10-26 to
11-05). With no configured limit, that point sat near 94% of the recorded
window: **Sol 6.1 437,522-456,118 of 475,000 (n = 73: 69 in 17 threads, plus
4 in 2 older roots after the change, overlap not established); Astra
239,130-245,418 of 258,400 (n = 10)**.

The probe dates `model_auto_compact_token_limit = 250000` in
`~/.codex/config.toml` to 2026-11-05T04:58Z from the file's modification time;
the file is off-repository and mutable, and no digest of it is recorded. Codex
applies the limit only to sessions started after that change. One such session
compacted at 248,987 and restarted at 30,980 (n = 1); two older roots kept
compacting near 449K (n = 2). A spawned
child takes its parent's limit: a builder launched by an older root reached
338,682 without compacting (n = 1). A session's compaction point therefore
depends on when it, or its root, started, not on its window.

Under the limit, only that one Sol 6.1 session has been observed compacting
(n = 1); Astra has been observed only without a configured limit (n = 10), so
its compaction point under the limit is unknown. The levels are tokens. The
**band floor 235,000** sits 4,130 below the lowest automatic compaction
observed (Astra, 239,130) and 13,987 below the one post-limit Sol compaction.
The **trigger 200,000** is a chosen margin of 35,000 below the floor, not
measured. They are
**hypothesis H2, not a product rule or a calibrated optimum.** Their value is
that they can be refuted, where an undefined "when warranted" produced no
handoff decisions in the 2026-10-21 pilot.

H2 replaces H1: 150,000 and 171,267 of 258,400, or 58% and 66% of the window.
H1's band was measured on the 2026-10-21 models
([derivation](../../../metrics/probes/compaction-band-20261021.json)) and may
include requested compactions. On a 475,000 window its shares put both warnings
above 250K, so neither arrived before an automatic compaction. For a session
started before the limit, H2 warns early rather than late. If the configured
limit changes, restate both levels with the probe's script.

A context restarted after an automatic compaction measured about 31K on
2026-11-05 (n = 1). One opened after a replacement measured about 25K on
2026-10-16 and settled near 116K. That a *fresh launch* opens similarly is
`unverified`, though its standing floor - system prompt, tool schemas,
repository instructions - is the same.


## Status

H2 (2026-11-05) replaces H1 for Codex sessions; Claude transcripts keep the
levels in the [anthropic host](anthropic-host.md#context-check) reference.
Both change the instruction the 2026-10-21 pilot registered. Testing either
needs a new cohort, opened by a measurement-role amendment to that
[pre-registration](../../../metrics/probes/handoff-boundary-pilot-prereg-20261021.md)
before the first decision; it cannot be scored against the closed cohort. The
[diagnosis](../../../metrics/probes/handoff-boundary-why-no-handoffs-20261021.md)
records why the earlier block produced no handoffs.
