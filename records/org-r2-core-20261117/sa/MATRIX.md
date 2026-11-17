# S-A evidence matrix: `work-engine-organization-authority`

Source: worktree `/home/bline/cbm-work/wt-org-r2-core-20261117` (HEAD `561c6ffc`, uncommitted).
**[R1] Remediation 1 (2026-11-17)** changes this matrix; rows marked **[R1]** are new or corrected, and
the rest describe checkpoint `1b8bf301`. Record: `sa/remediation-1/RESULT.md`.

Tests: `rust/crates/work-engine-organization-authority/tests/admission.rs` (**[R1]** 43 with the
fixture feature; 29 at the checkpoint), `src/gate.rs` unit test (1), `tests/base_candidate_shape.rs`
(1, shape preconditions; named `probe_tmp.rs` in the pre-checkpoint text), **[R1]** 7 compile-fail
doctests (3 at the checkpoint).
Runs: checkpoint `sa/runs/final/` (custody summary sha256 `780df77f…fed103`, 6/6 passed);
**[R1]** `sa/remediation-1/runs/final/` (summary sha256 `cc52a651…c9924b`, 5/5 passed).
Mutants: `sa/mutants/<name>.patch` + custody records in `sa/mutants/records-20261117T111045Z/<name>/`,
summary `sa/mutants/summary.json` (gate.rs sha256 recorded there; every mutant restored and the
restoration sha-verified). Each kill below is a `test result: FAILED` line in the mutant's custody
stdout (checked for all 18; no compile error among them). The per-name directories directly under
`sa/mutants/` are a superseded first run (in it `r1-neutralized` failed to compile, i.e. was not a
kill); use the `records-…` directory.

Oracle classes (Amendment 6.1 §6): G1 isolated input (named check refuses, all others pass),
G2 coupled (exact failure set and reasons), S observation (no outcomes exist), P production
(exact production refusal set and the fixture path admits the same candidate).

