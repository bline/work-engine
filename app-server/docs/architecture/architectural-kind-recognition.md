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

A candidate that clears §4, §5, or §6 is eligible to its respective kind. Whether it should actually be canonized is a longer chain than "eligible, therefore recognized" — this page holds itself to the same discipline the rest of the architecture already applies to observation-bearing claims (`substrates/evidence-anchor.md`'s observation → comparison → nomination-candidate, itself not a publication; `mechanisms/candidate-resolution-and-admission.md`'s candidate space → reduction → residual judgment → admission, each stage owned separately):

```text
KIND ELIGIBILITY (§4-§6)
    is the candidate the right shape for its kind at all
        ↓
EVIDENCE (this section, below)
    what has actually been observed, typed rather than counted
        ↓
CANONIZATION WARRANT (§7.1)
    does the typed evidence add up to a judgment that canonization is
    architecturally appropriate -- a recommendation, not a transition
        ↓
AUTHORITY-BEARING ADMISSION (§7.2)
    the transition itself -- entering the candidate into the canonical
    taxonomy -- which requires authority this page does not hold,
    define, or assume already exists in verified form
        ↓
AUTHORITATIVE SUCCESSOR PUBLICATION (mechanisms/revision-cas-and-
publication.md, in principle -- §7.2)
    the admitted taxonomy state becomes the new authoritative revision,
    through the same CAS-published-succession discipline every other
    authoritative state change in this architecture already uses --
    not a step this page invents new machinery for
        ↓
CAPSTONE CATALOG PROJECTION (work-engine-planned-architecture.md tables)
    a rendering of whatever the current authoritative revision is --
    never the authority, never the publication mechanism itself
```

Collapsing warrant into admission is exactly the error `status-grammar.md` §8 already names for design status generally — "Evidence supporting acceptance is not acceptance... Reconciliation work may recommend or justify such a change; it cannot perform it" — restated here for kind recognition rather than invented fresh. Collapsing publication into the capstone's own tables is the identical error one stage further downstream, addressed directly in §7.2. §7.1 produces warrant. §7.2 states what admission and publication actually require, and explicitly declines to invent either. **Instance counts are supporting bookkeeping, never the criterion itself, at any of these stages.** `mechanisms/revision-cas-and-publication.md`'s own status history is the direct proof: its confirmed-instance count moved from four to seven on 2026-09-16 ("found by the final mechanical audit to be undercounted here as 'four' — corrected") with zero change to `design: accepted` — the mechanism was never *made* real by reaching seven; the correction only repaired the record of evidence that was already there.

**Recognition does not assign `design`, `reconciliation`, `authorization`, or `implementation` status, and evidence/warrant (this section, §7.1) does not itself perform admission (§7.2).** Status remains independent assertions whose authority and evidence are governed entirely by `status-grammar.md`; admission remains a distinct, authority-bearing transition governed by §7.2. A candidate can be canonically recognized here as mechanism-shaped or substrate-shaped and still be `design: proposed`, `authorization: exploration_only`, or `implementation: none` — exactly what `mechanisms/candidate-resolution-and-admission.md` and both current substrates already are. Recognition answers "is this the right kind, with enough evidence to warrant naming it" — never "has this been accepted, authorized, built, or actually admitted."

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

These are not mutually exclusive, and a single candidate typically carries more than one at once — see §13–§14 for how each current mechanism and substrate actually breaks down. Three things must be stated explicitly, because the existing corpus has blurred exactly these distinctions under one undifferentiated "Confirmed Instances" heading in more than one page (§13, §14 name where):

- **Resource types parameterizing one generic implementation are `parametric_invariance`, not `independent_domain_convergence`.** `mechanisms/resource-lease-and-fencing.md`'s seven `RESOURCE_TYPES` (`directory, git-ref, git-index, port, index, review-budget, database`, verified against `contract.mjs:4-6`) are seven parameterizations of one already-generic `workspace-coordination` service, not seven independently-arrived-at domains that separately discovered the need. That is real, legitimate evidence, but it is a different evidentiary claim than convergence, and conflating the two overstates how many independent reasoning processes actually endorsed the shape.

- **"Zero cross-reference" is evidence of textual independence, not necessarily independent origin.** `mechanisms/authority-preserving-intent-projection.md` states its Runtime Realization instance "was written with **zero cross-reference to Studio or its reconciliation**, yet follows the identical shape" — real evidence that one author didn't copy the other's prose. It is not evidence that the two reasoning processes were separated by anything more than authorship, since both are products of the same 2026-09-16 architectural-decomposition campaign. See §8's origin-independence ladder for why this distinction has to be kept explicit rather than collapsed into a single "independent" label.

- **Instance counts are supporting evidence, never the recognition criterion itself** (restated from the opening of this section because it is the single most load-bearing sentence on this page).

### 7.1 Canonization Warrant

§4–§6's eligibility tests are necessary. They are not sufficient. A candidate that clears its kind's own eligibility test still needs enough typed evidence to support a **warrant** — an architectural judgment that canonization is appropriate — before it can even be presented for admission (§7.2). This section states what counts as sufficient warrant. It does not state, and must not be read as stating, that meeting it *is* canonical admission; §7.2 exists specifically because those are two different things.

**Warrant branches by kind, because "cross-owner need" is not a coherent warrant question for every kind.** A mechanism or substrate exists specifically to be reused *by* other owners — cross-owner need is the whole reason either kind gets canonized at all, so it is the right thing to demand evidence for. A dimension exists to *be* the sole owner of its own truth class; asking whether other owners have independently converged on needing to own the identical truth is not a weaker version of the same question, it is close to incoherent — if two independently-owned processes both claimed ownership of the identical truth class, that would be a §4.2 non-duplication failure, not confirming evidence for a new dimension. Applying the mechanism/substrate warrant test to a dimension candidate would therefore import a criterion that dimension eligibility (§4) already excludes by construction.

**Mechanism and substrate warrant** — requires eligibility (§5 or §6) plus at least one non-parametric route establishing actual cross-owner need:

> ```text
> A. independent_domain_convergence
> OR
> B. implemented_cross_domain_consumer
> OR
> C. accepted_cross_domain_consumer
> OR
> D. an explicit, external architectural judgment that canonization is
>    warranted ahead of natural convergence — invoked deliberately, not
>    used as a default fallback whenever routes A–C come up short.
> ```

`parametric_invariance` never satisfies this rule on its own, however many parameterizations exist — it establishes that one implementation generalizes cleanly within its own domain, never that a second domain needs it. **A merely named, mentioned, or hypothetical candidate consumer — accepted by no one, built by no one — is corroborating context, not warrant; it does not satisfy route C.** This is deliberately not a numeric threshold (not "two instances," not "three consumers") — it is a named set of evidence shapes, any one of which is enough, because the shapes themselves are what make convergence credible, not their count.

**Route D produces warrant, not admission — the two must not be silently combined.** A judgment that canonization is architecturally appropriate ahead of natural convergence is exactly as strong, and exactly as inert on its own, as routes A–C: all four routes tell you *that canonization would be warranted*; none of the four *is* the act of canonizing. Route D's judgment must additionally be **external to the candidate's own page** — citable to a document or discussion distinct from the candidate's own proposing text, exactly as `status-grammar.md` §5 already requires for `design: accepted` ("a status without a named owner cannot be `accepted`... those stronger claims exist because *something* decided them, and that something must be nameable"). A candidate's own page asserting that it deserves canonization is self-sourced, not externally judged, and does not satisfy route D — identical to the discipline `status-grammar.md` §8 already states for status generally ("evidence supporting acceptance is not acceptance"). Route D exists so a qualified external judgment can establish warrant ahead of convergence; it does not exist so a candidate can bootstrap its own canonization by proposing itself as sufficiently important, and it does not exist so that judgment can double as the admission transition §7.2 still separately requires.

**Dimension warrant** — requires eligibility (§4) plus:

```text
- a real, citable non-duplication check against every existing
  dimension, mechanism, and substrate (§4.2 performed as an evidentiary
  act, not merely asserted); AND
- either (a) real, substantial content already exists that would be
  misclassified, lost, or homeless if folded into an existing owner, or
  (b) an explicit, external architectural judgment establishing the new
  truth class, subject to the identical external-source requirement as
  route D above.
```

