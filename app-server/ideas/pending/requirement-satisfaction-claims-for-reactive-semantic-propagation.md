# Requirement-Satisfaction Claims for Reactive Semantic Propagation

## Status and authority

Exploratory recognition/scoping note, synthesized from a multi-turn
reconciliation discussion (Claude and Sol, prompted and directed by the
operator) that began by pressure-testing a candidate "reactive consequence
activation" mechanism and, over several rounds, discovered that most of what
it would have needed is already present -- at varying maturity -- in
`proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/`.

**This note does not propose a new generic reactive or satisfaction
mechanism.** An earlier, more elaborate candidate (working title
"Revision-Bound Requirement Satisfaction and Transition Detection") was
deliberately not drafted once this recognition surfaced. This note does not
accept `requirement-satisfaction-v1` as a real profile, does not assign it a
permanent owner, does not authorize any implementation, does not extend
`organizational-facts-v1`'s exploratory status, and does not decide whether
Decision-Specific Readiness assessments specifically should ever be
materialized this way. It asks a single pressure-testable question and
records the falsifiers for it.

```yaml
idea_provenance:
  origin: synthesized_from_raw_material
  raw_material: >-
    inline multi-turn reconciliation discussion in this session between
    Claude and Sol; not separately preserved as an ideas/history/ transcript
  captured_on: 2026-09-25
```

The candidate novelty is narrow, and it is a recognition, not a proposal:

> Work Engine may not need a new mechanism to let durable semantic state
> changes reactively surface latent, authorized-but-dormant work. Requirement
> or readiness judgments might be materialized as ordinary claims, in which
> case the already-approved claim refresh, exact-revision reliance, and
> selective-reopening semantics already supply the propagation this thread
> was trying to design from nothing -- provided readiness materialization
> does not collapse the ownership boundaries those semantics already exist to
> protect.

---

## 1. Recognition

Across several rounds of decomposing a "lightning fires when a standing
requirement becomes satisfied" idea, every safety property this thread
independently derived -- nomination without authority, exact-revision
reliance that does not silently follow successor revisions, a downstream
owner (not the claim layer) deciding whether a change matters, and a claim
event never activating an inactive role on its own standing -- turned out to
already be written down, in almost identical shape, in this repository's own
claim-maintenance-and-reliance-propagation proposal. That proposal was never
about reactive cognition activation when it was formed; it was about keeping
claims fresh. The recognition is that its propagation shape may generalize
to the reactive-activation problem without requiring a single new
architectural owner -- if, and only if, a readiness or requirement judgment
can be truthfully expressed as a claim in the first place.

---

## 2. Status boundary (verified directly against the source documents)

The claim-maintenance-and-reliance-propagation proposal contains two
genuinely different maturities under one semantic model, and this note's
entire premise depends on keeping them separate:

```text
LOWERED FARTHER -- design: accepted, operation contract passed

nominate_impact -> open_refresh_episode -> publish_refresh_judgment
    fully specified in operation-contract-surface.md, "passed after
    six review rounds" (evidence-and-claims.md SS6); design: accepted
    there; implementation: none.

SEMANTIC MEANING APPROVED, OPERATION CONTRACT STILL OWED

exact-revision reliance -> applicability -> selective-reopening
obligation -> delivery/recovery
    decision.json's disposition is `approve_proposal_meaning`, with
    `permanent_architecture_settled: false` and
    `implementation_authorized: false` stated explicitly.
    operation-contract-surface.md states directly, in its own words:
    "It deliberately does not cover reliance, selective-reopening
    obligations, or delivery/recovery."
```

Both halves are real. Neither is implemented. But only the first half has
been lowered to an implementable operation contract; the second half is
approved *meaning*, still awaiting the same kind of design pass the first
half already received -- work that was already owed independently of
whether this note's own question survives.

---

## 3. Pressure test 1 -- can a readiness/requirement judgment be materialized as a claim at all?

**Precedent, reused directly rather than invented:**
`evidence-and-claims.md` SS7, `planning-facts-v1`, is a *proposed* (not
accepted) profile carrying exactly one load-bearing constraint: "a
`planning-facts-v1` claim's proposition must trace to an explicitly declared
branch-plan field by direct restatement, never by inference from the plan's
evidence -- otherwise the profile becomes a second, unaccountable planner."

Applied verbatim to a hypothetical `requirement-satisfaction-v1`:

```text
Decision-Specific Readiness
    performs the actual judgment
        v
"ready / blocked / uncertain for decision D under contract C"
        v
requirement-satisfaction-v1 (hypothetical)
    may materialize that already-produced judgment
        v
Claim Evidence owns: identity, revision, provenance, reliance,
                      refresh lifecycle

Claim Evidence may NOT: derive readiness itself
```

**Decision-relative subject/proposition identity.** Decision-Specific
Readiness's own defining property is that the same evidence basis can be
simultaneously `ready` for one decision and `blocked` for another. A
materialized claim whose subject is merely "the requirement" cannot express
this without the two verdicts looking like a direct contradiction over one
proposition. Subject/proposition identity for a materialized readiness
assessment would need to be closer to:

```text
exact decision identity
    + readiness-contract revision
    + bounded proposal / semantic subject
```

so that, for the same proposal `P` and the same evidence `E`:

```text
(P, placement-contract@C1)   -> ready
(P, activation-contract@C7)  -> blocked
```

are two claims about two different subjects, not one contradicted claim.
What exact fields constitute that identity is itself the open profile-design
question this pressure test leaves unresolved.

**Falsifiers:**

```text
Fails this test if:
  the profile ever needs to inspect evidence and decide `ready` itself,
      rather than restate a verdict Decision-Specific Readiness already
      produced; or
  decision-relative identity cannot be expressed without collapsing two
      genuinely different verdicts into one proposition.
```

---

## 4. Pressure test 2 -- does ordinary claim refresh truthfully model satisfaction evolution?

Test candidate transitions -- `blocked -> ready`, `ready -> uncertain`,
`uncertain -> ready`, and so on -- against three things already owned
elsewhere, none invented here:

- the profile-owned `proposition_equivalent` hook (already named in the
  operation-contract surface): does the domain profile treat two state
  values as the same underlying proposition with a changed truth value, or
  as different propositions entirely?
- exact requirement/readiness-contract-revision binding: a comparison across
  a *changed requirement-contract revision* (`C17@v4` vs. `C17@v5`) is not
  automatically a semantic transition -- it may mean only that the question
  itself changed, not that the answer did. This is the same discipline
  Revision/CAS and claim identity already enforce elsewhere through
  revision-scoped comparison, reused rather than reinvented here.
- whether `changed` / `retained_unchanged` are sufficient refresh outcomes
  while the successor claim's domain payload carries the actual readiness
  state (`changed` does not itself need to encode `blocked -> ready`; it
  only asserts that the proposition changed, while the successor claim's own
  payload carries `state: ready`), or whether satisfaction evolution
  requires semantics the existing claim-revision lifecycle cannot represent.

**Falsifier:** fails this test if satisfaction evolution requires
distinguishing cases `changed`/`retained_unchanged` cannot express without a
new, parallel outcome vocabulary.

---

## 5. Pressure test 3 -- does existing reliance/reopening preserve ownership for this use?

**Reused directly:** `decision.json`'s own reopening condition, verbatim,
without inventing a new falsifier for this note:

> "A real domain consumer cannot distinguish candidate impact,
> investigation, unchanged refresh, changed support, applicability, and
> reopening without collapsing owners."

Applied to a consumer this thread actually cares about -- a not-yet-active
Builder or Reviewer vantage whose eventual instantiation depends on a
readiness claim's reopening -- test whether that consumer's own path keeps
candidate impact, investigation, applicability judgment (who decides this
reopening actually matters), and the separately-authorized activation
itself all distinct, honoring `semantic-model.md`'s own explicit invariant:

> "A claim event does not activate an inactive role... Scheduling or
> control-plane delivery may surface it only under separately established
> role and activation authority."

**Falsifier:** fails this test if satisfying activation for a latent vantage
requires the claim/obligation path itself to acquire any authority beyond
"notify the owning domain that reconsideration may be warranted" -- even if
pressure tests 1 and 2 both pass.

---

## 6. Pressure test 4 -- can selective reopening represent *first* activation, not only reconsideration?

**The approved reliance model is narrower than pressure test 3 assumed.**
`semantic-model.md`'s own text binds a reliance record to "one consumer
artifact and immutable consumer revision," and selective reopening applies
when changed support reaches an *active exact-revision reliance*. That
cleanly handles an already-existing consumer being reconsidered:

```text
REOPENING

durable cognition consequence already exists
        v
its supporting world changes
        v
reconsider it
```

