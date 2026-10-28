# Round-2 tooling: build and code review (2026-10-28)

Manual orchestration, coordinator Claude session `1fa9767a` (Opus 5.5). Assignment record:
`~/cbm-work/round2-tools/assignment-record.json`. Not profiled: no pilot round was active.

- **Builder:** `gpt-6.1-sol`, effort `medium`, standard tier (observed in its rollout),
  thread `01a1840e-2dfb-76b0-a714-24c940f86253`, launched with `role_host.py` in an isolated
  clone, retained across three packets (sha256 `dc8ac4b8`, `846efcef`, `d47c9ce0`) after
  context checks (`retain_ok` at 81K and 122K tokens). 42 calls, $1.24 at the frozen API
  rates. Its sandbox made `.git` read-only, so the coordinator committed its work.
- **Code reviewer:** `gpt-6-astra`, effort `high`, standard tier, a fresh `codex exec`
  session (thread `01a18415-7e74-7b83-a1f3-0d0398cbe21d`), retained for two follow-ups.
  22 calls, $4.23. It ran the tests, injected faults and removed locking in temporary copies;
  the clone was unchanged after each review.
- **History:** review of `5987a47c`: not ready (C-01, C-02 P1; C-03 to C-05 P2; C-06 P3).
  Follow-up of `3547d998`: not ready (C-08 P1, C-09 P2; C-02, C-04 partly). The coordinator
  then replaced manifest-recovery trust with self-verifying records. Follow-up of `57066b1a`:
  **ready to integrate**, no new findings.
- **Integration:** fast-forward of `47587cd6` to `57066b1a`, with HEAD checked equal to the
  builder's base in the same command; 79 tests pass in the repository, none skipped.

The three review messages follow unchanged.

---

## Review of 5987a47c

Verified both commits, branch `round2-tools`, and all three supplied hash prefixes. The working tree remained unchanged.

`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s metrics/scripts/tests` completed: **50 tests passed, 0 failures, 0 errors, 0 skips**.

**Findings**

1. **C-01 — P1: A failed pause write leaves a recorded trigger unenforced.**  
   [Review and repair]( /home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:626) persist their episode records before calling `pause_round`; enrollment checks only `PAUSED`. Injecting an `OSError` at that call reproduced both failures:
   - A Terra P1 remained recorded, but new enrollment succeeded. Retrying `review` exited 2 because its record already existed.
   - Three Terra repairs reached the configured budget, but new enrollment succeeded. Retrying `repair` counted the same completion again, producing four rounds.

   **Consequence:** missed safety pauses and inflated repair counts. **Recommend:** make trigger enforcement recoverable before further enrollment, with retry identity for completed repairs and explicit accounting for owner-approved resumes.

2. **C-02 — P1: Exclusive creation does not make enrollment crash-consistent.**  
   [Enrollment publication]( /home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:583) creates the final filename before writing JSON, then appends its checksum separately. A real subprocess terminated immediately after `O_EXCL` creation left an empty `enrollment.json`; both retry and `status --task-id` exited 1. Separately, failing `append_sums` left complete JSON: retry returned the arm with exit 0, but `review` and `verify` failed because the manifest was missing.

   **Consequence:** the prescribed retry cannot reliably recover a usable enrollment. [Other episode writes]( /home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:494) also separate record publication from manifest publication. **Recommend:** publish complete records atomically without replacement and provide recoverable manifest completion, preserving any committed arm. Include appropriate durability barriers.

3. **C-03 — P2: Assignment publication bypasses the withdrawal lock.**  
   [Withdrawal]( /home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:618) checks for assignments and then writes its terminal record. [Assignment]( /home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:283) does not acquire the round lock. A deterministic interleaving ran the actual assignment command between that check and withdrawal publication. Both succeeded; assignment preceded the withdrawal timestamp, and `status` reported no inconsistency.

   **Consequence:** a dispatched episode can be counted as withdrawn before dispatch. **Recommend:** coordinate assignment publication and terminal transitions under the same lock, reject conflicting state, and report this inconsistency.

4. **C-04 — P2: Enrollment replay returns a corrupted arm without checking integrity.**  
   The [existing-record branch]( /home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:562) reads `arm` directly. After changing an enrolled Terra record to Sol without changing `SHA256SUMS`, retry returned Sol with exit 0; `verify` correctly exited 1.

   **Consequence:** the same task can receive two successful arm responses following accidental record corruption, despite available evidence detecting that corruption. **Recommend:** validate enrollment integrity before returning an existing arm or using records for admission limits; handle recoverable publication interruptions separately.

