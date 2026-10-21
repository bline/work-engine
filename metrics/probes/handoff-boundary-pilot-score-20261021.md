# Scoring the worker-owned handoff boundary pilot

Status: first scoring pass, cohort end time **2026-10-21T21:55Z**, against the
[pre-registration](handoff-boundary-pilot-prereg-20261021.md) and its two
amendments. Per-thread counters are frozen in
[the score dataset](handoff-boundary-pilot-score-20261021.json), SHA-256
`41c997d0f24db7f0ef1903f44d747808be108b13888f9cbdea242fae90920a42`. The pilot is
still running; this scores what exists now and nothing is revised retroactively.

Written in the claude-metrics measurement role. No instruction was given to
either arm, and nothing here was posted to the Codex chatboard.

## Cohorts at the end time

Membership by `session_meta.timestamp`, as registered, not revised.

| cohort | n | compaction markers | self check-in invocations | thread-hours |
| --- | ---: | ---: | ---: | ---: |
| `pre_change` | 35 | 28 in 9 threads | 7 threads | 62.7 |
| `transition` | 3 | 0 | 3 threads, 30 calls | 1.9 |
| `post_change_r1` | 1 | 0 | 7 calls | 0.2 |
| `post_change_r2` | 0 | - | - | - |
| `post_change_r3` | 5 | 0 | 5 threads, 19 calls | 1.4 |

`post_change_r3` is `bounded_inference_b2_plan` (21:08:50Z),
`bounded_inference_b2_plan_review` (21:18:12Z), `bounded_inference_b2a`
(21:24:44Z), `bounded_inference_b2a/auth_worker` (21:26:16Z) and
`bounded_inference_b2a/auth_reviewer` (21:38:21Z). All five are
`subagent.thread_spawn` records with distinct `agent_path` values, so none is a
successor replacing a predecessor.

The baseline's 20 `pre_change` compaction markers are now 28. The registration's
"counters are live" clause applies: cohorts are fixed, counters are re-read.

## Primary outcome: B for the post-change cohort, which is `not established`

`post_change_r3` holds 13 child-thread boundary observation records and 11
`continue` statements read verbatim in durable artifacts. Every statement read
is `continue`; none is `prepare handoff`. The pairing is not one-to-one, because
some artifacts carry a single judgment covering two observations. No thread
reached a compaction marker, and all five threads retained observation JSON and
wrote at least one decision. That is outcome **B**, which the registration
records as `not established` rather than success, because it cannot be
distinguished from the assignments being small enough not to need a handoff.

The arithmetic makes the point sharper than the registration anticipated. The
instrumented-from-launch cohorts total **3.5 thread-hours** against
`pre_change`'s 62.7. At `pre_change`'s observed 0.45 markers per thread-hour,
3.5 hours predicts about **1.6 markers**. Observing zero is consistent with no
effect at all. No efficiency, quality or compaction-avoidance claim follows from
the zero.

What the cohort does establish is that the four legs are all readable at once:
invocation in the rollout, retained JSON, decision in a durable artifact, and a
declared transfer availability. The highest fraction at a recorded `continue`
is **69.3%** - `auth_worker` at 21:50:10Z, 179,007 of 258,400, epoch 0, no
lineage, reason "Worker continued to finish bounded checks/report with retained
context". That thread has not compacted. It is the closest the post-change
cohort has come to the decision the pilot is about, and it still decided
continue.

## Outcome A not observed

No `prepare handoff` decision, no `handoff_ready` declaration and no verified
successor exist in the scored domain.

Completeness basis: the domain is every `.json` and `.md` file under
`~/.local/state/work-engine` with mtime at or after 2026-10-21T20:22:24Z,
excluding build trees and vendored source copies, that carries a `decision*` or
`transfer_availability` key or a boundary/continuation filename - 120 records.
Every occurrence of `handoff_ready`, `successor` or `replacement requested` in
that set was read in context. All are either contract restatements of what a
transfer would require, explicit nulls (`successor_session_meta_id: null`), or
negations ("No replacement/handoff_ready requested"). The second leg is the five
`session_meta` records above, none of which binds a predecessor. Absence is
established within that domain, not for the campaign.

## Outcome C observed: 11 instances in 4 threads, all retrofitted `pre_change`

This is the outcome the pilot targets, and it appears only in threads that were
launched before the check-in block existed and received it by propagation while
running. All four are in root `01a15b93`'s `live4_execution_supervisor` subtree.
Each had recorded `continue` decisions of the registered form - for example
"parent-mediated transfer available, no replacement requested" and "ample
headroom and retained context make replacement costlier".

