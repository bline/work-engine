# Scoring the Claude builder arm: campaign R2 (read-set instruction change)

Scored 2026-10-24 by the measurement role (`e702a3e4`), which is excluded from
both arms of the prospective correspondence trial and built nothing in R2.
Registration: [`claude-builder-r2-prereg-20261022.md`](claude-builder-r2-prereg-20261022.md),
amendments 1-6. Run record
`~/.local/state/work-engine/r2-pressure-controller-20261022/assignments.json`.
Campaign ended 2026-10-23T15:50:38Z (board 2642) on the user-agreed condition:
P1 integrated, no R2 builder or remediation worker running.

## The result in one line

**The treatment did not do what it says.** Context at first code write rose 6%
and pre-write reading rose with it, so neither clause of "read less, commit
after each independently passing unit" shows in the data. What did change is
the failure mode the text was written against: **no R2 builder spent a whole
context without writing code**, where one of seven did in R1. Cost per 1k Rust
lines fell sharply, but for reasons R2 does not isolate.

| | prediction | observed | n | verdict |
| --- | --- | --- | --- | --- |
| **H1** context at first `.rs` write falls below 102,049 | fall | median **108,214** | 9 vs 6 | **disconfirmed** |
| **H2** no builder writes no code | 0 of n | **0 of 9** | 9 | **supported** |
| **H3** handoffs per Rust slice below 1.5 | fall | **2.0** | 3 slices | **disconfirmed** (see defect 2) |
| **H4** code-slice cost below 1.517 Mwt/krs | fall | **0.67 / 0.86 / 1.15** | 3 slices | **supported** |
| **H5** orchestration share below 36.2% | no effect expected | **31.1%-37.0%** | 1 campaign | **not established** |

## H1 and the reading claim

The prereg's disambiguation rule says a fall in H1 with pre-write read calls
unchanged is earlier committing rather than leaner reading, and that only a fall
in both supports the reading claim. **Neither fell**, so the rule does not even
engage.

| slice | builder | context at first `.rs` write | pre-write read calls |
| --- | --- | --- | --- |
| E1 | `a74f07e00db29f8e0` | 107,877 | 20 |
| E1 | `a68d60a23bab394e2` | 93,645 | 18 |
| E2 | `a4753525221f23204` | 128,738 | 24 |
| E2 | `aa3038194d5ecde25` | 108,214 | 16 |
| E2 | `a69f21553464cbc6e` | 105,596 | 17 |
| P1 | `a8b36f8b30274b385` | 135,584 | 32 |
| P1 | `ac604a3dcfdd0db66` | 117,615 | 23 |
| P1 | `a131e8667687eea02` | 123,231 | 17 |
| P1 | `aa725a9ba777a9a84` | 91,659 | 17 |

**B1**: R2 median 108,214, mean 112,462, range 91,659-135,584, n=9, against the
pre-change median 102,049, n=6. **+6,165, +6.0%.**

**B6**: R2 median 18, mean 20.4, range 16-32, against **14.0** over the same six
pre-change builders *re-counted with this record's own read-class counter*. The
registered B6 (median 15.5) is not used here; see defect 4.

Every value is the first `.rs` write whose `tool_result` is not an error, found
by listing all `.rs`-touching write candidates per transcript and **reading the
matched command** — the discipline amendment 3 imposed after a substring
detector produced two wrong baseline values. The detector was validated by
re-deriving all six pre-change values from the R1 transcripts before it was
pointed at R2; five reproduce amendment 3 exactly and the sixth is defect 1
below.

## H2: the failure mode the treatment targets

R1: `a0e59f73949c117b9` read to 145,858 and declared a handoff having written
nothing — **1 of 7, 14.3%**. R2: **0 of 9**. Every code-slice builder wrote
Rust. This is the one hypothesis the data supports in the direction claimed, and
it is the one the treatment text was written for.

It is also the hypothesis with the weakest discriminating power: one instance in
seven against zero in nine is consistent with a treatment effect and with
ordinary variation, and no test here separates them.

## H3: handoffs

Eight outcome-A boundaries, **six builder and two reviewer**, over three
Rust-building slices: E1 one, E2 two, P1 three. The B3-comparable figure counts
builder handoffs only, since R1's four were all builder handoffs: **6 / 3 = 2.0**.

Against the registered B3 of 1.5 this is disconfirmed and worse. Against the
**corrected** B3 of 2.0 (defect 2) it is disconfirmed and **exactly equal**.
Those two readings differ in kind, and the corrected one is the true comparison.

Four of the six builder handoffs were declared **below** the 150,000 trigger
(149,436; 148,809; 141,296; 145,226), against two of four in R1 — workers
continuing to stop at a phase boundary before they are obliged to.

