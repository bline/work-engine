# Shadow task-profile pilot, round 1: review (2026-10-28)

Plan: [`../dogfood-selection-plan-20261027.md`](../dogfood-selection-plan-20261027.md).
Records: `~/.local/state/work-engine/task-profile-pilot/task-profiles/`, 10 entries,
frozen 2026-10-28 15:09:55Z to 16:19:42Z under skill release `200bd77f` (guidance
sha256 `8cab5a55…`). Reviewed with the owner on 2026-10-28; the owner chose
"proceed with fixes". Records are not yet copied to the data repository.

## Against the plan's criteria

| Criterion | Result |
|---|---|
| Every eligible dispatch has a record | 10 records: 9 profiles, 1 failure record. 0 eligible dispatches unrecorded: 4 unrecorded workers in the window were read-only diagnosis (their supervisors' packets say so), and the one Claude session started in the window is recorded. One record is an adapter that launched the recorded Claude builder, so one task is counted twice. |
| Profile frozen before selection | 8 of 10. In 2 the ordering is not established: one `selected_at` equals the declaration start to the microsecond (copied), one was truncated to whole seconds (0.57 s before the freeze). |
| Profiling changed nothing | 7 dispatched packets are byte-identical to the frozen packet; 2 add only identity-binding and collection lines; 1 (the failure) has no frozen copy and scans clean. Two rp3 packets told their children about the pilot, and the native Claude builder read the revision ID and pilot instructions while exploring the run directory (no declared values: no `expected_write_set` or `declared_by` text in its transcript). The automatic leak check, which looked only for exact profile tokens, missed both. |
| Assignments follow the model selection | Requested and attributed models and efforts match in 10 of 10. Builders: `gpt-6.1-sol` medium 4, `claude-sonnet-5-5` high 1, Terra 0. Reviewers: `gpt-6-astra` high 5. Whether the Sol builds were hazard-class is not established. One profile was declared by an Astra supervisor, before Astra was limited to review. |
| Snapshots verify | `verify`: 10 of 10 verified, recompute included. 9 git snapshots; 2 records declared a manifest, against the git-only restriction, which the applied guidance had not carried. |
| Declarations vary | Over 9 profiles, distinct declared values: objective 9, expected write set 8, read set 8, verification 7, acceptance, recovery, state, exclusions and dependencies 6 each, persistence 5, uncertainties and compatibility 4, work type 3, interfaces 2, authority state 1. No field was declared unknown in any profile; whether that reflects packets or guessing is not established. |
| Overhead measured | Not established. 6 of 9 declaration times are 0.1 to 0.15 s, so the start time was taken after writing; none recorded coordinator call IDs. Validation failure rate 2 of 10: 1 compute crash, 1 invalid. |

## Defects found

1. **The guidance omitted the git-only snapshot restriction** of the plan's
   pilot restrictions. 2 coordinators declared manifest snapshots: 1 record
   crashed, 1 is invalid.
2. **`taskprofile compute` (v0.2.1) raised `KeyError: 'root'`** on a manifest
   without `root`. Fixed in agent-routing-lab v0.2.2 (`373c12f3`, pushed and
   tagged 2026-10-28); Work Engine pins it from `a1d665e8`.
3. **Sidecars were written beside packets** in run directories children read.
4. **An adapter was profiled as the builder.**
5. **Declaration start and selection times were taken by hand**, late or copied.
6. **The automatic leak check matched only exact tokens.**
7. **Failure records archived no sources**, so a failed dispatch's packet
   cannot be compared with the dispatched one.
8. **`--harness` values were free text**; the tier appeared in 1 of 10.

## Fixes (2026-10-28)

The guidance now: names the active round, run directory and cap in the plan's
Status lines; profiles the model that does the work, not its launcher; keeps
sidecars in `<run>/sidecars/`; requires git snapshots; forbids mentioning the
pilot in packets or files a child reads; stamps the start with
`task_profile_record.py start`; and fixes the `--harness` format with the tier.
`task_profile_record.py` adds `start`, archives sources in failure records, and
reports pilot mentions and the revision hash, case-insensitively.

## Spot-check of two declarations (2026-10-28)

`s10-config-readback-repair` (builder, declared by a `gpt-6.1-sol` xhigh
coordinator) and `rp3-git-abbrev-standalone-review` (reviewer, declared by a
`gpt-6.1-sol` xhigh supervisor), each read against its frozen packet (n = 2):
- **No declared value was invented.** Every declared value traces to packet text.
  The reviewer profile marks `parent_task_id` unknown, so coordinators do use
  unknown; "no unknown declared fields" reflects very specific packets.
- **Read sets are summaries.** The builder declares 2 items; its packet names at
  least 5 to read.
- **Obligation fields carry other things.** `state` held identities to record,
  and `persistence` held the result files to write. The template never defined
  them; Appendix B now does.
- **The snapshots describe the wrong tree.** Both tasks work outside the
  repository (a private variant; a private candidate repository), yet each git
  snapshot measured the whole Work Engine tree (3,096 and 2,701 files) with no
  coverage warning. The builder's coordinator noted it under `uncertainties`.

**Correction to the fixes above:** making git snapshots mandatory was wrong for
this workload; the two manifest declarations were the right instinct, badly
formed. The guidance now declares a git commit only when the inputs are in the
repository at that commit, and otherwise marks the snapshot unknown with that
basis. The check that would have caught it: comparing each record's snapshot
with its read set before setting the rule.

## Next (owner decision)

Round 2 is drafted in the plan (section "Round 2"): a builder comparison in
which a recorded flip picks `gpt-6.1-terra` or `gpt-6.1-sol` at medium where the
coordinator judges either acceptable, with review and acceptance unchanged. Not
approved yet. Round 1's records are in agent-routing-data (local commit
`69a8686`, not pushed).
