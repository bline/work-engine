# Observed use of manual context management

Observation cutoff: **2026-10-24 01:49 UTC**. This is a retrospective log audit,
not a treatment comparison or an independent code-quality score. Running
sessions may have changed after the cutoff.

The handoff discipline is working better than automatic monitoring adoption.
The S10 chain kept recorded context below compaction through repeated transfers
and checked retained child work. B2 demonstrates actual control-event delivery,
but also demonstrates that receiving a warning does not ensure the next task
gets a fresh capacity decision. One telemetry defect is reproducible.

## Coverage and method

The audit indexed 260 available Codex session metadata records. The cohort is
20 roots launched since October 22 whose user messages explicitly reference
the S10 quota-repair or B2 handoff paths, plus their **93 recursive Codex
descendants**. It is a bounded recent cohort, not an all-time inventory of
either project. Two Claude CLI review sessions were located separately:
`588b328e-1242-4af7-9743-d84a8f683d79` and
`ea771f4a-a549-4b31-96ef-a403f0fccf87`.

Profiles use distinct `token_usage_record.response_id` values attributed by
`thread_id`. A child's `session_id` can name its root, so it is unsuitable for
per-child attribution. Input totals include cached input; they are not billed
cost. Compactions are recorded markers. Truncation counts are diagnostic output
markers, not an exact lost-token measurement. Some inter-agent message bodies
are encrypted; their delivery is observable but their meaning cannot be audited
from these records.

Frozen prefix snapshots, source paths, hashes, per-session profiles and the
reproduction are retained privately under
`/home/bline/.local/state/work-engine/context-usage-audit-20261024/`.
[Aggregate data](context-control-usage-20261024.json) records the scope and limits.
Raw transcripts are not published in the repository.

## Observed context behavior

| Cohort | Sessions | Model responses | Median peak input | Compaction markers |
| --- | ---: | ---: | ---: | ---: |
| S10 roots | 15 | 1,211 | 161,254 | 0 |
| S10 Codex descendants | 68 | 3,571 | 124,708 | 0 |
| B2 roots | 5 | 705 | 232,767 | 2 |
| B2 Codex descendants | 25 | 1,500 | 138,921 | 0 |

The two located Claude review transcripts also have no recorded compaction
boundary. They are excluded from this Codex token/call table.

These groups have different work and span changing instructions. Their difference
does not establish a treatment effect. Compaction alone also does not prove lost
work: B2's current root invoked Wind Walker and recovered its durable continuation
afterward. The audit does not establish whether every semantic obligation survived.

Of 93 Codex descendants, 91 have at least one final response in their retained
prefix. Two end without one; surrounding parents show replacement/recovery work.
A final response does not prove that a later resumed assignment completed, and
this audit has not certified all process-closure or handoff obligations.

## The two active roots

### S10: `01a16b84-9bcc-7301-87fc-c1a41024c44d`

This root **is not enrolled for automatic context-pressure events**. Its
assignment explicitly declares source-bound checks and `automatic_monitoring:
false`. Its board listener is board-only. It uses the older `23a0ea4a…` telemetry
pin and owns explicit checks. That is the supported fallback, rather than a
false claim that its board subscription supplies context stats.

The log shows actual checks and retained-work estimates. Examples include
builder `69,497 + 40,000`, later `121,599 + 25,000`, and reviewer
`115,549 + 10,000`. Its own latest recorded input at the cutoff was 119,548.
The builder and reviewer were retained across remediation instead of blindly
replaced. This is useful evidence of the skill being applied to decisions.

Sources: root transcript lines 74, 84, 107, 120, 239, 284, 337, 366 and 429;
`root-envelope-diagnosis-20261023/assignments-v1.json` and its referenced check
records. The assignment file is live state; the transcript prefix fixes what
was observed during this audit.

### B2: `01a16a7b-53af-76e3-b8c1-a7c58a4b8e98`

This root **did establish automatic control delivery**, using the control-capable
`a5f9db1f…` binary, its actual session/transcript binding, an owned one-shot
listener and self-thread wakeup collection.

- At 20:51:33 UTC it observed `planning` at **150,807** tokens. The complete
  event reached model-visible tool output at 20:52:02, about 30 seconds later
  (root transcript line 394). The root recorded a retain decision for bounded
  settings inspection (line 398).
- An `approaching_band` event at **179,644** was archived and a context judgment
  recorded at 23:09:59 (line 629): complete the already bounded live action,
  with about 78K of the actual window remaining. Archiving is weaker evidence
  than showing the full event in model input, so this is not counted as another
  independently verified full-payload delivery.
- At 23:15:11 an explicit control check delivered `in_band` at **206,014** into
  model-visible output (line 717). This was an explicit check, not a listener
  delivery. The continuation called for reassessment before substantial work.
- At 01:32:57 the user requested a new telemetry repair. The first subsequent
  model response used **212,825** input tokens (line 790). There is no new
  source-bound capacity check or recorded fresh/retain judgment in the bounded
  repair interval before compaction. Input rose to **245,372** and the host
  compacted at 01:43:02 (line 929).

The defect in behavior is at that new-work boundary, not failure to receive the
earlier warning. The earlier bounded-action judgment does not size a later
implementation task. Since unchanged `in_band` pressure is deliberately quiet,
the monitor provides no escalating warning as 206K becomes 245K.

## Findings

### CU-01: zeroed reset accounting becomes reassuring telemetry

