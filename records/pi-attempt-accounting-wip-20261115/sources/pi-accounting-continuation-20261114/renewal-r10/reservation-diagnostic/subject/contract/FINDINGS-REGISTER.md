# P1.29 finding register and revision-3 dispositions (P1.30)

> Historical P1.30 document, carried unchanged into revision 4 except for this note. NEW-1 and NEW-2 and OQ-1 to OQ-4
> are superseded by the P1.32 dispositions in `DECISION-REGISTER.md` (closure authority, idle and retired sessions).

**Sources, kept apart:**
- **Reviewer findings:** the sealed partial Astra response. P1.29 `review/sealed/reviewer-response.md`, sha256
  `dc8031ba…`, seal `8b7634b7…`, re-verified in P1.30. It is not reinterpreted here; quotations are paraphrased with
  the finding id.
- **Coordinator reproductions:** P1.29 `repro/repro_p129.py`, rerun in P1.30 with byte-identical output
  (`evidence/p129-rerun.txt`).
- **Newly inferred risks:** marked **NEW**. They are coordinator inferences made while repairing, not reviewer
  findings.

"Rev-2 result" is what revision 2 (`1.1-s1`) produces. "Rev-3 result" is the corrected result in `1.2-s1`. Every
"Check" is a corpus case with a validator rule, plus a sabotage or self-test that must fail when the repair is undone
(`validator/`).

## Interaction clusters

Findings are repaired by cluster, not one by one.

| Cluster | Findings | Shared defect | Shared repair |
|---|---|---|---|
| **K1 What makes money finite** | R-01, R-02, R-03, R-04, R-05, R-07 | revision 2 defined finiteness locally (per row of §8) and never carried it into completeness, rounding, tier sets, commitments, conflicting maxima or predecessor lines | one rule: **a high is finite only with a usage bound** (FB-1 reworded). Plus one attempt-level fact, **billing `Established`/`Unresolved`** (§6.2), which rollup, predecessor and commitment rules read instead of the obligation's execution state |
| **K2 Authority evidence is never filtered away** | R-08, R-09, R-12, (Q8 O-7) | candidate and admission rules filtered on ancestry, time or an implicit validator guard before classifying the authority evidence | explicit admissions are always candidates (O-3a); O-7 uses obligation intervals and clock trust; closure is an explicit first admission step (§12.4) |
| **K3 Lifecycle and commitment release** | R-10, R-11, R-04 | one exclusive lifecycle state carried two facts (terminal state, post-retirement execution); A5 forbade the release A-3 requires | lifecycle = terminal state + `reopened` fact; commitment release is an enumerated permitted exception to A5; open commitments are unenforced (R-04) |
| **K4 Totality of keys and correlation** | R-06, R-13, R-16 premise | keying gaps for malformed reservations; the settlement join confused with auxiliary error joins; `refused` rows taken as non-forwarding proof; S2a scope inconsistent with its subset | C-1/C-2 completed; `correlation.auxiliary`; forwarding from the attempt's own rows; S2a includes RS-1..RS-3 |
| **K5 Valuation details** | R-14, R-15, (R-01 Anthropic part) | V-3/V-4 text diverged from producer behaviour and from the corpus | null Anthropic tier is standard; residual writes min/max; contradictory split counts are not a usage bound |
| **K6 Money text** | R-02, focus 11 | MONEY.md proof and tolerance wording | integer argument for `T_round`; corrected inequality; `ToleranceRange` |

## Completed findings R-01 to R-15

