# Second-tier review: same-session context continuity (subject `7370f619…`)

**Verdict: `changes_required`.** The candidate's core separation is sound: compaction is not root transfer, and both actuation and resumption must be covered by current authority. Four distinctions are missing from the contract text itself before acceptance. The rest of the Opus findings are realization evidence obligations, pilot design, or wording. No owner decision blocks progress.

**Durable state:** episode begun at revision `c6fa0f9d…`, initial result recorded with transition `fable-initial-result-1` at revision **`23102c398b990a6201c2ad0ea116e1fed09cd42b`**, ref `refs/work-engine/independent-review-state/2443c66a…`. Read-back confirms 12 open findings, status `active`, `pending_next_action: await_remediation`. Publication complete.

**Specialist applicability (agent-instruction-review): applicable.** The contract insertion at `proposal.md:40-114` and the loaded text at `context-boundary.md:16-40` are materially normative. Advisory only; no acceptance, implementation authority, or safe-runtime declaration.

## Finding dispositions

| Opus | Opus sev | Disposition | Fable sev | Causal reason and location |
| --- | --- | --- | --- | --- |
| 1 | P1 | uphold | P1 | `proposal.md:50-51` says restriction "remains binding" but `:64-66` only reuses lease revocation; the owner loop re-grants readiness after recompilation (`manager:601-606`) and the manager does not own human intent (`manager:75-78`). Nobody is assigned to judge whether input restricts the lifecycle operation itself. Irreversible (`manager:708-713`). `pilot.md:65-66` passes a run that compacts after "do not refresh". |
| 2 | P1 | qualify | P2 | `proposal.md:64-65` already quantifies over all human input; the requirement is correct. The gap is realization evidence (DESIGN §3.9) and pilot claim identity: `manager:626-649` orders only the switchboard; `pilot.md:55-57` names no path. Not an acceptance blocker. |
| 3 | P1 | qualify | P2 | `context-boundary.md:25-29` is a limitation, not a grant; "reads as permission" is overstated. Missing distinction is described-vs-available: the proposal's own boundary (`:126-128`) is not loaded, and no host contract supports the route today (`manager:644-646` rejects direct compact; a Claude manual root has no lifecycle host). |
| 4 | P2 | uphold custody gap, reject contradiction | P2 | `proposal.md:83-84` binds custody only to host-dispatched effects, so a role-dispatched compaction escapes the no-duplicate obligation. But `manager` invariant 9 already permits model actuation under host-owned state, and the design view's invariant 1 is proposed, not binding. No contradiction with the owner. |
| 5 | P2 | uphold | P2 | The wakeup is a third input category: not queued human input (`protocol:561-570`) and not the tool-less rehydration turn (`manager:694-697`). `proposal.md:73-79` gives it attribution but no admission position; `:67` departs from release-after-acceptance silently. |
| 6 | P2 | reject as contradiction | P3 | `proposal.md:50` and `:47-49` read together match the non-reservation default that `packet.json:23` reopens on. "Permit" is ambiguous; DESIGN §11 forbids imposing the affirmative-grant reading. Wording fix only. |
| 7 | P2 | qualify | P3 | Outcome already governed at `proposal.md:54-56` and checked at `pilot.md:69` ("approval remains pending"). Author/deliverer is a recording refinement. The pilot fixture change is accepted: the wakeup must assert closure so the case can fail. |
| 8 | P2 | uphold | P2 | `proposal.md:107-109` "resumption or notification" admits a realization whose only route is the resumption. `:102-103` requires distinguishing failure but names no route by which failure reaches the operator. S10 was announcement then silence. `pilot.md:118-119` scores visibility on nominal runs only. |
| 9 | P2 | qualify | P2 (pilot only) | `pilot.md:111-112` "admissible evidence" is undefined for post-pause work, unauthorized actions and wakeup replay. Bounded fix: the fixture closes the action set, so name a source per case. No global absence proof or all-tools monitor is required. |
| 10 | P3 | uphold | P3 | `proposal.md:35-37` claims reuse of "existing" protocol contracts, but attribution, abort-from-reserved and control-turn identity are new_context-shaped (`protocol:173-184, 542-549`; `manager:651-657`). Record as realization obligation. |
| 11 | P3 | uphold, not blocking | P3 | Intent-projection view (proposed status) overlaps `proposal.md:70-71, 118-120` on pending disposition. Scope "ordering" to lifecycle actuation and resumption; add a relationship note. Owner choice stays with the architecture owner. |

## New finding

**SSC-FABLE-1, P3.** Ambiguous scope of newer human input is not fail-closed at the admission boundary. `proposal.md:68-70` governs "affected domain work", leaving the successor or host to judge whether an ambiguous pause covers the pending compaction or the resumed work. `manager:485` makes ambiguity exact/escalate at compile time and invariant 3 forbids guessing closure, but neither is restated for lifecycle admission. One clause closes the shared root of Opus 1 and 5 without an approval step.

## Minimum necessary contract changes

Contract text (`proposal.md`):
1. **Lifecycle-targeting input.** Input ordered before actuation that restricts, or may restrict, the lifecycle operation itself leaves re-actuation unestablished until current authority resolves it. Reason: compaction is irreversible. Add one pilot row.
2. **Role request vs host admission.** A role may judge or request a transition; the host retains readiness and admission. Any lifecycle effect not dispatched under the host's durable transition operation leaves ordering, custody and no-duplicate claims unestablished.
3. **Wakeup admission identity.** Automatic continuation is an admitted input with a position in the same authoritative order; its productive admission is checked against every human input ordered before it. Record the reconciliation-participation delta against the protocol.
4. **Failure route.** A failed or unresolved resumption outcome must reach the operator through an owned evidenced route, or the announcement states where the outcome appears and what its absence means.
5. **Ambiguous scope** (SSC-FABLE-1): ambiguous coverage leaves admission unestablished rather than narrow.

Loaded text (`context-boundary.md`): one sentence stating that automatic resumption after self-invoked compaction is not an established route under current role or host contracts, while recovery from provider-initiated compaction still goes through Wind Walker.

Pilot: freeze human-usable ingress paths and inject through each or mark unsupported; adversarial wakeup content; per-case evidence source and completeness basis; score transparency on denied, interrupted and crash cases.

Wording: clarify "permit" at `:48` as coverage under existing authority without reservation and with a supporting host contract; qualify `:35-37` as subject to a realization binding compaction attribution and abort; scope "ordering" at `:118-120`.

## Owner decisions

None block progress. Opus's four questions resolve as follows: the authority default is settled by DESIGN §11 plus the non-reservation wording; host-only vs role-request is already answered by manager invariant 9; the intent-projection owner question is deferrable because both views are proposed; placement is out of scope by instruction. The one fact the owner should know is that the route is currently unestablished in both existing environments, Codex and Claude manual roots, which is a realization gap rather than a decision.

## Limitations

Read-only prose review of the frozen subject; no tests, commands or transcripts. Supplemental design views were read in cited ranges only and treated as proposed. The prior advisory review named in `packet.json:11` was not read. Requested effort was default; actual effort is not exposed and is not inferred. Opus findings were disclosed before my reading, so no independence from tier-1 reasoning is claimed.
