# Re-review: outer-context-continuity subject-v2

**Subject:** `subject-v2.json` (SHA256 `aa9f3796…45bf`), checked against `delta-v2.patch` and `remediation-v2.json`. This is a continuation by the same retained reviewer, so it makes no new freshness or independence claim. I didn't recompute file digests.

**Applicability:** applicable. The changed prose is normative text agents load, and the changed reader backs it.

**Verdict: `unresolved`, for one reason only.** The v2 `SKILL.md` and `coordinator.md` add a load trigger for a file this subject doesn't include (OCT-OPUS-10).

Within the outer-context-continuity scope, the result is **`acceptable_as_is`**:
- all nine earlier findings are resolved;
- two low items are recommended but don't block (OCT-OPUS-11, OCT-OPUS-12).

## Earlier findings

| ID | Status | Basis |
|---|---|---|
| OCT-OPUS-1 | **Resolved** | context-boundary.md:207–219 (v2). The manual successor instruction now carries the saved model, effort and tier, including the intended Fast state. The successor's first actions now include checking its own applied settings. One leftover for non-Codex roots is filed as OCT-OPUS-12. |
| OCT-OPUS-2 | **Resolved** | codex-host.md:73–79 and same-session-context.md:86–90 (v2). "Last-known configuration" is now separate from "preservation observed"; with no new record, preservation is reported as unestablished, and "do not let" became "detect and surface". Saving the reading before replacement is required at same-session-context.md:50–52 and codex-host.md:45–46. Two pieces of evidence show the check is practical: (1) the probe has applied records at thread start, at each turn and at a retained delivery (probe JSON:186–228); (2) fixture A has a later, distinct record in the same rollout (line 12 → line 59, different digest; n=1). I can't see from those files alone that line 59 comes after the compaction. |
| OCT-OPUS-3 | **Resolved** | root_rotation.py:360–361: `commit` passes `self.a.host_arg` to `launch`. Line 613 defines `--host-arg` as repeatable, and `watch` calls `commit` (580). I also checked line 655: `--detach` restarts the script with the original arguments minus `--detach`, so the settings arguments reach the detached launch. The text now puts the flags on `commit`/`watch` (context-boundary.md:202–206). |
| OCT-OPUS-4 | **Resolved** | same-session-context.md:50 (v2) now names the skill and role references needed for recovery. This matches Wind Walker (SKILL.md:14–15, 22–25): anything future work depends on needs a durable reference. |
| OCT-OPUS-5 | **Resolved** | role_host.py:95–110, 114 (v2) check every record's shape and return exit 32. Four new tests cover this and the CLI exit codes, for 8 in total. A related availability question is filed as OCT-OPUS-11. |
| OCT-OPUS-6 | **Resolved** as scoped | codex-host.md:68 names Codex 0.164.0, and role_host.py:129 reports `recorded_cli_version` and `tier_mapping_basis`. Optional leftover: when the recorded version is missing or isn't 0.164.0, the output still says `launch_arguments_available`. |
| OCT-OPUS-7 | **Resolved** | codex-host.md:60–64 (v2). Keeping the observed root setting, keeping a known user override, and not switching to a generic role default together make the order of precedence clear. Keeping a non-Fast root only preserves how its builders were already running, and any real conflict stays with its owner. |
| OCT-OPUS-8 | **Resolved** | context-boundary.md:35–37 and same-session-context.md:62–66 (v2) now agree. Existing human authority governs, ordinary continuation implies no fresh approval, and a wakeup still can't approve, widen or reactivate work. The author is kept separate from the delivering actor in the detailed reference; the always-loaded copy says the transport may present it as user input. |
| OCT-OPUS-9 | **Resolved** | same-session-context.md:76–77 (v2) now says "this session". The remaining "root" wording (lines 10, 13, 27) is about root transfer and is correct there. |

