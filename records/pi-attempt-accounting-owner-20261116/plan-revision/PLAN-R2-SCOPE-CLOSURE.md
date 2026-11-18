# S1 plan revision R2.1: scope closure and requirements transfer

R2.1 incorporates the independent Fable xhigh plan review `REVIEW.md` (ACCEPT_WITH_CHANGES,
sha256 `9d8fc453…`): findings F1–F10 are addressed in place; the change log is section 9.

Standing: revision of the full-realization plan
`/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/full-realization-r1/planning/PLAN.md`
(sha256 `d1e93793…`), prepared 2026-11-17 by the S1 owner, Pi session
`01a1e569-2805-72ff-b837-03d1577c405d`, under operator direction. It is a scope-closure and
requirements-transfer document. **Full S1 is not completed and must not be represented as
completed.** It authorizes no implementation in this accounting lane.

## 1. Operator decisions this revision implements (2026-11-17)

- **R2, legacy consumer.** The `role_host.py` / `pi_container.py` provider-execution route is
  intended for retirement in favour of the hosted architecture. Full S1 and S2a are not to be
  implemented solely to support that legacy consumer. Retirement requires an explicit cutover,
  disposition of live consumers and resumable sessions, and preservation of historical
  accounting evidence.
- **R1, future profile.** S1's obsolete future-profile contract is dropped as an implementation
  target. Its applicable financial invariants and regression cases are preserved and referred to
  the dev vertical for qualification against the actual hosted ledger. The future-profile
  contract is not redesigned in this campaign.
- **G1** (production-ledger gap found by the cross-check) stays with the dev-vertical owner; no
  accounting-lane implementation is authorized.
- No push. Budget stays bounded; any further independent review is costed before a ceiling
  increase is requested.

## 2. What is complete, and its standing

- All eleven historical R8 findings are **closed bounded** within the reviewed private subject
  `3883d04` (repository copy `experiments/pi-attempt-accounting/s1`, merges `46cf6a0d`,
  `b3577d7a`, not pushed). Acceptance records:
  `records/pi-attempt-accounting-owner-20261116/slice1/ROOT-ACCEPTANCE-SPEC-R1.json` and
  `slice2/ROOT-ACCEPTANCE-SLICE2.json`, including their explicit limits. The recorded process
  deviation (option-3 slices went beyond the first-stage scope of the original plan review
  without a prior design review of the narrowed plan) is in the private record
  `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice2/ROOT-ACCEPTANCE-SLICE2.json`
  (`process_deviation`), the guard and board seq 5861; **the repository copy predates it and
  repository sync is pending** (it lands with the next operator-requested records merge).
- 19 of 175 cases have independent owner dossiers; supplied outputs were corrected in 11 cases
  with attributed revisions (`oracle/r11-revisions/REVISIONS.json`); raw inputs are unchanged.
- Root admissions (conditional fixtures only): `slice2/ROOT-ADMISSIONS.json`, 10 entries: nine
  historical cases and one private control outside the 175
  (`R11-CTL-admission-namespace-grouped`). Consumer limit: `validate_corpus.py`
  `admission_namespace_control` compares `e.get('raw_input_sha256', digest) == digest`, so an
  entry lacking that field would match any inputs; all 10 entries carry it and the file is
  root-owned outside the subject, so this is latent, not exercised.

## 3. Disposition of every case and scope

The authoritative per-case/per-scope map is `CASE-DISPOSITION.json` (schema `/2`, sha256
`fe758096…`), generated deterministically by `build_disposition.py` (sha256 `3d3d81ef…`):

```
python3 -I build_disposition.py \
  /home/bline/code/work-engine/experiments/pi-attempt-accounting/s1 \
  /home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/full-realization-r1/derivation-stage/LEDGER.json \
  /home/bline/code/work-engine/records/pi-attempt-accounting-owner-20261116 CASE-DISPOSITION.json
```

The universe source is that `LEDGER.json` (sha256 `56d48f66…`), accepted by the predecessor as
`ACCEPTED_BOUNDED_INVENTORY_ONLY` (`ROOT-LEDGER-DISPOSITION.json`): it fixes the 175/374
identities, not financial truth. The generator also reads `slice2/ROOT-ADMISSIONS.json` beside
this directory. Selection rules: profile from the ledger; findings from owner dossiers and
`REVISIONS.json` (normalised to `R8-*-01`); invariants from a fixed finding-to-invariant map
plus a root-curated `INVARIANT_TAGS` list; retirement evidence from a root-curated list. The
curated lists are selection proxies, not exhaustive classifications: the history-only cases
remain available to the receiving owners. Every case and scope carries all applicable
dispositions:

