# Dogfood selection plan, phase 0: shadow-mode task profiles (2026-10-27)

Status: **draft for owner review. Not active.** Nothing here changes dispatch until
the owner approves it. The guidance text in Appendix A has not been applied to
`skills/`: production loads skills from the live tree, so editing them would
activate the change.

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
| Run the existing script `taskprofile.py` (no model call of its own) | Add any model call. The coordinator writes the declaration in its own turn |

Model and effort keep coming from the active user selection
([model selection](../docs/model-selection.md)), as
[coordinator guidance](../skills/manual-orchestration/references/coordinator.md#dispatch-and-the-retained-or-fresh-choice)
already requires. That guidance's "learn from outcomes without creating a new
gate" applies here as well.

## Sequence at each eligible dispatch

1. **Freeze the plan.** The accepted packet and its authoritative documents are
   final. Record their sha256.
2. **Declare.** The coordinator writes the stage B sidecar (Appendix B) from the
   frozen packet and its sources. It names no builder, reviewer, model or effort.
3. **Compute and freeze.** Run `taskprofile.py compute` then `validate`. Write the
   profile once. On failure, write a failure record with the errors, then go on
   to step 4.
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
packet documents and snapshot identity; `taskprofile` computes it (v0.2.0). Each record is written once. A correction is a new file that names
the one it supersedes.

Layout under the run directory:

```
task-profiles/<task_revision>/
  sidecar-stageB.json        declaration as written by the coordinator
  profile-stageB.json        compute output (profile-failed.json if compute failed)
  validation.json            validate --production result; failures are recorded, never blocking
  assignment.json            the assignment observation (retained continuations add assignment-02.json, ...)
  overhead.json              instrumentation cost and the tool pin
  attribution-<assignment>.json   models actually used, from the usage ledger
  SHA256SUMS                 append-only index
```

`metrics/scripts/task_profile_record.py` writes these records, never overwriting
them:
- `profile` writes the sidecar, profile, validation and overhead records.
- `assign` writes the assignment observation. It requires the child's session or
  thread ID, and checks that the dispatched packet contains none of the profile's
  identifiers.
- `attribute` resolves the models and efforts the child sessions actually ran,
  from the usage ledger, and flags any substitution.
- `verify` checks that the hash list exists, that every file is listed and
  matches, and that each record's `prior_files` hashes still match. It then
  recomputes each profile with the tool for that record's own pin. Exit 0 means
  fully verified, 2 means hashes pass but no recompute was done, 1 means a
  problem.

Integrity covers accidental change, not an adversary. The hash list is
append-only, and every record after the first also stores the hashes of the
files written before it. The data repository's git history is the
tamper-evident copy.

Tests: `metrics/scripts/tests/test_task_profile_record.py`, 11 cases. They cover
the full cycle, model substitution, an unknown child, and builder and reviewer
revisions. They also cover each integrity failure that v0.1.0 missed: a missing
hash list, an unlisted file, an edit hidden by a rewritten hash list, a skipped
recompute, an invalid revision ID, and a leak into the dispatch packet.

- **Profile** (`profile-stageB.json`): the task-profile v0 output, which
  carries identity, input snapshot, declarations, computed features, coverage
  and provenance. `provenance.declared_by` must give the coordinator's role,
  model, effort, harness and version, plus the guidance version (the sha256 of
  the coordinator guidance file used).
- **Assignment observation** (`assignment.json`), kept separate from the profile
  so the selected configuration never becomes part of the task's description:

  | Field | Value |
  |---|---|
  | `task_revision` | join key |
  | `profile_sha256` | hash of the profile, or of the failure record |
  | `profile_frozen_before_assignment` | true, or false with a reason |
  | `role` | builder or reviewer |
  | `requested` | model, effort, harness |
  | `reported` | model and effort as the coordinator reports them, marked `source: coordinator` |
  | `child_sessions` | the child's Claude session or Codex thread IDs; attribution uses these |
  | `assigned_at` | timestamp |
  | `run` | run registry name |
  | `retained_continuation` | true when it continues an existing assignment on the same revision |

- **Overhead** (`overhead.json`):
  - start and freeze timestamps for the declaration;
  - coordinator call IDs in that interval, so the usage ledger can price them;
  - validation errors;
  - the count of unknown declared fields;
  - any re-declaration.

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

- **The supervisor change at the next Codex reset is an environment change.** The
  Astra to `gpt-6.1-sol` xhigh switch (window reset 11-01 per the experiment plan)
  changes who writes declarations. Profiles carry the coordinator's identity.
  Recommendation: run the pilot entirely after the switch, so all 10 share one
  coordinator. The switch has its own gates (supervisor decision replay O2, then
  a monitored pilot) and doesn't wait for this plan.
- **Introducing the sidecar is an environment change too,** under the experiment
  plan's re-pin rule. Asking for declarations may itself change how plans are
  written. Records from before and after are not compared directly.

## Pilot: the next 10 eligible dispatches

| Criterion | Check |
|---|---|
| Every eligible dispatch has a profile record | 10 of 10 have `profile-stageB.json` or `profile-failed.json` |
| No profile was frozen after its assignment | `profile_frozen_before_assignment` true in 10 of 10, or reasons recorded |
| Profiling changed nothing | No dispatched packet or prompt contains the sidecar or profile (search the packet and prompt text for the `task_revision` and field names); assignments follow the active model selection |
| Snapshots verify | Re-running `compute` on each sidecar later gives identical bytes; snapshot is a git commit or a verified manifest, otherwise reported as unknown |
| Declarations vary | Report, per field, how often it is declared and how many distinct values occur. No pass threshold is set in advance; the review decides whether a field is useful |
| Overhead measured | Median and maximum declaration wall time, coordinator tokens and API-equivalent cost per profile, and validation failure rate |

After the 10, the owner reviews the profiles and the discrepancies. Possible
outcomes: make collection permanent, revise the schema, or stop.

## Before activation

1. The extractor's canonical home is
   [agent-routing-lab](https://github.com/bline/agent-routing-lab) (Apache-2.0),
   as `taskprofile`. Work Engine pins tag `v0.2.0`, commit
   `534cd79d6c3e489bacfce2ad684187f7d3e8af85`, and does not keep its own copy.
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
2. Done in agent-routing-lab v0.1.0 and v0.2.0: `validate --production` requires the
   provenance fields above, and `taskprofile manifest` writes a hashed manifest
   for dirty-tree baselines.
3. Records go to the private
   [agent-routing-data](https://github.com/bline/agent-routing-data) repository
   under `observations/work-engine/<task_revision>/`, copied from the run
   directory.
4. Owner approval of this plan and of Appendix A. Then apply Appendix A in one
   commit, so the guidance version recorded in each profile is a single hash.
   Appendix A's script path changes to the installed `taskprofile` command.

## Out of scope for phase 0

Adaptive or randomized assignment, a separate profiler call (kept for later
agreement audits), CatBoost or any router, retrospective extraction from old
packets, and publishing data.

## Appendix A: proposed coordinator guidance (not applied)

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
>    reported configuration, the dispatched packet and the child's session or
>    thread ID. After the child finishes, run `task_profile_record.py attribute`
>    against the usage ledger.
>
> Never put the sidecar or profile in a child's packet or prompt.

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

Enumerations, from `taskprofile.py`:
- `work_type`: implementation, planning, investigation, qualification, review,
  authority-preparation, documentation, other.
- `authority_state`: approved, proposed-not-approved, preparation-only, other.
- `verification.kind`: build, test, lint, format, fixture-oracle,
  independent-review, manual, other.
- `authority_refs.kind`: authority, contract, proposal, clarification, plan,
  guidance, other.