## H4: economics

Gross added Rust lines, counted over each integration range on
`rescue-detached-20260911`; every changed file in all three ranges is `.rs`.

| slice | range | Rust lines | core cohort | Mwt/krs | with remediation | Mwt/krs |
| --- | --- | --- | --- | --- | --- | --- |
| E1 | `89396aea..f2fe643d` | 3,295 | 2.219 Mwt, 3 agents | **0.673** | 2.870 | 0.871 |
| E2 | `ab3acf34..928570a6` | 3,589 | 3.088 Mwt, 5 agents | **0.860** | 4.061 | 1.132 |
| P1 | `c5d8d106..8db88681` | 4,260 | 4.894 Mwt, 7 agents | **1.149** | 5.751 | 1.350 |

Against B4 — S1 1.255, S2 1.778, mean 1.517 — all three slices are below the
mean on both definitions, and all three are below S1 on the core definition.
Mean 0.894 core, 1.118 with remediation.

**This is not attributable to the treatment.** R2 builds a different subject, in
a crate family the campaign itself created, with a contract formed once and
reused across three slices rather than once per campaign. The prereg's
between-campaign confound covers exactly this. The honest statement is that R2
was cheaper per Rust line than R1; why is not established.

Campaign totals: subagents **17.525 Mwt** over 893 calls and 26 agents,
**1.573 Mwt/krs** over 11,144 Rust lines, against R1's 3.611. R1's campaign
figure was inflated by a slice-3 cohort that produced 63 Rust lines; R2 has no
such cohort, which accounts for part of the gap by construction.

## H5: orchestration share

The coordinator session `c1d31e88` also ran fix-c, fix-d, the coordination-channels
work and a board listener inside R2's calendar window, and no per-call
attribution separates them. Two bounds:

- whole R2 window 01:10:45Z-16:10:29Z: 10.299 Mwt over 211 calls, **37.0%**;
- union of the R2 cohort windows, excluding the 06:00-14:08 gap in which the
  non-R2 work ran: 7.906 Mwt over 170 calls, **31.1%**.

The comparator is itself unsettled — the prereg froze B5 at 36.2% and the R1
score artifact it derives from records 33.9% (defect 3). The interval straddles
both. **H5 is `not established`**, which is an acceptable outcome for a
hypothesis registered as secondary with no effect expected.

## Context discipline

**37 context-check records** across 18 distinct agents, all against the same
150,000 trigger. 36 non-zero readings, 47,199-158,847, median 124,294;
33 `retain_ok`, 4 `fresh`; 4 at or over the trigger, **none** at or over the
202,533 band floor.

Every record carries `input_tokens`, `recommendation` and `trigger_tokens`, so
scorability item 4 is met. (The copies in `assignments.json` are bare integer
lists and cover only 6 of 25 assignments; the authoritative records are the 37
JSON files in the campaign state directory.)

**No compaction occurred anywhere in R2.** Completeness basis: the 26 R2 agents
were enumerated from `assignments.json`, all 26 transcripts were located, and
all 26 plus both coordinator session transcripts (`c1d31e88`, `26e8ee40`) were
searched for `compactMetadata` and `compact_boundary`. Zero matches of either,
in any file. All 37 checks independently report `epoch: 0` and
`compaction_markers: 0`.

## Scorability against the registration

| item | status |
| --- | --- |
| 1. read set / added lines / builder count (as amended) | **met** — amendment 6: E2 2,427, P1 2,252, nine code-slice builders |
| 2. sha256 of each reference file actually read | **met** — `reference_sha256` in each worker result; digests unchanged at campaign end |
| 3. read set stated with its line count | **met** — `read_set` per assignment, `read_set_actual` per result |
| 4. context checks at each phase boundary | **met** — 37 records with the three required fields |
| 5. branch, commit range, net added lines per language | **met** — three integration ranges, all-Rust |

## Defects found while scoring

**1. A frozen baseline value came from a command the harness refused.**
Amendment 3 recorded `a27d1ec9c1e4acca4`'s first `.rs` write as 113,522. That
call's `tool_result` is `is_error: true` — the harness refused it as too complex
to verify against the worktree boundary, so no file was written. The first
effective write is 114,292 with 16 pre-write reads. **B1's median is unchanged
at 102,049**; the mean moves 97,249 → 97,377. Immaterial to every comparison
here, and recorded because the next use of that table should not re-inherit it.

