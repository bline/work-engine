# Deterministic Refactor Pressure from Work Engine Evidence

## Status

Architectural idea / derived-analysis proposal.

Candidate amendments adding a proposed Candidate Trajectory primitive as a
within-slice Rework input (section 6.3) — while leaving Historical Recurrence
(section 6.5) owned by cross-slice aggregation, unchanged — are collected in
the [Candidate Trajectory upstream amendments](candidate-trajectory-upstream-amendments.md).
Not accepted or applied.

The purpose is to determine when implementation structure is creating enough repeated friction that refactoring deserves consideration, while adding little or no inference burden to execution agents.

The proposed mechanism does **not** ask a builder whether code is ugly, complex, or in need of refactoring. It derives refactor pressure after execution from immutable Work Engine evidence.

```yaml
idea_status:
  architectural_supersession: not_applicable
  architectural_supersession_note: "Corrected 2026-09-17, per unknown-drain campaign: close-read all 16 sections. SS1-4 ground the proposal in existing skills/-layer evidence (Code Change Profile, slice-supervisor receipts, telemetry ingress) without making any new ownership claim -- they cite what already exists. SS6-15 (the five pressure-dimension scoring formulas, the proposed derivation pipeline, weighting/admission/falsification mechanics) are deterministic-analytics detail entirely within the skills/code-change-profile and slice-supervisor service layer, the same register as the Candidate Trajectory family and structural-plan-ir-addendum-execution-profile-scoring-and-strategy-selection.md -- no dimension, mechanism, or substrate is a plausible owner for a scoring-formula proposal built entirely on top of already-existing App Server skills. SS16's 'architectural conclusion' is a positioning summary of the same analytics content, not a new claim. not_applicable, not none: this was a real read of every section (not a term grep), and it found no architectural-domain claim to check coverage for in the first place."
  residue: none
  residue_note: "Corrected 2026-09-17, item-level drain: the full-document close-read the prior note called for was completed as part of the supersession recheck (all 16 sections, see architectural_supersession_note above), plus a dedicated keyword scan for open-question markers (remains/should/needs/future/recommended/unresolved/undecided/TBD) across the whole document. Zero genuine open architectural questions found -- the few hits are either a receipt-schema field name ('unresolved concerns') or descriptive prose about what the pressure score informs later ('future remediation/refactors'), not live questions. Consistent with architectural_supersession: not_applicable: a pure scoring-formula proposal has no dimension/mechanism/substrate stake to leave unresolved."
  backlog: present
  backlog_ledger: "'Recommended first experiment' (SS15, Phase A-D) tagged [PLAN: OPEN] on direct evidence -- zero real fixtures, backfill runs, or campaigns found anywhere."
  audit_scope:
    - staged-plan-section
    - keyword-scan: full_document
  audit_scope_completeness: complete
  audit_scope_completeness_note: "Corrected 2026-09-17: full document now close-read section by section (supersession recheck) plus a dedicated residue keyword scan; zero UNCHECKED tags remain anywhere in the document."
  status_as_of: 2026-09-17
```

```yaml
idea_provenance:
  origin: direct_capture
  related_reconciliations:
    - candidate-trajectory-upstream-amendments.md (proposes SS6.3 amendment, not accepted or applied)
```

The central question is:

> Does a structural region repeatedly impose more implementation cost, spread, rework, or validation burden than the bounded change being executed would predict?

That question is unusually well matched to evidence Work Engine already records.

---

## 1. Existing architectural foundation

Work Engine already contains most of the evidence plane needed for a first implementation.

### 1.1 Code Change Profile is already the physical change characterization layer

`skills/code-change-profile/` implements a deterministic analyzer over immutable slice-checkpoint subjects.

Its current contract deliberately derives physical observations rather than semantic judgments:

- changed files;
- additions and deletions;
- diff hunk count;
- file categories;
- test-file count;
- documentation-file count;
- configuration-file count;
- bounded changed Python symbols;
- top-level module distribution.

The analyzer reconstructs the change from immutable baseline and result trees, validates the checkpoint through the checkpoint owner, verifies the task-patch digest, and generates canonical content-addressed output.

