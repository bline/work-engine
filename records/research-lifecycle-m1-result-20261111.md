# Research lifecycle M1 — local implementation result

Date: 2026-11-11. Status: **M1 accepted and integrated locally**.
The user subsequently authorized committing the task-owned planning, records and
implementation. Integration evidence below describes the state before that commit.
The user approved the [M0 contract](../planning/research-lifecycle-m0-contract-and-first-use-20261111.md), then said “proceed” at 2026-11-12T00:44:15.424Z. The [M0 approval](research-lifecycle-m0-approval-20261111.md) and [M1 packet](../planning/research-lifecycle-m1-implementation-packet-20261111.md) bound this implementation. The supervisor owns this bounded acceptance.

## Result

The additive `research-register::use_bundle` module provides versioned research subject, evidence, assessment, qualification, audit request and applicability records. Its private `ValidUseBundle` requires the existing `ValidRegister` proof and checks the pinned source/register/study/target/request relationships. It retains exact JSON and verified Git bytes for reconstruction. Unknown and not-assessed uncertainty remain distinct; conflicting assessment revisions remain representable.

The upstream manifest can validate before projection or execution. Passive prepared-basis, adoption and feedback records refer backward and preserve source execution identity. Their authority and outcome labels remain assertions. Structural validity establishes no scientific qualification, authenticated applicability, delivery, admission or execution authority.

Implementation and wire details: [module](../rust/crates/research-register/src/use_bundle.rs), [tests](../rust/crates/research-register/tests/use_bundle.rs), [schema documentation](../rust/crates/research-register/docs/use-bundle-v1.md), plus the additive export in `src/lib.rs`. No dependencies or legacy register semantics changed.

## Exact accepted subject

| Binding | Value |
| --- | --- |
| Shared baseline and unchanged HEAD | `1eaa698f6ade66097b4e7d8a8556983e056d570d` |
| Accepted private checkpoint | `168d82e141bdd0e5d7b00b8c3a20a9c07f6febe2` |
| Tree | `96634b55929f1779345d4732dd51fd065d8b5f1f` |
| Parent candidate | `c908c39b44c6a6f62b0306bcae482f3c0f7e8662` |
| Task patch SHA-256 | `fee0bcc0a922253803502df62242fafbe389e80602474ea62e663ee5a7e13dbc` |
| Gate summary SHA-256 | `6a7b9310bc514f678bd98cf52ab7bfa0a67f5a8141591b7dafa8856b64a9a405` |
| Plan / scope | `M1-packet-v1` / `M1-records-and-validation` |

The private ref is `refs/work-engine/checkpoints/research-lifecycle-m1-20261111/slice-1/candidate-2`. Only the four assigned crate paths were transferred. Their hashes matched the checkpoint afterward. Shared HEAD and index stayed unchanged; other sessions’ work and earlier planning drafts were preserved. Nothing was pushed or deployed.

## Qualification evidence

Full crate tests: **85 passed**, one optional extracted-register environment helper ignored. Existing history tests ran with full repository history. Formatting and clippy passed with locked/offline dependency use where applicable. All final gates bind 20 input files, including the owned files and relevant Cargo files. Integration reran the full crate suite in the shared checkout and verified all recorded input hashes. Integration summary SHA-256: `aa2adcb028ed74ebb6874eb3491663771bfb85c065e8f9000c3a0797308f7cb1`. No recorded test processes remain.

Separate Astra review **M1-ADV-01** initially found one P2: several negative tests could pass because another invalid field masked the intended check. The retained builder isolated those violations, added valid controls and exact diagnostic assertions, and demonstrated four targeted disabled-check mutations. Production source was restored before passing final gates. The retained review resolved the P2 and found no new actionable P1/P2. Earlier failed test, clippy and mutation-runner receipts remain preserved. The initial empty-input custody limitation was repaired by the final source-bound receipts.

Builder: `01a1cd3a-487c-7230-bc4d-8fea010d08a1`, requested and observed Sol 6.1 / medium / default. Reviewer: `01a1cd47-20a2-77a3-aa1d-465b74452295`, requested and observed Astra / xhigh / default; host turn records show read-only permissions. Explicit identity acknowledgement was collected after the initial report. This is recorded configuration, not a backend fulfillment claim. The reviewer inspected exact immutable source and custody rather than rerunning tests. Mutation evidence is targeted, not exhaustive.

Graph generation after integration: `2026-11-12T01:17:09Z`, Tier 2, all four paths content-verified. The module and tests contain unresolved calls; exact-source review supports the bounded conclusions. No complete call-graph or live composition claim follows.

Builder custody also records one recovered workspace mistake: an omitted working directory briefly created a new file of its own bytes in the shared checkout. It moved those bytes into its isolated file and removed the accidental file; existing shared bytes were preserved. Integration subsequently rechecked all affected destination paths.

## Limits and continuation

A4, task-host admission, real research publication, H2 execution and baseline comparison have not been implemented or run by M1. R-2 integration/independent acceptance and Q-B1 distribution clearance remain open. M0’s D-6 and scoped dispositions retain their prior standing. No scientific finding or new grant is accepted by this result.

The objective archive is `/home/bline/cbm-work/research-lifecycle-m1-20261111/`: assignment, authorization provenance, checkpoint receipts, review reports and exact terminal events, mutation evidence, source-bound gate records and integration receipt. The isolated worktree remains available for evidence. Root continuation and budget records own collection/closure status; no active follow-on work is implied.

Last measured objective cost before closure: **$6.6138172 of the supervisor’s $20 cap**, API-equivalent dollars adjusted for subscription speed, at 2026-11-12T01:18:01.996921Z. Account meter: 97% remaining. These are a cost proxy and an account-wide reading; the reporting tail is unsettled. Preparation, failed attempts, builder, review, repair, integration and coordination are included in the objective window. Earlier M0/planning objectives are separate.
