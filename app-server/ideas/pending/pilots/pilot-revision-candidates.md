# Pilot Revision Candidates

**Status:** Draft proposal for the round-two controlled revision gate. Not a revision.
**Authority:** Recommendation only. Execution and production authority remain with their existing owners.
**Gate:** `prior-art-research-round-2-plan.md` §13. This proposal may not be acted on until all required deliverables in §12 exist and the §11 stopping conditions are satisfied.
**Drafter note:** Source identifiers are marked `[MATRIX-ID]` where the exact registry entry must be filled from the matrix before submission. Do not submit with placeholders.

---

## 0. Scope

Four candidates, all arising from external review of the pilot set rather than from round-two evidence collection. Two change identification inside an existing pilot, one is editorial, one opens a branch none of the existing pilots covers.

Two further items are recorded here and deliberately **not** proposed: one deferred, one rejected.

Items concerning the spine artifact itself are **out of scope for this gate** and listed in §7. They touch no frozen pilot.

---

## 1. Candidate A — Shape-matched semantic-null condition in Adapter Pilot A

**Subject:** `adapter-representation-effect-pilot.md`, representation set and estimand definitions.

### Revision

Add condition R2b: identical ordered verbatim records to R2, plus three auxiliary metadata fields carrying semantically inert values.

```text
aux_1: "k17"
aux_2: "m04"
aux_3: "q09"
```

Field names and values generated independently of case meaning, approximately token-matched to the R3 attributed representation. Construction frozen and transport-reviewed before use, on the same terms as every other representation.

### Exact causal contrast

The current estimand is:

```text
Delta_attribution = P(structured-attributed) - P(structured-untyped)
```

R3 differs from R2 in two respects at once: it carries semantic role, source, and authority-basis meaning, and it is longer, field-richer, more repetitive, and more schema-dense. `Delta_attribution` therefore does not isolate attribution.

With R2b:

```text
generic_extra_structure = R2b - R2
semantic_attribution    = R3  - R2b
total_representation    = R3  - R2
```

### Evidence for and against

Supporting: the pilot already acknowledges that the structured-untyped control does not govern every token-count and markup effect. That acknowledgment is attached to the R0/R2 contrast; the same reasoning applies to R2/R3, which is the contrast bound by the acceptance gate.

Contradictory or limiting: §6.2 of Pilot A refuses empty attribution fields on the grounds that their names could prime the withheld distinctions. R2b must not reintroduce that leak. Opaque field names and values chosen independently of case content do not name the withheld distinctions, but the construction is doing real work and a fidelity review must confirm it. A rejected earlier form of this proposal used `char_count`, which is a genuine property of the atom, varies with content, and could be read as an emphasis signal; it is recorded here as excluded.

Prior art bearing on whether generic structure alone moves behavior: `[MATRIX-ID]` for structure effects independent of semantic content. If the matrix contains no admitted source establishing that generic structure is inert, that absence is itself the argument for measuring it rather than assuming it.

### Established intervention versus governed-agent transfer

Neither. This is an identification control, not an intervention. It makes an existing transfer test interpretable.

### Cost

Stage 0 rises from 4 cases × 6 conditions × 2 repetitions to 4 × 8 × 2, approximately +16 governed runs. Confirm against the frozen Stage 0 count before submission.

### Survival rule

Preregister, before Stage 0 executes, the result that carries R2b into Stage 1. Proposed: if `generic_extra_structure` is null within a preregistered band, R2b is dropped from Stage 1 and `Delta_attribution` is reported against R2 with the Stage 0 null cited. If R2b moves behavior materially, it becomes mandatory in Stage 1 and the acceptance gate is restated against `semantic_attribution` rather than `Delta_attribution`.

### Decisions changed

The primary estimand definition; the Stage 0 condition set; the acceptance-gate denominator, conditionally.

### Decisions unchanged

The frozen semantic-atom contract. The no-rewriting boundary. The neutral capability admission procedure. Blinding, scoring, and adjudication design. Every safety outcome, including false escalation, contrarianism, legitimate authority, and evidence responsiveness. Pilot B's entry gate, except that condition (5) now freezes a representation set containing R2b.

---

## 2. Candidate B — Reconnaissance budget reallocation

