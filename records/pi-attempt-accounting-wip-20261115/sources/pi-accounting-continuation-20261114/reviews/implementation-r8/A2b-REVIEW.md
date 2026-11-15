**INCOMPLETE — A2b bounded review. One P2 finding preserved; no acceptance recommendation.**

Subject: revision8, contract `1.7-s1`, manifest `21df51b69391110beb6952c0eaebfb3c5bf99ac0ef19ce78a225fd24412afb36`. Assigned projection: `dispatch/A2b-cases.json`, SHA256 `ea466afcf9f840ed88041eee4a8bc90bc33c62ca220e384ce2a4644c373f1f81`.

**A2b-R8-01 — P2: Subscription groups consume a dollar-ticket provenance declaration.**

- **Location:** [VC08 provenance]( /home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/oracle/r8-completion-B/VC08-subscription-measured-request.provenance.json), `$.initial_scope_contracts[0].reference_namespace` and `$.grant_payloads[0].guarantee`; corresponding bundle `$.documents[0].initial_contracts` and `$.documents[1].origins`.
- **Raw fact:** VC08 contains `subscription_proxy_admitted.rid=1` and a request carrying `subscription_proxy.rid=1`. Neither is a dollar-ticket reference.
- **Actual:** the supplied contract declares `reference_namespace: "dollar_ticket"`. Its payload promises only interpretation of a “usable dollar rid.” The result nevertheless groups both origins into `base:adm:S1/turn-1:rid=1`. The same namespace mismatch appears in the inspected U06-unlabelled, V03-labelled-control and V03-labelled-api bundles.
- **Expected:** admission references require their separately declared namespace and applicable conditional guarantee. A dollar-ticket grant cannot establish that guarantee.
- **Owner/invariant:** C initial-reference admission; PLAN §3’s separate namespaces and the prohibition on substituting provenance guarantees.
- **Consequence:** the supplied grouping provenance is unsupported by its stated payload. I have **not demonstrated a changed monetary amount or executed a consumer bypass**.
- **Origin:** present in R8 completion-B records; introduction history unestablished. Integrate with C1/C2 before acceptance.

**Financial coverage completed**

I read all **53 assigned cases’ raw inputs and current financial expectations**, including evaluation money, using exact deduplication: 54 distinct raw log lines, 11 session-header shapes, 35 attempt-output shapes and 13 valuation shapes. I inspected linked financial premises for **98 origins, 54 active units and 112 low scopes**.

No additional incorrect monetary expectation was established. In particular:

- V01-undercharge retains **low60M / AtLeast800M**.
- Fully priced primary multiidentity retains **low20M / Exact80M**.
- V02’s unsplit writes support **10M–16M**; undercharge retains **AtLeast300M**.
- R6-C02 retains **low1,000M / AtLeast1,000M** despite error-status closure.
- VC07 retains its locally Exact15M request and separate AtLeast0 anchor.
- VC11 retains two local15M observations, high30M and aggregate owned low15M without physical-independence proof.
- VC12 retains both same-`n` cross-context candidate pairs.
- Genuine unbudgeted completion, missing-launch evidence, malformed status/amount/n, source order, unreadable framing and subscription applicability remain distinct.

Evaluation arithmetic was assessed against supplied ownership and lifecycle premises; **B1/B2 still own their complete attribution/admission validity**.

**Prior finding dispositions**

| Prior finding | Bounded disposition |
|---|---|
| A-02 | V01-undercharge’s cited local-low discrepancy is repaired in the inspected expectation and owned rollup. Other protected cases belong to A1/A2a. |
| A-04 | U08 now carries `TierUnestablished(Missing)`; broader wire/schema clearance remains outstanding. |
| A-01/A-03 | Relevant finite expectations inspected; general conservation, confluence and total applicability not established. |
| A-05/A-06/A-09 | Not cleared by this review. |
| A-07/A-08 | Current reports identify recurrence; I did not independently finish their source/probe obligations and do not close them. |

Cross-module relations remain necessary for controls outside this assignment, notably `C1-control-before-reference` and `A1-03-unreferenced-log`, and for other modules’ cases referencing VC01–VC08, VC11–VC15, S02, U04, U08 and V01 controls. The other **122 financial cases were not reviewed**.

**Evidence and execution**

- Verified all **941 frozen subject entries**, **1,285 capsule-bound files**, six dispatch projections, archive hash and named policy/review hashes.
- Verified equality of every retained projection field against all 53 original case files.
- Read R6 policy/adoption/plan, original P1–P6/D3, architecture acceptance, previous implementation review, relevant guard history and current A1/A2a/C2 reports.
- Read applicable canonical C/V/F/X-component/supported-low declarations, R contract sections, money rules, pinned rate files and targeted actual Python consumer code.
- Inspected ROOT-JOIN-VERIFICATION and all six joined custody records; verified their record hashes and the supplied binary/validator hashes. Read their reported outputs. **Complete custody input/output-digest reconciliation remains unfinished.**
- Commands were read-only `python3` inspections/hash comparisons, `cat`/`sed`/`rg`, graph status/coverage and the pinned `child_context.py check`. **No suite, sabotage, compiled test or financial mutant was executed.**

External evidence paths returned `outside_project/unavailable`; conclusions use exact source reads, without graph-completeness claims. Postgate metadata drift remains disclosed.

**Unfinished obligations**

Complete applicable X branch coverage, shared finite A/W/O/P/M/G controls and their negative-test meaning, independent low-assertion premise checks, and deliberate consistent-wrong-output probes. A useful unexecuted probe is whether V01’s multiidentity high/claimed charge can be understated together with matching derivations and rollups while preserving its independently asserted low. This is a hypothesis, **not a finding**.

Exhaustive reference closure, historical reversals and authoring provenance remain C2 obligations. Full arbitrary raw conformance/confluence, live authority, B predecessor accounting continuity and S2a readiness remain unestablished. D3 does not authorize a reducer as a routine repair.

**Identity, process and interruption**

Acknowledged root’s binding: reviewer `01a1d88a-9e2a-7801-aed3-64705ff5585d`, rollout `/home/bline/.codex/sessions/2026/11/13/rollout-2026-11-13T22-32-05-01a1d88a-9e2a-7801-aed3-64705ff5585d.jsonl`; collector `01a1d6e2-32ba-7701-9115-740dcb41af11`. Root’s binding records Astra/xhigh/default and read-only/never; the context check independently reported Astra/xhigh. Backend fulfillment, invoice/quota and actual spend are unknown.

No files, commits, settings, delegates or provider launches were created; no process remains outstanding. The host demanded immediate refresh, but the read-only grant prevents publishing the required continuation/chatboard pointer. I therefore preserve this native partial for root sealing rather than reset unsafely. **The context interruption is not an implementation rejection.** Root retains collection, acceptance and architectural escalation authority.
