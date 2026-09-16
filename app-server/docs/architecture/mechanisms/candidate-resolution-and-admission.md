# Candidate Resolution and Admission

> **Question:** How does a bounded candidate space become an admitted selection, using deterministic reduction wherever possible, without the reduction mechanism ever acquiring the domain's own judgment authority?

## Purpose

This is a **mechanism view**, not a dimension view. It describes a reusable reduction-and-admission protocol that recurs across independently owned truth domains, not a class of architectural truth this page owns itself.

> **The mechanism reduces and validates a candidate space. It never supplies the domain meaning of what survives the reduction.**

Independently owned dimensions keep reaching for the identical shape without copying it from one another — `organizational-compilation.md` §7 (accepted design, 2026-09-15), `runtime-realization.md` §4 (proposed, checked against source 2026-09-16), and, confirmed by direct falsifier testing during the front-end proposal→decision chain's own pressure-testing (2026-09-16), `material-decision-selection.md`'s own final selection step. A fourth, `portfolio-selection.md`'s own final disposition step, is named as likely but not yet formally confirmed (§ Confirmed Instances). Independent instances converging on one shape, each with its own decision owner but the identical reduction structure, is the evidence for documenting it once, here.

---

## The Boundary: What the Domain Owns, What the Mechanism Owns

```text
DOMAIN OWNS                          MECHANISM OWNS
    candidate meaning                    candidate-set reduction protocol
    requirements                         eligibility intersection
    authority ceiling                    deterministic resolution where possible
    residual semantic choice             explicit residual decision surface
                                          admission of the selected result
```

The mechanism never invents what a "candidate" is, what "available," "authorized," or "required" mean for any given domain, or who the decision owner is when judgment remains. Every one of those is supplied by the domain that uses it.

---

## The Reusable Shape

```text
candidate universe
        ↓
AVAILABLE   — reject candidates that cannot actually be constructed or
              realized from what currently exists
        ↓
AUTHORIZED  — reject candidates the current authority ceiling, policy,
              or delegation rules do not permit
        ↓
SATISFIES(REQUIRED) — reject candidates that fail the domain's own
                       declared requirements; resolve whatever is already
                       mechanically determined by known inputs
        ↓
admissible candidates
        ├── 0 → explicit gap or failure — the mechanism can establish
        │       impossibility
        ├── 1 → mechanically determined — the mechanism can establish
        │       determinacy; no judgment needed
        └── N → the mechanism can establish that judgment remains, and
                returns the residual decision to the domain's own owner
                        ↓
                     selected
                        ↓
                     admitted
```

**The zero/one/many distinction is the central fact this mechanism exists to establish, not an incidental detail of it.** At each of the three outcomes, the mechanism proves something real — impossibility, determinacy, or the continued existence of an irreducible choice — but it can never itself supply the domain meaning that choice requires. That is precisely Work Engine's own recurring discipline of pushing inference out until only irreducible semantics remain, applied at the mechanism level rather than restated separately by every dimension that needs it.

---

## What This Mechanism Does Not Decide

- **it does not decide what a candidate means** — an "organization," a "runtime realization," or any other domain-specific candidate shape remains entirely the owning dimension's own content;
- **it does not decide who owns the residual N-candidate judgment** — `organizational-compilation.md` names organizational authority; `runtime-realization.md` names "the supervisor, another authorized role, the operator, or the human who owns budget or product authority" — a different, domain-specific answer each time, never supplied by the mechanism itself;
- **it does not decide what counts as available, authorized, or required** — those are domain-defined predicates the mechanism applies, never predicates it invents;
- **it does not admit anything on its own authority** — admission is exercised by whichever domain-owner class `authority-and-ownership.md`'s own model grants that authority to, never by the mechanism.

---

## The Connection to Invalidation-Never-Mints-Authority

`authority-and-ownership.md` §12's generative invariant — "failure of an authorized candidate does not authorize a previously unauthorized alternative" — is stated in terms of this exact mechanism:

```text
candidate invalidated
        ↓
rerun Candidate Resolution and Admission
        ↓
against the SAME authority ceiling

never:

candidate invalidated
        ↓
expand authority until something works
```

Rematerialization after invalidation (`runtime-realization.md` §8) and recursive organizational compilation after local evidence confirms a plan (`organizational-compilation.md` §12) are both, structurally, an ordinary rerun of this mechanism against unchanged authority — not a special recovery path, and never an occasion to widen what's permitted.

---

## Confirmed Instances

### Organizational Compilation — accepted design, not implemented

`organizational-compilation.md` §7, accepted 2026-09-15: organizational authority selects a candidate organization from `available ∩ authorized ∩ satisfies(required)`. *Available* = realizations constructible from current role primitives, capabilities, runtime realizations, and resources. *Authorized* = the subset the authority ceiling, delegation rules, workflow policy, and effect boundaries permit. *Required* = properties the organization must satisfy (independence, continuity, effect separation, semantic obligations, capability needs), some mechanically known, an unresolved remainder resolved by bounded judgment. Zero candidates = organizational gap; one = mechanically determined; N = the only place a genuine selection judgment belongs. `ExecutionEnvelope` records the admitted selection — it does not select.

### Runtime Realization — proposed, not implemented

`runtime-realization.md` §4: deterministic machinery rejects candidates that fail requirements or exceed ceilings, applies explicit prohibitions and budget bounds, tests dependency validity against the current capability generation, and ranks candidates when policy gives a complete ordering. A decision owner — "the supervisor, another authorized role, the operator, or the human who owns budget or product authority" — resolves the remainder only when admissible candidates differ in genuinely undetermined meaning (review independence, evidence strength, continuity loss, latency, a new cost/authority tradeoff). Zero candidates = explicit gap; one = mechanically determined; N = the decision-owner's residual judgment.

