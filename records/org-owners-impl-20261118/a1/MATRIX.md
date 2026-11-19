# A1 matrix: scope.digest is the exact-subject digest (S-A check S1)

Mutant runs: `mutants/runs-1/` (custody, `--keep-going`; summary sha256 `4d09981c…`; 16/27 steps "failed" = every mutant test step, as intended). Every mutant was killed by an assertion panic (not a compile error), verified from the step stdout/stderr. `pre-hashes` == `post-hashes` (S-A gate.rs `2513877872a0…`, delivery gate.rs `cdda8039ffb0…`): exact restore.
Gate: `runs/gate-1/` (13/13 passed). Test names are in `work-engine-organization-authority/tests/admission.rs` (SA), `work-engine-review-delivery/tests/gate.rs` (DG), `.../native_vertical.rs` (NV), `work-engine-review-preparation/tests/fixture_consumer.rs` (PF).

| # | Invariant | Named tests | Mutant | Result |
|---|---|---|---|---|
| I1 | `SubjectRef::scope_digest` = sha256 of canonical JSON `{commit,patchIdentity,tree}` (episode codec), lowercase hex | SA `s1_scope_digest_is_the_episode_codec_digest_of_the_subject`; NV `assumption_world_is_the_frozen_world_without_realization_constraints` (= frozen `initialSubject.sha256` `18108e56…`) | A1-M2 digest ignores commit and tree | killed (SA test0 and test1) |
| I2 | A subject differing in any one of commit / tree / patchIdentity refuses S1 alone | SA `s1_a_subject_differing_in_each_field_refuses_only_s1` | A1-M1 S1 always passes | killed |
| I3 | Frozen-world stand-in scope (`patchIdentity`) refuses S1 alone (frozen-world admissions are negatives) | SA `s1_a_patch_identity_stand_in_scope_digest_refuses_only_s1`; DG `the_frozen_stand_in_scope_digest_refuses_only_s1` | A1-M1 | killed (both) |
| I4 | S1 compares the obligations the request vantage serves, not the candidate's first/every obligation | SA `s1_compares_the_obligation_the_vantage_serves` (ob-b vantage passes, ob-a refuses) | A1-M3 first candidate obligation | killed |
| I5 | `discharge(ScopeDigestToSubject) == Admitted(&[S1])`; it is no longer an AssumedDischarge | DG `discharge_is_total_admitted_kinds_name_their_check_ids`, `the_assumption_world_screens_into_admitted_and_assumed_mapping_kinds`; NV `native_vertical` admitted/assumed lists (asserted in gate-1 native-1) | A1-M4 discharge reverts to AssumedDischarge | killed (both DG) |
| I6 | S1 is in `CheckId::ALL`; every admission lists exactly the ten ids | SA `s1_the_base_admission_passes_s1_and_lists_every_check` | A1-M5 S1 dropped from ALL | killed (plan §1 expected a panic at `gate.rs:572`; it does not panic: the map keeps S1 unread. The listing test kills it.) |
| I7 | A participant vantage serving no obligation refuses S1 (`ScopeDigestUnserved`) | SA `s1_a_vantage_that_serves_no_obligation_refuses_s1` | A1-M6 unserved vantage passes | killed |
| I8 | A vantage the candidate lacks refuses S1 (`VantageUnknown`) alongside R2/R3 | SA `g2_a_vantage_the_candidate_does_not_have_and_no_binding_refuse_r2_and_r3` (updated exact set) | A1-M7 unknown vantage passes | killed |
| I9 | No candidate: S1 is `CandidateUnavailable` like O4/O5/O8/R1/R2 | SA `g2_o3_no_derived_candidate_leaves_candidate_checks_unavailable` (updated) | — (removing S1 from that list panics `every check evaluated`; covered by the same test) | gate-1 pass |

Negative controls / regressions (gate-1, unmutated):
- N1 revoked pin after an S1-passing admission still `Revoked`: SA `s1_a_pin_revoked_after_admission_is_still_revoked`.
- N2 every other check unchanged on the S1 refusals: the `assert_exact` / `refused() == [S1]` sets in I2, I3.
- N3 frozen file byte-identical (`git diff f772612f -- …/frozen/` empty); 1A goldens unchanged (work-engine-organization tests pass in test-four-plus-sa).
- N4 consumer subject comparison still load-bearing: PF `s1_a_gate_proof_for_another_candidate_subject_or_boundary_is_refused` "other subject" now admits over requirements scoped to the other subject (S1 passes) and the consumer refuses on `subject`.
- N5 frozen world still refused before spawn by known incompatibility: DG `the_frozen_world_is_a_known_incompatibility`.

Not mutated (structurally equivalent): "S1 checks only the first served obligation". Derivation gives each reviewer vantage exactly one served obligation (`work-engine-organization/src/derive.rs:592`, `serves: vec![obligation.id.clone()]`), so a multi-obligation vantage cannot be constructed and that mutant is equivalent. I4 covers the reachable multi-obligation case (a multi-obligation candidate).