Source evidence:

- `skills/code-change-profile/references/profile-contract.md`
- `skills/code-change-profile/scripts/code_change_profile.py`
- `skills/code-change-profile/schemas/change-profile-v2.schema.json`

This matters because Refactor Pressure should not invent another representation of change shape. Code Change Profile is already the correct lower-level observation layer.

The accepted baseline plan is even more explicit. `planning/code-change-characterization-baseline/slice-01-accepted-plan.md` excluded from its first slice:

- semantic classification;
- historical graph comparison;
- profile aggregation;
- correlation analysis;
- causal claims;
- a universal complexity score.

It also explicitly left `structural_graph`, `invariant_catalog`, and `classifier` unused in the current profile provenance.

Refactor Pressure therefore belongs naturally **above** this baseline rather than inside it.

---

## 2. Work Engine also knows the execution context of the change

Ordinary static-analysis systems see a diff.

Work Engine can see both the diff and the execution envelope that produced it.

The durable slice receipt identifies:

- `run_id`;
- `slice_number`;
- slice title and goal;
- accepted/stopped/failed status;
- plan acceptance;
- validation results;
- placement evidence;
- workflow route;
- route revisions;
- validation breadth;
- review findings and remediation;
- deferred scope and unresolved concerns;
- checkpoint identity.

Source:

`skills/slice-supervisor/references/receipt-schema.md`

The receipt schema specifically recommends normalized engineering fields including:

- `changed_file_count`;
- `test_totals`;
- `review_findings`;
- `review_fix_iterations`;
- engineering input/output/context measurements;
- workflow route and route revisions;
- validation breadth;
- reasoning escalation and replacement counts;
- exploration outside configured evidence;
- slice wall-clock time;
- anomalies and deferred scope.

That means Refactor Pressure can be computed as a **join between structural change observations and execution observations**, rather than asking an agent to synthesize those facts.

---

## 3. Authoritative runtime telemetry is already separated from model self-report

The telemetry ingress boundary is especially useful.

`skills/slice-supervisor/references/telemetry-ingress.schema.json` binds telemetry to both `run_id` and `slice_id`, along with corroborated builder-runtime identity.

It currently records authoritative measurements for:

- input tokens;
- cached input tokens;
- output tokens;
- reasoning tokens;
- peak context when available;
- turn count;
- wall-clock duration.

Each measurement explicitly distinguishes `observed` from `unavailable` and retains event provenance.

`skills/slice-supervisor/scripts/harvest_codex_telemetry.py` reconstructs these values from the actual rollout rather than accepting builder estimates. Interrupted turns are represented explicitly rather than silently synthesized.

This is important for two reasons.

First, execution drag can be measured without adding another reporting burden to the builder.

Second, inference-cost measures can remain corroborating evidence rather than primary structural evidence because Work Engine can distinguish missing data, interruptions, provider failures, and other confounds.

---

## 4. Checkpoint manifests give Refactor Pressure a truthful change boundary

The slice-checkpoint contract gives an even stronger primitive.

A checkpoint binds:

- immutable baseline and result commits/trees;
- task patch digest;
- run, slice, and attempt;
- plan version;
- scope revision;
- gate receipt;
- explicit path manifest.

Each manifest path also carries an attribution:

- `task_owned`;
- `user_owned_baseline`;
- `pre_existing_overlap`;
- `generated_dependency`;
- `validation_dependency`.

Source:

`skills/slice-checkpoint/references/checkpoint-schema.md`

This is more useful for refactor analysis than raw Git churn.

For example, a change touching ten files is not automatically suspicious. But a change whose bounded task-owned implementation requires several generated or validation dependencies, repeatedly crosses modules, or expands beyond the normal footprint for similar slices may be structurally interesting.

The checkpoint therefore provides the correct immutable unit of analysis.

---

# 5. Proposed derived concept: Refactor Pressure

Refactor Pressure should measure **accumulated empirical friction associated with structural coordinates**.

It should not attempt to answer:

> Is this code good?