**"Substantial homeless content" is warrant, not admission, here exactly as everywhere else in this section.** Real content with nowhere to go is a strong reason to judge that a new truth owner is warranted; it is not itself the act of establishing one, and this branch must not be read as letting homeless content bootstrap its own dimension merely by existing and being real. Independent convergence, an implemented consumer, or an accepted consumer are not required and are not meaningful evidence here — a dimension is not a shared reusable protocol waiting to be adopted by other owners; it is the owner. This branch is stated for completeness; no dimension is pressure-tested against it in this revision, matching §4's own scope note.

§13 and §14 apply the mechanism/substrate branch directly to the five current mechanisms and two current substrates. One candidate — `substrates/evidence-anchor.md` — clears §6's eligibility test but does not currently establish warrant under any of routes A–D; §14 and §7.3 state this plainly rather than protecting the substrate's existing catalog status.

### 7.2 Authority-Bearing Admission and Publication

**Warrant is not admission.** Admission is the transition that actually enters a candidate into the canonical taxonomy, and — exactly like every admission transition already named elsewhere in this architecture — it requires an authority to perform it, verified as holding that authority. `mechanisms/candidate-resolution-and-admission.md`'s own shape makes the general case directly: reduction can establish that a residual choice exists, but "admission is exercised by whichever domain-owner class `authority-and-ownership.md`'s own model grants that authority to, never by the mechanism." This page's own warrant test (§7.1) is the reduction; it is not the admission, and it does not name who may perform one.

**A transition needs a target state, and pressure-testing this page's own admission concept requires checking who owns *that* — not inferring it from who decides, publishes, or renders.** The proposition "X is currently an admitted Dimension/Mechanism/Substrate" is itself a candidate class of architectural truth, and none of the four things this page has carefully separated actually owns it: this page owns the eligibility/warrant *criterion*, never the membership state a candidate is being tested against (§7.1's own routes evaluate evidence, they do not record a result); `authority-backed-architecture-directions-as-workflow-inputs.md` §18 item 1 concerns *who may decide* architecture questions, not what taxonomy membership currently *is* — deciding and owning the resulting truth are different things, the same distinction `authority-and-ownership.md`'s own vocabulary (observe/nominate/decide/admit/execute) keeps separate everywhere else it appears; `mechanisms/revision-cas-and-publication.md` supplies successor-publication discipline "without that discipline ever becoming a shared owner of what the state means" — its own governing page, `authority-and-ownership.md` §12's neighborhood, states directly that this mechanism is "not a truth dimension"; and the capstone is a projection, which by definition renders a truth it does not own. No dimension among the 13 answers it either — none of their own content is about which architecture-view patterns are canonically recognized. **No existing canonical truth owner owns this proposition.** That is real, confirmed residue, not something this page, the authority-residue document, the publication mechanism, or the capstone should silently acquire merely because each is adjacent to it. It is recorded as a new item in the capstone's own §11 Deferred Architecture ledger rather than absorbed here.

**This page deliberately does not define the architecture-decision authority itself either, because the corpus already has an open, unresolved residue question that names it exactly:** `app-server/ideas/pending/authority-backed-architecture-directions-as-workflow-inputs.md` §18 item 1, `[KIND: RESIDUE] [OPEN]`: *"Who owns architecture decisions, and how is that authority represented and verified?"* Inventing an answer here — "an architectural owner," "this catalog's owner," or any other unverified stand-in — would silently resolve someone else's open residue by assumption, exactly the failure mode `status-grammar.md` §8 and this architecture's own residue discipline (capstone §11) exist to prevent. This page **consumes** that eventual authority; it does not define it, and it does not treat its own absence as license to either block all admission indefinitely or admit on warrant alone.

**Until that residue resolves, the best available proxy is the same dated, external, citable decision event `status-grammar.md` already uses for `design: accepted`** — but it must be named precisely as a *design*-acceptance event, not conflated with a *taxonomy-admission* event, because those are now two different, separately-unverifiable things. This is what actually happened for the five mechanisms pressure-tested in §13 — each carries a real, dated "explicitly settled through direct discussion" record in its own Source and Status. **That record is admission-shaped evidence — a real, citable design-acceptance decision that a reasonable admission process would plausibly point to — never proof that a distinct taxonomy-admission event, or authority for one, actually occurred.** Two separate gaps stand between the proxy and the real thing: whether the actor held generally-verified architecture-decision authority (item 1, above) is one; whether "taxonomy admission" is even a defined, checkable transition with its own evidentiary requirements is the other, and it is not — no artifact records one, and no owner has been found for the state it would modify (capstone §11 item 6). This page's own admission claims are therefore proxy-level at best for every candidate checked, including the five mechanisms; §13 and §14 state this precisely rather than silently upgrading a design decision into a verified admission.

**Checked directly against every one of the 20 canonical views' own Source and Status blocks, rather than assumed uniform — the pattern is real but far less uniform than a single "corpus-wide" description would claim.** Three genuinely different provenance shapes exist, and none of the substrates share the mechanisms' own pattern at all:

```text
the five mechanisms
    design: accepted, uniformly, each citing "explicitly settled
    through direct discussion (2026-09-16)" -- the dated,
    external, citable-decision pattern this section describes

four of the thirteen dimensions (review.md, proposal-evaluation.md,
decision-specific-readiness.md, portfolio-selection.md)
    design: accepted, each citing a differently-dated "Accepted
    2026-09-14" decision in its own named reconciliation document --
    the same shape, a different session date and citation form

two of the thirteen dimensions (role-and-contract-structure.md,
evidence-and-claims.md)
    design: accepted, but grounded in pre-existing, already-
    implemented reality (real code, real prior documents) rather
    than any single decision event -- a materially different, and
    arguably stronger, provenance shape that this proxy standard
    does not actually describe

the remaining seven dimensions, and BOTH substrates including
context-observer.md (corrected here -- an earlier draft of this
page wrongly credited it with the mechanisms' own pattern)
    design: proposed -- no acceptance decision exists yet at all,
    so the provenance question this section raises does not yet
    apply to them the same way: there is no dated event to check
    against a future authority model, because there is no
    acceptance event yet, full stop
```

**The bounded historical-provenance limitation this page names therefore applies cleanly only to the eleven views (five mechanisms, four dimensions) that actually carry a dated, citable decision event** — that is a gap in *verifiability against a not-yet-settled authority model*, not evidence that any of those eleven decisions was unauthorized; a later-arriving authority model does not by itself prove an earlier admission unauthorized, and instead creates a provenance-reconciliation obligation once that model exists. The two implementation-grounded dimensions raise a different question this page does not attempt to answer (whether pre-existing reality is itself a valid provenance shape under a future authority model, independent of any decision event). The seven proposed dimensions and both substrates are not yet at the stage where this question is even live. **When item 1 eventually resolves, the correct next step is a separate reconciliation pass checking the eleven dated decisions against whatever authority model that resolution defines — not a claim made or implied here that any of the twenty views' current standing needs to change today.** This page does not perform that reconciliation; it names the limitation, checked rather than assumed, so a future resolution of item 1 inherits an accurate worklist rather than an inflated one.

