# Reconciliation: Project UI Review Capability

## Status

Reconciliation of `ideas/ui-review-capability.md` against current
implementation, `app-server/ideas/pending/`, `proposals/`, and existing
prospective-architecture documents — including its own named dependency,
`ui-experience-evidence.md` and its formed proposal, evaluated inline here
rather than treated as a separate queue item. Wave 4 of the sequel
reconciliation queue.

Per instruction: the idea was not improved or modernized. Applying the
patterns established by items 1–11: check whether a plausible neighboring
mechanism is genuinely the same shape before retiring to it; couple to a
formed-but-undecided proposal rather than assuming it is settled substrate;
and check this session's own prior reconciliations
(`ai-accessible-browser-seam-reconciliation.md`, the joint seam-review/
architectural-review reconciliation) for direct connections before treating
this idea as isolated.

## Idea summary (unchanged)

`ideas/ui-review-capability.md` asks for a review capability judging whether
a human-facing interface faithfully and efficiently represents the machinery
it exposes, using four co-equal lenses — Truth, Maintainability,
Explainability, Aesthetics — each given a distinct attributed reviewer
perspective for material human-facing review, with accessibility framed as a
validity requirement across all four lenses (not a fifth lens) and an
explicit, still-open falsification test for whether a fifth dimension
exists. It requires rendered evidence (potentially through
`ui-experience-evidence.md`), mechanism evidence (repository state,
ownership, provenance, uncertainty, reversibility, failure modes), and the
consuming project's own design doctrine — keeping observation, interpretation,
and proposal strictly separate. Its own "Does not own" section excludes Work
Engine Studio, project design doctrine, Chrome Vision, UI experience-evidence
identity/capture/lineage/freshness, repository evidence, and implementation
decisions.

## What a prior pass already established, re-verified rather than re-derived

`root-ideas-reconciliation.md:334-337` already found no app-server UI-review
role, panel, or four-lens judgment mechanism, and confirmed the idea is
"correctly still gated behind its dependency"
(`ui-experience-evidence.md`). Re-checked directly: still true. That pass
predates this session's `ai-accessible-browser-seam-reconciliation.md` and
the joint seam-review/architectural-review reconciliation, both of which
materially change what "gated behind its dependency" now means — see below.

## The four lenses already exist, but scoped to one product, not Work Engine

`skills/ui-design-principles/SKILL.md` already defines Truth,
Maintainability, Explainability, and Aesthetics as "one design system,"
word-for-word matching this idea's lens names, but explicitly scoped to one
consuming project: "Shape Site2JSON so its internal structure, human-facing
expression, and continued evolution reinforce one another... This skill
reconstructs the draft doctrine in `../site2json/DESIGN_PRINCIPLES.md`." The
idea's own Status line already states this precisely: "the current
`ui-design-principles` skill is a retained Site2JSON compatibility surface,
not a generic Work Engine UI architecture owner." Direct inspection confirms
this is accurate, not stale.

This does not mean Truth/Maintainability/Explainability/Aesthetics should be
promoted to Work-Engine-owned doctrine — the evidence points the other way.
`skills/ui-design-principles/SKILL.md` explicitly says "do not apply as a
generic visual-style guide," and this idea's own falsification test for a
fifth dimension is deliberately left open rather than closed. Freezing the
four names as a universal ontology would contradict both.

What survives is narrower and more structural: a **project-parameterized
`UIReviewProfile` contract** — project/doctrine revision, required
concerns/lenses, a judgment contract per concern, evidence requirements, and
independence/completeness requirements — with Site2JSON's four-lens set as
the **first demonstrated profile**, not the global ontology:

```text
UIReviewProfile
    project / doctrine revision
    required concerns/lenses
    judgment contract per concern
    evidence requirements
    independence/completeness requirements

Site2JSON profile (first concrete instance):
    Truth
    Maintainability
    Explainability
    Aesthetics
```

