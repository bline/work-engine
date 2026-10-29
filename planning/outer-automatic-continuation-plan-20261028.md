# Outer automatic continuation: implementation plan

Implement the [required automatic continuation](../records/outer-context-automatic-resumption-requirement-20261028.md): the same authorized work resumes after context replacement without another human message. Recovery is part of that continuation. Preserve the [failure-study evidence and qualifications](../metrics/probes/outer-context-failure-20261028.md).

## First bounded outcome

An isolated agent receives a harmless assignment, prepares durable continuation, starts an owned continuation controller and ends its turn. The controller observes the actual boundary, collects context replacement, and delivers an attributed runtime wakeup. The agent automatically loads recovery guidance, reconciles current direction and unfinished obligations, obtains usable observation, and completes the original assignment exactly once. The external test observer collects evidence but supplies no wakeup, repair or permission signal.

Use the existing outer-host adapter as the placement candidate. `role_host.Codex.deliver` already checks status, resumes the exact thread, verifies its returned identity and invokes `turn/start`; the returned turn ID is recorded in the host ledger. These are mechanics to compose, not proof of a complete controller. The compaction and resumption request outcomes need durable, truthful collection and reconciliation.

The bounded implementation write scope is the existing `skills/manual-orchestration/` host/control adapter, focused tests and conditional continuity instructions, with the existing durable-state/chatboard interface as a dependency. Resolve any necessary change to that dependency explicitly. Extend the existing adapter without introducing a new runtime service or changing the Rust lifecycle/S10 workstream. No skill text may describe automatic capability as installed until its implementation and release establish that fact.

## Protected outcomes

- Automatic recovery continues the standing assignment. A wakeup carries its actual runtime author and operation identity; it needs no fresh human grant and creates no new task authority. Do not introduce a separate, ambiguous controller-permission gate.
- Current pauses, cancellations and unresolved approvals retain their meaning. A pause test must isolate that constraint with the other continuation conditions satisfied.
- Persist enough operation identity and evidence to reconcile uncertain provider entry or delivery. Never blindly replay an uncertain compact or wakeup. Duplicate logical delivery must not duplicate the declared effect.
- Parse usable, session-bound observation after the boundary. Tool exit zero and a compaction marker alone are insufficient. Recovery-only work may establish observation automatically before dependent work.
- Preserve collectors, pending questions and model/effort/service-tier intent; verify actual recovered settings. Listener or delivery failure needs a collected, observable outcome and named owner.

## Qualification and integration

First prove the complete ordinary path. Then exercise intervening pause/cancellation, duplicate wakeup, client loss around send, controller interruption and listener failure against the actual implemented path. Freeze every executable used, including the board script, before entry. Use a non-deduplicating effect counter and assertions derived from native turns and raw effects; supplied boolean predicates cannot substitute for evidence. Keep manual intervention and unsupported cases explicit.

Coordinator owns remaining contract/placement decisions. Terra/medium is the default implementer once those are settled; select a stronger builder only for concrete unresolved decisions left with it. A fresh Opus 5.5/xhigh reviewer can review the implementation and effective instructions, with retained follow-up for ordinary remediation. These are proposed assignments, not launched sessions or new experiment limits.

After focused checks and review, integrate the proven host capability and instructions into the outer skill, perform a bounded installed-version trial, and publish under the applicable commit/deployment authority. Preserve dated evidence throughout.

## Planning evidence

Tier 2 graph verification, project `home-bline-code-work-engine`, generation `2026-10-28T22:40:46Z`, store `ue1e402d4ef799455g2`: exact snippets `role_host.Codex.deliver` lines490–507 and `_start_turn` lines466–473 support the existing same-thread delivery mechanics. Coverage checked in one complete page for `role_host.py`, `codex_app.py`, `root_rotation.py` and `codex_chatboard.py`: content verified with no recorded gaps, best-effort completeness caveat. No claim is made that discovering these primitives establishes the complete control path or arbitrary input ordering.
