# Architecture guard: Pi attempt accounting (financial semantics)

**Authority:** owner rule of 2026-11-13, given during P1.34: escalate on architectural blindness. This file is
append-only. It applies to every later repair or implementation dispatch for this subsystem, whether a contract
revision, a reader repair or S2a. Product enforcement is not claimed; this is a coordinator dispatch control.

## Rule

| Signal | Required response |
|---|---|
| First independent rejection with serious defects (P1 or blocking P2) | Record an architectural-risk warning; allow one bounded ordinary repair |
| Second rejection after a repair, within the same subsystem | Block ordinary repair; require an Astra xhigh architectural/design review first |
| Confirmed serious regression caused by a repair, or recurrence of the same invariant violation | Escalate to design review immediately, regardless of count |

Further rules:
- Infrastructure interruptions (Flex capacity, reviewer budget stops, transport, upstream cutoffs) are recorded but are
  not rejections. Findings confirmed offline from such a run stay evidence.
- Inherited defects count toward recurring-boundary evidence. An "inherited" label never removes evidence.
- Renaming a subsystem, revision or finding does not reset history.
- A triggered gate permits review, diagnosis and preservation. It forbids ordinary repair dispatch and implementation.

## Subsystem and invariant identity

- **Subsystem:** the financial semantics of Pi attempt accounting, meaning settlement, reservation, valuation and
  exposure. Two implementations of the same responsibility count together: the Python ledger reader (A1 to A1.3.4) and
  the S1 contract for the pure Rust reducer (revisions 1 to 5).
- **Invariant I-1:** uncertain, incomplete, rejected or diagnostic evidence never lowers conservative exposure or a
  retained reservation. In S1 this is F3 / X-2 and F13.

## Evidence history (from round reports; verdicts as recorded there)

| Round | Subject | Outcome | Counts as | I-1 evidence |
|---|---|---|---|---|
| P1.9 | Unit A (reader) | REJECT, 5 P1 | rejection 1 (reader) | — |
| P1.11 | A1 | REJECT, 5 P1 | rejection after repair | A1-01: `upstream_error` and other non-request settlements erase reservations (inherited); A1-06: cacheWrite lowers a charge |
| P1.13 | A1.1 | REJECT, 5 P1 | rejection after repair | not established here |
| P1.15 | A1.2 | REJECT, 6 P1 | rejection after repair | not established here |
| P1.17 | A1.3 | incomplete (T-1 429 race) | infrastructure, not a rejection | — |
| P1.19 | A1.3 | incomplete (budget stop); C1/C2 P1 confirmed offline | infrastructure; findings stay evidence | not established here |
| P1.21 | A1.3.1 | REJECT | rejection after repair | not established here |
| P1.23 | A1.3.2 | REJECT | rejection after repair | not established here |
| P1.25 | A1.3.3 | REJECT; N-01 P1 **regression caused by the A1.3.3 repair** | regression trigger | N-01: an identity issue trips a reservation fallback and lowers the high bound (0.05 against 0.08) |
| P1.27 | architecture | design change to a pure Rust reducer from a normative contract (owner-approved) | design escalation (not an Astra xhigh review) | — |
| P1.29 | S1 revision 2 | partial REJECT (Astra xhigh, cut at 900 s), 12 P1 | rejection 1 (contract) | not established in this file |
| P1.32 | S1 revision 4 (after the revision-3 repair) | Module A REJECT, 4 P1 | **rejection 2 after repair** | A-01: `Valid(Usage)` without a usage bound lowers the retained reservation, 500M → 30M |
| P1.33 | revision 5 | ordinary repair cycle (owner-authorised before this rule existed) | — | — |

## Trigger state at P1.34 launch: TRIGGERED

- **Rule 2:** two rejections of the S1 contract, with a repair (revision 3) between them. On the reader there were
  repeated rejections after repairs.
- **Rule 3:** I-1 recurred across both implementations (A1-01, N-01, A-01), and N-01 was a repair regression.
- The P1.27 design change was a redesign, but not the Astra xhigh architectural review the rule requires. I-1 recurred
  after it.

