# Manual decomposition and short-lived builder experiment

Status: user-authorized manual experiment, 2026-10-20. This is not a new skill,
campaign engine, or replacement for the accepted S10 contract. The first subject
is the unfinished S10 completed-effect repair. Existing implementation and
qualification evidence is retained with its limitations; work is not restarted
to make an experiment cleaner.

## Question and scope

Can a shared decomposition and current, narrow evidence packets let fresh
`gpt-6-sol`/medium builders complete coherent work with less reconstruction and
context growth, without increasing omissions, repair or integration cost?

The mechanism is bounded work, not a token watcher. Workers cannot reliably
observe their current context through the available collaboration interface.
Do not add polling to workers or supervisors. Slice completion and deliberate
unfinished-work handoffs are events; the supervisor remains active in long waits
when no independent work remains. Context replacement preserves the Wind Walker
continuation-independence requirement.

The pilot changes how authorized work is dispatched. It does not grant paid
provider access, historical migration, publication, or acceptance of weaker
product behavior. The accepted S10 contract and its user qualifications own those
boundaries. The reviewer remains the retained Astra episode; this experiment
does not change its model or independence claims.

## Artifact model

Use ordinary Markdown and JSON files; no service or new generic persistence
framework is needed. Keep the supervisor specification out of worker startup.
The run directory is
`/home/bline/.local/state/work-engine/manual-decomposition-s10-20261020`.

| Artifact | Owner and purpose |
| --- | --- |
| `shared-contract.json` | Root: exact accepted contract/clarification and user qualifications, candidate base, authority and deferred obligations. Reference owners instead of duplicating their full text. |
| `decomposition-vN.md` | Decomposer/root: obligation families, dependencies, slice outcomes, exclusions and coverage uncertainty. Version rather than overwrite accepted decomposition decisions. |
| `evidence-vN.json` | Root: immutable path/hash bindings, claim scope, producer and limitations. Include mutable candidate file hashes; a base commit alone is insufficient. |
| `slices/<id>/packet-vN.md` | Decomposer/root: one ready assignment with relevant constraints, exact evidence, source entry points, outcomes, tests and escalation boundaries. Workers receive only this packet and necessary authority. |
| `slices/<id>/result-vN.json` | Worker: input binding, changed files/hashes, checks and result references, unresolved work, invalidated assumptions and proposed downstream impact. Preserve test failures and unavailable measurements. |
| `slices/<id>/handoff-vN.md` | Worker when unfinished: current objective, changes, settled decisions, failures, live process ownership, next action and remaining obligations. This is not acceptance. |
| `refresh-vN.json` | Root: accepted result, affected packet/evidence revisions, retained evidence with a reuse basis, invalidated claims, and any contract change referred to its owner. |
| `run-state.json` | Root: current worker and slice, latest packet, status, pending reviewer results and next eligible work. Update on real events, never by polling. |
| `experiment-result.md` | Root: outcomes, observed costs, omissions, reconstruction, operational problems and a disposition for skill extraction. |

Artifact examples produced by this run are the primary outcome. The eventual
skill candidate is a concise role/phase-specific operating specification plus
these demonstrated packet shapes, not a copy of this conversation or a claim
that an automatic decomposer has been validated.

## Initial decomposition of the remaining S10 work

The candidate already contains 17 changed files from builders g1/g2. Those are
unreviewed implementation, not accepted slice results. G2 fixed an actual
post-callback hold gap and passed focused tests, but did not establish real
two-store cuts or selected-host restart behavior. Reconcile that starting point.

| Slice | Outcome and evidence boundary | Dependency |
| --- | --- | --- |
| A: committed-read attribution cuts | Real lifecycle and task-owner stores close/reopen across before-owner-commit, owner-abort, after-owner-commit/before-ack and completed-callback cuts. Exact selected-host/runtime entry demonstrates held status and no repeated read or semantic send from unmatched state. Fix defects necessary for those claims. | Existing g1/g2 candidate, explicitly provisional |
| B: owner compatibility and material admission | Reconcile existing owner/schema tests; fill uncovered requested/returned length, legacy opacity, migration rollback/old-reader refusal, idempotence/conflict/currentness, trust and byte/count/prompt/unavailability checks. Verify limits are S10-specific and required qualification result fits. | Accepted A result and refreshed evidence |
| C: selected code-mode and two replacements | Exact `custom_tool_call exec` output exposes actual read result; fresh controlled two-cycle/FIFO/omission qualification preserves six obligations, pending approval and completed work without replay. No initial-input answer resupply. | Accepted A/B and updated fixture/binary bindings |
| D: final composition and review | Exact default provider-disabled composition, relevant package suite/fmt/Clippy, immutable candidate and evidence manifest, retained Astra review and root integration. | Accepted A/B/C evidence |

These are initial coherent outcomes, not fixed file-count or context-budget
partitions. Split a slice if its packet reveals an independent outcome; merge
only if that preserves ownership and reduces duplicate reconstruction. Record
the reason before dispatch. No slice silently drops an unresolved obligation.
Detailed downstream packets remain drafts until predecessor changes are known.

## Dispatch, acceptance and evidence refresh

1. Root binds the candidate revision and relevant evidence, reviews the slice's
   applicability against the shared contract, and marks a packet ready. Give a
   fresh worker `fork_turns="none"` and explicit Sol/medium selection. Record
   requested identity; observed identity stays unknown unless runtime evidence
   establishes it. Mandatory repository instructions still apply; do not make
   the worker read the entire orchestration brief or experiment history.
