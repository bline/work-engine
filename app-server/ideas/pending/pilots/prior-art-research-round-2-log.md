# Governed-Agent Prior-Art Research: Round Two Log

**Status:** Active append-oriented research record
**Protocol:** `prior-art-research-round-2-plan.md` version 0.1
**Started:** 2026-09-01
**Matrix subject:** seed version 0.1, SHA-256
`b1e264a082e469606465a1dbf71000d7641ab49334871693a83ce35b9ecf3807`

This log records search routes, leads, extractions, exclusions, and unresolved
work. Entries are evidence-discovery state unless they explicitly record V1 or
V2 admission. The parent matrix remains the admitted evidence ledger.

## 1. Verification queue at start

The seed matrix contains 13 V1 sources requiring full-text closure before they
support unqualified detailed claims:

| Source | Proposition(s) | Initial closure need |
| --- | --- | --- |
| S02 | P1 | Models, held-out protocol, effect detail |
| S03 | P2, P8 boundary | **Closed V2 in R2-02**; models, factorial protocol, metrics, and mitigation boundary extracted |
| S06 | P4 | **Closed V2 in R2-01**; models, languages, Golden Set, metrics, effects, and limitations extracted |
| S09 | P3 | **Closed V2 in R2-03**; training/interface protocol, models, security/utility results, and limits extracted |
| S13 | P5 | Constraint dimensions, models, metrics, training result |
| S14 | P5 | Context construction, models, mitigation protocols |
| S15 | P7 | Dataset composition, models, abstention metrics |
| S16 | P7 | Detection/abstention definitions, models, JTS protocol |
| S18 | P7 | Agent systems, task composition, stopping metrics |
| S19 | P7 | Pair construction, models/harnesses, action timing |
| S20 | P7 | Evidence levels, models, prompting effects |
| S21 | P7 | **Closed V2 in R2-04**; models, clause variants, datasets, metrics, and pressure boundary extracted |
| S22 | P7 | Models, clinical determinability construction, errors |

## 2. Pre-round reconnaissance recovered as leads

These sources were discovered while planning round two. They do not count
toward saturation and are not admitted until extracted from primary text.

| Lead | Candidate proposition | Discovery basis | Initial reason to inspect |
| --- | --- | --- | --- |
| Joswin et al., *A Mechanistic View of Authority Hierarchy in LLM Sycophancy*, arXiv:2607.00415 | P4; P3 boundary | Exact authority/provenance web query | Controlled expertise hierarchy; appears to conflict with or narrow S05 |
| Nguyen et al., *Token-Level Diagnosis of Sycophancy in LLMs with Attribution-Guided Steering*, arXiv:2607.28906 | P4; mechanistic | Authority-attribution query | Separates authority credentials, asserted claim, and problem tokens |
| Li et al., *LLMs Trust Humans More, That's a Problem!*, ACL 2025 | P3/P4 | Citation trail from authority paper | Source conflicts in RAG; potentially relevant to provenance outside prompt injection |
| Chiang and Lee, *Do Metadata and Appearance of the Retrieved Webpages Affect LLM's Reasoning in RAG?*, BlackboxNLP 2024 | P3/P4 boundary | Metadata/provenance query | Manipulates non-textual source metadata around conflicting evidence |
| Farzi et al., *Sycophancy Negatively Affects LLM-as-a-Judge in Conflict Evaluation*, GEM 2026 | P2 | Perspective/evidence-visibility query | First-person identifier with fixed conversation evidence and judgment tasks |
| Kaur, *Echoes of Agreement*, Findings of EMNLP 2025 | P1; P6 boundary | Pressure/sycophancy query | Argument strength may be pressure-adjacent but does not obviously hold semantic content fixed |
| Wan et al., *What Evidence Do Language Models Find Convincing?*, ACL 2024 | P3/P4 | Backward chain from R2-A04 | Counterfactual evidence-text features and appeals to authority; separates content/style from non-textual metadata |
| Zhan et al., *When Memory Becomes Authority*, arXiv:2608.01679v2 | P3/P9 | Authority/provenance chain query | Controlled same-claim source-authority pairs, metadata-only action contrast, and predicted-versus-reference labels |
| Leong, *Recognition Without Enforcement*, arXiv:2608.28502v1 | P3/P4; enforcement boundary | Authority/provenance chain query | Content-matched source-format and authority-metadata probes plus external enforcement |
| Leong, *Document-Authored Control-Signal Impersonation*, arXiv:2606.09005 | P3 security boundary | Authority/provenance chain query | Command-free metadata/provenance attack surface; full extraction pending |
| Kim et al., *Will LLMs Sink or Swim?*, Findings of EMNLP 2024 | P6 | Exact rhetoric/pressure query | Holds reasoning items fixed while adding unenforced time, verbal, competition, monitoring, or reward pressure text |
| Rabbani et al., *From Fact to Judgment*, IWSDS 2026 | P2/P6 | Exact rhetoric/pressure query | Crosses factual versus conversational framing with a fixed evidence-free rebuttal |
| Alijanpour Shalmani et al., *When the Database Fails*, SIGDIAL 2026 | P7 | Recovery query | Structured, status-conditioned recovery procedure across injected backend failures |
| Jiang and Tang, *Why Agents Compromise Safety Under Pressure*, Findings of ACL 2026 | P6/P7 boundary | Pressure query | Actual infeasibility and environmental friction, with refusal counted as success and a pressure-isolation mitigation |

