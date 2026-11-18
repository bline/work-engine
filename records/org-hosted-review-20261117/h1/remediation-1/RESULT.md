# H1 remediation 1: result

Input: root instruction "H1 remediation 1" against Fable review `h1/review/REVIEW.md` (verdict
`changes_required`, read in full). Same worktree/crate/constraints as `h1/PACKET.md`; nothing
committed; no other crate edited (`runs/main-1/diffstat.stdout`: tracked changes still only
`rust/Cargo.lock` +16, `rust/Cargo.toml` +1; no new files, 19 crate files edited in place).

## Changes mapped to findings

| Finding | Change |
|---|---|
| P2-1 (a) | `secret-assignment` now has 4 patterns: quoted literal with optional type annotation (`NAME: &str = "…"`) and whitespace allowed inside; unquoted token-shaped value to end of line (≥ 16 of `[A-Za-z0-9+/=_.-]`, a letter and a digit, non-trivial); key/value call/header form (`"x-api-key", "…"`, `authorization`, `x-auth-token`, …); `Bearer\s+[A-Za-z0-9._~+/-]{20,}`. The typed-declaration guard now only keeps forms with no literal (`token: String,`, `let api_key: Option<String> = None;`, `password: Option<String>,`, `let token = parse_token(&input)?;`). |
| P2-1 (b) | `credential-token-prefix` adds JWT `eyJ….eyJ`, `sk_live_`/`rk_live_`, `ASIA`, `hf_`, `xapp-`, `xoxe`, `AGE-SECRET-KEY-1`, `PuTTY-User-Key-File-`. |
| P2-1 (c) | New rule `sensitive-member-path` (basename, case-insensitive): `.env*`, `*.pem`, `*.key`, `*.p12`, `id_rsa*`, `id_ed25519*`, `.netrc`, `.npmrc`, `auth.json`, `credentials*`; reported with line 0. |
| P2-1 (d) | `private-path` adds `/var/lib/work-engine`; the tail is now `(?:[^A-Za-z0-9_]|$)`, so `~/.codex-api`, `~/.claude.json` and `~/.pi.` match. |
| P2-1 (e) | New rule `binary-patch`: a `GIT binary patch` line (e.g. a deleted binary file's base85 in `.review/patch.diff`). |
| P2-1 | Rule set id now `work-engine.review-hosted-secret-rules.v2`; 7 rules. Tests: `subject::v2_alternatives_each_hit` (one positive per added alternative: 9 prefixes, 7 assignment forms, 4 path forms, binary patch; 12 member-path patterns), `subject::sensitive_member_and_deleted_binary_are_refused_by_materialize` (synthetic repos), negatives extended. |
| P2-2 | `check` takes `RunArtifacts` (bin: `--run-inputs <base>/inputs --job <base>/job.json`; `load_run`) and adds four failing checks: `snapshot_served` (`inputs/snapshot-manifest.json`, written at `analyze.py:64` in the inventory shape of `:31-37`, has the SHA-256 that `job.json` `workspace.manifest` pins at `:102`, and as path→sha256 equals `record.snapshot_manifest`); `subject_bound_to_scope` (`subject.patch_identity` = recomputed preparation `scope_digest` = served manifest digest of `.review/patch.diff`); `job_id_bound` (`record.job_id` = `job.json` `job_id`, `:99`, and `authority.json` `job_id`, `:91`; derived from the run name at `:88`); `limits_bound` (`record.limits` = `packet.json` `max_provider_requests`/`max_snapshot_reads`, `:96-97`, env defaults `:23-24`; `authority.json` `task_sha256`, `:94`, = `record.task_sha256`). Grounding is counted against the served manifest (`NotRun` if unreadable). Tests: `check::served_snapshot_must_be_the_vetted_one_and_pinned_by_the_job`, `subject_must_be_bound_to_scope_and_served_patch`, `job_id_and_limits_must_be_the_runs`, `run_directory_loads_result_ledger_bodies_and_job_artifacts`, `grounded_evidence_is_counted_against_the_served_manifest`. Consequence for the root: `review-job-inputs --job-id` must be `dev-vertical-hosted-analysis-<run name>`. |
| P2-3 | `delivery` requires each body's `model` = `gpt-6-luna` and its declared tool names (top-level `tools` plus `additional_tools` items, the Host's reading at `pi_coding_hosted.rs:31-45`) = exactly `snapshot_list`, `snapshot_read`. Test: `check::delivery_refuses_another_model_or_tool_set` (4 variants). |
| P3-1 | `AnalysisResult` reads `settlement`, `provider_requests`, `snapshot_deliveries`, `hosted_attempts`; the envelope carries them as `host_reported` (not checked). Envelope field `provider` renamed `requested_provider`; schema `…envelope.v2`; `NOT_CLAIMED` adds the serving-route line. CLAIMS.md claim 2 reworded (provider id vs serving endpoint; route/payer Host-reported, `analyze.py:73-76`). Test: `check::host_reported_fields_are_carried_and_unknowns_are_null`. |
| P3-2 | `undischarged_blockers`, `world.run_id`, `frozen_world.known_incompatibilities` are `Option` (null when not established). Same test. |
| P3-3 | Production `git` removes `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_OBJECT_DIRECTORY`, `GIT_ALTERNATE_OBJECT_DIRECTORIES` and sets `GIT_NO_REPLACE_OBJECTS=1` (`subject::GIT_REDIRECTS`); test support `git_command()` removes the same five. Not separately tested. |
| P3-4 | `check_members` refuses empty, `.`, `..` and `.git` components. Test: `subject::unsafe_member_path_components_are_refused`. |
| P3-5 | Made true: `inputs::write` → `write_bundle` stages in a sibling `.<name>.staging-<pid>` directory and renames it into place after every write; on failure removes only the staging directory it created. `lib.rs` sentence narrowed accordingly (a crash mid-write can leave the staging directory). Test: `subject::bundle_writes_atomically_and_leaves_nothing_on_failure` (success path + a mid-write I/O failure). |
| P3-6 | `check_read_budget`: refuse when 1 (`snapshot_list`, a Host delivery counted by the read limit, `pi_coding_analysis.rs:10`) + ⌈len(`.review/patch.diff`)/24 576⌉ > `max_reads`; record `snapshot_read_pages` (sum over all members) in `JobInputs`. Called in `inputs::build`. Test: `subject::read_budget_counts_snapshot_list_and_patch_pages`. Counting the list is stricter than the review's formula. |
| P3-7 | `delivery` also requires user text length = task + 202 (`HOST_SUFFIX_BYTES`); no suffix text copied. Test: `check::delivery_refuses_extra_text_after_the_task_and_suffix`. |
| P3-8 | 15 recorded mutants (below), covering context-fit bound, prompt bound, tree mismatch, subject equality, a new secret rule (JWT, plus member path and binary patch), and the new checks. |
| P3-9 | CLAIMS.md claim 5 names the counter: the Host's `tool_requests` set must be empty at completion (`pi_coding_analysis.rs:296`, read); snapshot reads are Host deliveries. Grounding (claim 4) now against the served manifest; new claim 3 for the run binding. |

