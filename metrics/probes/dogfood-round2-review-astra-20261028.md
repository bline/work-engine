# Round 2 design review: gpt-6-astra, high (2026-10-28)

- **Subject:** section "Round 2" of `metrics/dogfood-selection-plan-20261027.md`,
  sha256 `eb21251509280662483ead5c2e84d67c1427a775e684791d653e6b9658137a64`, commit
  `992c8e896dd99269f7cdff200e1eaba2f4b4bec7`; the reviewer verified the hash.
- **Reviewer:** `gpt-6-astra`, reasoning `high`, service tier `default` (observed in the
  rollout's `thread_settings_applied`), a fresh `codex exec` session, read-only sandbox,
  thread `01a183f1-da34-72e1-bbd7-dd99a4b46e3e`. Requested by the owner ("Let's use
  Astra at high reasoning"). 13 calls, $1.86 at the frozen API rates.
- **Packet:** `~/cbm-work/round2-review/packet.md`. It named the subject, context files
  with hashes, four questions and the areas to cover. It did not pass on the author's
  own list of suspected weaknesses.
- **Verdict:** not ready. Two P1 and five P2 findings, one P3.

The reviewer's final message follows unchanged.

---

Verified: the plan’s SHA-256 is `eb21251509280662483ead5c2e84d67c1427a775e684791d653e6b9658137a64`. It and all six supplied context files match commit `992c8e896dd99269f7cdff200e1eaba2f4b4bec7`.

**Findings**

1. **R2-01 — P1: Eligibility is not protected from knowledge of the assignment.**

   **Evidence:** [Plan lines 243–252](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:243) place eligibility after profile freeze and derive the arm deterministically from the revision ID. That ID incorporates coordinator-controlled task identity, packet documents and snapshot identity ([lines 72–75](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:72)).

   **Consequence — inference:** The arm is computable before the eligibility decision. A coordinator can consciously or inadvertently admit work selectively, or vary identities before the first recorded profile. Excluding a second revision of the same task ID does not prevent either route. A correctly recomputed hash would verify allocation arithmetic, not establish unbiased enrollment. I found no evidence that anyone has done this.

   **Recommendation:** Freeze the stable task identity, packet and eligibility decision before revealing an unpredictable assignment. Preserve one assignment through retries and revisions; record every screened task and subsequent withdrawal. Make allocation retrieval idempotent and verify its binding to the enrollment record.

2. **R2-02 — P1: The stop rules can react too late to constrain production exposure.**

   **Evidence:** [Plan lines 260–263](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:260) check stops “after each completed flipped dispatch,” including a P1 on Terra’s first artifact. The round does not define whether completion means initial builder return, first review or final acceptance, nor a shared admission check, in-flight limit or pause/resumption owner.

   **Consequence — inference:** A P1 discovered during review can remain outside the stop check while repairs continue. Other coordinators can admit more flipped work meanwhile. The unchanged review and escalation requirements protect individual acceptance decisions, but the proposed experimental stop does not reliably bound further exposure.

   **Recommendation:** Evaluate the P1 trigger when the finding becomes known. Make pause state visible to every coordinator and check it before admitting another flip. Define in-flight handling, a concurrency bound, and owner-authorized resumption. Pausing the experiment must preserve ownership of repair and collection for work already running.

3. **R2-03 — P2: The experimental unit and attribution through repairs are underspecified.**

   **Evidence:** The cap and outcomes are per “flipped builder dispatch” ([plan lines 240–258](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:240)), while retained continuations create additional assignments on the same revision ([lines 184–185](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:184)). The [model guide lines 71–79](/home/bline/code/work-engine/docs/model-selection.md:71) require Terra escalation for specified findings or a second repair. Outcomes are delegated to existing records without defining their extraction ([plan lines 170–172](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:170)).

   **Consequence — inference:** Coordinators can count continuations differently, count one task several times, or assign Sol’s repair burden to Sol rather than the original Terra assignment. “First-review findings” can also conceal coordinator-requested repairs before the first formal review. Accepted-only repair summaries would favor an arm whose difficult tasks were abandoned.

   **Recommendation:** Define one enrolled task episode as the unit. Keep its original arm through continuations, escalation and packet revisions; report actual model changes separately. Freeze definitions of first artifact, first-review pass, repair round and abandonment. Bind artifact hashes, review receipts, assignment windows and terminal disposition to that episode. Include all enrolled tasks, with pending and missing outcomes explicit. Demonstrate the joins on representative existing records before activation.

4. **R2-04 — P2: Comparing cumulative repair totals confounds performance with arm size.**

   **Evidence:** The stop triggers when Terra’s **total** repair rounds exceed Sol’s by three, once both have four observations ([plan lines 260–262](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:260)). Hash allocation does not ensure equal arm sizes.

   **Consequence — arithmetic counterexamples:** Seven Terra tasks and four Sol tasks, each requiring one repair, trigger the stop despite identical per-task burden. Four Terra tasks requiring two repairs each versus ten Sol tasks requiring one each do not trigger it, despite Terra’s doubled per-task burden. Completion-order imbalance can produce this even with balanced enrollment.

   **Recommendation:** Separate an absolute production-rework budget from comparative monitoring. For the comparison, use a prespecified denominator-aware rule or balanced completed cohorts, retaining pending tasks and escalations. Describe the stop as a safety heuristic, not evidence that one model is inferior.

5. **R2-05 — P2: Activation does not reconcile the deployed coordinator contract with round 2.**

   **Evidence:** [Coordinator line 26](/home/bline/code/work-engine/skills/manual-orchestration/references/coordinator.md:26) stops when `<run>/task-profiles/` reaches the cap. Round 2 counts only flipped builders and explicitly excludes reviewer and unflipped records from that cap. Its activation conditions require approval, a tested `flip`, and Status changes, but no guidance update ([plan lines 267–268](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:267)). Existing guidance has no flip step. Profile failures can return `task_revision: null` ([script lines 176–200](/home/bline/code/work-engine/metrics/scripts/task_profile_record.py:176)), leaving the proposed hash-based assignment path undefined.

   **Consequence — inference:** Following the deployed guidance can stop collection well before 20 flipped tasks. Coordinators must improvise how to invoke allocation, count it, and handle instrumentation failures.

   **Recommendation:** Make activation conditional on aligned coordinator guidance and tested round-specific admission/counting semantics. Explicitly supersede the passive phase-0 selection rule for eligible round-2 tasks. Define profile/flip failure behavior that preserves ordinary production progress while recording whether enrollment occurred.

6. **R2-06 — P2: Builder eligibility does not establish compatibility with the fixed review route.**

   **Evidence:** Eligibility checks whether both builders at medium are acceptable, but then declares a separate Astra/high reviewer “unchanged” ([plan lines 243–255](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:243)). The [model guide line 45](/home/bline/code/work-engine/docs/model-selection.md:45) uses xhigh for architecture, ownership contracts and unresolved-design subjects; [lines 8–11](/home/bline/code/work-engine/docs/model-selection.md:8) preserve owning campaign/provider contracts.

   **Consequence — inference:** A task whose builder choice is flexible can still require a different reviewer effort or provider. Calling Astra/high unchanged does not establish that it preserves the task’s contract. Escalation can also change the review needs during the episode.

   **Recommendation:** Require compatibility with the proposed review route at enrollment. Otherwise leave the task unflipped. Preserve required review escalation during execution and record it as an outcome. The guide’s production requirements should remain explicit precedence boundaries.

7. **R2-07 — P2: The proposed cost and tier measurements overstate what their sources establish.**

   **Evidence:** Round 2 requests “quota-weighted” cost and takes tier from `--harness` ([plan lines 256–259](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:256)). The script stores that field under `requested`; attribution checks model and effort, not observed tier ([script lines 286–348](/home/bline/code/work-engine/metrics/scripts/task_profile_record.py:286)). The [model guide lines 95–96](/home/bline/code/work-engine/docs/model-selection.md:95) explicitly says API prices do not establish subscription quota for Terra or Sol 6.1. The [Fast probe lines 53–56](/home/bline/code/work-engine/metrics/probes/codex-fast-tier-speed-20261028.md:53) also labels its quota multiplier documented, not measured.

   **Consequence — inference:** Reported quota savings could be estimates presented as observations. Tier changes, harness versions and concurrent Fast traffic can contaminate wall-time comparisons. Builder-and-repair cost alone also omits review and coordination costs that the guide identifies as material.

   **Recommendation:** Bind observed tier per assignment and retained turn, following [Codex-host lines 34–41](/home/bline/code/work-engine/skills/manual-orchestration/references/codex-host.md:34). Label modeled costs as API-equivalent estimates, with explicit multiplier assumptions. Report total episode cost where available and separate unavailable components. Keep comparable harness/tier conditions together and qualify latency for concurrency and waiting.

8. **R2-08 — P3: Profile hygiene does not address reviewer knowledge of the assigned model.**

   **Evidence:** The [coordinator rules lines 27–30](/home/bline/code/work-engine/skills/manual-orchestration/references/coordinator.md:27) prohibit pilot/profile disclosure. The automatic check scans packet text for profile-related tokens ([script lines 278–281](/home/bline/code/work-engine/metrics/scripts/task_profile_record.py:278)); it does not establish model masking or inspect assembled context. Round 1 documented actual packet and exploratory-read leakage ([review line 15](/home/bline/code/work-engine/metrics/probes/dogfood-pilot-round1-review-20261028.md:15)). Round 2 does not specify reviewer masking.

   **Consequence — inference:** Reviewers can know the builder identity through provenance or surrounding artifacts and apply different scrutiny. Because review findings are the primary outcome, this is a measurement-bias risk even with identical reviewer models.

   **Recommendation:** Omit unnecessary arm identity and comparison expectations from reviewer inputs while preserving required provenance elsewhere. Explicitly carry forward assembled-prompt and relevant exposure audits. Record unavoidable unblinding and disclose it in the reading.

**Verdict: not ready.**

The proposal is a reasonable direction for collecting production evidence, and preserving review, acceptance and escalation is appropriate. The allocation and stop defects block approval; the P2 items should be resolved before launch.

Twenty dispatches can support a descriptive report about the **coordinator-selected eligible subset**. They cannot establish that Terra passes “as often” or needs “as few repairs” in an equivalence sense. Illustratively, even zero failures in ten independent, identically distributed Terra tasks leaves a one-sided 95% upper failure-rate bound of about 26%; related tasks and early stopping further complicate interpretation. Report uncertainty, enrollment/exclusion counts, task clustering and stopping history. The existing “cannot rank the models” qualification should remain.

**Not checked**

- No execution, mutations, agents, provider calls, commits or pushes were performed. The proposed `flip` implementation and runtime enforcement remain unverified.
- I did not inspect sealed materials, `~/cbm-work/routing-data-audit/`, raw round-1 records, private run registries, usage ledgers or deployed skill copies. Round-1 observations above are attributed to its supplied review.
- Specification-absence claims are bounded to the full plan and the coordinator, Codex-host and measurement references. I also inspected the record script and searched those sources for allocation, eligibility, masking, repair, continuation, pause, cap and escalation provisions. These are not repository-wide absence claims.
- Graph coverage reported no recorded gaps for relied-on repository files; exact source reads and commit-byte comparisons supplied the substantive evidence. Clean graph coverage is not proof of completeness.
---

## Follow-up review of revision 2 (same session, 2026-10-28 19:37–19:41Z)

Subject: revision 2, plan sha256 `0ea91c7f05ab5f21b0dc2688c07f0f34bcb6f3f9b70d2ad23b7c0a45fedacd6f` at commit `f91fb226`. `codex exec resume` of thread `01a183f1-da34`, `gpt-6-astra` at `high`, standard tier; 6 calls, $1.10 at the frozen API rates. Verdict: **not ready** (R2-09 is P1). The final message follows unchanged.

Verified: both the working file and commit `f91fb2265f6d823ee9ea738aa2ed3e04222cd11b` have SHA-256 `0ea91c7f05ab5f21b0dc2688c07f0f34bcb6f3f9b70d2ad23b7c0a45fedacd6f`. The six original context files are unchanged.

**1. Disposition of previous findings**

“Resolved” below means resolved in the proposed design; implementation and activation remain unverified.

| ID | Disposition | Evidence and remaining issue |
|---|---|---|
| **R2-01** | **Partly resolved** | [Lines 280–288](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:280) replace predictable hashing with a random draw after recorded screening, preserve existing assignments and retain withdrawals. However, the blanket enrollment-failure rule can discard an assignment already recorded; see **R2-09**. |
| **R2-02** | **Partly resolved** | [Lines 292–305](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:292) establish shared pause state, immediate P1 handling, owner-controlled resumption, bounded Terra concurrency and continued ownership of existing work. The repair-budget trigger still lacks a specified source of current, open-episode repair counts; see **R2-11**. |
| **R2-03** | **Partly resolved** | [Lines 247–264](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:247) establish episode-level attribution, explicit outcomes and separate pre-review fixes. [Lines 336–342](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:336) require evidence joins and preservation of missing outcomes. Repair-round semantics and the first-review denominator remain problematic; see **R2-10** and **R2-12**. |
| **R2-04** | **Resolved** | [Lines 299–309](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:299) replace unequal-arm cumulative comparison with an absolute rework threshold and descriptive rates. The threshold’s implementation contract has the separate issue in **R2-11**. |
| **R2-05** | **Partly resolved** | [Lines 356–368](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:356) explicitly require tested commands, released guidance, episode-based counting and demonstrated joins before approval. The proposed failure fallback is unsafe for partially completed enrollment; see **R2-09**. |
| **R2-06** | **Resolved** | [Lines 273–278](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:273) require review-contract compatibility; [lines 313–314](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:313) preserve review escalation. A newly introduced builder-escalation misstatement is covered by **R2-10**. |
| **R2-07** | **Resolved** | [Lines 324–330](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:324) require observed tier, explicitly estimated/modelled costs, review costs and disclosure of missing components. The newly specified wall-time method has a separate defect; see **R2-13**. |
| **R2-08** | **Resolved** | [Lines 315–320](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:315) withhold unnecessary builder identity, extend the packet check and require disclosure of unblinding. Actual exposure control remains an execution check. |

**2. New findings**

**R2-09 — P1: An enrollment command failure is incorrectly treated as proof that enrollment did not occur.**

- **Evidence:** [Lines 281–286](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:281) draw and record the assignment. But [lines 362–364](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:362) say that if `enroll` fails, the task is recorded as “not enrolled” and dispatched as usual.
- **Consequence — inference:** A failure after recording the arm—for example, while returning the result—can cause an enrolled task to be relabelled unassigned and dispatched under discretionary selection. That breaks original-arm attribution and can miscount the cap and in-flight population. A command failure does not establish which durable effects occurred.
- **Recommendation:** Distinguish confirmed pre-enrollment failure from committed enrollment and uncertain completion. Reconcile uncertain results through the stable task identity; preserve any recorded arm and never redraw or erase it. Specify recovery tests at these failure boundaries, including concurrent retries. Ordinary production may continue under authorized recovery, but the experimental history must remain truthful.

**R2-10 — P2: “Repair round” counts interaction turns, and the escalation wording delays the guide’s required switch.**

- **Evidence:** [Lines 258–261](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:258) define a repair round as “each builder turn” and describe escalation “after a second repair round.” The unchanged [model guide lines 71–75](/home/bline/code/work-engine/docs/model-selection.md:71) requires switching when “a second repair round is needed.”
- **Consequence — inference:** One remediation cycle split across continuations can count as several repairs, making the metric sensitive to orchestration rather than defects. The “after” wording also permits Terra to perform a second repair that the guide assigns to Sol.
- **Recommendation:** Define a repair round around a review-driven remediation cycle and its resubmission, grouping continuation turns within that cycle. Escalate when the second round becomes necessary, before executing it. Preserve all other guide triggers, including newly discovered hazard-class work.

**R2-11 — P2: The rework threshold depends on live counts, but the specified durable count is written only at closure.**

- **Evidence:** [Lines 296–300](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:296) require action when ten repair rounds have been used and name `close` and `enroll` as the checking commands. [Lines 336–338](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:336) specify writing repair counts in the terminal `close` record. Within the reviewed plan, no update or authoritative read of open-episode repair progress is specified.
- **Consequence — inference:** The threshold can be crossed by ongoing work while subsequent admissions see only closed-episode totals. Limiting Terra to two open episodes does not establish a current cumulative repair count.
- **Recommendation:** Define how completed repair cycles become available to the shared budget while episodes remain open, and who triggers the pause. Ensure admission checks use that current state. Explicitly describe ten rounds as a **new-enrollment pause threshold**, since already admitted work is allowed to continue beyond it.

**R2-12 — P2: “First-review passes per closed episode” conflates review results with withdrawal and closure timing.**

- **Evidence:** [Lines 263–264](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:263) include withdrawal before dispatch as a terminal disposition. [Lines 306–308](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:306) specify first-review passes per closed episode.
- **Consequence — inference:** One reviewed pass and one pre-dispatch withdrawal produce a 50% figure although the only completed first review passed. Conversely, an open episode with a known first-review failure is omitted until closure. Thus the reported rate can change because of administration or repair duration rather than first-artifact quality.
- **Recommendation:** Report first-review outcomes when that review completes, independently of episode closure. Separate the pass rate among reviewed episodes from any enrollment-level success measure. Report withdrawn, never-reviewed and pending episodes explicitly; keep all enrollments in the overall accounting.

**R2-13 — P2: Completed-turn durations do not establish episode wall time.**

- **Evidence:** [Lines 331–332](/home/bline/code/work-engine/metrics/dogfood-selection-plan-20261027.md:331) derive “wall time” from ledger turn durations. The ledger describes these as durations of completed Codex turns ([usage_ledger.py lines 52–57](/home/bline/code/work-engine/metrics/scripts/usage_ledger.py:52)).
- **Consequence — inference:** Summing turn durations omits gaps between turns and can double-count overlapping work. Qualifying the number for waiting and concurrency does not make it elapsed time from dispatch to disposition.
- **Recommendation:** Define episode wall time using explicit start and terminal timestamps. Report accumulated turn duration separately. Keep elapsed-to-date measurements for pending episodes distinct from completed-episode durations.

**3. Verdict: not ready.**

Revision 2 resolves the main conceptual problems with predictable allocation, unequal-arm stop comparisons, review compatibility and model masking. **R2-09 still blocks approval**; the P2 findings should be corrected before the round starts. The listed implementation, release and join demonstrations remain legitimate activation prerequisites, not evidence that they have already passed.

**4. Not checked**

- No files were changed, agents launched, providers called, tests executed, or commits/pushes performed.
- I did not inspect sealed material, the excluded routing-data directory, private episode records, deployed guidance or actual enrollment behavior.
- The open-progress specification absence in R2-11 is bounded to the complete revised plan, its diff and retained unchanged context. I searched the plan for enrollment, failure, repair, closure, pause, concurrency and measurement provisions; it is not a repository-wide absence claim.
- I checked the ledger’s tier and duration representation, not its runtime completeness. Graph coverage reported no recorded gaps for the inspected paths; that does not establish implementation correctness.
- This was a retained follow-up review, not a new independent review episode.