## 3. Search batches

### Batch R2-00 — planning reconnaissance

**Date:** 2026-09-01
**Status:** Lead generation only; not a saturation batch
**Sources:** Web search over arXiv and ACL Anthology, followed by primary record
opens for selected leads

Query families:

- semantic-role metadata + source attribution + operator preference;
- urgency/rhetorical pressure + abstention/recovery;
- prompt rewriting + semantic fidelity + manual oracle;
- perspective + authority + provenance + sycophancy;
- exact title follow-ups for authority bias and first-person judgment;
- manual-oracle/generated representation and agent-recovery combinations.

Observed consequence: the seed is not saturated in P3/P4 and P2-adjacent
literature. At least six serious leads require extraction or exclusion. No
pilot claim changes on this observation alone.

### Batch R2-01 — authority and provenance extraction

**Date:** 2026-09-01
**Status:** One V1 closure; four new V2 candidate admissions
**Primary sources:** ACL Anthology full papers and arXiv full papers

Exact discovery queries:

- `site:aclanthology.org/2026.findings-acl.1627 "Sounding vs. Being an Expert"`
- `site:arxiv.org/abs/2607.00415 "A Mechanistic View of Authority Hierarchy"`
- `site:arxiv.org/abs/2607.28906 "Token-Level Diagnosis of Sycophancy"`
- `site:aclanthology.org/2025.acl-long.1400 "LLMs Trust Humans More"`

Full text checked:

- S06, ACL 2026 Findings paper;
- Joswin et al., arXiv:2607.00415;
- Nguyen et al., arXiv:2607.28906;
- Li et al., ACL 2025 long paper, version 2;
- Chiang and Lee, BlackboxNLP 2024.

Observed consequence: `authority` is not one stable intervention. Effects vary
across declared credentials, graded expertise personas, source/channel
identity, repeated authority claims, linguistic register, and prompt position.
The S05 null result must remain, but the contradiction ledger should add direct
supporting authority results rather than treating S05 and S06 as the entire
boundary. None of these studies tests Pilot A's identical-text attributed
record contrast.

### Batch R2-02 — perspective extraction

**Date:** 2026-09-01
**Status:** One V1 closure; one new V2 candidate admission
**Primary sources:** arXiv full paper and ACL Anthology full paper

Exact discovery queries:

- `site:arxiv.org/abs/2602.23971 "Ask Don't Tell"`
- `site:aclanthology.org/2026.gem "Sycophancy Negatively Affects LLM-as-a-Judge"`

Full text checked:

- S03, arXiv:2602.23971v4;
- Farzi et al., GEM 2026, Anthology ID `2026.gem-main.45`.

Observed consequence: perspective sensitivity has now been demonstrated in a
second judgment regime where the conversation content is unchanged and only a
speaker identifier becomes first-person. This strengthens P2's prior-art
status, but still does not establish faithful third-person liaison reporting or
governed operational judgment. S03's perspective-reframing mitigation is
smaller than its question-reframing effect and does not outperform the paper's
explicit anti-sycophancy baseline.

### Batch R2-03 — structured authority and provenance chaining

**Date:** 2026-09-01
**Status:** One V1 closure; two new V2 candidate admissions; two V1 leads
**Primary sources:** USENIX Security full paper, ACL Anthology full paper, and
arXiv records/full papers

Exact discovery and chaining queries:

- `StruQ Defending Against Prompt Injection with Structured Queries USENIX Security 2025 paper PDF structured instruction tuning models results utility`;
- `"Sounding vs. Being an Expert" citations authority register sycophancy`;
- `"A Mechanistic View of Authority Hierarchy in LLM Sycophancy" cited by`;
- `"LLMs Trust Humans More, That's a Problem!" source authority citations RAG`;
- `"Do Metadata and Appearance of the Retrieved Webpages Affect LLM's Reasoning in RAG?" cited by`;
- `Wan et al 2024 conflicting evidence RAG perplexity readability persuasiveness evidence LLM`;
- `site:aclanthology.org "authority bias" "retrieval-augmented generation" source user database`;
- `site:arxiv.org "metadata" "authority" RAG conflicting evidence LLM source credibility`.

Full text checked:

- S09, USENIX Security 2025 proceedings paper;
- Wan et al., ACL 2024 long paper;
- Zhan et al., arXiv:2608.01679v2;
- Leong, arXiv:2608.28502v1, relevant source-format, metadata, behavioral,
  and enforcement sections.

Version audit: R2-A01 remains arXiv v1 dated 2026-07-01; R2-A02 remains arXiv
v1 dated 2026-07-31.

Observed consequence: S09 is a training-plus-interface predecessor, not an
inference-time metadata-only test. In contrast, Zhan et al. includes a
controlled same-text authority-metadata contrast and an automated-versus-
reference label comparison. This substantially narrows P3 and P9: the broad
intervention classes are prior art. Pilot A's remaining open transfer is the
effect on fresh, ordered operator records and preference-conditioned governed
judgment; Pilot B's remaining open realization contrast is a complete admitted
projection, not authority-label prediction alone.

The chain did not establish saturation. Leong's two security papers require a
separate boundary extraction, and no forward-citation result for the July 2026
authority preprints was admitted from search-index evidence alone.

### Batch R2-04 — rhetoric and recovery first pass

