accept_private_readiness

# S-A recheck 1: remediation of REVIEW-1

Reviewer: the REVIEW-1 session, retained context. Read-only except this file. Scope: the remediation diff
`1b8bf301..bfaa455c`, the new tests, `sa/ROOT-RULINGS-1.md`, `sa/remediation-1/RESULT.md`, the updated
`sa/RESULT.md` and `sa/MATRIX.md`, `sa/ROOT-GATE-3.json` and `sa/root-gate-3/`. No build or test was run.

Verdict for `bfaa455c`: **accept_private_readiness**. F-1 (the only P2) is closed with tests and mutants. Every
ruling is applied as written. No check was weakened. The claim corrections are truthful. One statement in my
own REVIEW-1 was wrong and is corrected in §3. P3 items to carry are in §4.

## 1. Subject and evidence

| Item | Value | How established |
|---|---|---|
| Checkpoint / tree / patch | `bfaa455c…f03a` / `5ca8321c…3918` / `62dae9df…8514`, parent `1b8bf301` | from the request; tree and patch digests not recomputed (`unverified`) |
| Diff shape | 3 files, 560 insertions, 27 deletions: `src/gate.rs`, `src/lib.rs`, `tests/admission.rs` | executed: `git diff --stat 1b8bf301 bfaa455c`; the whole diff was read |
| Mutant source = subject source | export-3 `gate.rs` sha256 `f2158877…effbc8`, `lib.rs` `5cae02cc…399b`, `tests/admission.rs` `a2fc052b…e19e` equal the remediation record; `observation.rs` `42ff4d6d…d183` equals the first export | executed: `sha256sum` |
| Root gate 3 | all features 1 + 43 + 1 + 7 = 52 passed; default 1 + 0 + 1 + 7 = 9; organization 156 | read from `root-gate-3/test-*.stdout`; not re-run |
| Mutants | 7 of 7 gate mutants show a test panic and `test result: FAILED`; the two-barrier doctest mutant shows `(line 76) - compile fail ... FAILED`; the single-barrier doctest mutant shows 7 passed (survived) | read from `remediation-1/mutants/*/test.stdout` (9 records) |
| Doctest diagnostic | E0451 (`SealedObservation` literal), E0451 plus "type `Entry` is private" (`AuthorityState` literal), E0624 (`AuthorityState::new`), E0382 (second `admit`) | read from `remediation-1/diagnostics/doctest-errors/test.stdout` |
| Toolchain | `rustc 1.92.0 (ded5c06cf 2025-12-08)`, stable | read from `root-gate-3/toolchain.stdout` |

Individual mutant records' own input digests were not checked. Board state remains `not established` (the
read was denied in this session). My cost and usage are `not established`.

## 2. Rulings: resolved or not

| Item | Ruling | Result | Basis |
|---|---|---|---|
| F-1 | tests and a mutant for the 6.1 F4 containment fields, missing grant, R3 digest; correct both documents | **Resolved** | Four tests isolate the grant's problem scope, the decision scope's problem scope, actions and boundaries, each with a control. The boundaries test uses a requirement that names two boundaries, with two non-covering scopes and three covering controls (exact, superset, unrestricted). Four containment mutants and the R3 digest mutant are killed by an assertion (records). The recommended cases were added too: missing grant, superseded grant, ambiguous component binding, unknown, unbound and ambiguous vantage, ambiguous acceptance. |
| F-2 | `admit` takes the observation by value | **Resolved** | `gate.rs` signature is `observation: SealedObservation`; a second `admit` with the same value does not compile (diagnostic: E0382). Freshness in time is still the host's, as the ruling records for S-B. |
| F-4 | R4 requires the acceptance profile to equal the candidate's | **Resolved** | New arm in `r4` refuses `RealizationProfileMismatch` before pinning; test asserts the exact set {R1, R2, R4}; mutant killed. |
| F-5 | O4 uses exactly-one committed principal | **Resolved** | `covered` returns `PrincipalAmbiguous` when more than one committed principal carries the named id. With exactly one, a grant held by a non-committed issuance of that name still refuses as `PrincipalNotCurrent`, so the rule matches O8's (static). Test asserts O4 and O8 together; mutant killed. |
| F-11 | compile-fail doctest for non-constructibility | **Resolved, with the limit the builder states** | Three doctests, run with and without `fixture`. Each fails for its intended error on the unmutated source (diagnostic). The `AuthorityState::new` doctest does not pin the visibility of `new` by itself: with `new` public it still fails to compile because `Entry` is private (mutant survived). Opening both barriers is detected. |
| F-12 | exact reasons in the ids-only test; add unrepresented stale conditions | **Resolved** | The test now asserts four `AuthorityScopeUnresolved` reasons; `sa/RESULT.md` l.157-160. |
| F-7 | state O3's limit | **Resolved** | `sa/RESULT.md` l.151-156; `CheckId::O3` and `OperationRequest::new` docs. |
| Choice 1 | reword the `EffectAdmission` doc | **Resolved** | "The only value that can permit an effect, and only when the consuming host's own regate … also passes". |

