# A2 matrix: OrganizationAdmission v1, S-A check OA (fixture standing)

Source: worktree `/home/bline/cbm-work/wt-org-owners-a2-20261118` on `f772612f` plus uncommitted
changes. Hashes: S-A `gate.rs` `3958e386…`, delivery `gate.rs` `7d52f4cc…` (git hash-object; equal in
`mutants/runs-1` pre/post hashes and `runs/gate-2/hashes`).
Mutant run: `mutants/runs-1/summary.json` (sha256 `b550b7eb…`): 10 applied, 10 killed by test
failures (not by compile errors: every killed step prints `test result: FAILED` with the named test
panicking), 10 restored with the recorded original sha256; pre-hashes = post-hashes.
Gate: `runs/gate-2/summary.json` (sha256 `1dfa9639…`): 15/15 passed.

S-A tests are in `work-engine-organization-authority/tests/admission.rs` (feature `fixture`). Delivery tests
are in `work-engine-review-delivery/tests/{gate,native_vertical}.rs`.

| # | Invariant / claim | Named test(s) | Mutant | Result |
|---|---|---|---|---|
| I1 | The gate takes the organization from the committed admission of the candidate; the admission carries it (`organization()`) and pins the admission | `fixture_admission_binds_principal_actor_subject_boundary_revision_and_realization` | A2-M2-skip-pin | killed |
| I2 | Exactly one committed head: two committed admissions of the candidate refuse OA `OrganizationHeadsAmbiguous` | `g2_oa_two_committed_admissions_of_the_candidate_are_ambiguous_heads` | A2-M1-first-of-two-heads | killed |
| I3 | Match on all three terms: an admission for another candidate digest, requirements digest or problem scope does not admit (three controls in one test, plus a positive control) | `g2_oa_an_admission_of_another_candidate_requirements_or_scope_does_not_admit` | A2-M3-ignore-scope, A2-M4-ignore-candidate, A2-M5-ignore-requirements | killed ×3 |
| I4 | Only committed admissions count: one revoked before admission leaves the candidate unadmitted | `g2_oa_an_admission_revoked_before_admission_leaves_the_candidate_unadmitted` | A2-M8-revoked-admission-counts | killed |
| I5 | O8 and R3 select vantage bindings by the derived admission, not by vantage alone: bindings of another organization's admission do not bind this one (control: rebinding to the own admission admits) | `g2_oa_bindings_of_another_organizations_admission_do_not_bind_this_one` | A2-M6-bindings-by-vantage-only | killed |
| I6 | Revoking the admission after admission makes the point check `Revoked([admission])` | `state_revoked_when_the_organization_admission_is_revoked_after_admission` | A2-M7-point-check-ignores-organization-pins (also kills A2-M2) | killed |
| I7 | Superseding it after admission makes the point check `Stale(IssuanceSuperseded([admission]))`; the successor (same `organizationId`, revision 2, predecessor named) admits only once bindings name it | `state_stale_when_the_organization_admission_is_superseded_and_its_successor_needs_new_bindings` | A2-M7 | killed |
| I8 | OA is one of the outcomes every admission carries; an unadmitted candidate refuses delivery before spawn with exactly `[OA, O8, R3]` | `sb_an_admission_refusal_refuses_before_spawn` (native_vertical) | A2-M9-oa-dropped-from-all | killed |
| I9 | Delivery discharge: Separation, ComponentBinding and ObserverIdentity name OA (their R3/O8 depend on the derived organization) | `discharge_is_total_admitted_kinds_name_their_check_ids` (gate) | A2-M10-separation-discharge-drops-oa | killed |
| I10 | `OperationRequest::new` takes no organization id | `compile_fail,E0061` doctest in S-A `lib.rs` (runs in `test-sa-default-doc` and `test-four-plus-sa`) | none (arity; a mutant would need a new parameter) | doctest passes |
| I11 | The delivery summary carries the derived organization and no caller-asserted organization (`CallerAsserted` has `actor_identity` only) | `canonical_preparation_reaches_the_real_sdk_and_the_existing_consumer` (via its `positive_assertions` helper, native_vertical.rs) (`summary.organization() == AdmittedOrganization { admission, organization_id: ORG }`) | none (no alternate source exists; the field was removed) | passes in `native-1` |

## Negative controls (each refuses; each has a positive control in the same test or world)

1. Revoked before admission → OA `OrganizationUnadmitted`, O8 `SeparationUnresolved` ×4, R3 `VantageUnbound` (exact set).
2. Admission for another candidate content digest → same exact set.
3. Admission for another requirements digest → same exact set.
4. Admission for another problem scope (slice 2) → same exact set.
5. Two committed heads → OA `OrganizationHeadsAmbiguous` + O8/R3 cascade (exact set).
6. Bindings pinned to another organization's admission (OA passes) → exactly O8 ×4 and R3 `VantageUnbound`.
7. Revoked after admission → `AdmissionState::Revoked([admission])`.
8. Superseded after admission → `AdmissionState::Stale(IssuanceSuperseded([admission]))`.
9. Successor admission without rebound bindings → exactly O8/R3 refuse; positive after rebinding.
10. Delivery: revoked admission → refused before spawn, `refused() == [OA, O8, R3]`, no child, consumer never ran.
11. Existing: no derived candidate → OA `CandidateUnavailable` and R3 `VantageUnbound` added to the exact set (`g2_o3_no_derived_candidate_leaves_candidate_checks_unavailable`).

Positive controls: the base world admits in every OA test file; production policy's exact refusal set is
unchanged (`p_production_refusal_set_is_exact_and_fixture_admits_the_same_candidate`: OA passes on the
fixture admission record in both domains; production still refuses on O4/O5/O8/R3).
