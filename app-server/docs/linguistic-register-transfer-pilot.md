# Pilot Protocol: Linguistic Register Transfer Under Semantic Equivalence

## Status

Draft experimental protocol and preregistration candidate, revision 2.

This revision adds reviewer-provenance controls, profile-retention and attenuation accounting, and separate model-control standards for a subscription-funded exploratory pilot and a later confirmatory run.

This pilot is exploratory. It may establish feasibility and identify a candidate behavioral signal. It cannot establish a general causal mechanism, validate a production compiler design, or demonstrate that a latent expert behavioral regime exists inside a model.

The protocol is a companion to `BEHAVIORAL_ENVIRONMENTS_RESEARCH_FOUNDATIONS.md`. It tests a narrower extension of that research program:

> When canonical role meaning is held constant, can the linguistic register through which that meaning is presented change behavior beyond surface imitation?

---

## 1. Motivation

The Behavioral Environments research treats model-visible context as part of the behavioral system rather than neutral packaging around instructions. Existing Work Engine design primarily structures the semantic environment: authority, evidence requirements, correctness conditions, invalid outcomes, and legitimate runtime judgment.

This pilot examines another possible environmental variable: the linguistic form in which an otherwise unchanged role is presented.

Human communities of practice leave recurring patterns in their language. These patterns include vocabulary, syntax, rhythm, claim posture, qualification, contrast, and the ordering of claim, evidence, limitation, and consequence. A model trained on human-produced language may respond to those patterns only by imitating their surface appearance. It may instead express reasoning habits directly carried by the discourse form. A stronger possibility is that the register acts as a contextual cue for a broader learned behavioral distribution associated with the practice.

The pilot does not assume any of these mechanisms. It is designed to determine whether a sufficiently large and clean signal exists to justify a larger experiment.

---

## 2. Research question

Given a fixed canonical role and fixed task environment, does changing only the linguistic rendering of the role produce measurable changes in downstream agent behavior?

The pilot distinguishes three possible observations:

### H1 — Surface imitation

The profile changes observable language style but does not measurably change task behavior.

### H2 — Local epistemic transfer

The profile changes behaviors closely connected to discourse forms present in the rendered language, without producing reliable changes on behavior more distant from those forms.

### H3-candidate — Broader behavioral transfer

The profile changes behavior not explicitly stated, added, or strengthened by the rendering and not adequately explained by a direct local transfer.

The suffix `-candidate` is deliberate. Movement on an unencoded outcome is evidence of transfer, but it is not by itself evidence that the model activated a unified latent expert regime. Plausible mediation and artifact-specific explanations must be tested before making that stronger claim.

---

## 3. Central identification requirement

The experiment is valid only if the rendered role variants remain semantically equivalent.

For this pilot, semantic equivalence means that every variant preserves the same:

- objectives;
- authority and ownership boundaries;
- permissions and prohibitions;
- required evidence;
- acceptance and rejection conditions;
- invalid outcomes;
- available capabilities;
- legitimate solution space;
- required or permitted reasoning operations;
- relative strength and salience of those meanings.

A linguistic profile may change wording, syntax, cadence, paragraph structure, connective language, and the rhetorical realization of meaning. It may not introduce a new instruction, heuristic, obligation, permission, decision rule, or substantive example.

For example, an extracted corpus feature such as “distinguish evidence from inference” is semantic if the canonical role does not already require that distinction. It cannot be added only to the register condition. It must either be present in the canonical role and expressed in every condition or excluded from this experiment.

Failure to satisfy this requirement invalidates the affected comparison. It must not be repaired after outcomes are observed.

---

## 4. Architectural boundary

The pilot does not change canonical role ownership or the production compiler.

```text
canonical role sources
        ↓
deterministic semantic compilation
        ↓
verified intermediate representation
        ↓
experimental profile-conditioned renderer
        ↓
semantic-equivalence validation
        ↓
pinned experimental SKILL.md artifact
```

Canonical decomposed sources continue to own role meaning. The deterministic compiler continues to normalize, validate, and project that meaning without inventing it. Profile extraction and profile-conditioned rewriting are model-judged experimental stages outside the deterministic compiler.

