# Reconciliation: Proposal Research Maturity, Readiness, and Freshness

## Status

Reconciliation of `ideas/proposal-research-maturity-and-freshness.md` against
current app-server implementation and current prospective architecture.
Wave 2, item 5 of the sequel reconciliation queue — the first item checked
against `app-server/ideas/pending/` as an explicit source of implementation
ideas, per updated instruction, not only `app-server/docs/` and root
`ideas/`.

Per instruction: the idea was not improved or modernized. Every clause was
checked against implemented code or an existing prospective-architecture
document, applying the lessons from items 1–4: verify exact vocabulary and
type signatures rather than family resemblance, check whether a prior
reconciliation pass already covered this exact idea file, and check every
plausibly relevant document (including now the compressed idea's own
un-compressed history source) before drafting a conclusion.

## Idea summary (unchanged)

`ideas/proposal-research-maturity-and-freshness.md` (a compressed
descendant of the fuller, archived
`ideas/history/2026-08-22-pre-reconciliation/research-maturity-evidence-snapshots-and-staleness.md`
— see below) asks that proposal research accumulate in reusable layers and
express which decisions current evidence can support. It defines a
cumulative maturity ladder (R0 Captured → R1 Formed → R2 Situated → R3
Characterized → R4 Organization-qualified → R5 Activation-ready), states
that readiness is decision-specific (a proposal can be simultaneously
placement-ready, portfolio-ready, organization-blocked, and
activation-blocked), and defines freshness/refresh semantics (mechanical
analysis marks a claim `candidate-stale`; attributed judgment determines
whether it remains current, needs revision, is stale, refreshed, or
superseded). Its own Status line already says it should consume the shared
claim-centered evidence-lineage candidate rather than redefine it, and its
"Does not own" section already excludes the shared claim-lineage schema,
proposal evaluation, organizational compilation, portfolio decisions, and
continuous monitoring infrastructure.

## What a prior pass already established, re-verified rather than re-derived

`root-ideas-reconciliation.md:265-274` already found that this idea's own
Status line points at a claim-centered evidence-lineage candidate whose App
Server placement is `claim-evidence-service.md`, and that
`revisioned-research-and-execution-architecture.md` §27 (coordinate
adequacy — re-checked directly here: it is about experimental-replay
reconstruction fidelity, a different subject) and
`evidence-calibrated-plan-resolution-and-continuous-capability-learning.md`'s
evidence-sources section (re-checked directly here: its own "Relationships
and boundaries" table never mentions proposal readiness, and it is about
provider/model capability evidence for execution routing, a different
subject) are adjacent but do not define proposal readiness stages or
claim-freshness semantics. Both re-checks confirm the prior finding rather
than overturn it.

## The idea's own un-compressed history, checked directly (not previously done)

`proposals/adaptive-specialized-review/revision-bound-review-artifacts/packet.json`
names `ideas/history/2026-08-22-pre-reconciliation/research-maturity-evidence-snapshots-and-staleness.md`
as `idea_evidence` in its `origin_refs`. That 871-line archived document is
the un-compressed source this idea's current ~70-line file was reduced from
during the reconciliation-map history process (per the user's own account
of that process — a merge/compression, not a migration-staleness signal).
It is not itself a live idea to reconcile separately, but it grounds what
the compressed idea actually means and is checked directly here for the
first time:

- Its §10–13 ("Two staleness channels," "Freshness states," "Refresh
  behavior," "Git storage choices") are, word for word in places, the same
  design that became `claim-evidence-service.md`'s "Impact, refresh, and
  reliance propagation" pipeline: mechanical impact detection marking a
  claim `candidate_stale`, attributed judgment producing
  current/stale/refreshed/superseded, typed proposal relationships as
  propagation candidates that "do not prove that related research is
  stale." This is not merely a vocabulary match to be double-checked per
  the items-1–4 lesson — it is the same design lineage, confirmed by the
  packet's own `idea_evidence` reference.
- Its §4.4 "Judgment artifact" example (a freshness judgment with
  id/kind/subject/conclusion/decision_scope/producer/authority/evidence/
  limitations/`changes_contract`/`human_approval`) is a domain-specific
  instance of exactly the fields `claim-evidence-service.md`'s refresh
  judgments already carry — not a competing shape, and not the same subject
  as item 4's still-open "durable semantic judgment" residue (this one is
  scoped to claim freshness specifically, not general role judgment
  ancestry).