## Runs

- `h1/remediation-1/runs/main-1/` (plan `plans/main.json`, same shape as `h1/plans/main.json`, hashes
  extended with the remediation `mutate.py`): summary sha256 `24b4fc3e…426f`, **9/9 passed** — fmt,
  clippy `-D warnings`, `test-hosted` 45 passed (check 18, revision 6, subject 17, task 4),
  preparation 23 + 1, delivery 8 + 1, organization 97 + 4 + 4 + 45 + 6, hashes, diffstat, status.
- `h1/remediation-1/mutants/runs-1/` (plan `plans/mutants.json`): summary sha256 `711f6657…4f31`;
  "failed 32/47" in the summary line counts **passed** steps (15 apply + 15 restore + restored.test
  + restored.hashes = 32), the 15 failures being the killed `.test` steps.

**Correction to `h1/RESULT.md`:** it says of `mutants-1` "The summary says 'failed 20/29' because the
9 killed test steps and their `--keep-going` accounting count as failures". Wrong: 20 is the passed
count (9 apply + 9 restore + 2). Error class: interpreting a field without reading its siblings
(CLAUDE.md rule 4). Consequence: none on the conclusion (the 9 kills and restores stand), but the
stated reason was wrong. Check that would have caught it: the arithmetic against the plan's 29
steps, done here.