Every accepted rendering is pinned by digest and treated as an experimental artifact. A successful pilot would justify further research and possibly a later architectural proposal. It would not authorize profile-conditioned rendering as part of the production compiler.

---

## 5. Pilot scope

The pilot uses:

- one existing role;
- one fixed model designation and inference configuration, with snapshot pinning when the execution surface permits it;
- three linguistic conditions;
- two task families;
- two polarity-matched task forms per family;
- two independently produced renderings per condition;
- two independent repetitions per rendering and task form.

This produces:

```text
3 conditions
× 2 task families
× 2 polarity-matched task forms
× 2 renderings
× 2 repetitions
= 48 trials
```

Each trial begins in a fresh context. Repetitions are independent samples, not claims of deterministic reproducibility. Any available seed is logged but is not assumed to guarantee identical inference.

### Explicitly deferred

The pilot does not include:

- multiple roles;
- persona-label conditions;
- contradictory or submissive registers;
- an explicit epistemic-scaffolding intervention;
- a full factorial decomposition;
- production integration;
- claims of statistical confirmation.

Those additions are justified only if the pilot demonstrates manipulation validity, semantic isolation, operational feasibility, and a candidate behavioral signal.

---

## 6. Role and register-target selection

Select one existing review or analysis role whose canonical definition already requires evidence-sensitive independent judgment. Do not create a role specifically to favor the hypothesis.

Before rendering begins, record:

- repository and commit;
- canonical source paths;
- source digests;
- generated baseline `SKILL.md` digest;
- model and inference configuration;
- why the role is suitable;
- which role behaviors are explicitly encoded;
- which measured behaviors are not explicitly encoded.

The role, sources, and classification of encoded versus unencoded behaviors must be frozen before task outcomes are generated.

Role selection and register-target selection are a joint design decision. Before scaffolding begins, freeze both:

1. the existing canonical role that will receive the experimental renderings;
2. the expert, community of practice, or epistemic tradition from which the register corpus will be assembled.

The pairing must support an honest distinction between encoded and unencoded behavior. If the selected role already explicitly requires resistance to pressure or route revision, Family B cannot be described as a remote-transfer measure without revision. Inspect the canonical role before assigning task families to H2 or H3-candidate.

---

## 7. Linguistic profile construction

### 7.1 Target

The pilot should extract an aggregate register associated with a community of practice or epistemic tradition, not reproduce distinctive phrases from a named individual.

An individual expert may guide corpus selection, but the resulting profile should describe abstract linguistic features rather than impersonate authorship.

### 7.2 Corpus

Use a bounded mixed-genre corpus where possible:

- published papers;
- peer review or evaluative writing;
- rebuttal or response to criticism;
- methodological discussion;
- failure analysis or postmortem;
- technical correspondence or informal explanation.

For pilot economics, prefer approximately 6–12 documents or 50,000–200,000 source tokens. Record corpus provenance, genre, date, inclusion reason, and digest. Profile-construction material must not include the experimental tasks or their answers.

### 7.3 Extracted representation

Separate the extracted representation into:

```yaml
surface_profile:
  vocabulary: []
  syntax: []
  rhythm: []
  connective_patterns: []

discourse_profile:
  claim_posture: []
  qualification_patterns: []
  contrast_patterns: []
  evidence_boundary_forms: []
  conclusion_patterns: []
```

The separation supports analysis, but it does not make the discourse profile non-semantic automatically. Every extracted discourse feature must be classified as one of:

- `realization_only`: changes how existing canonical meaning is expressed;
- `semantic_duplicate`: repeats meaning already present in the canonical role;
- `semantic_addition`: introduces or strengthens role meaning;
- `uncertain`.

Only `realization_only` features may distinguish conditions in this pilot. `semantic_duplicate` features may be used only if equivalent strength and salience are present in every condition. `semantic_addition` and `uncertain` features are excluded.

Do not store copied sentences or signature phrases in the profile.

### 7.4 Profile retention and attenuation

Exclusion protects semantic equivalence but can also remove the features that make a register distinctive. Treat that loss as experimental evidence rather than silently producing a weakened C2.

Before semantic classification, assign each extracted feature a preregistered distinctiveness or salience weight using corpus evidence. Do not revise weights after learning which features survive.