5. **C-05 — P2: A torn screening append poisons subsequent screening and status.**  
   [Screening]( /home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:523) appends without checking the existing tail; [status]( /home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:690) parses every line unconditionally. Injecting a partial append followed by an I/O failure, then retrying, produced successful enrollment but concatenated the new screening event onto invalid JSON. Status exited 1 instead of reporting records and inconsistencies.

   **Consequence:** enrollment can proceed without a separately parseable eligibility line, and accounting becomes unavailable. **Recommend:** detect incomplete log tails before drawing, preserve damaged evidence through an explicit recovery mechanism, and let status report malformed entries alongside readable records.

6. **C-06 — P3: Concurrency tests do not reliably expose missing locking.**  
   The [process tests]( /home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/tests/test_task_profile_round2.py:97) launch processes without synchronizing their critical sections. With locking removed in a temporary script copy, **19 of 20 test invocations passed**; only one cap test failed. The [failure-after-write test]( /home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/tests/test_task_profile_round2.py:143) correctly tests its chosen boundary, but that boundary is after checksum publication.

   **Recommend:** add controlled process barriers and failures at record creation, manifest publication, and pause enforcement. Retain the existing late-failure test.

Normal-path coverage is useful: both arms, refusal outcomes, late-failure replay, checksums, backward hashes, close calculations, and resume history pass. All **22 legacy recorder tests** pass. `profile` and `attribute` are unchanged; `assign` adds the requested leak checks. `verify` retains its checking and exit-code behavior, but its stdout format changes from text lines to JSON.

For the absence claims above, I inspected the complete new command implementations and searched the recorder and round-2 tests for locking, exclusive creation, synchronization, checksum validation, pause calls, and JSON parsing. Graph coverage reported no recorded gaps for the reviewed source files.

**Verdict: not ready.** C-01 and C-02 block integration; C-03 through C-05 should be fixed before activation.

**Not checked:** real pilot data, providers, multi-host/network-filesystem locking, whole-machine power-loss behavior, or external consumers of the changed `verify` output. Crash evidence comprises fault injection and one abrupt subprocess termination; broader power-loss consequences remain inference.
---

## Follow-up 1: 5987a47c..3547d998

Verified `5987a47c780ee860ecd92d1d3b1ecc953a4c20f7` → `3547d998bf37f816cb581185835e816cfb3e987d` on `round2-tools`. Both source hashes match the builder’s account. Repository files remained unchanged.

The full suite passed: **69 tests, 0 failures, 0 errors, 0 skips**.

**C-01 through C-07**

| Finding | Assessment | Evidence and remaining work |
|---|---|---|
| **C-01** | **Resolved** | The failed-pause and receipt-idempotence tests passed, including owner resume and subsequent triggers. [Trigger enforcement](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:664) derives outstanding facts before enrollment; repeated repair receipts no longer increment counts. |
| **C-02** | **Partly resolved** | Complete-file publication and recovery of entirely missing checksum entries work; the corresponding fault-injection tests passed. However, injecting a **partial checksum append** left complete enrollment JSON that retry and status could not recover: both exited 1. [Recovery](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:546) treats the malformed manifest as fatal. Interrupted recovery also has the new defects below. |
| **C-03** | **Resolved** | Both controlled assignment/withdrawal tests passed. [Round-aware assignment](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:900) shares the lock and rejects withdrawn tasks; the status test confirms conflicting historical records are reported. |
| **C-04** | **Partly resolved** | [The corruption test](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/tests/test_task_profile_round2.py:366) passed: existing checksum mismatches prevent replay and admission. However, recovery can overwrite the evidentiary meaning of an already-recorded hash and certify a changed arm; see C-08. |
| **C-05** | **Resolved** | Torn-tail refusal, malformed-line reporting, exact-byte backup, and bounded tail removal tests passed. [Screening recovery](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:630) preserves earlier bytes. |
| **C-06** | **Resolved** | Independently removed locking in a temporary script copy and ran all four controlled concurrency tests: **4 failures, 0 errors**, exposing admission and assignment/withdrawal violations. The unmodified tests pass. The late-failure test remains. |
| **C-07** | **Resolved** | [Text-output regression test](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/tests/test_task_profile_round2.py:458) passed for both profile and episode lines. Legacy recorder tests also pass; verification retains its documented exit codes. |

For C-02, remaining work is to preserve and recover an interrupted checksum tail without accepting changes to previously established hashes. The current tests inject failure before checksum appending, rather than during it.

**New findings**

