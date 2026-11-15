**Design verdict: REVISE_PLAN — a local qualification amendment.**  
**Implementation verdict: CHANGES_REQUIRED for the four-claim stage.**

Root’s proposed model7 correction is necessary but insufficient. Exact applicability gaps also affect other selected fixtures; reservation-class outputs are incompletely qualified; and Python can report P9 consumed without executing its financial predicate. I found no contradiction requiring a financial architecture reopen.

This report binds frozen subject `/home/bline/cbm-work/pi-accounting-continuation-20261114/pilot-r9/subject-r2`, manifest SHA256 `0cad3d5ed66b9ef3c4796cb4039a16646d0ce0d451153948b08f972d8286df6a`. I verified all 959 manifest entries, all 23 source bindings, both supplied executable hashes, and the supplied root-summary hash.

**Blocking findings**

1. **P2 — Exact grant and endpoint applicability remains incomplete beyond model7.**  
   **Origin:** inherited/corroborating applicability failure, with incomplete R9 finite enforcement; not another historical rejection round.

   Both known model7 grants retain baseline subjects:

   - VC01: baseline `08500d9d…`, actual control `8e9cedfb…`.
   - A04: baseline `b51f04a1…`, actual control `976a4b2a…`.

   Their referenced provenance payloads also retain those baseline hashes. The frozen grant locations are [VC01 bundle:22](/home/bline/cbm-work/pi-accounting-continuation-20261114/pilot-r9/subject-r2/oracle/r9-pilot/cases/model7-VC01-openai-usage-settlement.bundle.json:22) and [A04 bundle:22](/home/bline/cbm-work/pi-accounting-continuation-20261114/pilot-r9/subject-r2/oracle/r9-pilot/cases/model7-A04-undated-settlement-between-dated-neighbours.bundle.json:22). **Lawful composed model7 qualification remains UNMET.**

   Additional decisive probes preserved raw fixtures and financial outputs while changing supplied evidence:

   | In-memory substitution | Python actual `main(check … --pilot)` | Exact Rust `evidence` executable |
   |---|---:|---:|
   | Equal-reservations `covered_extent_ref` → zero digest | Accepted, 0 failures | Accepted structurally |
   | Equal-reservations initial-contract `scope_id` → `different-scope` | Accepted, 0 failures | Accepted structurally |
   | VC11 grant payload reference → P9’s existing provenance payload | Accepted, 0 failures | Accepted structurally |
   | Conditional VC07 anchor’s `raw_bytes_sha256` → zero digest | Accepted, 0 failures | Accepted structurally |

   Python’s equal-reservation check compares selected extent fields but omits the grant’s `covered_extent_ref` and contract-scope agreement. Conditional VC07 checks the pinned raw file and admitted pair without binding the supplied endpoint origin records back to that raw subject: [r9_pilot.py:93](/home/bline/cbm-work/pi-accounting-continuation-20261114/pilot-r9/subject-r2/validator/r9_pilot.py:93), lines 167–179.

   The corresponding Rust finite branches have the same material omissions: [independent_raw_axes.rs:1367](/home/bline/cbm-work/pi-accounting-continuation-20261114/pilot-r9/subject-r2/tooling/spec-check/tests/independent_raw_axes.rs:1367), lines 1417–1439. **Rust finite-mutant acceptance is source-derived, not freshly executed.**

   The structural executable explicitly leaves external evidence truth unestablished; its acceptance is consistent with that boundary. The defect is promoting the composed finite qualification without the missing raw/payload application checks.

2. **P2 — The equal-reservation claim does not constrain the supplied reservation-class inventory.**  
   **Origin:** introduced R9 qualification omission.

   Starting from `oracle/r9-pilot/cases/equal-reservations.bundle.json`, remove the second reservation class from `primary_memberships[0].reservation_classes` and its matching entry from `primary_validities[0].distinct_reservation_refs`. Preserve both raw reservation rows, origin ownership, rejected settlement, all amounts and all grants.

   **Executed result:** Python’s actual pilot entrypoint still returns **0 failures**, and the exact Rust `evidence` executable accepts the bundle structurally. The result now claims only one reservation class despite the independently registered two-origin premise.

   [r9_pilot.py:88](/home/bline/cbm-work/pi-accounting-continuation-20261114/pilot-r9/subject-r2/validator/r9_pilot.py:88) checks raw byte distinction and no-release behavior, but never compares the control’s supplied membership with that inventory. Structural membership/witness agreement checks only the jointly edited sets. Rust’s finite branch likewise reads the bundle’s grant fields but not its membership or validity inventory: [independent_raw_axes.rs:1353](/home/bline/cbm-work/pi-accounting-continuation-20261114/pilot-r9/subject-r2/tooling/spec-check/tests/independent_raw_axes.rs:1353).

   The existing positive correctly has two classes and maximum 500M. That positive does not establish rejection of a consistently wrong class result.