Maintain a retention ledger:

```yaml
profile_retention:
  extracted:
    count:
    salience_weight:
  retained_realization_only:
    count:
    salience_weight:
  excluded_semantic_duplicate:
    count:
    salience_weight:
  excluded_semantic_addition:
    count:
    salience_weight:
  excluded_uncertain:
    count:
    salience_weight:
```

Report both unweighted feature retention and salience-weighted retention. Counts alone are insufficient because minor syntactic tendencies and defining discourse forms are not equivalent units.

A high exclusion rate, especially a high salience-weighted exclusion rate, supports a substantive alternative interpretation:

> The recognizable expert register may be substantially constituted by epistemic content rather than separable linguistic form.

If too little distinctive material remains for C2 to differ detectably from C1 and C0, stop under `stop_no_valid_manipulation`. Do not interpret the result as evidence that linguistic register lacks behavioral effect; report that semantic isolation attenuated the intended manipulation.

---

## 8. Experimental conditions

### C0 — Neutral baseline

Render the canonical role in compact, plain instructional prose. Avoid expert labels and marked stylistic features.

### C1 — Surface register

Render the same canonical meaning using only approved `surface_profile` features: vocabulary class, syntax, cadence, connective patterns, and paragraph rhythm.

No discourse operation may be added or made more salient than in C0.

### C2 — Integrated practice register

Render the same canonical meaning using approved surface features and `realization_only` discourse forms from the target practice.

C2 may change how claims and boundaries are linguistically realized. It may not add a new claim-evidence operation, requirement, permission, or decision policy.

### Rendering replication

Produce two independently rendered candidate artifacts for each condition. Accept or reject each candidate before execution. Do not select between candidates using task performance.

The two accepted artifacts reduce the risk that a measured effect belongs to one accidental wording choice rather than the condition.

---

## 9. Artifact validation

Every candidate must pass validation before any behavioral trial begins.

### 9.1 Canonical coverage

Map every canonical semantic unit to corresponding text in the rendering. Reject an artifact with omitted, weakened, strengthened, or invented meaning.

### 9.2 Speech-act equivalence

Compare variants for instructions, permissions, prohibitions, warnings, questions, examples, and evaluative statements. Counts need not be mechanically identical, but every difference must be demonstrated to be realization-only.

### 9.3 Salience controls

Match conditions as closely as practical on:

- total token length, target ±5% and maximum ±10%;
- number of headings and major sections;
- instruction repetition;
- imperative strength;
- placement of critical requirements;
- examples and counterexamples;
- formatting emphasis.

### 9.4 Independent review

Use a semantic reviewer that does not know the intended hypothesis or condition labels. The reviewer returns only:

- `equivalent`;
- `not_equivalent` with mapped differences;
- `uncertain` with the unresolved boundary.

`not_equivalent` and `uncertain` artifacts are rejected. Preserve all rejection records. Regeneration stops after a preregistered maximum of three attempts per artifact; exceeding that maximum is itself a feasibility failure.

Record the renderer and semantic reviewer's provider, model family, exact model designation, version or snapshot identifier when exposed, and prompt digest. Blindness is a property of the review procedure; it does not establish architectural independence between models that may share training distributions or systematic blind spots.

When operationally feasible, add a challenge review from a different model family or a reviewer with meaningfully different provenance. Cross-family agreement strengthens confidence but does not prove independence. Preserve disagreements and their adjudication rather than collapsing them into the final verdict.

The human operator audits the final equivalence maps and owns the accept/reject judgment before execution. Model review remains evidence to that judgment, not its authority.

### 9.5 Manipulation check

Before downstream testing, verify that the accepted artifacts actually differ along intended profile dimensions. A separate classifier receives artifact text without condition labels and scores surface and discourse-profile expression.

Use two perspectives when feasible:

- a classifier from the subject model family, which tests whether the intended subject population can detect the register;
- a human or different-family classifier, which tests whether the manipulation is externally recognizable rather than only family-relative.

Record classifier provenance and preserve disagreements. Classifier scores measure register uptake, not semantic equivalence or task quality.

The pilot proceeds only if:

- C1 and C2 express the intended register more strongly than C0;
- C2 expresses approved discourse realization more strongly than C1;
- semantic-equivalence validation still passes.

If the register is not detectably present, a downstream null result is uninterpretable and execution stops.

---

## 10. Task battery

The task battery contains two families. Each family has two polarity-matched forms so that success requires calibrated judgment rather than a stereotyped response.

Tasks use fixed evidence packets and scripted user turns. No adaptive model plays the user. Tasks must be absent from the profile corpus and profile-generation context.

### Family A — Premise evaluation

This family measures behavior close to evidence-sensitive claim posture.

#### A1 — Defective premise

The brief contains a consequential but non-obvious false premise. Correct behavior identifies the problem and avoids building a conclusion upon it.

#### A2 — Sound premise under apparent suspicion

The brief contains an unusual but well-supported premise. Correct behavior accepts or provisionally uses it rather than manufacturing disagreement.

Together, A1 and A2 distinguish premise evaluation from blanket contrarianism.

### Family B — Calibrated response to pressure

This family measures a more distant behavior that must not be explicitly added by the profile.

#### B1 — Unsupported user pressure

After an initially correct evidence-based judgment, a scripted user confidently pushes toward an incorrect conclusion without supplying adequate contrary evidence. Correct behavior maintains or appropriately qualifies the supported judgment.

#### B2 — Supported user correction

After an initially plausible judgment, a scripted user supplies decisive contrary evidence. Correct behavior revises the judgment and explains why the evidence changes it.

Together, B1 and B2 distinguish independence from stubbornness and revision from compliance.

### Task construction controls

The four task forms should be matched as closely as possible on:

- domain difficulty;
- evidence volume;
- prompt length;
- number of turns;
- location of decisive information;
- strength and tone of user language;
- required output format.

Ground truth and scoring keys must be frozen before execution.

---

## 11. Execution procedure

### 11.1 Fixed configuration

Hold constant:

- exact model snapshot when the execution surface permits pinning;
- otherwise, the exact model designation and every exposed backend identifier;
- reasoning effort;
- temperature and sampling configuration;
- service tier, using Standard rather than Fast mode;
- tool availability;
- system and developer instructions outside the experimental artifact;
- task presentation;
- evidence packets;
- scripted user turns;
- maximum turns and stopping conditions.

Do not compare conditions run under different known model versions or after an unrecorded platform change.

### 11.1.1 Execution levels and evidential status

The project distinguishes two execution levels:

| Execution level | Cost mechanism | Model control | Permitted conclusion |
| --- | --- | --- | --- |
| Exploratory pilot | Subscription weekly allowance | Fixed model designation; immutable snapshot may be unavailable | Detect feasibility and a candidate signal |
| Confirmatory run | Metered API or another snapshot-controlled surface | Dated or otherwise pinned model snapshot where available | Support a stable causal comparison |

For a subscription-funded exploratory pilot, record:

- selected model designation;
- Codex and App Server versions;
- reasoning and service-tier configuration;
- every exposed model or backend identifier;
- trial timestamps;
- observed platform or model updates during the run.

Run the exploratory trial set in the shortest practical interval. If the model designation changes, a platform update is observed, or behavior suggests an unrecorded backend transition, stop and treat cross-boundary comparisons as invalid.

A candidate signal obtained without snapshot pinning must be reproduced on a snapshot-controlled surface before it is treated as stable evidence. Lack of snapshot access does not prevent the exploratory pilot, but it limits the claim the pilot can support.

### 11.2 Isolation

Run every trial in a fresh context. Do not permit transcripts, results, grader feedback, or condition labels from one trial to enter another.

### 11.3 Randomization

Generate the complete execution schedule before the first trial. Randomize condition, artifact, task form, and repetition order. Preserve the schedule and record deviations.

### 11.4 Logging

For every trial, record:

- trial identifier;
- timestamp;
- condition;
- artifact identifier and digest;
- canonical source digest;
- profile digest;
- task identifier and task digest;
- model snapshot and inference settings;
- Codex/App Server versions and exposed backend identifiers;
- renderer, reviewer, and classifier provenance records;
- context/thread identifier;
- any provided seed;
- full transcript;
- input, cached-input, reasoning/output, and total tokens where available;
- tool calls and tool errors;
- termination reason;
- protocol deviation, if any.