| ID | Sev | Requirement | Counterexample (reproduced) | Origin | Rev-2 result | Rev-3 result | Clauses | Cases | Check |
|---|---|---|---|---|---|---|---|---|---|
| R-01 | P1 | FB-1; §8 usage rows; V-5 | VC01 with `terminal` not a success (status 200, valid counts): `Valid(Usage)` | rev 1 row, contradicted by rev-2 FB-1 | `Exact(15,000,000)` | `AtLeast(15,000,000)`, `UnresolvedExposure`, not established | FB-1; §8 rows 1–4; V-5; Table X | new `P130-R01-usage-incomplete-terminal` | validator Table X requires `complete` for usage `Exact`; sabotage "R-01 incomplete usage made Exact" |
| R-02 | P1 | FB-1 vs X-0 | gpt-6.1-terra flex, 21 cached tokens: valuation 525 nano, charge 1,000 nano | rev 2 (FB-1 wording) | contract contradictory | `Exact(1,000)` basis `ProxyCharge` over usage bound 525; admitted | FB-1; §2 rounding; X-0; MONEY | new `P130-R02-rounded-up-charge-admitted` (valid control) | validator accepts `Exact` with basis `proxy_charge` only over a complete usage bound; derivation re-priced |
| R-03 | P1 | V-3; Q5 | U08: `observed_tier` null | rev 2 (V-3) | low 7.5M, `Exact(30M)`, admitted | `Unpriceable{TierUnestablished}`, low 0, `AtLeast(30M)`, not established | V-3; §8; A6 | U08 changed; Q5 unchanged | validator: OpenAI tier missing or unrecognised gives `unpriceable`; sabotage "missing tier made finite"; self-test `false_finite_missing_tier` |
| R-04 | P1 | A-3; FB-2; Q10 | VC14: a 0.48 reservation passes the 0.5 cap; the same tokens served at priority cost 0.948, charged uncapped | rev 1 (A-3), exposed by FB-2 | open-session commitment `485,000,000` finite, admitted | commitment `AtLeast(485,000,000)`, `UnenforcedCommitment`, not established | §2.2; §12.3 A-2, A-3, A-5; RESERVATION-BOUNDS | VC14, S08-control, C1-control changed; new `P130-R04-priority-overrun-above-cap`, `P130-R04-retired-session-releases-commitment` | validator: open commitments `at_least`; self-test `cap_as_bound` |
| R-05 | P1 | F3; §8 rejected/conflicting | N01 plus a distinct duplicate settlement | rev 1 | `AtLeast(50M)` (valuation high 80M dropped) | `AtLeast(80M)`: max includes every usage observation's high | §8; V-8; F3 | new `P130-R05-conflicting-keeps-valuation-high` | validator Table X conflicting row; sabotage "conflicting drops valuation high" |
| R-06 | P1 | C-1, C-2; Obligation type; §8 | reservation with an unusable `rid`; malformed `reserved`; settlement without reservation; N01 `producer_shape` on a valid settlement | rev 1 | no rule / contradictory | Row key + `AtLeast(R or 0)` + `MalformedEvidence`; `Obligation::None{cause}`; NoReservation keys `Row`; correlation = key join, RS rules in `auxiliary` | C-1, C-2, §4 types, §6, X-0, F1 | new `P130-R06-reservation-rid-missing`, `P130-R06-reservation-amount-malformed`; 6 cases' correlation | validator F1 check (closed ⇒ exact key join); sabotage |
| R-07 | P1 | §12.2; D1 | D1-successor with the predecessor's attempt closed by `upstream_error`, session retired | **rev 2** | predecessor line `Exact(0)`, successor admitted | predecessor line `AtLeast(500M)`, successor not established | §6.2 billing; §12.2; A4 | new `P130-R07-closed-unresolved-predecessor-constrains-successor` | validator predecessor line from billing; self-test `lost_predecessor_obligations` |
| R-08 | P1 | O-3, O-7 | OWN-two-admitting-objectives with both trees empty | **rev 2** | no candidates; blocks nobody | candidates `[o, p]`; both blocked | O-3a; O-7; T2 | new `P130-R08-admissions-without-tree-still-block` | validator: admitting objectives ⊆ candidates; self-test `erased_admission_contradiction` |
| R-09 | P1 | §12 admission | U04: established, closed, within limit | rev 1 | contract admits; corpus refuses (unstated guard) | **refused** by closure precedence (Q11) | §12.4; A7 | U04 pair, D1-closed (rev 2 `not_established` → `refused`); new `P130-Q11-closure-uncertain` | validator closure step before headroom; self-test `closure_precedence` |
| R-10 | P1 | L-4, L-5; A-3, A-4 | N07-reopened plus a final dated retirement | rev 1 | two exclusive states | `retired`, `reopened: true`; commitment released; `ReopenedWithoutAdmission` blocks | §10; §12.3; Table L | lifecycle of 2 cases; new `P130-R10-reopened-then-retired` | validator lifecycle fields; reason implied by `reopened` |
| R-11 | P1 | A5 | N07-control-retired without its retirement: refused/not established; with it: admitted | rev 1 | invariant forbids required release | A5 lists permitted exceptions (valid retirement, valid release); forbidden additions still checked | A5; §12.3 | new pair `P130-R11-open-session-before-retirement` (permitted), `P130-A5-unreadable-line-added` (forbidden) | validator `evidence_addition_of` rank check |
| R-12 | P1 | validator O-7 | outstanding reservation 06:30, no retirement, candidate window from 06:40 | **rev 2** | validator demands candidates `[]` | candidate kept (interval open-ended) | O-7; §11.2; validator | new `P130-R12-outstanding-interval-keeps-candidate`, `P130-R12-untrusted-clock-keeps-candidate` | validator interval from obligation and lifecycle; self-test `timestamp_only_placement` |
| R-13 | P2 | §6.1; C-9 | refused row after identity refusal/upstream error; N04 header-write `stream_error` | rev 2 (§6.1), rev 1 (C-9) | N04 rid 2 `unknown` | `forwarded` | §6.1; C-9 | N04-budgeted-header-failure-own-reservation and every attempt with a `stream_error` | validator forwarding rule |
| R-14 | P2 | V-3 Anthropic | VC02 with `observed_tier` null | rev 1 | `MalformedTier`, rejected | standard; admitted | V-3; §7.1 | new `P130-R14-anthropic-null-tier-admitted` (valid control) | validator re-pricing; case |
| R-15 | P2 | §7.1 `anthropic_price` | unsplit residual writes | rev 1 (text only; corpus already min/max) | text: low at max rate | low at min, high at max | §7.1; V-4 | V02 cases (expectations unchanged) | derivation records re-priced |

