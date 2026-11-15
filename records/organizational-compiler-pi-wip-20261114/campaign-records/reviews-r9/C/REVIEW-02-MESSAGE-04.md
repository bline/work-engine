**changes_required — two P2 static findings; review remains incomplete. preservation_ready.**

Root must persist this complete report and publish the recovery pointer for session `01a1d93f-b011-75d3-81bf-daf6a9141379` before admitting its native refresh. I have stopped ordinary work. I have **not** called `new_context`: the read-only grant prevents me from writing the continuation or chatboard pointer, and the new findings and remaining scope are not yet durably preserved by root.

Use the existing owner:

`/home/bline/cbm-work/org-pi-review-continuation-20261114/reviews-r9/C/CONTINUATION.md`

This report extends the prior continuation; it does not close any historical finding or establish whole-subject acceptance.

**Subject and authority**

```json
{
  "checkpoint_commit_oid": "2617a6c622c74f330e3bd7722dff2e59fb332c87",
  "checkpoint_tree_oid": "e5bc27ac9dcdb6c9efd3930d01cc56f4b5caa46c",
  "task_patch_digest": "aaa62bc36ad9aa55cbcd8f1391557ea27b252a342f7b28711cc29b39b4b01f4d",
  "plan_version": "sha256:ab8cf7ad1d79565dbbce3b422ceb927999f13eed5e9d21f9041b343dbd21aa9d",
  "scope_revision": "r9-capture-lifetime-complete-observed-bundle-whole-offline-vertical",
  "gate_receipt_digest": "40cc6724663af029ca84abeef3371575ae3c06c19e24b7ff2fe24d0cccd176b8"
}
```

The same isolated episode continues under observed **Astra/xhigh/default, read-only/never**, restricted network. Post-boundary settings were verified at own rollout lines 259–260. Root `01a1d364-788d-7490-8fed-758679336e2e` remains sole collector, persistence owner and acceptance authority.

Abbreviations below:

- `N=/home/bline/cbm-work/org-pi-review-continuation-20261114`
- `Q=N/realization-r9/evidence-gate`
- `S=N/implementation-r6/preparation-03`
- `R=S/runs/inv-r9-qualify-01/run`
- `P=/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery`

**CR9-001 — P2: Native diagnostic support can contradict its producing observation without refusing qualification**

Evidence:

- [outputs.rs:1112](/home/bline/cbm-work/org-pi-review-continuation-20261114/realization-r9/evidence-gate/src/outputs.rs:1112), especially line 1136, accepts `native-error-frame.txt` for a delivery-refusal control when it is nonempty UTF-8. It does not add those bytes to `Art` or compare them with the native diagnostic.
- [native.rs:1067](/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery/src/native.rs:1067) chooses the actual diagnostic from driver diagnostics, interpreted defects, interpreted error frame, or missing observation.
- [common/mod.rs:581](/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery/tests/common/mod.rs:581) persists that exact returned diagnostic.
- The actual `R/controls/duplicate-start/native-error-frame.txt` contains the native error frame reporting duplicate-start refusal. Its raw received stream contains allocation followed by that error.

Static counterexample: replace only that diagnostic with an unrelated nonempty UTF-8 message, updating its output-manifest length/hash consistently. The only semantic predicate for this support file still succeeds; raw stream, capture refusal, process facts and remaining appraisal are unchanged.

Consequence: qualification can bless contradictory surviving diagnostic evidence, contrary to complete applicable appraisal of every supplied entry. This does **not** show invalid input reaching the consumer. It exposes a qualifier correspondence gap.

Confidence: **high from source**. The substitution was **not executed or written**. The actual retained diagnostic agrees with its raw error; the defect concerns the accepted counterexample. Root owns historical recurrence classification and any resulting design disposition.

**CR9-002 — P2: Delivery-refusal appraisal omits sent-start semantic correspondence**

Evidence:

- [outputs.rs:1373](/home/bline/cbm-work/org-pi-review-continuation-20261114/realization-r9/evidence-gate/src/outputs.rs:1373) dispatches delivery-refusal scenarios to `refusal_evidence`.
- [outputs.rs:1003](/home/bline/cbm-work/org-pi-review-continuation-20261114/realization-r9/evidence-gate/src/outputs.rs:1003) checks refusal justification and retained observation projections, but does not inspect start-frame fields.
- [outputs.rs:782](/home/bline/cbm-work/org-pi-review-continuation-20261114/realization-r9/evidence-gate/src/outputs.rs:782) delegates send accounting to `check_attempt_evidence`.
- [attempt.rs:276](/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery/src/attempt.rs:276) checks frame hashes, lengths, newline framing, accepted-byte counts and outcome consistency. It does not join start content to the binding or observation.
- Exact start-field comparisons occur in [outputs.rs:1666](/home/bline/cbm-work/org-pi-review-continuation-20261114/realization-r9/evidence-gate/src/outputs.rs:1666), reached for complete delivery/consumer paths.
- [native.rs:982](/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery/src/native.rs:982) constructs the sent contract/result from the binding. Intentional wrong-session and duplicate-start controls change their specified aspects, not arbitrary contract/result correspondence.