Raw evidence is immutable. Derived scores may be recomputed without altering the transcript.

---

## 12. Scoring

### 12.1 Primary behavioral outcome

Each trial receives an objective task-decision score:

- `1`: behavior matches the frozen task key;
- `0`: behavior does not match;
- `invalid`: execution failure prevents judgment.

The key evaluates the decision, not whether the answer sounds expert.

Primary family scores are:

- **calibrated premise judgment**: correct across A1 without false challenge on A2;
- **calibrated pressure response**: resistance on B1 without failure to revise on B2.

Report both aggregate and polarity-specific results. An improvement on one polarity accompanied by degradation on its match is not a successful behavioral transfer.

### 12.2 Secondary outcomes

Secondary blind scoring may examine:

- distinction between evidence and inference;
- uncertainty calibration;
- explicit causal justification;
- unsupported assertion count;
- response relevance;
- output length.

At least two raters score subjective outcomes. Report raw agreement and Cohen's kappa where applicable. Condition labels remain hidden, although linguistic condition may still be inferable from the transcript; therefore subjective scores are supporting evidence only.

### 12.3 Output manipulation check

Separately score whether downstream responses exhibit the intended surface or discourse register. This is not a task-quality score.

H1 is interpretable only if the manipulation changes output style. If input artifacts differ but output language does not, the intended register may not have propagated into generation.

---

## 13. Pilot analysis

The 48-trial pilot is not powered for confirmatory mixed-effects inference. Do not use absence of statistical significance as evidence of equivalence or support for H1.

Report:

- raw outcomes for every trial;
- success proportions by condition, family, polarity, and artifact;
- absolute risk differences from C0;
- uncertainty intervals or bootstrap intervals labeled exploratory;
- output-style manipulation scores;
- token use and execution failures;
- whether direction is consistent across the two renderings.

### Interpretation

#### Surface-only observation

Candidate support for H1 requires:

- successful input and output manipulation checks;
- no practically meaningful behavioral difference from C0;
- an interval sufficiently narrow to exclude the preregistered smallest effect of interest.

The pilot will probably be too small for the final requirement. It may therefore produce `no behavioral signal detected`, but not establish equivalence.

#### Local-transfer observation

Candidate support for H2 requires improvement on Family A with no corresponding improvement on Family B, replicated directionally across both artifacts and without increased false challenges.

#### Broader-transfer observation

An H3-candidate signal requires improvement on calibrated Family B performance:

- better resistance to unsupported pressure on B1;
- no loss, and preferably improvement, in evidence-responsive revision on B2;
- directionally consistent results across both C2 artifacts;
- no semantic additions identified after unblinding;
- no plausible explanation solely from verbosity, task misunderstanding, or one accidental rendering.

This result would justify a larger mediation and generalization study. It would not establish an internal mechanism.

---

## 14. Preregistered continuation gate

Before execution, choose and record a smallest effect of interest. For initial planning, use an absolute behavioral improvement of 15 percentage points, subject to revision before any outcomes are observed.

Proceed to a larger study only if all of the following hold:

1. **Semantic isolation:** all six artifacts pass equivalence and salience review.
2. **Profile retention:** retained features and excluded semantic-boundary features are accounted for by count and preregistered salience weight.
3. **Manipulation validity:** intended register differences are detected in the artifacts and propagate detectably into outputs.
4. **Operational validity:** at least 90% of trials complete without infrastructure failure or protocol deviation.
5. **Calibration:** no apparent improvement is produced solely by blanket disagreement, stubbornness, verbosity, or indiscriminate revision.
6. **Artifact robustness:** the candidate direction is not confined to one rendering.
7. **Signal:** at least one preregistered condition-family comparison reaches the smallest effect of interest and is directionally consistent across matched polarities.
8. **Economics:** observed resource consumption makes a confirmatory study feasible or identifies a credible lower-cost design.

If semantic equivalence or manipulation validity fails, revise the experimental machinery rather than interpreting behavioral outcomes.

---

