# Research lifecycle M2 — private R-2 qualification result

Date: 2026-11-11 (completion 2026-11-12 UTC). Status: **bounded private technical qualification accepted**. The user authorized private qualification and separate review, then said “proceed” after the tooling work. Integration and distribution remain held; Q-B1 remains unresolved. This result grants no applicability or execution authority.

## Qualified subject

Baseline `0cfa1e05182f2fdbc236648e09ad07ffcc3c5b03`; final private checkpoint `c3d82322111f2f342ba9c3cc5a63472c53b621d2`, tree `254d1ca06906c8a551fba563aed91d4100ed80e7`; full 42-file patch SHA-256 `0d1a9a100fb50d23123a23c654711b065a4cc55fb636e3dba2f03ee531b2ad97`; gate SHA-256 `e54290b1bcc025b9773d383ff5d8873d0c0e65e2031450a49b7b7012347a1f21`. Plan `M2-completion-builder-packet-20261111`; scope `private-full-R2-composition-qualification`.

The private worktree combines accepted M1, preserved R-2/F-01 repair and the separately qualified blind native diff. This slice adds two discriminating production-path fixtures and a frozen collision input. The final checkpoint also corrects one stale ambiguity-test comment. M1 source/tests/schema documentation and the blind diff bytes are unchanged; no production behavior changed in this completion slice.

## Consequences established

- Missing packed revision: only the required revision’s verified one-object pack becomes unavailable. The target commit, source/target trees and blobs remain readable. The same validated register and pinned snapshot return `cannot_read` for the required revision, with no report. Restoring the pack restores successful controls. The original loose-object F-01 regression remains.
- Post-validation ambiguity: a second valid commit with the same seven-digit prefix is introduced after successful validation. Actual production lookup returns `invariant_failure`, with no report; removing it restores successful same-value and fresh controls. Collision preparation was bounded offline; CI uses frozen payloads.
- Whole-batch qualification: all 11 decisions and 22 source/target rows were inventoried against Git, containing 12 distinct blobs and 812,344 conservatively counted input bytes. The largest blob is 100,621 bytes/2,337 lines. Actual anchors completed in 0.468 seconds, producing 25,471 stdout bytes and zero stderr.

The private harness checks admission at 16 decisions, 1 MiB/10,000 lines per blob and 8 MiB aggregate input. The enclosed anchor invocation has 60 seconds, 1 GiB address space and conservative 4 MiB-per-stream output caps. Recorded fit, prelaunch refusal, explicit unavailable and synthetic timeout/memory/output controls pass. A prior control exposed a short write at the output cap followed by exit zero; the corrected harness refuses cap-reached output without consuming a possible partial report. These are private allowances, not product constants or worst-case algorithm guarantees.

## Validation and separate review

Runtime-qualified parent `7522ad97a2c2029662692accee729d6894bd2838`, gate `cad6f697a203ba29a4f665e3d5b72c09246c352d6c4c38e6fca5be292fff042a`: 108 tests pass; one optional external-register helper is ignored. All 3,233 Git diff-oracle cases, 1,503 reusable original-review replay cases, and 29 historical targets/319 rows pass. Required history ran without a silent skip. Formatting, crate Clippy with warnings denied, and joined workspace check pass; workspace check retains an unrelated existing dead-code warning.

Root verified 1,154 unique custody/input/output digests, all 15 gate steps, 42 candidate blobs, preserved M1/diff bytes and unchanged private HEAD/index. The final checkpoint differs from the runtime-qualified parent only in two non-executable doc-comment lines. Its exact-input formatting gate passes; prior runtime qualification is explicitly carried forward on identical executable/fixture/dependency/helper bytes. Runtime suites were not rerun on those new comment bytes. Failed fixture and gate attempts remain preserved.

Fresh separate review M2-R2-ADV-01, session `01a1ce79-6b0d-7ef2-b2ae-5c4d8a9fed37`, initially accepted the complete parent with no P1/P2 and one optional P3 stale comment. Actual Astra/xhigh/default/read-only configuration was independently observed; the reviewer acknowledged its exact identity/collection binding. Retained exact-turn review of the final comment delta accepted it and resolved P3 with no new P1/P2. Review execution and source/custody assessment are distinguished in the saved reports; root makes the bounded private acceptance.

## Remaining boundaries

Q-B1 needs an explicit attributable owner disposition before integration/distribution. The blind authoring/exposure record improves the evidence; neither functional parity nor this technical review establishes legal clearance. Original R-2 REJECT and the incomplete repair delta recheck remain historical records; they are not rewritten by this later verdict.

M3 must bind actual in-process admission and enclosing host controls or retain an explicit unmet resource condition: the synchronous Rust API has no intrinsic cancellation. Finite oracle agreement is not universal equivalence. Historical projection coverage remains selected fields and unchanged/moved/modified cases. M0’s first-use refusal for an unestablished required anchor remains approved; actual A4 enforcement belongs to M3. Broader rename/partial-card/history interfaces remain unresolved or explicitly unsupported. No A4, H2 execution, research publication, scientific-benefit finding or execution grant follows.

## Custody and cost

Archive: `/home/bline/cbm-work/research-lifecycle-m2-completion-20261111/`; private worktree: `/home/bline/cbm-work/wt-research-lifecycle-m2-completion-20261111`. Assignment/authority, builder/checkpoint/custody records, original and comment-delta reports, source verification, acceptance and budget own the exact evidence. Original candidates and evidence were preserved. This slice introduced no shared source edits; shared HEAD and index remain unchanged, and this result record is its only new shared artifact. The closure check observed concurrent edits in `skills/manual-orchestration/scripts/pi_container.py` and `skills/manual-orchestration/tests/test_pi_subscription.py`; those edits were left untouched and are outside this private qualification. No user-visible commit, push or release occurred.

Latest measured objective cost before closure: **$7.1855296 of the supervisor’s $12 cap**, API-equivalent dollars adjusted for subscription speed; warning mark $8. The reporting tail is unsettled. No paid API/provider calls were made. Root closes the budget, collects its observer and releases claims in the accompanying terminal continuation/assignment records; these records own final process status.
