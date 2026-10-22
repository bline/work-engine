# Result: an Anthropic agent as builder, n=1

Status: result, 2026-10-21T23:35Z, against
[the pre-registration](anthropic-builder-arm-prereg-20261021.md) committed at
`4c0cb123` **before any source line was written**. Recovered figures are frozen
in [`anthropic-builder-arm-cost-20261021.json`](anthropic-builder-arm-cost-20261021.json).

Slice commit: `e785aeb7`, `feat(tools): report child context usage from the
agents subcommand`.

**I am the arm here and I do not score it.** What follows is arithmetic,
recovered denominators and condition-by-condition compliance. The verdict on
what it means is the user's or an independent party's.

## Acceptance conditions

| # | condition | outcome |
| --- | --- | --- |
| 1 | flag off, output byte-identical | **met**, 8 of 8 input combinations - two parents x {plain, `--recursive`} x {text, `--json`} - against the pinned production binary `23a0ea4a...`; one stated deviation below |
| 2 | each child row carries its latest usage | **met**: input, cached, output, timestamp, epoch, window, fraction, lineage markers |
| 3 | per-child values equal the single-session path | **met, checked not assumed**: 20 of 20 children under root `01a15f02`, five fields each, zero mismatches |
| 4 | unestablished usage carries a distinguishable reason | **met**: `absent_counters` and `unreadable_rollout` as separate classes, each with a reason, no zeros, and one unreadable child does not cost the parent its other answers |
| 5 | new tests, existing suite still passes | **met**: 30 pass, 0 fail - the 27 existing plus 3 new |
| 6 | `fmt --check` and `clippy -D warnings` clean | **met**, `--all-targets` |
| 7 | `rust/Cargo.toml` and `rust/Cargo.lock` unchanged | **met**: no dependency added, change set is three files in the crate. Both workspace files were already modified by other sessions before this slice began, and `Cargo.lock`'s mtime (16:12 local) precedes this build (17:40 local), so the build did not touch it |

**Stated deviation on condition 1.** `--help` output necessarily changed, in both
the top-level and `agents` usage strings, to document the new flag. Condition 1
holds for report output; an undocumented flag would have been the worse choice.

## Recovered cost

| measure | value |
| --- | --- |
| build segment | **44 model calls, 1.553 Mwt**, 43,424 output tokens |
| pre-registration segment, excluded from build | 11 calls, 0.410 Mwt |
| Rust lines added | **289** (135 source, 156 tests), 2 deleted |
| **build cost per 1,000 Rust lines** | **5.38 Mwt/krs** |
| including the pre-registration | 6.79 Mwt/krs |
| wall clock, marker to slice commit | about 6 minutes |
| baseline range | **2.04 - 5.11 Mwt/krs** (16-40 Plus-units/krs at 7.826 units per Mwt) |

Weighted tokens are the baseline's own unit, `uncached + cached/10 + 5*output`,
with Anthropic `cache_creation` counted as uncached and `cache_read` as cached.
No Plus-unit or currency figure is given: a Plus-unit is Codex weekly quota and
this arm consumed none.

## What these numbers are not

**This is a first-candidate cost against a baseline of accepted-slice costs.**
The baseline's per-slice figures cover iteration to acceptance - mean 2.8
candidate attempts, no slice accepted on its first candidate, about 6 review
findings resolved per slice. This slice has had one candidate and no independent
review. **5.38 Mwt/krs is therefore a lower bound on the comparable figure**, and
it will rise by whatever review and remediation cost. Comparing it to 2.04-5.11
as though the work were finished would be the error to catch.

**M9 and M10 remain `not established`, not zero.** Candidate attempts to
acceptance and review findings resolved both need an independent reviewer and
acceptor. Codex is at quota. Code that passes its own author's gate is not an
accepted slice, and the seven conditions above were written by the same party
that then satisfied them.

**The predicted small-slice penalty landed, and favours caution either way.** The
pre-registration said in advance that a few-hundred-line slice should look worse
per line than 5,236-43,296-line baseline slices, because fixed costs - reading
the crate, its conventions, its test harness - divide by a far smaller
denominator. The slice is 18 to 150 times smaller than the baseline slices and
came out 5% above the top of their range. Both the direction and the modest size
of the gap are consistent with that prediction, which is why the number should
not be read as a capability result in either direction.

**Spec independence was weak.** I identified this gap in the handoff diagnosis
and then built it, so specifier and builder were the same party; baseline slices
had separate planning. This favours the arm.

**Harness, not only model.** Claude Code's tools, caching and context handling
differ from Codex's multi-agent v2. As with the handoff pilot, the comparison
cannot separate the two.

**n=1.**

## One in-session error worth recording

The first cost computation reported a build segment of 409 calls and 10.338 Mwt,
giving 35.77 Mwt/krs. The wrong claim was that figure. The class: the segment
boundary was compared as a **string** between a transcript timestamp in `Z` form
and a git commit time in `-06:00` form, so `17:27` sorted below `23:25` and the
segment swept in records from previous days. The consequence would have been to
report this slice as roughly 7 times more expensive than it was, against a
baseline it would then have appeared to fail badly. The check that caught it was
internal consistency: the build segment exceeded the total since the marker,
which is impossible for a subset, and that assertion is now in the script.

## What would complete this

An independent review of `e785aeb7` against the seven conditions and the crate's
own standards, yielding M9 and M10. Until then the cost figure stands alone,
and the ledger rule applies: a cheaper run that needs more attempts or lets more
defects escape is not an improvement.

## Amendment 1, 2026-10-22: the cost was over-counted about 2x

Written by a different Claude session (coordinator `bf60b4c1`), not the arm.

**The wrong claim:** build segment 44 calls, 1.553 Mwt, 5.38 Mwt/krs, and the
reading that this sits above the baseline range 2.04-5.11.

**Error class:** `metrics/scripts/claude_transcript_usage.py` deduplicated on
`(message.id, timestamp)`. Claude Code writes one API response as several
assistant records sharing `message.id` with different timestamps, so each
response was counted once per record. On this arm's transcript
(`e702a3e4-...jsonl`) the whole-file over-count is 2.17x (4,632 rows counted
against 2,170 distinct ids). Found by the CS-1 builder's A4 reconciliation in
run `readiness-r1-20261021` and confirmed by its tier-1 reviewer across 117
transcripts.

**Corrected figure:** counting each `message.id` once, with the maximum
`output_tokens` across its records, the build segment from the recorded split
`23:27:08Z` gives **19 calls, 0.661 Mwt (2.29 Mwt/krs)** ending at the
`e785aeb7` commit time `23:31:07Z`, or **22 calls, 0.797 Mwt (2.76 Mwt/krs)**
ending at this result's commit `23:32:46Z`. The arm did not record its segment
end, so the exact figure is `not established`; under the old rule the same two
windows give 41 and 50 calls, bracketing the recorded 44.

**Consequence:** the slice's first-candidate cost falls inside the baseline
range rather than above it. The predicted "small-slice penalty" is not
observed in this figure. Every other caveat above still applies (n=1,
first-candidate against accepted-slice baselines, weak spec independence,
harness confound), and M9/M10 remain `not established`.

**The check that would have caught it:** compare rows counted with distinct
`message.id` values. The script now deduplicates on `message.id` with maximum
output, and states the rule in its docstring.