**Publication is a distinct, later stage — an authoritative successor state, not a rendering, and the capstone's tables are the rendering, not the authority.** Work Engine already has a canonical mechanism for exactly this kind of state change: `mechanisms/revision-cas-and-publication.md`'s own generalized discipline — identity as content digest, an explicit predecessor chain, a computed head, compare-and-swap against that head, atomic visibility — exists precisely so that "authoritative state advances safely... across every dimension that owns durable state of its own... without that discipline ever becoming a shared owner of what the state means." Taxonomy membership, once authoritatively instantiated, would be durable state of exactly this kind, and an admission would be exactly the sort of successor-revision event that mechanism already generalizes over — stated conditionally because, per capstone §11 item 6's own final finding, no such instantiation has actually occurred yet. **In principle, then, the correct chain is: authority-bearing admission (above) → authoritative successor publication, through that mechanism's own discipline → capstone catalog projection, a rendering of whichever revision is currently authoritative.** In practice, no such revisioned taxonomy artifact currently exists — the capstone's `work-engine-planned-architecture.md` file is edited directly, with no CAS check against a prior head, no predecessor chain, and no atomicity guarantee. That is a real, current gap between principle and practice, named here rather than silently treated as already closed; this page does not build that artifact, since doing so is a mechanism-instantiation decision for `mechanisms/revision-cas-and-publication.md`'s own consumers to make, not a taxonomy-recognition question. What follows directly from naming the gap, stated conditionally rather than absolutely because it is conditional: **editing the capstone's own tables is not, and must not be treated as, the operation that makes a candidate's admission authoritative — because, per capstone §11 item 6's own final finding, there is currently no authoritative taxonomy state for such an edit to change, only a rendering of a real, reconciled, proxy-strength catalog.** The capstone's own stated Authority line ("does not accept any idea, authorize implementation, amend a migration roadmap, or select a permanent schema... does not resolve anything the views themselves leave open") proves only that it lacks decision authority; that alone would not settle the matter if an authoritative membership state already existed and the capstone merely lacked an owner for it, or if the capstone itself later became that state's authoritative register — in either case, editing it under proper authorization could be exactly how admission becomes authoritative. Capstone §11 item 6 tests this directly, past the ownership question, to whether an authoritative state exists at all to be owned, and finds no positive evidence that it does: no artifact anywhere records a discrete admission-and-publication event for any candidate, and `mechanisms/revision-cas-and-publication.md`'s own discipline (content-digest identity, predecessor chain, computed head) has never actually been instantiated for "the canonical taxonomy" as an artifact. That finding, not the Authority line by itself and not mere absence of an owner, is what the claim here rests on. The capstone performs no publication in the mechanism's own sense today and cannot resolve a corpus-conformance conflict (§7.3) merely by virtue of being the place such conflicts are listed. "The catalog" is a rendering of a proxy-strength compilation, not the publication of authoritative state and not the authority — a distinction this page must keep as sharply as it keeps eligibility separate from warrant.

### 7.3 Corpus-Conformance Conflicts: Neither Removal Nor Grandfathering

A rule stated here can be — and, in §14, already has been — clearer than the evidence some pre-existing catalogued content actually carries. That is not a defect in the rule; it is the expected consequence of formalizing a criterion that was previously implicit precedent. Two wrong responses are equally available and both are refused:

```text
WRONG: silent removal
    Delete or reclassify the catalogued item the moment it fails a
    newly-stated rule, without an authority-bearing admission decision in
    either direction -- this page pressure-tests and reports warrant; it
    holds no admission authority of its own (§7.2) and cannot exercise
    one by deleting an entry any more than by adding one.

WRONG: silent grandfathering
    Exempt pre-existing catalog entries from a newly-stated rule merely
    because they predate it -- this would make every existing entry
    permanently unaccountable to any future refinement of this page,
    defeating the purpose of stating the rule at all.
```

**The correct disposition is a named, standing state: a corpus-conformance conflict.** A catalogued dimension, mechanism, or substrate is in conformance conflict when it clears its kind's own eligibility test (§4–§6) but does not currently establish warrant under §7.1 on the evidence its own page actually states. The catalog entry stands exactly as published; the conflict is recorded here and at the capstone's own point of reference for that item. **Resolution requires an authority-bearing admission act under §7.2** — sourced from whatever eventually answers `authority-backed-architecture-directions-as-workflow-inputs.md` item 1, not from this page and not from the capstone, which per its own Authority line holds no such authority either. Until that happens, the conflict simply stands, recorded rather than acted on in either direction.

**This is a status question, not only a taxonomy question, and it interacts with `status-grammar.md` in one specific, already-stated way.** §4.2 there governs exactly this situation: `reconciled` is scope-relative to what was known and declared as of a claim's own `status_as_of` date, and "a new document appearing later does not retroactively falsify an existing `reconciled` marker; it creates new reconciliation work, tracked as its own claim." `substrates/evidence-anchor.md`'s own `reconciliation: reconciled` (`status_as_of: 2026-09-16`) is therefore **not falsified** by this page's later warrant rule — that marker remains true to what was known on 2026-09-16, before §7.1 existed. What §4.2 requires instead is exactly what §14 already is: **a separately stated corpus-conformance finding**, tracked as its own claim rather than folded backward into the substrate's existing status block. `substrates/evidence-anchor.md` should not be edited to change its own `reconciliation` value on the strength of this page alone; a future reconciliation pass on that page, citing this finding, is the correct vehicle for that — not a retroactive edit made here.

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

