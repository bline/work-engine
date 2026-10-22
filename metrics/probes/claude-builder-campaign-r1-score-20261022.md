# Scoring the Claude builder arm: campaign readiness-r1-20261021

Status: campaign complete, scored 2026-10-22. Dataset:
[`claude-builder-campaign-r1-score-20261022.json`](claude-builder-campaign-r1-score-20261022.json).
Pre-registration: [`anthropic-builder-arm-prereg-20261021.md`](anthropic-builder-arm-prereg-20261021.md).
Slice 1 alone was recorded earlier in `claude-builder-slice1-20261022.json`; this
record supersedes its headline figure as a campaign estimate, and leaves the
slice-1 numbers themselves unchanged.

One orchestrator session (`bf60b4c1`) ran 25 agents over 3.27 hours and produced
10,203 net added Rust lines on `worktree-agent-a27d1ec9c1e4acca4`
(`6a945b15..8e68518a`, 23 commits, 28 files, all additions). The branch is
**integrated**, user-confirmed, as `12a770bb..24c41cb9` on
`rescue-detached-20260911` (see the amendment at the end).

## What the Codex pilot registered and never saw

The [handoff boundary pilot](handoff-boundary-pilot-prereg-20261021.md)
pre-registered outcome A — a worker declares a context boundary and a successor
continues the work — and observed it zero times in 120 records across roughly 62
thread-hours. The [diagnosis](handoff-boundary-why-no-handoffs-20261021.md)
attributed that to the absence of any context-replacement primitive at any Codex
role: workers hold `exec`, `send_message` and `wait_agent`, supervisors add
`spawn_agent` and `followup_task`, and none of them replaces a context.

**Outcome A occurred four times here**, n = 4:

| boundary | reading at declaration | tool said | turnaround |
| --- | --- | --- | --- |
| S1 builder → successor | 165,624 | `fresh` | 30.5 s |
| S2 part a → a2 | 145,858 | `retain_ok` | 54.5 s |
| S2 part a2 → a2-2 | 159,897 | `fresh` | 58.4 s |
| S2 part b → b2 | 149,478 | `retain_ok` | 27.9 s |

Turnaround is the predecessor's last recorded activity to the successor's first,
27.9–58.4 s, mean 42.8 s. This is the declare-to-acceptance interval that the
pilot's amendment named as the missing measurement, and it is now measured on
this host rather than inferred.

The mechanism is parent-mediated, which is why it works: the predecessor writes a
handoff file and returns `handoff_ready`, and the coordinator launches a
successor whose entry read is that file. The primitive is the parent's `Agent`
tool. **Two of the four declared below the 150,000 trigger**, at a phase
boundary, with the tool still saying `retain_ok` — workers handing off before
they were obliged to.

Three further `handoff*` files under `s3/` are **not** outcome A and are excluded
by name in the dataset: one is a read-set handoff to a downstream assessor, and
two carry headers stating they are not `handoff_ready`. A further 18 matches are
copies of this repo's own pilot probe files sitting inside slice 3's subject
tree, since slice 3 took the metrics work as its subject.

## Context discipline

47 self-checks by 22 distinct agents, all against the same 150,000 trigger.
Readings 45,987–204,161, median 126,183; 39 `retain_ok`, 8 `fresh`.

**No compaction occurred anywhere in the session.** Completeness basis: every
record of the orchestrator transcript and of all 28 subagent transcripts was
searched for `compactMetadata` and `subtype == compact_boundary`; the only
matches are the literal strings inside skill text and inside tool-input source
code, each read and confirmed as such. All 47 checks independently report
`compaction_markers: 0` and `epoch: 0`.

For margin, the auto-compaction band on this machine was re-derived from every
`compactMetadata` record under `~/.claude/projects`: **21 events, `preTokens`
202,533–968,704**, all `trigger: auto`. The campaign's trigger was therefore set
52,533 below the lowest compaction ever observed here. Exactly one of the 47
readings exceeded that floor — 204,161, by `a4675d48b42da1f6b`, which belongs to
the CS-1 slice and not to R1 — and it did not compact.

What fraction of a window these levels represent is **not established**: Claude
Code transcripts record no context window, so every level here is absolute. The
Codex compaction band is denominated in a known window and this one is not, so
the two are comparable as margins, not as percentages.

## Economics

The baseline's per-slice costs exclude orchestration and record it separately at
+25%, so the like-for-like figure is subagents only.

| | weighted | per 1k Rust lines |
| --- | --- | --- |
| R1 subagents (1,635 calls) | 36.847 Mwt | **3.61 Mwt/krs** |
| R1 orchestrator (355 calls) | 18.869 Mwt | |
| R1 total | 55.716 Mwt | 5.46 Mwt/krs |

Restated in the baseline's display unit, 28.3 units-equivalent per 1k Rust
lines. That conversion uses the baseline's fitted gpt-6 price of 7.826 units per
Mwt purely to put an identically-defined weighted-token count on the same axis;
this arm consumes no Codex quota and the figure is not a quota claim.

Against the baseline's own slice rows:

| slice | Rust lines | Mwt/krs |
| --- | --- | --- |
| lifecycle S0–S3 | 11,403 | 3.98 |
| lifecycle S4 | 5,492 | 3.49 |
| lifecycle S5 | 5,236 | 5.17 |
| lifecycle S6/S7 | 14,873 | 3.16 |
| domain libraries | 43,296 | 2.03 |
| **R1 campaign** | **10,203** | **3.61** |

R1 sits inside the lifecycle band, 9% below `lifecycle S0–S3` — the nearest
comparator both by size and by multi-slice shape. It is 78% above the
domain-libraries figure, and a new standalone crate with its own workspace is
arguably that category, so the direction of the comparison depends on which
comparator is accepted. Orchestration overhead was **33.9%** against the
baseline's 25%: worse, not better.

