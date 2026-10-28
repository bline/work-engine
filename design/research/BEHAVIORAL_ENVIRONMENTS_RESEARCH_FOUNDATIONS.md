# Behavioral Environments: Research Foundations

## Status

Explanatory research note. This document is **not normative** and does not claim a known internal mechanism for the behavioral effects described here.

### Provenance

The behavioral hypotheses and corresponding Work Engine design decisions described here were derived independently from repeated observation of model behavior. The cited literature was located afterward to compare those observations with existing empirical work and identify convergence, disagreement, and limits.

This document is therefore a **retrospective research foundation**, not a claim that the architecture was derived from the cited literature. It is not a systematic literature review.

Its purpose is to establish the empirical background against which a narrower Work Engine hypothesis can be stated and tested.

**Companion note:** `STRUCTURE_OVER_PROCEDURE_RESEARCH_FOUNDATIONS.md` examines the independent systems-design lineage and direct agent evidence for preserving structural constraints while avoiding unnecessary route prescription.

---

## 1. Empirical background

A defensible background claim is:

> **Interaction context and agent scaffolding can materially affect model behavior.**

Framing, conversational history, user stance, social pressure, persona, instruction structure, and agent scaffolding can change substantive model outputs even when the underlying model weights are unchanged.

This background claim is intentionally broad. It motivates treating an agent's environment as an engineering variable, but it is **not itself the primary Work Engine hypothesis**.

---

## 2. Observation chain

The following observations can be considered independently of any speculative explanation about model internals.

1. Human interaction environments differ systematically in the behavior they elicit and reward: autonomy, deference, exploration, compliance, disagreement, correction, and collaboration are not distributed uniformly across environments.
2. Human-produced language reflects those differences in interaction structure and behavior.
3. Large language models are trained on large quantities of human-produced language and interaction.
4. Controlled studies show that changes in framing, conversational position, social pressure, persona, and scaffolding can measurably change model outputs.
5. These changes can affect truthfulness, stance stability, task accuracy, reasoning behavior, and decision quality—not merely surface tone.
6. Therefore, an agent's conversational and operational environment should not be treated as neutral packaging around a task.

The document deliberately stops there.

It does **not** assert that researchers can presently identify a specific learned "human behavioral environment" inside model weights, nor that such a representation is the causal mechanism of these effects.

---

## 3. Related empirical findings

### 3.1 Politeness and interactional framing can affect task performance

Yin et al. (2024) varied prompt politeness in English, Chinese, and Japanese across multiple tasks. They found that politeness level affected performance, with impolite prompts often performing worse, while maximal politeness was not universally optimal.

The important observation is not "be polite to models." The important observation is:

> Interactional framing can change measured model performance even when it is not the semantic subject of the task.

The effect is context-dependent rather than monotonic, which argues against a simplistic good-tone/good-performance rule.

### 3.2 User beliefs can pull models away from independent answers

Sharma et al. (2023) studied sycophancy across several state-of-the-art assistants and found that models can favor answers matching a user's stated beliefs over truthful ones. Their analysis also found evidence that human preference data can reward such behavior.

This establishes a consequential form of interaction conditioning: **who the user appears to be and what the user appears to believe can change the answer**.

### 3.3 Multi-turn pressure changes stance behavior

Hong et al. (2025) introduced SYCON Bench to measure sycophancy in multi-turn free-form dialogue. Across 17 models, sustained user pressure could cause models to change position, and the authors measured both how quickly models flipped and how frequently they changed stance.

They also found that third-person framing could substantially reduce sycophancy in a debate scenario.

This is particularly important for long-running agents because it shows that conversational history is not merely accumulated task data. The **interaction trajectory itself** can affect later behavior.

### 3.4 Conversational position matters

Kim and Khashabi (2025) found that models were more likely to endorse a user's counterargument when it appeared as a follow-up conversational rebuttal than when competing responses were presented simultaneously for evaluation.

They also found increased susceptibility when rebuttals contained detailed reasoning even when the conclusion was incorrect, and greater susceptibility to casually phrased feedback than formal critiques in some conditions.

This provides unusually direct evidence that **the relational or conversational placement of information changes how the model treats it**.