Refused-then-retried first writes are an instrument artifact in **both** arms:
one of six pre-change builders and three of nine R2 builders hit it. The gap
between the refused attempt and the effective write is 770 tokens for
`a27d1ec9c1e4acca4`, 1,436 for `a74f07e00db29f8e0`, 1,665 for
`a8b36f8b30274b385` and **6,807** for `a4753525221f23204`, whose retry came
after further reading. The artifact is small except where the retry is not
immediate.

**2. B3 was frozen at the wrong value.** The prereg records "B3 handoffs per
Rust-building slice: 1.5 (3 handoffs over S1 and S2)". The R1 score artifact it
derives from records **four** outcome-A instances — `s1`, `s2_a`, `s2_a2`,
`s2_b2` — all four inside S1 and S2, with three further `s3/` files excluded by
name. The correct pre-change figure is **4 / 2 = 2.0**. This changes H3 from
"disconfirmed and worse" to "disconfirmed and exactly equal", so it is material
to the conclusion and not only to the number.

**3. B5 disagrees with its own source.** The prereg froze 36.2%; the R1 score
artifact records `orchestration_share: 0.339` and its prose says 33.9%. The 2.3-point
gap is not reconciled here. A plausible cause is the pre-correction pass of
`claude_transcript_usage.py`, which deduplicated on `(message.id, timestamp)`
and overcounted, but that is a hypothesis, not a finding. H5 is reported against
both.

**4. B6 is instrument-dependent and the instrument was never defined.**
"Read-class tool calls" appears in the registration with no enumeration of which
calls are read-class. This record counts `Read`, `Grep`, `Glob`, `NotebookRead`,
`WebFetch`, `WebSearch`, and any `Bash` command beginning a clause with `cat`,
`head`, `tail`, `sed -n`, `less`, `grep`, `rg`, `find`, `ls`, `wc`, `awk`,
`git show/log/diff`, `sha256sum` or `stat`. Under that counter the pre-change
median is **14.0**, not the registered 15.5. The R2 figure is therefore compared
only against the re-derived baseline, never against the registered one. Any
future B6 comparison must re-derive both sides with one counter.

**5. E2's added-line count differs by 9 from amendment 6.** Amendment 6 recorded
3,598 from `git diff --numstat f2fe643d..HEAD` at the time of writing; the
integration range `ab3acf34..928570a6` gives 3,589. The criterion is >= 1,000 and
both clear it. The integration range is used here because scorability item 5
names the commit range.

## Limits

**n = 3 slices, 1 independent chain.** Carried forward from amendment 6: E2's
read set is 69% E1's output and P1's is 31% E1+E2's, so eligibility is
endogenous to the campaign and the three slices are not three independent draws
for H1-H3. Predecessor Rust may be cheaper to read than unfamiliar repository
code; this is stated, not corrected for.

**Between-campaign comparison.** R2's subject, crate family and contract
structure all differ from R1's. H4 in particular compares two different build
problems.

**Reviewers are inside the orchestration being measured.** Unchanged from R1.

**The treatment is one of several differences.** Amendments 1 and 5 record
`c56ab8ec` and `skills/coordination-channels` as covariates, not treatment
changes. Neither is controlled.

**Window fraction is not established.** Claude Code transcripts record no
context window, so every reading here is absolute, as in R1.

## Method

Per-transcript accounting deduplicates assistant records by `message.id` and
takes the maximum `output_tokens`, as `metrics/scripts/claude_transcript_usage.py`
establishes; the first pass of this scoring did not, and inflated every cost
figure by roughly 2x before it was corrected against that script. Weighted
tokens are `uncached + cached/10 + 5 x output`. No Codex quota is consumed by
this arm and no plus-unit claim is made.

## Amendment 1, 2026-10-24: one of the 37 context checks is not a measurement

Prompted by the outside audit
[`context-control-usage-20261024.md`](context-control-usage-20261024.md)
(Astra, uncommitted at the time of writing), finding CU-01: after a compaction
reset, zeroed accounting fields were read as a valid `normal, input_tokens: 0`
observation. The same failure is present in this campaign's own data, from a
different cause, and the audit's recommended repair would not catch it.

**The instance.** `p1-portfolio/context-check-tests-predispatch.json`, agent
`ab6975a352dac866b`, checked at 15:36:46Z and observing 15:18:38Z, reports
`input_tokens: 0`, `recorded_model: "<synthetic>"`, `recommendation: retain_ok`,
`headroom_to_trigger: 150000`, `headroom_to_band_floor: 202533`. It is the one
zero reading among the 37, and the reason given is "input 0 is below the
trigger 150000".