3. **P2 — An inapplicable registration route bypasses P9 while reporting it consumed.**  
   **Origin:** introduced R9 dispatch defect.

   I first executed the coherent P9 mutation: charge/high/component **700M→500M**, rollup **700M→500M**, commitment **300M→500M**, with raw and registered premises unchanged. Python correctly rejected it for the raw-charge, rollup and component predicates.

   Then I changed only P9’s registration by adding:

   ```json
   {
     "control_file": "../../corpus/cases/P9-claimed-charge-above-reservation.json",
     "bundle_file": "oracle/r8-completion-B/P9-claimed-charge-above-reservation.bundle.json"
   }
   ```

   All four claim IDs, their relations, case counts and raw premises remained present.

   **Executed result:** the same wrong financial outcome passed Python’s actual pilot entrypoint, which printed **“consumed4 claims … 0 failures.”**

   The generic schema admits these fields. Dispatch selects `control_claim`, which has no retained-charge branch; `case_claim` skips registrations containing `control_file`; reference checking skips `case_claim` when given a pilot binding. Consumption is nevertheless appended unconditionally. See [r9_pilot.py:37](/home/bline/cbm-work/pi-accounting-continuation-20261114/pilot-r9/subject-r2/validator/r9_pilot.py:37), lines 82–94, and [validate_corpus.py:1403](/home/bline/cbm-work/pi-accounting-continuation-20261114/pilot-r9/subject-r2/validator/validate_corpus.py:1403).

   This violates the required **inapplicable/unconsumed fail-closed** condition. Rust dispatches retained-charge checks by claim ID and would still reject this wrong P9 output; **I do not claim a composed Python-plus-Rust financial bypass.**

**Standing of the four claims**

| Claim | Bounded implementation judgment |
|---|---|
| **P9 retained charge** | Supported for the exact registered subject. Python rejects the coherent 500M outcome semantically. Rust independently checks raw 400/500/700M, usage 15M, retained components, attempt, rollup and commitment. Registration robustness fails finding 3. |
| **Primary/anchor application** | VC11 and gated VC11 have independently constrained exhaustive primary inventories. Supplied witness, obligation, valuation and exact-link relations are materially stronger; historical VC07 retains its separate AtLeast0 anchor. Full exact applicability remains incomplete under finding 1. |
| **Actual response_model7** | Both actual numeric-7 controls block completeness/eligibility while preserving supported 15M and the 500M hold; A04 retains its separate 300M outstanding reservation. The Rust priced multiidentity 20–80M control passes. **Neither model7 case is a lawful composed positive because its grant subject is stale.** |
| **Equal reservations** | The supplied positive correctly distinguishes the two timestamp/byte-distinct 500M reservations and retains maximum 500M. Actual class-output enforcement fails finding 2. Same-origin/admitted-copy controls support bounded structural/helper behavior, not complete raw provenance qualification. |

Missing, duplicate and removed case registrations have explicit rejection checks. **The broader mandatory-registration claim is not established**, because an inapplicable route can silently leave P9 unconsumed.

**Required plan revision**

Retain the existing financial semantics and spec/oracle ownership split. Before further ordinary repair, revise the bounded qualification obligations to require:

- Exact applicability across the **selected seven cases and two conditional bundles**, including referenced payload selection, raw subject, origin mapping, scope/namespace and admitted copy/link meaning. Fixing only the two model7 payloads leaves demonstrated holes.
- Independent comparison of the equal-reservation control’s actual membership and validity inventories with its two raw evidence classes, separately from maximum arithmetic and release eligibility.
- Relation-specific registration routes and selectors, with consumption recorded only after the required predicate executes. The P9 route substitution above must fail.
- Correct and wrong financial outputs under identical lawful premises; provenance substitutions must fail for applicability reasons. Preserve the distinction between structural validity, raw facts and financial correctness.

These requirements can use finite typed checks. The inspected implementation does not demonstrate a need for a general grant interpreter, expression language, partition solver or financial reducer. I found no generator import supplying the inspected independent financial expectations.

**Execution, limits and custody**

I freshly executed the actual Python pilot positive, Rust registered-subject qualification, Rust multiidentity/reservation helper controls, and all nine selected bundles through the exact structural executable. Those passed their stated predicates.

Adversarial Python executions used in-memory read substitutions through the existing `main` entrypoint. Rust structural probes consumed anonymous memory descriptors. **No frozen files changed.** Rust finite-mutant conclusions above come from exact source inspection and the retained worker/root test evidence; I did not rerun tests that create temporary subject trees. My model7 mutations established malformed-identity rejection, not a newly demonstrated lawful, fully consistent composed negative.

Review covered the requested amendment/adoption/R5 material, derivations, registry/schema, selected raw rows and outputs, referenced payloads, and material Python/Rust implementation ranges. Graph coverage reported the frozen paths `outside_project/unavailable`; exact-source fallback supplied the evidence, without graph completeness claims.

I did not audit all 175 cases, complete A1/C2/A2b remainders or B2, or clear broader namespace, integer-domain, constructor, dependency, S1 or S2a obligations. **All eleven historical findings remain OPEN.** This report grants no implementation acceptance, closure, integration or publication authority.

Verified reviewer: `01a1d8c2-754e-7c13-84ec-570c1b872e56`, recorded **gpt-6-astra / xhigh / default**. Root remains sole design/acceptance owner and native-final collector. The context listener terminated with `unknown control option --step-percent`; explicit checks supplied fallback observations. Latest reading: **179,235 input tokens, epoch 0**, before this final, below preparation 185,000. No context replacement occurred. Actual cost/backend fulfillment remains unknown.

**All commands and subprocesses were collected. No owned process or child remains.**
