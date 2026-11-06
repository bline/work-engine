# First useful local Work Engine development pilot

Recorded 2026-11-06 UTC. Status: **bounded pilot and exact repository task
accepted** by the integration owner after retained independent review.
Source acceptance is `9a731e57a213242ba7f325a28cd33181bf3bba54`; the final
publication receipt identifies the subsequent records commit and local merge.

An actual Events HTTP MCP request reached the Rust command, real job server
and Pi producing runtime. Pi added useful regression tests to the repository's
timestamp parser, passed its original independent validator, reset/recovered
its context and returned a receipt. The exact exported patch passed the real
crate suite, mutation checks and separate review before publication.

The first live run exposed a consumer receipt-precision defect. It was repaired
and qualified offline using the **unchanged original raw receipt** through
capture, persistence, command serialization and restart. There was **one actual
provider trial**, not a second run of the corrected consumers. Historical
rounded projections remain preserved as failed evidence.

## What changed

- A separate version-2 `coding-job-local-live-v1` / `local-live` profile joins
  existing domain owners. Offline profiles keep their restrictions. Trusted
  startup pins and exact operator authorization bind the job; transport
  credentials and enable flags do not mint execution authority.
- The same producing executable supports fixed run/inspect/reconcile/cancel
  verbs while retaining its legacy entry. Run installs the existing lifecycle
  gate. Readback uses the same binary-bound journal; reconciliation is not a
  replacement execution. Receipt delivery remains separate from acceptance.
- Server and command consumers enable `serde_json`'s `float_roundtrip` feature
  to preserve native floating-point values. The misleading verbatim-stdout
  projection label is corrected.
- The actual development task adds two tests/eight explicit-offset boundary
  cases to `rust/crates/context-pressure/src/timestamp.rs`. All **5,847 original
  bytes** remain unchanged; the returned file is 6,766 bytes, adding 919 bytes
  in 29 lines. No parser behavior change or production parser bug is asserted.

Private source lineage: `c436de8b` implemented the join, `a77732cd` repaired the
test prerequisite, `3a6c4c4d` records the actual exported task patch, and
`9a731e57` integrates the precision repair (private original `4be395fd`).

## Actual execution and acceptance

| Evidence | Observed result |
| --- | --- |
| Outer foreground custody | Exit 0, 86.5 seconds, no leftover processes |
| Actual operation | `190638c1e851ba397245f606384b9ed7a392407779b80b6627622b86e800de27` |
| Raw receipt SHA-256 / ID | `42ba964cb7c9f0430fb16828fdb94de18cfa1fc58d199cdf1b7ece9b8e6b26c6` |
| Provider/harness | Pi SDK 1.5.0, `openai-codex`, `gpt-6-luna`; four HTTP-200 completed responses |
| Effect/context bounds | One workspace command, one physical SDK reset, two sessions, three containers, no implicit retries |
| Original independent acceptance | Workspace run and separate immutable validator each passed 5/5 tests |
| Repository qualification | All 37 standalone crate tests and formatting passed; no tests ignored |
| Mutation controls | Original UTC test still passes both mutants; new tests reject reversed offset adjustment and missing lower TimeClip bound |
| Retry/restart | Exact repeat after collection and server restart preserve the original operation/receipt and entry/run records; no second collector effect |
| Cleanup | Runtime records verify 38 artifacts and 25 process streams; fresh Root inspection independently finds all three owned containers absent; projected native access files removed |

The successor began with zero messages, no tools and auto-compaction disabled,
then recovered the exact checkpoint and reached `CompletedSettled`. Reset was
deliberately triggered at low observed usage, not natural context exhaustion.

The same producing process acknowledged delivery. Native receipt acceptance
remains `owner_pending`, with domain settlement `not_claimed`. Root's supervised
task/pilot acceptance and authorized local Git publication are separate from
that native field; no journal or task-state transition was fabricated.

[Runtime validation](RUNTIME-VALIDATION.json),
[caller-path and fresh cleanup checks](FLOW-VALIDATION.json),
[task output](TASK-OUTPUT.json), [crate custody](TASK-CRATE-CUSTODY.json) and
[mutation controls](MUTATION-GATES.json) bind those observations. The
[raw receipt](RAW-RECEIPT.json) and [original wrapper](RAW-HOST-WRAPPER.json)
are copied byte-identically. Generated service/job/MCP secrets and native
credential contents are excluded from the record.

