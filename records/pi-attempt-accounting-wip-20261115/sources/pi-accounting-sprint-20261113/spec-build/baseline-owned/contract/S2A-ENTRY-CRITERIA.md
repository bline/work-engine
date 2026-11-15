# S2a entry criteria: budgeted API-lane attempt reducer only (not authorised)

**Contract:** `pi-accounting-contract/1.4-s1` (revision 5). Revision 5 changes S2a stages C to K: replacement eligibility
and the conservative-exposure measure (A-01), error-status usage (A-02), valuation totality (A-04), model
canonicalisation `mc-1` (A-05), status rules per path (A-06), the typed `amount_source` (A-08) and conflicting
reservations (A-09). A-03 (gate billing bound) is stage R and the future gate, outside S2a. Revision 2's version of this page wrongly said Q2 keeps
`Exact(R)`, and excluded RS-1 to RS-3 while its grading subset needed them (P1.29 R-16 premise). Both are corrected
here.

**Scope.** Stages C (budgeted correlation, **including** the shape rules RS-1 to RS-3 for error and refusal rows
joined to budgeted attempts), S, V, X and K for API-lane sessions with dollar reservations. That includes the
attempt-level facts `forwarding`, `execution` and `billing` (§6.1, §6.2).

**Excluded:**
- subscription lanes;
- unbudgeted requests;
- lifecycle L, attribution T, rollup R, commitments and admission;
- the evidence adapter E. S2a consumes typed evidence from a test fixture loader inside its own test suite.

## Entry gates (all required before S2a starts)

1. **Independent design review** of the current revision completed under the modular plan (`review-plan/` of P1.30): module A
   (financial semantics) and module C (corpus and validator) each return a complete verdict with no unresolved P1.
   Module B (lifecycle, attribution, admission) must have no unresolved P1 that touches stages C to K. The final
   integration review is required before S5, not before S2a.
2. **Owner answers** are recorded for every question that touches stages C to K. Q2, Q5, Q6 and Q10 are decided and
   applied (revisions 2 and 3). OQ-1 to OQ-4 (P1.32) touch only commitments, closure and admission, which S2a excludes.
3. **The contract is frozen** at a reviewed version, with every review correction applied and the validator passing
   on the corpus (check, revisions, self-test, sabotage).
4. **A crate decision is recorded** (D3: one crate, I/O-free core) with its name and workspace location, and an S2a
   budget authorised.

## Contract sections S2a must satisfy

| Section | Content | S2a obligation |
|---|---|---|
| §2, §2.1 | money, `High`, bases, rounding direction, `T_round` and `ToleranceRange`, FB-1 usage bound, FB-2 | full (checked `i128`, no floats, basis tie rule X-0) |
| §3 | evidence types for `Reservation`, `Settlement`, `StreamError`, `IdentityRefusal`, `Refused`; `Field<T>`; model canonicalisation MC-1 to MC-3 | consume as typed input; E-1 to E-5 are not S2a's |
| §4 | C-1 (including unusable `rid`), C-2 (including `Row` keys for no-reservation settlements), C-5 to C-9, C-11; `key_join` and `auxiliary` | full |
| §5 | settlement decision, the `Valid` conditions, reject reasons, transitions | full |
| §6, §6.1, §6.2 | obligation (including `None`), typed `amount_source`, release versus replacement, `Replacement`, forwarding, execution versus billing | full |
| §7.1, §7.2 | pricing primitives; V-1 to V-8 for OpenAI and Anthropic | full (V-3 tier set per R-03/Q5) |
| §8 | Table X rows for budgeted decisions | full |
| §9 | classification codes produced by S to X | the subset for budgeted attempts |
| §13 | canonical output and determinism | full |
| TRANSITIONS | Table S1, Table V, Table X (budgeted rows), "no second fallback" | full |
| INVARIANTS | F1 to F6 (F6 within a source), F10, F11, F12 | full, as properties |

## Corpus subset S2a must pass (attempt-level fields only)

**Selection rule** (applied by `validate_corpus.py results --scope s2a`): every `current_producer` case with at least
one attempt keyed `res:` (a reservation) in a session whose launch lane is `api`. `future_qualified_gate` cases are
excluded: their attempt facts duplicate current-producer cases. The validator prints the selected case ids. The P1.30
selection is recorded in `validator/s2a-subset.txt`.

**Attempt fields graded:** key, correlation (`kind`, `auxiliary`), evidence, settlement, obligation, valuation,
exposure, blocking, diagnostics, forwarding, execution, billing, replacement.

**Excluded from S2a grading:** attribution, objectives, commitments, admission and lifecycle. Cases outside the
selection rule stay in the corpus for S5 and S7.

## Exit criteria

- 100% agreement on the subset's attempt-level fields (`validate_corpus.py results --scope s2a`).
- Proptest properties for F1 to F6 and F10 to F13.
- Mutation testing: semantic equivalents of reader mutants K01 to K04, K15 to K21 and the F-table mutations, all
  caught.
- No I/O in the core. Code review against the contract.
