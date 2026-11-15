**INCOMPLETE — A1 bounded review; 3 P2 findings preserved. No acceptance.**

Subject: frozen R8 manifest `21df51b69391110beb6952c0eaebfb3c5bf99ac0ef19ce78a225fd24412afb36`. All 941 entries matched; archive, capsule bindings, dispatch projections and named policy/review hashes also matched.

The host required immediate context refresh. Safe recovery was unavailable under the read-only grant without an acknowledged session binding or writable continuation. This native report preserves the findings for root collection.

1. **A1-R8-01 — P2: Supplied coverage accepts witnesses that do not authorize the covered obligation.**

   Location: [accounting.rs:197](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/src/accounting.rs:197), `validate_result`; the exact bundle JSON paths below.

   Two counterexamples passed the pinned Rust binary:

   - Starting from `oracle/r8-controls/VC07-conditional.bundle.json`, change both `documents[0].exact_links[0].to_origin_ref` and `documents[1].definite_links[0].to_origin_ref` from `"u"` to `"a"`. The only link becomes **a→a**, yet coverage still retires the auxiliary anchor with request `u`. Removing `coverage_decisions[0].definite_link_refs` also passes.
   - Starting from `oracle/r8-completion-A/P132-FG-VC11-gated.bundle.json`, mark `documents[1].primary_validities[0].validity_result="rejected"` and redirect the first coverage’s `validity_ref` and `obligation_id` to the second obligation. Coverage of the rejected first obligation still passes.

   **Expected:** reject the supplied record’s inconsistent release authority. An anchor needs co-reference to its covered request; a validity witness must apply to the covered primary origins.

   **Actual:** both mutations exit **0**, reporting `structurally_valid`.

   **Owner/invariant:** F exact coverage, C-validity witness scope, `WHOLE-COVERAGE`. This is an internal reference/coherence defect; detecting it requires no arbitrary raw reducer. It is **not** a demonstrated whole-gate bypass.

   **Origin:** introduced in R8’s supplied-record checking; exact introduction commit unestablished.

2. **A1-R8-02 — P2: Inherited A-07 remains in the active independent arithmetic qualifier.**

   Location: [independent_raw_axes.rs:106](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/tests/independent_raw_axes.rs:106), `identities`, `price_range` at line 133, and `accepts_usage_axis` at line 895.

   Change VC01’s request `response_model` to numeric `7`. `identities` skips the malformed value, retaining the valid launch/request identity. The unchanged calculation yields `(15M,15M,true)` and accepts the eligible expectation.

   **Expected:** malformed identity prevents complete replacement; retain the 500M reservation and unresolvedness. **Actual:** the existing axis accepts the unchanged eligible 15M result.

   The new `F2-model7` literal fixture states the correct outcome, but does not repair or challenge this function. `oracle/README.md` still advertises provider-qualified identity qualification.

   **Owner/invariant:** V-1, P1/P2 identity establishment; independent financial qualification. **Origin:** inherited A-07. This conclusion is source-derived; I did not compile or execute a new Rust test.

3. **A1-R8-03 — P2: Inherited A-08 still equates amount uniqueness with reservation uniqueness.**

   Location: [independent_raw_axes.rs:269](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/tests/independent_raw_axes.rs:269), `frozen_raw_coverage_and_independent_arithmetic`, emitted `comparison_applicable: nums.len()==1`.

   Two reservation rows with the same 500M amount but different timestamps are distinct conflicting evidence. Nevertheless, the axis emits `comparison_applicable=true` because their distinct-amount list has length one.

   **Expected:** reservation-relative comparisons require exactly one distinct reservation. **Actual:** the claimed established axis permits comparisons for this conflicting pair.

   The new Python duplicate controls correctly count distinct raw lines, but leave this Rust assertion unchanged. No existing frozen-facts group exercises this equal-amount conflict, explaining why the historical pass does not challenge it.

   **Owner/invariant:** C-validity/`UniqueReservation`, original P3. **Origin:** inherited A-08.

**Prior dispositions within A1**

- **A-01:** R6 explicitly changes initial orphan grouping. H1/H2’s declared amounts follow that amendment; arbitrary executed conservation/confluence remains unestablished.
- **A-02:** protected local lows are restored: A06 15M, P133-A09 15M, P130-R05 20M, V01-undercharge 60M. This closes the cited value discrepancies within the inspected expectations.
- **A-07/A-08:** remain open as above.
- **A-03–A-06/A-09:** not cleared by this bounded report.
- **Low multiplicity:** inspected VC11 expectations correctly retain local 15M+15M while using aggregate low 15M without physical independence. L1/L2/L3’s declared floors are consistent with R6.

**Coverage and omissions**

Read required doctrine, manual-orchestration references, Wind Walker, R6 plan/adoption/policy, original P1–P6/D3, architecture review, previous implementation review and relevant guard history.

Inspected all **69 assigned cases’ financial inputs and current outputs**, using exact deduplication: 43 distinct raw lines, 27 financial output shapes, 15 valuation derivations and 150 monetary rollup scopes. Checked pinned rates, local/high tags and bases, settlement/replacement/obligation fields, and evaluation arithmetic conditional on attribution/gate premises owned by B.

Read accounting.rs, financial portions of decisions.rs, targeted independent_raw_axes.rs ranges, independent_r8_controls.rs, H/L/F/C fixture premises and supplied assertions, and relevant linked financial records. A linked-record census covered 69 bundles, 129 origins and 68 active units; all initial-scope grants were conditional fixtures, with no physical-independence frames.

Unfinished: complete per-case linked-premise verification, all negative fixture executions, full custody inspection, and final composed coverage assessment. A2’s other **106 cases were not reviewed**, apart from explicitly named core controls. Declaration conflicts around retained historical transition/no-double-count wording were noticed but not fully adjudicated.

**Execution and limitations**

- Verified binary SHA `66ec52f8d51bf591a29b529948fd5e9957929234701ba9867216c1c845aeefd2` and validator SHA `ab5f78d47d6105791a8ca14cf0c0799015aca67dd7935a48d18756735417429a`.
- Executed five stdin probes using `<binary> evidence <frozen-subject> /dev/stdin`: two baselines and three mutations; all exited 0.
- Created no files; made no edits, commits, settings changes, delegations or provider launches. No owned process remains outstanding.
- Read ROOT-JOIN-VERIFICATION; did not independently complete custody verification or rerun sabotage.
- External-source graph coverage was `outside_project/unavailable`; conclusions use exact source, without graph-completeness claims.

The context interruption itself is **not an implementation rejection**. The substantive findings remain evidence for root’s architecture-guard disposition before repairs. D3 remains binding; full raw conformance, executable confluence, live provenance and S2a readiness remain unestablished.

**Binding:** collection owner acknowledged as `01a1d6e2-32ba-7701-9115-740dcb41af11`. No verified reviewer SID/rollout binding was received or inferred. Applied model/effort/tier and invoice/quota observations are unavailable. Root retains sealing, acceptance, escalation and future development.
