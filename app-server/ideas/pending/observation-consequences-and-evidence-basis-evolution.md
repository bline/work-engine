# Observation Consequences and Evidence-Basis Evolution

## Status and authority

Exploratory architectural idea, synthesized from a multi-turn reconciliation
discussion (Claude and Sol, prompted and directed by the operator) that
pressure-tested [`active-evidence-acquisition-during-execution.md`](active-evidence-acquisition-during-execution.md)
and [`evidence-basis-topology-for-bounded-inference.md`](evidence-basis-topology-for-bounded-inference.md)
against each other and against `PHILOSOPHY.md`.

This document does not accept a design, assign a permanent semantic owner,
change the roadmap, authorize implementation, grant evidence access, increase
any role's authority, establish a new architectural dimension, or select
among the candidate classifications it records (§ Candidate classification
hypotheses). It narrows a question the two source documents both raised but
did not resolve, for later intake and reconciliation.

```yaml
idea_provenance:
  origin: synthesized_from_raw_material
  raw_material: >-
    inline multi-turn reconciliation discussion in this session between
    Claude and Sol, reviewing active-evidence-acquisition-during-execution.md
    and evidence-basis-topology-for-bounded-inference.md; not separately
    preserved as an ideas/history/ transcript
  captured_on: 2026-09-24
```

The candidate novelty is narrow:

> An attributed observation produced by a lawful probe does not imply one
> generic evidence-admission transition. It may independently affect
> episode-local inquiry state, durable evidence state, judgment-relative
> topology, and later evidence projection — with different owners,
> durability, publication requirements, and authority boundaries. Work
> Engine has not yet determined whether coordinating these consequences
> requires only composition over existing owners, a non-owning
> representation, a shared publication protocol, a shared evidence-basis
> mechanism, or a distinct semantic owner.

Whether this is a new dimension, a shared mechanism, a non-owning
representation, or pure composition over existing owners is explicitly
unresolved (§ Candidate classification hypotheses).

---

## Recognition: "accepted" was hiding several different questions

The acquisition document states the evidence basis evolves as
`E1 = E0 + admitted consequence of O1`, and immediately warns that not every
observation becomes canonical evidence. Read together with the topology
document, that single `+` sign is compressing at least three separately
owned questions into one symbol:

```text
O1 = attributed observation

Does O1 change what the current inquiry episode should do next?
Does O1 establish a durable evidentiary fact?
Does O1 nominate, change, or contradict a relationship in the
    decision-relative evidence topology?
```

A concrete example makes the split visible:

```text
runtime probe result:
    "the failure occurs only when capability X is absent"

episode-local inquiry consequence:
    enough discrimination exists to stop investigating hypothesis A

durable evidence consequence:
    the observation is an exact-subject, attributed runtime result

candidate semantic/topological consequence:
    capability X may be relevant to this implementation judgment
    (a nomination, not yet an accepted relationship)

decision-readiness consequence:
    possibly none yet -- required independent confirmation may be absent
```

These four facts can diverge freely. An observation can end an inquiry
episode without ever becoming durable evidence; it can become durable
evidence without changing any topology relationship; it can nominate a
topology relationship without that nomination being accepted; and none of
the above guarantees a decision becomes `ready`.

---

## Two nested loops, not a five-stage pipeline

Evidence does not move through acquisition, admission, topology,
readiness, and decision as one ordered pipeline. The source documents
describe two loops operating at different grains, interacting through
related but non-identical evidence state and projections -- the inquiry
loop potentially producing changes that a later readiness assessment
consumes, never one shared, undifferentiated basis:

```text
                  EXECUTION-TIME INQUIRY LOOP  (episode-local grain)

 current episode-local basis / projection
              |
              v
 consequential uncertainty
              |
              v
     choose lawful probe
              |
              v
    attributed observation
              |
      (fans out -- see next section)
              |
              v
   inquiry disposition: continue_inquiry / sufficient_for_action /
                         tolerate_uncertainty / block / escalate
              |
              v
          loops again, or stops


              --------------------------------

             READINESS LOOP  (decision grain)

     published/current evidence basis
                  +
       readiness contract for decision D
                  |
                  v
          ready / blocked / uncertain
                  |
                  v
        informs the decision owner (never becomes it)
```

