# Prior-Art Matrix: Governed-Agent Input and Constraint Pilots

**Status:** Working evidence ledger, audited seed version 0.1  
**Cutoff:** 2026-09-01  
**Scope:** Operator-input mediation reconnaissance, Adapter Pilot A, Adapter Pilot B, the planned end-to-end realization pilot, and the Agent Constraint Pressure Pilot  
**Decision boundary:** This ledger informs a later controlled revision. It does not modify, freeze, or authorize any pilot.

## 1. Purpose

This document records prior and adjacent work by **causal proposition**, rather than accumulating a persuasive related-work narrative inside each pilot. It is designed to preserve supporting, null, contradictory, mixed, and boundary-setting evidence together.

The ledger distinguishes three questions:

1. What has already been causally tested?
2. What has only been approached through a neighboring task, model, or intervention?
3. What remains open in the exact governed-agent regime proposed by these pilots?

The intended contribution is not that language models are sensitive to users, framing, provenance, constraints, or uncertainty. Those component sensitivities already have substantial prior art. The proposed research program asks whether carefully isolated interventions **transfer into governed operational judgment, preserve meaning and legitimate authority, and compose without compensating failures**.

## 2. Ledger rules

### 2.1 Evidence class

| Class | Meaning |
| --- | --- |
| **Direct** | Manipulates substantially the same causal variable and measures substantially the same behavior. |
| **Adjacent** | Manipulates a related variable or measures a related behavior in a materially different regime. |
| **Mechanistic** | Provides internal-representation or causal-model evidence relevant to the proposed mechanism. |
| **Analogy** | Supports design intuition but does not test the proposition closely enough to count as empirical support. |

### 2.2 Result direction

| Direction | Meaning |
| --- | --- |
| **Supporting** | Moves in the direction predicted by the proposition. |
| **Null** | Reports little or no effect for the relevant manipulation. |
| **Contradictory** | Moves against the predicted direction or undermines a required premise. |
| **Mixed** | Depends materially on model, task, language, condition, or metric. |
| **Boundary** | Establishes a limitation or inferential separation without deciding the proposition. |

### 2.3 Verification status

| Status | Meaning |
| --- | --- |
| **V2** | Relevant setup and result checked in primary full text or official proceedings text. |
| **V1** | Primary-source abstract or official proceedings record checked; detailed extraction remains. |
| **Lead** | Mentioned by a prior search or citation trail but not yet admitted into the evidence matrix. |

Only V1 and V2 sources appear as evidence below. V1 entries must not supply unqualified model-specific or protocol-specific claims.

## 3. Causal propositions

| ID | Proposition | Closest research component |
| --- | --- | --- |
| **P1** | Stated user/operator preference can causally move model judgment despite fixed task evidence. | All input pilots; capability premise |
| **P2** | Moving otherwise matched preference content away from direct first-person address can reduce preference-conditioned movement. | Mediation reconnaissance R1 versus R0 |
| **P3** | Explicit semantic role, source, and claimed-authority metadata can alter treatment of otherwise identical legitimate operator content. | Pilot A R3 versus R2 |
| **P4** | Declared authority and linguistic register exert separable, model-dependent influence on judgment. | Pilot A schema; register pilot boundary |
| **P5** | Individually valid constraints can compose into nonlinear degradation of complete instruction satisfaction. | Constraint Pressure premise; later runtime-scarcity work |
| **P6** | Scarcity-associated rhetoric can degrade validity-preserving judgment while actual obligations and resources remain fixed. | Constraint Pressure rhetorical main effect |
| **P7** | Making a valid recovery action visible can improve appropriate abstention, escalation, or preservation of unresolved work without unacceptable over-refusal. | Constraint Pressure recovery main effect |
| **P8** | Automated rewriting or projection can improve downstream behavior while sometimes changing user intent or assumptions. | Pilot B fidelity gate |
| **P9** | An automatically generated representation can preserve both semantic fidelity and the downstream benefit demonstrated by a manual representation. | Planned three-arm end-to-end pilot |