But the original motivating idea contains a stronger case this model does
not obviously cover:

```text
AWAKENING

durable obligation/consequence exists,
but its realization does not
        v
its prerequisites become satisfied
        v
consider realizing it for the first time
```

If Builder/Reviewer vantage `K` does not yet exist, there is nothing yet
that can hold a reliance on the readiness claim it would eventually depend
on. That is not reopening; it is first activation from a latent semantic
dependency, and the note must not slide across this difference.

**The pressure test:**

> What durable consumer artifact exists *before* the latent vantage is
> instantiated, such that it can own an exact-revision reliance and later
> receive a selective-reopening obligation?

Candidate answers, none selected here:

```text
a readiness/activation decision artifact?
a dormant workflow obligation?
an accepted organizational requirement?
a previously admitted but unrealized role contract?
a standing consequence/activation contract?
nothing?
```

If one of these already exists as a durable artifact, the lightning still
reduces cleanly -- the consumer is not the future Builder, it is the durable
semantic object whose consequence is "Builder should now be realized":

```text
durable activation/obligation artifact K
        |
        +-- relies on readiness claim Q@r1

Q refreshes -> ready
        v
K candidate-impacted
        v
K's owner reassesses
        v
existing authority admits activation
        v
instantiate vantage
```

If no such pre-instantiation consumer can truthfully exist, reliance/
reopening solves reactivation and reconsideration, but not the original
"latent consequence becomes newly activatable" case -- a real residual seam,
not a detail this note can fold away. The governing principle, if this test
fails: **before cognition can awaken reactively, the reason that cognition
may someday need to exist must itself already have a durable owner** --
otherwise something would need to scan the world and invent work from
satisfaction, which reopens exactly the authority problem this whole thread
exists to avoid.

**Falsifier:** fails this test if no durable, pre-instantiation consumer
artifact can be identified that is capable of owning an exact-revision
reliance before the vantage it concerns is ever realized.

---

## 7. Classification outcomes

```text
A -- Clean profile + latent-consumer reuse
Readiness assessment can be materialized as a claim, AND a suitable
pre-instantiation consumer artifact already exists to own the reliance.
No new generic architecture required.

B -- Profile works; propagation lowering remains
Same semantic answer as A, but the already-known reliance/selective-
reopening operation-contract work (SS2) must still be completed before
any of this is implementable -- independent of this note's own question.

C -- Claim mapping fails
Some property of readiness identity, revision, or lifecycle cannot
truthfully fit Claim Evidence without collapsing ownership (pressure
test 1 or 2's falsifier triggers). Return to architecture intake; a
different, still-undesigned mechanism may be needed after all.

D -- Organizational application remains separate
The readiness mapping works, but extending the same pattern into
organizational-facts/latent-vantage activation requires its own,
later reconciliation, because `organizational-facts-v1` is only a
named exploratory possibility (SS8), not a proposed profile the way
`planning-facts-v1` is -- weaker precedent, not equal footing.

E -- Latent-consumer gap
Readiness claims and reliance/reopening both work (tests 1-3 survive),
but no existing durable object can represent the not-yet-realized
consequence that must own the reliance (pressure test 4 fails). The
remaining novelty narrows to one precise question: what is the durable
identity of work that is authorized or meaningful but not yet
realizable because its prerequisites are unsatisfied?
```

No classification is selected by this capture. `E` is the outcome most
likely to isolate whatever narrow, genuinely new architecture (if any)
survives this entire thread.

---

## 8. Motivating interpretation (not part of what this note asserts)

The following is the reason this question is worth testing. It is not a
claim this note makes, and it must not be read as accepted architecture:

```text
expensive cognition establishes durable meaning
        v
domain owner publishes an attributed judgment
        v
claim materialization (if pressure test 1 survives)
        v
exact-revision reliance (already accepted, SS2 upper half)
        v
a relied-upon source revision changes
        v
selective reopening (approved meaning, SS2 lower half)
        v
the owning domain decides whether to reconsider, reopen, or activate
        v
authorized cognition begins only where and when it is actually needed
```

If classification A or B survives intake, the original "compression and
expansion become one continuous process" intuition would not require Work
Engine to become reactive from scratch -- it would mean a propagation
substrate for exactly this shape was already designed, just never connected
to cognition activation as an application. That is a materially stronger and
more interesting result than inventing a parallel mechanism would have been.
This note tests whether that connection holds; it does not assume it does.

