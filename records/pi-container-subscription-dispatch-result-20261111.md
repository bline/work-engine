# Pi subscription-container dispatch — tested candidate, review incomplete

Date: 2026-11-11 (completion 2026-11-12 UTC). Status: implementation preserved privately, **not accepted or integrated**. The owner explicitly requested a parallel worker; this work used the existing shared $8 subscription proxy allowance and no additional provider API allowance.

## Implementation subject

Baseline `836aeb7e8f3d25d3bfbf31c8306174b3696e44f2`; private checkpoint `9d0aefb557e2ca118412f9ab8a90f4140eb06889`, tree `3a58ac73209f9d9696a25b34820f5b36b8f4852e`; patch SHA-256 `1f21617d4aca156e0b0deebc581bfdc660f84e419883d361617c5ba98c4f4aba`; plan `subscription-dispatch-packet-v1`; scope `host-only-codex-credentials-native-sse`; gate SHA-256 `f6165cf73fd4634f5dc6b4e4ed3f9abc55daaf48083825e3befe029482fc3e6e`.

Worker `/root/pi_subscription_dispatch`, session `01a1cdb3-c6ad-7ea3-9666-ab83f1e914b6`, requested/observed Sol6.1 medium/default, completed six isolated paths: `role_host.py`, `pi_container.py`, `metrics/scripts/usage_ledger.py`, conditional `pi-host.md`, existing `test_pi_container.py` and new `test_pi_subscription.py`. Exact hashes and artifact locations are in the archive below.

The candidate adds the native `openai-codex` subscription SSE container lane. Host credentials remain in host memory/runner stdin; the container receives synthetic credentials. The proxy rebuilds provider headers, applies pinned model/field/tool/store and request bounds, and bounds compressed/decoded request size. Unsupported subscription output-token, dollar and tier settings are explicitly refused. Billing/quota are unknown; the ledger attributes subscription requests to the Codex pool rather than paid API spend. These implementation claims remain subject to the unfinished independent review.

## Evidence and incomplete review

Root reran final bytes across 95 transport/container, 21 host and 61 ledger tests, plus whitespace checking: all pass, all six source hashes match every gate, no leftover processes. Focused native Pi/Docker tests use synthetic host credentials and a local provider; two turn containers were removed and real credential material was not supplied. Earlier helper errors and an environment-fingerprint refusal caused by source changes during a test are preserved separately; final stable-byte evidence supersedes only the exercised claims.

PI-DISPATCH-ADV-01 launched fresh read-only Astra/xhigh/default session `01a1cdc3-7e6e-7e52-a08f-5db5a1ba496d`, exact turn `01a1cdc3-8231-7821-beea-e60563d5b5fb`. Actual settings and realized read-only grant were verified. Root interrupted and collected this review to reserve closure within the $8 shared proxy cap. **It returned no verdict.** Worker completion and passing tests do not substitute for the required separate review. Root did not accept, integrate, commit or release these changes.

No real subscription-provider request was made. Live use remains **unestablished** because the native transport cannot enforce the separately authorized smoke test's 64-output-token cap. A synthetic end-to-end test is a different claim and does not waive that boundary. Existing host `zstandard` is required; missing dependency refuses launch. Actual quota/billing and broad credential/security correctness remain review concerns.

## Continuation

Archive `/home/bline/cbm-work/pi-container-subscription-dispatch-20261111/` owns `RESULT.md`, assignment/continuation, final hashes, root gate records, checkpoint receipt, review binding/exact interrupted turn and budget-stop evidence. Isolated worktree: `/home/bline/cbm-work/wt-pi-container-subscription-dispatch-20261111`.

Next owner: root or explicitly adopted successor must finish the retained review episode (or truthfully record reviewer replacement), repair any material findings, revalidate affected bytes and decide integration under a new approved allowance. Skill deployment is separately owned and was not performed. Do not infer live readiness, zero cost or acceptance from this preserved candidate. The worker and reviewer are collected/stopped, with no owned build/container process remaining.

Shared budget/cost and the independently accepted private diff replacement are recorded in `records/research-lifecycle-m2-blind-diff-result-20261111.md`. Shared source/index and HEAD `836aeb7e` remain unchanged; new result records are uncommitted.