## Partial and focus-area items

| Item | Status | Rev-3 treatment |
|---|---|---|
| **R-16** | **Unverified partial reviewer finding.** The text ends at "S2a excludes". Its severity and full claim are not established | Only its **premise**, taken from focus area 12, was checked: S2a excluded RS-1..RS-3 while its grading subset needed them (N01 ×5, N04 ×2). `S2A-ENTRY-CRITERIA.md` now includes RS-1..RS-3 in S2a. That does not confirm R-16's full content |
| Focus 6 (Table T vs §12.1) | confirmed (rev 1) | Table T: Before/After owned attempts stay in O's lines (P-5) and block with `AttributionContradiction` |
| Focus 11 (MONEY) | confirmed (rev 1) | corrected inequality; `T_round` justified by integrality; `ToleranceRange` replaces the unspecified scaling formula |
| Stale `Exact(R)` statements | coordinator-found (P1.29) | `S2A-ENTRY-CRITERIA.md` gate 2 and the F3 exception rewritten; whole-set search recorded in `evidence/consistency-pass.txt` |

## Newly inferred risks (NEW, not reviewer findings)

- **NEW-1 Closure stamp authority.** Q11 needs a definition of "authoritative closure". Revision 3 treats a well-formed
  `closed_at` no later than the reference time as authoritative. A malformed value, or one after the reference time,
  is `ClosureUncertain`.
- **NEW-2 Idle open session.** R-04 shows an idle open session can later exceed its cap. Revision 3 therefore treats
  every open or pending commitment as unbounded, not only sessions with a request in flight. This is the reading of
  "unbounded or unverifiable in-flight exposure blocks" that is consistent with R-04. It costs three admitted controls
  (VC14, S08-control, C1-control). Owner question OQ-1.
- **NEW-3 Contradictory Anthropic split counts.** `w5m + w1h > cache_write` means the counts contradict each other.
  Revision 3 treats such usage as not a usage bound (V-5). No current case changes: the producer keeps the
  reservation there, which is `AtLeast` already.
- **NEW-4 Known amounts already above the limit.** Revision 3 keeps `not_established` ahead of `refused`. The owner's
  Q11 answer covers closure only. Owner question OQ-2.