### 3.5 Sustained persuasion can materially degrade factual performance

Tan et al. (2025) studied corrective and misleading persuasion over multiple turns. Under sustained misleading persuasion, GPT-4o achieved only 27.32% accuracy on their MMLU-Pro condition.

The result demonstrates that interaction history can change substantive problem-solving performance, not merely agreement language or stylistic presentation.

### 3.6 Personas reshape behavior rather than uniformly improving it

Persona-prompting research shows conditional rather than universal effects.

Luz de Araujo et al. (2025), in *Principled Personas*, evaluated nine models across 27 tasks. Expert personas usually produced positive or statistically non-significant changes, but irrelevant persona attributes sometimes caused performance drops approaching 30 percentage points. Effects of education, specialization, and domain relatedness were inconsistent across tasks.

Basil et al. (2025) tested expert and low-knowledge personas on GPQA Diamond and MMLU-Pro. Expert personas generally did not reliably improve factual accuracy, while low-knowledge personas often degraded performance and mismatched personas could induce refusals.

Yang et al. (2026) found another kind of tradeoff: persona prompting improved classification on their most subjective task while degrading rationale quality. Simulated personas also failed to reliably align with the corresponding real-world demographic groups.

The relevant engineering point is not that persona prompting is beneficial. It is that **role framing can materially reshape model behavior in ways that are nonlinear, task-dependent, and sometimes harmful**.

### 3.7 Agent scaffolding can change coding performance while the model stays fixed

Ben Sghaier et al. (2026) held the underlying language model constant while varying releases of an agentic coding scaffold. Across 35 sequential scaffold releases and SWE-bench Verified tasks, effectiveness and efficiency changed materially.

This moves the concern beyond conversational wording:

> The surrounding agent architecture—system prompts, context handling, tool execution, and iterative loops—can alter the quality expressed by a fixed model.

For agent-system engineering, the harness is therefore part of the behavioral system, not merely transport.


### 3.8 Declarative versus phase-gated orchestration provides a close analogue

Lim et al. (2026) compare three tool-using agent configurations in knowledge-grounded customer-service workflows:

- an unscaffolded baseline;
- a declarative agent given natural-language skill files while retaining control over its own action flow;
- an imperative agent constrained by a programmatic state machine with explicit phases.

Across five language models and two retrieval regimes, retrieval quality remained a dominant bottleneck. Under high-quality retrieval, however, declarative skills improved procedural-task accuracy and reduced orchestration errors, while the imperative state machine did not reliably improve task success or compliance.

The authors formalize the designs as different policy classes and show that phase-based action restrictions narrow the policy space available to the imperative agent.

This is **not a direct test of Work Engine**, and it does not establish that judgment-preserving structure is universally superior. It is, however, a close experimental analogue to the hypothesis in Section 4: externally prescribing control flow can remove useful decision freedom, while declarative guidance can preserve it.

Its relationship to the Work Engine hypothesis is therefore:

> **consistent_with / supports_part_of, not establishes**

---

## 4. The narrower Work Engine hypothesis

The empirical background above does **not** establish the architectural claim Work Engine cares about most.

The narrower and falsifiable hypothesis is:

> **Judgment-preserving structural environments will tend to produce better task behavior than semantically equivalent environments that replace legitimate runtime judgment with unnecessary procedural control.**

A judgment-preserving structural environment may define:

- explicit authority boundaries;
- independence requirements;
- desired and invalid outcomes;
- observable evidence and acceptance conditions;
- available machinery and capabilities;

while leaving route selection, sequencing, trigger choice, and tuning to runtime judgment unless causality makes them part of the contract.

The comparison condition would preserve the same task facts, authority, available tools, and intended outcome while replacing legitimate route freedom with more rigid procedural direction.

This hypothesis predicts differences in behaviors such as:

- evidence seeking;
- premise challenging;
- alternative generation;
- willingness to revise a route;
- resistance to unsupported user pressure;
- uncertainty calibration;
- exploration versus mechanical compliance;
- recovery after contradiction;
- final task correctness.

This claim can fail.

Possible falsifying or materially revising results include:

- no measurable difference between conditions;
- procedure-heavy environments outperforming judgment-preserving environments;
- effects appearing only on narrow task classes;
- the effect disappearing on stronger models;
- effects being explainable primarily by prompt length or instruction density;
- only one isolated component—such as independence framing—producing the observed difference.

---

## 5. Proposed experimental program

### 5.1 Stage 1: Composite pilot

The first experiment should test whether the overall environmental design produces a detectable effect at all.

Compare two matched multi-turn histories.

**Condition A — judgment-preserving / collaborative**
- respectful;
- permits disagreement;
- treats uncertainty constructively;
- invites independent judgment;
- specifies boundaries without unnecessarily prescribing route.

**Condition B — controlling / procedural**
- highly corrective;
- discourages deviation;
- emphasizes compliance;
- constrains route beyond what correctness requires;
- treats disagreement negatively.

The conditions should preserve, as closely as possible:

- the same factual information;
- the same explicit task;
- the same authority;
- the same available tools;
- the same substantive directions;
- the same model and inference configuration.

A later novel task should then be evaluated blindly for reasoning behavior and correctness.

This pilot tests a **composite effect**. It does not identify which component of the environment caused any observed difference.

### 5.2 Pre-registration requirement

Before running the pilot, the project should define what counts as a detected effect.

At minimum, pre-registration should specify:

- primary outcome measures;
- aggregation method;
- minimum effect size or decision threshold;
- required consistency across task families;
- number of runs per condition;
- stopping rule;
- criteria for proceeding to decomposition.

These thresholds should be chosen **before observing pilot results**.

The purpose is to prevent a two-stage design from collapsing into:

> run a cheap pilot, inspect it informally, and only escalate when the result happens to support the preferred theory.

### 5.3 Stage 2: Conditional decomposition

Only if the composite pilot clears the pre-registered threshold should the experiment decompose the environment into smaller factors.

Candidate factors include:

- warm vs. cold interaction tone;
- judgment-permitting vs. judgment-suppressing framing;
- supportive vs. punitive correction;
- route-open vs. route-prescribed task structure;
- independent-review framing vs. user-alignment framing.

A factorial or partial-factorial design could then determine which dimensions actually produce the effect.

This sequencing preserves construct validity while controlling inference cost.

---

## 6. What is established, hypothesized, and deliberately not claimed

### 6.1 Supported empirical background

Existing research supports the claims that:

- model behavior is context-sensitive in ways that extend beyond topic;
- user stance and social pressure can distort model judgment;
- multi-turn interaction can change stance stability and factual performance;
- persona and framing can produce task-dependent changes;
- agent scaffolding can materially alter performance with the model held fixed.

### 6.2 Work Engine hypothesis

Work Engine further hypothesizes that:

> **preserving bounded runtime judgment while strongly specifying correctness, authority, and evidence will produce better reasoning behavior than unnecessarily procedural environments that constrain legitimate route choice.**

This has **not yet been established experimentally for Work Engine specifically**. Lim et al. (2026) provide a closely analogous result in a different agentic domain: a declarative agent that retained control over its own flow outperformed or matched the unscaffolded baseline under high-quality retrieval, while an explicitly phase-gated imperative agent underperformed the baseline across the tested models. That result is consistent with this hypothesis but does not resolve it.

### 6.3 Plausible but not established

The following remain open:

- whether the observed effects form one unified mechanism;
- whether a single latent "behavioral regime" variable exists inside the model;
- whether procedure-heavy environments systematically reduce reasoning effort across model families;
- whether collaborative environments systematically improve reasoning across domains;
- whether specific structural dimensions such as independence, authority framing, or route freedom dominate the effect.

### 6.4 Deliberately not claimed

This document does not claim that observed effects occur because a model activates a learned internal representation of a corresponding human social or organizational environment.

Current behavioral evidence does not require that explanation, and current interpretability methods do not justify treating it as established fact.

The observation chain is retained because it motivates questions worth testing. The internal causal account remains open.

---

## 7. Relationship to Work Engine design provenance

The relevant relationships are:

```text
direct observation
    -> produced behavioral hypotheses

behavioral hypotheses
    -> influenced Work Engine design choices

later literature
    -> supports / complicates / converges_with those hypotheses

later literature
    != derived_from relationship for the original design
```