The readiness loop reads a *published* basis; it does not itself decide
what enters that basis. The inquiry loop decides what to investigate next
and when to stop *for the consequence the executor is currently authorized
to realize*; it does not decide whether the resulting basis is sufficient
for a named downstream decision. Collapsing these into one "sufficiency"
concept would silently merge two different owners.

### At least three sufficiency concepts, not one

```text
inquiry sufficiency
    enough support to stop probing / take the next delegated
    consequence within an execution episode
    (judged within the acquisition loop under the governing
    execution contract; permanent ownership remains unresolved)

decision readiness
    enough evidence under readiness contract C to permit decision
    authority to consider decision D ready
    (owner: decision-specific-readiness.md)

plan readiness
    a narrower, downstream sufficiency judgment over an already-
    compiled implementation contract, evaluated after a decision has
    already been made and sealed
    (owner: implementation-contract-compilation.md's own stage-1
    contract characterization; decision-specific-readiness.md SS6
    already states this is a genuinely different fact from its own
    pre-decision judgment)
```

`decision-specific-readiness.md` already refuses to let its own dimension
collapse a proposal-absolute reading into a decision-relative one, and
already refuses to let "plan readiness" be conflated with its own
pre-decision sufficiency judgment. This document adds a third: inquiry
sufficiency is a distinct, earlier, episode-local judgment that neither of
the other two owns or should absorb.

---

## The observation fan-out

An attributed observation's consequences fan out into (at least) three
independently governed branches, only two of which are candidates for
durable publication. A fourth stage -- projection -- is deliberately kept
downstream and separate, because collapsing "durable" and "projected" was
an earlier drafting error in this same discussion, corrected below.

```text
                         attributed observation
                                  |
             +--------------------+---------------------+
             |                    |                      |
             v                    v                      v
      inquiry consequence    evidence consequence   semantic nomination
      for this episode       / establishment         / relationship effect
             |                    |                      |
             |                    |               domain-owned judgment
             |                    |                      |
             v                    v                      v
      episode-local state    candidate durable       candidate topology
      / uncertainty          evidence revision        consequence
      (may never leave            |                      |
       the episode)                v                      v
             |             evidence/claim revision   topology revision
             |             (evidence-and-claims.md   (if topology exists
             |              owns identity, lineage,   as a durable artifact
             |              provenance, reliance)      at all -- open,
             |                    |                    SS Candidate
             |                    |                    classification)
             |                    |                      |
             |                    +-- references by ------+
             |                        exact revision
             |                        (never merges into
             |                         one owner or one
             |                         revision type)
             |                               |
             |                               v
             |                      projection compilation
             |                     (a separate transformation,
             |                      not implied by publication)
             |                               |
             |                               v
             |                      episode-local projection
             |                               |
             |                               v
             |                            inference
             |                               |
             +-------------------------------+
                                              v
                                  realized salience exists only
                                  in this realization -- not an
                                  intrinsic property of the
                                  evidence item or the projection
```

Two disciplines this diagram is deliberately protecting, both stated
directly by the topology document and confirmed by re-reading it after an
earlier draft of this diagram collapsed them:

1. **An observation need not mutate durable state at all.** It may affect
   only the current inquiry episode's disposition and never become a
   candidate for durable evidence or topology revision.
2. **A durable evidence revision and a durable topology revision are not
   the same publication, and must not be drawn as one merged box.** The
   topology document is explicit: *"The scaffold may reference claims and
   anchor observations. It cannot become a second claim store, dependency
   registry, relevance owner, or readiness judge."* `evidence-and-claims.md`
   remains the sole owner of claim identity, lineage, and reliance; a
   topology artifact, if one is ever built, would reference a claim
   revision by exact identity rather than co-own a merged revision with it.