It should answer:

> When work passes through this structural region, does implementation repeatedly become disproportionately expensive or diffuse?

A first model can remain deterministic:

\[
RPS(c,t)=
w_aA +
w_sS +
w_rR +
w_vV +
w_hH
\]

for structural coordinate \(c\) through observation time \(t\).

A structural coordinate initially does not need to be more sophisticated than a repository path/module. Later versions can use symbols or graph nodes.

---

## 6. Pressure dimensions

### 6.1 Change Amplification — A

Change Amplification measures physical implementation footprint.

The Code Change Profile already provides:

- file count;
- changed-symbol count where supported;
- additions/deletions;
- hunk count;
- module distribution;
- test/config/documentation distribution.

A simple initial form could be normalized against historical peer changes:

\[
A =
\frac{\text{observed change footprint}}
{\text{expected footprint for comparable accepted slices}}
\]

No semantic interpretation is required initially.

Useful signals include:

- number of task-owned files;
- number of modules crossed;
- hunks per changed symbol;
- changed files per bounded slice;
- source/test/configuration spread.

Absolute size should not itself imply pressure. The useful signal is **unusual amplification relative to comparable work**.

---

## 6.2 Scope Spill — S

Scope Spill measures how far physical implementation extends beyond the core task-owned region.

Checkpoint attribution already distinguishes task-owned change from:

- generated dependencies;
- validation dependencies;
- pre-existing overlap.

The durable slice also retains plan version, scope revision, placement evidence, deferred scope, and route revision information.

Potential deterministic observations include:

\[
S =
f(
\text{module spread},
\text{dependency-attributed paths},
\text{scope revisions},
\text{route revisions}
)
\]

A one-time scope revision should not imply bad structure.

Repeated expansion around the same structural region across unrelated slices is the stronger signal.

---

## 6.3 Rework — R

Rework measures repeated work required before the slice reaches an accepted immutable checkpoint.

Some evidence already exists directly:

- review-fix iterations;
- review findings;
- candidate attempts;
- route revisions;
- stopped/failed attempts when histories are joined;
- reasoning/replacement counts where populated.

Additional deterministic rework evidence could later be derived from successive candidate checkpoint trees without inspecting chain-of-thought.

For example:

- same path changed across multiple candidate attempts;
- same symbol repeatedly modified before acceptance;
- accepted result reversing an earlier candidate change;
- repeated remediation concentrated in the same module.

This dimension should count observable revision behavior, not infer why the model changed its mind.

---

## 6.4 Validation Drag — V

Work Engine receipts already preserve configured validation requirements and their outcomes, validation breadth, semantic-test results, review findings, and remediation.

Validation Drag should ask:

> How much verification disturbance accompanies otherwise bounded work in this region?

Possible observations:

- failed validation stages before terminal acceptance;
- number of remediation cycles;
- required validation dependencies;
- late semantic rejection;
- review findings per changed source unit;
- validation breadth exceeding the historical norm for comparable slices.

Provider/network failures must not count as architectural pressure.

The receipt schema already separates provider failures, infrastructure failures, fallbacks, and workflow routing, making this exclusion practical.

---

## 6.5 Historical Recurrence — H

Historical recurrence is what turns friction into refactor pressure.

A difficult feature should not condemn its implementation region because it was difficult once.

Instead, pressure should increase when **independent slices repeatedly encounter the same structural coordinates** and show elevated A/S/R/V values.

A simple version could use:

\[
H_c = \sum_i d(t_i)\,P_{i,c}
\]

where:

- \(P_{i,c}\) is pressure attributable to coordinate \(c\) in slice \(i\);
- \(d(t_i)\) is a recency-decay function.

The trigger should therefore depend both on magnitude and recurrence.

Example:

> Do not raise a refactor candidate because one slice gives `module-x` a pressure score of 0.91.

Prefer:

> Raise a candidate because `module-x` has landed in the upper historical pressure quantile across four independent accepted slices spanning three different goals.

That is much harder to confuse with feature difficulty.

---

# 7. Execution Drag should be validation, not the primary score

