Me: I was just thinking about criminal investigation and how work engine would be an interesting match for that. Gathering evidence, decomposing hypothesis. I could see a system that, instead of building software, builds a representation of a case with theories, branches, evidence connections etc. Building a representation of a case would be the same from investigators on up to lawyers (same basis). 
Seems like an intermediate format would be needed in most domains with reports etc built off of that. In the software domain, we treat the code itself as the final format.

Sol:
Yes — I think you’ve hit on something fairly fundamental about Work Engine’s domain model.

In software, we get away with treating **the repository as both the evolving representation of the problem and the principal output artifact**. The architecture, dependencies, implementation decisions, and much of the evidence about what exists are concretized directly in code. Tests, plans, and documentation orbit that executable representation.

A criminal investigation makes the missing layer much easier to see because there is no equivalent artifact you can “build” directly. The investigation needs to construct a **durable semantic representation of the case**, and *then* produce human artifacts from it.

Something like:

```text
raw observations / records / testimony / physical evidence
                         │
                         ▼
              evidence normalization
                         │
                         ▼
              ┌─────────────────────┐
              │      CASE IR        │
              │                     │
              │ claims              │
              │ evidence            │
              │ provenance          │
              │ events/timeline     │
              │ entities            │
              │ relationships       │
              │ hypotheses          │
              │ contradictions      │
              │ uncertainty         │
              │ missing evidence    │
              │ alternative theories│
              └─────────────────────┘
                    │     │     │
          ┌─────────┘     │     └──────────┐
          ▼               ▼                ▼
 investigative       prosecutor's       defense
    report               theory           theory
          │               │                │
          ▼               ▼                ▼
      briefing         charging         motions /
      material         documents          briefs
```

And I think your point about **investigators through lawyers sharing the same basis** is particularly important. They shouldn't necessarily share the same *judgments*, permissions, or purposes, but they can operate over the same underlying representational substrate.

That starts looking strikingly similar to what you've been building.

An investigator node might decompose:

> Was Smith physically capable of being at location X at 21:14?

into subordinate inquiries:

- where Smith was observed,
- travel-time constraints,
- phone-location reliability,
- witness reliability,
- vehicle evidence,
- alternative interpretations.

Those inquiries return evidence-backed propositions rather than prose. The parent can accept some, leave others unresolved, reject unsupported inferences, and form higher-level hypotheses.

A prosecutor could later consume exactly those propositions while applying a different contract:

> Is there sufficient admissible evidence supporting each element of offense Y?

And a defense team could operate against the same representation with:

> What alternative explanations remain consistent with the admitted evidence, and what prosecution propositions depend upon contested evidence?

The **case representation doesn't have to choose among them**. That is crucial. It records that theory A exists, what claims it depends upon, what evidence supports or undermines those claims, who made judgments about them, and under what rules.

That maps almost eerily well onto your distinction between **evidence, claims, judgments, authority, and projection**.

### I think this exposes a general Work Engine architecture

There may be three broad layers rather than simply “plan → execution → artifact”:

```text
                         DOMAIN INPUTS
                              │
                              ▼
                   governed investigation
                              │
                              ▼
             ┌──────────────────────────────┐
             │   DOMAIN SEMANTIC STATE / IR │
             │                              │
             │ What does the organization   │
             │ currently know/believe/      │
             │ dispute, and why?            │
             └──────────────────────────────┘
                       │             │
                  execution      projection
                       │             │
                       ▼             ▼
                change the       reports
                represented      decisions
                world/state      explanations
                                 filings
                                 plans
```

The middle artifact is the interesting bit.

For criminal investigation it might be a **Case IR**.

For medicine it could be a longitudinal clinical representation.

For scientific research, an experiment/evidence/claim graph.

For business strategy, a market/organization/decision representation.

For incident response, an evolving causal incident model.

And for software engineering, the unusual thing is that a lot of this gets collapsed into:

```text
repository state
```

because code is such an extraordinarily rich machine-readable representation.

