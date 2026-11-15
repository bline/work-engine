**Verdict: `changes_required` — one P2 finding.**

Reviewed [subject-r1.md](/home/bline/cbm-work/unified-interface-plan-review-20261113/subject-r1.md), SHA-256 `ed15f5c0a74f72c3f4de3f824686a884ba6966e60ba23566a70b902b75cff6f0`. All ten frozen context files match `source-manifest.json`.

The direction is sound: existing owners retain authority, browser and terminal share semantics, unknowns remain visible, and operational visibility precedes advanced economics. U1 is a useful bounded investigation. The following qualification gap needs correction before accepting the broader plan.

**Finding**

**UI-R1 · P2 — Refresh evidence can pass while admitted operational state remains stale.**  
Subject: [U3, line 79](/home/bline/cbm-work/unified-interface-plan-review-20261113/subject-r1.md:79), and qualification target at lines 88–93; source-map requirements at lines 62–66.

U3 requires demonstrating “a real source change,” but does not require establishing refresh coverage for each admitted semantic owner. The cited Events subscription mechanism resolves selected **sessions**; its replay mechanism replays that subscription’s retained events. These mechanisms do not, by themselves, establish notification coverage for independently changing task, review, limit or scheduler records.

Consequently, an implementation could pass the listed session-event, reconnect and replay checks while retaining an obsolete waiting state after an owner-only change. A scheduled item could also become due without any new transcript entry or record mutation. Connection health and displayed source age expose useful limitations, but do not establish the claimed timely operational updates.

**Bounded remedy:** Extend U1’s source map to identify each admitted owner’s refresh coverage and limitations. Require U3 qualification to exercise an owner change without transcript activity and a time-dependent attention transition, plus recovery after missed notifications. Bind the proposed five-second target to explicitly named sources, transitions and measurement endpoints. Notifications, reconciliation reads and clock-driven reevaluation remain mechanism choices; unsupported coverage must remain an unmet condition.

This follows the accepted design’s operational attention requirements and its requirement that projection cutoff and completeness remain visible ([frozen design, lines 158–173 and 334–336](/home/bline/cbm-work/unified-interface-plan-review-20261113/inputs/app-server/docs/control-plane-causal-observability-ui.md:158)).

**Optional advice**

Clarify the [U2 early-landing exception](/home/bline/cbm-work/unified-interface-plan-review-20261113/subject-r1.md:82): controlled-host qualification can precede unresolved production qualification, while still depending on settled identity, access and projection contracts. This would reconcile that exception with the client-builder responsibility at line 103.

**Limits and unresolved questions**

This was a plan review, not acceptance of the prototype, owner sources, UI-0/UI-1, or deployment. I read the required doctrine and frozen minimum set. The roadmap’s October cutoff was treated as historical. The accepted-pin mismatch and existing subscription/replay mechanisms were checked against frozen source; no runtime qualification was performed. Five seconds remains a proposed qualification target, not demonstrated behavior.

Graph coverage was checked at generation `2026-11-13T06:06:15Z`; corresponding live context files matched the archive. Relevant unresolved callsites were inspected directly. This is not exhaustive graph or implementation coverage.

Still unresolved: exact owner-backed joins, per-owner refresh coverage, remote authentication/delivery qualification, and deployment disposition. Their disclosed future status does not prevent U1 investigation. No files were changed, providers invoked, tests/builds run, or live publications made.

**Identity binding acknowledged**

- Session: `01a1d38f-4df6-7f12-aa5b-43a038789c42`; turn: `01a1d38f-4e73-7733-9d1a-3c6030b8edce`.
- Rollout: `/home/bline/.codex/sessions/2026/11/12/rollout-2026-11-12T23-19-06-01a1d38f-4df6-7f12-aa5b-43a038789c42.jsonl`.
- Parent/collection owner: `01a1d319-1e58-7883-90af-992d225517d3`; fresh top-level thread, with ledger collection binding rather than inherited runtime ancestry.
- Requested and recorded settings: `gpt-6-astra`, `xhigh`, `default`, read-only. Session metadata and the bound applied-record digest were verified; backend fulfillment/billing is not established.
- Context binary: `/home/bline/.local/state/work-engine/tools/context-status/d16c5f3799e04da374adc5595d9643354d64457ebca3cf3468cf4566824f1204/work-engine-context-status`; SHA-256 `d16c5f3799e04da374adc5595d9643354d64457ebca3cf3468cf4566824f1204`.
- Output/continuation route: this final response; root serializes it in the review archive and owns exact-thread collection.

**Resumable continuation:** r1 identity above; UI-R1/P2 remains open. Next obligation is a bounded plan amendment covering per-owner refresh qualification, followed by delta review. Same-session delta review is appropriate if the subject’s scope and ownership premises remain unchanged. Context checks observed 89,613 then 104,074 input tokens, with no compaction; retain this reviewer. No owned processes remain. Implementation, integration and product acceptance decisions remain with the root and applicable owners.
