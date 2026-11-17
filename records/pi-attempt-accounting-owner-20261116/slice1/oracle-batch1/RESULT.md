# Oracle batch 1: result

Scope: 8 whole historical cases and their 26 required scopes. The scope set was independently recomputed from raw evaluations (2 per objective configuration) and equals the derivation-stage UNIVERSE entries for these cases. Root admits, joins, reviews and accepts. No subject or predecessor file was modified. There were no network calls, no paid APIs, no commits and no child agents. The output directory is not a git repository, so nothing is staged.

Deliverables:
- `dossier/<case>.json` (8 files);
- `DOSSIER-SCHEMA.md`;
- `DERIVATIONS-BATCH1.md`;
- `AGREEMENT.json`.

Support material:
- `sourcerefs/` (byte-exact coordinates);
- `raw-views/` (output-free case views);
- `tools/`: navigation scanner, raw-view filter, and a dossier assembler that attaches SourceRefs to authored literals and fails closed on lexeme mismatch. It computes no money.

`Unexamined` does not appear anywhere: every family claim and every scope low is DerivedNominal or RequiredUnmet.

## Counts by status

| Unit | DerivedNominal | RequiredUnmet |
|---|---|---|
| Claims (153) | 128 | 25 |
| Case families (8 × 7 = 56) | 33 | 23 |
| Scope lows (26) | 22 | 4 |
| Scope highs (26; reported separately) | 16 | 10 |

Family status per case (D = DerivedNominal, R = RequiredUnmet). Columns are E (raw/identity/cardinality), NS (namespace/grants), PM (primary membership/validity/exact F), VA (valuation/retained amounts), T (ownership/candidates/lifecycle/placement), G (gate/commitment/provisionality) and CO (continuity/order).

| Case | E | NS | PM | VA | T | G | CO | Lows D/R |
|---|---|---|---|---|---|---|---|---|
| P9 | D | R | R | R | D | R | D | 0/2 |
| A06 | D | R | R | R | D | R | D | 0/2 |
| V01 | D | R | R | R | D | R | D | 2/0 |
| VC08 | D | R | R | D | D | D | D | 2/0 |
| P130-R07 | D | R | R | R | D | R | D | 4/0 |
| P132-FG-VC15 | D | R | R | R | D | R | D | 4/0 |
| P132-CP | D | R | D | D | D | D | D | 8/0 |
| N07 | D | D | D | D | D | D | D | 2/0 |

Inapplicability was derived, not skipped:
- N07: namespace, primary and valuation do not apply because there are zero origins.
- All 8 cases: continuity does not apply, because there is no predecessor accounting subject. P132-CP also carries a cross-evaluation equality claim.

## Every RequiredUnmet, with owner (all owned by root)

**D-SCOPE: admit the per-case conditional initial_reference_scope.** This covers 6 cases and 23 of the 25 unmet claims. The scope is dollar_ticket, same_origin_only, covering all origins of `S1/turn-1`. It is declared only in the bundle InputProof, with authority_ref `ROOT-ADOPTION-R6.json` or `ROOT-POLICY-R6.md@3dbe5356`, and neither document admits per-case scopes. I checked every declaration against the raw bytes: namespace, extent and duplicate rule (NS2, DerivedNominal). Every dependent claim carries a fully authored `consequence_if_admitted`.
- P9: NS3, PM2, VA3, G3; both scope lows; both highs.
- A06: NS3, PM2, VA3, G3; both scope lows; both highs.
- V01: NS3, PM2, VA3, G3; both highs. The lows hold in both branches.
- P130-R07: NS3, PM2, VA3, G3 (p committed), G5 (s predecessor_obligations); 2 p highs.
- P132-FG-VC15: NS3, PM2, VA3, G4 (p provisional), G5 (s standing and admission); 2 p highs.
- P132-CP: NS3 only. It does not decide anything here, because a single origin gives the same hold either way.

**Q-NAMESPACE: VC08 admission-namespace conditional grant.** This covers VC08/NS3 and VC08/PM2 (whether there are one or two codex units). Money is the same in both branches: codex pool, no money. The declared dollar_ticket contract is shown inapplicable.