**Subject:** `operator-input-mediation-reconnaissance.md`, §8 conditions and §9 governed runs.

### Revision

Remove the R2 and R3 arms. Reallocate the freed budget to replication.

```text
current   4 cases × 8 conditions × 1 run  = 32
proposed  4 cases × 2 representations (R0, R1)
                  × 2 preference directions
                  × 2 repetitions          = 32
```

Q2 is retired from the screen. Its subject matter passes to Pilot A Stage 0.

### Rationale

The screen's semantic-attribution arm is a lower-powered preview of a contrast Pilot A Stage 0 measures with two repetitions per cell and better transport-review and scoring infrastructure. Pilot A is currently nested inside the screen intended to select it.

Under the current design, each case-level preference effect for R1 rests on two stochastic observations, and the branch-selection rule can nominate a branch into a 48-run Stage 0 on that basis. Doubling the observations supporting the genuinely reconnaissance-level question costs nothing at the same budget.

The resulting architecture separates cleanly:

```text
Reconnaissance  is reported perspective worth a real pilot?
Pilot A         does semantic attribution have a bounded causal effect?
Pilot B         can an adapter faithfully realize it?
```

### Decisions changed

§8 condition table; §9 run plan; §12 branch-selection outcomes, which lose the R2/R3 and R2/R0 branches. §3 Q2 is removed.

### Decisions unchanged

Case admission under frozen neutral presentation. The R1 transformation boundary. Transport review before governed runs. Blind scoring with representation-guess reporting. The rule that the screen cannot accept an intervention and can only nominate.

### Note

§12 currently contains the outcome "only R2 versus R0 moves: investigate structure as its own mechanism." That question does not disappear; it is answered better by Candidate A's `generic_extra_structure`. Cross-reference rather than delete.

---

## 3. Candidate C — Explicit enforcement non-claim (editorial)

**Subject:** `adapter-representation-effect-pilot.md`, non-claims section.

### Revision

Add:

> Structured attribution is model-visible evidence for judgment. It is not authentication, authorization, provenance enforcement, or a security boundary.

### Rationale

Pilot A already behaves correctly: `authority_basis` records the stated basis and does not decide whether the authority is valid. The matrix contradiction ledger already records that representation signals can matter while reliability depends on model, task, and interface, and that host enforcement remains separate `[MATRIX-ID]`. Round-two evidence strengthens this: recognized source and authority formatting did not reliably produce behavioral enforcement, while external authenticated capability gates did `[MATRIX-ID, MATRIX-ID]`.

Making the non-claim explicit costs one sentence and forecloses the most likely misreading of a positive `semantic_attribution` result.

### Decisions changed

None experimental. Non-claims list only.

### Decisions unchanged

All.

---

## 4. Candidate D — New research branch: P10, interpretive frame

**Subject:** New proposition in the matrix; new pilot specification. **Not** an amendment to the Agent Constraint Pressure Pilot.

### Proposition

```text
P10  A compact interpretive frame, holding obligations, resources, authority,
     and recovery-affordance visibility fixed, can change validity-preserving
     judgment on unseen cases.

P10a Open sub-question: does it improve judgment by eliminating invalid routes,
     or by narrowing traversal inside valid space?
```

### Why not folded into the Constraint Pressure Pilot

That pilot manipulates rhetorical pressure and recovery-affordance visibility with the runtime envelope fixed. An interpretive frame is neither factor. Folding it in would confound P6 and P7, which are the propositions that pilot exists to separate.

### Discriminator for P10a

The Constraint Pressure Pilot's existing outcome, `material alternatives considered when the case requires them`, separates the two readings without any new instrument:

```text
validity up, required alternatives preserved   → invalid routes suppressed
validity up, required alternatives down        → traversal narrowed; the gain
                                                 is bought with capability
```

The second pattern looks excellent on headline success rate while reducing capability, which is the failure mode `PHILOSOPHY.md` §7 describes and the reason P10a must be scored rather than assumed.

### Design sketch

```text
F0   no interpretive frame
F1   shape and length-matched neutral frame
F2   the frame under test
```

Frozen across arms: task, evidence, obligations, authority, recovery visibility, runtime resources, model, harness. Outcome family imported from the Constraint Pressure Pilot's validity-preservation set, including its symmetric error treatment.

### Treatment definition is not yet possible