| thread | check-in | reading | % of 258,400 | compaction | gap | peak between |
| --- | --- | ---: | ---: | --- | ---: | ---: |
| `live4_execution_supervisor` | 19:57:43 | 174,829 | 67.7 | 20:29:48 | 32 min | 199,918 |
| `session_quota_repair_worker` | 19:11:45 | 118,686 | 45.9 | 19:29:45 | 18 min | 179,674 |
| `session_quota_repair_worker` | 19:46:13 | 134,118 | 51.9 | 20:03:31 | 17 min | 178,447 |
| `session_quota_repair_worker` | 20:14:46 | 109,654 | 42.4 | 20:25:37 | 11 min | 207,527 |
| `session_quota_repair_worker` | 21:05:43 | 157,876 | 61.1 | 21:07:38 | 2 min | 172,703 |
| `session_quota_repair_worker` | 21:43:44 | 196,031 | 75.9 | 21:44:25 | 41 s | 206,830 |
| `session_quota_repair_review` | 17:36:45 | 124,539 | 48.2 | 18:57:48 | 81 min | 208,482 |
| `session_quota_repair_review` | 20:26:23 | 217,221 | 84.1 | 20:28:20 | 2 min | 217,221 |
| `session_quota_repair_review` | 21:12:28 | 196,273 | 76.0 | 21:22:05 | 10 min | 206,111 |
| `session_quota_runtime_qualification` | 20:10:51 | 117,270 | 45.4 | 21:04:19 | 53 min | 185,304 |
| `session_quota_runtime_qualification` | 21:34:51 | 160,029 | 61.9 | 21:41:05 | 6 min | 185,061 |

The registration says C "is the only outcome that can show the judgment itself,
rather than its availability, to be miscalibrated". The readings separate it
into two shapes, and only one of them is about judgment.

**Shape 1, the reading was comfortable and growth outran it.** Five readings
fall in the 42-52% band, and **three** of them compacted within 11-18 minutes;
the other two ran 53 and 81 minutes before compacting. In the fastest instance
`session_quota_repair_worker` went from 109,654 at 20:14:46 to a 207,527 peak by
20:25:37 - about **9,000 tokens per minute** inside one assignment, n=1 for that
rate, one interval in one thread. A check-in at 42% is not a wrong judgment; the
instantaneous reading is a weak predictor over a single task's horizon, and the
brief's boundary check gives the agent a level when what decides the outcome is
a rate. `recent_growth` is in the CLI's output and none of these decisions cites
it.

**Shape 2, the reading was alarming and `continue` was recorded anyway.** Three
instances read 75.9%, 76.0% and 84.1% and compacted 41 s, 10 min and 2 min
later. These are class C in the registered sense: telemetry showed the boundary,
`continue` was recorded, compaction followed.

What those gaps are not is a handoff deadline. Each is time to compaction **under
continued work**, measured on a thread that kept consuming context. Deciding to
prepare a handoff changes that trajectory - the thread stops the token-heavy work
and starts writing a packet - so the remaining headroom under a handoff decision
is not the headroom observed here. Transfer latency is a plausible constraint at
these readings and this dataset does not measure it. Measuring it needs the
interval from a `handoff_ready` declaration to successor acceptance, plus token
growth during packet preparation, and outcome A has not occurred, so neither
exists yet.

This is a registered-in-advance outcome, not a story fitted to the numbers: C
was defined before any of these eleven instances existed, and the discriminating
check it prescribes - "inspect the recorded reason against the observed
trajectory" - is what separated the two shapes.

## Root stratum, reported separately and never pooled

Root `01a15f02` recorded two post-change decisions, both
`transfer_availability: "unavailable"`, both `continue-transfer unavailable`,
both class E. Per the registered scope limit, these measure the missing
root-transfer mechanism and yield no root-level handoff evidence.

One of them is the instrument working. The root compacted at 21:07:33. Its
21:08:04 check-in read 47,628 of 258,400 - 18.4% - and the record states that
the low reading "follows same-session compaction and does not erase prior
pressure", naming the previous observation of 200,742 at epoch 3 and all four
compaction markers from `lineage`. The pre-change failure mode this corrects is
on the record: `session_quota_repair_worker` first measured at 72,299 after five
compactions, with nothing in the output to say so. A thread that reads 18% after
a compaction and treats it as headroom has been told the opposite of its
situation, and both the CLI's `lineage` field and the brief's wording now
prevent that.

## Economics: 0 of 95 observations

No retained CLI observation carries an economics estimate. All 95 records with
a `context_fraction_proxy` field have `economics: null`, across 17 distinct
sessions, and every decision record reads "not estimated", several adding
"inputs insufficient" or "CLI supplied no scenario assumptions". Not one
invocation in the 44 enumerated rollouts passes a numeric value to
`--fresh-input-tokens`, `--preparation-tokens`, `--launch-tokens`,
`--rehydration-tokens` or `--reconstruction-tokens`. Those flag names do appear
54 times across 10 threads, which is why this needed checking rather than
asserting: every occurrence is the CLI guide's description of the flags or
`--help` usage output, and none is an invocation supplying values.