## Consequences

- **P1.34 Module A is permitted.** It is a review: one request, high effort, owner-authorised.
- **No ordinary revision-6 repair cycle** until a fresh Astra xhigh architectural/design review has assessed the
  contract's structure against this history.
- **S2a needs a recorded design-review decision bound to this history** before it starts, whatever Module A concludes.
  Narrow acceptance or passing tests cannot close the risk on their own.
- **Not authorised here:** the architectural review itself needs owner authorisation.

## Appended 2026-11-13T17:20Z after P1.34 Module A (sealed `d515e5f3…`)

| Round | Subject | Outcome | Counts as | I-1 evidence |
|---|---|---|---|---|
| P1.34 | S1 revision 5 (after the P1.33 repair) | Module A REJECT, 3 P1 and 6 P2, all reproduced; reviewer assessment `local_corrections` | **rejection 3 after repair** | A-01: §6.2 lowers error-status exposure with supported usage, a **regression caused by the P1.33 repair** (revision 4 §6.2 and §8 agreed). A-02: N03 releases R = 300M on a contradicted identity (inherited; the P1.33 rule was not propagated) |

**Trigger state: TRIGGERED, reinforced.**
- Rule 2: a third rejection after repair.
- Rule 3: a confirmed repair regression (A-01), and I-1 recurring a fourth time (A-01, A-02).
- The reviewer's `local_corrections` assessment is evidence for the architectural review. It does not close the gate.
- Coordinator hypothesis for that review: repairs fail to propagate across multiple statements of one rule (prose,
  tables, corpus, validator, labels).
- **Ordinary revision-6 repair stays blocked** until an Astra xhigh architectural/design review, which is not yet
  authorised.

## Appended2026-11-13 after P1.35 Astra xhigh architecture review

Completed review e60d2cb6, sealed response01e3c3cc8, seal816788d4; one request, no infrastructure failure. Verdict **REVISE_SPECIFICATION_ARCHITECTURE**, primary B; retain financial model, revise normative ownership and independent qualification. Root accepts bounded canonical C-K consolidation plus independent raw-evidence oracle and trusted gate interface. Authority: user explicitly delegated ownership and future development to root01a1d6e2. Full decision: `/home/bline/cbm-work/pi-accounting-sprint-20261113/ARCHITECTURE-DECISION.md`.

**State: DESIGN_REMEDIATION_REQUIRED.** Architecture-led work is permitted within the decision; another ordinary manually synchronized repair is not. Revision6 freeze and S2a remain gated by design realization, independent qualification and exact-current modular review. History, inherited defects and recurring I-1 remain evidence. No rejection counter is reset.

## Appended2026-11-13 after revision6 ModuleA current review

Complete Astrahigh review5e169aea (response2685b18c/seal7cbab070) requires changes:1P1,5P2. Root confirms A01: contradicted subscription lane table permits PiApi/AtLeast0 despite a positive well-formed reservation; INELIGIBLE-EXPOSURE retainsR. The same row is inherited from frozenrevision5CONTRACT:522. This is recurringI-1 evidence and counts despite inheritance. New canonical storage has not fully localized semantic branch/type/consumer authority. A02partitiongap,A03readsetmismatch,A04valuationstate mismatch,A05writerlabels,A06provisionality ambiguity also recorded. Full disposition /home/bline/cbm-work/pi-accounting-sprint-20261113/reviews/module-A/ROOT-DISPOSITION.md.

**State: DESIGN_REVIEW_REQUIRED (immediate recurring-invariant escalation).** No ordinary repair, revision7mutation or S2a until a fresh Astraxhigh design judgment and root disposition of this realization. Review bounded_correction assessment is evidence, not a waiver. ModuleB/C not launched and remain unestablished. Infrastructure/setup failures excluded; no history reset. Current root authority is delegated user ownership and100proxy/four-hour sprint.


## DESIGN_R2 root adoption2026-11-13T23:32Z