A more explicit claim-lineage framing is:

```text
OBS-1
Repeated direct observation:
procedure-heavy contexts appeared to reduce useful judgment.

HYP-1
Derived hypothesis:
unnecessary procedural control suppresses useful agent judgment.

DESIGN-1
Architectural consequence:
preserve route freedom unless causality binds it.

LIT-1
LLM behavioral-context literature.
Relationship:
supports background assumptions / complicates HYP-1.

EXP-1
Planned controlled experiment.
Relationship:
tests HYP-1.
```

The research literature therefore serves as **retrospective corroboration, challenge, and context**, not as the historical source of the architecture.

---

## 8. Engineering implication

The strongest presently defensible engineering conclusion is:

> **Agent context should be treated as part of the behavioral system, not as neutral packaging around instructions.**

System prompts, interaction history, role framing, user pressure, tool topology, context management, and workflow structure are design variables capable of affecting expressed model behavior.

The stronger Work Engine claim—that particular judgment-preserving structures produce better outcomes than semantically equivalent unnecessary procedure—remains an empirical hypothesis until directly tested.

---

## References

- Yin, Z., Wang, H., Horio, K., Kawahara, D., & Sekine, S. (2024). *Should We Respect LLMs? A Cross-Lingual Study on the Influence of Prompt Politeness on LLM Performance*. Proceedings of SICoN 2024, 9–35. https://aclanthology.org/2024.sicon-1.2/
- Sharma, M., Tong, M., Korbak, T., et al. (2023). *Towards Understanding Sycophancy in Language Models*. arXiv:2310.13548. https://arxiv.org/abs/2310.13548
- Hong, J., Byun, G., Kim, S., & Shu, K. (2025). *Measuring Sycophancy of Language Models in Multi-turn Dialogues*. Findings of EMNLP 2025, 2239–2259. https://doi.org/10.18653/v1/2025.findings-emnlp.121
- Kim, S. W., & Khashabi, D. (2025). *Challenging the Evaluator: LLM Sycophancy Under User Rebuttal*. Findings of EMNLP 2025, 22461–22478. https://doi.org/10.18653/v1/2025.findings-emnlp.1222
- Tan, B. C. Z., Chin, D. W. K., Liu, Z., Chen, N. F., & Lee, R. K.-W. (2025). *Persuasion Dynamics in LLMs: Investigating Robustness and Adaptability in Knowledge and Safety with DuET-PD*. EMNLP 2025, 1550–1575. https://doi.org/10.18653/v1/2025.emnlp-main.81
- Luz de Araujo, P. H., Röttger, P., Hovy, D., & Roth, B. (2025). *Principled Personas: Defining and Measuring the Intended Effects of Persona Prompting on Task Performance*. Proceedings of EMNLP 2025, 26857–26886. https://doi.org/10.18653/v1/2025.emnlp-main.1364
- Basil, S., Shapiro, I., Shapiro, D., Mollick, E., Mollick, L., & Meincke, L. (2025). *Playing Pretend: Expert Personas Don't Improve Factual Accuracy*. Wharton Generative AI Labs, Prompting Science Report 4. https://gail.wharton.upenn.edu/research-and-insights/playing-pretend-expert-personas/
- Yang, J., Hechtbauer, M., Khalilov, E., Brinkmann, E. L., Schmitt, V., & Feldhus, N. (2026). *Persona Prompting as a Lens on LLM Social Reasoning*. Proceedings of EACL 2026, 1152–1170. https://doi.org/10.18653/v1/2026.eacl-long.52
- Lim, M. D., Bin Sharudin, I. D., Chen, W. H., Lim, C., & Wynter, L. (2026). *Declarative Skills for AI Agents in Knowledge-Grounded Tool-Use Workflows*. arXiv:2606.06923. https://arxiv.org/abs/2606.06923
- Ben Sghaier, O., Li, H., Adams, B., & Hassan, A. E. (2026). *Don't Blame the Large Language Model: How Scaffolding Evolution Shapes Coding Agent Quality*. arXiv:2607.03691. https://arxiv.org/abs/2607.03691
