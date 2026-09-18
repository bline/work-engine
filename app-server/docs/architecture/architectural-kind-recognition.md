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
    Existing mechanisms are retained because they also satisfy it — see §10,
    the pressure test that checks this directly rather than assuming it.
```

`app-server/docs/work-engine-planned-architecture.md` §1 states this page exists and links here; it does not restate the criterion. Any future capstone edit that finds itself re-deriving eligibility or evidence rules belongs here instead.

This page does not reopen the 13-dimension / 5-mechanism / 2-substrate decomposition, and does not reclassify anything merely because it now exists. §10 pressure-tests the current five mechanisms against the criterion below and reports what it finds — including real documentation gaps — without repairing them here.

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

The fourth kind is not a residual "everything else" bucket — it is the default. A candidate pattern starts here and is promoted only by clearing §3 and §4 below. Most real, concrete artifacts in Work Engine correctly stay in this kind permanently (capstone §9's own domain instantiations: `CodeEvidenceAdapter`, `UIReviewProfile`, `control-plane-causal-observability-ui.md`) — promotion is the exception, not the default trajectory of a useful pattern.

---

## 3. Mechanism Eligibility — Necessary Conditions

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

4. ROUTE INVARIANCE
   Materially different lawful consumers or realizations can use the
   candidate without changing what it means. The same mechanism answers
   the same question regardless of which domain is asking it.

5. DISTINCTNESS
   The candidate cannot collapse into an existing dimension, mechanism,
   substrate, or domain detail without moving semantic ownership or
   destroying an invariant that currently holds. Shared vocabulary between
   two candidates is not, by itself, evidence against distinctness (§7).
```

**The refusal test (criterion 3) is necessary, not sufficient, and must never be read as the whole test.** A "What This Mechanism/View Does Not Decide" section that leaves substantial residue proves the candidate *can* be an authority-neutral abstraction. It does not prove Work Engine *benefits* from canonizing it as one — a beautifully bounded abstraction with a single consumer can still be a domain-local helper that will never be reused. Eligibility (this section) establishes that a candidate is the right *shape*; §4 establishes that canonizing it is actually *warranted*. Both are required; neither substitutes for the other.

---

## 4. Evidence Toward Canonical Admission — Confidence, Not Eligibility

A candidate that clears §3 is eligible. Whether it should actually be admitted to canonical `accepted`/`reconciled` status is a separate, evidentiary question, and **instance counts are supporting bookkeeping, never the criterion itself.** `mechanisms/revision-cas-and-publication.md`'s own status history is the direct proof: its confirmed-instance count moved from four to seven on 2026-09-16 ("found by the final mechanical audit to be undercounted here as 'four' — corrected") with zero change to `design: accepted` — the mechanism was never *made* real by reaching seven; the correction only repaired the record of evidence that was already there.

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

proposed_cross_domain_consumer
    A materially distinct domain has accepted the candidate as its own
    instance but has not yet built it.
```

These are not mutually exclusive, and a single mechanism typically carries more than one at once — see §10 for how each of the current five actually breaks down. Three things must be stated explicitly, because the existing corpus has blurred exactly these distinctions under one undifferentiated "Confirmed Instances" heading (§10 names where):

- **Resource types parameterizing one generic implementation are not independent domain convergence.** `mechanisms/resource-lease-and-fencing.md`'s seven `RESOURCE_TYPES` (`directory, git-ref, git-index, port, index, review-budget, database`, verified against `contract.mjs:4-6`) are seven parameterizations of one already-generic `workspace-coordination` service, not seven independently-arrived-at domains that separately discovered the need. That is real, legitimate evidence — `preexisting_generic_implementation` — but it is a different evidentiary claim than convergence, and conflating the two overstates how many independent reasoning processes actually endorsed the shape.

- **"Zero cross-reference" is evidence of textual independence, not necessarily independent origin.** `mechanisms/authority-preserving-intent-projection.md` states its Runtime Realization instance "was written with **zero cross-reference to Studio or its reconciliation**, yet follows the identical shape" — real evidence that one author didn't copy the other's prose. It is not evidence that the two reasoning processes were separated by anything more than authorship, since both are products of the same 2026-09-16 architectural-decomposition campaign. See §5's origin-independence ladder for why this distinction has to be kept explicit rather than collapsed into a single "independent" label.

- **Instance counts are supporting evidence, never the recognition criterion itself** (restated from the opening of this section because it is the single most load-bearing sentence on this page).

---

## 5. Origin Independence — An Ordinal Ladder, Not a Score

Distinct from *how many* consumers exist (§4) is *how independently* each one arose. This is not a scalar to compute and compare numerically — it is an ordinal ranking used to judge whether a given piece of convergence evidence is as strong as it is being treated:

```text
same claim / same document
    < separate documents, same design campaign
    < independently developed subsystem or domain
    < implementation predating architectural synthesis