- Its §14 "Interaction with organizational execution envelopes" states
  "Organizational compilation should consume an organization-qualified
  projection of the packet, not raw research history" — this is the exact
  seam item 3's reconciliation already placed as "Problem specification...
  explicitly an upstream input this idea consumes rather than produces —
  not this idea's own gap." The two reconciliations agree independently.
- Its §15 "Ontology minimization audit" already proposed treating "Readiness
  profile" as a "Configuration or projection of a decision contract" that
  "may express requirements for one decision without becoming a universal
  proposal state" — this is precisely what survives, unimplemented, in the
  compressed idea's "Readiness is decision-specific" section.

Disposition: this document is evidence, not a live idea; it does not change
what is retired below, but it substantially strengthens confidence that the
Freshness/Refresh retirement is correct (genetic lineage, not resemblance)
and that the R0–R5/readiness-profile residue is a genuine, long-standing
gap rather than an oversight in the compressed idea's editing.

## What is already implemented and designed

### `claim-evidence-service.md` / `app-server/src/services/claim-evidence` (retires the Freshness/Refresh mechanism, not freshness as a readiness concern)

Checked field-by-field and stage-by-stage, not assumed from the idea's own
Status-line pointer:

- the idea's mechanical/attributed split ("Mechanical analysis may mark a
  claim candidate-stale. Attributed judgment determines whether the
  conclusion remains current, needs revision, is stale, refreshed, or
  superseded") is exactly claim-evidence's `may_affect` nomination →
  optional domain-owned refresh episode →
  `retained_unchanged`/`changed`/`inapplicable`/`insufficient`/`contested`/
  `deferred`/`superseded` (`claim-evidence-service.md:457-471`);
  `superseded` is an exact vocabulary match, and the mechanical/attributed
  *distinction itself* — not just a shared word — is the same design,
  confirmed by the shared history above;
- the idea's "Refresh should reuse prior durable conclusions and gather new
  evidence only for dimensions plausibly affected by change" is exactly
  claim-evidence's stated preference for "on-demand refresh when a real
  consumer needs a decision" over continuous re-verification.

Disposition: retire the freshness/refresh **mechanism and its ownership** to
`claim-evidence-service.md`, with the idea's own Status line and "Does not
own" section already correctly anticipating this. Do not retire freshness as
a *concern* of this idea's remaining scope: freshness is a necessary
*referenced facet* of a decision-specific readiness assessment (see residue
below), and a claim's freshness change can invalidate or reopen a
previously published readiness assessment that relied on it —

```text
claim C17 becomes candidate-impacted
        |
        v
authorized refresh
        |
        v
C17 freshness changes
        |
        v
a readiness assessment that relied on C17
    may require reassessment
```

— without the readiness layer itself owning refresh. This propagation
consumer relationship is part of the residue's own open scope, not a
competing claim-evidence responsibility.

### `app-server/src/services/claim-evidence/authorized-vertical.mjs` (confirms substrate exists; confirms maturity/readiness does not)

`VERTICAL_PROFILES` in this file already includes `"proposal-research-v1"`
alongside `"revision-bound-review-finding-v1"` — proposal-research claims
are not merely named as an aspiration in `claim-evidence-service.md`'s
"two initial domain profiles" (`:121-124`); a real, implemented profile
exists in code. Its `validatePublication` function checks subject/evidence
baseline/decision-scope/content-set consistency — the generic claim
substrate. It has no field, state, or concept resembling R0–R5, a maturity
stage, or a decision-specific readiness profile.

