# S-A root rulings on REVIEW-1 (2026-11-17)

- **Choice 1 (Gate R + EffectAdmission in the pure crate):** the operator-approved R2REC subject (ROOT-SUMMARY) places Gate R R1–R4 and the five states in S-A; that supersedes O1's placement sentence. The consuming host still decides and commits the effect (allocation, single use, effect commit, regate with a fresh observation). Doc wording "the only value that permits an effect" must say "the only value that can permit an effect when the host's own regate also passes".
- **Choice 6 (pin every covering grant):** accepted as the fail-closed reading.
- **Choice 10 (installed-digest match in R3):** accepted; S-B maps ComponentBinding → O4 + R3.
- **Choice 11 (`RealizationAcceptance.provides`):** accepted at fixture standing; the production record schema needs owner approval later (§18.6).
- **F-4:** R4 requires `record.profile` equal to the candidate's profile (one comparison, one test).
- **F-2:** `admit` takes the `SealedObservation` by value, so a point check needs a new `observe_for`.
- **F-5:** O4 uses the same exactly-one committed principal resolution as O8.
- **F-11:** add a compile-fail doctest that `AuthorityState`/`SealedObservation` cannot be constructed outside the crate without `fixture`.
- **F-1:** required tests and mutant as REVIEW-1 §2 F-1 and §7.1; correct RESULT.md and MATRIX.md claims.
- **F-7, F-12:** add to "Explicitly unclaimed": O3 checks request self-consistency only (no committed admission anchor in this slice); stale conditions "episode revision moved", "realization marked stale", "admission head superseded" are host/episode-owner facts not represented here; fix the ids-only assertion to exact reason codes.
- **Recorded as S-B packet obligations:** F-2 (fresh observation for the regate), F-3 (first non-current state other than Unknown is final for that admission), F-7 (compare candidate/requirements/subject/realization digests with the binding), F-8 (assert and label `Domain::Fixture`), F-10 (key discharges on `CheckId`, subjects are diagnostics).
- Not taken: F-6 (latent, unreachable in v1), F-9, `o8_bindings` simplification — carried as P3.