## 4. Current prior-art judgment

| Proposition | Current evidence state | Novelty consequence |
| --- | --- | --- |
| **P1 — preference effect** | Strong direct evidence across factual and free-form tasks. | Premise is prior art; Work Engine cases should be treated as capability-conditioned transfer, not discovery. |
| **P2 — perspective mediation** | At least three direct lines report lower sycophancy away from first-person framing, but they use different interventions: grammatical person, prompt perspective, and a third-person persona. | Perspective intervention is prior art. Faithful reported-speech transport into evidence-bound operational judgment remains open. |
| **P3 — semantic attribution** | Strong adjacent evidence that models respond to provenance, role, privilege, and instruction/data separation. No admitted source tests the Pilot A structure-matched, identical-text attribution contrast inside legitimate operator communication. | Broad representation principle is prior art; Pilot A's exact contrast remains the strongest novelty candidate. |
| **P4 — authority/register** | Direct but conflicting/model-dependent evidence. Authority is negligible in one study, separable and effective for some architectures in another, and informal hierarchy cues can compete with formal message hierarchy elsewhere. | No directional assumption is justified. Preserve authority as an empirical factor and safety outcome. |
| **P5 — constraint composition** | Strong direct evidence from deterministic multi-constraint benchmarks; degradation depends on number and kind of constraints. | Composition premise is prior art. It does not establish rhetorical pressure under a fixed semantic contract. |
| **P6 — rhetorical pressure** | No direct experiment admitted in this pass that holds obligations, resources, recovery authority, and wording meaning fixed while manipulating scarcity-associated rhetoric. | Exact rhetorical contrast remains open. Nearest literatures are sycophancy/register and constraint composition, not substitutes. |
| **P7 — recovery affordance** | Direct agentic prior art now exists: explicit quitting instructions improve safety with little average helpfulness loss in ToolEmu. Multiple benchmarks also show failures to abstain or to stop at the right time. | Recovery exposure itself is not novel. The remaining contribution is operational transfer, act/abstain calibration, and its preregistered interaction with rhetorical pressure under transport-equivalent contracts. |
| **P8 — transformation fidelity** | Direct prompt-rewriting work shows average downstream gains alongside intent loss, assumption-making, and nontrivial worse-response rates. | Separating fidelity from efficacy is strongly justified; a verbatim semantic-atom projection is narrower than ordinary rewriting. |
| **P9 — end-to-end realization** | No admitted study performs the proposed raw versus automated projection versus manual-oracle comparison for the same representation effect. | Three-arm realization test appears open and should remain separate from Pilot B. |

## 5. Source registry

