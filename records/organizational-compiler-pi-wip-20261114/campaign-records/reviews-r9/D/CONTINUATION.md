Same isolated D episode. Root is sole persistence/collection owner. No refresh or ordinary scope is admitted; preserve all current holds and complete remaining scope. Original complete native final and later cap acknowledgment follow verbatim.

**changes_required — partial review; preservation required before refresh.** One P2 source-closure defect is established. Review D is unfinished; this is not whole-subject acceptance.

Root must persist this complete report at `reviews-r9/D/CONTINUATION.md` and publish its bound recovery pointer before my native refresh. I have not invoked `new_context`: the new findings and remaining obligations currently lack the required durable owner.

Identity and authority remain:

- Reviewer session: `01a1d950-c03e-7702-ae2c-7d83944683ef`.
- Sole collector and persistence owner: Root `01a1d364-788d-7490-8fed-758679336e2e`.
- Bound `identity-binding.json` records `gpt-6-astra/xhigh/default`, read-only/never. These are recorded settings, not backend-fulfillment or principal-independence evidence.
- Root owns listener `12267` / native `3753643`. My latest explicit telemetry reading was 130,546 input tokens, epoch 0, at `09:12:51Z`. A subsequent mandatory context-pressure instruction stopped ordinary work.
- Target3/cap5 remains inside unchanged user100/no deadline. Final local expenditure is unknown to me.

The exact reviewed subject is:

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

Here `N=/home/bline/cbm-work/org-pi-review-continuation-20261114`, `S=N/implementation-r6/preparation-03`, `Q=N/realization-r9/evidence-gate`, and `P1=/home/bline/cbm-work/wt-p1-vertical-20261113`.

**ORG-PV-R9D-001 — P2: material build-probe sources are missing from prospective and common input custody.**

The following files are absent from the cold verifier’s 448 recorded inputs, the map’s external inputs, and the source-manifest entries:

- `/home/bline/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/thiserror-2.0.21/build/probe.rs`
- `/home/bline/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/proc-macro2-1.0.107/src/probe/proc_macro_span.rs`

Their build scripts invoke rustc on these files and use the compilation outcome to choose configuration. They then delete the probe directory, including its temporary dep-info. Decisive source:

- [thiserror build.rs:32](/home/bline/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/thiserror-2.0.21/build.rs:32), with invocation at lines 103–153 and cleanup at 157–173.
- [proc-macro2 build.rs:165](/home/bline/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/proc-macro2-1.0.107/build.rs:165), with invocation through line 222 and cleanup beginning at 229.
- [verifier_bootstrap.py:15](/home/bline/cbm-work/org-pi-review-continuation-20261114/implementation-r6/preparation-03/verifier_bootstrap.py:15): discovery scans surviving `*.d` files; prospective additions include manifests/build scripts, without completing these scripts’ material source reads.

The actual cold-build outputs corroborate the relevant paths:

- `S/verifier-cold-target-01/debug/build/thiserror-8193892ab42fb844/output`
- `S/verifier-cold-target-01/debug/build/proc-macro2-2737d31f4c77ada2/output`

They contain the respective `cargo:rerun-if-changed` declarations. The source establishes their use; these are not kernel-load attestations.

**Static counterexample:** A preexisting change to an omitted unpacked probe can change its compilation or configuration result without changing any declared source input. Even another failing probe can preserve the same result while violating the claimed exact source closure. Cleanup removes the temporary dep-info, so bootstrap reconciliation still finds no uncovered surviving dep-info path. Registry reconciliation checks only the selected recorded source entries; retaining the `.crate` archive does not establish that the omitted unpacked file read during the build matched its archive member.

**Consequence:** Complete cold-verifier and common-build source/input closure is unestablished despite `uncovered: []`. This concerns RLA-005 and the source-closure obligation. Confidence is high for the omissions and mechanism. No counterexample was executed, no runtime regression is claimed, and historical recurrence/escalation classification belongs to Root. Present-day `thiserror` probe bytes match its archive member; that does not supply the missing prelaunch observation.

**Unresolved runtime correspondence concern — not yet a separate finding.**

The map’s `test-native/native-node` declaration uses argv0 `node`, cwd `P1/rust/crates/work-engine-review-delivery`, and PATH `/usr/bin:/bin`. Actual source:

- [native_vertical.rs:42](/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery/tests/native_vertical.rs:42) resolves `node` to a path before constructing `NativeConfig`.
- [native.rs:1027](/home/bline/cbm-work/wt-p1-vertical-20261113/rust/crates/work-engine-review-delivery/src/native.rs:1027) launches `cfg.node`, clears the environment, and sets the child cwd to the fixture cwd.
- `Q/src/lib.rs:300` checks descendant source membership and declared resolution, without mechanically establishing declaration/source correspondence.

The literal declaration differs from the child launch. Remaining work must decide whether the approved declaration represents the parent’s lookup operation adequately or falsely represents the descendant invocation. The supplied `native_node_selection` reconciliation is relevant; it must not be overlooked. I have not completed that interpretation.

Completed bounded evidence:

