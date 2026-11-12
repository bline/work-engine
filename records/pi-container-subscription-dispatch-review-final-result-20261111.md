# Pi subscription dispatch — completed review and bounded acceptance

Date: 2026-11-11 (review completed 2026-11-12 UTC). Status: **accepted exact repaired implementation; isolated and uncommitted**. This record supersedes the incomplete review standing in the earlier completion-attempt record; its history and budget overrun remain unchanged.

## Authority and allowance

The owner requested: “please extend the cap and finish the review”, at `2026-11-12T04:26:39.820Z`. Root selected a bounded **additional $4 proxy cap/$3 warning** for finishing the retained review and closure. This was a new review-only allowance, not retroactive approval of the prior overrun. Exact attribution lives in `authorization-source.json` and `budget.json` in the archive below. No new implementation, integration, provider/API request, commit, deployment or release was included.

The same isolated PI-DISPATCH-ADV-01 reviewer resumed its accumulated work. Its recorded input was 162,138 tokens; root recorded 181,540 and estimated 20,000/18,000 additional tokens respectively, within the observed band. No reviewer replacement or new freshness claim was made.

## Completed verdict and root acceptance

Retained Astra session `01a1cdc3-7e6e-7e52-a08f-5db5a1ba496d`, exact final turn `01a1ce03-c2e8-7fe2-9b14-80fe5a9371a2`, returned **accept**, bounded to the exact repaired subject. Requested and recorded settings were `gpt-6-astra`, xhigh, default tier, read-only filesystem and restricted network. Root independently read the realized grant/settings and collected the completed exact turn and full final report.

All three original P2 findings are **resolved**:

1. Native `response.done` completions retain identity/usage; uncertain subscription requests and upstream failures remain unpriced calls rather than disappearing from accounting.
2. Compressed requests use a validated bounded single zstd frame, explicitly refusing extra data. Regression cases reject concatenated and leading/trailing skippable frames, including the reproduced window bypass.
3. Resolved host auth paths are reserved on initial launch and retained validation, including current/launch homes and symlink aliases, while nonoverlapping skill mounts remain permitted.

The reviewer found no new material defect in the remediation delta. Root accepts this exact implementation as reviewed and ready for a separately scoped integration decision. This does not establish live-provider success, actual quota, distribution clearance or general resource safety outside the exercised contract.

## Exact subject and evidence

Baseline `836aeb7e8f3d25d3bfbf31c8306174b3696e44f2`; repaired private checkpoint `5f4839eb7b4f9b60562d2a4622433ac5cc5feae8`; tree `f5790483462885a8bac885b6ef37a8cf46d696b8`; task-patch SHA-256 `b0d9111af3ab12591893edaf987c731af4c4dec2981f4dead936fecdb807d17a`; plan `subscription-dispatch-packet-v1`; scope `three-p2-remediated-native-subscription`; gate SHA-256 `3dab3485505c57be71bf75b8dfe7d8f35ffa1b784f925db3f950f6ef639e9aeb`.

Final root qualification reports **180 tests plus diff-check passing**, with no leftovers. All six current file hashes match the private checkpoint and source manifest; all six inputs in every root gate match. The final reviewer verified those bindings and retained the original failed review and interrupted turns as history. The earlier worker broad run with missing input hashes was not used for exact-source qualification. No unchanged suite was rerun during this review-only continuation.

Live subscription-provider dispatch and actual quota remain **unestablished**. The transport cannot enforce the previously required 64-output-token live-smoke cap; synthetic native Pi/Docker tests are bounded evidence of another claim. No live provider request or waived cap was substituted. The R-2/Q-B1 integration/distribution hold and skill-release ownership remain separate.

## Cost and custody

Latest measured additional allowance spend before closure: **$1.2025844/$4** in speed-adjusted API-equivalent subscription proxy USD, with reporting tail unsettled. It is neither measured subscription quota nor a provider invoice. The prior $6 allowance's recorded overrun remains in `records/pi-container-subscription-dispatch-review-completion-result-20261111.md` and is not reassigned to this window.

Archive `/home/bline/cbm-work/pi-container-subscription-dispatch-review-final-20261111/` owns authority/budget, exact final review, actual grant/settings, source verification, root acceptance and closure. Earlier review/repair/gate evidence remains in `/home/bline/cbm-work/pi-container-subscription-dispatch-review-20261111/`. Worktree `/home/bline/cbm-work/wt-pi-container-subscription-dispatch-20261111` preserves the accepted bytes; private checkpoint above preserves reachability.

The reviewer completed and was retired, and collector34914 completed exit0 and was collected. No owned process or ongoing monitoring remains. No shared code integration, user-visible commit, push, deployment or skill release was performed. Result and proposal records remain uncommitted. The next action is local integration of the accepted exact tooling subject under the applicable owner authorization; this final verdict itself grants no such authority.