The two code slices are much cheaper than the campaign: **S1 1.26 and S2 1.78
Mwt/krs**, both below even the domain-libraries figure. The campaign average is
higher because contract formation spent 11.311 Mwt — 30.7% of all subagent cost
— before a line of code was written, and because slice 3 produced validation,
profiles and a Python consumer with no Rust to divide by. Reporting a slice
figure as the campaign figure would overstate the result by about 2.9×, which is
the error this record exists to prevent.

## Quality

| | S1 | S2 | S3 | tier-2 final |
| --- | --- | --- | --- | --- |
| reviewer | Opus | Opus | Opus | Fable |
| findings | 4 | 6 | 4 | 5 |
| severities | 4 P3 | 1 P2, 5 P3 | 4 P3 | 2 P2, 3 P3 |
| verdict | ready | ready, conditional | ready | integrate after fixes |

19 findings in all: 0 P1, 3 P2, 16 P3. Tier-1 mean 4.67 per slice against the
baseline's 6.0 over 11 slices with stated counts (range 2–10). The severity mix
has **no baseline counterpart**: the baseline records finding counts without
severities, so "fewer serious findings" is not a claim this comparison supports.

Candidate attempts, on the baseline's `accepted_candidate` semantics: S1 1, S3 1,
S2 2 (tier-1 ready was conditional on the P2 finding S2-T1-001, and S2c was the
fix round). Mean 1.33 over three slices. At campaign level the tier-2 final
returned "integrate after fixes" and S3e closed T2F-001/002/003, so the campaign
itself was accepted on its second candidate. The baseline's figure is a mean of
2.8 with **0 of 14 slices accepted on a first candidate**; two of three here
were. n = 3.

Tier-2 verified the manifest (9/9), re-ran the gates directly on `d1efa078`
(cargo 114 passed, 0 failed; clippy `-D warnings` clean on recompile; Python
unittest 23 OK) and recorded AC1–AC9 met.

## Defects found

- **R1-M-001 (P2)** Two copies of the manual-orchestration skill exist and have
  diverged. The `Skill` tool resolves to `~/.claude/skills/manual-orchestration`;
  worker prompts `cat` the repo copy. `cb61f807` wrote the read-set sizing
  guidance to the repo copy at 01:59:50Z and not to the user-scope copy, which
  still holds `fa163bad`; `worker.md` and `coordinator.md` differ by exactly
  those additions. So cohort membership cannot be read off a commit time alone.
  `afd6ae453ed824ee9` (S3b) read only the user-scope copy after the divergence
  and is the one confirmed instance. S2's prompt pins a required skill commit but
  no worker verifies the text it read against that pin.
- **R1-M-002 (P3)** My own trajectory watcher took idleness from the transcript's
  mtime, which timestamp-less records kept current for ten hours after the last
  conversation record, so its idle exit never fired. No effect on the data.

R1-M-001 is reported and not fixed: the skill text is the instrument of the arm
under measurement, and this role is excluded from both arms.

## Limits

- n = 1 campaign, n = 3 slices, n = 4 handoffs, all in one orchestrator session
  on one machine on one day.
- Every reviewer is a Claude agent inside the orchestration being measured. The
  only outside check on the verdicts is the gate re-run, which tier-2 did
  perform directly.
- Acceptance is scoped: each tier-1 verdict reads "as base for the next slice,
  not integration", and integration is `proposed_integrate` awaiting the user.
- Different harness, different provider, different quota. **This does not settle
  the Codex question.** It establishes that the handoff the Codex pilot
  pre-registered is reachable where a context-replacement primitive exists, and
  nothing about whether Codex workers would use one if given it.
- Slice 1 straddles the `fa163bad` instruction change: both builders read the old
  text and finished before the commit (reads at 00:55:35 and 01:05:44, last
  activity 01:11:58, commit 01:12:17), while its tier-1 reviewer read the new
  text at 01:12:54. Cohort labels per agent are in
  `claude-builder-skill-change-cohort-20261022.json`.
- Line counts: 10,203 is the net branch diff. Per-slice figures sum gross
  per-commit additions (10,310), so the two code slices' ratios are marginally
  optimistic.

## Amendment 1, 2026-10-22T13:55Z: integrated

The original record closed at `proposed_integrate`. R1 was subsequently
integrated and user-confirmed (`final-review/decision-branch-v2.json`), as
`12a770bb..24c41cb9` on `rescue-detached-20260911`, 23 commits onto base
`dc9fcc67`. Verified here: `git diff 12a770bb~1..24c41cb9` is 28 files, 11,415
insertions, 10,203 Rust lines — identical in size to the branch range scored
above, so **the denominator is unchanged and the headline figure stands**.
Post-integration gates: cargo 114 passed / 0 failed, Python unittest 41 OK.

The integration phase cost **2.026 Mwt over 22 orchestrator calls and launched
no agent**: the coordinator cherry-picked, verified and recorded it directly.
Because that is entirely orchestrator cost, the like-for-like figure of **3.61
Mwt/krs is unchanged**. Including orchestration the campaign moves from 5.46 to
**5.66 Mwt/krs**, and orchestration overhead from 33.9% to **36.2%** against the
baseline's 25% — the gap on the one measure where this arm was already behind
widens.

`decision-branch-v2.json` records that the integration base moved from
`cb61f807` to `dc9fcc67` because this measurement role committed between the
decision and the cherry-pick. Worth noting as a process fact: the measured and
the measuring sessions share a branch, and here the measurement landed inside
the subject's own integration record.