Admitting D-SCOPE turns 23 of the 25 unmet claims into nominal claims, and every unmet scope low and high in this batch. The same decision shape recurs across the corpus: the derivation stage located 172 such payload references.

## D3 conflicts

None. Every claim was stated with fixed typed records, finite sets, max/min and single additions or subtractions. I implemented no reducer, interpreter, solver or adapter. Every throwaway check was a single product, sum or SHA-256 of canonical JSON (for gate payload binding), and none of them is the source of a claim. The one mechanism that needed care is branch invariance: comparing the two premise branches by hand. That stays finite here (2 branches, at most 2 origins). For larger multi-origin or multi-session cases it is still finite, but much more laborious; see the estimate below.

## Historical findings touched (all eleven stay OPEN)

| Case | Direct | Bears on |
|---|---|---|
| P9 | R8-FIN-01: 700M charge retained by origin `:2` in every branch; AtLeast700M and 300M commitment if admitted | R8-COVERAGE-01, R8-OWNER-01, R8-PLACEMENT-01, R8-RESERVATION-01 |
| A06 | (none) | R8-FIN-01 (250M retention), R8-LOW-INVENTORY-01 (15M survives rejection, L4), R8-OWNER-01, R8-RESERVATION-01 |
| V01 | R8-IDENTITY-01: priced multiple identity 20M–80M stays finite, Exact80M if admitted (M1) | R8-COVERAGE-01, R8-RESERVATION-01 |
| VC08 | R8-NAMESPACE-01: dollar_ticket declared on admission-namespace origins (**disagreement**) | (none) |
| P130-R07 | (none) | R8-PLACEMENT-01 (closure not established, so open window and inside), R8-RESERVATION-01 |
| P132-FG-VC15 | R8-GATE-IDENTITY-01: present, nonempty, equal triple; test_mock payload binding verified | R8-COVERAGE-01, R8-RESERVATION-01 |
| P132-CP | R8-PLACEMENT-01: [06:30, Open) straddles the established 06:40 closure (**disagreement**) | R8-RESERVATION-01 |
| N07 | (none) | (none) |

R8-DOC-01: counts here are reconciled per claim and per scope. R8-SCHEMA-DOMAIN-01 and R8-OWNER-01 (constructor ownership) are spec-side and not exercised beyond C-owned validity claims.

## Agreement with supplied outputs (read only after derivation)

There were 150 agreements, 4 disagreements and 25 absent comparisons.

Disagreements (findings, not fixes):
1. VC08/NS1 and NS2: the bundle declares dollar_ticket for admission-namespace origins (R8-NAMESPACE-01).
2. P132-CP/T3: supplied placement is `inside` in both evaluations, but the raw interval straddles the closure (R8-PLACEMENT-01).
3. P132-CP/G3: p's supplied reasons omit PlacementStraddles in both evaluations (R8-PLACEMENT-01).

All supplied scope lows, seeds and alternatives agree. Supplied unit structure equals the D-SCOPE-admitted branch everywhere. That agreement is not admission.

One oracle self-correction was made after derivation. VC08 replacement went from `not_applicable` to ineligible/NoReservation, because `types.Replacement` has only eligible and ineligible variants and F-REPLACEMENT-1 applies. The reason is recorded in the claim.

Label notes: the supplied admission decision is `admitted` where §12.4 prose says `admit`. K diagnostics (e.g. A06 `IdentityAbsent`, `PartialUsage`) are outside the seven families.

## Disclosures