3. **A durable topology being correct does not mean a given projection
   renders it well, and a projection being well-formed does not mean a
   given inference episode finds the decisive part of it salient.** The
   topology document treats canonical/durable topology, model-facing
   projection, and realized salience as three separate facts for exactly
   this reason -- "too dense" is a candidate defect in selection,
   transformation, or rendering, not a license to delete canonical
   evidence.

Neither the inquiry role nor the evidence/topology machinery may collapse
these on its own. The topology document's own word for the topology-facing
half is already careful about this: *"The inquiry role may add an
observation or nominate a relationship. It cannot silently author a
semantic edge, close a material decision, or upgrade its own observation
into accepted source truth."* Nomination is not admission.

---

## Candidate central recognition

> A lawful probe producing an attributed observation does not imply one
> generic evidence-admission transition. The observation may independently
> affect episode-local inquiry state, durable evidence state,
> judgment-relative topology, and later evidence projection. These
> consequences may have different owners, durability, publication
> requirements, and authority boundaries. Work Engine has not yet
> determined whether coordinating them requires only composition over
> existing owners, a non-owning representation, a shared publication
> protocol, a shared evidence-basis mechanism, or a distinct semantic
> owner.

---

## Candidate classification hypotheses

Not five competing designs -- five increasingly strong claims, meant to be
tested in order rather than assumed. `evidence-basis-topology-for-bounded-inference.md`
already named four possible outcomes for itself (new dimension / shared
mechanism / representation / pure composition); this section restates them
as ordered, falsifiable hypotheses specifically about the observation-fan-out
seam, and keeps "representation" distinct from "shared mechanism" -- an
earlier draft of this section compressed them into one hypothesis and lost
a real, testable difference.

```text
H0 -- Pure composition
Existing evidence/claims, Evidence Anchor, domain-owned judgment,
Revision/CAS, and readiness already determine everything. No new
artifact needs to be built anywhere; only a cross-view description
is missing.

H1 -- Representation (single domain, non-owning)
One domain (for example, execution-time inquiry, or one judgment
contract) needs an explicit artifact -- a graph, a manifest, a
scaffold -- to make existing owner relationships, gaps, and
projection lineage visible. That artifact owns no new semantic
truth and is not required or reused by other domains.

H2 -- Shared publication protocol
No new semantic owner exists, but several independent domains
repeatedly need the same observation -> candidate consequence ->
attributed judgment -> successor revision protocol, even though
each domain's judgment content remains its own.

H3 -- Shared evidence-basis mechanism
Reusable state or lifecycle exists beyond publication alone:
predecessor binding, candidate-consequence queues, conflict
preservation, nomination tracking, revision lineage, or
projection-invalidation propagation shared across domains.

H4 -- New semantic dimension
Some irreducible truth about observation consequences exists that
none of the existing domains -- acquisition, evidence-and-claims,
Evidence Anchor, topology-as-representation, or readiness --
currently owns or could be made to own by enriching its own
contracts.
```

### Falsifiers

```text
Falsifies H0 (rules out pure composition):
    A durable consequence of an observation cannot be derived from
    an existing evidence owner or attributed to an existing domain
    judgment using ordinary Revision/CAS publication alone.

Distinguishes H1 from H2/H3 (rules out "this generalizes" claims):
    Does the same observation -> candidate-consequence -> judgment
    -> successor-revision shape recur, independently, across two or
    more domains that do not already share a mechanism -- or is it
    needed by exactly one domain, with no other consumer able to
    reuse the same protocol without inventing new semantics of its
    own? A representation built for one domain and never reused
    is H1, however useful it is locally; only recurrence across
    genuinely independent domains supports H2 or stronger.

Distinguishes H2 from H3 (rules out "protocol is enough"):
    Do the recurring needs extend past a shared publish/attribute
    shape into genuinely shared *state* -- conflict preservation,
    predecessor binding, projection-invalidation propagation -- that
    a bare protocol convention could not carry? If a documented
    convention over existing Revision/CAS suffices, H2 stands and H3
    does not.

Supports H4 (would require a new owner):
    A failure or a required judgment is correctly localized to
    "observation consequence routing" itself -- not to the initial
    representation, an existing domain's judgment, inquiry
    selection, or readiness -- and no existing or composable owner
    can be made to hold it without acquiring authority it does not
    already have (for example, evidence-and-claims.md acquiring
    topology-relevance authority, or the acquisition loop acquiring
    readiness authority).
```