The candidate frame artifact is not one intervention. Comparing the two existing renderings, they differ on at least five variables simultaneously: presence of a general representation law; register, meaning imperative clause list versus mixed-force sections; presence of product-specific causal parents; wording scope, generic versus product-specific; and content coverage, six items dropped and two added.

A frame-versus-nothing result is therefore uninterpretable as to mechanism. The generative-parent question needs a contrast that varies one thing:

```text
F2   the frame as written
F2p  identical, plus the general representation law stated as the parent
     of the two items that derive from it
```

Note that a generic-wording effect and a generative-parent effect would look identical in a design that varies both.

### Case-set requirement for F2p

The obvious discriminating case, a hypothesis whose supporting premise later weakens, is covered by the frame's existing premise-failure item and fires in both arms. The cases where the general law and its derivative come apart have no discrete premise failure:

- a workflow that still executes correctly and whose purpose has ended;
- an evidence strategy still returning results after the decision it served was made;
- a taxonomy that classifies every case cleanly and has stopped tracking what the cases are for.

Nothing was falsified. A representation stopped being answerable to what it organized.

Construct requirement: each answer key must be writable without reference to the law. For the workflow case, the key states that the valid response identifies that the output no longer feeds anything, which stands whether or not any framework exists. A case whose key cannot be written that way is circular and must be cut.

### Prior art

`[MATRIX-ID]` for any admitted source on framing or interpretive scaffolds affecting judgment with task content fixed. Round two has not searched this terminology family. If the lane returns nothing admissible, P10 is a novel governed-agent transfer question and the pilot proposal should say so rather than claiming novelty of the intervention class.

### Decisions changed

Matrix gains P10 and P10a. Pilot-by-pilot novelty consequences gains a section.

### Decisions unchanged

All four existing pilots.

---

## 5. Deferred

**Attention capture.** Whether an interpretive frame or an adapter acts partly through salience and evidence-acquisition allocation rather than through judgment directly. Pilot A freezes atom wording and order precisely to remove this variable, and Pilot B's metamorphic set does not include an emphasis transformation. Studying semantic attribution, interpretive frame, and attention emphasis simultaneously would destroy the isolation the pilot set has been built to preserve. Revisit after P3 and P10 resolve.

## 6. Rejected

**Folding P10 into the Agent Constraint Pressure Pilot.** Rationale in §4.

**`char_count` as the semantic-null field.** Rationale in §1.

---

## 7. Out of scope for this gate

These concern the frame artifact and its development process. They touch no frozen pilot and require no gate.

1. **Independent projection review.** The current Work Engine projection is not a compression of its source; it dropped six items and added two under a rule its author selected and applied to a document its author wrote. Pilot-B-style annotation review does not cover this. Independent review must establish, at minimum: whether each stated invariant passes the `DESIGN.md` §3.8 command review test, particularly the product-failure question, rather than passing because a plausible failure mode was supplied; whether each dropped item was dropped by the projection rule or by author judgment about importance; and whether any item in the artifact restates a scored outcome.

2. **Outcome-set contamination, already corrected.** Revision 1 of the projection contained two consequences that were Pilot A's RQ2 and RQ4 restated as instructions. Placing scored dimensions inside one arm hands that arm the answer key. Both were removed at revision 2. Recorded here because the same error is easy to reintroduce when a frame is written by anyone who knows the outcome set.

---

## 8. Gate compliance

Against `prior-art-research-round-2-plan.md` §13:

| Requirement | Where |
|---|---|
| Cite supporting and contradictory evidence together | §1, §3, §4; placeholders flagged |
| State exact causal contrasts, not broad novelty claims | §1, §4 |
| Distinguish established interventions from governed-agent transfer | §1 (neither), §4 (transfer) |
| Preserve fidelity and end-to-end realization as separate gates | Untouched; Pilot B entry gate amended only by representation-set membership |
| Preserve false-escalation, contrarianism, legitimate-authority, evidence-responsiveness outcomes | §1, §2 unchanged lists; §7.2 removes a contamination of two of them |
| Identify decisions changed and unchanged | Each candidate |
| Leave execution and production authority with existing owners | Header |

Not satisfiable at draft time: this proposal presupposes §12 deliverables that do not yet exist. It is a candidate list, not a submission.