The authoritative telemetry layer makes another measurement possible:

\[
D =
f(
\text{turn count},
\text{wall time},
\text{input tokens},
\text{output tokens},
\text{reasoning tokens}
)
\]

But these values are contaminated by:

- provider differences;
- model capability;
- reasoning effort;
- infrastructure failures;
- context lifecycle events;
- task difficulty.

Therefore Execution Drag should initially remain **outside the structural RPS equation**.

Instead it can test whether RPS predicts real execution cost.

For example:

\[
D_{normalized}
=
\frac{D_{observed}}
{\operatorname{median}(D_{\text{comparable slices}})}
\]

Then ask:

> Do high-RPS structural regions systematically produce high normalized execution drag?

If so, Work Engine has empirical evidence that the structural score captures something operationally meaningful.

This is more convincing than defining complexity by token expenditure.

---

# 8. Proposed architecture

Refactor Pressure should be a derived consumer, approximately:

```text
accepted/stopped slice checkpoint
          │
          ▼
 deterministic Code Change Profile
          │
          ├─────────────┐
          │             │
          ▼             ▼
   audit receipt   authoritative
   + validation     telemetry
          │             │
          └──────┬──────┘
                 ▼
       pressure observation
                 │
                 ▼
      historical aggregation
                 │
                 ▼
      structural pressure map
                 │
                 ▼
      refactor candidate signal
```

The builder is not in this derivation path.

The supervisor does not need to decide whether refactoring is warranted.

The post-hoc analyzer consumes already durable evidence.

This preserves the current Work Engine preference for structural evidence over agent judgment.

---

# 9. The existing Code Change Profile should remain unchanged initially

There is a strong architectural reason not to put RPS directly into `code-change-profile`.

The current profile contract deliberately owns **physical observations** and explicitly does not own semantics, architecture, outcomes, or policy.

That is a clean boundary.

Refactor Pressure is longitudinal and comparative. It needs multiple profiles plus execution records.

Therefore the relationship should be:

```text
code-change-profile
    ↓
physical immutable observations

refactor-pressure
    ↓
cross-slice derived historical analysis
```

This also allows Code Change Profile to evolve independently.

---

# 10. First version can be very cheap

Version 1 does not need graph integration, semantic classification, or new model calls.

It can operate entirely from:

1. accepted/stopped checkpoint receipts;
2. Code Change Profiles;
3. schema-v5 terminal audit receipts;
4. telemetry ingress already projected into those receipts.

A first per-coordinate record could look conceptually like:

```json
{
  "coordinate": "skills/slice-supervisor",
  "slice": {
    "run_id": "...",
    "slice_number": 12
  },
  "observations": {
    "files_touched": 7,
    "modules_crossed": 3,
    "candidate_attempts": 2,
    "scope_revisions": 1,
    "review_fix_iterations": 2,
    "validation_failures": 1
  },
  "pressure_components": {
    "amplification": 0.71,
    "scope_spill": 0.55,
    "rework": 0.63,
    "validation_drag": 0.48
  }
}
```

Historical aggregation can then produce:

```json
{
  "coordinate": "skills/slice-supervisor",
  "independent_slices": 7,
  "elevated_pressure_slices": 4,
  "pressure_quantile": 0.92,
  "trend": "rising",
  "candidate_state": "observe"
}
```

No inference is required.

---

# 11. Avoid fixed weights at first

There is no good reason yet to assert that, for example,

\[
0.30A + 0.20S + 0.25R + 0.25V
\]

represents true refactor need.

Work Engine now has enough durable history to calibrate rather than guess.

A safer first phase is to retain the component vector:

\[
P=[A,S,R,V]
\]

and derive percentile/rank information independently.

Then examine whether elevated component patterns predict:

- later execution drag;
- future remediation;
- repeated spill;
- future refactors;
- reduced pressure after refactoring.

Weights can be introduced only when evidence supports them.

This preserves the distinction between **measurement** and **policy**.

---

# 12. Refactor candidate admission

The system should initially produce candidates, not commands.

A conservative candidate rule could require all of:

1. at least three independent accepted slices touching the coordinate;
2. elevated pressure in at least two pressure dimensions;
3. recurrence across more than one slice goal or campaign;
4. no evidence that provider/infrastructure failure explains the elevation;
5. sufficient Code Change Profile coverage.

Possible states:

- `insufficient_history`
- `normal`
- `observe`
- `candidate`
- `post_refactor_observation`

This avoids turning a noisy score into architecture authority.

---

# 13. Post-refactor falsification

The strongest part of the design is that a refactor creates a testable prediction.

If the diagnosis was correct, subsequent comparable work through the affected structural region should show reductions in some combination of:

- change amplification;
- scope spill;
- rework;
- validation drag;
- execution drag.

Therefore a refactor should establish a revision boundary and compare pre/post observations.

Conceptually:

\[
\Delta P =
P_{\text{post}}
-
P_{\text{pre}}
\]

A useful refactor should produce negative pressure over subsequent comparable work.

If pressure remains unchanged, the original diagnosis was weak.

If it increases, the refactor may have worsened structural friction.

This makes Refactor Pressure falsifiable rather than aesthetic.

---

# 14. Later graph integration

The current Code Change Profile explicitly records the structural graph as `not_used / deferred_by_profile_scope`.

That is appropriate for the baseline, but the longitudinal pressure consumer is where graph information could later become valuable.

A later version could project pressure from paths/symbols onto Codebase Memory graph coordinates and derive:

- pressure concentration around high fan-in nodes;
- pressure propagation across dependency edges;
- repeated cross-boundary change paths;
- strongly connected pressure regions;
- change-radius expansion;
- structural neighborhoods whose downstream blast radius exceeds historical expectation.

At that point the unit of concern can move from:

> this directory is hot

toward:

> this dependency boundary repeatedly amplifies otherwise localized changes.

That is a much more useful refactor target.

But graph integration is not necessary to prove the idea.

---

# 15. Recommended first experiment

**[PLAN: OPEN — 2026-09-16. No durable execution evidence found within the surfaces checked: `app-server/src` and `planning/` (grepped for "historical backfill", "Refactor Pressure Score", "deterministic component observations" — zero hits), and Work Engine campaign/worktree state under `/home/bline/.local/state/work-engine` (no workstream named for this plan).]**

Rather than immediately making RPS part of production decision-making:

### Phase A — historical backfill

Run Code Change Profile where immutable checkpoint evidence permits it and join results to existing receipts.

Compute only deterministic component observations.

Do not emit refactor recommendations.

### Phase B — inspect distributions

Measure:

- component distributions;
- recurrence by coordinate;
- correlations among A/S/R/V;
- association with normalized execution drag;
- association with later remediation or repeat modification.

### Phase C — identify candidate historical cases

Select regions with repeated high pressure.

Blindly inspect whether those regions correspond to places where implementation history already suggests architectural trouble.

### Phase D — prospective validation

Begin emitting `observe` candidates prospectively.

When an actual refactor occurs, create a structural revision boundary and measure subsequent pressure.

Only after this should Work Engine consider a single composite score or automatic threshold.

---

# 16. Architectural conclusion

This does not need to be another responsibility placed on the builder.

In fact, doing that would discard one of Work Engine's strongest advantages.

The system already possesses:

- immutable bounded change subjects;
- attributed path manifests;
- deterministic physical change characterization;
- plan/scope identity;
- durable validation outcomes;
- review/remediation observations;
- workflow revisions and fallbacks;
- authoritative runtime telemetry;
- persistent cross-slice history.

Those are exactly the ingredients needed to ask whether architecture is producing repeated observable friction.

The clean abstraction is therefore:

> **Refactor Pressure is a historical derived property of structural coordinates, calculated from the consequences of executing bounded changes through them.**

It is not an opinion about code quality.

It is not a builder report.

It is not a universal static complexity metric.

And it does not need inference in its first useful form.

Work Engine can observe the pressure first, accumulate it over time, and use subsequent work—including subsequent refactors—to test whether the signal actually predicts anything.