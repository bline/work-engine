# Formation evidence: same-session context continuity

Recorded 2026-10-28 by Codex session
`01a1839d-27bf-7882-b2ac-0233b11805f1`. The verified local rollout header names
this session, source `vscode`, no parent thread, and contains the current human
request. Repository HEAD at evidence capture was
`2c7102058ec0d49ed4ead68a8e0fb7fd85082b48`; the source table below binds working
bytes separately because other sessions hold uncommitted lifecycle material.

## Human source and scope

The user asked whether the observed agent's self-compaction should be used or
prevented, whether it improves on a handoff file, and what its reduced
transparency teaches. After investigation, the proposed next step was a small
lifecycle-contract clarification, a compact manual-orchestration projection,
and pilot acceptance criteria covering authority, human input, provenance and
recovery. The user's exact response was: "Let's proceed with that."

That authorizes drafting these artifacts and the repository guidance
clarification. It does not decide candidate acceptance, authorize a lifecycle
implementation or live pilot, or request a deployed skill release. The
conversation is an attributed human source; the proposal manifest preserves
the candidate's separate decision boundary.

## Existing contracts and neighboring meaning

`DESIGN.md` is binding doctrine. Its sections 3.5, 3.9, 7, 11 and 12 protect
causal sequencing, admissible evidence, situated context lifetime, operator
authority and truthful provenance. `PHILOSOPHY.md` supplies non-normative
rationale. Wind Walker protects continuation independence without granting
effect authority. The existing root guide scopes approval and adoption to
ownership transfer and explicitly says compaction does not transfer the root.

The lifecycle-manager document owns the proposed semantic contract and already
distinguishes checkpoint safety, actuation, human-source attribution and
reconciliation. The companion transition protocol records accepted design
basis, not implementation authority. Its existing durable operation, lease,
input-custody and recovery boundaries are reused here. Neither document is
edited by this draft. The semantic-durability candidate separately leaves
automatic replacement in human interaction without an established contract.

| Source at capture | SHA-256 |
| --- | --- |
| `DESIGN.md` | `2213ab9a32925e7792353489599201bf71f3268eb8981cc192f87c57e2952095` |
| `skills/wind-walker/SKILL.md` | `bfc453242d01b625ecc96469c39f20afc43d8340bfc51523d4371a973c189005` |
| `skills/manual-orchestration/references/context-boundary.md` before clarification | `3165149e4a130ec1c35a674a0875b27dafa93eb8df16bc96c60e0133b5b5425d` |
| `app-server/docs/semantic-context-lifecycle-manager.md` | `f349c9c4f43c93ec17420c4e7335d1276ba0ef6c9ff8f179cda8e93ec555e660` |
| `app-server/docs/context-lifecycle-transition-protocol.md` | `0f4072ea16c764e9b56e82f10477a7d8add426ad636090fe9f3551d66a1ee8cd` |
| `proposals/agent-state/semantic-durability-context-lifetime/proposal.md` | `f456b960041d6c61214f1ebcdf16b7c926522d5b841c44a27591a770b63e7e02` |

## Direct operational observations

The investigated session was `01a18305-58a4-79c0-b319-7d0cf7ab1e8a`; its local
rollout is under `/home/bline/.codex/sessions/2026/10/28/`, named
`rollout-2026-10-28T08-58-48-01a18305-58a4-79c0-b319-7d0cf7ab1e8a.jsonl`.
At 16:48 UTC it read Wind Walker and inspected generated compaction schemas.
It announced same-session refresh before ending its turn at 17:23 UTC.
Record 1435 at 17:25:03 UTC reports a new window with exact predecessor linkage
and an opaque compaction item; record 1447 contains the runtime-authored wakeup
under role `user`. Later recovery read the snapshot, collected the controller
and restored monitoring. This is observable behavior, not access to hidden
reasoning or proof of complete semantic recovery.

Operational files share this local directory:

```text
/home/bline/.local/state/work-engine/s10-session-quota-repair-20261021/resume-20261022/root-successor-20261028-01a18305/same-session-refresh/
```

| File | SHA-256 |
| --- | --- |
| `context_refresh_controller.py` | `fb22ad4cd57f51e71b116a81090666193ce8010c49846521e845094e760f4eb6` |
| `state-v1.json` | `a1ce11de06210e9db2f00f2f0c5c167c86cbbbe842ceab45c6cb13b43326af07` |
| `resume-wakeup.txt` | `7f063296f669d146614f66059688101415af9973ae1b920fc582ccbe14591523` |
| `controller-result.json` | `3a2decc0b84d376e40601035a4f6a2f218c9a44576d293324a4a12a3e0fae23c` |

These are local forensic locators, not portable packet dependencies or a grant
to redistribute raw traces. The controller's successful result is
actor-attributed. Direct source shows separate input checks and actions, no
durable consumed-attempt check on restart, and delivery through `role_host` as
ordinary turn input. Those observations motivate prospective race and restart
cases; no concurrent-input or duplicate-effect failure was observed in this
single event. The last pre-compaction input was 429,188 tokens, the first
nonzero post-compaction input 28,426, and later rehydration increased it. This
does not establish net cost savings or superiority to handoff.

## Retrieval, inference and limits

Tier 2 verification used graph project `home-bline-code-work-engine`, indexed
generation `2026-10-28T17:53:34Z`, store `u7d40c2096d919f9fg2`, and direct text
observation for prose and operational artifacts. Coverage returned no recorded
issue for the thirteen inspected repository paths, with no remaining page;
that is best-effort metadata, not completeness. Operational artifacts are
outside the graph project and were read directly. No exhaustive code or
runtime-behavior claim is made.

The formation judgment is to keep one independently decidable interaction
contract with a bounded comparison pilot, alongside existing agent-state
proposals. The incident establishes feasibility and useful headroom, not a
permission precedent. Exact host realization, live child preservation and net
benefit remain unestablished. The present contract and changed root guidance
are materially normative agent-facing subjects; pilot sizing is research
procedure. No independent review or acceptance is claimed.
