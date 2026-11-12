# Pi subscription dispatch — completion attempt and budget stop

Date: 2026-11-11 (execution 2026-11-12 UTC). Status: **three findings repaired and tested; final delta review incomplete; no integration**. This record supplements the earlier dispatch result without rewriting it.

## Authority and cost

The owner approved [the completion proposal](../planning/pi-container-subscription-dispatch-review-budget-20261111.md) at `2026-11-12T03:28:12.640Z`: a new **$6 cap/$4 warning** in speed-adjusted API-equivalent subscription proxy USD, with no provider/API requests, skill deployment, commit/push or R-2 integration. Exact approval and proposal digest are in the archive's `authorization-source.json`. The previous objective remains closed. This objective is `744203dd-c0fe-4796-aa6c-93df198a20b8`; reused root/builder/reviewer sessions are bound from this approval window.

**Budget exceeded.** The pre-delta reading was $4.8936312; the next explicit reading reported $6.7296116 against the $6 cap. Root underestimated outstanding cost before admitting the delta review. On observing the crossing, root interrupted the reviewer and collected its exact turn; no new repair, integration or release followed. Latest measured cost before closure: **$7.2696324/$6**, with reporting tail unsettled. These figures are cost proxies, not measured quota or an invoice. No provider API requests were authorized or made in this slice. The failed estimate does not retroactively increase the approved allowance.

## Completed review and repairs

PI-DISPATCH-ADV-01 resumed the original isolated read-only Astra/xhigh/default reviewer, session `01a1cdc3-7e6e-7e52-a08f-5db5a1ba496d`. The exact resumed turn `01a1cdce-82e4-73f2-b6cd-2a76d4178a8c` completed with three P2 findings on the original six-file candidate `9d0aefb557e2ca118412f9ab8a90f4140eb06889`:

1. Native `response.done` completions lost their identity/usage in proxy accounting, and uncertain forwarded requests could disappear from call/control accounting.
2. Checking only the first compressed frame allowed concatenated zstd data to bypass the advertised decoder-window bound.
3. Explicit mounts of the newly consumed host Codex credential file or its ancestor remained possible at launch and retained delivery.

The original Sol6.1/medium/default worker repaired these cases in the isolated worktree. Native completion events now retain response identity/usage, and uncertain subscription requests remain unpriced calls. Compressed requests accept one validated zstd frame with explicit extra-data, declared-size, window and decoded-size checks. Mount guards reserve the resolved host auth file, including current/launch-home retained checks, while allowing nonoverlapping skill paths. Synthetic regression cases exercise the failures and positive controls. These are implementation/evidence statements; the final repaired subject has not received an acceptance verdict.

The worker's initial broad custody records passed but contained zero input hashes despite the result claiming six per gate. Root preserved that discrepancy in `custody-binding-correction.json` and reran the gates with explicit source bindings. The final root run passed **98 transport/container + 21 host + 61 ledger tests**, plus diff-check, with all six input hashes matching current bytes in every gate and no leftover processes. Native Pi/Docker tests used synthetic credentials and a local provider. The focused repair suite passed 11 tests.

## Exact repaired subject and unfinished review

Private checkpoint `5f4839eb7b4f9b60562d2a4622433ac5cc5feae8`, tree `f5790483462885a8bac885b6ef37a8cf46d696b8`, baseline `836aeb7e8f3d25d3bfbf31c8306174b3696e44f2`, parent candidate `9d0aefb557e2ca118412f9ab8a90f4140eb06889`; patch SHA-256 `b0d9111af3ab12591893edaf987c731af4c4dec2981f4dead936fecdb807d17a`; plan `subscription-dispatch-packet-v1`; scope `three-p2-remediated-native-subscription`; final root gate SHA-256 `3dab3485505c57be71bf75b8dfe7d8f35ffa1b784f925db3f950f6ef639e9aeb`.

The same isolated reviewer received this exact delta in turn `01a1cddb-78f3-75a1-b584-a7d67976d924`; actual Astra/xhigh/default/read-only settings were observed again. That turn was **interrupted for the observed budget crossing and returned no final verdict**. The three original findings are repaired by worker evidence but remain unclosed by the reviewer. Do not infer independent acceptance from passing tests or use the original candidate's review as acceptance of this new subject.

Next owner: root or an explicitly adopted successor must collect a completed delta disposition and resolve any remaining material finding under a new authorized allowance before deciding integration. Preserve this retained episode's actual history or attribute a replacement truthfully. Native live subscription use remains unestablished because the prior 64-output-token smoke cap is not enforceable by the transport. No live request or weaker substitute was used. Skill deployment and R-2/Q-B1 integration/distribution remain separately owned.

## Custody

Archive `/home/bline/cbm-work/pi-container-subscription-dispatch-review-20261111/` holds authority/budget, initial completed review, remediation result/source hashes, failed binding claim and corrected root gates, immutable checkpoint, exact interrupted delta turn, actual permissions, and continuation. Worktree `/home/bline/cbm-work/wt-pi-container-subscription-dispatch-20261111` preserves the repaired bytes.

Worker and all test results are collected; reviewer interrupted/retired and exact turn collected; context/budget observer85973 stopped and collected exit143. No owned process or ongoing monitoring remains. Shared tracked source/index were not changed by this attempt; result/proposal records remain uncommitted. No user-visible commit, push, deployment or skill release occurred.