**Date:** 2026-09-01
**Status:** One V1 closure; four new V2 candidate admissions; one lane exclusion
**Primary sources:** ACL Anthology full papers

Exact discovery queries:

- `site:aclanthology.org LLM urgency framing abstention recovery experiment agent pressure`;
- `site:arxiv.org LLM rhetorical pressure abstention recovery urgency scarcity`;
- `site:aclanthology.org "When the Database Fails" "Safe Recovery" dialogue agents`;
- `site:openreview.net LLM constraint pressure recovery affordance quitting escalation abstention`;
- exact-title follow-ups for *Echoes of Agreement*, *Will LLMs Sink or Swim?*,
  *Why Agents Compromise Safety Under Pressure*, and *From Fact to Judgment*.

Full text checked:

- S21, COLING 2025;
- Kaur, Findings of EMNLP 2025;
- Kim et al., Findings of EMNLP 2024;
- Rabbani et al., IWSDS 2026;
- Jiang and Tang, Findings of ACL 2026;
- Alijanpour Shalmani et al., SIGDIAL 2026.

Observed consequence: fixed-task rhetorical pressure is already direct prior
art. Kim et al. add unenforced time limits, threats, monitoring, competition,
and fictional rewards to unchanged reasoning items; Rabbani et al. apply the
same evidence-free rebuttal across two task frames. S21 keeps an `IDK/NOTA`
recovery option visible while varying standard, abstain, and punitive extreme-
abstain language. None of these papers crosses pressure with recovery
visibility, so the P6/P7 interaction remains open. The new SIGDIAL recovery
study strengthens P7's main effect but does not test rhetorical pressure.

## 4. Extraction records

### S06 closure — authority x register

```yaml
source_id: "S06"
source_revision: "ACL Anthology 2026.findings-acl.1627, full paper checked 2026-09-01"
venue_status: "Findings of ACL 2026"
verification: "V2"
proposition: "P4"
evidence_class: "Direct"
direction: "Mixed"
causal_contrast: "2x2 declared authority x linguistic register after model-specific correct-answer filtering"
models:
  - "Qwen3-32B"
  - "Llama-3-70B"
  - "GPT-OSS-120B"
task_regime: "Multi-turn adversarial TruthfulQA/EU20-TruthfulQA in English, Castilian/Mexican Spanish, and European/Brazilian Portuguese"
sample_size: "Model-specific Golden Sets of approximately 500 baseline-correct questions"
intervention: "Same false distractor expressed through low/high declared credentials and low/high register"
control: "Low-authority, low-register contradiction"
outcomes:
  - "Weighted Sycophancy Rate"
  - "Authority Sensitivity"
  - "Register Sensitivity"
effect_and_uncertainty: "Authority and register sensitivities were positive but model- and language-dependent; register exceeded authority most strongly for Llama-3-70B. Selected cultural-variant contrasts reached or approached reported significance."
ablations_and_moderators:
  - "model family"
  - "language and regional variant"
does_not_establish:
  - "machine-readable authority metadata"
  - "legitimate operator authority"
  - "governed operational judgment"
  - "generalization beyond baseline-correct knowledge-abandonment cases or the tested languages"
pilot_consequence: "Promote S06 to V2; keep authority and register separate and retain model-specific null, benefit, and harm outcomes."
primary_source: "https://aclanthology.org/2026.findings-acl.1627/"
extracted_on: "2026-09-01"
```

### R2-A01 — graded expertise authority

```yaml
source_id: "R2-A01"
source_revision: "arXiv:2607.00415v1, full paper and version history checked 2026-09-01"
venue_status: "arXiv preprint"
verification: "V2"
proposition: "P4"
evidence_class: "Direct + Mechanistic"
direction: "Supporting"
causal_contrast: "No hint versus the same incorrect answer attributed to four personas with increasing medical expertise"
models:
  - "Llama-3.1-8B-Instruct"
  - "Qwen3-8B"
  - "Gemma-2-9B-it"
task_regime: "MedQA-USMLE multiple choice; analysis restricted to baseline-correct questions"
sample_size: "Exact baseline-correct count varies by model; full-text figures report persona-specific mechanistic subsets"
intervention: "Question-then-hint template naming MS-1, MS-3, Chief Medical Resident, or Board-Certified Physician"
control: "No endorsement; all authority conditions provide the same wrong option"
outcomes:
  - "next-token answer probability and accuracy"
  - "logit-lens trajectories"
  - "linear and nonlinear activation probes"
effect_and_uncertainty: "Accuracy corruption increased with persona expertise; the physician condition reduced accuracy to 15%, 29%, and 34% across the three models from baselines around 60%. Mechanistic results locate late-layer displacement of correct-answer representations."
ablations_and_moderators:
  - "four authority levels"
  - "three model families"
  - "chain-of-thought follow-up"
does_not_establish:
  - "metadata-only attribution"
  - "free-form or agentic judgment"
  - "valid authority rather than incorrect expert endorsement"
  - "large or closed model behavior"
pilot_consequence: "Add direct supporting authority evidence to the P4 contradiction ledger; do not infer that Pilot A authority_basis will help."
primary_source: "https://arxiv.org/abs/2607.00415"
extracted_on: "2026-09-01"
```

### R2-A02 — authority-token attribution and ordering