| Class | Cases | Scopes | Meaning |
| --- | --- | --- | --- |
| Retained historical obligation | 175 | 374 | Immutable historical record (raw inputs, supplied outputs with revisions, dossiers, reviews, receipts) is preserved; never deleted or rewritten at legacy retirement. |
| Full S1 qualification | 175 | 374 | **Superseded**, not completed: no full derivation, full-dossier consumption or complete A/B/C review will be done for the legacy consumer (R2) or the future profile (R1). |
| Transferred qualification (dev vertical) | 73 | 158 | Regression scenarios for qualification against the hosted ledger: all 34 future-profile cases plus 39 current-producer cases tagged with at least one transferred invariant. |
| Transferred evidence (legacy retirement) | 12 | 24 | Root-curated current-producer cases about retired, resumed and reopened legacy sessions (now including U03 and its control); inputs to the cutover's disposition of resumable sessions. |
| Retained history only | 90 | 192 | Current-producer cases with no transfer; record retained, qualification superseded. |

Breakdown: 141 current-producer cases (18 derived, 39 transferred, 12 retirement evidence, 90
history only; the classes do not overlap) and 34 future-profile cases (1 derived, all 34
transferred). Findings R8-OWNER-01, R8-SCHEMA-DOMAIN-01 and R8-DOC-01 are S1-internal and do not
trigger transfer.

## 4. Requirements transferred to the dev vertical (owner `01a1e2a4`)

Transfer is a referral, not acceptance. The dev-vertical owner decides whether and how to
qualify each item against the hosted ledger's actual semantics; S1's expected outputs are
evidence about the S1 contract, not expectations for the hosted ledger.

1. **Financial invariants** (stated ledger-independently; the read-only cross-check
   `ledger-crosscheck/REVIEW.md` found five upheld and one gap):
   - **I1** retained raw charge cannot be erased or replaced by a smaller reservation (gap
     **G1**, already handed off, board 5863; closest S1 analogues `P133-A01-incomplete-usage-above-reservation`,
     `P130-R04-priority-overrun-above-cap`);
   - **I2** malformed/missing/null/empty identity cannot yield a complete or favourable settlement;
   - **I3** no self-declared authority (admission, route/payer, grants, scopes, ordering) and
     non-empty identities before equality;
   - **I4** placement from raw events and times;
   - **I5** distinct money evidence is not merged by equal amounts, and exact copies do not double
     a hold (scoped to money/holds: S1 E-1 still counts byte-identical failure lines as separate
     evidence items, e.g. `N04-duplicated-read-failure`);
   - **I6** uncertainty never releases a hold.
   Glossary from S1 finding to invariant: FIN-01→I1; IDENTITY-01→I2; NAMESPACE-01,
   GATE-IDENTITY-01→I3; COVERAGE-01→I3, I6; PLACEMENT-01→I4; RESERVATION-01→I5;
   LOW-INVENTORY-01→I6.
2. **Regression scenarios**: the 73 transferred cases. Each entry in `CASE-DISPOSITION.json`
   gives `case_file` (relative to the repository root, under
   `experiments/pi-attempt-accounting/s1/corpus/cases/`), owner `dossiers` (relative to
   `records/pi-attempt-accounting-owner-20261116/`), `findings`, `invariants`, and
   `root_admission_entry_ids`. Nine transferred cases (A06, P130-R07, P132-CP-closed-predecessor-outstanding,
   P132-FG-VC15, P9, V01, P130-T, P130-R08, P130-R02) rest on conditional-fixture root admissions;
   their S1 expectations hold only under those test premises. The 34 future-profile scenarios
   carry no per-case invariant tags (33 have no finding link); they concern gate qualification,
   authority identity, test-mock billing bounds, reopening and retirement under a gate, and the
   receiving owner maps them, with the hosted lifecycle gate contract
   (`planning/dev-vertical-hosted-gate-contract-20261116.md`) as the comparison point.
3. **Not transferred**: the S1 future-profile contract text (ExecutionGate, BillingBound, G-1..G-8,
   A-2..A-5) as a design to implement. It is superseded by the hosted architecture.

## 5. Requirements for the legacy-route retirement (owner: the dev vertical, by operator agreement 2026-11-17)

The role-host route is in active use (206 Pi events in the role-host ledger, latest
2026-11-17T00:17Z, from manual-orchestration roles). The owner's first step is scoping: the
live consumers and any gap between their uses and the hosted path, which today serves coding
jobs.

The operator's retirement conditions, with the S1 evidence that bears on each:

- **Explicit cutover**: no retirement by drift; a recorded switch of each live consumer to the
  hosted route.
- **Disposition of live consumers and resumable sessions**: S1's producer fact (CONTRACT §2.3,
  release `92a03789`): a retired Pi session can resume without objective admission under its
  original cap; retirement and idleness are labels, not enforcement; a declared cap is not a
  billing bound (§2.2). The 12 retirement-evidence cases exemplify this (for example
  `U03-retirement-before-turn-evidence`: turn evidence after retirement). Cutover must close or
  fence resumable sessions, not only stop new launches.
- **Preservation of historical accounting evidence**: role-host Pi proxy logs and ledgers, the
  `usage_ledger.py` Pi readings and their pinned reader identity, and the S1 records; usage
  history must remain readable after the route is removed.

## 6. Explicitly superseded work (not done, by decision)

Derivation of the other 156 cases; full-dossier consumption by `check_corpus`; wiring dollar_ticket
admissions into the actual check path and an admission input for Rust `evidence`; regeneration of
the 149 schema-stale oracle bundles; R8-COVERAGE-01's independent raw-membership half;
`r8-completion-A/DERIVATIONS.json` repair and pilot re-admission; canonical INITIAL-REFERENCE text;
the 19 null acknowledgements; the original A1/A2b/B2/C1/C2 review omissions and complete
exact-current A/B/C review; binding the derivation-stage `INTERFACES.md` proposal (only the
dossier shape was bound, by root, in ROOT-DECISIONS-R1); the slice-1b review P3-2 (Python `re`
`$` accepts a trailing newline in reference validators); S2a entry and any production reducer
from S1. Note that the `--pilot` route still consumes the stale `r8-completion-A/DERIVATIONS.json`
payload, so the subject holds two inconsistent records for five scopes (LOW-INVENTORY-01's
corrected bundles and LOW-ASSERTIONS versus that file). Each remains open as S1 work, and S1 is
closed without them.

## 7. Acceptance of this revision

- Root records the revision and posts both transfer packets on the board to the dev vertical
  (invariants and regression scenarios; retirement requirements and evidence).
- The independent Fable xhigh review covered retroactive conformance of the option-3 root
  decisions and found them consistent with the reviewed design, the deviation correctly
  remedied by disclosure rather than reversal (REVIEW.md, answer 5).

## 8. Budget and review cost (as costed before the review)

Spend: owner tree 120.46 plus predecessor 68.53 = 188.99 of the 200 ceiling (about 11.0 left)
at the time of writing. Measured costs of comparable reviews in this campaign (Pi-recorded,
API-equivalent): Fable xhigh 13.62 (first code review, incl. an interrupted start), 8.53
(slice-2 review with execution), 7.18 (retained re-check); Opus xhigh 3.06 (ledger cross-check).

- **Fable xhigh review of this revision** (fresh, read-only, inputs: this document,
  `CASE-DISPOSITION.json` summary and spot checks, SCOPE-EXPLORATION, the two acceptance records,
  the original PLAN): estimated **7–10**, best estimate 8.5 from the two fresh Fable runs.
  It fits under the current ceiling only with almost no margin (about 11 left, before root
  costs of collecting it, about 1).
- **Opus xhigh alternative** (the operator's default for normal reviews): estimated **3–5**.
- A ceiling increase would be needed only if the Fable review is chosen and its cost exceeds the
  remaining allowance; root would request a specific increase of 5 to cover the upper estimate
  plus collection.

Actual: the Fable xhigh review cost 10.00 (within the 7–10 estimate); the operator's choice of it
raised the ceiling to 205. Owner-tree spend after applying the review: 131.255538, plus predecessor 68.53.

## 9. Changes in R2.1 (plan review findings)

- F1 (P1): §2 cites the private record holding the process deviation and states repository sync
  is pending.
- F2 (P1): invariant tagging added (finding map plus root-curated `INVARIANT_TAGS`); transfers now
  follow invariants, adding 22 current-producer cases including the G1-shaped ones; curated lists
  are stated to be proxies.
- F3 (P2): U03 and its control added to retirement evidence.
- F4 (P2): glossary, `invariants`, `case_file`, base paths and `root_admission_entry_ids` added;
  S1-internal findings no longer trigger transfer; future-profile cases' missing per-case tags
  stated.
- F5 (P3): finding strings normalised. F6 (P3): universe source, standing and invocation cited.
  F7 (P3): §6 adds C1, INTERFACES.md, P3-2 and the pilot-route inconsistency. F8 (P3): admissions
  split stated. F9 (P3): admission-consumer leniency disclosed. F10 (P3): I5 scoped to money/holds.
