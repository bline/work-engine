# Review-r2 remediation, oracle side: result

Branch `join-r2`, commit `042eddc` (author `pi-accounting-oracle-author`), on top of `53e7db7`. The spec author follows for `namespace.rs` (D-2 Rust input validation, D-4 `classify`).

## Fixes

**D-1 (P2): admission authority on the actual path.** `check_accounting_references` now calls `root_admission_application` for every bundle, before shape validation.
- Every `admission` initial contract, and any grouping it carries, needs an exact entry in the root admission record from `R11_ROOT_ADMISSIONS`. The match covers case, namespace, guarantee kind, `conditional_fixture`, duplicate rule, raw input digest and covered origins with their raw digests.
- `root_admissions_path()` ignores a path inside the subject.
- With no record bound, the result is `REQUIRED/UNMET` (a failure), never a pass.
- The R11 control route shares the matcher, and its old skip of grant-less admission contracts is removed.
- The grant-presence message now says what it checks ("without grant_ref/scope_id").
- `oracle/README.md` records the bound: Rust `evidence` has no admission input, so a Rust pass is not admission; the `dollar_ticket` D-SCOPE gap is a recorded full-S1 limit and is not wired.

**D-2 (P3): grouping, Python.** `grouping_application` rejects a base with more than one origin unless every member has a scoped contract (non-`unestablished`, with `grant_ref` and `scope_id`) or the members are joined by admitted `exact_links` or `identity_bindings`. Its message names BASE §3 OriginBase.

**D-3 (P3): null identity.** A present `null` `model`, `response_model` or `authorized_model` is malformed identity evidence. This holds in Python (`raw_usage_policy_checks`) and in independent Rust (`malformed_identity`; `Value::Null` was added to the injection control). An absent member keeps its existing V-1 handling, and a control asserts that.

**D-4 (P3): classifiers.**
- One rule table, `oracle/r11-controls/raw-namespace-rules.json`:
  - dollar_ticket: D1 `usd_reserved`+`rid`, D2 `usd.rid`;
  - admission: A1 `subscription_proxy_admitted`+`rid`, A2 `subscription_proxy.rid`;
  - both families give `conflict`; neither gives none, which includes rid-less admission rows.
- Python `raw_reference_namespace` and independent Rust `r11_raw_namespace` both implement it. A `conflict` row under a declared namespace is rejected.
- Agreement control `oracle/r11-controls/raw-namespace-census.json`: all 333 corpus raw rows (with digests) plus 6 synthetic rule-coverage rows (settlement-only `usd.rid`, rid-less admission, rid-less `subscription_proxy`, both families, null rid, non-object).
- Python (`test_r11_review` D4) and Rust (`r11_raw_namespace_census_agreement`) each reproduce the census exactly, and the Python test checks the census against the current corpus bytes. The spec author's `namespace.rs::classify` should be aligned and checked against the same census.
- The review's case (an `admission` contract over P9's `usd.rid` settlement row) is rejected ("raw namespace dollar_ticket").

**D-5 (P3): dangling contract ref.** `namespace_application` reports "dangling initial_contract_ref" instead of skipping it. The independent Rust namespace predicate also returns a violation instead of `None` on a dangling ref (the review observation).

**D-6.** Left as root decided (ROOT-DECISION-R3).

## Evidence

**Controls in `validator/test_r11_review.py` (10 tests):**
- D-1, reviewer p1 shape: VC08 raw, one grouped unit, subject-declared `admission` scope and grant. Fails with "REQUIRED/UNMET: no exact root admission (R8-NAMESPACE-01)" both with and without `R11_ROOT_ADMISSIONS`.
- D-1: a forged admissions file placed inside the subject is ignored.
- D-2, reviewer p15 shape (grouped, `unestablished`, no grant): fails with "… BASE 3 OriginBase per origin (review-r2 D-2)".
- D-3: null in each of the three identity fields fails R8-IDENTITY-01; absent does not.
- D-4: census agreement, and the P9 settlement-row admission contract.
- D-5: the reviewer's `nope` contract ref fails with "dangling initial_contract_ref 'nope'".

**Positives:**
- The corrected VC08 passes.
- The R11 control passes `admission_namespace_control` with root admissions bound.
- Across the 175 cases, the new predicates add no failure: `check_case` 0, and `check_accounting_references` has no new tagged failures. Its 476 failures are the unchanged full-S1 bundle-shape gap.

**Root gate script** (`R11_ROOT_ADMISSIONS=…/slice2/ROOT-ADMISSIONS.json replay_gates.sh join-r2 join-r2-target oracle-r2-evidence/gates-review-r2`): all 18 gates rc=0, including `coverage_namespace_controls.py`, `test_r11_evidence.py`, `test_r11_findings.py` and `test_r11_review.py`. Rust `r11_` tests pass 7/7.

**Without `R11_ROOT_ADMISSIONS`** (`oracle-r2-evidence/gates-review-r2-noenv/`):
- `test_r11_review` rc=0: the D-1 counterexample still fails as unmet when no admission source exists.
- `test_r11_evidence` rc=1, only because its R11 control route requires the outside-bound record. That is the intended "no admission source = unmet" behaviour.

## Limits

- `dollar_ticket` scopes are not bound to the root record on either consumer (root's D-SCOPE full-S1 limit).
- Rust `evidence` has no admission input (README).
- The independent Rust r11 predicates do not add D-2 grouping enforcement; that belongs to the spec's `namespace.rs`/input validation in step 2.
- The census is generated by the Python classifier, so agreement is checked consumer-to-census rather than against a fourth authored source.