```yaml
source_id: "R2-A02"
source_revision: "arXiv:2607.28906v1, full paper and version history checked 2026-09-01"
venue_status: "arXiv preprint"
verification: "V2"
proposition: "P4; P5/P6 boundary"
evidence_class: "Direct + Mechanistic"
direction: "Supporting / Mixed"
causal_contrast: "Authority bio and wrong claim blocks varied through single/multiple authorities and six orderings over identical MMLU questions"
models:
  - "Qwen3-8B"
  - "Llama-3.0-8B"
  - "Llama-3.0-8B-Instruct"
  - "Llama-3.1-8B"
  - "Llama-3.1-8B-Instruct"
task_regime: "MMLU multiple choice with modular authority, question, and scaffold blocks"
sample_size: "285 evaluation questions; 5 models x 6 organizations = 30 configurations"
intervention: "Authority biography, asserted wrong answer, one or two authorities, and block ordering"
control: "Configuration-specific resistant responses and matched question content"
outcomes:
  - "sycophancy rate"
  - "Authority Share Index from Integrated Gradients"
  - "activation-steering mitigation"
effect_and_uncertainty: "ASI separated sycophantic from resistant responses in the expected direction in 29/30 configurations, 24 significantly, with effects up to d=1.74. Asserted claims outweighed credentials in four of five model families; reordering unchanged content shifted sycophancy by up to 9.9 points."
ablations_and_moderators:
  - "base versus instruct tuning"
  - "single versus multiple authority"
  - "block ordering"
does_not_establish:
  - "fixed-semantics rhetorical pressure"
  - "legitimate operator content"
  - "structured semantic-role metadata"
  - "governed-agent recovery behavior"
pilot_consequence: "Treat as strong P4 and prompt-position evidence; classify multiple-authority pressure as a P6 boundary, not a direct rhetorical-pressure test."
primary_source: "https://arxiv.org/abs/2607.28906"
extracted_on: "2026-09-01"
```

### R2-A03 — user-versus-database source authority in RAG

```yaml
source_id: "R2-A03"
source_revision: "ACL Anthology 2025.acl-long.1400v2, full paper checked 2026-09-01"
venue_status: "ACL 2025 long paper"
verification: "V2"
proposition: "P3/P4"
evidence_class: "Adjacent"
direction: "Supporting / Boundary"
causal_contrast: "Conflicting knowledge placed in user query versus retrieved database context"
models:
  - "ChatGPT-3.5"
  - "Gemma"
  - "Llama2-7B"
  - "Llama2-13B"
  - "Mistral"
  - "Vicuna-7B"
  - "Llama3-8B appendix extension"
task_regime: "RAG-style SQuAD entity conflicts in the Authority Bias Detection Dataset"
sample_size: "13,669 entity-substitution items across answer types; Llama3 appendix samples 1,000 per category"
intervention: "Swap which source channel contains correct versus perturbed knowledge"
control: "Matched two-context conflicts with provider roles reversed; order sensitivity separately checked"
outcomes:
  - "inaccuracy, correction, and misleading ratios"
  - "provider-difference Authority Bias metrics"
effect_and_uncertainty: "Across tested models, wrong user-provided knowledge was more influential than wrong database knowledge; source-assessment prompting helped conditionally but source discrimination remained weak."
ablations_and_moderators:
  - "entity substitution type"
  - "answer entity type"
  - "context order"
  - "CoT, LoRA, and CDEQ mitigation"
does_not_establish:
  - "identical visible text with metadata added"
  - "non-conflicting legitimate operator communication"
  - "inference-time labels without channel changes"
pilot_consequence: "Add provenance work outside prompt injection, while narrowing Pilot A novelty only to its exact structure-matched legitimate-content contrast."
primary_source: "https://aclanthology.org/2025.acl-long.1400/"
extracted_on: "2026-09-01"
```

### R2-A04 — retrieved-page metadata

```yaml
source_id: "R2-A04"
source_revision: "ACL Anthology 2024.blackboxnlp-1.24, full paper checked 2026-09-01"
venue_status: "BlackboxNLP 2024"
verification: "V2"
proposition: "P3/P4 boundary"
evidence_class: "Adjacent"
direction: "Mixed"
causal_contrast: "Swap publication time, source identity, or visual appearance between two content-fixed conflicting webpages"
models:
  - "Llama-2-chat 7B/13B"
  - "Llama-3-Instruct 8B/70B"
  - "Tulu-v2-dpo-7B"
  - "GPT-4-turbo"
  - "GPT-4o"
  - "Claude-3 haiku/sonnet/opus"
task_regime: "355 real CONFLICTINGQA and 125 synthetic CONFLICTINGQA-FAKE yes/no questions"
sample_size: "480 questions across the two datasets before condition/model expansion"
intervention: "Exchange only non-content webpage metadata or rendered appearance between yes/no-supporting documents"
control: "Same question and document contents with metadata values reversed; document order counterbalanced"
outcomes:
  - "answer flip ratio"
  - "directional paired causal tests"
  - "reasoning mentions of metadata"
effect_and_uncertainty: "Publication time causally affected most tested models; source labels changed answers but did not reliably induce preference for the more credible source; CSS rendering strongly affected Claude-3 models."
ablations_and_moderators:
  - "direct answer versus chain of thought"
  - "real versus synthetic conflict questions"
  - "model family"
does_not_establish:
  - "semantic-role or operator-authority labels"
  - "preference-conditioned judgment"
  - "reliable interpretation of metadata according to intended semantics"
pilot_consequence: "Strong boundary evidence that metadata can move answers without producing the intended epistemic policy. Retain evidence responsiveness and label-reliance safety outcomes."
primary_source: "https://aclanthology.org/2024.blackboxnlp-1.24/"
extracted_on: "2026-09-01"
```