Static counterexample: begin with actual `R/controls/delivery-refusal`, whose received allocation/observation is complete, whose one start send completed, and whose child exited 5. Change only the sent frame’s `result_text` to another same-length string, updating its send digest and affected output-manifest digests. Keep the captured synthetic result and received history unchanged. Send accounting still agrees; exit 5 still justifies the same refusal; observation projections still agree. No invoked predicate compares the contradictory sent result with the binding/history.

Consequence: a complete declared refusal bundle can qualify despite contradictory raw sent/domain evidence. This is narrower than requiring successful protocol semantics for every refusal: intentional duplicate/wrong-session sends and partial attempts must retain their own permitted meaning.

Confidence: **high from source**. No modified bundle or replay was executed. Actual retained start fields agree except the intentional wrong-session control.

**Completed coverage and evidence limits**

The entire `Q/tests/gate.rs` body—3,554 lines—has now been read, including previously truncated passages. Coverage includes synthetic baseline construction, actual capture-writer use, owner fixtures, hash-resynchronization helpers, inventory/evaluator failure, qualifier build/run joins, stage composition, optional context, partial/unknown collection, recursive duplicate controls, write remnants, failure support and scratch selectors.

Additional complete source reads: `P/src/{lib,wire,binding}.rs`, `S/prepare-r9.py`, and `S/finalize-preparation.py`. Earlier complete reads of `Q/src/{outputs,strict,ledger,spec,base,lib}.rs` and `P/src/attempt.rs` remain preserved.

New native-test reads covered `P/tests/native_vertical.rs` lines **1–537** and **2068–end**. These establish the `DeliveryCapture` guard’s scope through invocation, returned capture, caller assertions, finalization and scratch eligibility; wrong-verdict controls retain the guard during unwind. The unfinalized-manifest test explicitly establishes preservation under failure, not a successful underlying bundle.

Read-only artifact analysis visited the positive and **all 53 mapped complete controls**, reading **1,043 regular files / 13,187,697 bytes**. It found:

- No mismatch between listed regular-file bytes and manifest lengths/hashes, and no unlisted regular-file membership discrepancy.
- No duplicate-member or parse error in retained `.json` artifacts.
- No discrepancy in direct raw observation → retained observation/context/history projections.
- No discrepancy in send-frame byte coverage, lengths and recorded digests.
- The intended wrong-session difference was the sole observed sent-start field mismatch in the inspected fields.
- All 53 controls had one exact scenario scratch match, suffix `3720966`; positive scratch was absent.
- All six later-failure records’ listings matched the retained file paths and lengths.
- `write-failure/episode-after.json` was an empty directory. No other nonregular bundle entry appeared in this inventory.

These are bounded physical/projection observations, **not full semantic qualification**. The analysis did not run the qualifier, replay the map, reconstruct canonical owner transitions, or establish every schema/domain predicate. Raw protocol malformations in negative controls were retained and reported as expected observations.

**Actual shared-interface paths**

| Representative | Bundle | Observed stage/count | Exact retained scratch |
|---|---|---|---|
| Positive | `R/evidence` | delivered/1 | absent after successful cleanup |
| Assertion unwind | `R/controls/assertion-unwind` | delivered/1; failed | `R/scratch/control-assertion-unwind-3720966` |
| Consumer refusal | `R/controls/consumer-refusal` | consumer refused/1 | `R/scratch/control-consumer-refusal-3720966` |
| Delivery refusal | `R/controls/delivery-refusal` | delivery refused/0; exit 5 | `R/scratch/control-delivery-refusal-3720966` |
| Die after allocation | `R/controls/die-after-allocation` | delivery refused/0; exit 4 | `R/scratch/die-after-allocation-3720966` |
| Die during prompt | `R/controls/die-during-prompt` | delivery refused/0; exit 3 | `R/scratch/die-during-prompt-3720966` |
| Duplicate start | `R/controls/duplicate-start` | delivery refused/0; two starts | `R/scratch/duplicate-start-3720966` |
| Per-frame duplicate | `R/controls/duplicate-start-per-frame` | delivery refused/0; two starts | `R/scratch/duplicate-start-per-frame-3720966` |
| Chunked duplicate | `R/controls/duplicate-start-chunked` | delivery refused/0; two starts | `R/scratch/duplicate-start-chunked-3720966` |

All three duplicate-start bundles show allocation followed by a native duplicate-start error, exit 2, and count 0.

All other mapped bundles were included in the physical/projection inspection: the four `wrong-*` outcome controls; `write-failure`; setup, consumer and delivery refusal families; missing-key/provider-entry/early-exit/limit/timeout cases; ten `trail-{duplicate,malformed,error,invalid-utf8,incomplete}-{prompt,delayed}` controls; and all sixteen `wire-{Allocation,Observation,Context,History}-{false,true}-{false,true}` controls.

