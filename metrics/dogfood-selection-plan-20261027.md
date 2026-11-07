# Dogfood selection plan, phase 0: shadow-mode task profiles (2026-10-27)

**Current status (2026-11-07), superseding the Status paragraph below: round 2
is paused.** The marker `~/.local/state/work-engine/task-profile-pilot-2/PAUSED`
has existed since 2026-11-01T08:41:08Z, with reason "Terra trigger: P1 episodes
[]; repair rounds 10 (budget 10)". Only the owner resumes it. **Do not screen or
enroll any dispatch for round 2 while it is paused.** The Status paragraph below
is history: it records the state on 2026-10-28 and is not current. A round's
state lives in its run directory, not in this plan. This notice is a temporary
stopgap, not the admission authority.

Status (2026-10-28): **round 2 is active: a builder comparison** (section
"Round 2"). Run directory: `~/.local/state/work-engine/task-profile-pilot-2`.
Limits: a cap of 20 enrolled episodes, counted by `enroll` (not by
`task-profiles/` entries); at most 2 open Terra-arm episodes; 10 Terra repair
rounds before new enrollment pauses. Approved by the owner on 2026-10-28 ("Let's
do it") after design, tooling and join-check reviews. **Owner clarification,
2026-10-28:** an owner instruction to use Terra does not make a task ineligible;
"my terra decision does not mean you can not help round 2 by using different
builders". Where both arms are acceptable, the draw decides. Round 1 (run directory
`~/.local/state/work-engine/task-profile-pilot`, cap 10) completed and was reviewed
([review](probes/dogfood-pilot-round1-review-20261028.md)).

## Why

Model selection needs outcome data for the configurations we run, on tasks like
ours.
- **Public data doesn't qualify.** Current-model results with per-task outcomes
  exist (Harbor Terminal-Bench 4.0, NixBench, FrontierML), but none carries a
  reuse license. The sources that are licensed cover tasks, not current-model
  outcomes. The survey was recorded 2026-10-27 in the session that produced this
  plan.
- **Running a benchmark ourselves is out of reach.** Even a small calibration
  set, at typical per-trial costs for long agentic tasks, would take weeks of our
  subscription quota (unverified estimate).
- **So the data has to come from our own work,** recorded as it happens.

Phase 0 records a task profile at each dispatch and changes nothing else. Its
question is narrow: can the profile be produced reliably, cheaply and without
side effects? Whether profiles predict anything is a later question, for when
outcomes have accumulated.

Selection itself stays as it is. When it later becomes adaptive, it starts per
role and configuration, not from task features. In exploratory tests on a public
benchmark, deterministic task features gave no reliable routing gain over the best
fixed configuration on held-out tasks. The detailed results are kept locally and
not published, because the underlying trial data carries no reuse license
(`~/cbm-work/routing-data-audit/RESULTS-tb4-features.md`).

## Rules for phase 0

| The profile step does | The profile step does not |
|---|---|
| Record a stage B profile for each eligible dispatch before the builder or reviewer is selected | Enter the builder's or reviewer's prompt, packet or context |
| Record who declared it (coordinator model, effort, harness, guidance and schema versions) | Change the plan, its requirements, the model, the effort, the review route or acceptance |
| Record validation failures and unknown fields | Block or delay a dispatch because a field is unknown or validation failed |
| Run `task_profile_record.py`, which calls the pinned `taskprofile` (no model call of its own) | Add any model call. The coordinator writes the declaration in its own turn |