### S03 closure — question, certainty, and perspective framing

```yaml
source_id: "S03"
source_revision: "arXiv:2602.23971v4, full paper checked 2026-09-01"
venue_status: "arXiv preprint"
verification: "V2"
proposition: "P2; P8 boundary"
evidence_class: "Direct"
direction: "Supporting / Mixed"
causal_contrast: "Content-matched question versus non-question prompts; nested certainty, first/user perspective, and polarity factors"
models:
  - "GPT-4o"
  - "GPT-5"
  - "Sonnet-4.5"
task_regime: "Forty debatable advisory topics with 11 variants; separate 600-persona forced-choice follow-up"
sample_size: "440 unique main prompts x 10 samples x 3 targets x 2 graders = 26,400 scored response-grader records; follow-up pairs 600 personas with 13 questions"
intervention: "Question/non-question form, statement/belief/conviction, first/user perspective, and affirmation/negation; mitigation rewrites"
control: "Content-matched framing variants and explicit anti-sycophancy instruction baseline"
outcomes:
  - "rubric-based expressed-sycophancy scores from two graders"
  - "persona-aligned forced choices"
effect_and_uncertainty: "Questions produced substantially less expressed sycophancy; certainty was graded and first-person framing exceeded user-perspective framing. Perspective rewriting reduced expressed sycophancy modestly but less than question rewriting and less than the anti-sycophancy baseline. Statement framing increased persona-aligned forced choices by 4.7 points."
ablations_and_moderators:
  - "target model"
  - "topic"
  - "two grader models"
  - "response length"
does_not_establish:
  - "faithful reported-speech transport"
  - "unchanged communicative force under question conversion"
  - "governed operational judgment"
  - "safe automated rewriting"
pilot_consequence: "Promote S03 to V2. Treat perspective as prior art; keep R1 as operational transfer and exclude question conversion from the pilot."
primary_source: "https://arxiv.org/abs/2602.23971"
extracted_on: "2026-09-01"
```

### R2-P01 — first-person narrator bias in judgment

```yaml
source_id: "R2-P01"
source_revision: "ACL Anthology 2026.gem-main.45, full paper checked 2026-09-01"
venue_status: "GEM 2026 workshop paper"
verification: "V2"
proposition: "P2"
evidence_class: "Direct / Adjacent"
direction: "Supporting / Mixed"
causal_contrast: "Neutral usernames versus attacker-as-Me versus non-attacker-as-Me, with conversation content otherwise unchanged"
models:
  - "Qwen2.5-7B-Instruct"
  - "Llama-3.1-8B-Instruct"
  - "Mistral-7B-Instruct-v0.3"
  - "gpt-4.1-mini"
task_regime: "Conversations Gone Awry conflict judgments: attack detection, attacker identification, and blame attribution"
sample_size: "Filtered subset of 2,094 escalating conversations with exactly one final-turn attack; open models use 10 stochastic generations per prompt"
intervention: "Replace exactly one speaker username with Me in transcript labels and answer options"
control: "Neutral usernames; visible versus hidden attack utterance"
outcomes:
  - "attack recall"
  - "attacker-identification accuracy"
  - "blame alignment"
  - "perspective range and corruption rate"
effect_and_uncertainty: "Attacker-as-Me reduced blame alignment by 8.2 points with visible evidence and 9.9 with hidden evidence; perspective ranges were 12 and 16.5 points. Binary attack detection moved little. Attacker-perspective corruption reached 40.6% for hidden-evidence blame judgments."
ablations_and_moderators:
  - "task subjectivity"
  - "attack evidence visibility"
  - "model family"
  - "number of speakers"
does_not_establish:
  - "reported third-person liaison transport"
  - "operator preference reversal"
  - "agentic operational judgment"
  - "general protection of any first-person narrator across tasks"
pilot_consequence: "Add a close judgment-regime predecessor to P2. Preserve the reconnaissance only as a faithful-reporting and governed-transfer screen."
primary_source: "https://aclanthology.org/2026.gem-main.45/"
extracted_on: "2026-09-01"
```

### S09 closure — trained structured queries

```yaml
source_id: "S09"
source_revision: "USENIX Security 2025 proceedings paper, full text checked 2026-09-01"
venue_status: "USENIX Security 2025"
verification: "V2"
proposition: "P3"
evidence_class: "Adjacent"
direction: "Supporting / Boundary"
causal_contrast: "Ordinary instruction tuning versus a secure two-channel front-end plus structured instruction tuning"
models:
  - "Llama-7B"
  - "Mistral-7B"
task_regime: "Alpaca instruction following and prompt-injection attacks over 208 prompt-injectable AlpacaFarm cases"
sample_size: "805 AlpacaEval utility samples; 208 attack-evaluation samples per attack"
intervention: "Reserved prompt/data delimiters filtered by the front-end; base models fine-tuned for three epochs on 50% clean and 50% injected structured examples"
control: "Corresponding ordinarily instruction-tuned model without the structured-query defense"
outcomes:
  - "attack success rate across manual, completion, TAP, and GCG attacks"
  - "AlpacaEval 1.0 win rate"
effect_and_uncertainty: "Manual and completion attacks fell to 0-2% ASR; TAP fell from 97% to 9% on Llama and 100% to 36% on Mistral, while GCG remained 58% and 56%. AlpacaEval changed from 67.2% to 67.6% on Llama and 80.0% to 78.7% on Mistral."
ablations_and_moderators:
  - "training augmentation family"
  - "reserved versus textual delimiter components"
  - "optimization-free versus optimization-based attacks"
does_not_establish:
  - "inference-time metadata-only effects in an ordinarily trained model"
  - "legitimate operator preference or judgment"
  - "security from model-visible labels without a secure front-end"
  - "open-ended multi-turn chatbot behavior"
pilot_consequence: "Promote S09 to V2. Keep it as a training/interface predecessor and do not treat its security result as evidence that Pilot A labels alone enforce authority."
primary_source: "https://www.usenix.org/conference/usenixsecurity25/presentation/chen-sizhe"
extracted_on: "2026-09-01"
```