- **C-08 — P1: Recovery ignores its own durable hashes and can certify a changed arm.**  
  [Recovery computes current hashes](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:554), appends them to `recovered.jsonl`, then completes the manifest. Subsequent attempts do not compare against earlier recovery entries.

  **Reproduction:** fail checksum publication; let recovery durably record the enrollment’s original hash but fail checksum publication again; change the enrollment arm from Terra to Sol; retry. The command returned **Sol with exit 0**, appended a second recovery entry, and `verify` printed **`enrollments/a: verified`**. The first recovery entry still contained the original hash.

  **Consequence:** an accidental edit during interrupted recovery becomes accepted data despite durable evidence of the original bytes. **Recommend:** bind recovery completion to the previously recorded digest and refuse conflicting bytes. Add this failure-and-edit sequence as a regression test.

- **C-09 — P2: A torn recovery-log append blocks recovery with no supported repair command.**  
  [Manifest completion requires appending to `recovered.jsonl`](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:557). [The appender](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:614) rejects an incomplete tail and requests explicit recovery, but the recovery command handles only `screening.jsonl`.

  **Reproduction:** after an enrollment’s checksum publication failed, inject a partial recovery-intent append followed by an I/O error. Subsequent enrollment retry and status both exited 1 with “explicit recovery required”; `recover-screening` returned `no_recovery_needed`.

  **Consequence:** complete episode records become inaccessible through normal recovery and accounting commands. **Recommend:** provide a bounded, evidence-preserving recovery mechanism for this log, or publish recovery intents atomically. Test interruption of recovery itself.

For the absence claims, I searched the recorder’s `recovered.jsonl` references, recovery functions, and command dispatch. I found no comparison with earlier recovery digests and no recovery-log repair command.

**Verdict: not ready.** C-08 blocks integration; C-02’s remaining recovery gap and C-09 also require remediation.

**Not checked:** real pilot data, providers, multi-host/network-filesystem locking, whole-machine power loss, or external stdout consumers. Additional reproductions used synthetic temporary directories and injected I/O failures.
---

## Follow-up 2: 3547d998..57066b1a

Verified `3547d998bf37f816cb581185835e816cfb3e987d` → `57066b1a907658d325f7b607b9b94519e2c023fd` on `round2-tools`. Source hashes match the builder’s account; the working tree remained unchanged.

Full suite: **79 tests passed, 0 failures, 0 errors, 0 skips**.

**Finding status**

| Finding | Assessment | Evidence |
|---|---|---|
| **C-02** | **Resolved** | Independently tested all **81 incomplete prefixes** of an enrollment checksum line: each recovered, preserved the damaged bytes, and retained the arm. Interrupting recovery after backup publication also recovered successfully on retry. [Manifest repair](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:574) preserves earlier complete entries and rejects established hash mismatches. |
| **C-04** | **Resolved** | [Self-verification](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/task_profile_record.py:510) precedes episode reuse and manifest repair. Tests passed for edits across all episode-record types, missing content hashes, rewritten manifests, and resealed records conflicting with existing manifest hashes. |
| **C-08** | **Resolved** | [The failure-edit-retry regression](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/tests/test_task_profile_round2.py:181) passed: replay and new enrollment exit 1 without an arm or draw, and `verify` fails. The changed record is never re-listed. |
| **C-09** | **Resolved** | [The obsolete torn-log regression](/home/bline/cbm-work/round2-tools/work-engine/metrics/scripts/tests/test_task_profile_round2.py:206) passed: re-listing succeeds while leaving the old damaged log untouched. Searching the recorder found no remaining `recovered.jsonl` references. |

The earlier fixes **still hold**:

- **C-01:** Failed-pause, receipt-idempotence, resume-acknowledgment, and subsequent-trigger tests pass.
- **C-03:** Both controlled assignment/withdrawal races and historical-conflict reporting pass.
- **C-05:** Torn screening refusal, backup preservation, tail recovery, and malformed-line reporting pass.
- **C-06:** Independently removed locking in a temporary copy: **all four controlled concurrency tests failed**, with zero test errors. They pass against the reviewed implementation.
- **C-07:** Profile and episode text-output regression tests pass; verification exit-code tests remain passing.

I checked graph coverage and inspected episode-read call sites, self-hash validation, manifest repair, and command reporting. No recorded coverage gaps affected those files.

**New findings:** None identified in this diff.

**Verdict: ready to integrate.** This verdict covers the tooling; it does not authorize pilot activation.

**Not checked:** whole-machine power loss, network-filesystem behavior, live pilot/provider activity, or pre-v3 record migration, which the packet excludes. Validation used local processes, temporary fixtures, fault injection, and mutation testing.