A prior estimate in this discussion favored something in the H2/H3 range.
That estimate is explicitly not this document's conclusion -- the point of
recording ordered falsifiers is to test the hypotheses, not to pre-select
one and gather supporting material for it.

---

## Relationship to existing architecture

- [`active-evidence-acquisition-during-execution.md`](active-evidence-acquisition-during-execution.md) --
  owns probe selection, the inquiry loop, and the two stopping bounds
  (sufficiency floor, marginal-value ceiling). This document narrows one
  part of its open question 2 ("Is an inquiry episode durable semantic
  state or only attributed telemetry?") by separating it from the
  admission question entirely: the inquiry episode's own durability status
  and the observation's durable-evidence/topology consequences are
  different facts, not one question with two names.
- [`evidence-basis-topology-for-bounded-inference.md`](evidence-basis-topology-for-bounded-inference.md) --
  supplies the canonical-topology/projection/realized-salience split this
  document depends on throughout, and the "nominate, not author" boundary
  for the inquiry role. This document narrows its open question 1 ("Does
  evidence topology own any truth not already declared...") to the
  specific observation-consequence seam rather than the topology construct
  as a whole.
- [`../../docs/architecture/decision-specific-readiness.md`](../../docs/architecture/decision-specific-readiness.md) --
  owns decision-specific sufficiency judgments and, separately, already
  distinguishes itself from "plan readiness." This document adds inquiry
  sufficiency as a third, earlier concept that readiness does not own and
  must not absorb.
- [`../../docs/architecture/evidence-and-claims.md`](../../docs/architecture/evidence-and-claims.md) --
  remains the sole owner of claim identity, lineage, provenance, and
  reliance; a candidate durable-evidence consequence in this document's
  fan-out diagram is exactly this dimension's own concern, never a second
  owner for it.
- [`../../docs/architecture/substrates/evidence-anchor.md`](../../docs/architecture/substrates/evidence-anchor.md) --
  already produces weak `may_affect` nomination candidates through
  mechanical correspondence; this document's "semantic nomination /
  relationship effect" branch is a broader, inquiry-driven sibling of that
  same nominate-don't-author discipline, not a replacement for it.
- [`../../docs/architecture/implementation-contract-compilation.md`](../../docs/architecture/implementation-contract-compilation.md) --
  owns "plan readiness," the third sufficiency concept named above.
- [`governed-causal-diagnosis-and-epistemic-learning.md`](governed-causal-diagnosis-and-epistemic-learning.md) --
  the post-episode counterpart; a failure correctly localized to this
  document's fan-out (rather than initial representation, inquiry
  selection, or judgment) would be relevant evidence for its own causal
  attribution, not a competing diagnosis mechanism.
- `PHILOSOPHY.md` SS18, "Truth, establishment, and authority must remain
  separate" -- states independently, outside this idea thread entirely,
  that constraint, recording, and observation "answer different questions"
  and are "composable mechanisms rather than steps or a ladder," and that
  an unestablished claim is not resolved by whoever is nearest to the
  observation. This document's refusal to let inquiry disposition,
  durable-evidence establishment, topology nomination, and readiness
  collapse into one actor's progressive authority is a direct application
  of that already-accepted principle, not a novel constraint invented for
  this idea alone.

---

## Failure modes

### Evidence-event log masquerading as semantic state

Every observation gets durably published merely because it affected an
episode-local inference, producing an unbounded log that looks like
evidence but was never subjected to any domain-owned judgment. Directly
warned against by the acquisition document's own text: not every
observation becomes canonical evidence.

### Merged-owner diagram error

Durable evidence revision and topology revision get drawn, designed, or
implemented as one object or one store, silently giving whichever
mechanism materializes first (most likely evidence-and-claims.md, as the
more mature dimension) authority over topology relevance it was never
granted.

### Sufficiency collapse

Inquiry sufficiency, decision readiness, and plan readiness get merged
into one status field or one "is this done" boolean, losing the fact that
identical evidence can be sufficient for one and insufficient for another
simultaneously -- exactly the failure `decision-specific-readiness.md`
already prevents for its own two neighbors, now extended to a third.

### Premature classification

H1 (a useful, non-owning, single-domain representation) gets built and
then treated as though it already proves H2 or H3 merely because it
exists and is useful, without any second domain actually needing the same
protocol.

### Projection mistaken for canonical state

A rendering, compression, or salience concern (projection is too dense,
too sparse, poorly ordered) gets "fixed" by mutating or deleting canonical
evidence or topology, rather than by changing the projection
transformation.

### Nomination mistaken for admission

An inquiry-produced relationship nomination gets treated as an accepted
topology edge without ever passing through the domain-owned judgment the
topology document requires.

---

## Non-goals

This document does not currently propose:

- a single "evidence admission" gate, service, or authority;
- requiring durable publication of every observation merely because it
  affected episode-local inference;
- a relevance owner for topology nominations (deliberately left open by
  both source documents, and left open here);
- merging inquiry sufficiency, decision readiness, and plan readiness into
  one status, field, or owner;
- a merged durable store for evidence revisions and topology revisions;
- selecting among H0-H4 (§ Candidate classification hypotheses);
- a numerical or scalar measure of observation importance, relevance, or
  consequence strength;
- implementing a new service, schema, protocol, or role from this capture.

---

## Questions for later intake and reconciliation

1. Do durable evidence consequences and topology consequences of the same
   observation ever need to be co-published as one revision for
   consistency reasons, or does exact cross-referencing by revision
   identity always suffice?
2. Who is authorized to convert a topology nomination into an accepted
   relationship, and does that authority differ by judgment contract,
   architectural layer, or domain?
3. Is inquiry sufficiency ever consumed as an input to decision readiness
   (for example, as one of several required facets), or are they fully
   independent judgments that happen to read the same evidence basis?
4. Does H1 (a useful single-domain representation) recur often enough,
   across independent domains, to become real evidence for H2 or H3 --
   and what would count as "independent" enough to avoid
   double-counting one recurring author's habits as architectural
   recurrence?
5. What mechanically distinguishes a candidate durable-evidence
   consequence from a candidate topology consequence when a single
   observation plausibly produces both -- is this a property of the
   observation, the probe, the judgment contract, or the domain that
   requested the probe?
6. Does `evidence-and-claims.md`'s existing `may_affect` /
   nominate-refresh machinery already supply enough of the "nominate,
   don't author" discipline that a separate topology-nomination pathway
   would only be H1 for one domain, not new infrastructure?
7. If a reusable consequence-publication protocol (H2) is eventually
   warranted, does it belong nearer to Revision/CAS (infrastructure) or
   nearer to evidence-and-claims.md (semantic substrate), and does that
   placement choice itself require a new owner or merely an extension of
   an existing one?

---

## Compact hypothesis

> What consequences may an attributed observation legitimately produce for
> episode-local inquiry state, durable evidence state, judgment-relative
> topology, and later evidence projection; which of those consequences are
> mechanically derivable and which require domain-owned semantic judgment;
> and does coordinating them require any new owner, or only a shared
> publication discipline layered over evidence-and-claims.md,
> Evidence Anchor, decision-specific-readiness.md, and the judgment
> contracts that already exist? This document does not answer that
> question. It replaces "how does evidence get accepted?" with a version
> precise enough to be tested against the five ordered hypotheses above.