Nothing found anywhere defines this contract shape. This is the idea's own
explicitly-flagged open question, reframed as a contract rather than a
"generalized doctrine" — the lens ontology stays deliberately open to
evidence, per the idea's own falsification test.

## What is already implemented or formed (retires most of "Does not own" and the panel mechanism)

### `ai-accessible-browser-seam-reconciliation.md` (retires the rendered-evidence dependency's layering, not the evidence itself)

This session's own AI-Accessible-Browser reconciliation already established
a 4-layer ownership model materially relevant here: Chrome → AI-Accessible
Browser (capture/normalization/indexes/projections) → UI Experience Evidence
Interface (product-view semantics) → `claim-evidence` (claim lifecycle).
`ui-review-capability.md`'s own "Does not own" list (Chrome Vision, UI
experience-evidence identity/capture/lineage/freshness) already matches this
layering precisely — it was written consistently with this even before this
session's reconciliation made the layering explicit.

Disposition: retire "Chrome Vision" and "UI experience-evidence identity,
capture, lineage, or freshness" as already correctly excluded and now
concretely grounded in a real, already-reconciled layering — not merely a
sound boundary statement.

### `proposals/ui-experience-evidence/ui-experience-evidence-interface/` (couples, does not retire, the rendered-evidence input)

Checked directly: this proposal exists, is formed, but has no `decision.json`
— unlike `revision-bound-review-artifacts` and `adaptive-review-panel-coordination`
(both `approve_proposal_meaning`), this one has not yet been decided at all.
`ideas/ui-experience-evidence.md`'s own Status line already states this
correctly: "It is the authorized idea source for the formed candidate... The
proposal owns candidate meaning; this document preserves the broader
unresolved design space and does not authorize implementation." This
reconciliation does not decide the proposal or change its status; it
confirms `ui-review-capability.md`'s own framing ("Potentially through the
separately explored `ui-experience-evidence` boundary") is accurate as
written — a real, named, correctly-hedged dependency on an undecided
proposal, not a false or stale one.

### `adaptive-review-panel-coordination` + `concern-scoped-review-judgments` (couples specialist execution to the same substrate item 10 already dogfooded — but does not retire the required-concern semantics)

The idea's mechanism for *running* distinct reviewer perspectives is not a
new coordination mechanism to invent. It is the same open, model-interpreted
specialist registry `adaptive-review-panel-coordination`'s own proposal
already describes ("Potential specialist capabilities remain an open
registry"), using the same concern-scoped judgment substrate
`concern-scoped-review-judgments` proposes (`ReviewEpisode`/`ReviewResult`/
`ReviewJudgment`/`Finding`, each judgment scoped to one versioned "concern").
Item 10's reconciliation already found this exact pattern real and
dogfooded for one specialist (`agent-instruction-review`): a distinct
skill, delivered read-only with a zero-effect boundary, registered as one
specialist among several considered and exercised during real review.

The first pass of this reconciliation retired more than this evidence
supports: "four UI-review lenses would be four more concerns in the same
open registry" collapses *selection mechanics* (which the coordinator does
own) with a *semantic requirement the coordinator does not establish*. The
idea's own text is explicit: "for material human-facing review, preserve
four co-equal judgment lenses... assign one distinct reviewer perspective to
each lens... adaptive panel selection does not use one strong dimension as a
reason to omit another." An open, model-interpreted registry can select
specialists; it does not thereby know that, *for this review profile*,
certain concerns are mandatory, must remain independently attributed, and
whose omission is invalid rather than a legitimate selection choice. That
is exactly the `UIReviewProfile`'s own "required concerns" and
"independence/completeness requirements" fields above — not something
`adaptive-review-panel-coordination` supplies on its own:

```text
adaptive-review coordination
    owns: selecting/running specialists, independence/synthesis mechanics,
          the open specialist registry

UIReviewProfile
    owns: which concerns are required for this profile, concern-specific
          judgment semantics, the completeness/omission contract
```