Model and effort keep coming from the active user selection
([model selection](../docs/model-selection.md)), as
[supervisor guidance](../skills/manual-orchestration/references/supervisor.md#dispatch-and-the-retained-or-fresh-choice)
already requires. That guidance's "learn from outcomes without creating a new
gate" applies here as well.

## Sequence at each eligible dispatch

1. **Freeze the plan.** The accepted packet and its authoritative documents are
   final. Record their sha256.
2. **Declare.** The coordinator writes the stage B sidecar (Appendix B) from the
   frozen packet and its sources. It names no builder, reviewer, model or effort.
3. **Compute and freeze.** `task_profile_record.py profile` runs the pinned
   `taskprofile compute`, then `validate`. Write the profile once. On failure,
   write a failure record with the errors, then go on to step 4.
4. **Select.** Choose the builder or reviewer exactly as today.
5. **Dispatch and record the assignment** (below), referencing the profile.

Phase 0 enforces ordering, not blindness. The coordinator who declares the
profile still chooses the builder by hand and may have one in mind. Blindness
needs assignment by a separate step, which is out of scope here. A profile
written after the assignment is kept, but marked
`eligible_for_unbiased_analysis: false` with the reason.

## Records

There are three records, joined by `task_revision`. Its ID is the task ID plus
16 hex characters hashed from the schema, task ID, stage, target role, frozen
packet documents and snapshot identity; `taskprofile` computes it (v0.2.0 and later). Each record is written once. A correction is a new file that names
the one it supersedes.

Each round's records go in the one run directory its Status lines name, so the
cap can be checked across sessions; sidecars go in that directory's `sidecars/`
folder, never beside a packet. Round 1 used
`~/.local/state/work-engine/task-profile-pilot`. Layout under the run directory:

```
task-profiles/<task_revision>/
  sidecar-stageB.json        declaration as written by the coordinator
  profile-stageB.json        compute output (profile-failed.json if compute failed)
  validation.json            validate --production result; failures are recorded, never blocking
  assignment.json            the assignment observation (retained continuations add assignment-02.json, ...)
  overhead.json              instrumentation cost and the tool pin
  attribution-<assignment>.json   models actually used, from the usage ledger
  sources/<sha256>           copies of the packet and authority documents, named by their hash
  SHA256SUMS                 append-only index
```

`metrics/scripts/task_profile_record.py` writes these records, never overwriting
them:
- `profile` writes the sidecar, profile, validation and overhead records, and
  archives the source documents. It never blocks a dispatch:
  - a missing tool, an unreadable or invalid sidecar, a compute error or an
    invalid revision ID writes a failure record and exits 0;
  - a revision that already has a profile exits 3 ("already profiled").
- `assign` writes the assignment observation. It requires the child's session or
  thread ID, and checks that the dispatched packet contains none of the profile's
  identifiers.
- `attribute` resolves the models and efforts the child sessions actually ran,
  from the usage ledger, and flags any substitution. `--since` and `--until`
  bound it to an assignment's time window, which a retained or reused thread
  needs. Without them, every call of the session counts.
- `verify` checks that the hash list exists, that every file is listed and
  matches, and that each record's `prior_files` hashes still match. It then
  recomputes each profile with the tool for that record's own pin, and checks
  each archived source against its hash. Exit 0 means fully verified, 2 means
  hashes pass but no recompute was done, 1 means a problem, and 3 means there
  are no records.

Integrity covers accidental change, not an adversary. The hash list is
append-only, and every record after the first also stores the hashes of the
files written before it. The data repository's git history is the
tamper-evident copy.

Tests: `metrics/scripts/tests/test_task_profile_record.py`, 22 cases. They skip
unless the agent-routing-lab checkout (`AGENT_ROUTING_LAB`, default
`~/code/agent-routing-lab`) is at the pinned commit; check that the run reports
no skips.
- **Normal operation:** the full cycle, model substitution, an unknown child, and
  separate builder and reviewer revisions.
- **Integrity failures v0.1.0 missed:** a missing hash list, an unlisted file, an
  edit hidden by a rewritten hash list, a skipped recompute, an invalid revision
  ID, and a leak into the dispatch packet.
- **Second-review findings:**
  - a missing tool and an invalid sidecar each recorded as failures, not blocking;
  - a duplicate profile, and `verify` with no records, each exiting 3;
  - microsecond timestamps and selection order;
  - the attribution window;
  - a tampered archived source.

- **Profile** (`profile-stageB.json`): the task-profile v0 output, which
  carries identity, input snapshot, declarations, computed features, coverage
  and provenance. `provenance.declared_by` must give the coordinator's role,
  model, effort, harness and version, plus the guidance version (the sha256 of
  the coordinator guidance file as installed from the deployed skill release).
- **Assignment observation** (`assignment.json`), kept separate from the profile
  so the selected configuration never becomes part of the task's description:

  | Field | Value |
  |---|---|
  | `task_revision` | join key |
  | `profile_sha256` | hash of the profile, or of the failure record |
  | `profile_frozen_before_assignment_record` | freeze time earlier than the time the assignment record was written |
  | `selected_at`, `profile_frozen_before_selection` | when the model was actually chosen (coordinator-supplied), and whether the profile was frozen before it; `null` if not supplied |
  | `role` | builder or reviewer |
  | `requested` | model, effort, harness |
  | `reported` | model and effort as the coordinator reports them, marked `source: coordinator` |
  | `child_sessions` | the child's Claude session or Codex thread IDs; attribution uses these |
  | `assigned_at` | timestamp |
  | `run` | run registry name |
  | `retained_continuation` | true when it continues an existing assignment on the same revision |
  | `recorder` | path and sha256 of the record script that wrote it |

- **Overhead** (`overhead.json`):
  - start and freeze timestamps for the declaration;
  - coordinator call IDs in that interval, so the usage ledger can price them;
  - validation errors;
  - the count of unknown declared fields;
  - any re-declaration;
  - the record script's path and sha256 (`recorder`), since coordinators run it
    from their checkout rather than from a release.

  Timestamps live here, not in the profile, so the profile stays reproducible.

Outcomes are not new records. They come from the existing run registry and
usage ledger (acceptance, repairs, findings, per-role cost), joined by run and
`task_revision`.

## Eligible dispatches

**Eligible:** builder and reviewer dispatches in production work that carry a
frozen packet with a write scope or a review subject.

**Not eligible:**
- experiment arms, which have their own frozen manifests;
- scouts and recon without a write scope;
- administrative operations.

A retained continuation on the same revision reuses that revision's profile and
records only a new assignment observation.

## Measurement environment

- **The supervisor change is an environment change.** The Astra to
  `gpt-6.1-sol` xhigh switch changes who writes declarations. Profiles carry the
  coordinator's identity. This plan first recommended running the pilot entirely
  after the switch, so all 10 share one coordinator. **The owner decided on
  2026-10-28 to start before it** ("proceed"): the Codex reset came that day, the
  switch now waits for O2 and a real-slice validation (O4 or E5), and Terra's
  first production builds would otherwise go unrecorded. A switch during the
  pilot is reported as an environment change, using `declared_by`. The switch
  has its own gates and doesn't wait for this plan.
- **Introducing the sidecar is an environment change too,** under the experiment
  plan's re-pin rule. Asking for declarations may itself change how plans are
  written. Records from before and after are not compared directly.

## Pilot: the next 10 eligible dispatches

| Criterion | Check |
|---|---|
| Every eligible dispatch has a profile record | 10 of 10 have `profile-stageB.json` or `profile-failed.json` |
| No profile was frozen after its selection | `profile_frozen_before_selection` true in 10 of 10 (`--selected-at` supplied every time), or reasons recorded |
| Profiling changed nothing | No dispatched packet or prompt contains the sidecar or profile (search the packet and prompt text for the `task_revision` and field names); assignments follow the active model selection |
| Snapshots verify | Re-running `compute` on each sidecar later gives identical bytes; snapshot is a git commit or a verified manifest, otherwise reported as unknown |
| Declarations vary | Report, per field, how often it is declared and how many distinct values occur. No pass threshold is set in advance; the review decides whether a field is useful |
| Overhead measured | Median and maximum declaration wall time, coordinator tokens and API-equivalent cost per profile, and validation failure rate |

Restrictions for the first 10:
- **Git-commit snapshots only.** Manifest snapshots wait until manifest capture has
  been reviewed further.
- **Audit each dispatched prompt by hand,** in full. The automatic leak check only
  searches the packet for the profile's identifiers, not the assembled prompt.
- **Supply `--selected-at` on every assignment,** and give `attribute` the
  assignment's time window whenever the child thread is retained or reused.
- **Confirm that every source document was archived** (`overhead.json`
  `sources`) before copying the records to the data repository.

Limitations the pilot report must state:
- **Selection time is the coordinator's own account.** `--selected-at` is supplied
  by the coordinator, so it doesn't independently prove when selection happened.
- **Exact recompute needs the original paths.** The source documents are archived
  by hash, but `taskprofile compute` reads them from their original paths. If an
  original disappears, the archive can still be checked against its hash, but an
  exact recompute needs that file restored to its path first.

After the 10, the owner reviews the profiles and the discrepancies. Possible
outcomes: make collection permanent, revise the schema, or stop.

## Round 2 (revision 4, 2026-10-28)

The round's state is in the Status lines at the top. It was approved and
activated on 2026-10-28 (see "Activation" below).

**Design review: ready** (`gpt-6-astra` at `high`, standard tier, one retained
read-only session, 2026-10-28; [review](probes/dogfood-round2-review-astra-20261028.md)).
Revisions 1 and 2 were found not ready, revision 3 ready with one change, and this
revision 4 ready. The finding IDs (R2-01 to R2-15) are marked where they apply.
The verdict covered the design only; the activation prerequisites, including the
owner's approval, were met separately.

**Question.** On this project's own work, in the subset where either builder is
acceptable, do `gpt-6.1-terra` builds at `medium` pass review and need repairs at
rates like `gpt-6.1-sol` at `medium`? The answer is descriptive for that subset.
Round 1 had no Terra builds at all.

### Unit (R2-03)

One **enrolled episode**: a builder task from enrollment to a terminal
disposition. It keeps the arm drawn at enrollment through continuations, repairs,
packet revisions and escalation; the models actually used are reported beside it.
A task ID enrolls once.

Definitions, fixed before the round starts:
- **First artifact:** the builder's first result submitted for independent review.
- **Pre-review fixes:** changes the coordinator requests before that review,
  counted separately.
- **First-review pass (J-2):** the first independent review of the first
  artifact returns the verdict `accept` with no P1 or P2 finding (see "Review
  result" below).
- **Repair round (R2-10):** one review-driven remediation cycle, from a review
  that requires changes to the builder's resubmission for review. Continuation
  turns within the cycle are one round, not several.
- **Escalation (R2-10):** a builder change made under the model guide's current
  escalation rule, when its trigger arises and before the triggered work starts.
  Under the guide as clarified on 2026-10-28 (`d16eeba5`): when evidence exposes
  substantial unresolved decisions still delegated to the builder, or repeated
  failures on clear requirements support a model-capability explanation; a
  repair-round count or a risk-bearing subsystem alone is not a trigger. A
  finding that reopens design goes to its decision owner, and the builder is
  selected again for the remaining work. The episode keeps its arm; the
  escalation and its stated reason are outcomes.
- **Terminal disposition:** accepted, abandoned (with the reason), or withdrawn
  before dispatch (with the reason).

### Screening and enrollment (R2-01, R2-06)

**Amended 2026-10-30 (owner):** from the release that carries it, the
[screen-first amendment](dogfood-round2-amendment-proposal-20261029.md) replaces
steps 1 to 4 below and per-dispatch accounting:
- screen with `enroll --packet` before any profile;
- profile only tasks that enter the comparison;
- record later episode dispatches with `assign --episode`;
- run `attribute --all` at check-ins and round close.

For every builder dispatch the guidance would profile, before choosing the model:
1. Freeze the packet and profile it, as in round 1.
2. **Screen**, with `task_profile_record.py enroll`. The coordinator gives the
   task ID, the profile revision and an eligibility decision with its reason. A
   task is eligible only if:
   - `gpt-6.1-terra` and `gpt-6.1-sol` at `medium` are both acceptable for it
     under the model guide;
   - the round's review route, a separate `gpt-6-astra` reviewer at `high` on the
     standard tier, meets the task's review contract: not an architecture,
     ownership-contract or open-design subject the guide sends to `xhigh`, and no
     campaign or provider contract requiring something else;
   - the round is not paused and the in-flight limit is not reached (below).
   Every screening is recorded, eligible or not, in an append-only screening log.
3. **Draw**, only for an eligible task: `enroll` then takes one random bit from
   the operating system, after the eligibility decision is written, and records
   the arm in `enrollment.json` (task ID, revision, screened time, eligibility,
   reason, arm, draw source). The draw does not depend on task content, so the
   arm cannot be known or steered before screening. Enrolling a task ID that is
   already enrolled returns its existing arm and draws nothing.
   - **Failure handling (R2-09).** The enrollment record is created exclusively,
     one per task ID, before the arm is printed; a concurrent second call reads
     the first call's record. If `enroll` fails or its result is unclear, run it
     again for the same task ID: if a record exists, it returns that arm, and the
     task is enrolled. Only when `status --task-id` confirms no record exists is
     the task "not enrolled (instrumentation failure)" and dispatched as usual. A
     recorded arm is never redrawn, erased or relabelled.
4. Dispatch the drawn arm and run `assign` as before. If the task is withdrawn
   before dispatch, `withdraw` records why; it stays in the analysis.

### Pausing and limits (R2-02, R2-04)

- **Pause flag.** `task_profile_record.py pause --reason` writes `<run>/PAUSED`;
  while it exists, `enroll` records every screening as ineligible ("paused") and
  draws nothing. Only the owner lifts it: `resume` requires the owner's approval
  text, which is recorded.
- **Triggers, acted on when known, not when an episode completes:**
  - a P1 finding on any Terra-arm artifact, first or repaired: whoever receives
    the review runs `pause` at once;
  - the production rework budget, a **new-enrollment pause threshold** (R2-11):
    each completed repair round is recorded when its resubmission happens, with
    `task_profile_record.py repair`, so open episodes count. When the recorded
    Terra-arm rounds, open and closed, reach 10, `repair` writes the pause flag
    and `enroll` refuses new enrollment. Episodes already admitted continue past
    it.
- **In-flight limit.** At most 2 Terra-arm episodes may be open at once. The
  check runs before the draw and blocks the whole round, both arms, so it cannot
  bias which tasks get which arm.
- **Work in flight continues** under the normal review, repair and escalation
  rules. A pause stops new enrollment only.
- **Comparison is reported, not a stop rule.** Per-arm figures are reported at
  each owner check-in, and the owner may stop the round on them; no rule compares
  totals across arms of different sizes. The **first-review pass rate (R2-12)** is
  taken over episodes whose first review has completed, recorded when it
  completes with `task_profile_record.py review`, whether or not the episode has
  closed. Its failures are split by verdict: `changes_required` (defects) is the
  builder comparison; `blocked_evidence` and `blocked_scope` are reported apart,
  as packet problems (J-3). **Repair burden (R2-14)** is completed repair rounds per closed,
  dispatched episode with a known count, zeros included, reported separately for
  accepted and abandoned episodes. Open episodes' rounds to date are reported
  apart. Rounds among only the episodes that needed a repair, if reported, are
  labelled as that separate statistic. Withdrawn, never-reviewed and pending
  episodes are counted separately, and every enrollment stays in the overall
  accounting.

### Review route and masking (R2-06, R2-08)

- Enrolled episodes use the round's review route. If a review must escalate
  during the episode, it does; that is recorded as an outcome.
- **The reviewer is not told the arm.** The review packet names the subject by
  path and hash and gives the reviewer the artifact and verification evidence,
  not the builder's provenance or result file, which names its model. The
  `assign --role reviewer` leak check also looks for both arms' model names.
  When the reviewer can still see the builder's identity, the coordinator records
  that the episode was unblinded, and the reading reports how many were.
- **Review result (J-2, J-3).** The review packet of an enrolled episode asks for
  one verdict and P-scale severities (P1 blocks acceptance, P2 must be fixed
  before acceptance, P3 is an improvement), whatever scale the reviewer's own
  contract also uses:
  - `accept`: the artifact is acceptable for the reviewed scope; P3 findings only;
  - `changes_required`: at least one P1 or P2 defect in the artifact;
  - `blocked_evidence`: the reviewer cannot judge, because evidence the packet
    should have supplied is missing;
  - `blocked_scope`: the reviewer cannot judge, because the subject or packet scope
    is wrong.
  `review` records the verdict, the counts, the receipt and the **reviewer's
  session or thread ID (J-1)**, so the reviewer is joined without parsing the
  receipt. The two `blocked_` verdicts describe the packet, not the builder.

### Measurement (R2-07)

- **Tier** is the one observed in each assignment's rollout (the usage ledger's
  `service_tier`), not the `--harness` text. Episodes are compared within a tier;
  mixed-tier episodes are flagged.
- **Cost** is reported as API-equivalent dollars at the frozen rates, labelled an
  estimate; a quota-weighted figure is labelled modelled (the documented 2.5 for
  Fast). Each episode reports builder, repair and review cost, plus coordinator
  cost where the ledger can attribute it; missing components are listed.
- **Wall time (R2-13)** is elapsed time from the first dispatch (`assign`) to
  the terminal disposition (`close`). Accumulated turn duration, from the
  ledger's completed turns, is reported separately as active model time. A
  pending episode reports elapsed-to-date, kept apart from completed episodes.

### Records and joins (R2-03, R2-05)

`review` records each episode's first-review verdict, counts and reviewer when it
completes (`status` reports a reviewer with no reviewer assignment record), and
`repair` records each completed repair round; both are append-only. `close` writes
the terminal disposition once: counts (pre-review fixes, repair rounds,
escalations), the review receipts' paths and hashes, the artifacts' hashes, and
the assignment windows. The analysis includes every screened and
enrolled task; pending and missing outcomes stay explicit, and accepted-only
summaries are never reported alone. Before activation, the joins from profile to
enrollment, assignments, attribution, ledger and review receipts are
demonstrated on two round-1 records.

### Size and reading

- **Cap:** 20 enrolled episodes across both arms. Screening has no cap.
- **Reading:** per arm, n, first-review pass rate with an exact 95% interval,
  `changes_required` and `blocked_` counts separately,
  repair burden and escalations as defined above (closed, dispatched episodes
  with known counts, zeros included), and cost; with the screening
  and exclusion counts, clustering by root and workstream, unblinded episodes, and
  the pause history. No ranking or equivalence claim: at this size, even 0 failed
  first reviews in 10 Terra episodes leaves a one-sided 95% upper bound of about
  26% on the failure rate.

### Activation (R2-05)

All of these, then owner approval:
1. `enroll`, `withdraw`, `review`, `repair`, `close`, `pause`, `resume` and
   `status` built and tested, with the random draw, the screening log, exclusive
   and idempotent enrollment (including a failure after the record is written and
   two concurrent calls), the pause flag, the in-flight check and the live
   rework budget. **Done 2026-10-28** (`5987a47c` to `57066b1a`, 79 tests; code
   review ready to integrate:
   [record](probes/dogfood-round2-tooling-review-astra-20261028.md)). Episode
   records are self-verifying (`content_sha256`).
2. **Done 2026-10-28** (released with this change). The coordinator guidance updated for round 2 and released: screening,
   enrollment and the drawn arm replace "choose as today" for enrolled tasks;
   counting is by enrolled episodes, not `task-profiles/` entries; a failed
   `profile` leaves the task unenrolled and dispatched as usual, and a failed
   `enroll` follows the failure handling above.
3. The joins demonstrated on two round-1 records. **Done 2026-10-28**: the chain
   resolves ([join check](probes/dogfood-round2-join-check-20261028.md)); its gaps
   J-1 to J-3 are answered by "Review result" above and by `review` recording the
   verdict and reviewer (`f87d8353`, reviewed ready; a rerun on the same records
   joins cleanly).
4. These Status lines name round 2, its run directory
   (`~/.local/state/work-engine/task-profile-pilot-2`) and its limits (20 episodes,
   2 Terra in flight, 10 Terra repair rounds). **Done 2026-10-28** on the owner's
   approval; `init-round` wrote the run directory's `round.json`.

### Environment changes during the round

Releases of `manual-orchestration`, the skill that carries the round's steps,
recorded 2026-10-30 from the release history
(`~/.local/share/work-engine/skill-releases/history.jsonl`). Records from
different periods are reported apart and not compared directly. Each episode's
arm is drawn independently, so both arms run in every period.

| Release, deployed | What changed for the round | Guidance hash | Enrolled, Sol / Terra | Open at the change |
|---|---|---|---|---|
| `a7c1cf66`, 2026-10-28T21:42:45Z | Activation | `a6c1c27d…` (`coordinator.md`) | 2 / 2 | none |
| `46d30e9f`, 2026-10-29T01:00:11Z | Automatic same-session continuation for outer agents, in other files of the skill; the guidance file is unchanged | `a6c1c27d…` | 2 / 0 | 1 Sol, 1 Terra |
| `471f6106`, 2026-10-29T20:10:27Z | "Scope and budget" added to the guidance: a supervisor records hour and quota limits and stops dispatching at them. Also rules for writing packets | `3b9197b8…` (`coordinator.md`) | 2 / 4 | 1 Sol |
| `5695755f`, 2026-10-30T03:19:59Z | Coordinator role removed (Appendix A, item 13). The round's steps moved to `supervisor.md` unchanged, except "Declare your role as `supervisor`" and the file to hash | `f7a57b7a…` (`supervisor.md`) | 1 / 0 | 1 Sol, 1 Terra |
| `99450c33`, 2026-10-30T04:07:40Z | Budget observation ([plan](../planning/work-engine-budget-observation-implementation-plan.md)). In `supervisor.md`, "Scope and budget" now records the objective's sessions and per-pool cost limits, measures spend per objective with `usage_ledger.py tree`, and delivers budget events through the board listener. It treats the shared weekly meter as account-wide: a weekly-point limit stops only when the user set it. The round's own steps are unchanged. `usage_ledger.py`, which `attribute` reads from the checkout, changed at the merge (2026-10-30T04:00:09Z). Call rows gain `pools`, and prices were added for Fable 5.1, Opus 5, Opus 4.8 and Haiku 5.5. For the six sessions compared, the call rows were otherwise identical to the previous ledger's. `task_profile_record.py` is unchanged | `06c2fe06…` (`supervisor.md`) | 1 / 0 | 1 Sol |
| `4fb1a51f`, 2026-10-30T05:07:40Z | [Screen-first amendment](dogfood-round2-amendment-proposal-20261029.md) (Appendix A, item 14). Builder dispatches are screened with `enroll --packet` before any profile, and only tasks that enter the comparison are profiled. Later episode dispatches use `assign --episode`, and accounting is `attribute --all` at check-ins and round close. `task_profile_record.py` changed at the merge (2026-10-30T05:00:08Z) and stays compatible with the earlier commands. Enrolled episodes are profiled, drawn, reviewed and closed as before | `cd2831a7…` (`supervisor.md`) | 1 / 3 | 1 Sol |
| `af98ad41`, 2026-10-31T03:38:01Z (merged 2026-10-30T16:15:44Z) | `task_profile_record.py verify` recomputes each profile from its archived source copies, so a later edit to a live input no longer fails an intact record: before the change 14 of 97 profiles failed for that reason; after it, all 97 profiles and 18 episode records verify. Recorded values and the other commands are unchanged; this took effect at the merge. In `supervisor.md`, the review packet of an enrolled episode now carries the plan's severity and verdict definitions ("Review route and masking"), after a reviewer given only the verdict names returned `changes_required` with one P3. That reached agents at the deployment | `57916023…` (`supervisor.md`) | none yet; round paused since 2026-10-30T13:33:06Z | 1 Sol |

Enrollment counts are by enrollment time; every enrollment profile's hash matches
its period. The hash marks the period, not the declaring role: 33 of the 83
sidecars written before `5695755f` already declare `supervisor`. A supervisor
already running may follow text it loaded before a release, so the boundary is
approximate for episodes in flight. An episode stopped at a scope-and-budget
limit closes as `abandoned` with that reason, under the terminal-disposition
definition above; it is not a builder outcome.

## Before activation

0. **Approved 2026-10-27** (owner, after ChatGPT's review of v0.2.1), in this
   order:
   1. Install the pinned tool and run `check-tool`.
      **Done 2026-10-27:** installed at
      `~/.local/state/work-engine/tools/agent-routing-lab/1faaf78af460afee1126731816e0dc10553d9822/`
      from GitHub. `check-tool` passes on the default path, and a smoke profile in
      a scratch run directory verified.
   2. Complete the separately governed Sol 6.1 supervisor qualification and
      transition. **Reordered by the owner on 2026-10-28:** step 3 goes first,
      and the transition happens whenever its own gates pass (see
      [Measurement environment](#measurement-environment)).
   3. Apply Appendix A to the coordinator guidance in its own commit, then
      release and deploy that commit (`skill_release.py plan`, then deploy).
   4. Run the 10 eligible dispatches under the existing assignment policy, then
      review the observations before expanding collection.

   Not part of this plan: adaptive routing, more benchmark runs, task-aware
   selection.
1. The extractor's canonical home is
   [agent-routing-lab](https://github.com/bline/agent-routing-lab) (Apache-2.0),
   as `taskprofile`. Work Engine pins tag `v0.2.2`, commit
   `373c12f301ba2d7859522e67e506940d6a424ef3` (from 2026-10-28; round 1 ran on
   v0.2.1, `1faaf78a`, whose install stays for `verify`), and does not keep its own copy.
   v0.2.2 records a malformed manifest as a resolution failure instead of crashing.
   Installed 2026-10-28; `check-tool` passes, and re-profiling a round-1 sidecar
   with it reproduced that record's revision ID and validated.
   v0.2.0 (`534cd79d`) is superseded: an unrecognised field status escaped
   validation, and two snapshots could be declared at once.
   v0.1.0 (`54f9b35b`) is superseded: its task-revision ID ignored role and
   snapshot.
   - The pin lives in `task_profile_record.py`.
   - Install the release into
     `~/.local/state/work-engine/tools/agent-routing-lab/<commit>/` with
     `python3 -m venv <dir>` and
     `<dir>/bin/pip install "git+https://github.com/bline/agent-routing-lab@<commit>"`.
   - `task_profile_record.py check-tool` refuses an installed copy from any other
     commit.
   - The pin is recorded in every `overhead.json`.
   - Tags are immutable. A change to a feature definition or extractor behaviour is
     a new version and a new pin.
   - The prototype with Work Engine fixtures stays in `~/cbm-work/task-profile-v0`.
2. Done in agent-routing-lab v0.1.0 to v0.2.1: `validate --production` requires the
   provenance fields above, and `taskprofile manifest` writes a hashed manifest
   for dirty-tree baselines.
3. Records go to the private
   [agent-routing-data](https://github.com/bline/agent-routing-data) repository
   under `observations/work-engine/<task_revision>/`, copied from the run
   directory.
4. Owner approval of this plan and of Appendix A: given 2026-10-27 (step 0).
   Apply Appendix A in one commit and deploy it as one release, so the guidance
   version recorded in each profile is a single hash.
   Appendix A calls `task_profile_record.py`, which runs the pinned installed
   `taskprofile`.

## Out of scope for phase 0

Adaptive or randomized assignment, a separate profiler call (kept for later
agreement audits), CatBoost or any router, retrospective extraction from old
packets, and publishing data.

## Appendix A: coordinator guidance as approved 2026-10-27

To insert in `skills/manual-orchestration/references/coordinator.md`, after the
paragraph that begins "Before dispatch, compare the packet":

> **Task profile (shadow pilot).** For a builder or reviewer dispatch with a write
> scope or review subject:
> 1. Freeze the packet.
> 2. Note the time, then write a stage B sidecar
>    ([template](../../../metrics/dogfood-selection-plan-20261027.md#appendix-b-stage-b-sidecar-template))
>    from the packet and its authoritative sources. Name no model, effort or
>    builder in it. Record unknown fields as unknown; never guess to fill a field.
> 3. Before choosing the model, run
>    `python3 -I metrics/scripts/task_profile_record.py profile --run-dir <run> --sidecar <file> --repo <repo> --declare-started-at <time>`.
>    A validation failure is recorded and does not block dispatch.
> 4. After dispatch, run `task_profile_record.py assign` with the requested and
>    reported configuration, the time you chose the model (`--selected-at`), the
>    dispatched packet and the child's session or thread ID. After the child finishes, run `task_profile_record.py attribute`
>    against the usage ledger.
>
> Never put the sidecar or profile in a child's packet or prompt.

**Changes made when applying it** (approved by the owner on 2026-10-28, "proceed"):
1. **Scope.** "In production work", and the exclusions named: experiment arms,
   scouts, recon without a write scope, administrative operations. The approved
   text said only "with a write scope or review subject", which admits experiment
   arms; [Eligible dispatches](#eligible-dispatches) excludes them.
2. **Template reference.** The link `../../../metrics/...` became the repository
   path `metrics/dogfood-selection-plan-20261027.md`, Appendix B. A deployed
   release holds only `skills/`, so the relative link resolves to nothing from
   the installed copy.
3. **Guidance version.** The sidecar's `guidance_sha256` is the sha256 of
   `<skill-dir>/references/coordinator.md`, the installed copy, as
   [Records](#records) requires. The approved text did not say where it comes from.
4. **One run directory and a stop.** All records go in
   `~/.local/state/work-engine/task-profile-pilot`, and coordinators stop at 10
   entries and tell the owner. The approved text gave no run directory and no stop,
   so the 10 could not be counted across sessions and collection would continue
   past the pilot.
5. **Usage ledger.** The approved text said "against the usage ledger" without
   naming one. `usage_ledger.py build` makes a ledger for a time window, so step 4
   names that command and keeps its output outside the run directory, which is
   copied to the data repository.

6. **Supervisors.** In runs with supervisors, the supervisor launches the
   builders and reviewers, and it reads `supervisor.md`, not `coordinator.md`.
   `supervisor.md` now points to the task-profile step, and a supervisor declares
   itself as `supervisor`. In this plan, "coordinator" means whoever dispatches
   the builder or reviewer.

**Changed after round 1 (2026-10-28, owner: "proceed with fixes";
[review](probes/dogfood-pilot-round1-review-20261028.md)):**
7. **Rounds.** The guidance profiles only while the Status lines name an active
   round, its run directory and cap, so later rounds change this plan, not the
   guidance.
8. **The model doing the work.** An adapter or launcher is not profiled; the
   dispatch of the model it starts is, once.
9. **Hygiene.** Sidecars go in `<run>/sidecars/`; packets and files a child is
   told to read never mention the pilot, a profile or a revision ID.
10. **Git snapshots only**, the restriction the first text omitted.
11. **Times.** `task_profile_record.py start` stamps the declaration start, and
    `--selected-at` is taken to the microsecond, never copied.
12. **Harness format** `<cli>/<version>;tier=<tier>`.

`task_profile_record.py` also archives sources in failure records and reports
pilot mentions and the revision hash, case-insensitively.

Both commands' usage lines are generated blocks (`cli_synopsis.py`), so a
changed argument fails `skill_release.py check`.

`task_profile_record.py` now also records its own path and sha256 in every
record (`recorder`): coordinators run it from their checkout, so a change to it
during the pilot would otherwise go unrecorded.

**Changed 2026-10-29 (owner: "remove orchestrator role. make supervisor the top
level role"):**
13. **One top-level role.** The coordinator guidance moved into `supervisor.md`,
    and `coordinator.md` was removed. The sidecar's `guidance_sha256` is now the
    sha256 of `<skill-dir>/references/supervisor.md`, and the declaring role is
    `supervisor`. Earlier records hash `coordinator.md`, so the new value marks
    the switch.

**Changed 2026-10-30 (owner adopted the
[screen-first amendment](dogfood-round2-amendment-proposal-20261029.md)):**
14. **Screen first.**
    - Builder dispatches are screened with `enroll --packet` before any
      profile; exit 4 (`profile_required`) is the only path to a profile.
    - Later dispatches in an episode use `assign --episode`.
    - Dispatches outside the comparison, reviewers included, are not profiled.
    - Accounting is `attribute --all` at check-ins and round close.

## Appendix B: stage B sidecar template

Each declared field is
`{"status": "declared" | "none" | "not_applicable" | "unknown", "value": ..., "basis": "..."}`.
`value` is present only when `status` is `declared`. `basis` says where the
value came from (for example "packet section Acceptance" or "contract.scope").

```json
{
 "schema": "task-profile/v0",
 "stage": "B",
 "identity": {
  "task_id": "<stable task name>",
  "target_role": "builder",
  "parent_task_id": {"status": "unknown"},
  "work_type": {"status": "declared", "value": "implementation"},
  "authority_state": {"status": "declared", "value": "approved"}
 },
 "input_snapshot": {
  "git": {"status": "declared", "value": {"commit": "<40-hex>"}},
  "manifest": {"status": "not_applicable"},
  "authority_refs": {"status": "declared", "value": [{"kind": "contract", "path": "<abs path>", "sha256": "<hex>"}]}
 },
 "declared": {
  "objective": {"status": "declared", "value": "<one sentence>"},
  "read_set": {"status": "declared", "value": [{"text": "<item>", "path": "<repo path or glob>"}]},
  "expected_write_set": {"status": "declared", "value": ["<repo path or glob>"]},
  "interfaces": {"status": "declared", "value": ["<public interface changed>"]},
  "exclusions": {"status": "declared", "value": ["<out of scope>"]},
  "verification": {"status": "declared", "value": [{"kind": "test", "text": "<command or check>"}]},
  "uncertainties": {"status": "none"},
  "dependencies": {"status": "declared", "value": ["<external service or repository>"]},
  "obligations": {
   "acceptance": {"status": "declared", "value": ["<acceptance obligation>"]},
   "state": {"status": "unknown"},
   "persistence": {"status": "declared", "value": ["<schema or compatibility requirement>"]},
   "recovery": {"status": "declared", "value": ["<retry, rollback, cancellation or recovery obligation>"]},
   "compatibility": {"status": "none"}
  }
 },
 "provenance": {
  "adapter": {"name": "sidecar", "version": "0.1.0"},
  "declared_by": {"status": "declared", "value": {"role": "coordinator", "model": "<id>", "effort": "<effort>", "harness": "<name and version>", "guidance_sha256": "<hex>"}},
  "ignored_treatment_fields": [],
  "source_documents": [{"path": "<frozen packet>", "sha256": "<hex>", "bytes": 0}]
 }
}
```

Field meanings (added after round 1, whose spot-check found the obligation
fields used for other things). Declare a field only from the packet and its
authority documents; anything else is unknown.
- `read_set`: every document or path the packet tells the child to read, one item
  each, not a summary.
- `obligations.acceptance`: what must be true or delivered for the work to be
  accepted, deliverables and evidence included.
- `obligations.state`: runtime or shared state the change must keep consistent:
  concurrency, locking, ownership of mutable state. Not provenance or identities.
- `obligations.persistence`: stored data the change touches: schemas, file or
  wire formats, migrations, on-disk records. Not result files the child writes.
- `obligations.recovery`: required behaviour on failure: retry, rollback,
  cancellation, restart, fail-closed.
- `obligations.compatibility`: external interfaces or versions that must keep
  working.

Enumerations, from `taskprofile.py`:
- `work_type`: implementation, planning, investigation, qualification, review,
  authority-preparation, documentation, other.
- `authority_state`: approved, proposed-not-approved, preparation-only, other.
- `verification.kind`: build, test, lint, format, fixture-oracle,
  independent-review, manual, other.
- `authority_refs.kind`: authority, contract, proposal, clarification, plan,
  guidance, other.