Even there, though, Work Engine is already discovering that the repository is **not sufficient**. Your evidence basis, claims, architecture state, decisions, contracts, provenance, unresolved judgments, and execution state increasingly form an intermediate semantic layer *around* the code. The code represents what the system **is**, but not necessarily everything Work Engine needs to know about **why it is that way, what alternatives were considered, how strongly something is known, and which conclusions remain unresolved**. 

So software might actually be:

```text
evidence + semantic architecture state + repository state
                         │
                         ▼
                 Work Engine IR
                         │
               ┌─────────┴────────┐
               ▼                  ▼
         repository changes   projections
                              plans/reviews/
                              documentation
```

rather than being an exception with no IR at all.

And there's another consequence I like: **reports stop being state**.

A police report isn't the case.
A legal brief isn't the case.
A medical note isn't the patient model.
An architecture document isn't necessarily the architecture.

They're **projections of semantic state for a particular consumer, purpose, authority, and point in time**.

That fits very naturally with the publication/projection distinctions you've already been teasing apart in Work Engine. 

I could imagine the eventual Work Engine core being almost indifferent to whether the thing being constructed is software or a criminal case. The domain adapter would primarily define **what kinds of propositions exist, what constitutes admissible evidence, what transformations/judgments are permitted, and what projections are meaningful**. The recursive investigation, evidence acquisition, decomposition, competing-vantage structure, provenance, and authority machinery could remain largely the same.

That's a much more general framing than “agentic software development system.” It's closer to a **governed semantic construction engine** where software happens to be the first domain.

Me: I wonder if we should consider building in that intermediate format or maybe it mostly exists and isn't conceptually joined.

Sol:
I think it mostly exists already — but as a **federation of semantic states that we have deliberately kept separately owned**. What may be missing is the thing that says, “taken together, these are the current semantic representation of the work subject.”

That distinction matters a lot. I would **not** start by inventing a new authoritative `WorkEngineIR` that owns everything. That would undo a lot of the ownership decomposition we’ve spent time establishing.

Right now we already have something remarkably close:

```text
Evidence / observations
        │
        ▼
Evidence + Claims
        │
        ├──────────► provenance / revisions / reliance
        │
        ▼
Semantic Planning / Plan IR
        │
        ├──────────► decisions / unresolved questions
        │
        ▼
Implementation basis / contracts
        │
        ▼
Organizational Compilation
        │
        ├──────────► topology / vantages / authority
        │
        ▼
Runtime Realization
        │
        ▼
repository / effects
```

Alongside that are status grammar, publication, revision/CAS, admission, projection, review findings, architecture adoption state, and so on. We have repeatedly arrived at the same rule: expensive reasoning crosses a boundary and becomes **revisioned semantic consequence**, while downstream actors consume bounded projections instead of having to reconstruct the reasoning. 

That's awfully close to an IR architecture already.

### What's missing may be the *join*

I think we've been looking at the architecture mostly vertically:

> Who owns this proposition?  
> Who may change it?  
> How is it revised?  
> How does it become executable?

Your criminal-case thought makes us look horizontally instead:

> **What is the complete current representation of the subject being worked on?**

For a case:

```text
Case C
 ├─ entities
 ├─ events
 ├─ evidence
 ├─ claims
 ├─ hypotheses
 ├─ contradictions
 ├─ unresolved questions
 ├─ judgments
 ├─ decisions
 └─ provenance
```

No single component necessarily owns all of that.

Likewise, Work Engine could say:

```text
Software subject S
 ├─ repository state
 ├─ evidence state
 ├─ claims
 ├─ accepted semantic plans
 ├─ architecture decisions
 ├─ implementation contracts
 ├─ organizational state
 ├─ runtime realization state
 ├─ unresolved judgments
 └─ provenance/status
```

That **whole thing is the intermediate representation**, despite no single owner owning it.

That makes me think what we're missing is closer to a **federated semantic representation** than another semantic dimension.

### I would resist making it one giant schema

There's a seductive bad version of this:

```text
SemanticNode {
    type
    value
    status
    relationships[]
}
```

and then shove everything into a universal graph.

That would make Work Engine look beautifully general while quietly erasing the semantics we've worked hard to preserve. A planning decision, evidence claim, runtime realization, and authority grant aren't interchangeable nodes just because they can all be represented as JSON.