- **Output exposure before derivation.** In my first structural dump of P9 (before I built the output-free raw views), I accidentally printed the first ~600 characters of P9's top-level `expected` (attempt high at_least 700M proxy_charge, low 15M) and the first revision entries. All later reads excluded outputs until AGREEMENT. The P9 claims rest on raw lexemes plus policy. The 700M/300M consequences are also stated independently in root policy (AMD §3 P9-RETAINED-CHARGE; FRPLAN finding matrix R8-FIN-01). No other case was exposed.
- **Guard drift.** `ARCHITECTURE-GUARD.md` now hashes to `507637b9…`; the bound value is `2c4063d3…`. The file is append-only. It was not re-read, and no claim depends on it.
- **Interpretive residual risks:**
  - N07 legacy `objective_admission` commitment state: retired_resumable via O-1 equivalence. The alternative pending-like reading gives the same 800M AtLeast; only the label differs.
  - Valuation label `not_reported` vs `not_applicable` for dollar attempts with no usage (P130-R07, P132-CP). Money is the same under both by the X-BRANCH guards.
  - `consequence_if_admitted` for unadmitted-branch sketches (P9, A06) is narrative, not a claim.
- **Subject integrity.** All 8 case files still match their bound SHA-256. No non-`.git` subject file is newer than PACKET.md. `subject/.git` has newer worktree metadata (`worktrees/spec-r1`). That was not created by this worker, which ran no git commands. It is probably a concurrent spec worktree; root should confirm.
- **Infrastructure.** One provider usage-limit interruption mid-run. Work resumed from the saved outputs without redoing anything; no rejection is counted.

## Reuse attributions (each checked before use)

- Derivation-stage RAW-ORIGINS/LEDGER/FRONTIERS: case digests, line spans, line digests and located InputProof/payload fragments. All re-verified by independent scan or SHA-256, with 0 mismatches. Used for navigation only.
- `oracle/r9-pilot/claims.json` P9-RETAINED-CHARGE: component list 400/500/700/15M with origin refs and raw digests. Re-derived from raw lexemes, and the digests match.
- `oracle/R8-RAW-DERIVATIONS.md` P9/A06: 15M local lows. Re-derived with the pinned rates.
- derivation-stage DERIVATIONS.md N5 and N6: the VC08 namespace mismatch and the P132-CP straddle argument. Premises re-checked; the closure establishment and the open interval were derived again.
- Base plan §8 V01 statement (Exact80M, low 20M): a policy statement, checked against independent arithmetic.

## Cost measurement and estimate for the remaining 167 cases

Measured for this batch: roughly 1 to 1.25 hours of active worker time, excluding the interruption, spread over 2 context windows. About half was one-time reading of policy, contract and canonical C–K, about 300 KB in all. About half was per-case authoring and checking, roughly 5 to 7 minutes per case, using the shared templates. Output is 153 claims and 26 scope records, about 19 claims per case.

Reusable across cases now: the scanner and SourceRef extraction, the assembler and the claim templates (E1–E3, NS1–NS3, lifecycle, ownership, gate-none, continuity), the PREM-SCOPE premise shape, the branch-invariance method, and the AGREEMENT script pattern.

Done per case: reading the raw view (2–5 KB), validity and valuation specifics, the commitment/predecessor/admission arithmetic, branch analysis, and the agreement verdicts.

Estimate:
- Simple single-session dollar or subscription cases (about 60–70% of the corpus, by family names): about 5–10 minutes each.
- Multi-origin, conflicting, RS-anchor, stream-error, unreadable-line, multi-session or uncertain-ownership cases (the P130/P133 attribution families, N01/N04, DUP, VC11, C3, ORD and others): 20–40 minutes each. Exhaustive candidate and U inventories and Atomize triggers have to be hand-authored, and a second reader is advisable.
- Total: roughly 25–40 worker-hours, or about 12–20 worker sessions at 8–15 cases per session.

Realizable within D3: yes, as far as this batch shows. No case needed a reducer. The cost is dominated by authoring, and multi-origin branch analysis is the main source of error risk.

The largest single lever is a root decision on D-SCOPE before the next batch. If it stays unmet, roughly 75% of money-bearing claims across the corpus will stay RequiredUnmet, and authoring both branches roughly doubles the cost of multi-origin cases.

Recommended next step for root:
1. Bind the dossier schema, or revise it.
2. Decide D-SCOPE (and Q-NAMESPACE).
3. Then dispatch the remaining cases in partitions that share templates: simple dollar, subscription, P130/P133 attribution, and multi-origin conflict.
