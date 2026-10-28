# Governed-Agent Prior-Art Research: Round Two Plan

**Status:** Active research protocol; evidence collection begun 2026-09-01
**Protocol version:** 0.1
**Search cutoff:** To be frozen when all stopping conditions are met
**Authority:** Research and recommendation only; this protocol does not revise,
freeze, authorize, or execute a pilot
**Parent ledger:**
`prior-art-matrix-governed-agent-input-and-constraint-pilots.md`, seed version
0.1

## 1. Bound research subjects

Round two interprets the following exact source revisions. A changed digest makes
the affected interpretation stale until it is reconciled.

| Subject | SHA-256 at protocol start |
| --- | --- |
| `pilots/prior-art-matrix-governed-agent-input-and-constraint-pilots.md` | `b1e264a082e469606465a1dbf71000d7641ab49334871693a83ce35b9ecf3807` |
| `operator-input-mediation-reconnaissance.md` | `5a6f6f10efea15d8303e9c74768a112d769d2ed6bb5d4529fac43501e0d44d19` |
| `adapter-representation-effect-pilot.md` | `d688d7fa3b98cab1fa9834aff7f6e6168f7781e49314964dfe56bc46f935efef` |
| `operator-input-adapter-fidelity-pilot.md` | `808b44e2d1f9e629fc4400316f96954a11e26203044281d9e9d2111bc4a0ba2f` |
| `agent-constraint-pressure-pilot.md` | `d7036cbe27081a14e754979859373fe53230b6d7b9ebee9e3710ec32e981df11` |

The matrix remains the evidence ledger. The pilot specifications remain the
experimental source of truth until the controlled revision gate in this plan is
satisfied.

## 2. Objective

Produce an auditable matrix revision that is sufficient to decide, for every
causal proposition P1-P9:

1. what is an established premise;
2. what is a bounded transfer or calibration question;
3. what exact causal contrast remains open;
4. what has been weakened, contradicted, or preempted; and
5. what consequence, if any, follows for a pilot specification.

Round two is an evidence-closing audit, not a search for a persuasive novelty
narrative. Supporting, null, contradictory, mixed, and boundary results receive
the same admission treatment.

## 3. Protected distinctions

Research must not collapse these distinctions:

- evidence discovery versus evidence admission;
- adjacent evidence versus direct evidence;
- novelty of an intervention class versus novelty of its governed-agent
  transfer;
- transformation fidelity versus downstream efficacy;
- recovery visibility versus recovery authority;
- rhetorical pressure versus actual semantic or runtime scarcity;
- model-visible metadata versus host-enforced authority or provenance;
- absence of an admitted source versus proof that no source exists;
- research recommendation versus authority to revise or execute a pilot.

## 4. Research priorities

### 4.1 Priority A: semantic attribution and authority

Primary propositions: P3 and P4.

Test the apparent novelty of the structure-matched contrast:

```text
identical ordered records with verbatim text
versus
the same records plus semantic_role, source, and authority_basis
```

Search beyond prompt-injection security into source credibility, evidence
provenance, metadata effects, multi-source RAG, information extraction,
multi-agent evaluation, and authority-conditioned sycophancy. Preserve whether
the intervention changes text, channel, training, visible metadata, or claimed
authority.

### 4.2 Priority B: rhetorical pressure and recovery

Primary propositions: P6 and P7.

Search for experiments that manipulate urgency, scarcity-associated wording,
punitive framing, stacked imperatives, or argument pressure while holding actual
obligations and resources fixed. Separately search recovery, quitting,
escalation, abstention, and timely stopping.

The central novelty threat is evidence for the interaction:

```text
rhetorical pressure x visible recovery affordance
```

Do not treat general constraint-count, long-context, or actual-resource studies
as tests of rhetorical pressure.

### 4.3 Priority C: transformation fidelity and realization

Primary propositions: P8 and P9.

Search prompt rewriting, semantic equivalence, meaning preservation, intent
drift, assumption insertion, lossy summarization, multi-agent communication,
human-oracle representation, and generated-versus-manual projection. Record
fidelity and downstream utility separately.

The key end-to-end contrast is:

```text
raw input
versus automated admitted projection
versus manual-oracle admitted projection
```

### 4.4 Priority D: perspective transfer and moderators

Primary propositions: P2 and P4.

Search first-person, third-person, reported speech, narrative perspective,
persona, evidence visibility, authority, register, and communicative force.
Preserve the exact intervention; grammatical-person substitution, a named
third-person persona, question conversion, and faithful liaison reporting are
not interchangeable.

## 5. Round structure

### Pass 1: verification closure

Resolve every V1 source that may support a pilot or novelty statement. Promote
it to V2 only after relevant full text has established the recorded setup and
result. Otherwise narrow the claim, retain V1 with an explicit unresolved
field, or exclude the source.

### Pass 2: backward and forward chaining

Search citations separately for matrix groups S03-S11, S12-S14, S17-S19, and
S23. Record which seed produced each lead.

### Pass 3: exact causal-combination search

Run the priority searches above, including:

- perspective x authority;
- authority x provenance;
- rhetoric x abstention;
- constraints x recovery;
- evidence visibility x perspective;
- manual oracle x generated representation.

### Pass 4: null, reversal, and boundary search