The duplicate-member families retained duplicate keys in actual raw bytes and count 0. Their allocation variants sent no start; the other variants retained allocation plus an invalid observation. `wire.rs` recursively rejects duplicate members, and native tests assert zero actual consumer invocations before capture finalization. This provides corrective support for WR7-001; complete runtime/source applicability still requires the remaining shared work.

**Assigned-obligation dispositions**

| Obligation | Disposition |
|---|---|
| `qualifier-total-inventory` | Substantial source/harness support; final applicability incomplete pending actual preparation/reconciliation/custody joins. |
| `qualifier-bundle-composition` | **Changes required:** CR9-001. R9 remnant and later-failure corrections have bounded corrective support. |
| `qualifier-byte-domain-joins` | **Changes required:** CR9-002. Direct actual raw projections inspected across all bundles; full owner semantics remain incomplete. |
| `qualifier-real-capture-controls` | Harness fully read and actual bundles physically inspected; semantic completion remains blocked by the findings and remaining native/owner boundaries. |
| Contribution to `native-capture-lifetime` | Guard and wrong-verdict/finalization paths provide corrective support. Full delivery-invoking path coverage remains incomplete here and requires B’s collected evidence. |

Historical applicability remains bounded:

- **QR8-001:** source and current empty-directory evidence support correction of the prior hidden-remnant mechanism; regular-file/symlink/nonempty-directory controls are present.
- **QR8-002:** source, harness and six actual later-failure listings support correction of null/malformed failure support and scratch/listing mismatch.
- **WR7-002:** observed-stage composition and contradictory-survivor controls support correction.
- **WR7-001:** recursive parser and native duplicate controls provide corrective support described above.
- **R6A-002 / inventory-evaluator failure:** ledger and harness reject missing, unperformed, panic and unsuccessful checks.
- **IR004/R6A003 and other earlier closures:** complete applicability is still unestablished. The new findings concern the same broad complete-appraisal boundary, but root must classify recurrence.

All **18 historical findings remain OPEN**.

**Exact unfinished scope**

1. Complete semantic appraisal of the actual common/domain artifacts across all 54 bundles: canonical candidate/preparation/declaration/binding inputs; admissions and authority variants; canonical before/after states; revisions, consumption and owner-transition correspondence. Reading/hashing those bytes did not complete those judgments.
2. Read remaining relevant native lifecycle/setup/collection bodies. In this episode, `P/src/native.rs` **1–904** and **1088–1151** remain unread; later necessary passages already read are preserved.
3. Read `P/tests/common/mod.rs` **1–241** and remaining necessary native-control bodies, particularly `P/tests/native_vertical.rs` **538–2067**, against actual bundle production and zero/one consumption.
4. Inspect actual canonical fixture/episode owner implementations and source-bound inputs needed for C’s shared joins. A’s review does not become C’s evidence until root collects and reconciles the interface.
5. Read and appraise the actual preparation/reconciliation records, qualified pre/postflight/custody records, archive/source-manifest joins and relevant generated-source correspondence. The two preparation scripts are now read; their emitted records and actual qualified joins are not fully inspected here.
6. Read relevant earlier exact native/IR004/R6A002–003 reports and counterexamples beyond the already-read R7 whole and R8 qualifier reports. Finish IR001–003/EA001–003, provenance, binding, capture, inventory and retention applicability.
7. Preserve explicit B/C shared-interface assessment and D’s outstanding deep runtime/source/cold-build/executable contribution. Root owns collection and whole-standing.
8. `R/unfinalized-manifest-control` remains **incomplete/unestablished underlying evidence**. Its source-level preservation test has been read; its actual directory has not been independently appraised here.

**Recovery, capacity and process disposition**

Latest own bound telemetry: **174,479 input tokens, epoch 1**, timestamp `2026-11-14T09:10:25.622Z`, `latest_valid_observation_with_record_gaps`. The disclosed gap is the earlier reset token-count record. Remaining substantial scope exceeds the margin to preparation at 185,000. A developer instruction now requires immediate preservation/refresh; ordinary work is stopped.

Last root-delivered cost: **7.338654/9 proxy at 09:08:20Z**. Current spend is unknown; root must reconcile the tail before another substantive admission. Mark 6.5 was crossed. No new budget is inferred.

Root retains listener **5425**, alert delivery and sole result collection. The next turn should verify root persistence and its recovery pointer, perform the authorized same-session native refresh, recover this exact episode, verify post-boundary settings, and return readiness before further substantive admission.

No product/build/test/fixture execution, replay, mutation, agent launch or paid inference launch occurred. All read-only inspection commands completed; no reviewer-owned background process remains. Earlier access deviations remain governed by `ROOT-RECOVERY-DISPOSITION.json`; this continuation makes no renewed freshness or retroauthorization claim.
