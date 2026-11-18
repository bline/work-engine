# H1 matrix: outcome → tests → mutants

Crate `rust/crates/work-engine-review-hosted` (worktree `wt-org-hosted-review-20261117`, uncommitted on
`700f45e3`). Tests: `runs/main-1/test-hosted.*` (34 pass: check 12, revision 6, subject 12, task 4).
Mutants: `runs/mutants-1/` (custody summary sha256 `c4fb979d…`), applied and restored by
`mutants/mutate.py`; each `<id>.test` step exit 101 with the listed tests failing (no compile
error in any mutant log); restore verified by SHA-256 per file and by `restored.hashes` equal to
`main-1/hashes`; `restored.test` passes.

| Outcome | Tests (file::name) |
|---|---|
| 1 Requirements revision (D) | revision::revision_is_the_frozen_requirements_with_exactly_the_authorized_changes (exact set of 5 differing JSON paths; frozen bytes pinned `70853189…`; DERIVATION.md derivation reproduces the revision; proposition content); revision::revision_compiles_with_the_unchanged_compiler |
| 2 Preparation and refusal (P1-2) | revision::hosted_claim_states_what_the_host_enforces; revision::frozen_candidate_with_the_hosted_claim_is_refused (2 `provider_family` known incompatibilities; confinement unestablished); revision::frozen_candidate_writes_no_job_inputs (no output dir); revision::revision_world_prepares_and_freezes_with_every_blocker_kept (18 blockers, all `unestablished`; mutation/capabilities `unestablished`; deterministic contract) |
| 3 Task rendering | task::template_is_digest_pinned; task::task_is_template_then_contract_verbatim_and_fits_the_prompt_bound; task::task_over_the_prompt_bound_is_refused (exact boundary); task::limits_outside_the_host_bounds_are_refused |
| 4 Subject materialization (D3/OQ-2) | subject::identity_ok_snapshot_is_changed_files_patch_and_subject (P3-4: manifest[patch] = patchIdentity); subject::tree_mismatch_is_refused; subject::patch_identity_mismatch_is_refused; subject::scope_digest_not_the_patch_identity_is_refused; subject::job_inputs_bind_the_revision_scope_digest_to_the_subject; subject::merge_and_root_commits_are_refused; subject::binary_and_non_utf8_members_are_refused; subject::oversize_snapshot_is_refused (bytes, member count, path length); subject::request_context_bound_is_snapshot_times_one_and_a_quarter_plus_task; subject::each_secret_rule_refuses_with_rule_and_path_and_no_content (all 5 rules); subject::rule_negatives_stay_quiet; subject::job_inputs_binary_refuses_and_writes_nothing |
| 5 Result check + envelope | check::acceptable_fixture_is_accepted_and_ungrounded_evidence_is_flagged; check::grounded_evidence_is_counted; check::remediation_fixture_is_accepted; check::fenced_answer_is_refused_without_repair; check::subject_mismatch_is_refused; check::answer_sha_mismatch_is_refused; check::schema_violation_is_refused; check::contract_digest_mismatch_is_refused; check::delivery_prefix_mismatch_is_refused; check::delivery_refuses_attempt_digest_mismatch_extra_user_text_and_missing_body; check::state_directory_loads_result_ledger_and_bodies; check::result_check_binary_writes_the_envelope_and_exits_by_outcome |
| 6 No overclaim | `CLAIMS.md`, crate doc (`src/lib.rs`), `check::NOT_CLAIMED` carried in every envelope (asserted non-empty by `assert_fixed_standing`) |

