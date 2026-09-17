# Proposed Upstream Amendments for Candidate Trajectory

## Status

Candidate amendments supporting
[Candidate Trajectory — Durable Foundation](candidate-trajectory-durable-foundation.md),
[Remediation Delta for Native Review](candidate-trajectory-remediation-delta-for-native-review.md),
and [Builder-Side Candidate Trajectory Consumption](candidate-trajectory-builder-side-consumption.md).
They are not accepted or applied and do not authorize implementation. Each
target's decision owner retains authority over its disposition.

The entries identify possible changes to existing pending idea documents (or,
where noted, to Candidate Trajectory's own documents). They are design
hypotheses about how each document's evidence sourcing should change now that
a Candidate Trajectory primitive has been proposed, not execution findings.

```yaml
idea_status:
  architectural_supersession: none
  residue: unknown
  residue_note: "Corrected 2026-09-17, per review: a keyword-scan-only pass on a 744+ line document is not sufficient scope to support the universal negative 'no residue exists.' residue: unknown, not none, until a full close-read or a broader targeted check is done."
  backlog: present
  backlog_ledger: "One item tagged KIND: BACKLOG (the shared event-store implementation question near A2), confirmed open, no dedicated staged-plan section otherwise. backlog: present stands because this is an existential (one confirmed source suffices), unlike residue: none's universal claim above."
  audit_scope:
    - keyword-scan: full_document
  audit_scope_completeness: partial
  audit_scope_completeness_note: "Full document not close-read line-by-line this pass (744+ lines across A-G sections); keyword scan only."
  status_as_of: 2026-09-16
```

```yaml
idea_provenance:
  origin: direct_capture
```
Section A targets Deterministic Refactor Pressure; section B targets
Evidence-Calibrated Plan Resolution and Continuous Capability Learning;
section C targets Revisioned Research and Execution Architecture (mostly
resolved by amending Candidate Trajectory's own foundation document instead);
section D targets Incremental Terminal Accounting Projection. The four targets
are independent of each other; no amendment depends on another being
accepted. A broader survey found most of `app-server/ideas/pending/` does not
clear this document's evidentiary bar for a genuine connection — see E5 for
the narrower candidates that were surfaced but deliberately not drafted.

## Amendment map

| Target | Candidate changes |
| --- | --- |
| Deterministic Refactor Pressure | Add Candidate Trajectory as a within-slice Rework input alongside (not upstream of) final-candidate Code Change Profile observations; leave Historical Recurrence owned by cross-slice aggregation; correct its section 1.1 framing of `code-change-profile`'s ownership status; qualify Phase A backfill for slices predating this primitive. |
| Evidence-Calibrated Plan Resolution and Continuous Capability Learning | Propose Candidate Trajectory as a concrete, production-derived, inference-free observation source for the continuous-learning flow, at three distinct levels (measurement, capability learning, capability modification) rather than as a single generic signal; correct an earlier framing that treated it as a direct closure-cost measurement. |
| Revisioned Research and Execution Architecture | Record identity-architecture cross-validation (already applied directly to `candidate-trajectory-durable-foundation.md` section 3.1, not to this target); name an optional, undrafted pointer the target's own owner could add. |
| Incremental Terminal Accounting Projection | Record the shared incremental-projection/anti-reconstruction pattern as corroborating precedent; explicitly guard against Candidate Trajectory facts defaulting into terminal accounting as a semantic sink. |

## A. Deterministic Refactor Pressure from Work Engine Evidence

Target: [deterministic-refactor-pressure-from-work-engine-evidence.md](deterministic-refactor-pressure-from-work-engine-evidence.md).

### A1. Correct the section 1.1 ownership framing

Section 1.1 describes `skills/code-change-profile/` as the physical
characterization layer and cites the skill's own schema/contract files
directly. That is still accurate at the analyzer level, but it predates (or at
least does not mention) that `review-subject` is already the documented App
Server service boundary in front of that analyzer
(`app-server/docs/review-subject-service.md`), and that `docs/skills-migration-plan.md`
(S7) already records this ownership migration as landed
(`app-server/docs/skills-migration-plan.md:1059-1076`). Candidate change: add a
pointer to `review-subject-service.md` alongside the existing skill-level
citations, so a reader does not infer that promoting Code Change Profile to a
host service boundary is still open work — it is not; what remains open is
retaining the *relationship* between successive profiles, which is what
Candidate Trajectory adds.

### A2. Add Candidate Trajectory as a Rework input, not a mandatory intermediary for all profile evidence

An earlier pass at this amendment proposed inserting Candidate Trajectory
between Code Change Profile and every pressure observation. That overstates
Trajectory's role: it is essential for **within-slice rework evidence**, but
it is not the owner of the final candidate's physical characterization, and a
slice with exactly one candidate has no trajectory delta at all — forcing all
profile evidence through Trajectory would make single-candidate slices look
like a degenerate case of something they are not. Corrected architecture:

```text
                     Code Change Profile
                         /            \
                        /              \
             final candidate      Candidate Trajectory
             observations         (when candidate count > 1)
                        \              /
                         \            /
                          v          v
                       pressure observation
                          ^          ^
                          |          |
                   audit/validation
                   authoritative telemetry
```

Both branches feed the same pressure observation for a slice; a
single-candidate slice simply has an empty/absent Trajectory contribution
rather than a missing intermediary step.

Section 6.3 (Rework) already anticipates needing exactly the within-slice
branch and currently proposes deriving it ad hoc: "Additional deterministic
rework evidence could later be derived from successive candidate checkpoint
trees without inspecting chain-of-thought... same path changed across
multiple candidate attempts; same symbol repeatedly modified before
acceptance; accepted result reversing an earlier candidate change; repeated
remediation concentrated in the same module." Every one of those is a direct
readout of a Candidate Trajectory delta once that primitive exists. Candidate
change: revise section 6.3 to source these observations from Candidate
Trajectory records directly instead of describing them as a future ad hoc
derivation.

**Historical Recurrence (section 6.5) is not sourced from Candidate Trajectory
directly.** Recurrence is the fact that the *same structural coordinate*
exhibits pressure across *independent slices/campaigns* — a cross-slice
aggregation. Candidate Trajectory only ever sees within one slice/attempt; it
cannot itself observe recurrence. The corrected evidence chain is:

```text
Candidate Trajectory
        |
        v
within-slice rework evidence
        |
        v
slice pressure observation
        |
        v
cross-slice aggregation
        |
        v
historical recurrence
```

Candidate change: revise section 6.5 to keep Historical Recurrence owned by
the existing cross-slice aggregation step (section 5's "historical
aggregation" stage), consuming per-slice pressure observations that are
themselves better-evidenced once Rework draws on Candidate Trajectory — not
revise section 6.5 to claim Trajectory as a direct source.

### A3. Revise Phase A (section 15) to depend on Candidate Trajectory, with an explicit historical-coverage caveat

Section 15's Phase A ("historical backfill") proposes running Code Change
Profile "where immutable checkpoint evidence permits it and join results to
existing receipts." Candidate change: once Candidate Trajectory exists,
*prospective* Phase A backfill should be a join against Candidate Trajectory
records (which already carry predecessor bindings and deltas) rather than a
separate checkpoint-chain reconstruction effort.

This is true only prospectively. Slices that completed before Candidate
Trajectory existed have no trajectory records — reconstructing one after the
fact requires that the intermediate candidate checkpoints are still
retrievable, which is not guaranteed. Candidate change: state one of the
following explicitly in section 15 rather than assuming backfill coverage is
uniform:

> First reconstruct Candidate Trajectory records from retained immutable
> candidate evidence where available, then consume those records; or use
> Candidate Trajectory when present and treat historical runs without
> recoverable intermediate candidates as having reduced rework coverage.

This is not a new evidentiary standard — it is the same measurement-state
discipline Candidate Trajectory itself inherits from `code-change-profile`
(`observed` / `unknown` / `unsupported` / `failed` / `not_applicable`, per
`skills/code-change-profile/references/profile-contract.md:27-39`), applied to
historical coverage rather than a single measurement: absent history should be
recorded as reduced coverage, never fabricated or silently backfilled as zero
rework.

### A4. Do not change the falsifiability design (section 13) or the deliberately-cheap first version (section 10)

No candidate change here — flagging explicitly that sections 10 and 13 remain
sound independent of this amendment. Candidate Trajectory makes their inputs
cheaper and more reliable to obtain; it does not change the pre/post
comparison design or the "no fixed weights at first" position in section 11.

## B. Evidence-Calibrated Plan Resolution and Continuous Capability Learning

Target: [evidence-calibrated-plan-resolution-and-continuous-capability-learning.md](evidence-calibrated-plan-resolution-and-continuous-capability-learning.md),
and, for B1 specifically, amendment D1 of its own
[upstream amendments](evidence-calibrated-plan-resolution-upstream-amendments.md).

Candidate Trajectory is relevant to that document at three distinct levels.
Conflating them would overclaim what a deterministic structural observation
can establish, so the entries below are grouped accordingly:

```text
measurement              what happened during execution?

capability learning       what does repeated evidence suggest about
                          realization x projection x task?

capability modification   which cognitive work can be removed from the
                          realization entirely once it is hosted instead?
```

### B1. Outcome evidence for closure-cost predictions, not a direct closure measurement (measurement level)

An earlier pass at this amendment framed Candidate Trajectory as measuring
"additional closure" directly, to support amendment D1's question of "whether
characterization predicts the additional closure needed by a candidate
realization." That overstates what Trajectory can establish. The target
document defines the actual closure question as "what additional decisions
must be resolved for this realization to execute within its contract" —
a *pre*-execution, planner-side question. Candidate Trajectory only sees what
happened *after* execution began: repeated modification, surface
expansion/contraction, paths/modules entering or leaving, and exact C1→C2
change content. It cannot see whether an observed expansion happened because
closure was missing, the executor was weak, the task was genuinely hard, the
projection was wrong, or legitimate discovery occurred during implementation.

Candidate change: revise D1 to read Candidate Trajectory as **outcome evidence
for validating a closure-cost prediction, not a direct measurement of
closure**. For example:

```text
planner predicts: realization R needs extra authority-boundary closure
actual:           no extra closure supplied
execution:        C1 -> C2 adds an authority-owning module;
                   C2 -> C3 modifies it again;
                   review finding concerns the ownership boundary
```

That is strong evidence worth investigating. It is not, by itself, proof that
insufficient closure caused the outcome.

### B2. Dimension-correlated outcome evidence, not a generic rework signal (measurement level)

The target document's resolution profiles vary multiple independent
dimensions — `dependency_structure`, `repository_localization`,
`authority_boundaries`, `judgment_boundaries`, `integration_contracts`, and
others. Candidate change: note that Candidate Trajectory's value is sharper
than a generic "this execution struggled" signal — it can potentially supply
*dimension-correlated* outcome evidence. But the two kinds of correlation are
not equally direct, and conflating them would smuggle semantics into a
primitive whose own contract is deliberately physical (paths, modules,
symbols, categories, attribution, dependency deltas — see the foundation
document's delta contract). Candidate Trajectory does not itself know that a
path is "authority-owning" or that two modules constitute separate Plan IR
integration regions; it only knows that a path/module entered, left, or was
modified again. So:

```text
direct trajectory correlations (no join needed):
  repository_localization resolution  <->  new modules entering during remediation
  dependency_structure resolution     <->  new validation/dependency surface during repair

joined correlations (trajectory + an independent semantic mapping):
  authority_boundaries resolution  <->  path entry/modification
                                        + an authoritative authority-ownership mapping
  integration_contracts resolution <->  trajectory delta
                                        + a Plan IR region/boundary mapping
```

The joined form is not a weaker idea — it is architecturally the right one: it
keeps Candidate Trajectory semantically blind (matching the foundation
document's non-goals) and pushes the "what does this change relate to"
question to whatever owns that mapping, the same separation already proposed
for [targeted validation selection](candidate-trajectory-builder-side-consumption.md)
(trajectory supplies "what changed"; a separate dependency/relationship map
supplies "what that implies"). None of these correlations, direct or joined,
should be presumed causal (see B1). But direct correlations are more specific,
and therefore more useful for evaluating *which* dimension's resolution
mattered, than an undifferentiated rework score would be — and the joined
correlations require that a real mapping exist and be trustworthy before they
mean anything at all.

### B3. An attribution stream, not another capability score (capability learning level)

The target document explicitly warns against **capability collapse** — "a
single score hides task, realization, projection, or verification
differences" — and its research question 9 already asks "can failures be
attributed across plan, projection, executor, harness, verification, and
integration?" Candidate change: note that Candidate Trajectory is a good fit
for that attribution doctrine specifically because it is a host-derived
observation stream grounded in independently owned immutable checkpoint and
physical-profile evidence, rather than executor self-report — not because its
own service/storage ownership is settled (the foundation document explicitly
leaves that an open implementation decision). That evidentiary independence,
not a particular ownership boundary, is the property B3 needs: a future
correlation across canonical plan, projection profile, realization, Candidate
Trajectory, review findings, validation, and acceptance does not require
asking the executor to narrate why it struggled. It adds an evidence source to an attribution design the
document already wants; it does not propose the attribution design itself.

### B4. A revision-bound production observation source, generated without a separate model-evaluation call (capability learning level)

The "Continuous learning and authority" section states that "controlled
pilots could establish initial capability evidence" and that "authorized
production then supplies revision-bound observations that strengthen,
weaken, or refine that evidence." Candidate change: note that once Candidate
Trajectory exists, it is exactly this kind of revision-bound production
observation, generated as ordinary deterministic execution exhaust of normal
Work Engine execution — candidate count, repeated path/symbol modification,
surface expansion/contraction, module entry/exit, remediation concentration —
without a separate model-evaluation call to obtain it:

```text
ordinary production -> Candidate Trajectory -> revision-bound structural
observations -> capability evidence -> targeted pilot only when ambiguity
matters
```

This is **inference-free, not cost-free.** The target document itself notes
that "deterministic work also has runtime and maintenance costs even when its
model-inference cost is zero," and Candidate Trajectory's own foundation
document is explicit that symbol-level "modified again" facts require an
additional Git tree diff and AST re-derivation beyond the physical profile
already computed — a real, if bounded, compute cost, not a free byproduct.
What this observation class avoids is a *model* call to obtain it; it does not
avoid engineering, storage, or maintenance cost.

This makes the document's continuous-learning loop concrete rather than
abstract for at least this one observation class, and reduces (without
eliminating — see B1's ambiguity) how often a targeted pilot is needed just to
obtain structural outcome data.

### B5. Richer fixtures for controlled replay and counterfactual resolution experiments (capability learning level)

The "Evidence sources and counterfactuals" table distinguishes controlled
pilot, controlled historical replay, and production execution, and notes that
"production does not directly reveal what another strategy would have cost"
and that replay "may omit state relevant to the claim." Candidate change: note
that [Candidate Trajectory](candidate-trajectory-durable-foundation.md) directly
improves historical-replay fidelity for this document's resolution
experiments, because it preserves the actual intermediate candidate sequence
instead of leaving it to be reconstructed from Git or model memory after the
fact — which is precisely the gap the foundation document identifies in
current campaign state (`service.mjs:255-278` overwrites the predecessor
candidate and profile on every replacement). Once preserved, a historical
coordinate can state "under projection P1/realization R, C1→C2→C3 actually
happened this way," and a research branch can replay the same starting problem
under P2/R2 and compare trajectory shape, not only terminal acceptance.

### B6. Builder-side consumption changes what is being measured, not just what is measured (capability modification level)

[Builder-side trajectory consumption](candidate-trajectory-builder-side-consumption.md)
proposes moving mechanical facts out of the builder's inference burden while
leaving semantic consequences with the builder. Candidate change: note that if
this is ever adopted, it does not merely give the target document's capability
evidence a new input — it changes what capability evidence *means*. Today a
realization's apparent capability implicitly includes its ability to remember
and reconstruct mechanical facts about what it itself just changed. After
Candidate Trajectory adoption on the builder side, that burden is removed from
the realization and performed by the host instead. The target document already
requires capability evidence to be indexed by "execution realization, problem
characterization, judgment dimension, projection resolution, and verification
environment" and to retain "context and projection, capability grant" — this
amendment adds that host-supplied deterministic observation is itself part of
that context, so pre-adoption and post-adoption capability evidence for the
same nominal realization are not directly interchangeable. Candidate change:
record this explicitly as a versioning boundary wherever capability evidence
is stored, rather than silently pooling evidence gathered before and after any
future adoption of builder-side Candidate Trajectory consumption.

### B7. What this section does not establish

- It does not claim any of B1–B6's correlations are causal; B1 and B2 exist
  specifically to block that inference.
- It does not make Candidate Trajectory a capability score, a routing input,
  or an admission criterion. Its own foundation document already restricts it
  to physical observation (`candidate-trajectory-durable-foundation.md`,
  section 3.4).
- It does not authorize using Candidate Trajectory data for routing or
  admission outside whatever authorized-evidence discipline the target
  document's own "Continuous learning and authority" section already
  requires.
- It does not resolve B6's versioning question, only records that it exists.

## C. Revisioned Research and Execution Architecture

Target: primarily [Candidate Trajectory — Durable Foundation](candidate-trajectory-durable-foundation.md)
itself (already amended directly — see its section 3.1), not
[revisioned-research-and-execution-architecture.md](revisioned-research-and-execution-architecture.md).
This section records why, and names the one light-touch option left for the
research document's own owner to consider.

### C1. This is cross-validation, not merely an example — recorded on Candidate Trajectory's side

The research document independently requires (section 23): "Derived
comparison artifacts such as diffs are projections, not subject identity...
bind [their] endpoint identities and a canonical projection format, or record
the generator and every option or configuration input that can change [their]
bytes." Candidate Trajectory's own three-layer identity split (candidate
identity / profile observation / trajectory derivation, foundation document
section 3.1) is the same rule, reached independently from candidate
remediation rather than experimental reproducibility. Section 27 ("Coordinate
Adequacy Is Claim-Relative") similarly corresponds to the posture Candidate
Trajectory's own delta output needs: physical facts established
unconditionally, sufficiency judged per consuming claim.

This is why the primary candidate change already landed directly in
`candidate-trajectory-durable-foundation.md` rather than here: the research
document "deliberately remains a conceptual reference rather than a registry
of every concrete primitive" (per its own stated purpose), so Candidate
Trajectory is the side that benefits from citing the external grounding, not
the side that should be rewritten to list Candidate Trajectory as an example.

### C2. Optional, deferred candidate change for the research document's own owner

If the research document's owner ever wants a pointer in the other direction,
a single line under its "Contributing directions and distinct owners" section
naming Candidate Trajectory as one concrete instance of the reproducibility
requirements in section 23 would be sufficient — not a rewrite, not a new
subsection. This document does not propose that edit; it only names the
option so it is not lost. The research document's own scope decision governs
whether such a pointer is worth adding at all.

## D. Incremental Terminal Accounting Projection

Target: [incremental-terminal-accounting-projection.md](incremental-terminal-accounting-projection.md).

### D1. Shared architectural pattern — real, worth recording

Both proposals independently reach the same anti-reconstruction move. The
target document: "Make terminal accounting an incrementally maintained
projection of admitted runtime events... Terminalization should then bind and
validate the current projection rather than rediscover history... Missing
instrumentation remains an explicit availability state with provenance; it
never becomes an inferred zero," with "event identities and projection
revisions... idempotent and CAS-bound" and "recovery and continuation packets
carry the projection identity, not a model-authored summary." Candidate
Trajectory makes exactly the same move for candidate evolution specifically:
append the transition when it occurs (at `bindCandidate` time) instead of
letting a later consumer reconstruct C1→C2 from Git or model memory, with the
same idempotent-replay and crash-recovery obligations (foundation document
section 5). Candidate change: note this shared pattern as independent
corroboration that incremental, append-at-admission projection is the right
shape for this class of problem in Work Engine generally, not specific to
either proposal.

### D2. Guardrail: shared pattern and possibly shared event infrastructure, not a shared semantic owner

The target document's own ownership shape is deliberately narrow: it owns
"pre-provider failure, provider entry, transport outcome, correction,
retained-session retry, fallback, repository-evidence stage, and review-gate
use," and its accounting projector "owns deterministic aggregation and
internal consistency, but no workflow decision or acceptance authority."
Candidate Trajectory has its own durable subject (candidate evolution) and
serves several consumers beyond receipt accounting — reviewer context, builder
context, capability learning, Refactor Pressure, research replay (section C
above). Candidate change: do **not** propose that Candidate Trajectory
facts flow into terminal accounting as a default sink merely because an
append-oriented projector already exists there. Instead:

```text
candidate bound
      |
      +-- Candidate Trajectory event/projection
      |       owns candidate evolution facts
      |
      +-- terminal-accounting consequence
              only if the receipt contract actually
              requires a derived accounting fact
```

rather than:

```text
candidate bound -> put all trajectory facts in receipt accounting
```

Whether the two proposals should share the same underlying append-only event
store/projector *infrastructure* (as opposed to semantic ownership) is a
legitimate implementation question for whoever picks up either proposal; this
document takes no position on it and does not require they share
infrastructure.

### D3. Non-goals

- This does not propose merging Candidate Trajectory's storage with the
  terminal-accounting event store.
- This does not make terminal accounting a consumer of Candidate Trajectory by
  default; only a receipt-contract requirement (a field the receipt schema
  actually needs) would justify that specific consequence, decided by the
  receipt schema's own owner.
- This does not weaken the target document's own ownership boundary
  ("no workflow decision or acceptance authority" for the accounting
  projector).

## E. What this amendment does not touch

### E1. Target A — Deterministic Refactor Pressure

- It does not propose building Refactor Pressure now, or on any schedule.
  Sol's original sequencing conclusion — durable trajectory foundation first,
  reviewer-delta consumer second, "then the historical/refactor work becomes
  almost embarrassingly straightforward" — is adopted as-is: Refactor Pressure
  remains the last, longitudinal consumer, not a co-requirement of Candidate
  Trajectory.
- It does not touch sections 2–5, 7, 9, 12, 14, or 16 of the target document;
  their content is orthogonal to what Candidate Trajectory changes.
- It does not propose a pilot for Refactor Pressure beyond what its own
  section 15 (Phases A–D) already specifies. That phased historical-backfill-
  then-prospective-validation design is itself the right empirical process for
  a longitudinal, cross-slice claim; it does not need a separate controlled
  pilot in the sense used elsewhere in `app-server/ideas/pending/pilots/`,
  because it is not testing a model-judgment framing effect — it is testing
  whether a deterministic structural signal predicts execution outcomes it did
  not previously have access to at all.

### E2. Target B — Evidence-Calibrated Plan Resolution and Continuous Capability Learning

- It does not propose a resolution profile schema, a routing policy, or a
  capability-evidence storage schema. Section B only proposes candidate
  observation sources and one versioning caveat (B6); the target document and
  its own amendment sheet retain sole ownership of those designs.
- It does not touch amendments A–C, E, F, or G of
  `evidence-calibrated-plan-resolution-upstream-amendments.md`; only D1 is
  addressed, and only to correct what kind of evidence Candidate Trajectory
  can supply to it.
- It does not propose a pilot beyond what the target document's own
  "Evidence sources and counterfactuals" section already structures (B5 only
  proposes that Candidate Trajectory improves the fidelity of one evidence
  class — controlled historical replay — already named there).
- It does not resolve whether dimension-correlated evidence (B2) is strong
  enough to inform routing; that remains gated on the target document's own
  evidentiary bar for moving from "observe" to a policy input.

### E3. Target C — Revisioned Research and Execution Architecture

- It does not propose any edit to `revisioned-research-and-execution-architecture.md`.
  The primary change already landed in Candidate Trajectory's own foundation
  document (section 3.1); section C above only names, without proposing, an
  optional pointer the research document's owner may add later.
- It does not claim the two designs' convergence proves either is correct in
  general — only that reaching the same identity-separation rule from
  unrelated directions is corroborating evidence for that specific rule.

### E4. Target D — Incremental Terminal Accounting Projection

- It does not propose merging Candidate Trajectory's storage with the
  terminal-accounting event store, or making terminal accounting a default
  consumer of trajectory facts (see D2/D3's explicit guardrail).
- [KIND: BACKLOG] [OPEN — an implementation-choice question for whoever eventually implements either proposal, not an ownership/architecture question] It does not propose a shared event-store implementation; D2 identifies that
  as an open question for whoever implements either proposal, not a decision
  this document makes.
- It does not touch the target document's provider/harness/campaign ownership
  boundaries (`ProviderTurnPort`, `HarnessRuntimePort`, campaign/review
  services) or its migration sketch.

### E5. Three minor candidates surfaced by survey, deliberately not drafted here

A broader survey of `app-server/ideas/pending/` also surfaced three narrower
candidates — `proposal-decision-gated-implementation-compilation.md` (its
Stage 0/3 divergence metrics as further instances of the outcome-evidence
pattern already in B1/B4), `scoped-workflow-event-surface.md` (candidate
binding as a named "committed semantic event" whose payload Trajectory could
enrich), and `semantic-inventory-integrity-projection-separation.md`
(`classifySkillsMigrationIntegrity`'s baseline-vs-candidate-only limitation
sharing Trajectory's exact profile-delta blind spot for one classifier). None
are drafted into a section here: each is small enough to be a single bullet if
and when that specific target document is actually being amended, and drafting
them now without an active reason would be exactly the kind of unforced,
associative connection this document has otherwise tried to avoid.

## Boundaries and evidence limits

This document does not own target content, acceptance of any amended
proposal, measurement schemas, or implementation authority. The companion
documents own their respective architectural hypotheses; each target retains
its own semantics. Section C's primary change was applied directly to
`candidate-trajectory-durable-foundation.md`, which this effort does own; that
edit is not itself an amendment awaiting another owner's acceptance the way
sections A, B, and D are. These amendments have no pilot results attached —
they describe proposal text and current source-code state as read across
2026-09-13, not implemented runtime behavior.