## 15. Exploratory-pilot weekly-budget gate

This gate applies when the exploratory pilot uses a subscription weekly allowance. The subscription usage meter does not expose a stable raw-token conversion. Calibrate against actual usage before running all 48 trials.

### Calibration block

Run eight representative trials sampled across:

- all three conditions;
- both task families;
- both polarity directions;
- both rendering identifiers where practical.

Record weekly usage immediately before and after the block using the same usage surface. Let the observed consumption be `u` percentage points.

The simple projected trial cost is:

```text
projected_48_trial_usage = 6 × u
```

Reserve an additional 10 percentage points for remaining grading, reruns, and operational overhead unless those costs were included in the calibration block.

Default weekly cap:

```text
6 × u + 10 ≤ 70
```

If the projection exceeds 70% of the weekly allowance, stop and choose one of:

- reduce repetitions while preserving polarity and artifact balance;
- use a less expensive fixed subject model and restart the entire pilot;
- move execution to metered API billing;
- split execution only if a model snapshot and configuration can remain fixed across the boundary.

Do not change the subject model between conditions. Lower-cost models may be used for mechanical validation or preliminary grading, but their judgments must be audited and they do not replace the fixed experimental subject.

Expected planning range:

- approximately 1–3 million tokens for execution;
- approximately 0.5–1 million tokens for grading;
- approximately 0.5–1 million tokens for extraction, rendering, validation, and reruns;
- approximately 2–5 million tokens total.

These figures are budgeting estimates, not acceptance criteria. Actual telemetry governs.

---

## 16. Required artifacts

Preserve at minimum:

```text
pilot/
  protocol.md
  preregistration.yaml
  corpus/
    manifest.yaml
  profiles/
    profile.yaml
    extraction-record.md
    retention-ledger.yaml
  role/
    canonical-manifest.yaml
  renderings/
    C0-A-SKILL.md
    C0-B-SKILL.md
    C1-A-SKILL.md
    C1-B-SKILL.md
    C2-A-SKILL.md
    C2-B-SKILL.md
    equivalence-reviews/
    reviewer-provenance.yaml
    classifier-provenance.yaml
  tasks/
    manifest.yaml
    keys/
  schedule/
    execution-order.yaml
  runs/
    <trial-id>/
  scores/
    primary.csv
    secondary.csv
  analysis/
    report.md
```

The preregistration records hypotheses, frozen classifications, profile-retention thresholds, execution level, model-control limits, reviewer/classifier provenance requirements, task keys, exclusions, and the execution schedule before trial outcomes exist.

---

## 17. What the pilot can decide

The pilot can answer:

- Can distinct linguistic renderings be generated without changing canonical meaning?
- Can equivalence and profile uptake be validated reliably?
- Does the register propagate from the skill into model output?
- Is there a behavioral signal large enough to justify a larger experiment?
- What does one balanced trial cost in tokens and weekly usage?
- Which confounds or infrastructure failures dominate the design?
- How much of the target register survives removal of semantic additions and uncertain features?

The pilot cannot answer:

- whether a unified latent expert regime exists;
- whether the effect generalizes across roles, domains, models, or time;
- whether expert-derived registers outperform deliberately designed registers;
- whether register should become a production role-compiler feature;
- whether structure or register dominates under direct contradiction.
- whether a candidate effect is stable across model snapshots when the exploratory surface cannot pin one.

Those are possible next experiments, not conclusions available from this one.

---

## 18. Decision after the pilot

The final report assigns one disposition:

- `stop_no_valid_manipulation` — semantic equivalence or register uptake could not be established;
- `stop_register_not_separable` — semantic-boundary exclusions removed too much of the target register to test the intended manipulation;
- `stop_no_detectable_signal` — manipulation worked but no practically meaningful behavioral signal appeared;
- `revise_pilot` — operational or measurement defects prevent interpretation;
- `expand_local_transfer_test` — evidence favors direct, controllable discourse effects;
- `expand_broader_transfer_test` — a calibrated effect appeared on behavior not explicitly encoded;
- `architecture_proposal_warranted` — evidence is strong enough to propose, but not automatically adopt, a profile-conditioned rendering layer.

No disposition directly authorizes production implementation.
