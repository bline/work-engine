**INCOMPLETE — C2 review. Established findings require changes; no acceptance recommendation.**

Subject: `/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject`, manifest SHA256 `21df51b69391110beb6952c0eaebfb3c5bf99ac0ef19ce78a225fd24412afb36`. Root `01a1d6e2-32ba-7701-9115-740dcb41af11` retains collection, acceptance, escalation and repair authority.

### Findings

**C2-R8-01 — P1: Actual Python accepts loss of a retained raw charge.**

- Location: [validate_corpus.py:451](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/validator/validate_corpus.py:451), `table_x`; raw checking at line 547. Case: `corpus/cases/P9-claimed-charge-above-reservation.json`.
- Raw fact: reservation `reserved_usd=0.4`; settlement claims `reserved_usd=0.5`, `charged_usd=0.7`; supported usage is 15M nano.
- Executed counterexample: leave all raw input, independent low assertions, references and supplied accounting bundle unchanged. Change:
  - `expected.attempts[0].settlement.charged`: `700000000 → 500000000`
  - `expected.attempts[0].exposure.high.nano`: same change, retaining `proxy_charge`
  - `evaluations[0].expected.objectives[0].owned_outstanding.high.nano`: same change
  - `...commitments[0].committed`: `300000000 → 500000000`
- **Actual:** actual Python `check_case` plus `check_accounting_references` returns **zero failures**.
- **Expected:** reject the missing 200M retained charge. Table X reads the supplied settlement charge without establishing its agreement with the raw settlement. Matching rollups and residual commitments therefore conceal the error.
- Owner/invariant: raw charge retention through X/R and the claimed actual-Python high qualification.
- Origin: **unknown introduction**; the defect is present in revision8. This is a bounded counterexample on an existing case, not a demand for arbitrary raw reduction.
- Root-owned full CLI/Rust-corpus probe was requested; no result received before this report.

**C2-R8-02 — P2: The malformed response-model false positive from A-07 remains.**

- Locations: [independent_raw_axes.rs:106](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/tests/independent_raw_axes.rs:106), `identities`; `price_range` at line 133; `accepts_usage_axis` at line 895; Python [validate_corpus.py:547](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/validator/validate_corpus.py:547).
- Executed counterexample: load VC01, change only the request’s raw `response_model` to numeric `7`, retain its complete 15M valuation and eligible replacement, and invoke actual Python `check_attempt`.
- **Actual:** zero failures. Rust source still ignores non-string response identities and retains the valid launch/request identity; I did not execute that Rust function directly.
- **Expected:** malformed identity evidence cannot establish the complete identity set needed for replacement.
- `F2-model7` is a separate literal assertion over a simplified record whose **request `model`** is numeric. It does not challenge this actual response-model path or retire the defective arithmetic helper.
- Owner/invariant: P1 identity establishment, A-07 and independent raw-axis qualification.
- Origin: **inherited recurrence of A-07**.
- A root-owned Rust probe was requested; its result remains outstanding.

**C2-R8-03 — P2: F can consume another unit’s C-validity evidence.**

- Locations: [accounting.rs:197](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/src/accounting.rs:197), `validate_result`; [corpus.rs:94](/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/subject/tooling/spec-check/src/corpus.rs:94); Python `check_accounting_references`.
- Executed counterexample: in the supplied VC11 bundle, exchange the two primary-validity records’ `distinct_reservation_refs` and `distinct_settlement_refs` between turn1 and turn2. Keep obligation IDs, coverage references, units, raw input and money unchanged.
- **Actual:** actual Python reference checking returns zero failures. The bound Rust binary’s `evidence` command accepts the mutated bundle via `/dev/stdin`.
- **Expected:** reject a coverage witness whose primary evidence belongs to the other covered obligation. Both witnesses still have cardinality one and valid references, but their ownership is wrong.
- Owner/invariant: affirmative C→F witness application to the exact covered origins. This is supplied-record reference coherence, within D3.
- Origin: **unknown introduction**; demonstrated in revision8.

### Prior finding dispositions

- **A-02:** the four protected local lows are present, and actual Python’s forced-zero controls challenge them. This bounded repair is supported.
- **A-07:** remains open through C2-R8-02.
- **A-08:** Python now counts exact raw reservation bytes. However, Rust’s retained report still sets `comparison_applicable` using `nums.len()==1`. I found no matching conflicting-one-amount group in the frozen fact census; I have **not closed A-08 or promoted this observation to another finding**.
- **A-01/A-03:** finite fixtures do not establish general confluence or total raw applicability; those broader claims remain expressly unestablished.
- **A-04/A-05/A-06/A-09:** no independent closure from this incomplete C2 review; C1 owns their complete declaration/interface assessment.

### Coverage and execution

Verified:

- All **941 manifest entries**, **1,285 capsule-bound files**, and **six dispatch projections** matched their recorded digests.
- Structural census: **175 cases, 186 active units, 322 inventoried financial origins, 374 low scopes**; **83 budgeted Exact** and **16 unbudgeted Exact** units.
- Active-unit/result/exposure references, raw-origin hashes, low-scope references and shard claim counts across that census.
- Direct execution against the frozen subject:
  - `python3 -B validator/validate_corpus.py check corpus oracle/rates` — exit0, 175/374.
  - Bound `pi-accounting-spec-check corpus <subject>` — exit0, 175/374.
- Joined custody record and stdout/stderr hashes matched. Join1 preserves its revision failure; join2 records 6/6 passing.
- Bound binary SHA256 matched `66ec52f8d51bf591a29b529948fd5e9957929234701ba9867216c1c845aeefd2`.
- Compared recorded joined inputs with frozen equivalents: drift was confined to integration `SOURCE-BINDINGS.json`, `oracle/R8-COMPLETION.json`, and `oracle/README.md`. One external control-binding path was not independently resolved.

Read the principal Python consumer paths, complete independent Rust arithmetic axes, Rust corpus/accounting consumers and main entrypoint, relevant structural controls, A/B authoring/materialization/negative-control helpers, finite inventories and joined evidence. The authoring helpers use bounded literal inventories; I established no hidden general reducer or import of generator financial decisions into their expected arithmetic.

**Omissions:** complete external-reference/payload-fragment closure; full sabotage mutation/expected-reason audit; independent replay of all 342 B reversal entries and all historical reversals; complete authoring-history attestation; remaining library/dependency sections; requested root probes. I did not rerun long sabotage, compile Rust, or independently audit every case’s financial and attribution truth assigned to A/B.

### Process and identity

The mandatory context-refresh boundary interrupted the review. Under this read-only grant I could not publish the required continuation/chatboard pointer, so I stopped instead of resetting with unresolved meaning.

No files changed, commits, delegation, provider launches or settings changes. All launched read-only processes terminated. Graph coverage reported the external subject `outside_project/unavailable`; no graph-completeness claim.

**No verified reviewer SID/rollout/context-status binding was received or acknowledged.** Runtime model/tier fulfillment, invoice and proxy spend remain unknown.

The retained-charge defect and A-07 recurrence warrant root’s architectural-guard disposition before ordinary repairs. Full arbitrary raw conformance, executable confluence, live authority and S2a readiness remain unestablished.