Repeat the main terminology families with terms such as `null`, `no effect`,
`reversal`, `failure`, `degradation`, `over-refusal`, `excessive abstention`,
`intent loss`, and `utility tradeoff`.

### Pass 5: synthesis and controlled revision proposal

Reconcile evidence by causal proposition and prepare matrix v0.2, an exclusion
ledger, a contradiction ledger, and a pilot-impact memo. Do not edit pilot
claims during earlier passes.

## 6. Search sources

Use primary or official sources for admission:

- arXiv paper records and versioned full text;
- ACL Anthology proceedings records and PDFs;
- AAAI proceedings records and papers;
- OpenReview forum records and papers;
- USENIX proceedings;
- official publisher full text when it is the primary record.

Search engines, indexes, author pages, citation graphs, and secondary prose may
produce leads. They cannot by themselves support an admitted result.

## 7. Admission criteria

A source may enter the evidence matrix only when a primary abstract, official
proceedings record, or full text establishes a relevant empirical or formal
result.

For empirical admission, record:

- the manipulated or observed variable;
- control or comparison condition;
- model names and versions;
- task regime and sample size;
- prompt, training, interface, or runtime difference;
- outcome and metric;
- effect direction and uncertainty when reported;
- material ablations and moderators;
- verification level;
- the closest proposition;
- what the study does not establish;
- the bounded pilot consequence.

Formal or conceptual work may be admitted only as Mechanistic, Boundary, or
Analogy evidence with its inferential limit explicit.

## 8. Exclusion criteria

Record rather than silently discard serious near-matches. Common reasons
include:

- no primary or official source available;
- no relevant empirical or formal result;
- changes communicative force rather than transporting it;
- changes training rather than inference-time representation;
- concerns untrusted injected content rather than legitimate operator input;
- tests answer abstention rather than agent action;
- changes actual resources rather than rhetoric;
- changes text and attribution together without an isolating control;
- reports only aggregate utility without fidelity evidence;
- duplicate or superseded version;
- insufficient protocol detail for the proposed claim.

An excluded source may remain a citation-chain lead.

## 9. Evidence-card schema

Each admitted experiment or distinct result receives one card with:

```yaml
source_id: ""
source_revision: ""
venue_status: ""
verification: "V1 | V2"
proposition: "P1-P9"
evidence_class: "Direct | Adjacent | Mechanistic | Analogy"
direction: "Supporting | Null | Contradictory | Mixed | Boundary"
causal_contrast: ""
models: []
task_regime: ""
sample_size: ""
intervention: ""
control: ""
outcomes: []
effect_and_uncertainty: ""
ablations_and_moderators: []
does_not_establish: []
pilot_consequence: ""
primary_source: ""
extracted_on: ""
notes: ""
```

Do not force unavailable information into a zero or negative result.

## 10. Search and extraction record

The append-oriented working record is
`prior-art-research-round-2-log.md`. It owns:

- exact queries and search dates;
- databases or endpoints searched;
- leads and their originating query or citation;
- V1 extraction state;
- exclusions and reasons;
- unresolved access or interpretation issues;
- saturation observations.

The matrix owns admitted evidence and synthesized judgments. The log owns the
research route and unresolved work. Pilot documents own experimental contracts.

## 11. Stopping conditions

A priority lane is complete only when:

1. every relevant seed V1 source is resolved or explicitly left unresolved;
2. planned backward and forward citation searches are recorded;
3. exact-combination searches are recorded;
4. null, reversal, and boundary searches are recorded;
5. serious near-matches have dispositions; and
6. two consecutive query or citation batches yield no new admissible Direct or
   materially decision-relevant Adjacent source.

Clean saturation is a bounded search judgment, not proof that no undiscovered
source exists.

## 12. Required deliverables

Round two closes with:

1. matrix version 0.2 with a frozen search cutoff;
2. resolved source registry and experiment-level evidence cards;
3. query and citation-chain log;
4. exclusion ledger;
5. contradiction and boundary ledger;
6. peer-review and revision-status audit for 2026 preprints;
7. P1-P9 disposition table;
8. pilot-impact memo identifying changed and unchanged decisions; and
9. a controlled joint-revision proposal for the four current pilot documents.

## 13. Controlled revision gate

Pilot specifications may be proposed for joint revision only after all required
deliverables exist and the stopping conditions are satisfied. The proposal
must:

- cite supporting and contradictory evidence together;
- state exact causal contrasts instead of broad novelty claims;
- distinguish established interventions from governed-agent transfer tests;
- preserve fidelity and end-to-end realization as separate gates;
- preserve false-escalation, contrarianism, legitimate-authority, and evidence-
  responsiveness safety outcomes;
- identify each pilot decision changed by prior art and each decision left
  unchanged; and
- leave execution and production authority with their existing owners.

Research completion does not itself authorize pilot execution or production
mediation.

## 14. Initial research order

Begin with two parallel evidence lanes inside the same research record:

1. resolve the existing V1 registry, starting with sources that may alter P3,
   P4, P6, P7, or P9; and
2. admit or exclude the already discovered P3/P4 authority, provenance,
   metadata, and perspective leads.

This ordering tests the most decision-relevant novelty boundaries before
expanding lower-value background coverage.