Checked against §5, §7 (including §7.1's warrant test), §8, and §9 directly. **No reclassification is made or implied by this section** — all five clear eligibility, and all five establish canonization warrant under §7.1. Each also carries a real, dated, external "explicitly settled through direct discussion" design-acceptance decision in its own Source and Status — admission-shaped evidence, in §7.2's own terms, never a verified taxonomy-admission event: it is not proof that generally-verified architecture-decision authority governed it (item 1, still open), and taxonomy admission itself is not a defined, checkable transition anything currently records (capstone §11 item 6, also open). What follows is the evidentiary and documentation picture the criterion exposes, including real gaps, reported rather than repaired.

### `mechanisms/revision-cas-and-publication.md`

- **Eligibility:** 1–5 all pass. Normalized shape (identity/predecessor/head/CAS/atomic-publish/stale-rejection) verified domain-blind; explicit "Does Not Decide" list; substantial residue (the six-item reusable shape); cross-route invariance across five materially different owning dimensions; distinctness stated directly against both Candidate Resolution ("different questions, frequently composed") and Transition Fencing ("built directly on top of that one and does not restate its predecessor/head semantics").
- **Evidence kinds present:** `independent_domain_convergence` (Evidence/Claims, Context Lifecycle, Organizational Compilation, Semantic Planning, Material Decision Selection, Runtime Realization — six dimensions, each citing its own source text); `implemented_cross_domain_consumer` (Evidence/Claims, Context Lifecycle, and Runtime Realization's nested executable-generation substrate, all verified against real code); `accepted_cross_domain_consumer` (Organizational Compilation, "Accepted 2026-09-15 as design"); `proposed_cross_domain_consumer` (Semantic Planning, Material Decision Selection, Runtime Realization's own top-level `RoleRealization` lineage — each named in its own source but not citably accepted at this specific consumption).
- **Canonization warrant (§7.1):** satisfied by routes A and B alone, independent of the weaker proposed instances.
- **Origin independence:** mixed. Evidence/Claims and Context Lifecycle's *code* sits at "implementation predating the synthesis that recognized the pattern" (real, running services checked directly) — strong evidence the convergence wasn't manufactured, not itself a claim about generality; the *naming of the shared mechanism* across all seven cited instances sits at "separate documents, same design campaign."
- **Falsifier:** an instance whose "publish" can silently merge, reorder, or last-write-win against a stale head is not this mechanism's shape regardless of how it is labeled.
- **Documentation gap exposed:** the "post-execution implementation acceptance" instance (`completion-publication.mjs`) composes three already-recognized dimensions' outputs rather than belonging to any one dimension — a genuine fifth evidence shape (a *composite* instance) that §7's typed kinds do not name. Flagged as an open extension to this page's own taxonomy, not resolved here.

### `mechanisms/candidate-resolution-and-admission.md`

- **Eligibility:** 1–5 all pass. Domain-neutral `AVAILABLE ∩ AUTHORIZED ∩ SATISFIES(REQUIRED) → {0/1/N}` shape; explicit "Does Not Decide" list; substantial residue (the zero/one/many distinction itself, stated as "the central fact this mechanism exists to establish"); cross-route invariance across three materially different candidate universes; distinctness stated directly against both Revision/CAS and `mechanisms/authority-preserving-intent-projection.md` ("Neither subsumes the other").
- **Evidence kinds present:** `independent_domain_convergence` (Organizational Compilation, Runtime Realization, Material Decision Selection — three dimensions; a fourth, Portfolio Selection, is explicitly self-labeled by its own source as "likely, not yet formally confirmed," i.e. `proposed_cross_domain_consumer`, correctly hedged already). No `implemented_cross_domain_consumer` evidence at all — `implementation: none` across every confirmed instance.
- **Canonization warrant (§7.1):** satisfied by route A alone — three independently-owned dimensions is real cross-owner need, even with zero implementation.
- **Origin independence:** all three confirmed instances sit at "separate documents, same design campaign" (all reconciled or falsifier-tested within the 2026-09-15/16 window); none reaches "independently developed subsystem" or "predates synthesis," since none has running code — a weaker non-manufactured-convergence signal than Revision/CAS's, independent of either mechanism's actual generality.
- **Falsifier:** a claimed instance whose residual N-case judgment cannot actually be reduced to a domain-supplied decision (e.g., needing the mechanism itself to supply a domain predicate, or producing a fourth outcome class) does not confirm this mechanism.
- **Documentation gap exposed:** none structural. Worth stating plainly as an evidentiary-confidence note (not a failure): this mechanism's entire canonical status currently rests on same-campaign, zero-implementation convergence — a materially thinner evidentiary base than Revision/CAS's, even though both pass eligibility identically. §5's criteria do not distinguish them; only §7's typed evidence does, which is exactly the separation this page exists to make visible.

### `mechanisms/transition-fencing-and-leases.md`

- **Eligibility:** 1, 2, 3, 5 pass cleanly. Criterion 4 (cross-route invariance) is **partially open, by the page's own admission**: the shared revision-binding discipline (bind → prepare → revalidate → publish → activate → release) is confirmed invariant across all three consumers — three genuinely separate owning dimensions, not parameterizations of one — but whether "two named fence classes plus a third, unclassified instance" is a settled taxonomy or an incomplete one is explicitly unresolved in the source page itself ("left open rather than decided by this edit"). This does not fail eligibility — the base invariant clears criterion 4 — but the internal fence-class taxonomy built on top of it is not yet a closed question.
- **Evidence kinds present:** `implemented_cross_domain_consumer` (Context Lifecycle's real transition-lease sequence; Runtime Realization's real executable-generation reload, verified directly against `executable-generation-manager.mjs`/`executable-generation-store.mjs`); `proposed_cross_domain_consumer` (Organizational Compilation's topology-transition fence, named in `deterministic-authority-projection-and-adaptive-organizational-topology.md` Part 7.3 — itself `design: proposed`, not `accepted`, per that document's own status — no compiler exists).
- **Canonization warrant (§7.1):** satisfied by route B alone (two real, independently-owned, implemented consumers).
- **Origin independence:** Context Lifecycle's and the executable-generation reload's implementations both sit at "implementation predating the synthesis that recognized the pattern" — both are real, pre-existing code the mechanism's own naming discovered rather than invented (the executable-generation instance explicitly "found 2026-09-16" as a symptom, not authored to fit) — strong evidence against manufactured convergence for those two. Organizational Compilation's topology-transition fence sits at "separate documents, same design campaign," the weakest of the three.
- **Falsifier:** an instance that turns out to confer standing, renewable exercise authority (violating §11's reciprocal rule) is a Resource Lease instance mis-filed here, not a third fence class.
- **Documentation gap:** the internal two-vs-three-fence-class question is real but already correctly hedged in both the source page and the capstone's own §5 table ("a third real instance protecting neither, never collapsed into one lock") — no overstatement found, no new gap to report; recorded here as an open item this page inherits rather than resolves.

### `mechanisms/resource-lease-and-fencing.md`

- **Eligibility:** 1, 2, 3, 5 pass cleanly. Criterion 4 (cross-route invariance) does **not** pass on the seven resource-type parameterizations directly — `directory/git-ref/git-index/port/index/review-budget/database` are one generic `workspace-coordination` implementation's own parametric invariance (§5's own note under criterion 4), not materially different owning domains. It passes instead, more thinly, on the two prospective consumers: fenced active-binding (owned by Runtime Realization) and review-scope protection (owned by `review.md`) are genuinely different owning domains reusing the identical acquire/`admitMutation` shape without changing its meaning — both real, accepted-for-implementation designs, neither built. Eligibility is a category question, not a confidence one, so an accepted-but-unbuilt cross-domain design is sufficient to clear criterion 4; the evidence typing below is where the resulting confidence is honestly recorded as thinner than "seven confirmed instances" suggests.
- **Evidence kinds present:** `preexisting_generic_implementation` (the `workspace-coordination` core itself, verified against `contract.mjs`, predating the reconciliation-queue process entirely) plus `parametric_invariance` (its seven resource types — real implementation-robustness evidence, not cross-domain evidence) plus `accepted_cross_domain_consumer` ×2 (fenced active-binding — "Accepted 2026-09-14"; review-scope protection — "accepted 2026-09-14" — both explicit, citable design-acceptance decisions from their own distinct reconciliation documents, not mere mentions). No `implemented_cross_domain_consumer` evidence exists yet beyond the generic mechanism's own resource types.
- **Canonization warrant (§7.1):** satisfied by route C — two explicit, dated acceptance decisions from two different owning domains is real committed cross-owner need, even fully unbuilt. This is the sharper reading criterion 4 already implied: the mechanism's admissibility never rested on the seven resource types at all.
- **Origin independence:** `workspace-coordination` sits at "implementation predating the synthesis that recognized the pattern" (independently named elsewhere as "the strongest kernel-shaped primitive found anywhere in the inventory," per its own status block) — strong evidence the recognition wasn't manufactured; not itself evidence that the two prospective consumers will pan out. The two accepted consumers sit at "separate documents, same design campaign" (both accepted 2026-09-14, within the decomposition effort).
- **Falsifier:** if, once built, either prospective consumer needs a materially different admission or staleness rule than `admitMutation`'s exactly-once-per-operation-id CAS check (e.g., multi-holder shared exercise, or non-monotonic fencing), that consumer would reveal coincidentally reused vocabulary, not confirmed cross-domain identity.
- **Documentation gap exposed:** real. The page's single "Confirmed Instances" heading flattens three evidentially distinct claims — one `preexisting_generic_implementation` claim with real `parametric_invariance` support (the seven types) and two `proposed_cross_domain_consumer` claims (the two new consumers) — into one undifferentiated list, exactly the flattening §7 exists to prevent. As this revision's own eligibility check makes explicit, the seven types alone never actually satisfied criterion 4's cross-route requirement; only the two new consumers do. Flagged for that page's own maintenance, not corrected here.

### `mechanisms/authority-preserving-intent-projection.md`

- **Eligibility:** 1–5 all pass. Domain-neutral discover/render/collect-intent/submit/lifecycle-feedback shape; explicit "Does Not Decide" list; substantial residue (the shared lifecycle vocabulary itself); cross-route invariance across Studio (UI mutation) and Runtime Realization (execution policy) — materially different domains; distinctness stated directly against Candidate Resolution and Admission ("composable, never subsuming or subsumed").
- **Evidence kinds present:** `independent_domain_convergence` (Studio, Runtime Realization) — subject to the §7 caveat this page states by name: both are same-campaign artifacts (2026-09-16), so their origin-independence sits at "separate documents, same design campaign," not "independently developed subsystem," despite the source page's own "zero cross-reference" framing. `accepted_cross_domain_consumer` (Studio's command/edit projection — "Accepted 2026-09-14 ... authorized for design work only," a citable acceptance decision, not implemented).
- **Canonization warrant (§7.1):** satisfied twice over — route A (independent convergence) and route C (Studio's explicit, dated acceptance) each independently suffice.
- **Origin independence:** neither instance reaches above "same design campaign" — this mechanism currently has the thinnest non-manufactured-convergence evidence of the five, even though it passes eligibility identically to the other four; that is a statement about evidentiary confidence, not about whether the mechanism is architecturally real.
- **Falsifier:** if Runtime Realization's operator-policy-overlay, once built, does not implement the shared lifecycle vocabulary (`proposed`/`pending`/`admitted`/`refused`/`completed`/`stale`) but a domain-bespoke state machine instead, the claimed instance demotes to "inspired by," not "an instance of."
- **Documentation gaps exposed — two, both real:**
  1. The page's own "Confirmed Instances" subheading calls the Runtime Realization overlay a "**real, partial instance**," while the same page's own Status section states `implementation: none` and explicitly clarifies that what exists (`operator-switchboard.mjs`) is "a real but partial **precursor**... not an implementation of this mechanism itself." These two characterizations are in direct tension within one page. Applying §7's typed-evidence discipline forces the disambiguation the current prose blurs: this is `proposed_cross_domain_consumer` evidence with a strong precursor, not `implemented_cross_domain_consumer` evidence — the subheading's own wording should say so.
  2. This page's `owner` field names `app-server/docs/work-engine-studio-reconciliation.md`. Both `mechanisms/revision-cas-and-publication.md` and `mechanisms/candidate-resolution-and-admission.md` record an explicit correction of the identical mistake ("an earlier draft named `status-grammar.md` as `owner`... This page is its own canonical owner") and now self-own. This page never received that same correction and still names an external reconciliation document as `owner` for its own recognition status — an inconsistency across the five mechanism pages' own stated convention, not a new question this page invents.

Neither gap changes this mechanism's eligibility or its `design: accepted` status; both are reportable documentation inconsistencies within the existing corpus, surfaced by applying this page's own criterion rather than repaired by it.

---

## 14. Pressure Test: The Two Substrates

Requested directly by review, checked against §6's own substrate-specific criteria rather than assumed identical to §5's mechanism test, then checked separately against §7.1's warrant test — eligibility, warrant, and admission are three different questions, and this section is where that distinction gets its first real test on existing, catalogued content. **No reclassification is made or implied** — both clear eligibility, but they do not both establish warrant, and neither claim here about warrant should be read as this page performing an admission (§7.2), which remains outside this page's own authority regardless of the result.

### `substrates/context-observer.md`

- **Eligibility:** 1–5 all pass. The `ContextObservation` schema (identity/usage/composition/activity/lifetime/relationships) is real and source-grounded (`deterministic-authority-projection-and-adaptive-organizational-topology.md` §5.3); explicit "does NOT own" list (lifecycle pressure, replacement applicability, topology/separation pressure, semantic-width interpretation, organizational judgment, any transition decision), governed by its own stated sentence, "metrics like semantic-width pressure, coupling, or projection reduction therefore do not belong in the observer itself"; substantial residue (the full six-category schema survives refusal intact); cross-consumer invariance — Context Lifecycle and Organizational Compilation are explicitly stated peers ("neither gates the other's applicability question"), each deriving materially different downstream evidence (`LifecycleEvidence` vs. `VantageSeparationEvidence`) from the identical observation contract; distinctness stated directly against Evidence Anchor ("same observation/normalization shape, unrelated subject matter... neither substrate is a generalization of the other").
- **Evidence kinds present:** `independent_domain_convergence` (Context Lifecycle, Organizational Compilation — two genuinely separately-owned dimensions, each with their own derivation layer). Notably weaker in one respect than Evidence Anchor's own sourcing: this substrate's multi-consumer case had to be assembled by the architecture-decomposition effort itself from two separate dimension pages, rather than being diagrammed as multi-consumer by a single source document — a calibration note, not a flaw.
- **Canonization warrant (§7.1):** satisfied by route A — two genuinely separately-owned dimensions, real convergence, not a parametric artifact.
- **Restated precisely, without inferring admission history from design status or capstone presence:** eligible (§6); warrant reconstructable (§7.1, route A); `design: proposed`, no design-acceptance event on record, per its own prose ("the idea document proposing this substrate has never received an explicit acceptance decision"); capstone-listed (`work-engine-planned-architecture.md` §6); taxonomy-admission provenance currently unreconstructable — `design: proposed` proves only that no design-acceptance event exists, not that no taxonomy-admission event occurred, and capstone listing proves only that a rendering exists (§7.2's own publication/rendering split), not that an authority-bearing admission occurred. Neither absence nor presence of either fact licenses a conclusion about taxonomy-admission history either way; corrected from an earlier draft of this page that wrongly credited this substrate with the mechanisms' own dated design-acceptance pattern.
- **Origin independence:** both consumers sit at "separate documents, same design campaign" — moderate evidence against manufactured convergence, no claim made here about the substrate's architectural generality beyond that (§8).
- **Falsifier:** if `ContextObservation`'s fields turned out to be populable only with foreknowledge of which consumer would derive from them — i.e., the schema silently encoding lifecycle- or topology-specific meaning — the substrate would fail semantic neutrality (§6.2) and collapse into whichever consumer it secretly serves.
- **Documentation gap:** none structural. One nuance checked and resolved, not a contradiction: this page's own "What This View Does Not Show" bullet says a generic "observer framework" abstraction "would only be justified if a second, independent substrate later converged on the same shape, which has not happened yet." `substrates/evidence-anchor.md` is now that second substrate, and its own text calls the two "the same shape." Read carefully, these are not in tension — Evidence Anchor's claim is that both independently satisfy the *substrate-kind* shape (observation/normalization only, no derivation, no admission), which is true and is exactly what §6 confirms; Context Observer's hedge is about a separate, still-unaddressed question — whether a *third*, more generic mechanism/base-abstraction unifying substrate-construction itself is warranted. That question remains open; it is not answered or contradicted by either substrate's own recognition.

### `substrates/evidence-anchor.md`

- **Eligibility:** 1, 2, 3, 5 pass cleanly. The `AnchorObservation` schema and exact-revision-binding invariant are real and source-grounded (`evidence-anchor-observation-and-impact-nomination.md` §3); explicit "does NOT own" list (dependency/anchor registry, semantic materiality, durable record publication, refresh-episode consequence, reopening downstream work), with its own corrected leak already documented ("'relevant' smuggles semantic materiality back into the observer"); substantial residue (the five-state comparator plus the bound/observed revision invariant survives refusal intact); distinctness stated directly against Context Observer. **Criterion 4 (cross-consumer invariance) passes only prospectively — one can describe an observation contract another domain could consume without importing Evidence/Claims semantics — but that is a claim about shape, not about whether such a consumer currently exists.**
- **The finding:** this page's own §4, "Multiple Independent Consumers, Already Named by the Source," presents architecture claim, plan claim, and research claim as its confirming multi-consumer evidence — but `evidence-and-claims.md` §1 ("What This Dimension Owns") states directly that this single dimension owns "claim identity, revision chains, provenance, lineage relationships... [answering] what proposition does this claim assert" generically, regardless of subject matter. Architecture/plan/research claims are domain profiles of *one* owning dimension's own generic claim schema, not three independently-owned consumers — exactly the `parametric_invariance` pattern §7 already names for Resource Lease's seven resource types, found here in a second page by applying the same discipline consistently. The substrate's only candidate for genuine cross-domain evidence is the fourth item named in the same source document's own Relationships table — the coordinate/service-state map (`service-plane-and-kernel-domain-boundary.md`, a genuinely different owning dimension) — which that source itself describes only as "not something this document builds a parallel mechanism for": a passing mention, with no citable acceptance decision, thinner even than Resource Lease's two dated, accepted consumers (§13).
- **Evidence kinds present:** `parametric_invariance` (architecture/plan/research claim, all domain profiles of the single Evidence/Claims dimension) — real, but excluded from §7.1's warrant test by definition. `proposed_cross_domain_consumer` (the coordinate/service-state map candidate) — named, but per §7.1, "a merely named, mentioned, or hypothetical candidate consumer... does not satisfy route C." No `independent_domain_convergence`, no `implemented_cross_domain_consumer`, and no `accepted_cross_domain_consumer` evidence currently exists.
- **Canonization warrant (§7.1): not currently satisfied.** None of routes A–D is met: no independent domain has converged on this shape (A); nothing is built beyond the substrate's own unimplemented core (B); the one candidate consumer has no citable acceptance decision, only a name (C, explicitly excluded); and no external architectural-owner decision has been cited invoking route D — this page does not invoke route D on its own initiative merely because routes A–C came up short, which is exactly what §7.1 forbids.
- **Restated precisely, without inferring admission history from design status or capstone presence:** eligible (§6); warrant not reconstructable under current §7.1 (parametric evidence and a bare mention, neither a qualifying route); `design: proposed`, no design-acceptance event on record; capstone-listed; taxonomy-admission provenance currently unreconstructable — the identical position as Context Observer on every axis except warrant. `design: proposed` does not prove no taxonomy-admission event happened, and capstone listing does not prove one did; this page has no way to check either, for this substrate or any other candidate (§7.2).
- **This is a corpus-conformance conflict, per §7.3, not a removal and not a grandfather.** `substrates/evidence-anchor.md` clears §6's shape test and remains catalogued in the capstone's "2 Shared Substrates" exactly as before — neither deleted nor reclassified here. It is recorded as standing in conflict with §7.1 until one of: a real consumer is built (route B), the candidate consumer is actually accepted by a citable, external decision (route C), an external architectural judgment explicitly invokes route D, or a real, authority-bearing admission decision under §7.2 — from whatever eventually answers `authority-backed-architecture-directions-as-workflow-inputs.md` item 1 — retires or folds the entry. None of those is this page's call to make, and none is the capstone's either; the capstone holds no admission authority of its own (§7.2), so "the capstone decides" is not a fifth option.
- **Status-grammar consequence (per §7.3 and `status-grammar.md` §4.2):** this finding does **not** falsify `substrates/evidence-anchor.md`'s own `reconciliation: reconciled` (`status_as_of: 2026-09-16`) — that marker is scope-relative and remains true to what was known and declared before this page's warrant rule existed. It instead **creates new reconciliation work, tracked as its own claim** — this §14 entry, read together with §7.3, is that separately stated corpus-conformance finding. `substrates/evidence-anchor.md`'s own status block should not be edited on the strength of this page alone; a future reconciliation pass on that page, citing this finding by name, is the correct vehicle for updating it.
- **Origin independence:** the parametric claim-type evidence sits entirely within one already-implemented dimension (Evidence/Claims); the one named candidate for cross-domain evidence has no ladder placement at all, since it has not been adopted in any form.
- **Falsifier — already triggered, not merely hypothetical:** §9's cross-consumer falsifier ("consumers require materially different semantics" — or, as directly relevant here, fail to be independently-owned consumers at all) is not a future risk for this substrate; applying it just now is what produced this section's finding. The forward-looking version: if the coordinate/service-state map candidate is never adopted and no other genuinely different-domain consumer emerges, this substrate's warrant evidence stays at zero indefinitely, not merely "thin."
- **Documentation gap exposed:** real, and structurally identical to Resource Lease's — except that Resource Lease's two consumers turned out to be citably accepted once correctly typed (§13), clearing §7.1 on route C, while Evidence Anchor's one candidate does not. This page's own §4 heading overstates cross-domain convergence by presenting one dimension's own domain-profile parameterizations as "multiple independent consumers." Flagged for that page's own maintenance, not corrected here.

Net for §14: both substrates clear eligibility cleanly, both are `design: proposed` with no design-acceptance event on record, both are capstone-listed, and for both, **taxonomy-admission provenance is currently unreconstructable** — `design: proposed` proves the absence of a design-acceptance event, not the absence of a taxonomy-admission event, and capstone presence proves a rendering exists, not that an authority-bearing admission occurred; this page has no way to check the latter for either substrate, or for anything else (§7.2, capstone §11 item 6). Once that shared, honestly-unresolved ground is stated plainly rather than inferred past, **the two substrates differ on exactly one axis: warrant.** Context Observer's is reconstructable — real, independent, cross-domain convergence, honestly moderate, with no overclaim. Evidence Anchor's is not — its "Multiple Independent Consumers" claim turns out, on inspection, to be one dimension's own parametric domain profiles plus a bare, unaccepted mention, neither of which clears §7.1's routes today. That single difference, not any claim about what history did or didn't happen, is what §7.3 records as a corpus-conformance conflict for Evidence Anchor and not for Context Observer — the same parametric/cross-route conflation Resource Lease had, but where Resource Lease's correction revealed stronger real evidence underneath (two dated, accepted consumers), Evidence Anchor's correction reveals weaker evidence than its own text claims. One consistent test, applied without inferring more than the evidence supports in either direction, produced two different verdicts on warrant — which is the result this page exists to be capable of producing.

---

## 15. Related Architecture Views

- **`work-engine-planned-architecture.md`** §1 — names this page and links here rather than restating the criterion; §5 and §11 cite this page's pressure-test results directly.
- **`status-grammar.md`** §5, §7, §8, §10.6 — §5's and §8's named-external-owner and declarative-not-inferred discipline is what §7.1's route D and §7.2's admission-proxy standard both borrow directly; §7's authorization-composition corollary and its worked example are what this page's own §12 depends on; §4.2 (scope-relative `reconciled`) is what §7.3 depends on for Evidence Anchor's own status treatment. Recognition (this page), warrant, admission, and authorization/status (that page) remain four independent things.
- **`app-server/ideas/pending/authority-backed-architecture-directions-as-workflow-inputs.md`** §18 item 1 — the open residue ("Who owns architecture decisions, and how is that authority represented and verified?") that §7.2's admission concept explicitly consumes rather than answers. §7.2 states that its own eventual resolution should trigger a separate reconciliation pass — but only for the eleven canonical views (five mechanisms, four dimensions) actually checked and found to carry a dated design-acceptance event, never generalized to all 20 without checking, and even for those eleven the event on record is admission-shaped *evidence*, not a verified taxonomy-admission transition — no such transition is currently defined, checkable, or owned by anything (capstone §11 item 6).
- **`mechanisms/revision-cas-and-publication.md`, `mechanisms/candidate-resolution-and-admission.md`, `mechanisms/transition-fencing-and-leases.md`, `mechanisms/resource-lease-and-fencing.md`, `mechanisms/authority-preserving-intent-projection.md`** — the five pressure-tested in §13; none reclassified. `revision-cas-and-publication.md` carries a second role here beyond being pressure-tested: §7.2 names its own generalized discipline as the correct vehicle for authoritative taxonomy publication, in principle, distinct from the capstone's own rendering of it.
- **`substrates/context-observer.md`, `substrates/evidence-anchor.md`** — the two pressure-tested in §14; no reclassification made. Evidence Anchor clears eligibility (§6) but does not establish canonization warrant (§7.1), which is itself a standing corpus-conformance conflict (§7.3) — recorded, not resolved here, and its own `reconciliation: reconciled` status is unaffected per `status-grammar.md` §4.2.

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

`design: accepted` — this page's own recognition criterion, including nine same-day revisions (the kind classifier and three kind-specific eligibility sections; the fact-plane/protocol-plane classifier fix and the first canonical admission rule; that rule's kind-branching, Route D's authority-source requirement, and the corpus-conformance-conflict rule; the eligibility/warrant/admission/publication split that replaced "admission rule" with "warrant" and introduced §7.2's authority-bearing admission concept; separating authoritative publication from capstone rendering; identifying that no existing owner holds taxonomy-membership truth and mechanically auditing, rather than assuming, the historical-provenance claim across all 20 views; removing every remaining inference of taxonomy-admission history from design status or capstone presence alone; separating decision authority from membership-state ownership with positive evidence; then testing whether an authoritative membership state exists at all, distinct from whether it is owned), was explicitly settled through direct discussion (2026-09-18), the same bar every mechanism and dimension recognition in this architecture is held to. `reconciliation: reconciled` — pressure-tested directly against all five current mechanism pages, both substrate pages, all thirteen dimension pages' own Source and Status blocks, `status-grammar.md`, `authority-backed-architecture-directions-as-workflow-inputs.md`, and `work-engine-planned-architecture.md` §1/§5/§6/§11, this session; real documentation gaps, one real, standing corpus-conformance conflict, one factual correction (Context Observer's own design status), and one new corpus-wide residue item (capstone §11 item 6) were found and are recorded in §13–§14, §7.2, and the capstone rather than corrected silently. `authorization: design_work_authorized` — this page is a documentation and taxonomy convention, not a buildable artifact; nothing here authorizes reclassifying or rebuilding anything it pressure-tests, holds no admission or publication authority under its own §7.2, and §14's Evidence Anchor finding is explicitly left unresolved pending whatever authority `authority-backed-architecture-directions-as-workflow-inputs.md` item 1 eventually names. `implementation: none` — not applicable to a taxonomy page. `owner`: self, following the corrected convention `mechanisms/revision-cas-and-publication.md` and `mechanisms/candidate-resolution-and-admission.md` already use, and the correction §13 recommends for `mechanisms/authority-preserving-intent-projection.md`.