## Mutants (15/15 killed; each by a test failure, no `error[` in any `.test` log; every restore
verified per file; `restored.hashes` equal `runs/main-1/hashes` for check, subject, secrets, task,
inputs)

| Mutant | Killed by |
|---|---|
| R1-M1-context-fit-slack (`* 4 + 5`) | subject::request_context_bound_is_snapshot_times_one_and_a_quarter_plus_task |
| R1-M2-prompt-bound-slack (`+ 1`) | task::task_over_the_prompt_bound_is_refused |
| R1-M3-tree-unchecked | subject::tree_mismatch_is_refused |
| R1-M4-subject-equality-unchecked | check::subject_mismatch_is_refused |
| R1-M5-jwt-rule-never-matches | subject::v2_alternatives_each_hit |
| R1-M6-snapshot-served-unchecked (`&&` → `||`) | check::served_snapshot_must_be_the_vetted_one_and_pinned_by_the_job |
| R1-M7-body-model-unchecked | check::delivery_refuses_another_model_or_tool_set |
| R1-M8-user-text-length-unchecked | check::delivery_refuses_extra_text_after_the_task_and_suffix |
| R1-M9-read-budget-slack (`+ 1`) | subject::read_budget_counts_snapshot_list_and_patch_pages |
| R1-M10-env-member-path-missed | subject::v2_alternatives_each_hit, sensitive_member_and_deleted_binary_are_refused_by_materialize |
| R1-M11-binary-patch-rule-never-matches | same two |
| R1-M12-dotdot-component-allowed | subject::unsafe_member_path_components_are_refused |
| R1-M13-job-id-unchecked | check::job_id_and_limits_must_be_the_runs |
| R1-M14-attempt-digest-unchecked | check::delivery_refuses_attempt_digest_mismatch_extra_user_text_and_missing_body |
| R1-M15-staging-not-removed | subject::bundle_writes_atomically_and_leaves_nothing_on_failure |

Still without a recorded mutant (tests exist): parent shape, answer digest, member count,
`limits_bound`, `subject_bound_to_scope`, the tool-name part of P2-3, PEM/email/private-path
alternatives beyond the ones listed. n = 1 mutant per listed property.

## Carried (not established / not done)

- The real retained body shape vs `user_texts`/`tool_names`, the `pi-state.json` shape beyond
  `analyze.py:139/146`, and the real `snapshot-manifest.json`/`packet.json`/`authority.json`/`job.json`
  against `load_run`: synthetic only. The review's recommendation stands: run one offline
  `analyze.py` scenario through `review-result-check` before any live request.
- Whether the real subject `96808de3…` exists, fits, and passes rule set v2 (root H0). v2 is broader;
  a false-positive refusal on that commit is more likely than with v1 (`unverified`).
- P3-3 has no dedicated test (setting `GIT_DIR` in a test process would affect parallel tests).
- Regex behaviour of v2 is established by the listed tests only; no corpus run.
- Live-route availability (board 5866: quota exhausted until 2026-11-21) unchanged.
- Context reading `usage_unavailable` (no binding); cost of this remediation not observable by me.

`map_ready` /home/bline/cbm-work/org-hosted-review-20261117/h1/remediation-1/RESULT.md