Nothing weakened (static, whole diff read): in `gate.rs` the removed lines are doc comments, the `&` on the
observation parameter and the old `r4` call; no condition was removed or relaxed, and the two new branches
only add refusals. In the tests no test was removed (29 → 43; 14 new functions counted in the diff). Three
existing tests changed only to take a second `observe_for` after `admit` consumed the first; one was
strengthened (F-12). The generalized world helper builds the same default world as before.

Claim corrections: `sa/RESULT.md` l.5-9 and l.122-132 and `sa/MATRIX.md` l.4-5 and l.59-73 say what was wrong,
mark what describes the earlier checkpoint, and report the surviving doctest mutant as a survivor. I found no
false statement in them. Two small omissions are in §4 (N-2, N-5).

## 3. Correction to REVIEW-1

- **Wrong claim:** REVIEW-1 §6 said the compile-fail doctests "pin error `E0599`".
- **Error class:** a statement about tool behaviour made without running the check that would show it wrong.
- **What is true:** on this toolchain the error code in `compile_fail,E….` is not enforced. Basis: the
  `state-new-public-doctest` mutant makes `new` public, so E0624 cannot be emitted for that doctest, and it
  still passed as "compile fail … ok" (record). The toolchain is stable 1.92.0.
- **Consequence:** all seven doctests assert only "does not compile". The three original not-`Clone`
  doctests were credited with more specificity than they have; they would also pass if an export were renamed.
  The REVIEW-1 verdict did not depend on this. `sa/MATRIX.md` row 28's `not established` can be read as
  "not enforced", on the basis above.
- **Check that would have caught it:** reading `root-gate/toolchain.stdout` against the behaviour of a
  deliberately wrong code, or a `no_run` diagnostic like the builder's.

## 4. P3 items to carry

Carried from REVIEW-1, unchanged by ruling:

- F-6: first class of a dimension wins in R1/R2 (latent; one class per dimension in v1).
- F-9: `AdmissionState::Current` is constructible by any caller.
- `o8_bindings` is redundant with the exactly-one binding rule.
- F-11 second half: the O9 wiring in `evaluate` is tested at function level only.
- S-B packet obligations as the root recorded them: F-2 (a fresh observation for the regate), F-3, F-7, F-8, F-10.

New from this recheck:

- **N-1.** Doctest specificity (§3). The four new doctests have a recorded diagnostic of their intended
  errors; the three original ones rest on static reading (each names an exported type and calls `.clone()`).
- **N-2.** `AuthorityRevoked` is still not asserted by any test, and neither claim document lists it. The
  guard is covered indirectly: the revoke-and-reissue test asserts the revoked grant is not pinned (static).
- **N-3.** Two committed acceptances of one digest under different profiles refuse as ambiguous instead of
  selecting the candidate's profile. Fail closed; unreachable with one profile in v1.
- **N-4.** `ReasonCode` gained `PrincipalAmbiguous` and `RealizationProfileMismatch` and is not
  `non_exhaustive`; an exhaustive match in S-B must handle both (the builder flagged this).
- **N-5.** Minor: `g1_o4_two_committed_principals…` is a G2 oracle (the matrix labels it correctly);
  `state_a_revocation_after_admission…` repeats `state_revoked_when_a_pinned_grant_is_revoked` in effect, the
  F-2 property being carried by the doctest; `sa/RESULT.md` l.35 calls the test-file rename's author
  `not established` while `sa/ROOT-GATE.json` records that the root made it; the non-containment new tests
  have no recorded mutant (stated by the builder).

## 5. Not checked

Tree and patch digest recomputation; `remediation-1/runs/`; clippy and fmt records beyond the gate summary
status; mutant record input digests; any file outside the remediation diff other than the two claim documents.
No forbidden path was read.
