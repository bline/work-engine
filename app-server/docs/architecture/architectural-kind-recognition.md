# Architectural Kind Recognition

> **Question:** When a pattern recurs across independently owned truth domains, what evidence and what test decide whether it is recognized as a dimension, a mechanism, a substrate, or merely domain/implementation detail — and how much evidence is eligibility versus how much is confidence?

## 1. Purpose

This is a **taxonomy-level canonical page**, not a mechanism, dimension, or substrate page — it does not itself own any class of architectural truth, preserve any invariant, or supply any fact. It owns the **recognition criterion** by which every other page in this architecture is classified.

That criterion has been applied consistently since the 2026-09-16 architecture-views decomposition — `mechanisms/revision-cas-and-publication.md` explicitly refuses to count its executable-generation finding as "an 8th instance"; `mechanisms/resource-lease-and-fencing.md` stays "structurally distinct from Transition Fencing" rather than merged on shared vocabulary; `substrates/context-observer.md` and `substrates/evidence-anchor.md` were each named only once two independent consumers existed. But it was never stated as its own rule — the five mechanism pages functioned as precedent defining the criterion (a candidate resembles existing mechanisms, therefore it is one) rather than as evidence *under* a stated criterion. This page closes that circularity:

```text
before this page:
    X is a mechanism because it resembles things already called mechanisms.

after this page:
    X is a mechanism because it satisfies the criterion stated here.
    Existing mechanisms are retained because they also satisfy it — see
    §13, the pressure test that checks this directly rather than assuming
    it.
```

**Revised 2026-09-18, after direct review of the first version.** That version defined a real, sophisticated eligibility test for exactly one kind (mechanism), then declared dimensions untested and substrates "structurally eligible for the identical test" without ever checking — reintroducing, at the level of this page itself, the hidden category coupling it exists to prevent. Fixed here by giving each of the three promotable kinds its own classifier branch (§3) and its own eligibility test (§4–§6), rather than generalizing the mechanism test outward. The same review caught a second, subtler leak (recognition evidence silently implying `status-grammar.md` values, §7), an overclaim in the origin-independence ladder (§8), and a real evidentiary conflation inside the Resource Lease pressure test that this revision also finds, once named, in Evidence Anchor (§13–§14).

`app-server/docs/work-engine-planned-architecture.md` §1 states this page exists and links here; it does not restate the criterion. Any future capstone edit that finds itself re-deriving eligibility or evidence rules belongs here instead.

This page does not reopen the 13-dimension / 5-mechanism / 2-substrate decomposition, and does not reclassify anything merely because it now exists or because §14 pressure-tests the substrates for the first time.

---

## 2. The Four Architectural Kinds

Three of these are already named in the capstone's own §2 (`work-engine-planned-architecture.md`); this page states all four canonically, including the fourth, which previously existed only as a residue-disposition value (`DOMAIN_DETAIL`, capstone §11) rather than as a named architectural kind in its own right:

```text
DIMENSION
    Owns a class of architectural truth. Changes what Work Engine believes
    or is committed to about its domain.

MECHANISM
    Preserves an invariant across truth owners without acquiring their
    semantic authority.

SUBSTRATE
    Supplies facts or capability without acquiring consumers' semantic
    authority.

DOMAIN / IMPLEMENTATION DETAIL
    Realizes, parameterizes, represents, or consumes already-owned
    architecture without establishing another architectural truth owner.
```

