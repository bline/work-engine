# Reconciliation: Closed-Loop Engineering Learning

## Status

Reconciliation of `ideas/closed-loop-engineering-learning.md` against
current app-server implementation, `app-server/ideas/pending/`, and existing
prospective-architecture documents. Wave 4, item 11 of the sequel
reconciliation queue.

Per instruction: the idea was not improved or modernized. Applying the
patterns established by items 1–10: verify exact type/temporal-position
match rather than family resemblance before retiring a clause; decompose
compound sections before declaring them one residue; separate mechanical
comparison from attributed semantic judgment rather than letting one collapse
into the other.

## Idea summary (unchanged)

`ideas/closed-loop-engineering-learning.md` asks for a durable feedback loop
comparing a proposal's predicted consequences (expected value, complexity,
architectural reach, validation burden, risk) against the implementation's
observed consequences (actual scope, actual review burden, route revisions,
failures/recoveries, validation evidence, observed maintenance consequences),
preserving the difference as calibration evidence for later evaluators and
planners — without becoming automatic policy, model-selection, or acceptance
authority. Its own "Important boundary" section explicitly warns against
flattening the reasons a prediction can be wrong (poor understanding, poor
execution, environment change, unusual route difficulty, reasonable
uncertainty realized badly) into one score.

## What is already implemented (retires "Current evidence" and the predicted side of "Required consequence")

### The predicted side is confirmed already owned by item 6, not this idea

The idea's own "Required consequence" comparison table lists expected value,
complexity, architectural reach, validation burden, and risk as the
"proposal expectation" side. This is, dimension for dimension, item 6's
(`evidence-backed-proposal-evaluation.md`) already-reconciled scope: typed
evaluation estimates with their own scale, directionality, confidence, and
evidence cutoff (`evidence-backed-proposal-evaluation-reconciliation.md`).
This is exactly the relationship item 6's own "Does not own" section already
names: "calibrate future evaluation from outcomes... belong[s] to... closed-
loop learning." Both ideas agree independently, from opposite directions,
that item 6 supplies the predicted side and this idea consumes it — a
confirmed `SUPPLIES` relationship, not a new prediction schema for this idea
to define.

### The observed side decomposes across several real and pending sources, not one

Checked field-by-field rather than left as one bundled "actual X" list:

- **actual review burden** — reclassified, not simply retired to
  `review-finding-bridge.mjs`. `review-finding-bridge.mjs` can establish how
  many findings existed, their severity, exact revisions, evidence, and
  dispositions — finding *semantics*, not review *burden*. Burden is closer
  to review invocations/episodes, iterations, wall time, tokens, remediation
  cycles, provider attempts, and possibly operator attention — execution/
  accounting observations, not finding content. `incremental-terminal-accounting-projection.md`
  is the closer prospective supplier here too: it explicitly folds
  review-gate events together with provider, retry, and recovery events into
  one revision-bound accounting record. Review-finding records remain a
  legitimate *input* to burden analysis (e.g., counting remediation cycles
  via finding predecessor/successor lineage), but they do not own burden.
- **actual scope, failures/recoveries, validation evidence, and review
  burden** →
  `app-server/ideas/pending/incremental-terminal-accounting-projection.md`
  ("pending post-migration proposal," not yet implemented), which already
  proposes exactly this: an incrementally-maintained terminal receipt
  projection folding provider-entry/outcome, transport, retry, fallback,
  repository-evidence, and review-gate events into a revision-bound
  accounting record, explicitly to avoid "fabricated zeroes or repeated
  forensic work" from post-hoc reconstruction — the same problem this idea's
  "observed consequences" side would otherwise face.
- **route revisions** → `app-server/ideas/pending/candidate-trajectory-remediation-delta-for-native-review.md`
  and `candidate-trajectory-builder-side-consumption.md` (both authored
  earlier this session as part of the Candidate Trajectory family), which
  already propose an exact, host-owned remediation delta (C1→C2) between
  successive candidate attempts within one slice — the within-slice-level
  granularity this idea's "route revisions" needs, also not yet implemented.
- **provider/runtime telemetry** → confirmed real: item 10's reconciliation
  read an actual accepted slice's metrics record
  (`metrics/agent-instruction-review.jsonl`) in full, containing exactly this
  shape of provider/token/timing telemetry.
- **strategic planning handoffs** → confirmed real:
  `app-server/roles/strategic-planning-handoff.mjs`, dogfooded in
  `post-migration-strategic-plan.md` (items 5, 7, 8–9).