**Revised 2026-09-18 (first pass)** from the same-day original, per direct review: split the single, mechanism-shaped eligibility test into a kind classifier (§3) plus three kind-specific eligibility sections (§4–§6), rather than declaring substrates "structurally eligible for the identical test" without checking; made explicit that recognition evidence (§7) never assigns `status-grammar.md`'s own status values; narrowed the origin-independence ladder (§8) to claim only non-manufactured convergence, not architectural generality; separated `parametric_invariance` from cross-route/cross-consumer invariance and corrected Resource Lease's §13 entry accordingly; added the substrate pressure test (§14).

**Revised 2026-09-18 (second pass)**, per direct review of the first pass's own results: (1) sharpened the kind classifier (§3) from "supplies facts, semantically neutral" — a property mechanisms share too (Candidate Resolution's 0/1/N, Revision/CAS's head/staleness, and Transition Fencing's fence validity are all neutral facts) — to the fact-plane-vs-protocol/control-plane discriminator actually doing the work: a substrate's authoritative output never gates admission, mutation, publication, or transition; a mechanism's does; (2) reworded the Dimension eligibility criterion (§4.3) from "commitment or belief" to "authoritative meaning that cannot be reconstructed from a neutral observation contract alone," so descriptive dimensions like Evidence/Claims are caught without sounding like policy; (3) added §7.1, the Canonical Admission Rule, closing the gap the first pass's own eligibility/evidence split had opened but not finished: eligibility is necessary, but canonical recognition additionally requires at least one non-parametric route (independent convergence, an implemented consumer, an *accepted* consumer, or an explicit owner decision) — a merely named or mentioned candidate does not qualify; (4) split `proposed_cross_domain_consumer` into that weaker sense and a new `accepted_cross_domain_consumer` for a citable, dated acceptance decision, and retagged Resource Lease's two consumers and APIP's Studio instance accordingly, since both were, on inspection, actually accepted rather than merely proposed; (5) applied §7.1 to §14 without protecting existing catalog status, and found that `substrates/evidence-anchor.md` clears §6's eligibility test but does not currently satisfy the admission rule under any of routes A–D — its one candidate for cross-domain evidence is a passing mention, not an acceptance decision. That finding is recorded in §14 and left for the page's and capstone's own owner to resolve, not acted on here.