| Obligation | Established so far |
|---|---|
| Runtime identities | Read `runtime.rs`, relevant `lib.rs`/`spec.rs`, and runtime controls around `gate.rs:3000–3220`. Link-object versus endpoint content, directory-link resolution, explicit/PATH lookup, changed endpoints, contradictory custody kinds and real custody-producer agreement have inspected implementations. Source-to-descendant correspondence remains unresolved. |
| Custody summary | Independently checked all **20 actual records** against map argv/cwd/environment, required input membership, record hashes, stream hashes/lengths, ordered timestamps, zero exits, no timeouts/signals and empty leftovers. Summary order and plan pin matched; `not_run=[]`. Checked 541 distinct permitted input objects against current recorded bytes/kinds. Protected configuration paths were deliberately excluded from current filesystem reads. |
| Pre/postflight | Read actual outer custody and approved pins. Preflight was 24/24, `08:28:59.876735Z–08:29:37.158442Z`; common execution ran `08:29:37.309257Z–08:31:09.289269Z`; postflight was 113/113, ending `08:33:24.270171Z`. Both outer records report zero exit and no leftovers. Complete outer-stream/summary appraisal remains unfinished. |
| Relative dep-info | Read and hash-checked all **40 cold-build dep-info files**. Recomputed **349 paths** and all eight relative paths: `src/{base,ledger,lib,main,outputs,runtime,spec,strict}.rs`. Each resolves against Q and is prospectively bound. This earlier specific gap is now supported, subject to the separate omitted-probe defect. |
| Generated sources | Checked all three `OUT_DIR/private.rs` derivations for serde, serde_core and thiserror: actual Cargo build-script/compiler events, bound build-script source, generated/retained byte equality, and source templates/version substitution. |
| Registry closure | Compared **336 selected registry source entries** to members of **24 crate archives**, their recorded inputs and Cargo.lock archive checksums. All checked joins matched. This does not cover the omitted probes. |
| Cold verifier | Actual cold record binds 448 inputs exactly matching its prepared plan/prospective inventory. Cargo emitted a non-fresh verifier; terminal build message reports success. Emitted and retained hashes match `e5e7c49a62fe0f0d6741ed483666cb84be0cde53eec7731c78ad6cba56582e00`. Retained copy is mode0555, nlink1, distinct inode. Complete-input claim is blocked by R9D-001. |
| Five executable joins | Independently matched each retained manifest entry to actual Cargo stdout, producer/direct-record hashes, direct argv0, recorded executable input digest and current source/retained bytes. All retained copies are mode0555, nlink1, distinct inodes. |

The five verified executable hashes are:

```text
test-pure          f679d435b40bfb82a919e042a901dfce23429a87e310516de3f1b4897fa1c519
test-binding       15f9d3c93a573d36afd571e758d845495f4af729170dbf6dee8d771fb523bc5d
test-native        40c0b938340e46215d0794dd722ea77d596786687837cfa406a000c30e6b7470
test-pure-default  db757fb7eef2829bdcd49ad9e4189299a826a79ab3dd992680a5c960342886d2
qualifier-test     edba8431aac931841de24eb82ee41064176575aafd907aab9a14b42090b85c0e
```

All remaining D obligations must survive recovery:

1. Finish semantic source closure across preparation, helpers, static/nonliteral includes, neighboring owner APIs, and candidate/checkpoint/gate applicability. I read `prepare-r9.py`, `verifier_bootstrap.py` and `finalize-preparation.py`; this is not complete closure appraisal.
2. Independently finish all four archive member/manifest/source relationships. Root’s supplied physical reconciliation and the finalizer’s assertions were read; I have not independently repeated the complete four-archive correspondence.
3. Resolve the native-node declaration concern and complete all source-bound descendant declarations, including shell/tool context and inherited helper calls.
4. Finish relevant custody helper source inspection, bootstrap-check custody, outer pre/postflight stream/summary relationships and applicable historical tool-custody closures.
5. Finish the relevant executable/inventory negative-control bodies and their actual results. Mechanical five-way binary correspondence is complete; complete qualification semantics is not.
6. Finish reading relevant earlier exact findings/counterexamples and applicability dispositions. R8 qualifier report and R8 root reconciliation were read; R7 report received selected passages only. No historical finding is closed.
7. Complete graph coverage checks for newly relied-on private/dependency/helper paths and direct fallbacks. Supplied D coverage and all 112 fallback expressions were read; initial private sources are outside MAIN. No graph completeness claim applies.
8. Reconcile D’s input/source findings with A/B/C’s semantic coverage through Root. I have not independently reviewed all raw bundle/domain joins or adopted peer conclusions.
9. Recover any necessary truncated material before relying on it. Oversized early map/reconciliation/source outputs were truncated; selected relevant fields were subsequently reread, but the omitted material is not completed coverage.

The complete packet/common contract and D appendix were read. Repository manual-orchestration reviewer instructions, AGENTS/core, relevant evidence/review doctrine, R7/R9 plans, root design/scratch dispositions and R5 qualification boundary were loaded. R8 plan received substantial but not complete visible output.

No product/build/test/fixture execution, artifact mutation, qualification replay, retention replay, agent launch, session enumeration or ordinary rollout-message access occurred. Read-only analysis scripts inspected existing evidence. The required board read was performed under explicit authorization. No owned process remains; Root’s listener remains Root’s collection responsibility.

All18/history35 remain open. This report supplies a blocking defect and precise partial coverage, not completion of D or whole acceptance.

I invoked repository [Wind Walker](/home/bline/code/work-engine/skills/wind-walker/SKILL.md). Its requirement is: “Context replacement is valid only when correct continuation depends on no meaning represented solely in the current context.” Root persistence and the recovery pointer are therefore required before refresh; the read-only grant remains unchanged.

Acknowledged. Ordinary review is stopped; no refresh or further reads will occur before Root supplies the durable preservation binding and exact admission.

My preceding complete partial report remains the preservation payload. Add Root’s observations: D186610/epoch0 at09:15:25Z; local4.185938/5 and aggregate87.3436688/100 at09:15:50Z. Root owns listener12267 and collection.

Root reports C’s two static P2 findings adopted for design escalation. No ordinary repair authority is granted.