Fresh Astraxhigh5b9f03b7 completed exact packetf7d70c4a: REVISE_PLAN, primary decision/projection ownership and qualification interfaces. Response2c140d38/seal163eb9cc retained. Root adopts bounded result-bearing declaration/projection and causal-interface redesign at /home/bline/cbm-work/pi-accounting-sprint-20261113/r7/PLAN-AND-ROOT-ADOPTION.md sha256 05e2c10e72e6ffc8495ff9a61334ef4801041e71ef1eea5b9d8fb7a6005348d4. R6 stays rejected/immutable. State DESIGN_REMEDIATION_REQUIRED; only architecture-led R7 implementation authorized, ordinary repair/S2a/acceptance still blocked. Inherited recurringI1 and all rejection history retained. This design judgment is not a further implementation rejection; infra/model/transport failures excluded. No reset.


## Infrastructure-count correction 2026-11-13T23:46:49.108680+00:00

Root re-audit applies the user rule to the historical P1.29 row: the verified handoff explicitly records an incomplete review cut at900.9s. The interrupted attempt itself must NOT count as an implementation rejection. Supersede the table classification `rejection1(contract)` and any numeric threshold argument that depends on counting that interruption. Its reported12P1 and any independently confirmed findings remain attributed evidence; confirmed recurring invariants may still trigger immediate escalation. No findings are silently accepted from a cutoff. P1.32/P1.34 and complete revision6ModuleA findings, confirmed repair regressions and repeated inheritedI1 remain independent grounds for the current DESIGN_REMEDIATION_REQUIRED state. This correction fixes event classification; it does not reset valid history or clear a gate. P1.17/P1.19 and failedClaudeR7quota turns likewise remain infrastructure, zero rejection increments.


## Revision7 ModuleA escalation 2026-11-14T00:31:53.727802+00:00
Complete current Astraxhigh783244bb review (responseb4b1ed68/seal292ebec3) CHANGES_REQUIRED,2P1/7P2, architectureassessmentdesign_review_required. Root confirms A01: two orphan Row settlements500M each becomeone conflictingReservation group500M afteraddingR, reducingaggregate1000Mto500M withouteligibleusage. This is recurringI1 evidence at Cidentity/merge→Xaggregation, originunestablished. Root also confirmsA02 supportedvaluationlow15M/20M/60M erasedto0 inexpectations despiteowningX25; A05 wrongSconstructor mutant passesstructuralcheck; A09publishedschema rejectsboundaries (see root-reproduction/REPRODUCED.json for actualstanding). State DESIGN_REVIEW_REQUIRED immediately. Noordinaryrepair/R8mutation/S2a. Need an architectural design judgment of stablecorrelationidentity or explicitmergeaccounting and external/corpus qualification ownership beforeimplementation. Aalreadyxhigh identifiesneed/alternatives butdoesnotacceptanewplan. B/Ccurrentcandidate reviewsnotlaunched; prep isunaccepted. Rootauthorityoriginaluserownership100/deadline01:51:14; noreset/inheritedwaiver; infrastructureexcluded.

## DESIGN_R3 judgment — 2026-11-14T01:00Z

Fresh Astra xhigh Pi91e307d2 completed exact plan24312298 review: REVISE_PLAN, implementation_under_plan_recommended:false. Responsee075d0c82fd46258e3a1a64ff747e54c5c743b52272609c37f2ebc9e3a1786af; seald0cdecf45deade326ce1eae0e121db41c686968c37365221bacfb4e440e781f4. Two P1 design defects and one P2 interface gap remain: auxiliary association can still reduce aggregate80M to51M; late discovery of an earlier reservation contradicts unchanged I1 and settlement transitions; new scope/continuity/multiplicity interfaces need exact ownership. Root independently read C5-C8/V1/V8/SETTLEMENT/INV-F3 and pinned rates, confirming causal premises and20M/80M arithmetic, without claiming a reducer execution. Root adopts design rejection. Stable primary keys alone are insufficient; new plan must provide compositional monetary conservation and lawful eligibility discovery.