Immediately after compaction, transcript line 933 contains input/cache/output
counters all equal to zero, while `total_tokens` is 28,432. The reader accepts
this as a valid request observation. The archived `listener-output-11-reset.json`
reports `normal`, `input_tokens: 0`, no limitations, and an entire empirical
band's worth of headroom.

This was reproduced with the root's exact pinned binary and a frozen transcript
prefix ending at that record. A fresh control invocation returns a `normal`
cursor; the original running listener emitted the normal event on the epoch
change. Current `main.rs` lines 655–745 also show that safe numeric input/cache
values are accepted without distinguishing this reset placeholder.

**Recommended repair:** recognize reset/accounting placeholders and represent
the new context as unmeasured until a genuine request supplies usage. Do not
invent zero pressure or borrow a predecessor epoch's forecast. Preserve a
regression fixture for this actual record shape.

**Follow-up from board 2708, independently checked:** the same absence-as-zero
problem affects `skills/manual-orchestration/scripts/child_context.py` on Claude
transcripts. Its parser admits `<synthetic>` assistant usage and defaults absent
usage fields to zero; its assessment accepts any integer, including zero. The
retained R2 `p1-portfolio/context-check-tests-predispatch.json` actually reports
`<synthetic>`, zero and `retain_ok`. The metrics reader
`metrics/scripts/claude_transcript_usage.py` excludes `<synthetic>` already.
This is two demonstrated defective read paths and a third reader with a useful
filter, not three demonstrated failures. Repair the usable-measurement contract
across live readers, with both actual record shapes as regressions.

Two qualifications to that message matter. A decreasing counter can be a valid
first request after reset; suppressing every decrease would reject valid usage.
Also, the cited 99,981-token Claude call occurred at 15:37:57 UTC, **after** the
15:36:46 check. The last real reading before that check was 91,579 at 15:11:36.
It is historical evidence, not a measurement of current context at check time.
The correct conclusion remains that zero/retain_ok was unsupported; replacing
it with the later 99,981 would introduce hindsight.

### CU-02: warnings need to survive into the next scope decision

B2 received pressure information and maintained continuation state, but a later
task expansion proceeded without the needed capacity assessment. The skill
already calls for boundary checks; another generic warning would add little.

**Recommended change:** make the unresolved pressure decision visible in the
continuation/dispatch state at a new substantial task boundary. Either establish
that the new task and collection fit or transfer at a safe boundary. Consider
additional headroom-based escalation beyond `in_band`, but do not turn an alert
into automatic reset authority.

### CU-03: rollout is still split between automatic and manual routes

The S10 root is managing context through explicit checks; B2 is using automatic
delivery. The observed S10 result is good, but cannot count as evidence that the
control subscription is broadly adopted. Repeated successors retain the old
pin, so merely reloading the current skill does not upgrade that capability.

**Recommended change:** make provisioning a capable pin and choosing the actual
observation route easy at root recovery. Preserve explicit fallback where
delivery is unavailable and record which mechanism is active. Avoid claiming
that every child must run its own listener when its parent already owns timely
checks and delivery.

### CU-04: context-loading and notification costs remain visible

The cohort contains 384 output records with truncation markers: 256 in S10 and
128 in B2. Active S10 examples are root startup lines 23/44, builder startup
lines 26/36/49 and reviewer startup line 21. Large combined doctrine, skill,
handoff and packet reads appear repeatedly. The current S10 root reached about
63.9K input before its first child dispatch. This is a loading-cost signal, not
proof that every such token was unnecessary.

In contrast, waits are mostly behaving as intended: among parseable S10 wait
results, 322 were event releases and six timeouts; B2 had 88 event releases and
no timeout. Three S10 results could not be classified by that response parser.
The active S10 root requested hour-long waits which returned on child activity.
Many message bodies are encrypted, so their individual decision value is unknown.

Follow-up to the user's 55-second-wait report: the wait counts above cover
`collaboration.wait_agent`, not board listeners. A targeted search found eight
55-second board-wait launches in B2 root `01a16905-01f0-70b2-9396-4672b3f8bd34`.
Each retries quiet timeouts inside one host-side JavaScript cell, with a bounded
30/40/60-iteration cap and a self-wakeup only after that loop ends. This does not
establish one inference per 55 seconds. The context immediately before those
launches ranged from 75,681 to 171,359 tokens. It remains unknown whether this is
the particular session the user observed. The instruction update removes the
generic recommendation to wake on every quiet timeout and distinguishes host
retries from model rearming. It is not a measured behavioral result.

B2 also collected its own board posts 2677, 2685 and 2696 through its listener.
Those were three of seven archived board-message deliveries consumed before
the cutoff. Each causes collection/rearm work without introducing an external
coordination fact. This is a candidate for owner-aware notification suppression,
while preserving board history and cursor semantics.

Root input accounted for 28.35% of S10's and 39.09% of B2's recorded Codex input
volume. These are token shares including cached input, not quota or dollar
shares, and not comparable measures of productivity across different tasks.

## Recommended order

1. Repair the zero-reading telemetry defect and test the actual reset shape.
2. Close the new-task capacity-decision gap; receiving an alert is not its outcome.
3. Improve successor provisioning and explicit recording of the chosen route.
4. Reduce repeated/truncated startup loading and unnecessary self-post wakeups.

Keep the filesystem/control-channel design. The evidence points to telemetry
interpretation, boundary decisions and loading cost, rather than a need for a
daemon. No running session, instruction or production implementation was changed
by this audit.