Both proposals remain formed but undecided (`concern-scoped-review-judgments`
has no `decision.json` at all; confirmed in item 10's own investigation).
This is a `COUPLED_RECONCILIATION`, the same shape as items 4, 5, 6, 9, and
11: the substrate this idea needs for *running* specialists already has a
well-specified, real precedent, but the *required-concern/completeness*
semantics belong to the `UIReviewProfile` contract, not to the coordinator,
and this reconciliation does not assume either proposal intends to host
UI-review lenses merely because the shape fits.

### Evidence boundary (observation/interpretation/proposal) — retired as correct, now with direct precedent

The idea's "Evidence boundary" section (keep observation, interpretation,
and proposal separate) is retired as correct and consistent with every
mechanical/semantic split this queue has already established:
`claim-evidence`'s discovery-vs-reliance split, `evidence-anchor`'s
mechanical-comparator-vs-domain-owner-judgment split (items 8–9), and item
11's prediction-vs-calibration-diagnosis split. No correction needed; this
idea already applies the same discipline independently.

## A direct, partial answer to items 8–9's own open residue

Items 8–9's joint reconciliation
(`cross-cutting-seam-review-and-architectural-review-reconciliation.md`)
named "state-ownership ↔ UI-representation" as one of seam review's own
example seam types requiring a semantic "truthfully and proportionately"
judgment that `evidence-anchor`'s deliberately narrow comparator explicitly
refuses to make — and left "who makes that judgment" as part of that
reconciliation's own unresolved residue (residue item 2: "The semantic
'truthfully and proportionately' correspondence judgment... no owner or
record shape... exists anywhere").

This idea's "Truth" concern — "the interface corresponds to the real state,
relationships, uncertainty, consequences, and authority of the machinery it
represents" — is not merely *structurally similar* to that judgment; for the
concrete case of a material human-facing interface, it is a **candidate
domain owner** of exactly that judgment:

```text
generic seam review
    detects/frames: UI-representation <-> state correspondence seam
    (items 8-9's own residue; not resolved generically here)

UI review's Truth concern
    candidate domain owner of: the semantic correspondence judgment,
    specifically when material human-facing interface review is invoked
```

This does not resolve items 8–9's residue generically — seam review may
still need this judgment's owner for seams with no human interface at all,
which the "Truth" concern cannot supply. But for the one concrete case both
ideas name explicitly (UI-representation ↔ state), this reconciliation
identifies a real, named candidate owner rather than leaving both sides
symmetrically unresolved. Neither this idea nor items 8–9 is itself built,
so nothing is decided by this connection — it is recorded so the two are
not designed twice independently.

## The smallest remaining semantic consequence still lacking an owner

Decomposing rather than treating the idea as one residue:

```text
existing / coupled
    browser + rendered evidence (AI-Accessible-Browser layering, this
        session's own reconciliation)
    ui-experience-evidence candidate (formed, undecided proposal)
    claim/evidence lineage (claim-evidence, evidence-anchor)
    adaptive review coordination (specialist selection/execution mechanics)
    concern-scoped review artifacts (ReviewResult/ReviewJudgment/Finding)
    Site2JSON four-lens precedent (skills/ui-design-principles)

genuinely missing: one contract, not three separate gaps
    UIReviewProfile
        project/doctrine revision
        required concerns
        concern-specific judgment semantics
        per-concern evidence requirements (folds in what the first pass
            called "mechanism-evidence sourcing" -- e.g. Truth needs
            rendered state + authoritative underlying state + provenance +
            uncertainty + authority semantics; Maintainability needs
            ownership + reversibility + user responsibility + mutation
            lifecycle -- existing source owners/projectors supply whatever
            evidence satisfies these, not a new evidence-projection layer)
        independence/completeness rules (which concerns are mandatory,
            must remain independently attributed, and may not be omitted
            merely because one other concern was strongly satisfied)

first concrete candidate profile
    Truth, Maintainability, Explainability, Aesthetics (Site2JSON's
        existing doctrine, demonstrated but not yet generalized)

domain connection (partial answer to items 8-9's own residue)
    the Truth concern is the candidate UI-domain owner of the
    UI-representation <-> state semantic seam judgment, specifically when
    material human-facing interface review is invoked
```