2. Worker acknowledges its exact packet and source, implements and checks the
   bounded outcome, then returns a result or a safe unfinished-work handoff.
   No usage-file monitoring, automatic next-slice work or new child agents.
   The worker may choose implementation details, but an ownership/authority
   change goes back to the root rather than being invented locally.
3. Root checks the result against the owning contract, not just the packet's
   checklist. Obtain retained review where material and run required final
   checks. A worker saying done does not accept the slice. A partial result
   keeps the slice open even if the worker is replaced.
4. At acceptance, compare changed boundaries to dependent assumptions. Rebind
   affected evidence and issue new packet versions; do not merely update a
   hash next to an invalidated claim. Keep unaffected evidence only with a
   stated reuse basis. Evidence refresh does not authorize a changed contract.
5. Dispatch the next ready packet. Completion wakes the parent; idle-agent
   messages do not start work. Use a fresh launch for a new worker context and
   an explicit follow-up only for an intended retained continuation.

Never treat a packet as exhaustive merely because it is precise. Workers
reporting discovered gaps provide useful feedback, not proof of decomposition
completeness. Before dispatch and in final review, inspect coverage against the
accepted contract and the independently identified families: ownership/authority,
state persistence and uncertainty, effect duplication, compatibility, result
trust/provenance, bounds, source currentness and semantic preservation. This
family list is a task-specific map, not a universal complete taxonomy.

## Measurements and interpretation

Keep telemetry gathering retrospective or bundled with an existing event; do
not add model calls to monitor token usage. Record work artifacts separately
from metrics. Minimum observations per worker segment:

- requested/observed model and effort, packet size, required startup documents
  actually read, and session reference where available;
- entry/peak per-call input and cumulative input/cache/output from available
  logs, with definitions and missing values explicit;
- decomposition, packet preparation/refresh, worker, review and integration
  usage separately when attributable;
- additional retrievals and their purpose: necessary implementation evidence,
  stale packet repair, missing constraint, or repeated orientation;
- calls/time to an accepted outcome, corrections, review findings and semantic
  gaps discovered outside the packet; distinguish infrastructure failures;
- handoff/reconstruction work, early handoffs and involuntary compactions.

Peak minus entry context is not a rediscovery measurement. Launch latency is
not semantic acquisition cost. Account-window utilization is not context size.
Compare complete task costs under a consistent token/cache weighting where
available, not just peak context or one worker's bill.

This first run is an operational feasibility case on existing unfinished work,
not a controlled cost comparison. G1's missed soft context trigger and g2's
bounded handoff are observations, not matched experimental arms. A later cost
claim needs matched known work under monolithic and decomposed assignments,
same acceptance gates/model/effort and measured packet/refresher/reviewer cost.
Use disposable copies; acceptance of such a benchmark is separate from this
run. Evaluate any automatic decomposer separately on held-out known defect
families before workers see its packets; worker feedback cannot measure unseen
family omissions. Such a benchmark is not required to finish S10.

## Mid-run measurement amendment: packet discharge and displacement

Added during slice A after dispatch, from the user's parallel Claude experiment;
this is not preregistration for A. It adds reporting distinctions, not new
product acceptance criteria or authority to run paid comparison arms.

For each packet obligation, distinguish an evidence-backed disposition from an
assertion or unresolved item. A useful disposition names the property, relevant
states, evidence and bounded exhaustiveness basis, with limitations. Count these
categories against a stated denominator; a completion event alone establishes
none of them. Exhaustiveness is a claim to justify within a scope, not a demand
for an impossible universal proof. Product acceptance still follows the owning
contract and review.

Record findings outside the packet and map them to named obligations where
possible, preserving ambiguous overlap. An empty count does not establish an
absence of omissions. For known-defect benchmark arms, report sealed-key recall
separately from packet coverage and outside-packet findings. A decline in outside
findings can suggest attention displacement, but needs comparable subjects and
classification; raw counts alone do not establish its cause. Keep sealed keys
out of worker packets. The current S10 feasibility run has no sealed-key score.

When preparing subsequent packets, include plausible weak realizations and the
observations that would falsify them where known. These examples aid judgment;
they are not an exhaustive defect list or mandated implementation route. For A,
examples are a held label with an open entry gate, callback success without
owner readback, or preventing a second read while admitting another effect.
The already dispatched A packet contains related constraints; its result may
report these distinctions from available work without manufacturing a baseline
or doing extra exploration solely to populate metrics.

Coverage review remains broader than the packet. Record uncertainty about
omitted families and permit relevant outside-packet findings to return to the
supervisor for scope decisions. An instruction to look beyond a packet cannot
itself validate decomposition completeness. Compare its effect experimentally
before treating it as a sufficient safeguard. Preparing strong falsification
targets and evidence updates has a cost that belongs in total experiment cost.

## Completion and follow-on artifact

The manual experiment is complete when the dispatched S10 outcomes are accepted
or truthfully blocked, the evidence refresh chain is reconstructible, and an
experiment report distinguishes demonstrated mechanics from unestablished cost
or quality benefits. Report whether packets reduced avoidable reconstruction,
what missing information mattered, and whether boundaries were useful.

The final report proposes one of: continue the manual method, adjust its
boundaries/packets, extract a thin skill, or stop the approach. A skill proposal
must identify loading by role/phase, unfinished-handoff versus terminal-result
semantics, evidence invalidation and review ownership. It must reconcile the
existing slice-builder same-identity/terminal receipt rules explicitly; this
pilot does not override them. No generic workflow engine, historical import,
production recovery, scalable source retirement, live semantic adequacy or
human M6 acceptance is claimed by the experiment.