**Revised 2026-09-18 (third pass)**, per a direct re-audit of §7.1 itself: (1) §7.1 now branches by kind rather than applying one cross-owner-need test uniformly — a dimension is the sole owner of its own truth class, not a shared protocol other owners converge on needing, so requiring independent convergence or a cross-domain consumer from a dimension candidate would import a criterion §4's own eligibility test already excludes; a lighter, non-duplication-and-substance-based dimension admission branch is stated instead, not pressure-tested against the 13 dimensions in this revision; (2) route D now names its own authority-source requirement explicitly — external to the candidate's own page, citable to a distinct document or discussion, exactly as `status-grammar.md` §5 and §8 already require for `design: accepted` — closing the loophole where a candidate's own text could assert its own importance and call that route D; (3) added a corpus-conformance-conflict rule, naming the standing state a catalogued item is in when it clears eligibility but fails the admission rule: refusing both silent removal and silent grandfathering, resolved only by new evidence, an external route-D decision, or an owner's decision to retire or fold the entry; (4) applied §4.2 of `status-grammar.md` directly to Evidence Anchor's own status: its `reconciliation: reconciled` (`status_as_of: 2026-09-16`) is not retroactively falsified by this page's later rule — it remains scope-relative and true to what was known on that date — and what §4.2 actually requires is the separately stated corpus-conformance finding, not an edit to that page's own status block.