This is not three independent gaps (a generalized lens framework, mechanism-
evidence sourcing, and a panel-completeness question) as the first pass of
this reconciliation treated them. It is one missing contract —
`UIReviewProfile` — whose fields happen to answer all three. It is not a new
standalone architecture the way item 3's execution envelope was: it is a
domain profile over evidence and specialist-review machinery this queue has
already found real or already formed, not free-standing unowned territory.

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| Four lenses (Truth/Maintainability/Explainability/Aesthetics) | **Reclassified, not promoted to doctrine.** Real as Site2JSON's own precedent (`skills/ui-design-principles`); stands as the first concrete `UIReviewProfile` instance, not a Work-Engine-owned universal ontology — the idea's own falsification test stays open. |
| Falsification test for a fifth dimension | Retired as correctly framed — an explicitly open question, not resolved here or anywhere found. |
| Human access surface (accessibility as cross-lens validity, not a fifth lens) | Retired as a correct, self-consistent framing; no competing owner or conflict found. |
| Rendered evidence / `ui-experience-evidence` dependency | Retired as correctly gated — a real, formed, but undecided proposal; the idea's own hedge ("Potentially through") is accurate. |
| Mechanism evidence | **Folded into `UIReviewProfile`'s evidence-requirements field**, not a separate residue — per-concern requirements that existing source owners (claim-evidence, evidence-anchor, browser evidence) satisfy, not a new evidence-projection layer. |
| Reviewer-assignment / adaptive panel mechanism | **Split.** Specialist selection/execution mechanics retire to `adaptive-review-panel-coordination` + `concern-scoped-review-judgments`, following item 10's dogfooded precedent. Required-concern/completeness semantics do **not** retire there — they belong to `UIReviewProfile`, part of the residue. |
| Evidence boundary (observation/interpretation/proposal) | Retired as correct — consistent with every mechanical/semantic split this queue has established. |
| Does not own | Retired — each exclusion already has a confirmed owner, including this session's own AI-Accessible-Browser layering. |
| Connection to items 8–9's "UI-representation ↔ state" seam residue | **Partially resolved**, not merely surfaced — the Truth concern is a named candidate domain owner for the UI-specific instance of that judgment, though items 8–9's generic residue remains open for non-UI seams. |

## Recommended status change to the idea file

Update `ideas/ui-review-capability.md`'s Status section to note that this
reconciliation exists; that its four lens names are already real as
Site2JSON's own precedent (`skills/ui-design-principles`) but should stand
as the first concrete instance of a project-parameterized `UIReviewProfile`
contract, not a Work-Engine-owned universal ontology; that its
rendered-evidence dependency is correctly gated behind the still-undecided
`ui-experience-evidence` proposal, with mechanism-evidence needs expressed
as that profile's per-concern evidence requirements rather than a new
evidence-projection layer; that specialist execution is coupled to
`adaptive-review-panel-coordination` and `concern-scoped-review-judgments`
following the same pattern item 10 already dogfooded for
`agent-instruction-review`, while required-concern/completeness semantics
belong to the `UIReviewProfile` contract itself, not the coordinator; and
that its Truth concern is a named candidate domain owner (not a full
resolution) of the joint seam-review/architectural-review reconciliation's
own open "UI-representation ↔ state" semantic-judgment residue, specifically
for material human-facing interfaces, worth tracking so neither capability
is designed twice independently.

## Acceptance

**Accepted 2026-09-14** (explicit user decision, after a closer-look review
of this reconciliation as part of the sequel queue's acceptance pass). This
document's findings and disposition are confirmed accurate. The stated
residue — the `UIReviewProfile` contract — is authorized for **design work
only, not implementation yet**, pending its project/doctrine-revision,
required-concern, judgment-semantics, evidence-requirement, and
completeness-rule fields being specified.