**Suggestions:**
- **S3, S4, S5:** done.
- **S2:** declined, with a stated reason. That's the author's call.
- **S1** ("supported route" still unnamed): left as written. That's reasonable while the Fast pair is still running.

## New findings

**OCT-OPUS-10 (medium; scope and identity, not content): unreviewed loaded reference inside the subject.**
- **Where:** SKILL.md:14 and coordinator.md:20–21 (v2).
- **Observed:**
  - Both files add a trigger to load `references/organization-formation.md`.
  - That file exists in the live repository but isn't in subject-v2, and `remediation-v2.json` doesn't mention it.
  - The SKILL.md entry is the first bullet under "Load by role" and applies to any role.
- **Consequence:** accepting subject-v2 would look as if it covered a new loaded instruction that nobody in this review read.
- **Advice:** record those two hunks as out of scope, with their own owner and review, or take them out of this subject. I'm not judging the content.
- **Confidence:** high.

**OCT-OPUS-11 (low): the v2 reader hasn't been shown working on a real rollout.**
- **Where:** role_host.py:104–106.
- **Observed:**
  - All four fixture outputs lack `recorded_cli_version` and `tier_mapping_basis`, which v2 always emits (line 129). So the v1 reader produced them.
  - They do show that the v1 matching path works on real 0.164 rollouts at the default tier (3 sessions).
  - v2 now fails the whole read if any record's payload isn't an object. That includes `"payload": null`, which v1 tolerated with `or {}`.
- **Inferred:** a valid record of some other type with a null payload can't hide a later settings change. Rejecting it protects nothing and would make an ordinary rollout unreadable. I can't tell whether real 0.164 rollouts contain such records.
- **Advice:** run v2 `settings` once on the A/B rollouts. The Fast pair would also serve if it uses v2. If the output is correct, keep the code as it is.

**OCT-OPUS-12 (low): the manual-route settings clause isn't limited to Codex.**
- **Where:** context-boundary.md:209–216 (v2).
- **Problem:** the Roots procedure covers Claude roots too, but only a Codex root has a saved reading (step 3 says "For Codex"; the reader is Codex-only). A Claude predecessor would be told to pass on "saved" settings it doesn't have. Its successor would be told to check applied settings with no reader to do it.
- **Consequence:** this invites exactly the kind of unverified value the reader was built to avoid.
- **Advice:** say "for Codex"; otherwise give known settings with their source, or mark them unestablished.

## What the fixtures show (bounded)
- **Fixture B:** the predecessor (`01a1841a…`, line 12) and the successor (`01a18420…`, its own rollout, line 21) have the same gpt-6.1-sol / medium / default settings. The successor's record falls after launch (20:08:18Z) and before adoption (20:08:40Z).
- **Adoption board:** the `adoption` message ties the role, the predecessor session, the successor session and the handoff digest `a849303c…` together consistently.
- **Limits:** both pairs are at the default tier (n=1 each). A default-to-default match can't tell whether the explicit launch arguments were honoured or the shared default just happened to be `default`. The Fast pair is the case that decides it, and I'm not claiming its result.

## Not established, and limits of this pass
These lifecycle profiles are still unqualified:
- automatic self-resumption;
- ordering when a human sends input during a replacement;
- compaction the session invokes on itself;
- settings handling for Claude roots;
- the Fast pair.

The subject claims none of them, and doesn't claim a deployed automatic root lifecycle or a global skill release.

The real `session_meta.cli_version` field name isn't shown in the probe or the fixtures. I read only:
- the delta and the disposition;
- codex-host.md:43–86 and role_host.py:100–133 (v2);
- a search over same-session-context.md;
- the SKILL.md list;
- root_rotation.py:325–369 and 598–635, plus a search that found lines 580 and 655;
- Wind Walker;
- the probe;
- the five named fixture files.

I did not read `organization-formation.md` or the rest of `root_rotation.py`, including `prepare`.

This is advisory only. Acceptance and deployment stay with the coordinator and the decision owner, and this session stays available for any further bounded delta.