Completeness basis: the observation domain is every `.json` and `.stdout` file
under `~/.local/state/work-engine` outside build trees and vendored source
copies that parses as an object carrying `context_fraction_proxy` - the CLI's
own output shape - and the invocation domain is every `custom_tool_call` or
`function_call` record in the 44 enumerated rollouts whose text contains
`work-engine-context-status`. Absence holds within those two domains.

This is compliance, not failure: the brief instructs agents to record economics
as not estimated unless an explicit scenario supplies its assumptions, and no
one has constructed such a scenario. The consequence for measurement is that
the registered economics measure returns no data, and the brief's "compare the
extra handoff cost with avoided context re-send" is currently unexercised by
any arm. If an economic comparison is wanted, someone has to author the
scenario; the agents will not infer one, and under the current wording they are
right not to.

## Pinned binary and the correction that was not mine

`session_quota_repair_review` invokes the CLI through a wrapper that asserts the
binary's SHA-256 before executing it, and two pinned digests appear over the
run. The order is `2f9fc0d1...` first, from 20:05:22Z, then `23a0ea4a...` from
20:41:50Z - after the fork-identity repair, whose state directory was written
20:37-20:40Z. The build-lock risk raised against `cargo run` is handled, and the
digest change is visible rather than silent.

Scoring note on my own instrument: the first pass classified this thread's
invocations as "other" rather than self-queries, because the detector looked for
`--session <own id>` as adjacent tokens and these calls pass the id inside a
Python argument list. The wrong claim was that the thread had no self check-ins;
the class is a pattern-matching artifact read as a behavioural difference; the
consequence would have been to report a compliant thread as non-compliant and to
drop its three outcome-C instances, which are the most informative rows in this
document. The check that caught it was printing the raw invocation text instead
of trusting the tag.

## What would change the result

- A `post_change_r3` thread crossing 60-70% with a bounded continuation left,
  declaring `handoff_ready`, and a parent launching a verified successor:
  outcome A at n=1.
- A `post_change_r3` thread compacting: outcome C from launch rather than from
  retrofit, which would separate instruction-from-launch from propagation.
- Any thread citing `recent_growth` or a rate rather than a level in its
  recorded reason: the shape-1 gap closing.
- An authored economics scenario: the registered economics measure becoming
  scoreable at all. The orchestration side is preparing an explicitly labelled
  sensitivity scenario. Scoring will read it as an assumption-backed bound and
  report the assumptions beside the number, never as a measured saving.
- A measured declare-to-acceptance interval, which is the only thing that can
  turn the transfer-latency hypothesis above into a result.

Reproduction is as registered. The per-thread frozen values, including
`compaction_timestamps` and `compactions_after_first_checkin`, are in the score
dataset beside this file.

## Amendment 1, 2026-10-21T22:05Z: three revision-bound corrections

Raised by the orchestration side against the pinned report and verified here
against the rollouts before being accepted. The scored outcomes - B for
`post_change_r3`, A not observed, C at 11 instances - are unchanged.

**The 42-52% band.** The wrong claim was "four instances read 42-52% and
compacted within 11-18 minutes". Five readings fall in that band and three of
them compacted in that window; the remaining two ran 53 and 81 minutes. The
class is a count stated without re-reading the rows it summarises - the band and
the gap range were taken from different subsets of the same table. The
consequence was to overstate how tight the level-to-compaction coupling is,
which is the evidence for shape 1 being about rate rather than judgment. The
check that would have caught it is filtering the table by both conditions and
printing n, which is now done: five in band, three within 11-18 minutes.

**The gaps are not a handoff deadline.** The wrong claim was that at 84.1% with
two minutes left "that round trip cannot complete, so the actionable window had
closed", and that transfer latency "is the binding constraint at those
readings". Each gap is time to compaction under continued work; a handoff
decision stops the consumption that produced it, so the observed trajectory is
not the trajectory a handoff would have faced. The class is an unmeasured
counterfactual asserted as a finding. The consequence is the more serious of the
three, because it was the headline interpretation of shape 2 and it would have
pointed design attention at transfer latency on evidence that cannot support it.
The missing check is simply whether any declare-to-acceptance interval exists in
the data: none does, since outcome A has not occurred.

**Binary order.** The wrong claim was `23a0ea4a...` earlier and `2f9fc0d1...`
after the fork-identity repair. First appearances are `2f9fc0d1...` at
20:05:22Z and `23a0ea4a...` at 20:41:50Z, so the order is reversed and so is the
attribution: `23a0ea4a...` is the post-repair binary. The class is an ordering
inferred from the sequence in which I encountered the digests rather than from
timestamps. The consequence is confined to this paragraph - the pinning
behaviour it describes is unaffected - but it would have misdated which binary
the post-change cohort ran. The check is a first-appearance timestamp per
digest, which is what established the correct order.