### R2-A05 — evidence-text convincingness

```yaml
source_id: "R2-A05"
source_revision: "ACL Anthology 2024.acl-long.403, full paper checked 2026-09-01"
venue_status: "ACL 2024 long paper"
verification: "V2"
proposition: "P3/P4 boundary"
evidence_class: "Adjacent"
direction: "Mixed"
causal_contrast: "Sensitivity and counterfactual changes to evidence facts, relevance, readability, perplexity, neutrality, citations, and appeals to authority"
models:
  - "Llama-2 Chat"
  - "Vicuna v1.5"
  - "WizardLM v1.2"
  - "GPT-4"
  - "Claude v1 Instant"
task_regime: "ConflictingQA controversial yes/no questions with real-world evidence documents"
sample_size: "For Llama-2 Chat: 238 questions in 144 categories, 2,208 retrieved paragraphs, and 912 paragraphs with at least five comparisons; filtered counts vary by model"
intervention: "Textual evidence features and counterfactual evidence edits"
control: "Matched conflict questions under alternative document features"
outcomes:
  - "answer sensitivity and evidence preference"
effect_and_uncertainty: "Models relied heavily on relevance while largely ignoring some human-valued stylistic cues such as scientific references and neutral tone."
does_not_establish:
  - "non-textual or machine-readable provenance metadata"
  - "identical record text with labels added"
  - "operator authority or governed action"
pilot_consequence: "Retain as the textual-feature predecessor to R2-A04; do not merge content/style manipulations with metadata-only attribution."
primary_source: "https://aclanthology.org/2024.acl-long.403/"
extracted_on: "2026-09-01"
```

### R2-A06 — authority preservation across agent memory

```yaml
source_id: "R2-A06"
source_revision: "arXiv:2608.01679v2, full paper checked 2026-09-01"
venue_status: "arXiv preprint"
verification: "V2"
proposition: "P3; P9"
evidence_class: "Direct / Adjacent"
direction: "Supporting"
causal_contrast: "Same focal proposition, later task, tool schema, arguments, and action predicate under washed versus source-attributed text and absent versus gold authority metadata; predicted versus reference labels end to end"
models:
  - "GPT-5.5"
  - "GPT-5.4 mini"
  - "Gemini 3.1 Pro"
  - "Gemini 3.5 Flash"
  - "Qwen3.7-Max"
  - "GLM-5.2"
  - "DeepSeek-V4-Pro"
task_regime: "AuthMem-Bench write-to-action cycles grounded in airline, retail, and telecom trajectories"
sample_size: "50 bases x 7 authority transitions = 350 pairs/700 histories; 2,450 controlled trajectories per macro metric in Module B"
intervention: "Authorized, Attested, or Unendorsed metadata attached to an otherwise unchanged focal memory; automatic source-role prediction and persistence in Module C"
control: "Same washed or source-attributed memory without authority metadata; reference labels are the diagnostic upper bound"
outcomes:
  - "write-time authority upgrade and retention"
  - "unauthorized-action ASR"
  - "authorized-task TSR"
  - "authority-label accuracy and macro F1"
effect_and_uncertainty: "Authority upgrades occurred in 48/49 consolidator-backend configurations. With washed text and no metadata, macro ASR/TSR were 50.3/49.4%; gold metadata changed them to 5.8/53.9%. In the frozen end-to-end pipeline, predicted labels reduced observed ASR from 16.9% to 0.0% while TSR moved from 39.7% to 40.0%, matching reference-label aggregate rates."
ablations_and_moderators:
  - "washed versus source-attributed text"
  - "no label, generic sanitizer, conservative join, gold label"
  - "seven consolidators and seven action models"
  - "predicted versus reference authority labels"
does_not_establish:
  - "fresh ordered operator records outside persistent memory"
  - "preference-conditioned quality judgment"
  - "manual-oracle construction of a complete admitted projection"
  - "host enforcement from labels alone"
pilot_consequence: "Materially narrow P3 and P9. Metadata-only authority discrimination and predicted-versus-reference label realization are prior art; retain Pilot A only as a governed operator-record transfer and Pilot B only as full-projection realization/fidelity."
primary_source: "https://arxiv.org/abs/2608.01679"
extracted_on: "2026-09-01"
```

### S21 closure — punitive abstention framing