### Material Decision Selection — proposed, not implemented (third confirmed instance)

`material-decision-selection.md` §7, confirmed by direct falsifier test 2026-09-16: the dimension's own materiality test and authority classification (reserved/delegated) are irreducible domain content the mechanism cannot supply, but the final act — genuinely admissible routes → disposition (`selected`/`rejected`/`delegated`/`deferred`) — matches this mechanism's shape precisely, with the named decision owner supplying the residual N-case judgment. `AVAILABLE` = routes compatible with the proposal's current meaning, invariants, placement, and evidence; `AUTHORIZED` = the reserved/delegated authority ceiling; `SATISFIES(REQUIRED)` = the route-invariance materiality test. Zero candidates = no material choice remains; one = mechanically determined; N = the decision owner's residual judgment, exactly as this mechanism's shape predicts.

### Portfolio Selection — proposed, not implemented (likely fourth instance, not yet formally confirmed)

`portfolio-selection.md` §8, per its own falsifier test 2026-09-16: `PortfolioDecision`'s final dispositions (`select_for_campaign`/`defer`/`exclude`) over the set of ready/eligible proposals structurally match this mechanism's shape, with capacity/policy/authority as the filtering predicates. Flagged as likely, not confirmed, by that page's own text — the basis-binding and cross-proposal-analysis content is real dimension truth regardless of whether the final selection act is ever formally recorded as this mechanism's instance.

---

## Key Invariants

1. **The mechanism reduces and validates a candidate space; it never supplies the domain meaning of the residual choice.**
2. **Zero, one, and many surviving candidates are three genuinely different, mechanically establishable facts — impossibility, determinacy, and irreducible judgment — never collapsed into one.**
3. **The residual decision owner is always domain-supplied, never the mechanism's own.**
4. **Admission is exercised by whichever domain-owner class holds that authority; the mechanism itself admits nothing on its own standing.**
5. **Rerunning this mechanism after invalidation never expands the authority ceiling it resolves against — see `authority-and-ownership.md` §12.**
6. **Reusing this mechanism for a new domain does not inherit whatever residual-judgment owner or requirement predicates governed its prior use — each new instance defines its own.**

---

## What This View Does Not Show

This page does not define:

- what "available," "authorized," or "required" mean in any specific domain (each dimension's own page);
- who the residual decision owner is for any specific domain (each dimension's own page);
- how the selected result becomes durable, revisioned state once admitted (`mechanisms/revision-cas-and-publication.md`);
- how concurrently prepared candidate resolutions stay coherent against a changing authoritative revision (`mechanisms/transition-fencing-and-leases.md`, pending);
- any concrete scoring, ranking, or optimization algorithm for the deterministic-reduction steps — those remain domain policy, not mechanism content.

---

## Relationship to Revision/CAS/Publication

Different questions, frequently composed: this mechanism decides *what* gets admitted; `mechanisms/revision-cas-and-publication.md` decides how the admitted result is published as durable, safe, successor state. Organizational Compilation and Runtime Realization both use this mechanism to decide a selection, then (once either is implemented) would use revision/CAS to publish it.

## Relationship to Authority and Ownership

The residual-judgment step depends directly on `authority-and-ownership.md`'s observe/nominate/decide/admit/execute vocabulary — the domain owner who resolves an N-candidate remainder is exercising exactly the "decide" mode that page defines, and the eventual admission is exactly its "admit" mode. §12's invalidation-never-mints-authority invariant is stated in terms of this mechanism directly (see above).

---

## Related Architecture Views

- **`organizational-compilation.md`**, **`runtime-realization.md`**, **`material-decision-selection.md`** — three confirmed instances.
- **`portfolio-selection.md`** — a likely, not yet formally confirmed, fourth instance.
- **`mechanisms/revision-cas-and-publication.md`** — the mechanism a selection is published through once admitted.
- **`authority-and-ownership.md`** — owns the vocabulary and the invalidation invariant this mechanism's own residual-judgment step and rerun behavior both depend on.

---

## Source and Status

```yaml
architecture_status:
  design: accepted
  reconciliation: reconciled
  authorization: design_work_authorized
  implementation: none
  owner: app-server/docs/architecture/mechanisms/candidate-resolution-and-admission.md
  status_as_of: 2026-09-16
```

**Corrected before finalizing:** an earlier draft named `status-grammar.md` as `owner`. That document defines what the status fields *mean*; it cannot be the semantic owner of this mechanism's own content, which would quietly violate the same dimension-vs-mechanism-vs-substrate ownership discipline this whole architecture is built on. This page is its own canonical owner. The acceptance provenance stays in prose rather than in the `owner` field, since `owner` names *what owns the claim*, not *what evidence justifies its current status* — a distinction worth watching for recurring elsewhere before it earns a fifth grammar field of its own: the mechanism's own recognition and naming was explicitly settled through direct discussion (2026-09-16), the same bar every other mechanism/dimension recognition in this architecture is held to.

`reconciliation: reconciled` — confirmed across three independently owned dimensions, with a fourth likely. `implementation: none` — unlike Revision/CAS, none of the confirmed instances (Organizational Compilation, Runtime Realization, Material Decision Selection) has any implementation evidence at all; all remain accepted-design or proposed only. Each dimension's own page remains the authority on its own instance's status; this page does not restate or override any of them.