| Mutant | Edit | Killed by |
|---|---|---|
| H1-M1-known-incompatibility-not-refused | `revision.rs`: `if !known.is_empty()` → `if false && …` | revision::frozen_candidate_with_the_hosted_claim_is_refused, frozen_candidate_writes_no_job_inputs; all 12 check tests (frozen world no longer refused); subject::job_inputs_* (2) |
| H1-M2-patch-identity-unchecked | `subject.rs`: patchIdentity comparison disabled | subject::patch_identity_mismatch_is_refused |
| H1-M3-scope-digest-unbound | `subject.rs`: scope.digest binding disabled | subject::scope_digest_not_the_patch_identity_is_refused, job_inputs_bind_the_revision_scope_digest_to_the_subject, job_inputs_binary_refuses_and_writes_nothing |
| H1-M4-size-bound-quadrupled | `subject.rs`: `> SNAPSHOT_BYTES` → `> SNAPSHOT_BYTES * 4` | subject::oversize_snapshot_is_refused |
| H1-M5-pem-rule-never-matches | `secrets.rs`: `PRIVATE KEY` → `PRIVATE KEYS` | subject::each_secret_rule_refuses_with_rule_and_path_and_no_content |
| H1-M6-fence-stripped-before-parse | `check.rs`: trim + strip ```` ```json ```` fence before `parse_json` | check::fenced_answer_is_refused_without_repair |
| H1-M7-contract-digest-not-compared | `check.rs`: contract digest comparison `|| true` | check::contract_digest_mismatch_is_refused |
| H1-M8-delivery-prefix-unchecked | `check.rs`: `starts_with(task)` check disabled | check::delivery_prefix_mismatch_is_refused |
| H1-M9-envelope-consumable | `check.rs`: `consumable: false` → `true` | check::acceptable_fixture_*, remediation_fixture_is_accepted, fenced_answer_is_refused_without_repair, result_check_binary_* |

9 of 9 recorded mutants killed (n = 1 mutant per listed property; other properties have tests but
no recorded mutant).

## Remediation 1

Against `h1/review/REVIEW.md`; details and finding map in `h1/remediation-1/RESULT.md`. Tests:
`remediation-1/runs/main-1/test-hosted.*`, 45 pass (check 18, revision 6, subject 17, task 4).
Mutants: `remediation-1/mutants/runs-1/`, 15/15 killed, restores verified, restored hashes equal
`remediation-1/runs/main-1/hashes`.

| Finding | Tests (file::name) |
|---|---|
| P2-1 (a)–(e) rule set v2 | subject::v2_alternatives_each_hit; subject::sensitive_member_and_deleted_binary_are_refused_by_materialize; subject::each_secret_rule_refuses_with_rule_and_path_and_no_content; subject::rule_negatives_stay_quiet |
| P2-2 run binding | check::served_snapshot_must_be_the_vetted_one_and_pinned_by_the_job; check::subject_must_be_bound_to_scope_and_served_patch; check::job_id_and_limits_must_be_the_runs; check::grounded_evidence_is_counted_against_the_served_manifest; check::run_directory_loads_result_ledger_bodies_and_job_artifacts; check::result_check_binary_writes_the_envelope_and_exits_by_outcome |
| P2-3 body model/tools | check::delivery_refuses_another_model_or_tool_set |
| P3-1, P3-2 | check::host_reported_fields_are_carried_and_unknowns_are_null |
| P3-4 | subject::unsafe_member_path_components_are_refused |
| P3-5 | subject::bundle_writes_atomically_and_leaves_nothing_on_failure |
| P3-6 | subject::read_budget_counts_snapshot_list_and_patch_pages |
| P3-7 | check::delivery_refuses_extra_text_after_the_task_and_suffix |
| P3-3, P3-9 | no test (env removal; CLAIMS.md wording) |

| Mutant | Killed by |
|---|---|
| R1-M1 context-fit slack | subject::request_context_bound_is_snapshot_times_one_and_a_quarter_plus_task |
| R1-M2 prompt-bound slack | task::task_over_the_prompt_bound_is_refused |
| R1-M3 tree unchecked | subject::tree_mismatch_is_refused |
| R1-M4 subject equality unchecked | check::subject_mismatch_is_refused |
| R1-M5 JWT rule never matches | subject::v2_alternatives_each_hit |
| R1-M6 snapshot_served `&&`→`\|\|` | check::served_snapshot_must_be_the_vetted_one_and_pinned_by_the_job |
| R1-M7 body model unchecked | check::delivery_refuses_another_model_or_tool_set |
| R1-M8 user-text length unchecked | check::delivery_refuses_extra_text_after_the_task_and_suffix |
| R1-M9 read-budget slack | subject::read_budget_counts_snapshot_list_and_patch_pages |
| R1-M10 `.env` member path missed | subject::v2_alternatives_each_hit, sensitive_member_and_deleted_binary_are_refused_by_materialize |
| R1-M11 binary-patch rule never matches | same two |
| R1-M12 `..` component allowed | subject::unsafe_member_path_components_are_refused |
| R1-M13 job id unchecked | check::job_id_and_limits_must_be_the_runs |
| R1-M14 attempt digest unchecked | check::delivery_refuses_attempt_digest_mismatch_extra_user_text_and_missing_body |
| R1-M15 staging not removed | subject::bundle_writes_atomically_and_leaves_nothing_on_failure |

Correction: the `mutants-1` summary line "failed 20/29" counts passed steps (9 apply + 9 restore +
2), not failures; see `remediation-1/RESULT.md`.

## Remediation 2–4

Retarget to a closed set of two provider profiles (`openai-codex`/`gpt-6-luna` on r1, `claude-sdk`/
`claude-haiku-5-5` on the new revision r2, operator decision D 2026-11-18), the Messages delivery
check and its tests. Remediation 2 wrote the packet; remediation 3 got the crate compiling and wrote
the Messages path in `src/check.rs`, then stopped on its step budget; remediation 4 (this entry) fixed
the one failing test, compared the path with source at `5af34cbf`, wrote the Messages tests, three
mutants, CLAIMS.md item 5 and the hashes. Evidence is `remediation-4/{RESULT.md, test-output.txt,
final-hashes.txt, mutants/}`: **not** the `runs/` custody plan used in remediation 1 (no `mutate.py`
run; each mutant was one `Edit`, reverted by the inverse `Edit`, checked with `sha256sum`).
Tests at the end: 58 pass (check 27, revision 10, subject 17, task 4). `final-hashes.txt` is 24 lines
(22 crate files + `rust/Cargo.toml` + `rust/Cargo.lock`), `sha256sum -c` from the worktree root: 24 OK.

| Outcome | Tests (file::name) |
|---|---|
| r2 revision (D) | revision::revision_r2_is_the_frozen_requirements_without_only_execution_confinement (diff vs frozen = runId + both conditions' realizationConstraints and proposition; kept constraint `provider_family` `["anthropic"]`; DERIVATION reproduces r2); revision::revision_r2_compiles_and_the_claude_sdk_claim_has_no_known_incompatibility (no `KnownIncompatibility`, family check `ClaimedCompatible`, no confinement check) |
| Profiles and claims | revision::claude_sdk_claim_states_what_the_host_enforces; revision::hosted_claim_states_what_the_host_enforces; revision::frozen_candidate_on_the_claude_sdk_profile_stays_unmet_and_writes_no_job_inputs; subject::job_inputs_binary_refuses_and_writes_nothing (now passes `--provider openai-codex`) |
| Messages delivery (positive) | check::claude_sdk_messages_body_passes_and_records_insertions_by_digest (requested provider/model, r2 label, frozen world `prepared_conditions_unmet`, `system[]` and environment digests recorded, text not echoed); check::responses_profile_records_no_request_insertions |
| Messages delivery (refusals) | check::claude_sdk_refuses_a_request_for_another_model; check::claude_sdk_refuses_a_wrong_tool_set (extra, missing, missing placeholder, unprefixed name, duplicate, no tools array); check::claude_sdk_refuses_a_task_that_is_not_the_first_text_block (another text block first, non-text block first, non-text block carrying the task bytes, empty and null content, altered task, text after the suffix, missing suffix; string content accepted); check::claude_sdk_refuses_extra_user_text_and_stray_roles; check::claude_sdk_refuses_digest_mismatch_and_missing_body |
| Thinking | check::claude_sdk_thinking_is_neither_answer_nor_evidence (assistant thinking carrying a valid review answer is not read; empty answer fails `strict_json`; a thinking block as the answer fails `result_contract`). The checker never sees assistant turns as answers, so this pins non-reading, not a Host grammar. |
| No overclaim | check::claude_sdk_envelope_never_claims_confinement_or_route; `CLAIMS.md` (per-profile table, item 5, May-not-claim) |

| Mutant | Edit (`src/check.rs`, `messages_body`) | Killed by | Restore hash |
|---|---|---|---|
| R4-M1 model check disabled | `if parsed["model"] != model` → `if false && …` | check::claude_sdk_refuses_a_request_for_another_model (case `wrong model`); only failure | `caf32cf8…` = baseline |
| R4-M2 tool-set check disabled | `if names != tools` → `if false && …` | check::claude_sdk_refuses_a_wrong_tool_set (case `extra tool`); only failure | `caf32cf8…` = baseline |
| R4-M3 first-block type filter disabled | `.filter(\|b\| b["type"] == "text")` → `.filter(\|_\| true)` | check::claude_sdk_refuses_a_task_that_is_not_the_first_text_block (case `non-text block carrying the task bytes`); only failure | `caf32cf8…` = baseline |

3 of 3 recorded mutants killed (n = 1 mutant per property). Records: `remediation-4/mutants/M{1,2,3}-*.md`.
M1 and M2 were first run against one combined test, then the test was split so each has its own
killer and re-run; M1's mutated-file hash was recorded only for the first run. Limits: no real
retained Claude Code body was available, so the Messages shape is checked against the Host's own
fixtures and README, not an observed body (`not established` beyond that); no custody `runs/`; the
checker does not duplicate the Host's pinned tools-array digest.

Remediation 5: role-`system` messages in `messages[]` tolerated only at `messages[1]` or directly after a tool-result-only user message (string/text-block content, recorded by index and digest, not interpreted); mutant R5-M1 (accept at any position) killed by check::claude_sdk_refuses_system_messages_outside_the_observed_positions (and claude_sdk_refuses_extra_user_text_and_stray_roles); restore `af1e66ed…` = pre-mutant; 60 tests pass; `remediation-5/RESULT.md`.

Remediation 6: review-2 P3-1 (Host suffix content: HOST_SUFFIX_SHA256 required in task_with_suffix, both wires), P3-2 (job_profile_bound), P3-4 (CLAIMS.md observed Claude Code body shape, n = 2), P3-7 (frozen_world_unmet detail prints numbers); mutants R6-M1 (skip digest) killed by check::delivery_refuses_a_same_length_different_suffix, R6-M2 (skip job_profile_bound) killed by check::job_provider_and_model_must_be_the_profiles; restore `7d282178…` = pre-mutant; 64 tests pass; `remediation-6/RESULT.md`.