This is a rejected architecture candidate before implementation, not an additional implementation-rejection count. State stays DESIGN_REVIEW_REQUIRED; no ordinary repair, R8 mutation or S2a. Inherited/current evidence and all prior serious implementation rejections remain. Required watcher79687 collected0, exactidentity/effort/prompt/seal verified, Pi retired. Native planner retained for read-only R4 after152184+55000 context check; original6proxy allowance and01:51:14 deadline remain. No waiver from successful mechanical tests or running a review.

## DESIGN_R4 judgment and R5 owner policy subject — 2026-11-14T01:24Z

Fresh AstraxhighPi1a2a7e39 completed REVISE_PLAN1P1/2P2, response10679c678e8baea3a9d47566b26d8b91a92f1a0e9a768547855d678817e9f8f7, seal9b1e9501774072be9728021498f1dfe583b883c2446993673b5dd3a8bb9944f4. Monotone financial candidates addressRS80→51. Remaining issues: non-settlement prerequisite release changes oldliteralI1; generalinterpretation ownership/discharge/completeness insufficient; jointenvelope fails to specifyperattempt/additiveR low/high interface. Rootadopts design rejection. No additional implementation-rejection increment, no R8/ordinaryrepair/S2a. Watcher29027collected0/Piretired,2.66138proxy.

Root explicitly resolves policy conflict for a NEW R5 design subject in /home/bline/cbm-work/pi-accounting-sprint-20261113/r7/ROOT-POLICY-R5.md SHA57303fdb466c49ebe8f0d6fa095628c57c3f2018603114e12949af663f65eec5. This amends old INV-F3 rowkind/Rejected-to-Valid restrictions, not a claimunchangedliteralI1: only a complete affirmative exactcoverage eligibilitywitness mayrelease, whichever prerequisitearriveslast; uncertain evidence/diagnostics/scope assertion/subject rename alone neverrelease. Stableconditionalreferenceinitialscope andmonotonefinancialrelevance selected; generaljointinterpretationsnotselectedwithoutnecessity/completeconsumercontract. Newpolicy MUSTbeindependentlycriticized for safety/coherence beforeplanimplementation; no reviewlaundering, retroactiveR7acceptance or userarchitecturalguardchange. Retained read-onlyR5planner context186165+30kretain_ok,8mintarget. GuardremainsDESIGN_REVIEW_REQUIRED.

## R5 response and sprint disposition —2026-11-14T01:45Z

R5 remainsunaccepted: response96315599/seal74163160 REVISE_PLAN2P1/1P2. Root adopts source-derived refinement515→500, unsupportedduplicate-low30vspossible15, and weakRSco-reference findings; no reducerexecution claimed. Explicitrootpolicy57303fdb is recognizedcoherentprinciple, mechanismnotaccepted. Hostproxyterminalrowmissing/accounting_unknown/unresolvedadmission1 despite sealedcompletedPiresponse; no verifiedHTTPterminal/settledcostclaimed, infrastructurerowgap incrementsnoimplementationrejections. Pi2a40e367retired/watcher32661collected0. Full rootdisposition r7/reviews/design-r5/ROOT-DISPOSITION.md. Guard staysDESIGN_REVIEW_REQUIRED; noR8/ordinaryrepair/S2a; all inherited/historyevidence retained. Sprintclosingat originaltimeboundary, no newpaid dispatch.


## 2026-11-14 — R6 design judgment and bounded readmission

Fresh native Astra xhigh/default, independently observed read-only/never grant, session01a1d811-93de-7ef0-9cc7-76d34f3a90cd turn01a1d811-9716-7a50-be3b-6dff2250a6cd accepted exact plan43d0e7c8 with zero P1/P2/P3. Root adopted via /home/bline/cbm-work/pi-accounting-continuation-20261114/ROOT-ADOPTION-R6.json. New isolated S1 specification/oracle realization only is admitted. Whole-base monotone atomization, separate supported-low physical independence and affirmative auxiliary links address R5 three design findings at declaration boundary. This does not close implementation findings or accept R7/S2a/live behavior. All old rejections/inherited evidence and infrastructure exclusions remain. Another serious regression or recurring invariant violation requires immediate design escalation again; no counter reset.
