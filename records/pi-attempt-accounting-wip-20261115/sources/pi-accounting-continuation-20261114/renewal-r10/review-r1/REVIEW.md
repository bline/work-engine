**CHANGES_REQUIRED — one blocking P2 remains.** The three original R3 counterexamples are addressed in the selected finite subjects, but the independent Rust consumer does not fully enforce the reservation claim’s monetary consequence.

This review binds subject `/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/subject`, manifest SHA256 `9234cec46402ebddcd6ae221c76867b038001b6022254e1ebcac3fd43d8425d6`, and adopted amendment `90b6308e9608decf026437c89b98de597cd30046f56d792aa7461061f31f4abb`. All 967 files matched before review and remained byte-exact afterward. Archive, structural executable, independent Rust executable and root-summary hashes matched the packet.

**P2 — Rust reports the reservation claim passed without checking its supplied monetary maximum.**

**Origin:** inherited R9 qualification omission, corroborated in this R10 candidate; not an introduced R10 repair regression.

The reservation branch checks raw reservations of 500M, registered `maximum_reservation_nano = 500M`, two classes, and ineligible replacement. It never compares the selected attempt’s supplied exposure amount with that independently established maximum. It then prints `outcome=pass`. See [independent_raw_axes.rs:1403](/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/subject/tooling/spec-check/tests/independent_raw_axes.rs:1403), especially the predicate at line 1437 and consumption at line 1451.

A precise counterexample changes the equal-reservations case’s `/expected/attempts/0/exposure/high/nano` from `500000000` to `400000000`, preserving both raw reservations, grants, payloads, membership/validity inventories and ineligible replacement. None of the Rust branch’s predicates observes this changed field. Its bundle references the external attempt; structural acceptance does not establish that referenced financial value.

- **Executed:** Python’s actual pilot entrypoint rejects the mutation with `exposure … 400000000 != Table X … 500000000`. There were no raw-applicability failures.
- **Source-derived, not freshly executed:** Rust’s registered finite qualification accepts this counterexample. The relevant branch and subsequent checks do not inspect the changed amount. Filesystem-copy mutation tests could not be rerun under the read-only grant.
- **Consequence:** independent qualification of the required 500M retained maximum remains unmet. **No combined Python-plus-Rust bypass is claimed.**

The amended reservation obligation requires actual monetary consequence separately from class cardinality. Checking the registered expected maximum is insufficient.

| Original R3 finding | Coverage and bounded disposition recommendation |
|---|---|
| **1. Exact grant/endpoint applicability** | Addressed for the selected seven bindings, two conditional bundles and two auxiliary controls. Raw digests, original coordinates/bytes, inherited contracts, grant fields and referenced payloads were inspected. Recommend bounded disposition of the demonstrated defects only. |
| **2. Reservation membership/validity inventory** | The original joint-deletion counterexample is rejected; lawful same-origin and admitted-copy positives preserve one class. Recommend disposition of that specific inventory omission. The broader reservation consequence remains incomplete under the P2 above. |
| **3. Inapplicable registration and false consumption** | Relation-specific selectors reject the P9 `control_file` substitution. Missing, duplicate, empty and missing-case controls reject; reporting follows predicate evaluation. Recommend bounded disposition of the demonstrated routing defect. |

| Discriminating claim | Judgment |
|---|---|
| **P9 retained charge** | Supported within this stage. Correct 700M retention/300M commitment passes. Coherent 500M output under unchanged lawful premises passes structural checking and fails financial qualification. Python rejection was freshly executed; Rust rejection is supported by exact source and retained root custody. |
| **Primary/anchor application** | Supported for selected VC11, gated VC11, historical VC07 and conditional linked/unlinked controls. Exact endpoint applicability and the conditional financial inventories are constrained. |
| **Actual response-model 7** | Supported for both selected controls. Corrected subjects match their raw inputs; original namespace, scope and copy/link policy persist. Both consumers reject completeness/release for numeric 7 while retaining supported usage and the affected 500M hold. |
| **Equal reservations** | Class inventory and no-release checks are supported. Independent Rust enforcement of the actual 500M monetary result remains incomplete. |

I challenged the admitted-premise record rather than treating agreement with it as raw truth. Independent recomputation matched every selected raw digest and original row coordinate/hash. Model7 provenance preserves the baseline contracts and grant payload meaning while projecting the actual subject digest. The oracle rollout records the initial derivation publication before model7 corrections, and the conditional/composed derivation addenda before their corresponding corrections.

The admission snapshot was populated partly by copying supplied proof fields. That construction alone establishes no independence; the bounded conclusion above rests on checking their correspondence with raw evidence and inherited premises. I found no inspected generator-derived financial expectation, new general reducer or arbitrary grant interpreter.

Fresh execution covered:

- Actual Python pilot positive and Rust registered-subject positive.
- Ten existing Python test methods through a read-only, in-memory substitution adapter: twenty negative subcases and eleven resolved structural positives.
- Coherent wrong-P9 structural acceptance through a memory descriptor, followed by financial rejection.
- Rust multiidentity/reservation helper controls.
- The additional equal-reservation monetary counterexample in Python.

The original filesystem-copy Python route and Rust mutation suites were **not freshly rerun**. Their retained custody was inspected; root’s nine-step replay remains evidence of its actual predicates. The disclosed stale conditional references, symbolic component pointer, ordering and gated-payload-shape failures remain preserved. Unknown last authorship of generated `corpus/SCHEMA.md` remains unresolved; its retained generated check does not establish authorship.

**Design escalation is required before ordinary repair.** This is a serious recurrence of incomplete independent financial qualification: a rejected-settlement reservation can be understated while the independent consumer reports success. The architecture guard explicitly retains inherited evidence. This establishes a qualification defect, not a newly executed production reducer regression or a demonstrated need to replace the financial model.

All three R3 findings and eleven historical findings remain **OPEN pending root disposition**. Full 175/374 qualification, A1/C2/A2b remainders, B2, broader schema/wire/generated/dependency obligations and exact-current A/B/C reviews remain outstanding. No S1, S2a, integration, publication or production acceptance follows.

Reviewer binding acknowledged: `01a1d8ee-19cd-7e12-ad6c-73356f2366a4`, verified **Astra/xhigh/default/read-only/never**; root `01a1d6e2-32ba-7701-9115-740dcb41af11` remains sole acceptance, persistence and native-final collection owner. Graph coverage reported external paths unavailable; exact-source fallback supplied the review evidence.

My own bound wait delivered a normal cursor but **no measured initial telemetry**. It was stopped and collected; the one-shot wait also terminated and was collected. Explicit pinned checks supplied fallback readings, latest **165,659 input tokens, epoch 0** before final preparation. No context replacement occurred. Actual cost remains unknown; root owns aggregate budget observation.

**All commands and subprocesses are collected. No owned process or child remains. No files or settings were changed.**