**What it should have read.** That transcript holds exactly one `<synthetic>`
assistant record carrying a zeroed `usage` dict, at record 167 of 192. The
parser in `skills/manual-orchestration/scripts/child_context.py` takes every
`type == "assistant"` record with a `usage` dict and keeps the last one seen, so
at 15:18:38Z — when record 167 was the tail — the zeroed placeholder became the
reading. The last real call in that transcript, record 190, carries
`2 + 3,194 + 96,785`, about **99,981 tokens**. The tool reported 0 against a true
level near 100,000, in the reassuring direction, claiming the entire empirical
band as headroom.

**Why the guard missed it.** The tool already states the correct rule on its
other failure path: when `context-status` fails it returns
`recommendation: usage_unavailable` with the reason *"do not substitute parent
counters or assume zero"*. The placeholder path never reaches it, because the
admission test is `if not isinstance(current, int)` and **zero is an int**. A
type check stands where a validity check was needed. The sibling instrument in
this repo, `metrics/scripts/claude_transcript_usage.py`, excludes
`model == "<synthetic>"` by name at line 46. Two readers of the same transcripts
disagree about what counts as an API call, and the lax one is the one that
issues the safety recommendation.

**Scope of the repair, against the audit's.** CU-01 recommends recognising
"reset/accounting placeholders". That is scoped to the compaction-reset shape
and would leave this case open, since a `<synthetic>` record is not a reset
placeholder. The defect both instances share is one level up: **the reader
renders the absence of a measurement as a measurement of zero**, which is what
`CLAUDE.md` rule 2 forbids by name. A repair at that level — no usable request
record means `usage_unavailable`, never `retain_ok` — covers the reset shape,
the `<synthetic>` shape and whatever shape is next. Excluding `<synthetic>` as
the sibling script already does is the narrow half of it.

**Incidence and effect on this score.** 1 of 37 in R2; **0 of 47** in R1, whose
records were re-checked for the same shape while writing this. 1 of 84
Claude-side checks across both campaigns. No figure in this record moves: the
reading was already excluded as the single zero, and the context-discipline
section reports 36 non-zero readings. What changes is the scorability line —
item 4 counted 37 records as 37 measurements, and one of them is not one. **The
item remains met at 36**, since 36 genuine checks across 18 agents still cover
every phase boundary the registration names.

**One claim re-verified rather than assumed.** The audit reports a compaction
near 245K and two in the B2 Codex roots. Those are a different host with a
different window and do not bear on the Claude band, but the band floor is load
bearing in this record, so it was re-derived: **25 `compactMetadata` events now
under `~/.claude/projects`, minimum `preTokens` still 202,533**, with no event
since the original derivation below it. The 150,000 trigger remains 52,533 below
the lowest auto-compaction observed on this machine.

**Not repaired here.** `skills/manual-orchestration` is the instrument under
measurement in the prospective trial, and amendment 2's standing rule governs
changes to it. Editing it from the measurement role would be a treatment change
mid-trial. This is reported, not fixed.

## Amendment 2, 2026-10-24: context never falls within a builder run

Measured because I had claimed, from the four below-trigger handoffs, that they
were "the signature of a bare threshold with no exit band" and that
`context-pressure`'s hysteresis bands would fix them. An outside reading
(Astra) objected that hysteresis depends on pressure falling back across a
boundary, and that the observations establish the behaviour and not its cause.
That objection is correct, and the measurement settles it in the stronger
direction.

Across the **9 code-slice builders and all 305 API calls, context decreased
zero times.** Seven builders crossed 150,000; **none crossed back below it.**

| builder | calls | decreases | 150K up-crossings | back-down |
| --- | --- | --- | --- | --- |
| all nine | 305 | **0** | 7 | **0** |

A hysteresis exit threshold has nothing to act on in a strictly monotonic
context. For this workload it is **inert, not merely unproven**: the enter/exit
distinction can never fire. Hysteresis remains meaningful where context can
fall — a root that compacts, or any thread with context replacement — so this
is a scope result, not a verdict on the crate.

**Consequence for R3.** Trigger *level* and trigger *mechanism* are not
equivalent levers here. Varying flat-threshold against hysteresis on Claude
subagent builders would compare a policy against itself. The live question is
what the four early handoffs actually were — anticipated task growth, a
convenient commit boundary, or conservative judgment — and that is answered by
instrumenting the decision, not by changing the threshold shape.

**The ramp decomposition is published, not asserted.**
`metrics/scripts/builder_ramp.py` reproduces the per-call uncached/cached
sequence and the first-`.rs`-write adjudication for any builder transcript. The
decomposition quoted in discussion (about 15K fixed harness preamble, 3-5K
handoff file, about 11K DESIGN.md, 50-60K contract and source re-read) is
**read call-by-call for 3 of 9 builders and aggregated for 9**; it has not been
independently reproduced and should be treated as a targeting hypothesis until
it is.