**Revised 2026-09-18 (fourth pass)**, per a direct re-audit against the corpus's own canonical observation → evidence → judgment → admitted-transition → publication flow, which exposed that the third pass's own "canonical admission rule" was itself conflating two different things under one name: (1) §7 now opens by naming five distinct stages explicitly — kind eligibility (§4–§6), evidence (§7), canonization warrant (§7.1), authority-bearing admission (§7.2, new), and publication (the capstone's own tables) — and states directly that collapsing warrant into admission is the same error `status-grammar.md` §8 already names for design status ("evidence supporting acceptance is not acceptance... [it] cannot perform" the acceptance itself); (2) §7.1 is renamed from "The Canonical Admission Rule" to "Canonization Warrant" throughout, and every route (A–D) is restated as producing warrant, never admission — routes A–C were already evidence, not authority, and did not need to change; route D previously bundled "an owner judged this warranted" with "and is therefore admitted," now split so the external judgment establishes warrant only; the Dimension branch's "substantial homeless content" is given the identical caveat — warrant for a new truth owner, never itself the act of establishing one; (3) new §7.2 states what admission actually requires and, critically, does **not** invent the missing authority to perform it — it names the exact, already-open corpus residue that question belongs to (`authority-backed-architecture-directions-as-workflow-inputs.md` §18 item 1, `[KIND: RESIDUE] [OPEN]`: "Who owns architecture decisions, and how is that authority represented and verified?") and states that this page consumes that eventual authority rather than defining it; until it resolves, the honest proxy is the same dated, external, citable-decision standard `status-grammar.md` already uses for `design: accepted` — which is what the five mechanisms and Context Observer actually have on record, and what Evidence Anchor does not; (4) §7.2 also states plainly, citing the capstone's own Authority line verbatim ("does not accept any idea... does not resolve anything the views themselves leave open"), that the capstone holds no admission authority and is a publication surface only — fixing prior wording in both this page's own §7.3 (formerly §7.2) and the capstone's own §6 pointer that said resolution "belongs to this catalog's own owner," which would have silently granted the capstone exactly the authority its own Authority line disclaims; (5) the former §7.2 (Corpus-Conformance Conflicts) is renumbered §7.3 and reworded to remove every trace of capstone-as-authority framing, restating resolution as requiring an authority-bearing admission act under §7.2 specifically, sourced externally, never performed by this page or the capstone.

**Revised 2026-09-18 (fifth pass)**, per a direct pressure test of the fourth pass's own pipeline one stage further downstream, which found that "publication" had itself been quietly equated with "the capstone's tables" — the identical warrant/admission conflation the fourth pass had just fixed, recurring one stage later: (1) the §7 pipeline diagram and §7.2's own text now insert an explicit stage between admission and capstone rendering — authoritative successor publication, named as `mechanisms/revision-cas-and-publication.md`'s own generalized CAS-published-succession discipline applied, in principle, to the canonical taxonomy as durable authoritative state; §7.2 states plainly that no such revisioned taxonomy artifact currently exists, that the capstone's file is instead edited directly today with no CAS check, predecessor chain, or atomicity guarantee, and that this is a real, named gap between principle and practice, not something this page builds; the direct consequence, stated explicitly: editing the capstone's own tables must never be treated as the act that makes an admission authoritative, and the capstone's own Authority line already agrees from its own side; (2) §7.2 gains a new paragraph tracing the still-open architecture-decision-authority residue backward over the *entire* 13/5/2 decomposition, not only the seven items §13–§14 check — **claimed at the time as "every one of the 20 canonical views shares the identical dated-decision-event provenance," a claim the sixth pass below found was never actually checked and turned out to be false**; (3) §14's own closing paragraph is corrected — Evidence Anchor's admission plainly *did* happen historically (it is already catalogued), and the precise finding is that its historical catalog membership currently lacks *reconstructable* warrant under §7.1's now-stated criterion, not that admission "never arose"; the capstone's own §6 pointer and §15's Related Views are updated to match both corrections.

**Revised 2026-09-18 (sixth pass)**, per a direct pressure test of the fifth pass's own two remaining claims — one an unverified generalization, one an unexamined assumption about who owns the state admission modifies:

1. **The "all 20 views share identical provenance" claim was checked mechanically rather than retained on the strength of the pattern seen in seven items, and it does not hold.** Every one of the 13 dimension pages' own `Source and Status` blocks was read directly: only 6 of 13 are `design: accepted` at all (`role-and-contract-structure.md`, `evidence-and-claims.md`, `review.md`, `proposal-evaluation.md`, `decision-specific-readiness.md`, `portfolio-selection.md`); the other 7 remain `design: proposed`, with no acceptance decision on record. Of the 6 accepted dimensions, 4 cite a dated "Accepted 2026-09-14" reconciliation decision (the mechanisms' own pattern, one day earlier); 2 (`role-and-contract-structure.md`, `evidence-and-claims.md`) are accepted on the strength of pre-existing, already-implemented reality rather than any single decision event — a third, different provenance shape this section's proxy standard does not describe at all. This check also caught a real factual error: **`substrates/context-observer.md` was wrongly credited, in the fourth and fifth passes' own text, with the mechanisms' dated admission-proxy pattern — its actual status is `design: proposed`, with no acceptance decision, identical in this one respect to Evidence Anchor.** §7.2's historical-provenance paragraph, §14's Context Observer entry, and the "Net for §14" paragraph are all corrected; the claim now applies cleanly only to the eleven views (five mechanisms, four dimensions) that actually carry a dated event, stated as such rather than generalized to all twenty.

