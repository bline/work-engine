# Task-profile comparison contract

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: the active assignment is governed by the task-profile selection pilot; before admission, screening, model choice or episode accounting.

**Task profile (shadow pilot).** Before screening, run `python3 -I metrics/scripts/task_profile_record.py admission --run-dir <run>`. The plan [`metrics/dogfood-selection-plan-20261027.md`](../../../metrics/dogfood-selection-plan-20261027.md) supplies only the round's run-directory path (`<run>`); its Status lines are history, not state, and the gate reads the run directory alone. Screen only on exit 0. On any other result, including a usage error or a crash, do not screen, enroll or record; on `stop_reached` or `unknown`, tell the owner. Round 2 follows its [screen-first amendment](../../../metrics/dogfood-round2-amendment-proposal-20261029.md): only builder tasks that enter the comparison are profiled, and other dispatches, reviewers included, are not. A production builder dispatch with a write scope is screened; experiment arms, scouts, recon without a write scope and administrative operations are not. When an adapter or launcher starts the model that does the work, screen that model's dispatch once, not the adapter's.

1. **Freeze the packet.** Mention no pilot, profile or revision ID in it, or in any file the child is told to read.
2. **Screen** with `task_profile_record.py enroll --task-id T --packet P --eligible yes|no --reason R`. Your eligibility decision: both arms are acceptable under [model selection](../../../docs/model-selection.md), and the round's review route fits the task.
   - A not-eligible, paused or in-flight-limited screening is recorded there. Dispatch as usual, with no sidecar, profile or accounting.
   - Exit 4 (`profile_required`) means the task is eligible and the round can enroll it. Nothing is recorded yet.
3. **Only then, profile and enroll:**
   - Run `task_profile_record.py start` and keep the time it prints.
   - Write a stage B sidecar from the packet and its authoritative sources, using the template in the plan's Appendix B, under `<run>/sidecars/`: never next to the packet or where a child reads.
     - Declare a git commit (`input_snapshot.git`, with `manifest` not applicable) only when the task's inputs are in this repository at that commit. When they are elsewhere, such as a private variant or candidate repository, set both to unknown with that basis.
     - Name no model, effort or builder. Record unknown fields as unknown; never guess to fill a field.
     - Declare your role as `supervisor`. Its `guidance_sha256` is the `sha256sum` of `<skill-dir>/references/supervisor.md`, where `<skill-dir>` is this skill's directory.
   - Before choosing the model, run `task_profile_record.py profile` from the repository root with `--declare-started-at` set to the start time. A validation failure is recorded and does not block dispatch.
   - Then run `enroll --task-id T --task-revision REV --eligible yes --reason R`, and dispatch the arm it prints. If it reports the task not enrolled, choose as usual. If `enroll` fails or its result is unclear, run it again for the same task ID; only `status --task-id` showing no record makes the task not enrolled.
4. **Record the dispatch.** Run `task_profile_record.py assign --task-revision REV` with:
   - the requested and reported configuration;
   - `--harness` as `<cli>/<version>;tier=<tier>`, such as `codex/0.164.0;tier=priority` or `claude-code/2.1.312;tier=standard`;
   - `--selected-at`, the moment you chose the model, to the microsecond (`start` prints one; never copy another timestamp);
   - the dispatched packet, and the child's session or thread ID.

   Every later dispatch in the episode (continuations, repairs, revised packets and its reviewers) runs `assign --episode T` instead, and is not profiled.
5. **Accounting is not per dispatch.** At each owner check-in and at round close, build one usage ledger for the round's window (`usage_ledger.py build`, `--out` a directory outside `<run>`), then run `task_profile_record.py attribute --all --ledger <dir>`.

**During an episode,** run:
- `review` when the first review completes;
- `repair` at each resubmission after a review that required changes;
- `pause` at once on any P1 against a Terra-arm build;
- `close` at the end (`withdraw` if it was never dispatched).