Disposition: retire "Current evidence" as accurate — every named artifact is
real, though two of the "observed consequence" sources
(`incremental-terminal-accounting-projection.md`,
`candidate-trajectory-remediation-delta-for-native-review.md`/
`candidate-trajectory-builder-side-consumption.md`) are themselves still
pending, not built. This idea does not need to invent observation machinery;
it needs those pending designs to land, and then to consume their revisions
— the same `SUPPLIES` shape as the predicted side. Review burden specifically
routes to `incremental-terminal-accounting-projection.md`, not
`review-finding-bridge.mjs` — findings and burden are separate concerns with
different owners.

### `evidence-calibrated-plan-resolution-and-continuous-capability-learning.md` is a distinct, non-competing neighbor

Checked directly because both ideas use "calibrat-" vocabulary: that
document's own "Continuous learning and authority" section calibrates
*provider/model/plan-resolution execution strategy* from production
observations (which routing or capability profile performs well), not
*proposal-evaluation prediction accuracy* (was this proposal's expected
value/risk/complexity correct). `candidate-trajectory-upstream-amendments.md`
already keeps these targets explicitly separate — its own amendment sheet
treats "Evidence-Calibrated Plan Resolution" and this idea's territory as
independent targets, confirming this is not a naming collision that needs
resolving here.

## What the idea's own boundary already gets right

"Does not own" (raw execution metrics, proposal evaluation itself, roadmap
priority, automatic policy changes, model selection, reward optimization,
acceptance of future proposals) is retired as accurate: raw execution
metrics → terminal accounting projection and existing telemetry; proposal
evaluation → item 6; roadmap priority → item 7; the remainder have no
competing claimant found anywhere in this queue. The "Important boundary"
section (a prediction can be wrong for several distinct reasons that must
not collapse into one score) is retired as a correct design constraint on
whatever residue survives below — not itself a gap, but a requirement the
residue must satisfy.

## The smallest remaining semantic consequence still lacking an owner

Applying the mechanical/semantic split items 8–9 already established for
evidence-anchor and seam review, the comparison this idea asks for does not
survive as one undifferentiated "calibration" residue — but the first pass
of this reconciliation drew that split in the wrong place. It treated
"predicted value vs. observed value" as automatically commensurate once both
sides exist, needing only sequencing behind their upstream sources. That
holds for genuinely paired quantities (predicted cost vs. observed cost), but
the idea's own comparisons are frequently not that: predicted complexity
against observed effort, predicted risk against repairs or regressions,
predicted impact against a later-measured consequence, confidence against
prediction accuracy. These are calibration relationships, not the same
variable measured twice, and item 6's own reconciliation already established
that comparability requires declared units, scale, directionality, and an
explicit comparison surface before two measures can be compared at all.

```text
prediction revision (item 6)
        +
outcome observations (terminal accounting / remediation-delta, once built)
        |
        v  missing
prediction<->outcome correspondence contract
    for a given predicted measure: what observation answers it, unit/scale
    compatibility, aggregation window, cutoff, normalization if any,
    tolerance/interval semantics, treatment of missing evidence, and
    whether one observed outcome is even sufficient to evaluate a
    probabilistic prediction
        |
        v
deterministic comparison, only once that mapping exists
    matched / diverged / unresolved
```

**Prediction↔outcome correspondence and comparison is partially sequenced,
partially unresolved** — not simply "sequenced, not architecture" as the
first pass stated. Upstream prediction and observation sources must exist
before any comparison can run, but the mapping that establishes whether a
particular prediction and a particular observed consequence are even
comparable is itself a required semantic/typed contract, narrower than the
causal-judgment residue below but still genuinely unowned — no document
inspected here defines it.

The second half is not, as the first pass called it, an "attributed causal
judgment of why the prediction was wrong." That framing is stronger than the
idea's own evidence supports, and stronger than receipts can establish. A
bad observed outcome does not automatically mean the prediction was wrong:
a proposal that predicted substantial risk at low confidence, followed by
the risky outcome actually occurring, may have been a *good* prediction: a
probabilistic estimate is not necessarily falsified by one realized outcome.
The idea's own key question is closer to how to attribute failure modes
strongly enough to learn from them without pretending causal certainty the
receipts cannot establish.

**Attributed calibration diagnosis** — a record supporting multiple possible
contributing explanations (poor proposal understanding, execution defect,
environment change, unusual route difficulty, uncertainty realized
adversely, or unknown/insufficient evidence), each with its own evidence,
confidence, and limitations, rather than one singular verdict:

```text
comparison finding (matched / diverged / unresolved)
        |
        v  missing
attributed calibration diagnosis
    possible contributing explanations, each with evidence/confidence/
    limitations and its own causal strength -- not "the proposal was bad"
    or "the builder was bad" as a reflexive default, and not automatically
    "the prediction was wrong" merely because the outcome was adverse
        |
        v
what, if anything, future evaluation evidence should update
```

No document inspected in this reconciliation assigns either the
correspondence contract or the calibration diagnosis an owner or a record
shape.

Both pieces are coupled to `claim-evidence`'s existing domain-profile
pattern (the same shape as items 4, 5, 6, and 9's couplings), not
free-standing: a calibration record binding a predicted-value revision, an
observed-value revision, their comparison finding, and an attributed
calibration diagnosis is buildable on claim-evidence's substrate (subject
identity, evidence references, revision lineage) following the
`review-finding-bridge.mjs` pattern — but no such profile exists, and this
reconciliation does not assume `claim-evidence` intends to host it merely
because the fields would fit, per the same caution items 4 and 6 required
after an initial overreach.

```text
item 6 (predicted value)          terminal accounting + remediation-delta
    |                                 (observed value, both pending)
    +----------------+----------------+
                     |
                     v  missing (prediction<->outcome correspondence
                                  contract -- partially sequenced,
                                  partially unresolved)
              deterministic comparison, where valid
                     |
                     v  missing (genuine residue, no owner found)
        attributed calibration diagnosis
              (possible contributing explanations, not one verdict)
                     |
                     v  missing (coupled to claim-evidence's
                                  domain-profile pattern, not built)
              calibration record
                     |
                     v
   later evaluators (item 6) and planners (item 7) --
   informed, never automatically governed
```

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| Current evidence | Retired as accurate — every named artifact is real; two observed-side sources are themselves still-pending designs, not gaps this idea introduces. |
| Required consequence: predicted side | Retired — confirmed `SUPPLIES` from item 6, agreed independently by both ideas. |
| Required consequence: observed side | Retired as decomposed, not bundled — actual scope/failures/validation/review burden (pending: `incremental-terminal-accounting-projection.md`, reclassified away from `review-finding-bridge.mjs`), route revisions (pending: Candidate Trajectory remediation-delta ideas). |
| Required consequence: prediction↔outcome correspondence/comparison | **Not retired; partially sequenced, partially unresolved.** Upstream sources must land first, but the typed contract establishing whether a prediction and an observation are even comparable is itself a required, unowned piece — not automatic once both sides exist. |
| Required consequence: attributed calibration diagnosis | **Not retired. Primary residue.** Reframed from "causal judgment of why the prediction was wrong" — must support multiple possible contributing explanations with their own evidence/confidence/limitations, and must not treat an adverse outcome as automatic proof the prediction was wrong. No owner or record shape found anywhere. |
| Does not own | Retired — each exclusion already has a confirmed owner from this queue. |
| Important boundary (don't flatten causes into one score) | Retired as a correct constraint on the residue, not a gap itself. |
| Relationship to `evidence-calibrated-plan-resolution-and-continuous-capability-learning.md` | Retired as non-competing — distinct calibration target (execution-strategy learning vs. proposal-prediction-accuracy learning), already kept separate by `candidate-trajectory-upstream-amendments.md`. |

## Recommended status change to the idea file

Update `ideas/closed-loop-engineering-learning.md`'s Status section to note
that this reconciliation exists; that the predicted side is confirmed
supplied by `ideas/evidence-backed-proposal-evaluation.md` (item 6); that the
observed side (including review burden, reclassified away from review
findings) decomposes into two still-pending designs
(`incremental-terminal-accounting-projection.md`,
`candidate-trajectory-remediation-delta-for-native-review.md`/
`candidate-trajectory-builder-side-consumption.md`) this idea should consume
once they land rather than duplicate; and that the primary remaining open
scope is a prediction↔outcome correspondence/comparison contract (partially
sequenced, partially a genuinely unowned typed-comparability question) and
an attributed calibration diagnosis supporting multiple possible
contributing explanations — never a single causal verdict, and never
treating an adverse outcome as automatic proof a prediction was wrong — plus
a calibration-record domain profile (coupled to `claim-evidence-service.md`'s
existing pattern) to hold the comparison and that diagnosis together.