```

An instance at the top of this ladder (a real, running system built for its own reasons before anyone was looking for a shared mechanism — `workspace-coordination` predating the reconciliation-queue process entirely, or `semantic-context-lifecycle-manager.md`'s own implemented transition-lease sequence) is categorically stronger evidence than two documents drafted days apart in the same synthesis effort, even when both are honestly reported as "independent" in the narrower textual sense. §10 states, per mechanism, where its actual instances sit on this ladder rather than treating all "confirmed instances" as equivalent.

---

## 6. The Falsifier / Demotion Rule

A recognized mechanism (or substrate) is not permanently settled by having once cleared §3–§4. It should be reconsidered — not silently, through an explicit re-check against this page — if later evidence shows any of:

```text
- the supposed invariant turns out to be domain-specific after all
  (§3.1 fails on closer inspection);
- the refusal boundary leaves only a thin vocabulary or schema once
  restated precisely (§3.3 fails);
- its consumers require materially different semantics from one another,
  not merely different implementation shape (§3.4 fails);
- an existing canonical owner already owns the normalized truth being
  claimed (§3.5 fails).
```

Each recognized mechanism should be able to name what observation would demote it. §10 states one per mechanism, drawn from that mechanism's own weakest currently-cited evidence rather than invented generically.

---

## 7. Sibling-versus-Merge Test

Two candidate mechanisms sharing vocabulary, implementation substrate, or superficial shape stay **separate** only when one of these holds:

```text
- they preserve materially different invariants; or
- a rule one must enforce is a rule the other must NOT hold.
```

Differences only in lifetime, naming, calling convention, storage engine, or implementation shape are **insufficient** to justify a merge, and are equally insufficient, by themselves, to justify keeping two pages separate — the test cuts both ways. §8 applies this directly and checks the source text rather than deciding from analogy.

---

## 8. Worked Pressure Test: Resource Lease vs. Transition Fencing

Both mechanism pages already carry a direct, explicit "Relationship to [sibling]" section rather than assuming compatibility from shared fencing-token vocabulary — that much reconciliation work is already done. What was not yet stated as a sharp, enforceable rule is the §7 test itself: a concrete "X must, Y must not" pair.

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

This strengthens, rather than manufactures, the existing split: §7's test is satisfied (a rule Resource Lease must enforce — repeat exercise under one currency is lawful — is a rule Transition Fencing's own stated shape never permits), so the two pages should remain separate mechanisms. Because neither page states the reciprocal as its own explicit "must / must not" sentence, that is a real, minor strengthening opportunity for both pages' own text — noted here, not applied there; editing those two pages is outside this page's own scope.

The **third-instance question already flagged inside `transition-fencing-and-leases.md` itself** ("a third, real instance protects a different kind of thing than either named class... left open rather than decided by this edit," referring to the executable-generation reload) is a distinct, open question about that mechanism's own *internal* fence-class taxonomy — not a question about whether it should merge with Resource Lease. The page's own text already resolves the cross-mechanism question directly ("tested against both mechanisms directly and belongs to the other one, not this one"). §10 records the internal taxonomy question as still open, exactly as the source page states it, without resolving it here.

---

## 9. Authorization Is Not Inherited Through Composition

Recognition (this page) and authorization (`status-grammar.md`) are independent axes, and composing a candidate mechanism with its consumers must not blur that independence in either direction. `status-grammar.md` §7 already states the general rule ("implementation-authorized input → does not grant implementation authorization to what consumes it"); this page's own worked example (§8, `mechanisms/resource-lease-and-fencing.md`'s status block) is the sharpest concrete case: the generic mechanism itself carries `authorization: unrecorded`, while two of its consumer instances each carry an independent `status_override` of `authorization: implementation_authorized`, sourced to their own distinct reconciliation documents — neither direction inherited. `status-grammar.md` §7 has been given an explicit corollary and a sixth worked example (§10.6 there) recording this case by name, so the rule is stated where authorization itself is defined rather than duplicated here.

---

## 10. Pressure Test: The Current Five Mechanisms

Checked against §3–§6 directly. **No reclassification is made or implied by this section** — all five clear eligibility. What follows is the evidentiary and documentation picture the criterion exposes, including real gaps, reported rather than repaired.

### `mechanisms/revision-cas-and-publication.md`

- **Eligibility:** 1–5 all pass. Normalized shape (identity/predecessor/head/CAS/atomic-publish/stale-rejection) verified domain-blind; explicit "Does Not Decide" list; substantial residue (the six-item reusable shape); route invariance across five materially different owning dimensions; distinctness stated directly against both Candidate Resolution ("different questions, frequently composed") and Transition Fencing ("built directly on top of that one and does not restate its predecessor/head semantics").
- **Evidence kinds present:** `independent_domain_convergence` (Evidence/Claims, Context Lifecycle, Organizational Compilation, Semantic Planning, Material Decision Selection, Runtime Realization — six dimensions, each citing its own source text); `implemented_cross_domain_consumer` (Evidence/Claims, Context Lifecycle, and Runtime Realization's nested executable-generation substrate, all verified against real code); `proposed_cross_domain_consumer` (Organizational Compilation, Semantic Planning, Material Decision Selection, Runtime Realization's own top-level `RoleRealization` lineage).
- **Origin independence:** mixed. Evidence/Claims and Context Lifecycle's *code* sits at "implementation predating architectural synthesis" (real, running services checked directly); the *naming of the shared mechanism* across all seven cited instances sits at "separate documents, same design campaign" (all reconciled within the 2026-09-15/16 decomposition window).
- **Falsifier:** an instance whose "publish" can silently merge, reorder, or last-write-win against a stale head is not this mechanism's shape regardless of how it is labeled.
- **Documentation gap exposed:** the "post-execution implementation acceptance" instance (`completion-publication.mjs`) composes three already-recognized dimensions' outputs rather than belonging to any one dimension — a genuine fifth evidence shape (a *composite* instance) that §4's four typed kinds do not name. Flagged as an open extension to this page's own taxonomy, not resolved here.

### `mechanisms/candidate-resolution-and-admission.md`

- **Eligibility:** 1–5 all pass. Domain-neutral `AVAILABLE ∩ AUTHORIZED ∩ SATISFIES(REQUIRED) → {0/1/N}` shape; explicit "Does Not Decide" list; substantial residue (the zero/one/many distinction itself, stated as "the central fact this mechanism exists to establish"); route invariance across three materially different candidate universes; distinctness stated directly against both Revision/CAS and `mechanisms/authority-preserving-intent-projection.md` ("Neither subsumes the other").
- **Evidence kinds present:** `independent_domain_convergence` (Organizational Compilation, Runtime Realization, Material Decision Selection — three dimensions; a fourth, Portfolio Selection, is explicitly self-labeled by its own source as "likely, not yet formally confirmed," i.e. `proposed_cross_domain_consumer`, correctly hedged already). No `implemented_cross_domain_consumer` evidence at all — `implementation: none` across every confirmed instance.
- **Origin independence:** all three confirmed instances sit at "separate documents, same design campaign" (all reconciled or falsifier-tested within the 2026-09-15/16 window); none reaches "independently developed subsystem" or "predates synthesis," since none has running code.
- **Falsifier:** a claimed instance whose residual N-case judgment cannot actually be reduced to a domain-supplied decision (e.g., needing the mechanism itself to supply a domain predicate, or producing a fourth outcome class) does not confirm this mechanism.
- **Documentation gap exposed:** none structural. Worth stating plainly as an evidentiary-confidence note (not a failure): this mechanism's entire canonical status currently rests on same-campaign, zero-implementation convergence — a materially thinner evidentiary base than Revision/CAS's, even though both pass eligibility identically. The criterion in §3 does not distinguish them; only §4's typed evidence does, which is exactly the separation this page exists to make visible.

### `mechanisms/transition-fencing-and-leases.md`

- **Eligibility:** 1, 2, 3, 5 pass cleanly. Criterion 4 (route invariance) is **partially open, by the page's own admission**: the shared revision-binding discipline (bind → prepare → revalidate → publish → activate → release) is confirmed invariant across all three consumers, but whether "two named fence classes plus a third, unclassified instance" is a settled taxonomy or an incomplete one is explicitly unresolved in the source page itself ("left open rather than decided by this edit"). This does not fail eligibility — the base invariant clears criterion 4 — but the internal fence-class taxonomy built on top of it is not yet a closed question.
- **Evidence kinds present:** `implemented_cross_domain_consumer` (Context Lifecycle's real transition-lease sequence; Runtime Realization's real executable-generation reload, verified directly against `executable-generation-manager.mjs`/`executable-generation-store.mjs`); `proposed_cross_domain_consumer` (Organizational Compilation's topology-transition fence, named but no compiler exists).
- **Origin independence:** Context Lifecycle's and the executable-generation reload's implementations both sit at "implementation predating architectural synthesis" — both are real, pre-existing code the mechanism's own naming discovered rather than invented (the executable-generation instance explicitly "found 2026-09-16" as a symptom, not authored to fit). Organizational Compilation's topology-transition fence sits at "separate documents, same design campaign," the weakest of the three.
- **Falsifier:** an instance that turns out to confer standing, renewable exercise authority (violating §8's reciprocal rule) is a Resource Lease instance mis-filed here, not a third fence class.
- **Documentation gap:** the internal two-vs-three-fence-class question is real but already correctly hedged in both the source page and the capstone's own §5 table ("a third real instance protecting neither, never collapsed into one lock") — no overstatement found, no new gap to report; recorded here as an open item this page inherits rather than resolves.

### `mechanisms/resource-lease-and-fencing.md`

- **Eligibility:** 1–5 all pass, including route invariance — the seven resource-type parameterizations are themselves valid, confirmed route-invariance evidence (many materially different resource kinds pass through the identical acquire/`admitMutation` shape without changing its meaning). Route invariance being satisfied is a separate fact from which §4 evidence kind that satisfaction counts as (below).
- **Evidence kinds present:** `preexisting_generic_implementation` is this mechanism's actual, primary basis for recognition — `workspace-coordination`'s seven real resource types, verified against `contract.mjs`, predating the reconciliation-queue process entirely. This is legitimate, strong evidence in its own right (§4 names it as a first-class route, not a lesser one), but it is **not** `independent_domain_convergence`, and the source page's own "Confirmed Instances" heading currently lists it under the same undifferentiated label as its two genuinely cross-domain (but unbuilt) consumers. `proposed_cross_domain_consumer` ×2 (fenced active-binding, review-scope protection — both "accepted for implementation, not yet built"). No `implemented_cross_domain_consumer` evidence exists yet beyond the generic mechanism itself.
- **Origin independence:** `workspace-coordination` sits at "implementation predating architectural synthesis" (independently named elsewhere as "the strongest kernel-shaped primitive found anywhere in the inventory," per its own status block). The two prospective consumers sit at "separate documents, same design campaign" (both accepted 2026-09-14, within the decomposition effort).
- **Falsifier:** if, once built, either prospective consumer needs a materially different admission or staleness rule than `admitMutation`'s exactly-once-per-operation-id CAS check (e.g., multi-holder shared exercise, or non-monotonic fencing), that consumer would reveal coincidentally reused vocabulary, not confirmed cross-domain identity.
- **Documentation gap exposed:** real. The page's single "Confirmed Instances" heading flattens three evidentially distinct claims — one real preexisting generic implementation and two accepted-but-unbuilt cross-domain consumers — into one undifferentiated list, exactly the flattening §4 exists to prevent. Flagged for that page's own maintenance, not corrected here.

### `mechanisms/authority-preserving-intent-projection.md`

- **Eligibility:** 1–5 all pass. Domain-neutral discover/render/collect-intent/submit/lifecycle-feedback shape; explicit "Does Not Decide" list; substantial residue (the shared lifecycle vocabulary itself); route invariance across Studio (UI mutation) and Runtime Realization (execution policy) — materially different domains; distinctness stated directly against Candidate Resolution and Admission ("composable, never subsuming or subsumed").
- **Evidence kinds present:** `independent_domain_convergence` (Studio, Runtime Realization) — subject to the §4 caveat this page states by name: both are same-campaign artifacts (2026-09-16), so their origin-independence sits at "separate documents, same design campaign," not "independently developed subsystem," despite the source page's own "zero cross-reference" framing. `proposed_cross_domain_consumer` (Studio's command/edit projection, accepted design, not implemented).
- **Origin independence:** neither instance reaches above "same design campaign" — this mechanism currently has the thinnest origin-independence evidence of the five, even though it passes eligibility identically to the other four.
- **Falsifier:** if Runtime Realization's operator-policy-overlay, once built, does not implement the shared lifecycle vocabulary (`proposed`/`pending`/`admitted`/`refused`/`completed`/`stale`) but a domain-bespoke state machine instead, the claimed instance demotes to "inspired by," not "an instance of."
- **Documentation gaps exposed — two, both real:**
  1. The page's own "Confirmed Instances" subheading calls the Runtime Realization overlay a "**real, partial instance**," while the same page's own Status section states `implementation: none` and explicitly clarifies that what exists (`operator-switchboard.mjs`) is "a real but partial **precursor**... not an implementation of this mechanism itself." These two characterizations are in direct tension within one page. Applying §4's typed-evidence discipline forces the disambiguation the current prose blurs: this is `proposed_cross_domain_consumer` evidence with a strong precursor, not `implemented_cross_domain_consumer` evidence — the subheading's own wording should say so.
  2. This page's `owner` field names `app-server/docs/work-engine-studio-reconciliation.md`. Both `mechanisms/revision-cas-and-publication.md` and `mechanisms/candidate-resolution-and-admission.md` record an explicit correction of the identical mistake ("an earlier draft named `status-grammar.md` as `owner`... This page is its own canonical owner") and now self-own. This page never received that same correction and still names an external reconciliation document as `owner` for its own recognition status — an inconsistency across the five mechanism pages' own stated convention, not a new question this page invents.

Neither gap changes this mechanism's eligibility or its `design: accepted` status; both are reportable documentation inconsistencies within the existing corpus, surfaced by applying this page's own criterion rather than repaired by it.

---

## 11. Related Architecture Views

- **`work-engine-planned-architecture.md`** §1 — names this page and links here rather than restating the criterion; §5 and §11 cite this page's pressure-test result directly.
- **`status-grammar.md`** §7, §10.6 — the authorization-composition corollary and worked example this page's §9 depends on; recognition (this page) and authorization (that page) remain independent axes.
- **`mechanisms/revision-cas-and-publication.md`, `mechanisms/candidate-resolution-and-admission.md`, `mechanisms/transition-fencing-and-leases.md`, `mechanisms/resource-lease-and-fencing.md`, `mechanisms/authority-preserving-intent-projection.md`** — the five pressure-tested in §10; none reclassified.
- **`substrates/context-observer.md`, `substrates/evidence-anchor.md`** — structurally eligible for the identical §3–§4 test (both were named only after two independent consumers existed); not pressure-tested here, since this page's brief was the five mechanisms specifically, and reopening the substrates was not requested.

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

`design: accepted` — this page's own recognition criterion, eligibility/evidence split, origin-independence ladder, falsifier rule, and sibling-versus-merge test were explicitly settled through direct discussion (2026-09-18), the same bar every mechanism and dimension recognition in this architecture is held to. `reconciliation: reconciled` — pressure-tested directly against all five current mechanism pages' own text, `status-grammar.md`, and `work-engine-planned-architecture.md` §1/§5/§11, this session, not restated from summary; two real documentation gaps were found and are recorded in §10 rather than corrected on the pressure-tested pages. `authorization: design_work_authorized` — this page is a documentation and taxonomy convention, not a buildable artifact; nothing here authorizes reclassifying or rebuilding anything it pressure-tests. `implementation: none` — not applicable to a taxonomy page. `owner`: self, following the corrected convention `mechanisms/revision-cas-and-publication.md` and `mechanisms/candidate-resolution-and-admission.md` already use, and the correction §10 recommends for `mechanisms/authority-preserving-intent-projection.md`.