Disposition: retire "the shared claim-lineage schema" boundary claim (the
idea's own correct exclusion) with stronger evidence — not just a design
document but a real, implemented vertical profile. Confirm, not retire, that
the maturity/readiness layer this idea actually proposes is absent from that
implementation.

## What the idea's own boundary already gets right

`organizational-execution-envelopes.md` (item 3), `proposal evaluation`
(item 6, not yet reconciled), and `proposal-backed-portfolio-selection.md`
(item 7, not yet reconciled) are each named exclusions in this idea's "Does
not own" section. Item 3's reconciliation already independently confirmed
the organizational-compilation seam (see above). Items 6 and 7 are not yet
reconciled by this queue, so their ownership is not re-verified here beyond
confirming they exist as named idea files — no conflict found, nothing
retired prematurely on their behalf. `ideas/evidence-backed-proposal-evaluation.md`
(item 6) was read directly while checking this exclusion, and its own text
already lists reversibility, implementation complexity, maintenance burden,
validation burden, confidence, and freshness — nearly the entire R3
"Characterized" list this idea's ladder names. This is recorded below as a
likely `SUPPLIES` relationship (evaluation supplies the evidence a readiness
assessment reads) rather than resolved as an ownership question, since item
6 has not yet had its own standalone reconciliation.

One additional real consumer not named in the idea's own "Does not own"
list, found while checking `app-server/ideas/pending/`:
`proposal-decision-gated-implementation-compilation.md`'s "plan-conformance
gate" establishes "plan readiness" — a different, later-stage readiness
concept (whether an already-decided implementation plan is ready for an
executor), sequential with rather than competing against this idea's R0–R5
ladder (research maturity precedes decision; decision precedes plan
compilation; plan readiness is downstream of both). Not a conflict; noted as
an accurate additional `CORRESPONDS` neighbor, not a correction to the
idea's boundary.

## The smallest remaining semantic consequence still lacking an owner

The R0–R5 ladder and the decision-specific readiness profile do not have
equal architectural status, and the first pass of this reconciliation
treated them as one undifferentiated residue. The idea's own un-compressed
history already warns against this: it calls the R-levels "epistemic
states, not a mandatory ceremony," then immediately states "one scalar
level cannot express every kind of readiness" and directs consumers to
request a readiness profile instead. Decomposing R0–R5 against existing and
queued owners confirms the warning:

```text
R0 Captured    -> idea/intake provenance and rough problem (formation)
R1 Formed      -> proposal formation (already exists)
R2 Situated    -> placement / architecture / repository relationship
                  (already exists)
R3 Characterized -> expected value, complexity, risk, reversibility,
                  maintenance, validation burden, fan-out
                  -> nearly exactly `ideas/evidence-backed-proposal-evaluation.md`
                     (item 6, not yet reconciled by this queue, but its own
                     text already names reversibility, implementation
                     complexity, maintenance burden, validation burden,
                     confidence, and freshness almost verbatim)
R4 Organization-qualified -> a statement that enough information exists for
                  the organizational-envelope consumer; item 3 already
                  established the envelope consumes exactly this
                  "organization-qualified projection" as an upstream input,
                  which does not make "R4" an independent owner
R5 Activation-ready -> bundles accepted objective/scope, authority
                  decisions, selected organization, current runtime/
                  capability availability, validation requirements,
                  tolerated uncertainty, and stop/escalation conditions —
                  each already owned by formation, authority, organizational
                  compilation, or runtime admission
```

No individual R-stage is new canonical state this idea must own. **The R0–R5
ladder is downgraded from core residue to an optional, coarse derived
projection** — a plausible "minimum durable understanding reached" summary
label, but one that should be derived from the stronger underlying
state/readiness assessments below, not treated as a second canonical
lifecycle or state machine competing with formation, placement, evaluation,
organizational qualification, authority, and runtime admission.

**What genuinely survives, and is the primary residue, is the
decision-specific readiness assessment/profile itself** — for example:

```text
placement decision:        ready
portfolio selection:       ready
organizational compilation: blocked
    missing: independence requirement, mutation authority
activation:                 blocked
    missing: accepted envelope, current runtime capability
```

A targeted search confirms zero occurrences anywhere in `app-server/src`,
`app-server/docs`, or `app-server/ideas/pending` of "readiness profile,"
"readiness_profile," or this shape. This is not owned by:

- `claim-evidence-service.md` — owns the generic claim/evidence/lineage
  substrate and the freshness *mechanism*, not a sufficiency-for-a-named-
  decision *judgment* layered on top of it;
- `authorized-vertical.mjs`'s `proposal-research-v1` profile — owns claim
  publication mechanics for proposal research specifically, with no
  readiness field;
- `organizational-execution-envelopes.md` (item 3) — consumes an
  organization-qualified projection as an upstream input; does not produce
  the judgment that a proposal has reached that state;
- `ideas/evidence-backed-proposal-evaluation.md` (item 6) — a likely
  **supplier** of the evidence a readiness assessment would read (value,
  risk, complexity, reversibility, maintenance, validation burden,
  confidence), not the judgment of whether that evidence suffices for a
  *particular* decision. Evaluation answers "what do we currently believe
  about this proposal?"; readiness answers "is what we currently know
  sufficient for this particular decision?" These are different questions
  and should not be merged — recorded here as a `SUPPLIES`/`CORRESPONDS`
  relationship for whoever reconciles item 6 next, not resolved by this
  document;
- `proposal-decision-gated-implementation-compilation.md` — owns a later,
  narrower "plan readiness" concept, downstream of a decision already made;
- proposal packets' own `lifecycle_state`/`placement`/`uncertainty` fields
  (confirmed directly in `packet.json` files) — mechanical packet-validation
  states, already correctly distinguished by the idea's own "Current
  evidence" section from evidence sufficiency or readiness judgments.

**Ownership boundary, made explicit rather than assumed:** the idea's own
un-compressed history states readiness is "an attributed semantic
conclusion" that deterministic tooling cannot infer from field presence
alone. A readiness assessment must not become the authority it merely
informs:

```text
research evidence
        |
        v
freshness state
        |
        v
decision-specific requirements (a readiness contract)
        |
        v
attributed readiness judgment
    "the evidence basis is sufficient/insufficient for decision D
     under readiness contract C"
    -- not --
    "perform decision D"
```

A readiness assessment does not acquire portfolio-selection authority,
organizational authority, activation authority, or proposal-acceptance
authority merely by concluding `activation-ready`. Those remain with their
existing or separately queued owners (organizational-execution-envelopes,
decision-gated-compilation, and whichever owner items 6/7 eventually confirm
for portfolio selection).

```text
already owned
    proposal formation (R1)
    placement (R2)
    claim/evidence/freshness mechanism (claim-evidence-service.md)
    evaluation dimensions (R3-shaped; item 6, prospective SUPPLIES relation)
    organizational envelope consumption (R4; item 3)
    authority, runtime realization/admission (R5-shaped)

missing
    decision-specific readiness contracts
        what evidence/facets are required for a particular downstream
        decision
    attributed readiness assessments
        ready / blocked / uncertain, missing requirements, exact
        evidence + freshness basis, without acquiring the decision
        authority it informs

optional derived projection
    R0-R5 coarse maturity summary (not a canonical state machine)
```

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| Cumulative maturity (R0–R5) | **Downgraded, not a core residue.** Each stage decomposes into an existing or separately-queued owner (formation, placement, item 6's evaluation dimensions, item 3's organizational envelope, authority/runtime admission). Survives only as an optional, coarse derived summary label, not a canonical state machine. |
| Readiness is decision-specific (readiness profile) | **Not retired. Primary residue.** Zero occurrences of this shape found anywhere in app-server. An attributed sufficiency judgment for a named decision contract — explicitly not the decision/acceptance/activation authority it informs. |
| Freshness/Refresh mechanism and ownership | Retired — `claim-evidence-service.md`'s impact/refresh pipeline, confirmed by shared design lineage with the idea's own un-compressed history, not merely vocabulary resemblance. |
| Freshness as a readiness input | **Not retired.** A referenced facet of a readiness assessment; a claim's freshness change can invalidate or reopen a previously published readiness assessment without the readiness layer owning refresh itself. |
| Current evidence (packet validation ≠ evidence sufficiency) | Retired as correct — re-verified directly against `packet.json` structure; the idea's own distinction holds. |
| Does not own: shared claim-lineage schema | Retired — confirmed with stronger evidence than a design document: a real, implemented `proposal-research-v1` vertical profile exists. |
| Does not own: organizational compilation / portfolio decisions / continuous monitoring | Retired — each already has a confirmed or independently-queued owner; organizational compilation specifically re-confirmed against item 3's own finding. |
| Does not own: proposal evaluation | Retired as a boundary claim; recorded additionally as a likely `SUPPLIES` relationship (item 6 supplies R3-shaped evidence; this idea's readiness judgment consumes it) for whoever reconciles item 6 next. |

## Recommended status change to the idea file

Update `ideas/proposal-research-maturity-and-freshness.md`'s Status section
to note that this reconciliation exists; that the freshness/refresh
mechanism and ownership are retired to `claim-evidence-service.md` (with the
idea's own archived history at
`ideas/history/2026-08-22-pre-reconciliation/research-maturity-evidence-snapshots-and-staleness.md`
as confirming lineage evidence, not a live dependency), while freshness
remains a referenced input to readiness; that the R0–R5 ladder is downgraded
to an optional derived summary, not a canonical state machine, since each
stage decomposes into an existing or separately-queued owner; and that the
primary remaining open scope is the decision-specific readiness contract and
attributed assessment semantics — an attributed sufficiency judgment for a
named decision, explicitly not the authority it informs — with
`ideas/evidence-backed-proposal-evaluation.md`
(item 6) recorded as its likely evidence supplier, to be confirmed when item
6 is reconciled.

## Acceptance

**Accepted 2026-09-14** (explicit user decision, after a closer-look review
of this reconciliation as part of the sequel queue's acceptance pass). This
document's findings and disposition are confirmed accurate. Implementation
of the stated residue — the decision-specific readiness contract and
attributed assessment semantics — is authorized to proceed.