Do not tell the reviewer of an enrolled episode which model built it. Ask it for the plan's standard verdict with P1–P3 severities, and put the definitions in its packet, since its own scale may differ. P1 blocks acceptance, P2 must be fixed before acceptance, and P3 is an improvement. `accept` allows P3 findings only; `changes_required` needs at least one P1 or P2; `blocked_evidence` means it cannot judge because evidence the packet should have supplied is missing; `blocked_scope` means it cannot judge because the subject or packet scope is wrong. Record them with `review --verdict` and the reviewer's `--reviewer-session`.

<!-- cli-synopsis script=metrics/scripts/task_profile_record.py subs="start profile assign attribute admission enroll review repair close pause status" display="python3 -I metrics/scripts/task_profile_record.py" -->
```text
python3 -I metrics/scripts/task_profile_record.py start
python3 -I metrics/scripts/task_profile_record.py profile --run-dir RUN_DIR --sidecar SIDECAR --repo REPO --declare-started-at DECLARE_STARTED_AT [--call-id CALL_ID] [--tool-dir TOOL_DIR]
python3 -I metrics/scripts/task_profile_record.py assign --run-dir RUN_DIR (--task-revision TASK_REVISION | --episode EPISODE) --role {builder,reviewer} --requested-model REQUESTED_MODEL --requested-effort REQUESTED_EFFORT --harness HARNESS [--reported-model REPORTED_MODEL] [--reported-effort REPORTED_EFFORT] --run RUN --dispatched-packet DISPATCHED_PACKET --child-session CHILD_SESSION [--selected-at SELECTED_AT] [--retained]
python3 -I metrics/scripts/task_profile_record.py attribute --run-dir RUN_DIR (--task-revision TASK_REVISION | --all) --ledger LEDGER [--assignment ASSIGNMENT] [--since SINCE] [--until UNTIL]
python3 -I metrics/scripts/task_profile_record.py admission --run-dir RUN_DIR
python3 -I metrics/scripts/task_profile_record.py enroll --run-dir RUN_DIR --task-id TASK_ID --reason REASON (--task-revision TASK_REVISION | --packet PACKET) --eligible {yes,no}
python3 -I metrics/scripts/task_profile_record.py review --run-dir RUN_DIR --task-id TASK_ID --verdict {accept,changes_required,blocked_evidence,blocked_scope} --reviewer-session REVIEWER_SESSION --p1 P1 --p2 P2 --p3 P3 --receipt RECEIPT
python3 -I metrics/scripts/task_profile_record.py repair --run-dir RUN_DIR --task-id TASK_ID --review-receipt REVIEW_RECEIPT
python3 -I metrics/scripts/task_profile_record.py close --run-dir RUN_DIR --task-id TASK_ID --reason REASON --disposition {accepted,abandoned} --pre-review-fixes PRE_REVIEW_FIXES --escalations ESCALATIONS [--review-receipt REVIEW_RECEIPT] [--artifact ARTIFACT]
python3 -I metrics/scripts/task_profile_record.py pause --run-dir RUN_DIR --reason REASON
python3 -I metrics/scripts/task_profile_record.py status --run-dir RUN_DIR [--task-id TASK_ID]
# other subcommands (put --help after one for its usage): check-tool, verify, init-round, recover-screening, resume, withdraw
```
<!-- /cli-synopsis -->
<!-- cli-synopsis script=metrics/scripts/usage_ledger.py subs="build" display="python3 -I metrics/scripts/usage_ledger.py" -->
```text
python3 -I metrics/scripts/usage_ledger.py build [--since SINCE] [--until UNTIL] --out OUT [--no-codex] [--no-claude] [--streams [STREAMS ...]] [--no-pi] [--pi-state PI_STATE]
# other subcommands (put --help after one for its usage): reconcile-trial, windows, tree, control, pi-admit
```
<!-- /cli-synopsis -->

Never put the sidecar or profile in a child's packet or prompt.