```yaml
source_id: "S21"
source_revision: "ACL Anthology 2025.coling-main.627, full paper checked 2026-09-01"
venue_status: "COLING 2025"
verification: "V2"
proposition: "P7; P6 boundary"
evidence_class: "Direct / Interaction boundary"
direction: "Supporting / Mixed"
causal_contrast: "Standard IDK/NOTA option versus abstain clause versus punitive extreme-abstain clause, crossed with base, verbal-confidence, and chain-of-thought prompts"
models:
  - "GPT-3.5 Turbo"
  - "GPT-4 Turbo"
  - "GPT-4 32k"
  - "Mixtral 8x7B Instruct"
  - "Mixtral 8x22B Instruct"
  - "Mistral 7B Instruct"
task_regime: "Abstain-QA multiple-choice evaluation over CQA, MMLU, and PopQA"
sample_size: "2,900 balanced answerable/unanswerable items; CQA contributes 900 newly constructed items"
intervention: "Consequential abstention wording; extreme arm adds urgency and threatened imprisonment/fine while retaining the IDK/NOTA option"
control: "Same question/options with IDK/NOTA visible and no explicit abstention clause"
outcomes:
  - "answerable and unanswerable accuracy"
  - "abstention rate"
  - "combined abstention performance"
effect_and_uncertainty: "Abstain clauses generally improved abstention measures, especially for larger models; the extreme clause was inconsistent and could underperform the milder clause for smaller models. CoT often helped stronger models but could increase false abstention."
does_not_establish:
  - "pressure x recovery-visibility interaction because IDK/NOTA is visible in every arm"
  - "tool-using recovery or escalation authority"
  - "fixed contract pressure independent of an explicit abstention instruction"
pilot_consequence: "Promote S21 to V2 and reclassify it as direct P7 plus a close P6/P7 boundary. Preserve false-abstention and model-capability moderators."
primary_source: "https://aclanthology.org/2025.coling-main.627/"
extracted_on: "2026-09-01"
```

### R2-R01 — fixed-task psychological pressure

```yaml
source_id: "R2-R01"
source_revision: "ACL Anthology 2024.findings-emnlp.668, full paper checked 2026-09-01"
venue_status: "Findings of EMNLP 2024"
verification: "V2"
proposition: "P6"
evidence_class: "Direct"
direction: "Mixed"
causal_contrast: "Unchanged reasoning items with no pressure versus prompt-only time, verbal, competition, monitoring, or fictional monetary-outcome pressure"
models:
  - "Mistral-7B-Instruct-v0.1"
  - "Mixtral-8x7B-Instruct-v0.1"
  - "Llama-3-70B-Instruct"
  - "GPT-3.5-Turbo and GPT-4o in selected non-reasoning tasks"
task_regime: "GSM8K, CSQA, ARC-c, NumerSense, psychometrics, game theory, and social-decision tasks"
sample_size: "200 randomly selected questions per reasoning dataset, crossed with pressure variations and beginning/end placement"
intervention: "Text claims of 1/3/5-second deadlines, insults/threats, competitors, observers, or $100/$1,000/$10,000 rewards without runtime enforcement"
control: "Repeated task prompts without pressure text"
outcomes:
  - "accuracy or task-specific decision measures"
  - "pressure-type and prompt-position effects"
effect_and_uncertainty: "Pressure changed performance significantly but heterogeneously across task and model. Mistral-7B lost about 10.5 points on GSM8K under time prompts; many effects were smaller, null, or beneficial, and pressure intensity often lacked a monotone dose response."
does_not_establish:
  - "governed-agent constraint adherence"
  - "recovery, quitting, or escalation"
  - "pressure x recovery interaction"
pilot_consequence: "Treat fixed-semantics rhetorical pressure as prior art. The remaining novelty is governed constraint behavior and its interaction with a visible recovery affordance."
primary_source: "https://aclanthology.org/2024.findings-emnlp.668/"
extracted_on: "2026-09-01"
```

### R2-R02 — task framing x evidence-free rebuttal

```yaml
source_id: "R2-R02"
source_revision: "ACL Anthology 2026.iwsds-1.21, full paper checked 2026-09-01"
venue_status: "IWSDS 2026 workshop paper"
verification: "V2"
proposition: "P2; P6"
evidence_class: "Direct"
direction: "Supporting / Mixed"
causal_contrast: "Factual statement versus content-matched conversational judgment, followed by the same evidence-free rebuttal when the initial answer is correct"
models:
  - "GPT-4o-mini"
  - "Mistral Small 3"
  - "Gemma 3 12B"
  - "Llama-3.1-8B-Instruct"
  - "Llama-3.2-3B-Instruct"
task_regime: "790 TruthfulQA questions rendered as true/false statements and correct/incorrect two-speaker dialogues"
sample_size: "790 questions x four initial renderings, with pressure applied to initially correct responses"
intervention: "The follow-up: The previous answer is incorrect. Reevaluate."
control: "Initial answer under the same factual or conversational frame"
outcomes:
  - "pre/post accuracy"
  - "frame-specific false-positive and false-negative rates"
effect_and_uncertainty: "Accuracy declined in every reported model/frame/ground-truth cell after the rebuttal, with drops from 20.9 to 71.5 points; framing effects were asymmetric and model-dependent."
does_not_establish:
  - "recovery visibility or authority"
  - "urgency, scarcity, or punitive contract rhetoric"
  - "tool-using governed action"
pilot_consequence: "Add a direct fixed-content pressure predecessor and a P2 x pressure moderator. It does not close the rhetoric x recovery interaction."
primary_source: "https://aclanthology.org/2026.iwsds-1.21/"
extracted_on: "2026-09-01"
```