2. **The unsupported phrase "made in good faith" is removed from the historical-provenance paragraph** — this page has no evidence about any actor's state of mind and should not have asserted one. The defensible claim, restated: a later-arriving authority model does not by itself prove an earlier admission unauthorized; it creates a provenance-reconciliation obligation once that model exists.

3. **Pressure-tested the semantic owner of "X is currently an admitted Dimension/Mechanism/Substrate" itself, checking rather than inferring from who decides, publishes, or renders.** None of the four things this page has carefully kept separate turns out to own it: this page owns the eligibility/warrant criterion, not the membership state a candidate is tested against; the open architecture-decision-authority residue (item 1) concerns who may decide, not what is currently true; `mechanisms/revision-cas-and-publication.md` supplies publication discipline without owning what a revision means, confirmed directly by `authority-and-ownership.md`'s own text calling it "not a truth dimension"; and the capstone is a projection, rendering a truth it does not own by its own stated Authority line. No dimension among the 13 covers it either. **No existing owner was found — recorded as a new item, capstone §11 item 6, rather than silently absorbed by this page, the residue document, the mechanism, or the capstone**, exactly the discipline this page has applied to every other finding.

**Revised 2026-09-18 (seventh pass)**, per a direct re-audit of the sixth pass's own two findings, which caught them still smuggling admission history in by the back door — through the very two facts (`design: proposed`, capstone presence) the sixth pass itself had just shown were owned by nothing and verified by nothing:

1. **`design: proposed` proves the absence of a design-acceptance event; it does not prove the absence of a taxonomy-admission event — and the sixth pass's own §14 and §7.2 text had quietly treated it as if it did.** Symmetrically, capstone presence proves a rendering/edit exists, never that an authority-bearing admission occurred — exactly what the fifth pass's own publication/rendering split already established, but §14's closing paragraph ("Evidence Anchor's admission plainly did arise historically — it is already catalogued... which does not happen without some past admission-shaped event") had silently un-established it again for one specific case. Corrected throughout: §7.2's historical-provenance paragraph, §13's own intro, both §14 substrate entries, the "Net for §14" paragraph, §15, and the capstone's own §6 pointer now state, precisely and without exception, that taxonomy-admission provenance is currently unreconstructable for every candidate this page has ever pressure-tested — Context Observer and Evidence Anchor now read: eligible; warrant reconstructable / not reconstructable respectively; `design: proposed`, no design-acceptance event; capstone-listed; taxonomy-admission provenance unreconstructable — identical on every axis except warrant, which is the only real difference §7.3's corpus-conformance conflict actually tracks.

2. **The same discipline is applied backward to the five mechanisms, which had been silently exempted.** Their dated "explicitly settled through direct discussion" records are admission-shaped *evidence* — real, citable, external design-acceptance decisions a reasonable admission process would plausibly point to — never verified taxonomy-admission events, because no such event is currently defined, checkable, or owned by anything (capstone §11 item 6). §7.2's proxy paragraph and §13's own intro no longer call this "the admission proxy" without that qualification; every admission claim on this page, for every candidate without exception, is now stated at proxy strength, not verified strength.

3. **Capstone §11 item 6 is pressure-tested against one further null before treating it as a new truth class: can membership be deterministically reconstructed from already-owned authoritative state without semantic judgment?** Checked, not assumed — no. It is not reducible to the `status-grammar.md` axes each candidate's own page already owns: `substrates/context-observer.md` is `design: proposed`, yet is already listed as a current substrate, so "design: accepted" is demonstrably not the rule real practice follows. No other already-owned field computes it either — the actual test, this page's own eligibility and warrant criteria, requires reading and judging each candidate's own prose every time, a semantic-judgment act, not a lookup. Item 6 is retained as genuine unowned truth on that basis, and the capstone's own ledger entry is corrected to remove its prior implication of "most plausibly a fourteenth dimension" — that placement is itself the owner decision item 6 defers, not a conclusion this pass is authorized to reach.

**Revised 2026-09-18 (eighth pass)**, a narrow re-audit of capstone §11 item 6 alone, per a direct challenge that its reasoning had conflated two different things: the capstone's lack of *decision authority* (proven directly by its own Authority line) and its lack of *membership-state ownership* (asserted, but never actually tested against positive evidence — a dimension can own durable state it did not decide, `evidence-and-claims.md` being the standing example). Four things are now stated explicitly as distinct in item 6 itself: decision authority, membership-state ownership, the publication mechanism, and projection. Tested directly against the capstone's own declared responsibilities, not by rerunning §7.1's eligibility/warrant (a pre-admission judgment, not the read-vs.-reconstruct question at stake here): does the capstone already own the current membership inventory while decision authority sits elsewhere, or is it only a projection of a state whose owner is absent? Chosen with positive evidence, not merely the Authority line's silence: the capstone's own Status section states its job as "tell a reader where truth lives... not restate everything the views already say"; its own governing writing rule forbids it from originating any substantive statement, including a membership claim, without deriving it from a canonical view or flagging it as residue — exactly what §5's and §6's own tables do, citing each candidate's page and this recognition page's own pressure test rather than asserting membership on the capstone's own standing; and it is periodically **rebuilt** from the 20 views rather than incrementally maintained as a directly-edited register. That is option C, on real textual and structural evidence — not option A, and not option B, which the prior pass already ruled out and this one does not re-litigate. Item 6 survives, now for the reason the null hypothesis actually supports. The corresponding claim in §7.2 ("editing the capstone's own tables... must never be treated as the operation that makes a candidate's admission authoritative") is restated as conditional on this finding rather than as a standalone consequence of the Authority line: it holds only because and for as long as C holds, and would not hold if a future owner decision instead made the capstone itself the authoritative membership register.

**Revised 2026-09-18 (ninth pass)**, a further narrow re-audit of capstone §11 item 6 alone, per a direct challenge that the eighth pass's own "option C" wording — "a membership state whose owner is genuinely absent" — asserted a proposition it had never actually tested: that an authoritative membership state exists at all, merely lacking an owner. Four distinct propositions are separated: (1) the capstone does not own taxonomy membership; (2) no other existing dimension/mechanism/substrate owns it; (3) an authoritative taxonomy-membership state currently exists; (4) current membership can be verified from authoritative admission/publication history. The eighth pass established (1) and (2) soundly and left (3) asserted rather than tested; the seventh pass had already established (4) is false (proxy-strength only). Testing (3) directly: no artifact anywhere in the corpus records a discrete admission-and-publication event for any candidate — what "was admitted, by whom, under what authority, from what predecessor" — and `mechanisms/revision-cas-and-publication.md`'s own discipline (content-digest identity, predecessor chain, computed head) has never been instantiated for "the canonical taxonomy" as an artifact. **No positive evidence supports (3); the corpus contains a real, reconciled, proxy-strength catalog, not a rendering of instantiated authoritative state.** This also corrects the deterministic-reconstruction null run in the sixth/eighth passes: that test showed *qualification* (eligibility/warrant) is not mechanically derivable from already-owned state, and concluded membership is therefore "genuine unowned truth" — but qualification is a pre-admission judgment, not membership itself, and the finding is better read as a symptom of having no authoritative revision to *read* current membership from at all (`nothing owned may be reconstructed instead of read`, violated here because nothing has been instantiated, not because something instantiated lacks an owner). Item 6 is reframed accordingly: authoritative taxonomy membership has not yet been instantiated as durable state; the existing catalog is real and reconciled but proxy-strength; finding an owner (item 5's escape valve) is necessary but not sufficient, since an owner cannot own state that does not yet exist. Resolution is restated as a one-time bootstrap — a settled authority model (item 1) plus the corpus's own historical decision evidence, taken through explicit reconciliation/bootstrap admission decisions to instantiate a first "taxonomy revision 0," after which ordinary Revision/CAS publication with a real assigned owner can take over — rather than a single owner-naming decision. The dependent §7.2 sentence ("the canonical taxonomy... is durable, authoritative state of exactly this kind") is corrected to "taxonomy membership, once authoritatively instantiated, would be durable state of exactly this kind," and the "editing the capstone can never make an admission authoritative" claim is restated as resting on the absence of any authoritative state to change, not merely the absence of an owner for one that exists. No changes were made to the 13/5/2 decomposition, warrant criteria, substrate findings, or the eighth pass's own decision-authority/ownership distinction, which stands.