Instead, I think the intermediate representation should primarily contain **references between domain-owned objects**.

For example:

```text
SubjectRepresentationRevision
    subject = work-engine

    evidence_basis    -> EvidenceBasisRevision 81
    semantic_plan     -> PlanRevision 14
    architecture      -> ArchitectureStateRevision 22
    implementation    -> ImplementationBasisRevision 9
    organization      -> OrganizationRevision 31
    realization       -> RuntimeRealizationRevision 46
    repository        -> git:035eef3...
```

And then typed relationships can cross those boundaries:

```text
claim C17
    supports -> planning proposition P8

decision D4
    resolves -> planning question Q2

implementation obligation I12
    realizes -> decision D4

test T91
    provides evidence for -> implementation obligation I12

runtime realization R3
    instantiates -> logical vantage V7
```

The representation doesn't become the owner of `C17`, `D4`, `I12`, or `R3`.

It says:

> **These authoritative things, at these exact revisions, collectively constitute the current semantic representation of this subject.**

That's very compatible with the architecture we've built.

And it answers something we've bumped into repeatedly: claims are useful materializations but **must not become replacement truth owners**; organizational compilation may refine realization but cannot rewrite accepted semantic meaning; projections expose scoped semantic state without becoming its owner. 

### Then reports become compilation targets

This is where your criminal-investigation example really clarifies things.

Once that joined representation exists:

```text
                         Subject IR
                             │
       ┌─────────────────────┼──────────────────────┐
       ▼                     ▼                      ▼
 investigator report     legal brief          operator UI
       ▼                     ▼                      ▼
 narrative prose         argument             visualization
```

Those outputs become **compiled projections**.

And software fits too:

```text
                         Subject IR
                             │
           ┌─────────────────┼────────────────┐
           ▼                 ▼                ▼
      repository         documentation      review packet
      realization          projection        projection
```

The repository is unusual because it isn't merely a report—it is an executable realization of part of the semantic state. But it still isn't the whole representation.

That actually explains some of the complexity we've been encountering. We kept discovering things that **cannot live in code**:

- evidence provenance,
- unresolved alternatives,
- confidence/readiness,
- architecture adoption,
- judgments and their authority,
- why something exists,
- which semantic revision an implementation realizes,
- stale/superseded state.

Those aren't incidental metadata. They're pieces of the intermediate representation that a source tree cannot express adequately.

### This also connects directly to our recent “new owner or representation?” question

We've just been pressure-testing whether various concepts require a new semantic owner, shared mechanism, non-owning representation, or merely composition over existing owners. 

I think this idea should enter through exactly that gate.

My initial hypothesis would be:

**It is a non-owning representation over existing semantic owners until evidence proves otherwise.**

That is much safer than declaring a new Work Engine dimension.

Then we can ask what the representation needs that no existing owner supplies. If we find something genuinely unowned, *that particular semantic fact* gets an owner rather than making the entire IR authoritative.

There may be one especially interesting candidate for genuinely new semantics:

> **What constitutes the current subject representation?**

In other words, somebody or something may need to own the revision that says:

```text
Case revision 37 consists of:
 evidence revision 71
 hypothesis topology revision 12
 judgment set revision 28
 ...
```

That isn't ownership of the contained semantics. It's ownership of **composition/version correspondence**.

We've seen an analogous unresolved issue already around lineage/correspondence between revisions: individual domains can own their revisions while nobody necessarily owns the judgment that *these revisions belong together*. 

That might be the small architectural hole hidden inside this much bigger-looking idea.

So I wouldn't propose “let's add an IR layer” yet.

I'd phrase the discovery more narrowly:

> **Work Engine appears already to contain multiple domain-owned intermediate representations. Investigate whether it lacks a first-class, revision-bound, non-owning composition of those representations into the semantic state of the work subject, from which execution and consumer-specific projections can be derived.**

That feels like the right pressure test. And if it survives, it could be one of those changes that makes the architecture suddenly look far more domain-general without requiring us to generalize the existing dimensions themselves.