| Packet outcome | Guard | Tests (class) | Mutant (killed by) |
|---|---|---|---|
| 1 Records, `IssuanceId` ≠ `BodyDigest` | revoke disables exactly its issuance; reissue with identical terms has a new id, same body digest | `g1_o4_revoke_disables_exactly_its_issuance_not_a_reissue_with_identical_terms` | — (identity is structural; covered through O4/state mutants) |
| 1 No production constructor | records, observation, admission not constructible / not `Clone` | 3 compile-fail doctests in `lib.rs` show **only** that SealedObservation, EffectAdmission and Reservation are not `Clone` (**[R1]** the checkpoint row cited them for more than they show); constructors exist only under `fixture` (`records.rs` `fixture_constructors`, `fixture.rs`) | — |
| **[R1]** F-11 Non-constructibility of observation values | `SealedObservation` and `AuthorityState` cannot be built outside the crate | 3 new compile-fail doctests: `SealedObservation { .. }` literal, `AuthorityState { .. }` literal, `AuthorityState::new(..)`. Run with all features and with default features; each fails for its intended error on the unmutated source (diagnostic run `sa/remediation-1/diagnostics/doctest-errors/`: E0451, E0451, E0624) | `state-new-and-entry-public-doctest` (killed by the `AuthorityState::new` doctest); `state-new-public-doctest` alone **survived** (the private type `Entry` is a second barrier). Whether rustdoc on this toolchain enforces the error code of `compile_fail,E....`: `not established` |
| **[R1]** F-2 `admit` takes the observation by value | one observation decides at most one admission | compile-fail doctest (use of moved value, E0382); `state_a_revocation_after_admission_is_seen_only_by_a_new_observation` (point check on another `observe_for`) | none (a signature mutant is a compile failure, not a test-assertion kill) |
| 2 Observation, `Unknown` injection | no observation ⇒ no outcomes | `s_unknown_observation_yields_no_observation_and_so_no_outcomes` (S) | — (type-level: `admit` needs a `SealedObservation`) |
| 3 Seal: same observation, later boundary (6.1 B12b) | `admit` refuses an observation sealed to another boundary | `seal_same_observation_cannot_decide_a_later_boundary` (with control) | `seal-admit-boundary` |
| 3 Seal at the point check | later reserved epoch ⇒ stale; other boundary ⇒ stale | `seal_a_later_reserved_boundary_makes_the_admission_stale` | `state-boundary` |
| 4 O3 | recomputed candidate digest | `g1_o3_candidate_digest_of_another_candidate_refuses_only_o3` (G1); `g2_o3_no_derived_candidate_leaves_candidate_checks_unavailable` (G2) | `o3-digest` |
| 4 O4 | grant coverage (expiry, decision scope, names never bind) | `g1_o4_expired_grant_refuses_only_o4` (G1, with control at now−1), `g1_o4_unresolved_decision_scope_covers_nothing` (G1), `g1_o4_a_principal_named_like_a_component_binds_nothing` (G1), `g2_revoking_the_acceptor_principal_fails_o4_and_o8_together` (G2) | `o4-expiry` |
| **[R1]** 4 O4 containment (6.1 F4) | grant problem scope; decision-scope problem scope, actions, boundaries (controls: exact, superset, unrestricted) | `g1_o4_grant_for_another_problem_scope_refuses_only_o4`, `g1_o4_decision_scope_of_another_problem_scope_refuses_only_o4`, `g1_o4_decision_scope_without_the_required_action_refuses_only_o4`, `g1_o4_decision_scope_boundaries_must_contain_the_requirements_boundaries` (all G1, exact sets) | `o4-grant-problem-scope-skip`, `o4-ds-problem-scope-skip`, `o4-ds-actions-skip`, `o4-ds-boundaries-inverted` (each killed by an assertion in its test) |
| **[R1]** 4 O4 authority resolution | no grant for the permission; superseded grant (with and without a covering replacement); component bound twice | `g1_o4_no_grant_for_the_permission_is_authority_missing`, `g1_o4_a_superseded_grant_does_not_cover_unless_its_replacement_does`, `g1_o4_two_committed_bindings_of_one_component_are_ambiguous` (G1) | none recorded |
| **[R1]** F-5 O4 named principal exactly one | two committed principals with the named id resolve to none, as in O8 | `g1_o4_two_committed_principals_with_the_named_id_resolve_to_none` (G2: O4 `PrincipalAmbiguous` + O8 `SeparationUnresolved`) | `o4-named-principal-not-exactly-one` (killed) |
| 4 O5 | `none` rows refuse; fixture attests only separation | `g1_o5_a_none_row_other_than_separation_refuses_only_o5` (G1), `g2_o5_retained_continuity_…` (G2: O5+R2) | `o5-production-accepts-separation` |
| 4 O8 | actor anchors, not ids | `g1_o8_distinct_principals_with_the_same_actor_anchor_are_not_separate` (G1), `g1_o8_an_unbound_dependent_vantage_is_unresolved` (G1) | `o8-ids-not-anchors` (proposal mutant (a)) |
| 4 O9 | internal error / view error ⇒ `organization_integrity` / `view_invalid` | `gate::tests::o9_refuses_internal_errors_and_view_errors_as_integrity` (G1 at function level; negatives unreachable from valid requirements) | `o9-drop-internal-errors` |
| 4 R1 | per-condition constraints vs accepted classes | `g2_r1_r2_unstated_provider_class_is_unestablished_for_condition_and_vantage` (G2; R1 failure always co-fails R2) | `r1-neutralized` |
| 4 R2 | effective constraints + normative properties | `g1_r2_unstated_effective_property_refuses_only_r2` (G1) | `r2-skip-effective-normative` |
| 4 R3 | declared actor = bound actor; installed component digests | `g1_r3_realized_actor_differs_from_bound_actor` (G1), `g1_r3_installed_component_digest_differs_from_binding` (G1) | `r3-skip-actor` |
| **[R1]** 4 R3 realization digest | bound realization digest differs from the declared one | `g1_r3_bound_realization_digest_differs_from_the_realization` (G1) | `r3-skip-realization-digest` (killed) |
| **[R1]** 4 R2/R3 vantage resolution | vantage not in the candidate and unbound; vantage bound twice | `g2_a_vantage_the_candidate_does_not_have_and_no_binding_refuse_r2_and_r3` (`VantageUnknown`, `VantageUnbound`), `g2_two_committed_bindings_of_the_vantage_are_ambiguous_for_o8_and_r3` (`VantageBindingAmbiguous`) (G2) | none recorded |
| **[R1]** 4 R4 ambiguity | one digest accepted twice states nothing | `g2_two_committed_acceptances_of_one_digest_are_ambiguous_and_state_nothing` (G2: R1+R2+R4) | none recorded |
| **[R1]** F-4 R4 profile | acceptance profile must equal the candidate's profile (`RealizationProfileMismatch`) | `g2_r4_acceptance_for_another_profile_is_not_an_acceptance_for_the_candidate` (G2: R1+R2+R4) | `r4-ignore-profile` (killed) |
| 4 R4 | digest accepted by a committed `RealizationAcceptance` | `g2_r4_unaccepted_realization_leaves_r1_r2_without_accepted_values` (G2: R1+R2+R4; another digest is accepted) | `r4-accept-any-digest` |
| 4 Every check evaluated | no short circuit | all G2 tests (exact sets) | `short-circuit` (proposal mutant (f)) |
| 4 Production all-refuse | exact production set {O4, O5, O8, R3}; fixture admits same candidate | `p_production_refusal_set_is_exact_and_fixture_admits_the_same_candidate` (P) | `o5-production-accepts-separation`, `r3-production-passes` |
| 5 Admission binds principal, actor, subject, operation, boundary, revision, realization | positive control | `fixture_admission_binds_principal_actor_subject_boundary_revision_and_realization` | — |
| 5 Outcomes by check id | `EffectAdmission::outcomes()`, `AdmissionRefusal::Checks(Outcomes)` keyed by `CheckId` | every test using `assert_exact` | — |
| 5 State current | same revision; regate after unrelated commit | `state_current_at_the_same_revision_and_after_an_unrelated_commit` | `state-refused-skip-regate` (inverse) |
| 5 State revoked | pinned grant / acting principal revoked | `state_revoked_when_a_pinned_grant_is_revoked`, `state_revoked_when_the_acting_principal_is_revoked` | `state-revoked` (proposal mutant (b) analogue) |
| 5 State stale | pinned issuance superseded; later boundary | `state_stale_when_a_pinned_issuance_is_superseded`, seal test above | `state-stale-superseded`, `state-boundary` |
| 5 State unknown (fail closed) | no observation; regressed / conflicting revision; retracted pin | `state_unknown_without_an_observation`, `state_unknown_when_the_source_breaks_its_ordering_obligations` | `state-unknown-no-observation`, `state-unknown-regressed` |
| 5 State refused | regate refuses (time passed; new commit makes scope ambiguous) | `state_refused_when_regating_the_same_request_refuses`, `state_refused_when_a_new_commit_makes_the_decision_scope_ambiguous` | `state-refused-skip-regate` |

**[R1] Corrected (REVIEW-1 F-1).** The checkpoint text said the other O4 sub-reasons (scope
unresolved, insufficient actions/boundaries/problem scope, superseded grant, principal not
current), component ambiguity and the R3 digest mismatch were "not mutated individually". That
understated the gap: at the checkpoint no test exercised insufficient problem scope, actions or
boundaries, a superseded or missing grant, component ambiguity or the R3 digest mismatch, so there
was nothing for a mutant to fail. Rows above now give each a test; mutants are recorded for the
containment conditions, the R3 digest comparison, F-4 and F-5. Still without a recorded mutant:
scope unresolved, principal not current, superseded/missing grant, component ambiguity, the
vantage-resolution reasons, R4 ambiguity, `RevisionConflict` and `IssuanceRetracted` (all but the
last two have a test at R1; the last two had one at the checkpoint).

**[R1] F-12 masking control.** `state_refused_when_a_new_commit_makes_the_decision_scope_ambiguous`
asserted check ids only (`outcomes.refused() == [O4]`). It now asserts the exact reason set: the
shared `ds-1` loses its scope for the owner grant and for each of the three component-permission
grants (4 `AuthorityScopeUnresolved` reasons).
