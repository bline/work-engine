# C1 profile v1: rationale

Draft by worker `aa36a9c3b4f77173a` (packet R1-S0 v1) for coordinator `bf60b4c1`, who freezes it before S1 launches (`decomposition-v2.md` step S0; `contract-v2.md` §7 *Freezing*, AC6′a). Profile file: `c1-profile-v1.json`. Contract cited: `plan/contract-v2.md` (sha256 in `result-v1.json`). Decision-need source: `skills/manual-orchestration/references/coordinator.md` *Acceptance* (lines 29-31) and `references/supervisor.md` *Validation and review* and *Result* (lines 13-19), at repo HEAD `f57df489`. Those files are clean against that HEAD.

## Where the rationale lives

Contract §3.1 makes the profile schema closed (`deny_unknown_fields`), and its requirement objects carry only `id`, `class`, `statement` and either `predicates` or `evidence_from`. A `rationale` or `section` field would make S1's `check --kind profile` reject the profile. The rationale and section for each requirement are therefore here, keyed by id. In the JSON, mechanical and judgment requirements are kept apart by `class` and by order: all mechanical requirements come first. Separate arrays would also break §3.1.

## Profile-level fields

| field | value | why | v2 section |
| --- | --- | --- | --- |
| decision | `root-integration-of-private-candidate`, owner `coordinator` | the decision under test is the coordinator's integration of one privately qualified candidate | §3.1 example, §7 *Decision* |
| assessor_roles | `tier1-reviewer`, `supervisor` | Q2: the tier-1 reviewer assesses; the coordinator decides and is not an assessor | §1 Q2, §7, §6 rule 6 |
| roots | `destination` (git), `candidate`, `evidence`, `binaries` | one git root, at the destination, where the baseline is rechecked; candidate source identity never depends on git | §3.1 Roots, §3.4 rules 2, 5, 6, §7 |
| parameters | the 10 in §7, copied exactly | every subject-varying value is a parameter, so the profile holds none | §3.1 Parameter kinds, §7 *Parameters* |
| authority_refs | reconciliation 369-373, view invariants 170-175, SKILL.md 32, coordinator.md 29-31, supervisor.md 13-19 | the authorization, the invariants, the private-readiness/integration split and the decision need | §1 |

## Mechanical requirements (8 requirements, 12 predicates; contract §7 table, verbatim)

- **M-authority**: integration is lawful only under the authority the candidate was built under, so those bytes must be the bytes the candidate cites (§3.5 `manifest_match`, §3.3, §7).
- **M-source**: the decision needs exact source binding, meaning that what gets integrated is the qualified candidate, byte for byte (coordinator.md *Acceptance*; §3.4 rule 6, §3.5, §7).
- **M-binaries**: validation evidence is only about the binaries it exercised, so those bytes must be bound (supervisor.md: "binary/profile ... matter"; §3.5, §7).
- **M-evidence**: every record that the judgment requirements read must be the bytes that were recorded. It also digest-binds the files the JSON and grep predicates read, via `bound_by` (§3.1 `bound_by`, §3.5, §7).
- **M-suites**: the decision needs required gates passed. The battery cannot run tests, so the outcome is read from the validation record (§2.2 *Does not own*; §3.5 `json_compare` with `{each}`; §7).
- **M-review-bound**: the decision needs a retained review that reached a conclusion and did not itself make the root's integration decision (SKILL.md:32; §3.5 `json_present`/`json_equals`, §3.6 null rule, §7).
- **M-logs**: an independent read of the raw suite logs, so that a summary record cannot report a pass that the logs contradict (§3.5 `grep_count` with `per_path`, §3.6 grep rule 2 and `.exists`, §7).
- **M-baseline**: the decision needs affected destination bytes rechecked against the prepared baseline. Equal HEAD plus clean affected paths means the affected bytes equal the baseline's (coordinator.md *Acceptance*; §3.4 rules 3-4, §3.5 `git_*`, §7).

## Judgment requirements (8; always `indeterminate` from the tool, §5.3; assessed under §6 rule 4)