The fourth kind is not a residual "everything else" bucket — it is the default. A candidate pattern starts here and is promoted only by clearing §3's classifier and its target kind's own eligibility section. Most real, concrete artifacts in Work Engine correctly stay in this kind permanently (capstone §9's own domain instantiations: `CodeEvidenceAdapter`, `UIReviewProfile`, `control-plane-causal-observability-ui.md`) — promotion is the exception, not the default trajectory of a useful pattern.

**§4, §5, and §6 below are deliberately not one generalized test reused across all three promotable kinds.** A mechanism's defining property (preserves a transition/admission invariant) and a substrate's (supplies facts, and specifically triggers no admission or transition on its own standing) are different claims with different failure modes; testing a substrate candidate against a mechanism's own criteria would silently import an admission question a substrate is defined not to answer, which is exactly the coupling this page exists to prevent.

---

## 3. Kind Classifier

Checked in this order, because truth-ownership outranks the other two whenever it is genuinely present, and the fact-plane question is checked before the protocol/control-plane question since a substrate that also enables reuse across domains is still classified by *what plane its authoritative output lives on*, never by the reuse itself:

```text
CANDIDATE NORMALIZED TRUTH

Does it own semantic interpretation or architectural state whose
authoritative meaning cannot be reconstructed from a neutral
observation contract alone?
    yes → DIMENSION candidate → §4

no:
Does its authoritative output consist only of source-grounded
observation/capability facts, with no protocol state whose validity
governs admission, mutation, publication, or transition?
    yes → SUBSTRATE candidate → §6

no:
Does it preserve a reusable rule governing whether/how an operation,
admission, mutation, publication, or transition may proceed, across
independently owned truths?
    yes → MECHANISM candidate → §5

otherwise:
    DOMAIN / IMPLEMENTATION DETAIL
```

**"Supplies a neutral fact" is not, by itself, a discriminator between the substrate and mechanism branches — both intentionally possess that property.** Candidate Resolution's own reduction (`0`/`1`/`N`) is a neutral fact about a candidate space; Revision/CAS's head/staleness check is a neutral fact about a revision; Transition Fencing's revalidation is a neutral fact about fence validity. What actually separates them from a substrate is which *plane* that fact lives on: a mechanism's fact is protocol state whose validity gates whether an admission, mutation, publication, or transition may proceed; a substrate's fact never gates anything — it is read, derived from, and acted on entirely by its consumers. This is why the classifier asks about the *fact plane vs. the protocol/control plane*, not about neutrality, which both kinds share.

A candidate can still look superficially like more than one branch — `substrates/context-observer.md` enforces a real boundary discipline (what belongs to the observer vs. to each consumer's own derivation), which could be misread as mechanism-shaped. It classifies as SUBSTRATE, not MECHANISM, because its authoritative output is pure fact-plane content (the `ContextObservation` schema), and both substrate pages state directly, as their own Key Invariant, that the substrate "triggers no admission, transition, or publication on its own standing" — the one thing every mechanism in §5 exists specifically to govern. The classifier resolves this by asking which plane the candidate's own authoritative output lives on, not by counting surface similarities to an already-recognized kind.

---

## 4. Dimension Eligibility

Lighter than §5–§6 because dimension recognition was not the ambiguity this page's originating review found, and no dimension pressure test is run here (§13–§14 cover the five mechanisms and two substrates only; applying this section to the 13 dimensions is a legitimate future use of this page, not attempted in this revision).

```text
1. TRUTH OWNERSHIP
   The candidate changes what Work Engine believes or is committed to
   about its own subject matter — not a derived view of a truth another
   page already owns.

2. NON-DUPLICATION
   No existing dimension, mechanism, or substrate already answers the
   same question for the same subject matter — the capstone's own §4
   test, stated there as "no other dimension, mechanism, or substrate
   supplies" this.

3. AUTHORITY-BEARING CONTENT
   The candidate owns semantic interpretation or architectural state
   whose authoritative meaning cannot be reconstructed from a neutral
   observation contract alone. This catches descriptive dimensions
   (`evidence-and-claims.md` asserts what proposition a claim makes and
   what revision justified it — descriptive, not a policy or commitment)
   as well as normative ones — a dimension need not look like a decision
   to qualify, only its authoritative meaning must exceed what a
   substrate's fact-plane contract could supply on its own.
```

---

## 5. Mechanism Eligibility

Eligibility answers *can this candidate be a mechanism at all* — a category question. It is checked once per candidate and does not scale with how many times the candidate has been observed. A candidate that fails any one of these is not a mechanism regardless of how much reuse evidence exists for it.

```text
1. NORMALIZED INVARIANT
   After removing the idea/domain-local nouns, a reusable invariant or
   protocol remains — not a coincidence of shared vocabulary.

2. NON-OWNERSHIP
   The candidate does not become semantic authority for its consumers.
   It may constrain, encode, or protect meaning; it may never supply it.

3. SUBSTANTIAL RESIDUE AFTER REFUSALS
   After stating precisely what the candidate does not decide — meaning,
   domain policy, semantic consequence, residual judgment authority, and
   so on — something non-trivial still remains: a real protocol or
   invariant, not a thin naming convention left standing after everything
   else was refused away.

4. CROSS-ROUTE INVARIANCE
   Materially different lawful consumers or realizations, owned by
   different domains, can use the candidate without changing what it
   means. The same mechanism answers the same question regardless of
   which owning domain is asking it.

5. DISTINCTNESS
   The candidate cannot collapse into an existing dimension, mechanism,
   substrate, or domain detail without moving semantic ownership or
   destroying an invariant that currently holds. Shared vocabulary between
   two candidates is not, by itself, evidence against distinctness (§10).
```

**Criterion 4 must be kept sharply distinct from a weaker, adjacent claim: parametric invariance.** A single implementation surviving many parameterizations of one resource, configuration, or subject space — all still owned by one domain — is real, useful evidence that the implementation generalizes cleanly within its own territory. It is not, by itself, cross-route invariance, and does not by itself satisfy this criterion; §7 types it as its own evidence kind (`parametric_invariance`), and §13's Resource Lease entry is the worked case where the corpus's own text originally let the two get conflated.

**The refusal test (criterion 3) is necessary, not sufficient, and must never be read as the whole test.** A "What This Mechanism/View Does Not Decide" section that leaves substantial residue proves the candidate *can* be an authority-neutral abstraction. It does not prove Work Engine *benefits* from canonizing it as one — a beautifully bounded abstraction with a single consumer can still be a domain-local helper that will never be reused. Eligibility (this section) establishes that a candidate is the right *shape*; §7 establishes that canonizing it is actually *warranted*. Both are required; neither substitutes for the other.

---

## 6. Substrate Eligibility

Eligibility answers *can this candidate be a substrate at all*. A substrate's defining property is different in kind from a mechanism's: a mechanism preserves an invariant governing transition or admission; a substrate supplies facts or capability and triggers no admission, transition, or publication on its own standing — both `substrates/context-observer.md` and `substrates/evidence-anchor.md` state this directly as their own Key Invariant. Reusing §5's mechanism test here would silently import an admission/transition question a substrate is specifically defined not to answer — exactly the coupling this page exists to prevent.

```text
1. NORMALIZED OBSERVATION CONTRACT
   After removing consumer-specific derivation, a real, source-grounded
   fact or capability schema remains — not merely "whatever a consumer
   happens to want."

2. SEMANTIC NEUTRALITY
   The candidate supplies facts/capability but never a consumer-specific
   derived metric, admission, transition, or judgment. Materially
   different consumers may derive materially different conclusions from
   the identical supplied fact without contradiction.

3. SUBSTANTIAL RESIDUE AFTER "DOES NOT OWN"
   After stating precisely what it does not own — derived metrics,
   admission, transition, dependency/consequence judgment — a real,
   complete, checkable observation or capability contract remains, not a
   thin passthrough of raw reality.

4. CROSS-CONSUMER INVARIANCE
   Materially different, independently-owned consumers can consume the
   identical contract as peers — none gating another's applicability
   question — without the substrate changing what it supplies for either.
   Subject to the identical parametric-vs-cross-route distinction as
   §5.4: several claim or resource *types* owned by one dimension are
   parametric invariance, not several independently-owned consumers.

5. DISTINCTNESS
   Cannot collapse into an existing dimension, mechanism, or the other
   substrate without moving semantic ownership. Two substrates sharing an
   observation/normalization shape but differing subject matter remain
   distinct under §10's sibling-vs-merge test — shared shape is not
   shared invariant.
```

The refusal test (criterion 3) is necessary, not sufficient, here exactly as in §5: a substrate's "Does Not Own" list leaving real residue proves it can stay semantically neutral; it does not by itself prove Work Engine needs a named, canonical substrate rather than letting each consumer read raw reality itself. §7's evidence typing applies identically to substrate candidates.

---

## 7. Typed Recognition Evidence — Confidence, Not Eligibility, Not Status

A candidate that clears §4, §5, or §6 is eligible to its respective kind. Whether it should actually be **admitted to the canonical taxonomy as a dimension, mechanism, or substrate** is a separate, evidentiary question, and **instance counts are supporting bookkeeping, never the criterion itself.** `mechanisms/revision-cas-and-publication.md`'s own status history is the direct proof: its confirmed-instance count moved from four to seven on 2026-09-16 ("found by the final mechanical audit to be undercounted here as 'four' — corrected") with zero change to `design: accepted` — the mechanism was never *made* real by reaching seven; the correction only repaired the record of evidence that was already there.

**Recognition does not assign `design`, `reconciliation`, `authorization`, or `implementation` status.** Those remain independent status assertions whose authority and evidence are governed entirely by `status-grammar.md`. A candidate can be canonically recognized here as mechanism-shaped or substrate-shaped and still be `design: proposed`, `authorization: exploration_only`, or `implementation: none` — exactly what `mechanisms/candidate-resolution-and-admission.md` and both current substrates already are. Recognition answers "is this the right kind, with enough evidence to name it" — never "has this been accepted, authorized, or built."

Evidence is typed, not counted. At minimum, distinguish:

```text
independent_domain_convergence
    Two or more owning dimensions, reasoning independently, reached for
    the identical shape without copying one another.

preexisting_generic_implementation
    A single real, already-generic implementation exists, used across
    multiple parameterizations of one resource/config space.

implemented_cross_domain_consumer
    A materially distinct domain has built real, running code that
    consumes the candidate as its own instance.

accepted_cross_domain_consumer
    A materially distinct domain has made an explicit, citable
    design-acceptance decision to consume the candidate as its own
    instance, even though nothing has been built yet. Stronger than
    proposed_cross_domain_consumer: a real commitment exists, not merely
    a name.

proposed_cross_domain_consumer
    A materially distinct domain has been named or suspected as a
    plausible future consumer, without any explicit acceptance decision
    committing it. Weaker than accepted_cross_domain_consumer — this is
    corroborating context, not a commitment.

parametric_invariance
    The same implementation survives multiple parameterizations of one
    resource, configuration, or subject space within a single owning
    domain. Real, useful evidence of clean generalization within that
    domain; never sufficient on its own for cross-route invariance
    (§5.4, §6.4) or for independent_domain_convergence.
```

### 7.1 The Canonical Admission Rule

§4–§6's eligibility tests are necessary. They are not sufficient. A candidate that clears its kind's own eligibility test still needs an explicit admission route before it is canonically recognized — catalogued in the capstone as a named mechanism or substrate — otherwise "eligible" quietly substitutes for "recognized," leaving unanswered exactly the question this section exists to answer: *how much, or what combination, of typed evidence is actually sufficient?*

> **Canonical recognition requires eligibility plus at least one non-parametric route establishing actual cross-owner need:**
>
> ```text
> A. independent_domain_convergence
> OR
> B. implemented_cross_domain_consumer
> OR
> C. accepted_cross_domain_consumer
> OR
> D. an explicit architectural-owner decision that canonization is
>    warranted ahead of natural convergence — invoked deliberately, not
>    used as a default fallback whenever routes A–C come up short.
> ```

`parametric_invariance` never satisfies this rule on its own, however many parameterizations exist — it establishes that one implementation generalizes cleanly within its own domain, never that a second domain needs it. **A merely named, mentioned, or hypothetical candidate consumer — accepted by no one, built by no one — is corroborating context, not an admission route; it does not satisfy route C.** This is deliberately not a numeric threshold (not "two instances," not "three consumers") — it is a named set of evidence shapes, any one of which is enough, because the shapes themselves are what make convergence credible, not their count.

§13 and §14 apply this rule directly to the five current mechanisms and two current substrates. One candidate — `substrates/evidence-anchor.md` — clears §6's eligibility test but does not currently satisfy this rule under any of routes A–D; §14 states this plainly rather than protecting the substrate's existing catalog status.

These are not mutually exclusive, and a single candidate typically carries more than one at once — see §13–§14 for how each current mechanism and substrate actually breaks down. Three things must be stated explicitly, because the existing corpus has blurred exactly these distinctions under one undifferentiated "Confirmed Instances" heading in more than one page (§13, §14 name where):

- **Resource types parameterizing one generic implementation are `parametric_invariance`, not `independent_domain_convergence`.** `mechanisms/resource-lease-and-fencing.md`'s seven `RESOURCE_TYPES` (`directory, git-ref, git-index, port, index, review-budget, database`, verified against `contract.mjs:4-6`) are seven parameterizations of one already-generic `workspace-coordination` service, not seven independently-arrived-at domains that separately discovered the need. That is real, legitimate evidence, but it is a different evidentiary claim than convergence, and conflating the two overstates how many independent reasoning processes actually endorsed the shape.

- **"Zero cross-reference" is evidence of textual independence, not necessarily independent origin.** `mechanisms/authority-preserving-intent-projection.md` states its Runtime Realization instance "was written with **zero cross-reference to Studio or its reconciliation**, yet follows the identical shape" — real evidence that one author didn't copy the other's prose. It is not evidence that the two reasoning processes were separated by anything more than authorship, since both are products of the same 2026-09-16 architectural-decomposition campaign. See §8's origin-independence ladder for why this distinction has to be kept explicit rather than collapsed into a single "independent" label.

- **Instance counts are supporting evidence, never the recognition criterion itself** (restated from the opening of this section because it is the single most load-bearing sentence on this page).

---

## 8. Origin Independence — An Ordinal Ladder, Not a Score

Distinct from *how many* consumers exist (§7) is *how independently* each one arose. This is not a scalar to compute and compare numerically — it is an ordinal ranking:

```text
same claim / same document
    < separate documents, same design campaign
    < independently developed subsystem or domain
    < implementation predating the synthesis that recognized the pattern
```

**Higher placement means stronger evidence that the apparent convergence was not manufactured by the recognition process. It does not, by itself, mean stronger evidence that the abstraction is architecturally general.** These are different claims, and treating the ladder as measuring the second collapses it back into the same kind of scalar score this page exists to avoid. A preexisting implementation at the top of the ladder may simply be a locally generalized utility that has never encountered a second semantic domain — real, but possibly still domain-bound in every way that matters architecturally. Two same-campaign documents lower on the ladder, converging independently on an identical shape neither copied, can be exactly the signal of a genuine architectural joint, even though same-campaign placement makes the *non-manufactured* claim weaker. §13 and §14 report ladder placement strictly as evidence against manufactured convergence, never restated as a claim about generality.

---

## 9. The Falsifier / Demotion Rule

A recognized mechanism or substrate is not permanently settled by having once cleared §5 or §6. It should be reconsidered — not silently, through an explicit re-check against this page — if later evidence shows any of:

```text
- the supposed invariant turns out to be domain-specific after all
  (the normalized-invariant/observation-contract criterion fails on
  closer inspection);
- the refusal boundary leaves only a thin vocabulary or schema once
  restated precisely (the residue-after-refusals criterion fails);
- its consumers require materially different semantics from one another,
  not merely different implementation shape (the cross-route/cross-
  consumer criterion fails);
- an existing canonical owner already owns the normalized truth being
  claimed (the distinctness criterion fails).
```

Each recognized mechanism or substrate should be able to name what observation would demote it. §13–§14 state one per candidate, drawn from that candidate's own weakest currently-cited evidence rather than invented generically.

---

## 10. Sibling-versus-Merge Test

Two candidates sharing vocabulary, implementation substrate, or superficial shape stay **separate** only when one of these holds:

```text
- they preserve materially different invariants; or
- a rule one must enforce is a rule the other must NOT hold.
```

Differences only in lifetime, naming, calling convention, storage engine, or implementation shape are **insufficient** to justify a merge, and are equally insufficient, by themselves, to justify keeping two pages separate — the test cuts both ways. §11 applies this directly and checks the source text rather than deciding from analogy.

---

## 11. Worked Pressure Test: Resource Lease vs. Transition Fencing

Both mechanism pages already carry a direct, explicit "Relationship to [sibling]" section rather than assuming compatibility from shared fencing-token vocabulary — that much reconciliation work is already done. What was not yet stated as a sharp, enforceable rule is §10's own test applied as a concrete "X must, Y must not" pair.

Checked directly against both pages' own text (not decided from analogy):

**Resource Lease's own comparison row** (`resource-lease-and-fencing.md`, "Relationship to Transition Fencing and Leases"):
> lifetime: standing relationship, may renew over time
> staleness: an older holder/token loses exercise authority

**Transition Fencing's own comparison row** (`transition-fencing-and-leases.md`, "Relationship to Resource Lease and Fencing"):
> protects a one-shot preparation interval (bind → prepare → revalidate → publish → activate → release)
> staleness = the bound world revision changed while preparing

The reciprocal rule proposed for pressure-testing:

```text
Resource Lease:
    a current holder may repeatedly exercise protected authority across
    time and multiple operations until expiry, release, or supersession.

Transition Fence:
    completion of the bounded transition episode must not leave reusable
    standing authority for another activation; authority is exhausted by
    (or released after) the specific preparation → activation attempt.
```

**Result: supported, with one distinction worth stating precisely.** Resource Lease's own text directly states the reusable-exercise half: `admitMutation` is checked "exactly once per operationId" but the same lease may back any number of distinct operation IDs while it remains current — a "standing relationship... may renew over time" is not a one-shot grant. Transition Fencing's own text never states a literal "must not confer reusable authority" prohibition in those words, but its entire documented shape supports the reciprocal directly: every stated instance of this mechanism (Context Lifecycle's transition-lease, the topology-transition fence, the executable-generation reload) is described as a single linear sequence terminating at "release the fence," with no renewal step, no "hold" state, and no second activation ever described against the same fence acquisition — explicitly contrasted, in the sibling comparison table itself, against Resource Lease's "no preparation phase, no single activation event." A one-shot preparation interval that always terminates at release, with no renewal path stated anywhere in its own page, is what "must not leave reusable standing authority" means in this document's own vocabulary; it is a direct reading of the page's stated shape, not an inference from a different mechanism's behavior.

This strengthens, rather than manufactures, the existing split: §10's test is satisfied (a rule Resource Lease must enforce — repeat exercise under one currency is lawful — is a rule Transition Fencing's own stated shape never permits), so the two pages should remain separate mechanisms. Because neither page states the reciprocal as its own explicit "must / must not" sentence, that is a real, minor strengthening opportunity for both pages' own text — noted here, not applied there; editing those two pages is outside this page's own scope.

The **third-instance question already flagged inside `transition-fencing-and-leases.md` itself** ("a third, real instance protects a different kind of thing than either named class... left open rather than decided by this edit," referring to the executable-generation reload) is a distinct, open question about that mechanism's own *internal* fence-class taxonomy — not a question about whether it should merge with Resource Lease. The page's own text already resolves the cross-mechanism question directly ("tested against both mechanisms directly and belongs to the other one, not this one"). §13 records the internal taxonomy question as still open, exactly as the source page states it, without resolving it here.

---

## 12. Authorization Is Not Inherited Through Composition

Recognition (this page) and authorization (`status-grammar.md`) are independent axes, and composing a candidate mechanism with its consumers must not blur that independence in either direction — restated with emphasis here because §7 already forbids recognition from assigning authorization directly, and this section covers the adjacent case of authorization moving sideways between a mechanism and its consumers. `status-grammar.md` §7 already states the general rule ("implementation-authorized input → does not grant implementation authorization to what consumes it"); this page's own worked example (§11, `mechanisms/resource-lease-and-fencing.md`'s status block) is the sharpest concrete case: the generic mechanism itself carries `authorization: unrecorded`, while two of its consumer instances each carry an independent `status_override` of `authorization: implementation_authorized`, sourced to their own distinct reconciliation documents — neither direction inherited. `status-grammar.md` §7 has been given an explicit corollary and a sixth worked example (§10.6 there) recording this case by name, so the rule is stated where authorization itself is defined rather than duplicated here.

---

## 13. Pressure Test: The Current Five Mechanisms

Checked against §5, §7 (including §7.1's admission rule), §8, and §9 directly. **No reclassification is made or implied by this section** — all five clear eligibility, and all five satisfy §7.1's admission rule. What follows is the evidentiary and documentation picture the criterion exposes, including real gaps, reported rather than repaired.

### `mechanisms/revision-cas-and-publication.md`

- **Eligibility:** 1–5 all pass. Normalized shape (identity/predecessor/head/CAS/atomic-publish/stale-rejection) verified domain-blind; explicit "Does Not Decide" list; substantial residue (the six-item reusable shape); cross-route invariance across five materially different owning dimensions; distinctness stated directly against both Candidate Resolution ("different questions, frequently composed") and Transition Fencing ("built directly on top of that one and does not restate its predecessor/head semantics").
- **Evidence kinds present:** `independent_domain_convergence` (Evidence/Claims, Context Lifecycle, Organizational Compilation, Semantic Planning, Material Decision Selection, Runtime Realization — six dimensions, each citing its own source text); `implemented_cross_domain_consumer` (Evidence/Claims, Context Lifecycle, and Runtime Realization's nested executable-generation substrate, all verified against real code); `accepted_cross_domain_consumer` (Organizational Compilation, "Accepted 2026-09-15 as design"); `proposed_cross_domain_consumer` (Semantic Planning, Material Decision Selection, Runtime Realization's own top-level `RoleRealization` lineage — each named in its own source but not citably accepted at this specific consumption).
- **Admission rule (§7.1):** satisfied by routes A and B alone, independent of the weaker proposed instances.
- **Origin independence:** mixed. Evidence/Claims and Context Lifecycle's *code* sits at "implementation predating the synthesis that recognized the pattern" (real, running services checked directly) — strong evidence the convergence wasn't manufactured, not itself a claim about generality; the *naming of the shared mechanism* across all seven cited instances sits at "separate documents, same design campaign."
- **Falsifier:** an instance whose "publish" can silently merge, reorder, or last-write-win against a stale head is not this mechanism's shape regardless of how it is labeled.
- **Documentation gap exposed:** the "post-execution implementation acceptance" instance (`completion-publication.mjs`) composes three already-recognized dimensions' outputs rather than belonging to any one dimension — a genuine fifth evidence shape (a *composite* instance) that §7's typed kinds do not name. Flagged as an open extension to this page's own taxonomy, not resolved here.

### `mechanisms/candidate-resolution-and-admission.md`

- **Eligibility:** 1–5 all pass. Domain-neutral `AVAILABLE ∩ AUTHORIZED ∩ SATISFIES(REQUIRED) → {0/1/N}` shape; explicit "Does Not Decide" list; substantial residue (the zero/one/many distinction itself, stated as "the central fact this mechanism exists to establish"); cross-route invariance across three materially different candidate universes; distinctness stated directly against both Revision/CAS and `mechanisms/authority-preserving-intent-projection.md` ("Neither subsumes the other").
- **Evidence kinds present:** `independent_domain_convergence` (Organizational Compilation, Runtime Realization, Material Decision Selection — three dimensions; a fourth, Portfolio Selection, is explicitly self-labeled by its own source as "likely, not yet formally confirmed," i.e. `proposed_cross_domain_consumer`, correctly hedged already). No `implemented_cross_domain_consumer` evidence at all — `implementation: none` across every confirmed instance.
- **Admission rule (§7.1):** satisfied by route A alone — three independently-owned dimensions is real cross-owner need, even with zero implementation.
- **Origin independence:** all three confirmed instances sit at "separate documents, same design campaign" (all reconciled or falsifier-tested within the 2026-09-15/16 window); none reaches "independently developed subsystem" or "predates synthesis," since none has running code — a weaker non-manufactured-convergence signal than Revision/CAS's, independent of either mechanism's actual generality.
- **Falsifier:** a claimed instance whose residual N-case judgment cannot actually be reduced to a domain-supplied decision (e.g., needing the mechanism itself to supply a domain predicate, or producing a fourth outcome class) does not confirm this mechanism.
- **Documentation gap exposed:** none structural. Worth stating plainly as an evidentiary-confidence note (not a failure): this mechanism's entire canonical status currently rests on same-campaign, zero-implementation convergence — a materially thinner evidentiary base than Revision/CAS's, even though both pass eligibility identically. §5's criteria do not distinguish them; only §7's typed evidence does, which is exactly the separation this page exists to make visible.

### `mechanisms/transition-fencing-and-leases.md`

- **Eligibility:** 1, 2, 3, 5 pass cleanly. Criterion 4 (cross-route invariance) is **partially open, by the page's own admission**: the shared revision-binding discipline (bind → prepare → revalidate → publish → activate → release) is confirmed invariant across all three consumers — three genuinely separate owning dimensions, not parameterizations of one — but whether "two named fence classes plus a third, unclassified instance" is a settled taxonomy or an incomplete one is explicitly unresolved in the source page itself ("left open rather than decided by this edit"). This does not fail eligibility — the base invariant clears criterion 4 — but the internal fence-class taxonomy built on top of it is not yet a closed question.
- **Evidence kinds present:** `implemented_cross_domain_consumer` (Context Lifecycle's real transition-lease sequence; Runtime Realization's real executable-generation reload, verified directly against `executable-generation-manager.mjs`/`executable-generation-store.mjs`); `proposed_cross_domain_consumer` (Organizational Compilation's topology-transition fence, named in `deterministic-authority-projection-and-adaptive-organizational-topology.md` Part 7.3 — itself `design: proposed`, not `accepted`, per that document's own status — no compiler exists).
- **Admission rule (§7.1):** satisfied by route B alone (two real, independently-owned, implemented consumers).
- **Origin independence:** Context Lifecycle's and the executable-generation reload's implementations both sit at "implementation predating the synthesis that recognized the pattern" — both are real, pre-existing code the mechanism's own naming discovered rather than invented (the executable-generation instance explicitly "found 2026-09-16" as a symptom, not authored to fit) — strong evidence against manufactured convergence for those two. Organizational Compilation's topology-transition fence sits at "separate documents, same design campaign," the weakest of the three.
- **Falsifier:** an instance that turns out to confer standing, renewable exercise authority (violating §11's reciprocal rule) is a Resource Lease instance mis-filed here, not a third fence class.
- **Documentation gap:** the internal two-vs-three-fence-class question is real but already correctly hedged in both the source page and the capstone's own §5 table ("a third real instance protecting neither, never collapsed into one lock") — no overstatement found, no new gap to report; recorded here as an open item this page inherits rather than resolves.

### `mechanisms/resource-lease-and-fencing.md`

- **Eligibility:** 1, 2, 3, 5 pass cleanly. Criterion 4 (cross-route invariance) does **not** pass on the seven resource-type parameterizations directly — `directory/git-ref/git-index/port/index/review-budget/database` are one generic `workspace-coordination` implementation's own parametric invariance (§5's own note under criterion 4), not materially different owning domains. It passes instead, more thinly, on the two prospective consumers: fenced active-binding (owned by Runtime Realization) and review-scope protection (owned by `review.md`) are genuinely different owning domains reusing the identical acquire/`admitMutation` shape without changing its meaning — both real, accepted-for-implementation designs, neither built. Eligibility is a category question, not a confidence one, so an accepted-but-unbuilt cross-domain design is sufficient to clear criterion 4; the evidence typing below is where the resulting confidence is honestly recorded as thinner than "seven confirmed instances" suggests.
- **Evidence kinds present:** `preexisting_generic_implementation` (the `workspace-coordination` core itself, verified against `contract.mjs`, predating the reconciliation-queue process entirely) plus `parametric_invariance` (its seven resource types — real implementation-robustness evidence, not cross-domain evidence) plus `accepted_cross_domain_consumer` ×2 (fenced active-binding — "Accepted 2026-09-14"; review-scope protection — "accepted 2026-09-14" — both explicit, citable design-acceptance decisions from their own distinct reconciliation documents, not mere mentions). No `implemented_cross_domain_consumer` evidence exists yet beyond the generic mechanism's own resource types.
- **Admission rule (§7.1):** satisfied by route C — two explicit, dated acceptance decisions from two different owning domains is real committed cross-owner need, even fully unbuilt. This is the sharper reading criterion 4 already implied: the mechanism's admissibility never rested on the seven resource types at all.
- **Origin independence:** `workspace-coordination` sits at "implementation predating the synthesis that recognized the pattern" (independently named elsewhere as "the strongest kernel-shaped primitive found anywhere in the inventory," per its own status block) — strong evidence the recognition wasn't manufactured; not itself evidence that the two prospective consumers will pan out. The two accepted consumers sit at "separate documents, same design campaign" (both accepted 2026-09-14, within the decomposition effort).
- **Falsifier:** if, once built, either prospective consumer needs a materially different admission or staleness rule than `admitMutation`'s exactly-once-per-operation-id CAS check (e.g., multi-holder shared exercise, or non-monotonic fencing), that consumer would reveal coincidentally reused vocabulary, not confirmed cross-domain identity.
- **Documentation gap exposed:** real. The page's single "Confirmed Instances" heading flattens three evidentially distinct claims — one `preexisting_generic_implementation` claim with real `parametric_invariance` support (the seven types) and two `proposed_cross_domain_consumer` claims (the two new consumers) — into one undifferentiated list, exactly the flattening §7 exists to prevent. As this revision's own eligibility check makes explicit, the seven types alone never actually satisfied criterion 4's cross-route requirement; only the two new consumers do. Flagged for that page's own maintenance, not corrected here.

### `mechanisms/authority-preserving-intent-projection.md`

- **Eligibility:** 1–5 all pass. Domain-neutral discover/render/collect-intent/submit/lifecycle-feedback shape; explicit "Does Not Decide" list; substantial residue (the shared lifecycle vocabulary itself); cross-route invariance across Studio (UI mutation) and Runtime Realization (execution policy) — materially different domains; distinctness stated directly against Candidate Resolution and Admission ("composable, never subsuming or subsumed").
- **Evidence kinds present:** `independent_domain_convergence` (Studio, Runtime Realization) — subject to the §7 caveat this page states by name: both are same-campaign artifacts (2026-09-16), so their origin-independence sits at "separate documents, same design campaign," not "independently developed subsystem," despite the source page's own "zero cross-reference" framing. `accepted_cross_domain_consumer` (Studio's command/edit projection — "Accepted 2026-09-14 ... authorized for design work only," a citable acceptance decision, not implemented).
- **Admission rule (§7.1):** satisfied twice over — route A (independent convergence) and route C (Studio's explicit, dated acceptance) each independently suffice.
- **Origin independence:** neither instance reaches above "same design campaign" — this mechanism currently has the thinnest non-manufactured-convergence evidence of the five, even though it passes eligibility identically to the other four; that is a statement about evidentiary confidence, not about whether the mechanism is architecturally real.
- **Falsifier:** if Runtime Realization's operator-policy-overlay, once built, does not implement the shared lifecycle vocabulary (`proposed`/`pending`/`admitted`/`refused`/`completed`/`stale`) but a domain-bespoke state machine instead, the claimed instance demotes to "inspired by," not "an instance of."
- **Documentation gaps exposed — two, both real:**
  1. The page's own "Confirmed Instances" subheading calls the Runtime Realization overlay a "**real, partial instance**," while the same page's own Status section states `implementation: none` and explicitly clarifies that what exists (`operator-switchboard.mjs`) is "a real but partial **precursor**... not an implementation of this mechanism itself." These two characterizations are in direct tension within one page. Applying §7's typed-evidence discipline forces the disambiguation the current prose blurs: this is `proposed_cross_domain_consumer` evidence with a strong precursor, not `implemented_cross_domain_consumer` evidence — the subheading's own wording should say so.
  2. This page's `owner` field names `app-server/docs/work-engine-studio-reconciliation.md`. Both `mechanisms/revision-cas-and-publication.md` and `mechanisms/candidate-resolution-and-admission.md` record an explicit correction of the identical mistake ("an earlier draft named `status-grammar.md` as `owner`... This page is its own canonical owner") and now self-own. This page never received that same correction and still names an external reconciliation document as `owner` for its own recognition status — an inconsistency across the five mechanism pages' own stated convention, not a new question this page invents.

Neither gap changes this mechanism's eligibility or its `design: accepted` status; both are reportable documentation inconsistencies within the existing corpus, surfaced by applying this page's own criterion rather than repaired by it.

---

## 14. Pressure Test: The Two Substrates

Requested directly by review, checked against §6's own substrate-specific criteria rather than assumed identical to §5's mechanism test, then checked separately against §7.1's admission rule — eligibility and admission are different questions, and this section is where that distinction gets its first real test on existing, catalogued content. **No reclassification is made or implied** — both clear eligibility, but they do not both clear admission.

### `substrates/context-observer.md`

- **Eligibility:** 1–5 all pass. The `ContextObservation` schema (identity/usage/composition/activity/lifetime/relationships) is real and source-grounded (`deterministic-authority-projection-and-adaptive-organizational-topology.md` §5.3); explicit "does NOT own" list (lifecycle pressure, replacement applicability, topology/separation pressure, semantic-width interpretation, organizational judgment, any transition decision), governed by its own stated sentence, "metrics like semantic-width pressure, coupling, or projection reduction therefore do not belong in the observer itself"; substantial residue (the full six-category schema survives refusal intact); cross-consumer invariance — Context Lifecycle and Organizational Compilation are explicitly stated peers ("neither gates the other's applicability question"), each deriving materially different downstream evidence (`LifecycleEvidence` vs. `VantageSeparationEvidence`) from the identical observation contract; distinctness stated directly against Evidence Anchor ("same observation/normalization shape, unrelated subject matter... neither substrate is a generalization of the other").
- **Evidence kinds present:** `independent_domain_convergence` (Context Lifecycle, Organizational Compilation — two genuinely separately-owned dimensions, each with their own derivation layer). Notably weaker in one respect than Evidence Anchor's own sourcing: this substrate's multi-consumer case had to be assembled by the architecture-decomposition effort itself from two separate dimension pages, rather than being diagrammed as multi-consumer by a single source document — a calibration note, not a flaw.
- **Admission rule (§7.1):** satisfied by route A — two genuinely separately-owned dimensions, real convergence, not a parametric artifact.
- **Origin independence:** both consumers sit at "separate documents, same design campaign" — moderate evidence against manufactured convergence, no claim made here about the substrate's architectural generality beyond that (§8).
- **Falsifier:** if `ContextObservation`'s fields turned out to be populable only with foreknowledge of which consumer would derive from them — i.e., the schema silently encoding lifecycle- or topology-specific meaning — the substrate would fail semantic neutrality (§6.2) and collapse into whichever consumer it secretly serves.
- **Documentation gap:** none structural. One nuance checked and resolved, not a contradiction: this page's own "What This View Does Not Show" bullet says a generic "observer framework" abstraction "would only be justified if a second, independent substrate later converged on the same shape, which has not happened yet." `substrates/evidence-anchor.md` is now that second substrate, and its own text calls the two "the same shape." Read carefully, these are not in tension — Evidence Anchor's claim is that both independently satisfy the *substrate-kind* shape (observation/normalization only, no derivation, no admission), which is true and is exactly what §6 confirms; Context Observer's hedge is about a separate, still-unaddressed question — whether a *third*, more generic mechanism/base-abstraction unifying substrate-construction itself is warranted. That question remains open; it is not answered or contradicted by either substrate's own recognition.

### `substrates/evidence-anchor.md`

- **Eligibility:** 1, 2, 3, 5 pass cleanly. The `AnchorObservation` schema and exact-revision-binding invariant are real and source-grounded (`evidence-anchor-observation-and-impact-nomination.md` §3); explicit "does NOT own" list (dependency/anchor registry, semantic materiality, durable record publication, refresh-episode consequence, reopening downstream work), with its own corrected leak already documented ("'relevant' smuggles semantic materiality back into the observer"); substantial residue (the five-state comparator plus the bound/observed revision invariant survives refusal intact); distinctness stated directly against Context Observer. **Criterion 4 (cross-consumer invariance) passes only prospectively — one can describe an observation contract another domain could consume without importing Evidence/Claims semantics — but that is a claim about shape, not about whether such a consumer currently exists.**
- **The finding:** this page's own §4, "Multiple Independent Consumers, Already Named by the Source," presents architecture claim, plan claim, and research claim as its confirming multi-consumer evidence — but `evidence-and-claims.md` §1 ("What This Dimension Owns") states directly that this single dimension owns "claim identity, revision chains, provenance, lineage relationships... [answering] what proposition does this claim assert" generically, regardless of subject matter. Architecture/plan/research claims are domain profiles of *one* owning dimension's own generic claim schema, not three independently-owned consumers — exactly the `parametric_invariance` pattern §7 already names for Resource Lease's seven resource types, found here in a second page by applying the same discipline consistently. The substrate's only candidate for genuine cross-domain evidence is the fourth item named in the same source document's own Relationships table — the coordinate/service-state map (`service-plane-and-kernel-domain-boundary.md`, a genuinely different owning dimension) — which that source itself describes only as "not something this document builds a parallel mechanism for": a passing mention, with no citable acceptance decision, thinner even than Resource Lease's two dated, accepted consumers (§13).
- **Evidence kinds present:** `parametric_invariance` (architecture/plan/research claim, all domain profiles of the single Evidence/Claims dimension) — real, but excluded from §7.1's admission rule by definition. `proposed_cross_domain_consumer` (the coordinate/service-state map candidate) — named, but per §7.1, "a merely named, mentioned, or hypothetical candidate consumer... does not satisfy route C." No `independent_domain_convergence`, no `implemented_cross_domain_consumer`, and no `accepted_cross_domain_consumer` evidence currently exists.
- **Admission rule (§7.1): not currently satisfied.** None of routes A–D is met: no independent domain has converged on this shape (A); nothing is built beyond the substrate's own unimplemented core (B); the one candidate consumer has no citable acceptance decision, only a name (C, explicitly excluded); and no owner has made an explicit decision to canonize ahead of convergence (D — this page does not make that decision on its own initiative, and does not treat its absence as license to declare the substrate un-recognized either; §7.1's route D exists precisely for an owner to invoke here if warranted). **This substrate clears §6's shape test and is currently catalogued in the capstone's "2 Shared Substrates," but its own recognition evidence does not currently meet the bar §7.1 states for that catalog entry.** That is not a claim that the abstraction is wrong — the shape may be exactly right — it is a claim that admission currently rests on evidence this page's own rule says is insufficient. Resolving it is a decision for the page's and capstone's own owner (adopt the candidate consumer for real, find another, invoke route D explicitly, or reconsider the catalog entry) — not performed here, per this page's own repeated discipline of reporting contradictions rather than repairing architecture implicitly.
- **Origin independence:** the parametric claim-type evidence sits entirely within one already-implemented dimension (Evidence/Claims); the one named candidate for cross-domain evidence has no ladder placement at all, since it has not been adopted in any form.
- **Falsifier — already triggered, not merely hypothetical:** §9's cross-consumer falsifier ("consumers require materially different semantics" — or, as directly relevant here, fail to be independently-owned consumers at all) is not a future risk for this substrate; applying it just now is what produced this section's finding. The forward-looking version: if the coordinate/service-state map candidate is never adopted and no other genuinely different-domain consumer emerges, this substrate's admission evidence stays at zero indefinitely, not merely "thin."
- **Documentation gap exposed:** real, and structurally identical to Resource Lease's — except that Resource Lease's two consumers turned out to be citably accepted once correctly typed (§13), clearing §7.1 on route C, while Evidence Anchor's one candidate does not. This page's own §4 heading overstates cross-domain convergence by presenting one dimension's own domain-profile parameterizations as "multiple independent consumers." Flagged for that page's own maintenance, not corrected here.

Net for §14: both substrates clear eligibility cleanly. Context Observer also clears §7.1's admission rule, on real independent convergence, and its evidence is honestly moderate with no overclaim. **Evidence Anchor clears eligibility but does not currently clear admission** — the same parametric/cross-route conflation Resource Lease had, but where Resource Lease's correction revealed stronger real evidence underneath (two dated acceptances), Evidence Anchor's correction reveals weaker evidence than its own text claims. One consistent test, applied without protecting either page's existing catalog status, produced two different verdicts — which is the result this page exists to be capable of producing.

---

## 15. Related Architecture Views

- **`work-engine-planned-architecture.md`** §1 — names this page and links here rather than restating the criterion; §5 and §11 cite this page's pressure-test results directly.
- **`status-grammar.md`** §7, §10.6 — the authorization-composition corollary and worked example this page's §12 depends on; recognition (this page) and authorization (that page) remain independent axes.
- **`mechanisms/revision-cas-and-publication.md`, `mechanisms/candidate-resolution-and-admission.md`, `mechanisms/transition-fencing-and-leases.md`, `mechanisms/resource-lease-and-fencing.md`, `mechanisms/authority-preserving-intent-projection.md`** — the five pressure-tested in §13; none reclassified.
- **`substrates/context-observer.md`, `substrates/evidence-anchor.md`** — the two pressure-tested in §14; no reclassification made, but Evidence Anchor is flagged as clearing eligibility (§6) without currently clearing the admission rule (§7.1) — an open question for that page's and the capstone's own owner, not resolved here.

---

## Source and Status

```yaml
architecture_status:
  design: accepted
  reconciliation: reconciled
  authorization: design_work_authorized
  implementation: none
  owner: app-server/docs/architecture/architectural-kind-recognition.md
  status_as_of: 2026-09-18
```

`design: accepted` — this page's own recognition criterion, including two same-day revisions (the kind classifier and three kind-specific eligibility sections; then the fact-plane/protocol-plane classifier fix and the §7.1 canonical admission rule), was explicitly settled through direct discussion (2026-09-18), the same bar every mechanism and dimension recognition in this architecture is held to. `reconciliation: reconciled` — pressure-tested directly against all five current mechanism pages, both substrate pages, `status-grammar.md`, and `work-engine-planned-architecture.md` §1/§5/§11, this session; real documentation gaps and one real admission-evidence shortfall were found across the two pressure tests and are recorded in §13–§14 rather than corrected on the pressure-tested pages. `authorization: design_work_authorized` — this page is a documentation and taxonomy convention, not a buildable artifact; nothing here authorizes reclassifying or rebuilding anything it pressure-tests, and §14's Evidence Anchor finding is explicitly left for that page's and the capstone's own owner to resolve. `implementation: none` — not applicable to a taxonomy page. `owner`: self, following the corrected convention `mechanisms/revision-cas-and-publication.md` and `mechanisms/candidate-resolution-and-admission.md` already use, and the correction §13 recommends for `mechanisms/authority-preserving-intent-projection.md`.

**Revised 2026-09-18 (first pass)** from the same-day original, per direct review: split the single, mechanism-shaped eligibility test into a kind classifier (§3) plus three kind-specific eligibility sections (§4–§6), rather than declaring substrates "structurally eligible for the identical test" without checking; made explicit that recognition evidence (§7) never assigns `status-grammar.md`'s own status values; narrowed the origin-independence ladder (§8) to claim only non-manufactured convergence, not architectural generality; separated `parametric_invariance` from cross-route/cross-consumer invariance and corrected Resource Lease's §13 entry accordingly; added the substrate pressure test (§14).

**Revised 2026-09-18 (second pass)**, per direct review of the first pass's own results: (1) sharpened the kind classifier (§3) from "supplies facts, semantically neutral" — a property mechanisms share too (Candidate Resolution's 0/1/N, Revision/CAS's head/staleness, and Transition Fencing's fence validity are all neutral facts) — to the fact-plane-vs-protocol/control-plane discriminator actually doing the work: a substrate's authoritative output never gates admission, mutation, publication, or transition; a mechanism's does; (2) reworded the Dimension eligibility criterion (§4.3) from "commitment or belief" to "authoritative meaning that cannot be reconstructed from a neutral observation contract alone," so descriptive dimensions like Evidence/Claims are caught without sounding like policy; (3) added §7.1, the Canonical Admission Rule, closing the gap the first pass's own eligibility/evidence split had opened but not finished: eligibility is necessary, but canonical recognition additionally requires at least one non-parametric route (independent convergence, an implemented consumer, an *accepted* consumer, or an explicit owner decision) — a merely named or mentioned candidate does not qualify; (4) split `proposed_cross_domain_consumer` into that weaker sense and a new `accepted_cross_domain_consumer` for a citable, dated acceptance decision, and retagged Resource Lease's two consumers and APIP's Studio instance accordingly, since both were, on inspection, actually accepted rather than merely proposed; (5) applied §7.1 to §14 without protecting existing catalog status, and found that `substrates/evidence-anchor.md` clears §6's eligibility test but does not currently satisfy the admission rule under any of routes A–D — its one candidate for cross-domain evidence is a passing mention, not an acceptance decision. That finding is recorded in §14 and left for the page's and capstone's own owner to resolve, not acted on here.
