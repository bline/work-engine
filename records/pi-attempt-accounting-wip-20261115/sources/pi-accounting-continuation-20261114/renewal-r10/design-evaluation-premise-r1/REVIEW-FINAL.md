**REVISE_LOCAL_PLAN — one blocking P2.** The finite-admission strategy remains credible. I do not recommend reopening the financial model or introducing a general reducer, but the exact candidate needs one concrete addition before implementation admission.

This review binds:

- PLAN.md `a85dffca9027bd386962b01ee6bf624dafbcd565f7f620e4a6210296be969b92`
- PACKET.md `580427e7b48273add1bf8a631792d6f921fd8ac3b736a529021a19182a472fdb`
- Frozen subject manifest `d60017dac28aa3a831cf13db520af94d8224b53f70b12229240bff8dab30ec61`

The plan and packet remained unchanged. All **967 subject files** matched the frozen manifest, with no missing or extra files.

**EPR-01 — blocking P2: the concrete premise projection omits the separate test-verification input channel.**

[PLAN.md:13](/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/design-evaluation-premise-r1/PLAN.md:13) identifies complete `inputs`, evaluation objects minus `expected`, and the conditional raw/payload records. However, the selected gated VC11 case also has a governing **top-level** field:

```text
/trusted_fixture_inputs/gate_verifications/g-S1
```

That field is outside both `inputs` and `evaluations`. It contains the test-only verification source, authority/session binding, payload digest, policy reference, and verification evidence. See [the actual case:1205](/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/low-target-repair-r1/subject/corpus/cases/P132-FG-VC11-gated.json:1205).

This is a distinct trust boundary, not descriptive metadata. [CONTRACT.md:247](/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/low-target-repair-r1/subject/contract/CONTRACT.md:247) requires separately trusted gate verification or this exact test-harness input. The Python gate predicate reads it at [validate_corpus.py:193](/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/low-target-repair-r1/subject/validator/validate_corpus.py:193); Rust’s independent mock-binding predicate reads it at [independent_raw_axes.rs:605](/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/low-target-repair-r1/subject/tooling/spec-check/tests/independent_raw_axes.rs:605). The nominal gated evaluation reports a qualified gate, retired commitment of zero, and admitted request.

A fresh **read-only, in-memory comparison** established that deleting `trusted_fixture_inputs` leaves the named `inputs` plus evaluation-premise projection unchanged, while leaving every expected output unchanged. Thus that projection alone cannot establish the complete governing premise boundary.

Line 15 correctly requires discovery of additional causal premises. This finding identifies a concrete outstanding member of that obligation; it does not show that the finite strategy is unsound. Before adopting this exact plan, make the separate test-verification channel explicit in the frontier and qualification requirements.

**Origin and limits:** this is an unresolved candidate-design omission around an existing authority-input boundary. No introduced financial regression is established. Python already has separate gate checks; I am **not claiming that either complete current consumer accepts this deletion**, and I did not execute that mutant through them. The finding concerns the promised independent premise boundary and its required failure attribution.

**Recommended bounded revision**

Include the complete, presence-sensitive top-level trusted fixture input in each applicable case admission, with explicit absence for cases that have none. Preserve its test-only provenance and exact payload binding; a matching snapshot must not manufacture live verification authority.

The source-derived frontier should also classify case-level mode selectors that affect qualification—particularly `profile`—rather than assuming every governing input lies beneath `inputs` or `evaluations`. Each included or excluded channel needs a consequence-based justification. This can remain a compact finite interface.

Qualification should demonstrate both consumers rejecting deletion, null substitution, rebinding, and other material changes to the verification input under unchanged trusted admissions, for the premise reason. Existing gate-output and financial-output mutants must continue exercising their respective predicates under unchanged admitted premises.

**The remaining design is supported within this bounded scope.**

- Complete ordered evaluation admission addresses the demonstrated window/journal gap without evaluating arbitrary changed premises.
- Independent raw-policy derivation before literal admission preserves the distinction between nominal truth and snapshot applicability.
- Exact payloads, conditional assumptions, original inputs, and actual rate-document identities can form a finite boundary without an arbitrary grant interpreter.
- Separate Python and Rust enforcement is realizable. Missing/null/type distinctions, duplicate-member detection, inventory checks, and controlled malformed-input failures remain implementation acceptance obligations—not properties established by this review.
- The preservation envelope correctly retains the 123 selected-witness controls, prior financial negatives, lawful equivalence positives, all eleven admissions, and actual predicate-consumption checks.

I verified the retained root probe script, custody identity, and stdout digest. They record **six actual Rust/Python false accepts** for the `06:55` mutation. Those are executed root observations, not merely Rust source deductions. They establish omitted premise admission, not monetary loss under identical complete premises.

**Instruction review: applicable.** The exact plan and packet contain current authority, scope, sequencing, and acceptance instructions. Their causal reasons are exposed at the loading boundary: establish applicability before reporting qualification, preserve independent expectation authorship, and retain root authority. Advisory outcome: **restate** the concrete frontier as above; retain the other inspected instructions. No separate material instruction finding is added.

The complete architecture-guard history remains applicable. This is a design-candidate revision recommendation, not another implementation-rejection count or a history reset. All **three R3 and eleven historical findings remain OPEN**; full **175-case/374-scope** qualification, remaining A1/C2/A2b and B2 work, schema/wire/generated/dependency obligations, and exact-current complete A/B/C reviews remain owed.

Evidence used exact-source fallback: private-subject graph coverage was outside-project/unavailable. No new implementation, builds, consumer mutation suites, or paid API calls were performed. No files or settings changed. All commands and subprocesses completed and were collected; no child or live listener remains.

Latest explicit context observation: **162,848 tokens, epoch 0**; finalization fits below preparation threshold. Continuous context delivery remains unavailable because of the disclosed reader-option incompatibility. Cost remains root-accounted within the existing **6/100** allowance. Root retains adoption, persistence, and whole-native-final collection; this review grants no implementation or acceptance authority.