## Review and repairs

One fresh Astra/high/default read-only, restricted-network reviewer retained
the episode through source, operator, actual-output and remediation reviews.
This establishes separate reasoning context and an observed mutation boundary,
not provider independence or backend fulfillment. Root collected every exact
native turn and made acceptance decisions.

1. **LIVE-REVIEW-001:** the new external-command integration test lacked its
   prerequisite ignore marker. The focused repair preserved ordinary test
   usability and explicitly ran the configured tests against final source.
   [Initial finding](REVIEW-01-SOURCE.json),
   [accepted correction](REVIEW-02-SOURCE-REPAIR.json).
2. **LIVE-REVIEW-002:** four derived catalog-cost floats changed during server
   capture and remained rounded in caller/restart projections despite the
   original receipt ID. Raw receipt integrity, usage, integer settlement and
   task output were intact; exact payload passthrough was false. The preserved
   pre-fix control fails on the first exact value. Corrected consumers preserve
   the full original receipt and all four values through the actual consumer
   path and restart. [Actual-pilot finding](REVIEW-04-ACTUAL-PILOT.json),
   [repair and causal evidence](PRECISION-REPAIR.json),
   [final acceptance](REVIEW-05-PRECISION-REPAIR.json).

The final affected checks passed: 10 explicitly enabled Events/CLI tests;
ordinary integration tests 5 passed/5 ignored; server library 29 passed/1
ignored; command library 6 passed/1 ignored; command binary unit target zero
tests; server/command Clippy, formatting and builds. Earlier producer tests
and source review remain applicable to the unchanged producer, whose SHA is
`223b4c56fba64d7c201af1a78a7b6ac0c8865288e305fc58d2482faafb883149`.
The exact-original-receipt regression uses private operator/shell fixtures;
it does not perform another Host/provider realization or reconciliation.

An earlier unit fixture's direct journal truncation produced `custody_lost`;
atomic publication repaired that fixture and the failed check is retained.
It does not establish the cause of the historical intermittent CLI failure.
The first crate command selected the root workspace, which excludes this
standalone crate: it ran no tests and failed. The corrected standalone manifest
passed all 37. An initial mutation collector syntax error executed no gates;
its corrected collection preserves expected mutant failures as controls.

## Authority, cost and readiness

The user approved **$45 maximum / two hours** at 00:04:42.953 UTC, including
preparation, implementation, all actors, review/repair, trials and closure.
The prior $25 round stayed closed. Root wrote the detailed
[executed plan](../../planning/local-development-pilot-budget-and-acceptance-20261106.md)
under that scope; later-written plan bytes are not claimed as exact human
approval. The first feasibility gate passed before $8/00:24:42.953. Productive
work ended by 01:19:42.953; hard deadline remained 02:04:42.953. Later work was
review, records, guarded publication and closure.

[Accounting](ACCOUNTING.json) is a completed-call, speed-adjusted API-equivalent
cost snapshot, including all bound actors once. It conservatively retains
$1.40 reserved for at most two trials; only the first was used. Actual provider
usage totals 8,597 input and 474 output tokens. The pinned catalog's conservative
integer settlement is **2,510 microUSD**. Neither this nor the orchestration
cost proxy measures subscription quota or final billing; closing tails remain
attributable. Round cost includes infrastructure and qualification work, so it
is not a steady-state per-task cost or Pi/Codex comparison.

This establishes one useful, supervised local development path. Transport
identity used an explicit random-token fixture, not real OAuth onboarding.
The Codex supervisor, builder, independent review and publication remained
external orchestration around Work Engine's job execution. No deployment,
global activation, Web/Dot live-job UI qualification, arbitrary jobs, coding
after reset, actual SDK output-cap fault injection, claims-lane compatibility
or broad unattended/daily-use readiness is established.

The [unattended qualification plan](../../planning/pi-unattended-development-qualification-tests-20261105.md)
and separately versioned claims/continuation work remain later scope. Evidence
uses direct private source where graph coverage has unresolved calls; no graph
completeness claim is made. [Root acceptance](ROOT-ACCEPTANCE.json) and the
[evidence manifest](EVIDENCE-MANIFEST.json) preserve exact subjects and limits.