- **A-review**: the decision needs a retained review **with findings dispositioned** whose subject is this exact source. v2 has no predicate that compares a receipt field with a manifest parameter's digest, so source-of-review identity is judged here (supervisor.md *Validation and review*; §7).
- **A-filtered**: passing counts can hide filtered or ignored tests, and only a judgment can say whether those exclusions matter (supervisor.md "report unavailable or incomplete gates"; §7).
- **A-obligations**: the decision needs open obligations recorded, and whether they block integration is a judgment (coordinator.md "record open obligations"; §7).
- **A-scope**: whether the changes stay within the authority is semantic. M-authority and M-source only bind the bytes (§7).
- **A-coverage**: the suite list is a subject value, so whether it is sufficient, including the downstream consumers reached through shared contracts, must be judged (supervisor.md "validate affected downstream consumers"; §7).
- **A-affected-set** *(added; not in §7)*: M-baseline is only as strong as the `affected_paths` list. That list is a subject value, and v2 has no predicate that compares a list parameter with manifest entries, so whether it covers every written or overlapping path is a judgment (coordinator.md "affected and overlapping destination bytes"; §3.5).
- **A-collection** *(added; not in §7)*: the decision needs process and child collection, and that is recorded in prose results that no battery kind can verify (coordinator.md *Collection*; supervisor.md *Result*).
- **A-limits** *(added; not in §7)*: the decision needs limitations and unavailable evidence recorded and judged compatible. A-review covers only the review's own limits (coordinator.md *Acceptance*; supervisor.md *Result*).

**Effect of the three additions on the pre-registration (for the coordinator).** They add no predicates, so no derived checks, the plan and N_derived (AC6 b) are unchanged. AC4(i) predicts C1 is `uncertain` with "five attributed indeterminate". With this profile it would be eight. The ceiling stays `uncertain`, because every attributed requirement is `indeterminate` from the tool (§5.3), so the falsifier is unaffected. AC6(c) maps the A-requirements against D's limits and obligations, so the residual list grows by three. If the coordinator wants §7 verbatim, deleting those three objects gives exactly §7's set, and the profile still validates.

## Deliberately left out

- **Candidate-side git checks**: §3.4 rule 6 makes source identity the manifest, never candidate git.
- **Fixed-count `file_sha256` per artifact**: these would fix artifact counts per subject. §7 replaced them with manifests for subject independence.
- **Running tests or validators**: the battery cannot (§2.2, §10). Outcomes are read from records and logs.
- **Mechanical "no retained failures" / "no open P2"**: C1 tolerates dispositioned failures and obligations, which are judged under A-filtered and A-obligations. These checks are the sketch's stricter decision (§7 test profile, AC4).
- **Shared Cargo files unchanged / offline lock table**: these are R1-specific gates (AC8). A C1 profile for every candidate cannot name them; a subject's suite list and validation record carry them.
- **Mechanical review-subject = source digest**: not expressible in v2 (no manifest-digest-as-expected), and §7 prescribes no such receipt key. Judged under A-review.
- **Requested vs observed model identity, context checks, dispatch decisions**: these are supervisor-result content for accounting, not integration readiness. Where they limit the evidence, they fall under A-limits.
- **Freshness / claim reliance**: D5 and invariant 5. The freshness basis is the exact observation, recorded in the assessment, not a requirement.
- **Destination diff applying cleanly / candidate bytes written to the destination**: that is the integration act, which the coordinator owns (§10), not readiness.

## Subject independence

The profile names no subject path, digest or value:

- **Mechanical scan** of `c1-profile-v1.json` (scratchpad `validate_profile.py`): no hex run of 12 or more characters (no digest or commit); no string beginning with `/` (no absolute path); none of the literals `6a945b15`, `f57df489`, `s1-candidate`, `manual-decomposition`, `slices/D`, `decision-readiness/`, `core_and_downstream`, `terminal_ui`, `broad_affected`. Result: 0 hits for each.
- **Where subject specifics go**: roots → subject `roots`; manifests (with sha256), the validation record, the review receipt and the suite-log paths → `values` (§3.2); the suite names, the destination HEAD and the affected paths → `values`.
- **Literals that remain, and why they are not subject-specific**:
  - the key shapes `supervisor_final_tests.<suite>.exit_code` / `.counts.failed`, `review_conclusion`, `integration_decision`, and the value `not_made_root_owned`. §7 prescribes these as the C1 evidence shape for every subject, and S1-snap produces them for S1;
  - the cargo log lines `test result: FAILED` and `test result: ok`.
- **Scope limits**:
  - M-logs assumes cargo-test log output. A subject whose required suites are not cargo tests would be `unmet` there, and the remedy is a new profile version from the profile owner (D8);
  - `authority_refs` line ranges cite repository files at HEAD `f57df489` and can drift with later edits.