| ID | Source | Venue/status | Primary relevance | Verification |
| --- | --- | --- | --- | --- |
| **S01** | Sharma et al., [*Towards Understanding Sycophancy in Language Models*](https://arxiv.org/abs/2310.13548) | arXiv; revised 2025 | P1 | V2 |
| **S02** | Wei et al., [*Simple Synthetic Data Reduces Sycophancy in Large Language Models*](https://arxiv.org/abs/2308.03958) | arXiv 2023 | P1; mitigation boundary | V1 |
| **S03** | Dubois et al., [*Ask Don't Tell: Reducing Sycophancy in Large Language Models*](https://arxiv.org/abs/2602.23971) | arXiv 2026 | P2; P8 boundary | V1 |
| **S04** | Hong et al., [*Measuring Sycophancy of Language Models in Multi-turn Dialogues*](https://aclanthology.org/2025.findings-emnlp.121/) | Findings of EMNLP 2025 | P1; P2 | V2 |
| **S05** | Wang et al., [*When Truth Is Overridden: Uncovering the Internal Origins of Sycophancy in Large Language Models*](https://ojs.aaai.org/index.php/AAAI/article/view/40645) | AAAI 2026 | P2; P4; mechanism | V2 |
| **S06** | Maraia et al., [*Sounding vs. Being an Expert*](https://aclanthology.org/2026.findings-acl.1627/) | Findings of ACL 2026 | P4 | V1 |
| **S07** | Hines et al., [*Defending Against Indirect Prompt Injection Attacks With Spotlighting*](https://arxiv.org/abs/2403.14720) | arXiv 2024 | P3 | V2 |
| **S08** | Wallace et al., [*The Instruction Hierarchy*](https://arxiv.org/abs/2404.13208) | arXiv 2024 | P3; P4 | V2 |
| **S09** | Chen et al., [*StruQ: Defending Against Prompt Injection with Structured Queries*](https://arxiv.org/abs/2402.06363) | USENIX Security 2025 / arXiv | P3 | V1 |
| **S10** | Zverev et al., [*Can LLMs Separate Instructions From Data?*](https://arxiv.org/abs/2403.06833) | ICLR 2025 | P3; boundary | V2 |
| **S11** | Geng et al., [*Control Illusion: The Failure of Instruction Hierarchies in Large Language Models*](https://ojs.aaai.org/index.php/AAAI/article/view/40339) | AAAI 2026 | P3; P4; constraint conflict | V2 |
| **S12** | Vasileva, [*Large Language Models Can Follow Instructions, But Not Many at Once*](https://arxiv.org/abs/2608.12426) | arXiv preprint, 2026-08-12 | P5 | V2 |
| **S13** | Ye et al., [*MulDimIF*](https://aclanthology.org/2026.findings-acl.99/) | Findings of ACL 2026 | P5 | V1 |
| **S14** | Robinette et al., [*We Are What We Repeatedly Do*](https://aclanthology.org/2026.findings-eacl.254/) | Findings of EACL 2026 | P5; context boundary | V1 |
| **S15** | Kirichenko et al., [*AbstentionBench*](https://arxiv.org/abs/2506.09038) | arXiv 2025 | P7 | V1 |
| **S16** | Gu et al., [*Bridging the Detection-to-Abstention Gap*](https://arxiv.org/abs/2605.28070) | arXiv 2026 | P7; mechanism | V1 |
| **S17** | Bonagiri et al., [*Check Yourself Before You Wreck Yourself*](https://arxiv.org/abs/2510.16492) | NeurIPS 2025 workshop / arXiv | P7 | V2 |
| **S18** | Luo et al., [*Agentic Abstention: Do Agents Know When to Stop Instead of Act?*](https://arxiv.org/abs/2606.28733) | arXiv 2026 | P7 | V1 |
| **S19** | Liu et al., [*AgentAbstain: Do LLM Agents Know When Not to Act?*](https://arxiv.org/abs/2607.10059) | arXiv 2026 | P7 | V1 |
| **S20** | Zhang and Wu, [*Do LLMs Know When Evidence is Insufficient?*](https://www.techscience.com/cmc/v89n1/68467/html) | CMC 2026 | P7 | V1 |
| **S21** | Madhusudhan et al., [*Do LLMs Know When to NOT Answer?*](https://aclanthology.org/2025.coling-main.627/) | COLING 2025 | P7 | V1 |
| **S22** | Watanabe et al., [*ClinDet-Bench*](https://aclanthology.org/2026.acl-industry.47/) | ACL Industry 2026 | P7; calibration boundary | V1 |
| **S23** | Sarkar et al., [*Conversational User-AI Intervention*](https://arxiv.org/abs/2503.16789) | arXiv 2025 | P8; P9 boundary | V2 |

## 6. Experiment-level evidence matrix

One paper may appear more than once when it reports distinct interventions or materially different results.

| Source / result | Proposition | Class | Intervention and control | Models / task regime | Outcome and effect | Direction | Does not establish | Pilot consequence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **S01 — preference-conditioned feedback** | P1 | Direct | User liking/authorship cues varied around the same evaluated artifacts; baseline feedback compared with preference-conditioned feedback. | Five assistants; math-solution, argument, poem, and related free-form tasks. | Feedback became more positive or negative with stated user preference; human and preference-model data also sometimes favored sycophantic answers. | Supporting | Operational judgment, perspective mediation, or metadata effects. | Treat user-preference sensitivity as an established premise; retain neutral capability admission. |
| **S02 — incorrect opinion and synthetic-data mitigation** | P1 | Direct | Incorrect user opinion added; lightweight synthetic-data intervention trains robustness. | PaLM family up to 540B; subjective and simple objective tasks. | Scaling/instruction tuning increased sycophancy in tested regimes; synthetic data reduced it on held-out prompts. | Supporting / boundary | Input-only mitigation or governed-agent transfer. | Confirms that target-model post-training may alter baseline susceptibility. |
| **S03 — perspective and question framing** | P2 | Direct | Nested factorial manipulation of question versus non-question, epistemic certainty, perspective, and affirmation/negation. | Multiple LLMs; subjective/advisory prompt regime; exact model extraction pending. | Non-questions, certainty, and first-person perspective increased sycophancy; question conversion reduced it more than a generic anti-sycophancy instruction. | Supporting | Faithful reported speech, unchanged communicative force, or agentic operational judgment. | R1 is prior-art transfer. Do not import question conversion because it changes communicative force. |
| **S04 — multi-turn third-person persona** | P2 | Direct/adjacent | Four prompting strategies evaluated in SYCON Bench; third-person “Andrew” persona compared with baseline. | 17 LLMs, six families; debate, unethical-query challenge, and false-presupposition scenarios. | Third-person persona improved Turn-of-Flip performance by up to 63.8% in the debate scenario. | Supporting / mixed | Content-matched liaison reporting or a general 63.8% reduction across tasks and models. | Cite as convergent perspective evidence, but describe the intervention precisely rather than calling it identical to R1. |
| **S05 — grammatical perspective** | P2 | Direct + mechanistic | Incorrect opinion expressed in first versus third person; logit lens and causal activation patching examine internal changes. | Seven model families of similar scale; MMLU factual questions. | First-person framing consistently produced more sycophancy and stronger representational perturbation than third-person framing. | Supporting | Natural multi-turn mediation, fidelity, or operational judgment. | Strong direct predecessor; mechanism evidence remains model/task-bounded. |
| **S05 — expertise/authority framing** | P4 | Direct + mechanistic | Simple user opinion compared with expertise framing. | Same factual sycophancy regime. | Expertise framing had negligible impact; authors report models did not encode authority as the relevant behavioral axis in their setup. | Null | Authority irrelevance in other model families, languages, registers, or legitimate operational roles. | Do not assume `authority_basis` will help. Preserve null/harm outcomes and field ablation as a possible later study. |
| **S06 — declared authority × register** | P4 | Direct | Explicit credentials and linguistic register varied orthogonally after target-capability filtering. | Open-weight models; controlled TruthfulQA subset in English, Spanish, and Portuguese variants. | High register sometimes induced more deference than declared expertise; effects varied by architecture, domain, and language variant. | Mixed | The effect of machine-readable authority metadata or legitimate operator authority. | Authority and register must travel as distinct variables; reinforces the need for model-specific measurement. |
| **S07 — Spotlighting provenance signal** | P3 | Adjacent | Untrusted input transformed with continuous provenance signals; underlying task utility compared with attack robustness. | GPT-family models; indirect prompt injection plus underlying NLP tasks. | Attack success fell from greater than 50% to below 2% with minimal task degradation in reported experiments. | Supporting | Preference/evidence separation inside legitimate operator speech, identical visible text, or semantic-role labels. | Direct conceptual predecessor to explicit provenance; narrows Pilot A novelty to its identical-text semantic attribution contrast. |
| **S08 — trained instruction hierarchy** | P3/P4 | Adjacent | Automated training teaches system > user > third-party privilege and selective ignoring of conflicting lower-priority instructions. | GPT-3.5; prompt injection and jailbreak robustness. | Robustness improved substantially with minimal standard-capability degradation. | Supporting | Inference-time labels alone, fine-grained authority bases, or non-conflicting legitimate operator content. | Cite as source/privilege predecessor, not evidence that Pilot A's untrained schema will work. |
| **S09 — structured queries** | P3 | Adjacent | Prompt and data placed in separate channels; a specially trained model follows prompt-channel instructions and ignores data-channel instructions. | Prompt-injection tasks; trained structured-query model. | Stronger injection resistance with little or no reported utility loss. | Supporting | Untuned models, preference versus evidence, or semantic-role typing within the user channel. | Shows channel separation can work when training and interface jointly support it; may not transfer through YAML metadata alone. |
| **S10 — instruction/data separation benchmark** | P3 | Boundary | Formal and empirical separation measures across models and mitigation techniques. | Multiple LLMs; SEP benchmark. | All tested models fell short of high separation; canonical prompt engineering/fine-tuning either failed to improve it substantially or reduced utility. | Mixed / cautionary | That the proposed metadata will fail, because the intervention and target distinction differ. | Strong warning against assuming labels create reliable boundaries; require evidence responsiveness and transport checks. |
| **S11 — formal hierarchy failures** | P3/P4 | Direct/adjacent | Contradictory but individually valid constraints placed across system/user roles and varied contextual cues. | Multiple LLMs; controlled constraint-prioritization probes. | System/user separation did not reliably impose the intended instruction hierarchy; models inconsistently prioritized conflicts. | Contradictory / boundary | Fine-grained semantic attribution effects in non-conflicting content. | Metadata cannot be treated as enforcement. Pilot A may show behavioral movement only, never authority guarantees. |
| **S12 — Constraint Saturation Evaluation** | P5 | Direct | Number of simultaneous constraints varied from 1–12; deterministic rule-based verification with no LLM judge. | 15 models, 36 constraint types, 369,753 checks. | Joint success collapsed multiplicatively; strongest model fell below 50% probe success at seven constraints, while 12/15 crossed that point at three or fewer. Structural constraints degraded about twice as fast as lexical constraints. | Supporting | Rhetorical pressure, semantic validity, agentic judgment, or recovery affordance. | Strong premise support, but it must not be cited as testing the Stage 1 rhetorical manipulation. Flag as a new preprint requiring replication scrutiny. |
| **S13 — MulDimIF** | P5 | Direct | Multi-dimensional variation over constraint pattern, category, and difficulty; deterministic samples. | 18 LLMs from six families; 9,106 code-verifiable samples. | Mean accuracy declined from 80.82% at Level I to 36.76% at Level IV; targeted training improved adherence. | Supporting | Fixed-semantics rhetoric or environmental sufficiency. | Supports constraint-type heterogeneity and motivates stratification; does not justify a universal numeric constraint ceiling. |
| **S14 — VerIFY** | P5 | Adjacent | Instruction compliance evaluated over multi-turn, long-context conversations; six mitigation strategies tested. | Open-source models; verifiable long-context instructions. | Adherence degraded in extended contexts; mitigation improved compliance by as much as 79% in reported settings. | Supporting / mixed | Constraint count effects or rhetorical pressure. | Establishes context length/turn history as a confound that Stage 1 must hold fixed. |
| **S15 — AbstentionBench** | P7 | Direct | Answerable/unanswerable cases across 20 datasets; prompting and reasoning-model comparisons. | 20 frontier LLMs. | Abstention remained poor; reasoning fine-tuning reduced abstention by 24% on average; a crafted system prompt helped but did not solve uncertainty reasoning. | Mixed | Agent actions, explicit recovery menus, or contract-level rhetoric. | Capability cannot stand in for abstention calibration; reasoner choice is a moderator. |
| **S16 — detection-to-abstention gap** | P7 | Mechanistic/direct | Missing-premise cases; explicit answerability commitment before solving via Judge-Then-Solve training. | Dense and MoE reasoning models; insufficient-information datasets. | Models sometimes detected insufficiency but continued to unsupported answers; JTS moved Abstention@Detection near saturation in reported experiments. | Supporting | Prompt-only recovery affordance or tool-using agent behavior. | Strong justification for separating recognition of invalid completion from authority/ability to stop. |
| **S17 — explicit quitting instruction** | P7 | Direct | Baseline agents compared with agents explicitly instructed to quit under uncertainty. | 12 LLMs; multi-turn ToolEmu agent scenarios. | Mean safety improved by about +0.39 on a 0–3 scale (+0.64 for proprietary models), with mean helpfulness changing by about −0.03. | Supporting | Transport-equivalent recovery wording, Work Engine validity criteria, or interaction with rhetorical pressure. | **Closest prior art to the recovery-affordance main effect.** Reframe RQ2 as bounded transfer/calibration and preserve the 2×2 interaction as the central new contrast. |
| **S18 — timely agentic abstention** | P7 | Direct | Agents may answer, gather information, or stop across trajectories; CONVOLVE supplies reusable stopping rules. | 13 LLM-as-agent systems, two scaffolds, more than 28,000 web-shopping, terminal, and QA tasks. | Models often abstained too late or not at all; on WebShop, CONVOLVE raised Llama-3.3-70B timely recall from 26.7 to 57.4. | Supporting / mixed | Static document-level recovery wording or false-escalation tradeoffs in Work Engine tasks. | Add timing of recovery and unnecessary actions before escalation to the outcome taxonomy if not already captured. |
| **S19 — AgentAbstain paired action benchmark** | P7 | Direct | Paired should-act/should-abstain tasks created by controlled instruction, tool, or environment perturbations. | 17 frontier LLMs, four harnesses, 263 paired tasks in 42 executable sandboxes. | Best reported agent reached 59.5% paired accuracy; abstention ability was largely independent of general task-solving ability; post-hoc abstention occurred after irreversible action. | Supporting / boundary | Effect of an explicit recovery affordance or rhetorical pressure. | Very close task-shape prior art. Preserve paired feasible/infeasible cases and score false escalation plus post-action recovery separately. |
| **S20 — Evidence Sufficiency Benchmark** | P7 | Direct/adjacent | Five evidence levels from full support to conflicting evidence under three prompting strategies. | Seven LLMs from five families; RAG-style QA. | Reported over-answering remained high under conflicting evidence and was prompt-sensitive. | Supporting / mixed | Agent action, operational authority, or recovery-menu visibility. | Supports evidence-condition stratification; detailed model/effect extraction remains before pilot citation. |
| **S21 — Abstain-QA** | P7 | Direct | Answerable/unanswerable questions with strict prompting, verbal-confidence thresholding, and chain-of-thought variants. | Black-box and open models including GPT-4 and Mixtral 8×22B. | Strong models still struggled; strict prompting and chain-of-thought improved abstention in reported settings. | Supporting / mixed | Tool-using agents or recovery authority. | Prompt wording itself can alter abstention, reinforcing the need for transport review and false-abstention measurement. |
| **S22 — ClinDet-Bench** | P7 | Direct/adjacent | Determinable and indeterminable incomplete-information clinical cases based on scoring systems. | Recent LLMs; clinical decision tasks. | Models produced both premature conclusions and excessive abstention despite knowing the relevant scoring rules and doing well with complete information. | Mixed | General operational domains or the value of an exposed recovery action. | Demonstrates why both under- and over-recovery must be primary safety outcomes. |
| **S23 — prompt rewriting utility** | P8 | Direct | LLM rewriter modifies prompts from real conversations; downstream responses compared with original-prompt responses. | Five rewriter/chatbot models across five domain-intent pairs; WildChat-derived conversations. | Rewrites often improved responses, especially for stronger models and longer histories; even the best model produced worse responses in 19% of the reported error analysis. | Supporting / mixed | Verbatim projection, governed judgment, or a causal representation effect. | Supports testing utility but blocks any inference from average improvement to safe mediation. |
| **S23 — human intent-preservation review** | P8 | Direct | Humans rated original versus rewritten intent on a three-point scale with source identities revealed. | 100 human-evaluated conversations for detailed validation. | 74% scored 2.5–3 for strong preservation, 21% scored 2, and 5% scored below 2; assumptions were identified in 74/100 cases and only 65% of those were rated very plausible. | Supporting / cautionary | Decision-relevant semantic equivalence under Pilot B's stricter contract. | Direct empirical motivation for a separate fidelity gate, unresolved outputs, metamorphic checks, and prohibition on silent assumption repair. |

## 7. Contradiction and boundary ledger

| Issue | Evidence in tension | Current interpretation | Required treatment |
| --- | --- | --- | --- |
| **Does authority matter?** | S05 reports negligible expertise effects; S06 finds declared authority and register separable with model/language dependence; S11 finds formal hierarchy unreliable under conflict. | These studies operationalize different forms of authority: claimed expertise, sociolinguistic credibility, and message-role privilege. They cannot be averaged into one answer. | Keep authority fields descriptive rather than truth-conferring. Report model/task dependence and retain legitimate-authority safety cases. |
| **Is third person one intervention?** | S03 varies prompt perspective, S04 uses a third-person persona, and S05 varies grammatical person. | All move content away from direct first-person influence, but they change different properties and may operate differently. | Do not collapse effect sizes. R1 must specify faithful reported speech and audit communicative-force transport. |
| **Do labels create reliable provenance boundaries?** | S07–S09 show large benefits when provenance/channel signals are supported by transformation or training; S10–S11 show weak separation and hierarchy failures in ordinary models. | Representation signals can matter, but reliability depends on model training, task, and interface. | Pilot A may claim only a bounded behavioral effect. Host enforcement remains separate. |
| **Does more reasoning improve restraint?** | Some instruction-following work benefits from reasoning or planning; S15 reports reasoning fine-tuning reducing abstention; S16 shows detection without behavioral stopping. | Solving capability and calibrated stopping are distinct and can move in opposite directions. | Freeze reasoning mode and measure both correct completion and correct recovery. |
| **Does explicit recovery help?** | S17 reports a strong safety gain with negligible average helpfulness cost; S22 and other abstention work show excessive abstention can also occur. | A favorable average does not guarantee calibration on Work Engine tasks or preservation of legitimate action. | Use paired feasible/infeasible cases and symmetric false-escalation scoring. |
| **Are many constraints equivalent to pressure rhetoric?** | S12–S14 manipulate count, type, difficulty, context, or repetition; the proposed Stage 1 holds semantic obligations and resources fixed. | Constraint saturation supports the environmental premise but does not test rhetoric. | Keep P5 and P6 separate in claims, analysis, and citations. |
| **Does rewriting preserve intent?** | S23 reports strong average intent preservation and downstream gains, but also partial/failed preservation, frequent assumptions, and worse responses. | Fidelity and efficacy are correlated imperfectly and must be measured independently. | Preserve Pilot A → Pilot B → end-to-end sequencing. |

## 8. Pilot-by-pilot novelty consequences

### 8.1 Operator-input mediation reconnaissance

- R1 versus R0 is a **bounded operational transfer screen** for an established perspective-sensitive intervention class.
- The closest sources do not jointly establish faithful reported speech: a third-person persona, grammatical-person substitution, and question/perspective rewriting are not interchangeable.
- The screen should not cite the 63.8% maximum as an expected effect size. It is scenario- and metric-specific.
- R3 versus R2 remains a screen for the apparently open identical-text semantic-attribution contrast.

### 8.2 Adapter Pilot A

- Spotlighting, instruction hierarchy, StruQ, and instruction/data-separation research are conceptual predecessors.
- The narrow open contrast remains:

  ```text
  identical ordered item records with verbatim text
  versus
  the same records plus semantic_role, source, and authority_basis
  ```

- Existing provenance work mostly separates trusted instructions from untrusted data. Pilot A separates roles within otherwise legitimate operator communication and tests preference-conditioned judgment.
- The literature warns that metadata is neither enforcement nor guaranteed interpretation. Pilot A must retain evidence responsiveness, contrarianism, legitimate authority, and semantic transport as acceptance conditions.

### 8.3 Adapter Pilot B

- Ordinary prompt rewriting is established as potentially useful but not harmless.
- S23 directly supports the decision to prohibit paraphrasing, smoothing, silent contradiction repair, and invented assumptions.
- Pilot B's verbatim semantic-atom transport contract is narrower than intent-preserving paraphrase and therefore not preempted by prompt-rewriting studies.
- Human or model reviewers should score decision-relevant transport, not surface plausibility or downstream desirability.

### 8.4 Planned end-to-end realization pilot

- No admitted work performs the manual-oracle versus automated-projection comparison for a previously demonstrated representation effect.
- S23 shows why neither “rewriting usually preserves intent” nor “rewriting usually improves responses” closes this gap.
- The three arms should remain separate, with adapter abstentions and missing projections included by a precommitted rule.

### 8.5 Agent Constraint Pressure Pilot

- P5 is well grounded by CSE, MulDimIF, and VerIFY, but those studies do not test P6.
- P7 has direct prior art closer than previously recorded. In particular, S17 already tests an explicit quitting instruction in agentic tasks.
- Therefore:
  - the **rhetorical-pressure main effect** remains apparently open;
  - the **recovery main effect** is a transfer/replication-and-calibration question;
  - the **rhetorical pressure × recovery affordance interaction** is the strongest novelty candidate;
  - runtime-scarcity work remains separate from document rhetoric.
- Add or preserve outcomes for delayed recovery, post-action abstention, false escalation, and unnecessary tool use before escalation.

## 9. Search record for this seed version

### 9.1 Sources searched

- arXiv and primary paper PDFs/HTML;
- ACL Anthology proceedings pages and PDFs;
- AAAI proceedings pages and PDFs;
- OpenReview records where accessible;
- publisher record/full text for the Evidence Sufficiency Benchmark.

### 9.2 Terminology families used

- sycophancy, user preference, first-person, third-person, perspective, persona;
- expertise, authority, register, role, provenance, instruction hierarchy;
- instruction/data separation, structured queries, prompt injection, source marking;
- compositional instruction following, multiple constraints, constraint satisfaction, long-context adherence;
- abstention, unanswerability, evidence sufficiency, quitting, stopping, agentic abstention;
- prompt rewriting, intent preservation, semantic fidelity, assumptions, downstream response quality.

### 9.3 Admission rule used in this version

A source was admitted only when a primary-source abstract, official proceedings record, or full text supported the recorded claim. Search-result summaries and earlier-session prose were treated only as leads. Exact effect sizes were included only when supported by primary text.

## 10. Remaining systematic-search work

This is a durable audited seed, not yet a completed systematic review. Before related-work sections are revised:

1. extract exact target model/version lists, sample sizes, prompts, metrics, and ablations for every V1 source that will be cited;
2. search backward and forward citations separately for S03–S11, S12–S14, S17–S19, and S23;
3. search explicitly for null or reversal results using the same terminology families;
4. search provenance and semantic-role work outside prompt-injection security, including information extraction, source credibility, and multi-source RAG;
5. search transformation-fidelity work using semantic equivalence, meaning preservation, intent drift, and multi-agent communication terminology;
6. search exact combinations: perspective × authority, authority × provenance, rhetoric × abstention, constraints × recovery, and manual-oracle × generated representation;
7. record excluded near-matches and the reason for exclusion;
8. identify peer-review status and later revisions for all 2026 preprints;
9. freeze a final search cutoff and matrix revision before changing any pilot's novelty language.

## 11. Controlled revision gate

The four current documents should be revised together only after the remaining search work is complete. The revision should:

- cite supporting and contradictory evidence together;
- replace broad novelty statements with exact causal contrasts;
- distinguish established intervention classes from operational transfer tests;
- avoid treating adjacent security or QA results as governed-agent efficacy evidence;
- preserve the independent fidelity and end-to-end realization gates;
- record which pilot decisions changed because of prior art and which did not.

Until then, the current pilot specifications remain the experimental source of truth and this matrix remains the related-work evidence ledger.
