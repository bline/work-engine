# Round 2: screen before profiling (amendment, adopted 2026-10-30)

Status: **adopted by the owner on 2026-10-30** (decisions below). It takes
effect with the release that carries its recorder and guidance changes. Until
then, round 2 runs as written in
[the plan](dogfood-selection-plan-20261027.md#round-2-revision-4-2026-10-28).

Claude proposed it on 2026-10-29. This revision records the owner's decisions,
and the removal of the coordinator role on 2026-10-30. The supervisor is now
the role that dispatches, and its guidance is `supervisor.md`.

## Why

Round 2 profiles every builder and reviewer dispatch, but only enrolled builder
episodes enter the comparison. The run directory, read on 2026-10-30 at about
04:40Z, held 103 assignments:

| Assignments | Count | Models |
|---|---|---|
| In enrolled episodes, builders | 22 | 12 Sol, 10 Terra, randomised |
| In enrolled episodes, reviewers (matched by task name) | 7 | Astra |
| Outside the comparison, reviewers | 36 | 31 Astra, 5 Opus |
| Outside the comparison, builders | 38 | 28 Terra, 10 Sol, chosen by judgment |

The root of the S10 session on 2026-10-28/29 (one root, n=1) made 95 calls that
ran a pilot command, at $8.36 API-equivalent. 36 of them ($3.27) ran nothing
else, so the pilot's incremental cost in that root was between $3.27 and $8.36.

Profiles exist so that a later selection policy can be trained on what was known
at decision time (`docs/design.md` in agent-routing-lab). That use needs two
things:
- **Variation in the model chosen.** Reviewer profiles outside episodes have
  almost none.
- **Outcomes.** Round 2 records reviews and repairs only for enrolled episodes,
  and `agent-routing-data` has no outcome format yet.

## Decisions (owner, 2026-10-30)

1. **Screen first, and profile only builder tasks that enter the comparison:**
   yes.
2. **Accounting at owner check-ins and round close, not after each dispatch:**
   yes.
3. **No profiles outside the comparison during round 2.** Builder profiles
   outside the comparison resume at a sampling rate (for example 1 in 3) once
   outcomes can be recorded for them too. That is a later decision.

## Procedure

This replaces steps 1 to 4 under "Screening and enrollment", and the accounting
after each dispatch.

For every builder dispatch, before choosing the model:

1. **Freeze the packet.**
2. **Screen** with `enroll --task-id T --packet P --eligible yes|no --reason R`.
   It records the screening: the task ID, the packet's SHA-256 and the reason.
   - If the task is not eligible, the round is paused, or the Terra in-flight
     limit is reached, the screening is recorded and the task is not enrolled.
     Dispatch as usual, with no sidecar, profile or accounting.
   - If the task is eligible and the round can enroll it, `enroll` answers
     `profile_required` and records nothing yet.
3. **Profile and enroll** only in that case:
   - run `start`, write the stage B sidecar, run `profile`, then
     `enroll --task-revision REV --eligible yes`, as now;
   - the profile is frozen before the draw and before the model is chosen.
4. **Dispatch the drawn arm** and run `assign`, as now.
5. **Later dispatches in the episode** (continuations, repairs, revised packets
   and its reviewers) run `assign --episode T`:
   - they are recorded against the enrollment profile, labelled episode
     dispatches, and not profiled again;
   - the reviewer leak check still runs.

Dispatches outside an enrolled episode, reviewers included, are not profiled.

**Accounting.** `assign` records child session IDs, as now. At each owner
check-in and at round close, the supervisor builds one usage ledger for the
round's window and runs `attribute --all` against it.

## Recorder changes

All in `metrics/scripts/task_profile_record.py`, with tests:

- **`enroll --packet PATH` without `--task-revision`** records the screening
  with the packet's SHA-256. For an eligible task that the round can enroll, it
  writes nothing and exits with a distinct `profile_required` exit code,
  documented in `--help`. The enrollment with `--task-revision` re-checks pause,
  in-flight limit and cap, as now.
- **`assign --episode TASK_ID`**:
  - resolves the enrollment's task revision and records the assignment there,
    with `episode_dispatch: true`;
  - keeps the reviewer leak check;
  - refuses a task ID that is not enrolled.
- **`attribute --all --ledger DIR [--since ISO] [--until ISO]`** attributes
  every assignment that has recorded child sessions and no attribution yet. It
  reports each one and never overwrites a write-once record.
- **Compatibility:** `status`, `review`, `repair`, `close` and the analysis joins
  keep working for records written before and after the change.

## Guidance

The "Task profile (shadow pilot)" and "Builder-comparison rounds" steps in
`supervisor.md` change to match. That changes the guidance hash, which marks
the switch in the records. Builder instructions don't change.

## What is kept and what is lost

**Kept:**
- every screening and its reason;
- prospective profiles for every enrolled episode;
- the random draw, the pause rules, the masking, and the joins from profile to
  enrollment, assignment, review and ledger.

**Lost:** profiles of dispatches outside the comparison (74 of 103 so far).
Their routing value now is small, for the reasons in "Why". Decision 3 brings
builder profiles back by sampling, once outcomes exist for them.

## Effect on round 2

Enrolled episodes are profiled, drawn, reviewed and recorded the same way before
and after the change. What changes is supervisor work outside the comparison.
The release goes into the plan's table of environment changes, with its release
commit and guidance hash.