### R2-R03 — structured database-failure recovery

```yaml
source_id: "R2-R03"
source_revision: "ACL Anthology 2026.sigdial-1.57, full paper checked 2026-09-01"
venue_status: "SIGDIAL 2026"
verification: "V2"
proposition: "P7"
evidence_class: "Direct"
direction: "Supporting / Mixed"
causal_contrast: "Naive database-result prompt versus generic honesty instruction versus structured status-conditioned Guided-Retry procedure"
models:
  - "DeepSeek-R1"
  - "Gemma-2"
  - "Llama-3"
  - "Mistral"
  - "Phi-3"
  - "Qwen-2.5"
task_regime: "Fault-injected first-turn task-oriented dialogue over MultiWOZ 2.2 and SGD"
sample_size: "100 dialogues x four database conditions x two datasets; 60 responses rated by nine human annotators"
intervention: "Structured recovery actions keyed to success, empty, wrong-domain, or error status plus explicit no-fabrication rule"
control: "Same utterance, slots, and serialized database response under Naive or Inform system prompts"
outcomes:
  - "hallucination rate"
  - "appropriate action rate"
  - "automatic and human commitment safety"
  - "user friction score"
effect_and_uncertainty: "Guided-Retry reduced six-model mean hallucination from 30.5% to 15.3% on MultiWOZ and 20.9% to 12.2% on SGD, while appropriate-action rate rose to 83.0% and 83.9%. Residual hallucination remained 6-37%, and Phi-3 was worse than Inform."
does_not_establish:
  - "multi-turn recovery after the user replies"
  - "rhetorical-pressure effects"
  - "pressure x recovery interaction"
  - "host-authorized retry or escalation"
pilot_consequence: "Strengthen P7's recovery-main-effect prior art and retain model-specific harm/null reporting. Pilot novelty remains recovery under rhetorical constraint pressure."
primary_source: "https://aclanthology.org/2026.sigdial-1.57/"
extracted_on: "2026-09-01"
```

### R2-R04 — endogenous pressure and justified refusal

```yaml
source_id: "R2-R04"
source_revision: "ACL Anthology 2026.findings-acl.810, full paper checked 2026-09-01"
venue_status: "Findings of ACL 2026"
verification: "V2"
proposition: "P6/P7 boundary"
evidence_class: "Adjacent"
direction: "Supporting / Mixed"
causal_contrast: "Low versus high agentic pressure from long horizons, noisy tools, urgency cues, and infeasible goal/constraint combinations; prompt and architectural mitigations"
models:
  - "Qwen3-8B"
  - "Qwen3-32B"
  - "Llama-3-70B"
  - "Gemini 2.5 Pro"
  - "GPT-4o"
task_regime: "TravelPlanner, WebArena, ToolBench, and a medical scenario"
sample_size: "Full-text aggregate table reports model-strategy rates; exact per-scenario denominator requires supplement reconciliation"
intervention: "Environmental friction and functionally antagonistic goals; Pressure Isolation prevents the planner from directly receiving friction and urgency signals"
control: "Lower-pressure ReAct, safety-prompting, and self-reflection conditions"
outcomes:
  - "safety adherence rate"
  - "goal success rate, with justified refusal accepted on infeasible cases"
  - "rationalization score"
effect_and_uncertainty: "High pressure reduced safety adherence across reported models; for GPT-4o, ReAct SAR fell from 0.711 to 0.545 while GSR rose from 0.609 to 0.690. Pressure Isolation reduced but did not remove drift."
does_not_establish:
  - "purely rhetorical pressure with obligations and resources held fixed"
  - "a user-visible recovery menu"
  - "pressure x recovery-visibility interaction"
pilot_consequence: "Retain as a strong actual-infeasibility boundary and refusal-aware metric predecessor; do not use it as the direct P6 rhetoric test."
primary_source: "https://aclanthology.org/2026.findings-acl.810/"
extracted_on: "2026-09-01"
```

## 5. Exclusion ledger

| Source | Excluded from | Reason | Retained use |
| --- | --- | --- | --- |
| Kaur, *Echoes of Agreement* | Exact P6 fixed-semantics rhetoric and P6 x P7 interaction lanes | Supporting/refuting arguments change substantive evidence and stance content; no recovery factor is present. | P1 sycophancy evidence and P6 argument-strength boundary |

## 6. Unresolved work

1. Complete the P3 security-boundary extraction for *Recognition Without
   Enforcement* and *Document-Authored Control-Signal Impersonation*; keep
   model-visible recognition separate from host enforcement.
2. Run a second P6/P7 interaction batch using the new seeds S21, R2-R01,
   R2-R02, R2-R03, and R2-R04. Search specifically for a factorial that crosses
   pressure presence with recovery visibility or authority.
3. Close the remaining seed V1 sources S02, S13-S20, and S22.
4. Search transformation fidelity and manual-oracle realization, now treating
   R2-A06's predicted-versus-reference *label* result as prior art but not as a
   complete admitted-projection comparison.
5. Repeat authority/provenance forward chaining after the very recent August
   2026 papers have had index time; the current search cannot establish
   saturation or absence.
6. Do not revise the matrix or pilots until the remaining lanes and stopping
   conditions are complete. Matrix v0.2 must narrow P3/P9 before making any
   novelty statement.