---

## Relationship to existing architecture

- [`../../docs/architecture/decision-specific-readiness.md`](../../docs/architecture/decision-specific-readiness.md) --
  the source of the judgment a materialized claim would only ever restate,
  never derive; owns the readiness contract and attributed assessment this
  note's profile question is entirely about.
- [`../../docs/architecture/evidence-and-claims.md`](../../docs/architecture/evidence-and-claims.md) --
  SS6 (accepted refresh operations), SS7 (`planning-facts-v1`, the reused
  pure-projection precedent), SS8 (`organizational-facts-v1`, the weaker
  precedent), and SS9 (what the dimension explicitly does not own).
- `proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/operation-contract-surface.md` --
  the accepted, fully-specified nomination/episode/judgment operation
  contract; states directly that it excludes reliance, selective-reopening,
  and delivery/recovery from its bounded first vertical.
- `proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/semantic-model.md` --
  "Versioned reliance and applicability" and "Selective-reopening
  obligations," the approved-meaning content this note's pressure tests 2
  and 3 depend on directly.
- `proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/decision.json` --
  `approve_proposal_meaning`, the exact status this note relies on, and the
  reopening condition reused verbatim as pressure test 3's falsifier.
- [`observation-consequences-and-evidence-basis-evolution.md`](observation-consequences-and-evidence-basis-evolution.md) --
  an adjacent, independently-scoped seam from the same general territory
  (what an attributed observation/judgment may legitimately do to durable
  semantic state); related context, not a dependency of this note.

---

## Non-goals

This note does not currently propose:

- accepting `requirement-satisfaction-v1` as a real domain profile;
- authorizing any implementation of anything described here;
- extending `organizational-facts-v1` past its own exploratory status;
- asserting the SS7 motivating interpretation ("compression triggers
  expansion") as accepted architecture;
- a new generic reactive/satisfaction mechanism of any kind -- this is
  precisely the thing this thread considered at length and chose not to
  draft;
- deciding whether Decision-Specific Readiness specifically, as opposed to
  some narrower requirement class, is the right first candidate for
  materialization.

---

## Questions for later intake and reconciliation

1. Which Decision-Specific Readiness assessments, if any, are actual
   candidates for materialization, versus assessments whose own lifecycle
   or ownership semantics make direct claim materialization wrong or
   redundant?
2. What exact fields constitute subject/proposition identity for a
   materialized readiness assessment such that decision-relativity survives
   materialization without inventing a new identity concept beyond ordinary
   claim/subject/revision binding?
3. Does the reliance/selective-reopening operation-contract lowering (SS2)
   need to happen regardless of this note's own outcome, given it was
   already owed to the original claim-maintenance proposal independent of
   any reactive-activation motivation?
4. If classification C is reached, does the "lightning" intuition still
   motivate a genuinely new mechanism, or does the specific failure mode
   found redirect it toward a narrower fix within claim-evidence itself?
5. Who is the correct owner to actually run this pressure test --
   Decision-Specific Readiness's own maintainers, Evidence and Claims', or
   a joint reconciliation -- given that a wrong answer risks collapsing one
   dimension's authority into the other's?
6. If classification E is reached, does any existing durable artifact
   (an admitted-but-unrealized role contract in Organizational Compilation
   is the leading candidate) already represent "authorized work not yet
   realizable," or is the durable identity of latent, prerequisite-gated
   work a genuine gap nothing currently owns?
7. Is the reopening/awakening distinction (SS6) itself already recognized
   somewhere in this architecture under different vocabulary, or would
   naming it be new?

---

## Compact hypothesis

> Work Engine's claim-maintenance-and-reliance-propagation architecture
> already has approved (though not fully lowered) semantics for exact-
> revision reliance and domain-owned selective reopening. If a readiness or
> requirement judgment can be materialized as a claim through pure
> projection -- restating, never deriving, the owning domain's judgment,
> preserving decision-relative subject identity, and expressed through
> ordinary refresh dispositions rather than a new outcome vocabulary -- then
> *reconsideration* of already-existing cognition is already architecturally
> latent, requiring only a domain-profile decision and completion of
> already-owed operation-contract work. Whether *first activation* of
> not-yet-instantiated cognition reduces the same way depends on a further,
> separate question: whether a durable, pre-instantiation consumer artifact
> already exists to own the reliance before the cognition it concerns is
> ever realized. This note tests both claims; it asserts neither.
