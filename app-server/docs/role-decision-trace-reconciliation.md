# Reconciliation: Role Decision Trace

## Status

Reconciliation of `ideas/role-decision-trace.md` against current app-server
implementation and current prospective architecture. Wave 1, item 4 of the
sequel reconciliation queue.

Per instruction: the idea was not improved or modernized. Every clause was
checked against implemented code or an existing prospective-architecture
document, following the pattern established by items 1–3 of this queue.

**This document's first draft repeated a mistake `root-ideas-reconciliation.md`
had already corrected**: it quietly turned decision lineage into a would-be
third `claim-evidence` domain profile, the same overreach an earlier pass at
this exact idea made and then corrected
(`app-server/docs/root-ideas-reconciliation.md:100-138`). That correction —
missed on this document's first pass despite being about the same idea file
— is now the starting point rather than a late patch.

## Idea summary (unchanged)

`ideas/role-decision-trace.md` asks for an attributed semantic trace of
observable role judgments (assumptions, exclusions, sufficiency judgments,
decisions later invalidated) distinct from outcome artifacts (proposal
decisions, route revisions, receipts, review findings). It specifies a
decision-record shape (identity, role/actor, work/proposal/slice identity,
decision class, conclusion, confidence, evidence references and cutoff,
assumptions/limitations, relations to earlier decisions, lifecycle status,
downstream consequence), a relation vocabulary (`PREMISE_FOR`, `SUPERSEDES`,
`WEAKENS`, `CONTRADICTS`, `AFFECTS`, `REOPENED_BY`, `CHANGED_BECAUSE_OF`), an
"active decision set" distinct from the full historical trace, and
explicitly disclaims owning a future aggregating "Work Dossier" UI.

## The claim/decision distinction this reconciliation must not re-collapse

`root-ideas-reconciliation.md:100-138` already reached, and this document
now adopts, a specific finding: a claim is an evidence-backed statement
about reality; a decision is a selection or delegation among materially
different routes. These are not the same kind of object, and Work Engine
already treats them as separate first-class concepts:

`app-server/ideas/pending/proposal-decision-gated-implementation-compilation.md`
defines a "material decision set" and a "sealed decision-set revision"
(both confirmed present in that document, re-verified directly here) as a
distinct, versioned, non-claim object. Its own worked example makes the
type distinction concrete (`proposal-decision-gated-implementation-compilation.md:230-240`):

```text
Claim: proposal-packet discovery recursively recognizes every packet.json.
Decision: experimental lineage records may be placed adjacent to the
  proposal family without changing proposal-packet meaning.
Implementation constraint: no experimental artifact may be named
  packet.json, and an integration test must demonstrate discovery
  isolation.
```

And explicitly: "The claims capability caches semantic research questions
and their evidence so the implementation compiler does not repeat broad
repository research. It does not turn recommendations into decisions or
visibility into applicability. The decision set owns route selection."

This claim/decision distinction is useful, but it is scoped to the sealed
decision set specifically and must not be stretched to cover this idea's
full subject. The idea's own purpose is preserving "observable judgments
that materially shape, narrow, interpret, or evaluate the role's decision
space" — which explicitly includes assumptions, interpretations, sufficiency
judgments, placement hypotheses, and confidence changes, not only choices
among materially different routes. For example, "caller B appears
non-authoritative" and "component A likely owns lifecycle" are durable
semantic judgments and may be `PREMISE_FOR` a later route decision, without
themselves being a route selection at all. So the unresolved subject of this
reconciliation is broader than "decisions" in the sealed-decision-set sense:
it is **durable semantic judgment identity and ancestry, of which explicitly
authorized material decisions are one important subtype**:

```text
semantic judgment lineage
    interpretations
    sufficiency judgments
    assumptions
    placement judgments
    material route decisions
        -> sealed decision set
           (when workflow authority requires an explicit route selection)
```

This reframes the sealed decision set's role: it is a narrow existing
consumer/realization of *part* of this domain — the subset of judgments an
authorized owner has sealed into an explicit route selection — not an
obvious candidate owner of the whole judgment trace. It remains the clearest
existing evidence that Work Engine already has a distinct "decision" object
— versioned, sealed, revisioned — that is never expressed
as a `claim-evidence` entry. It is direct evidence against assuming
`claim-evidence` is the natural or only home for judgment/decision lineage,
and it raises a genuine open question this reconciliation does not resolve:
**is durable semantic judgment history a decision-domain use of a more general
revisioned-state primitive, or does the sealed decision-set architecture
already supply the canonical semantic object this trace should grow from?**
That question stays open here, as it was left open in the prior pass.

## What `claim-evidence` genuinely supplies (substrate, not semantic ownership)

`claim-evidence-service.md` / `app-server/src/services/claim-evidence`
supply real, reusable mechanics that a future decision-lineage design would
be foolish to rebuild:

- stable revision identity bound to an exact evidence world;
- evidence references, provenance, and immutable history;
- assumptions, limitations, and confidence as named record fields
  (confirmed in code at `review-finding-bridge.mjs:22-34`'s
  `revisionPayload`);
- exact-revision reliance mechanics (a consumer relies on one exact
  revision and decision scope, never "latest").

But claim-evidence's own scope statement is explicit that this substrate
does not thereby make it the semantic owner of every record shaped like it:
non-goals include "turn[ing] retrieval rank or graph reachability into
applicability" and "convert[ing] a builder's prose into a canonical
statement without verification and publication admission," and its own
domain-profile framing (`claim-evidence-service.md:121-128`) states "Their
domain owners retain materiality, severity, review episode, synthesis,
support, outcome, and decision semantics. The shared service owns only the
common evidence and lineage boundary" — decision semantics are named there
as a *domain owner's* responsibility, not the shared service's.

```text
claim-evidence supplies proven reusable consequences:
    stable revision identity
    evidence references
    provenance
    immutable history
    exact-revision reliance mechanics

but does not thereby become the semantic owner of:
    decision identity
    decision class
    route selection
    decision lifecycle
    decision ancestry
```

Disposition: retire "evidence references and cutoff" and "assumptions and
limitations and confidence" as *fields* whose mechanics are already proven
and reusable — a future decision record does not need to invent its own
versioning, provenance, or immutable-history plumbing. Do **not** retire
"stable decision identity," "decision class," "conclusion," or
"work/proposal/slice identity" as owned by `claim-evidence` — per the
section above, a decision is not a claim, and no evidence was found that
`claim-evidence` intends to become a third domain profile for it. The
relationship is `CORRESPONDS`/`COUPLED_RECONCILIATION` (shared substrate,
open ownership question), not "buildable as a third claim-evidence domain
profile."

## Relation vocabulary: re-evaluated by type signature, not name

The first pass of this reconciliation retired `SUPERSEDES`,
`CHANGED_BECAUSE_OF`, and `REOPENED_BY` because their spelling exactly
matches three of claim-evidence's relation names. That was an error of the
same kind items 1–3 warned against: same spelling does not prove same
semantic relation. Checked by endpoint type instead:

```text
claim-evidence's typed relations:
    claim revision --supersedes--> prior claim revision
    refresh episode --reopened_by--> trigger/event
    refresh judgment --changed_because_of--> adjudicated causal event

the idea's relations (endpoint types not yet defined by any owner):
    decision A --SUPERSEDES--> decision B          ?
    decision A --REOPENED_BY--> evidence/decision/event?  ?
    decision A --CHANGED_BECAUSE_OF--> evidence/decision/event?  ?
```

`claim-evidence`'s `supersedes` relates two revisions of the *same claim
lineage*; `reopened_by` relates a *refresh episode* (a claim-maintenance
process step) to its trigger; `changed_because_of` relates a *refresh
judgment* to a causal event. The idea's `SUPERSEDES`/`REOPENED_BY`/
`CHANGED_BECAUSE_OF` are stated over *decisions*, whose identity, revision
model, and lifecycle have not been established to be the same kind of
object as a claim revision or a refresh episode — that is exactly the
question the previous section leaves open. Reusing these three relation
names for decisions may well turn out to be correct once decision identity
has an owner, but that has to be demonstrated once decisions have a home,
not inferred now from vocabulary identity alone.

Disposition: none of the seven relations (`PREMISE_FOR`, `SUPERSEDES`,
`WEAKENS`, `CONTRADICTS`, `AFFECTS`, `REOPENED_BY`, `CHANGED_BECAUSE_OF`) is
retired. All seven remain open, pending a decision-identity owner against
which their endpoint types can actually be checked. `PREMISE_FOR`,
`WEAKENS`, and `CONTRADICTS` additionally have no name-level match anywhere
in claim-evidence's vocabulary at all (a targeted search of
`claim-centered-evidence-lineage`, `claim-maintenance-and-reliance-propagation`,
and `production-claim-evidence-interface` found zero occurrences of
"contradict," "premise," or "weaken" as relation types), so they are the
clearest evidence that a decision-to-decision reasoning graph is not
already covered by claim-evidence's revision-lineage-only topology
(`refreshes`/`corrects`/`supersedes`/`composes`/`may_affect`/`sourced_by`/
`reopened_by`/`produces`/`changed_because_of`/`relies_on`), whatever becomes
of the other four.

## Active governing-judgment state: un-retired, and not yet even a "projection"

The first pass retired "active decision set" to claim-evidence's
reliance/discovery split, reasoning "a runtime role's active decision set is
what it relies on." That is an inference this reconciliation made, not
something claim-evidence actually establishes. The idea's own text is
broader: the active set contains judgments currently relied upon by the
role, another role, current scope, route, state, a pending gate, finding, or
downstream artifact, with an explicit example of stale premises propagating
through judgment ancestry and changing what remains active. That is a
question about which *judgments* currently govern a role or workflow — not
the same question as which *exact claim revision* a consumer has recorded
reliance on.

A second pass narrowed this to calling it an "unresolved decision-domain
projection," which overstates what has been established. Whether a judgment
is currently governing execution may itself be an authoritative state
transition, not merely a read-only view over something else:

```text
judgment J created
    -> promoted into current reliance
    -> J becomes governing
    -> later evidence weakens J
    -> authorized owner retires/replaces that reliance
```

Only once somebody owns those promote/retire/supersede consequences could
"active set = deterministic projection over current authoritative reliance"
be established. Calling it a projection now would repeat the category
mistake this queue has been eliminating elsewhere: a projection does not
create ownership, and this reconciliation has not found an owner for the
promote/retire/supersede transitions a projection would need to sit over.

Disposition: **not retired.** This is an **active governing-judgment
state — unresolved.** It may ultimately be a deterministic projection over
separately owned exact-revision reliance/promotion/retirement transitions,
or it may need to be owned state in its own right; no owner for either shape
has been demonstrated. Both remain open:

```text
A. active governing-judgment state is owned state directly

or

B. owned promote/retire/supersede transitions
       -> deterministic active-set projection
```

## What the idea's own boundary already gets right

### "Does not own a Work Dossier" — grounded in a concrete example

`app-server/docs/control-plane-causal-observability-ui.md` is a real,
already-accepted (not pending) read-only UI design that aggregates exactly
the kind of causal chain this idea anticipates a future dossier doing
("candidate → generic finding → test remediation," "human decision
boundaries," cost/anomaly causal provenance). It draws on multiple existing
canonical owners rather than declaring itself a new semantic owner
("Project causal meaning from existing owners; do not make the UI a new
owner of workflow truth or authority"). This is the concrete instance the
idea's own "projection problem, not a second semantic owner" claim
predicted, found rather than assumed.

Disposition: retire this section as correct, now with a real example rather
than only a hypothetical future UI.

### Other boundary exclusions (proposal meaning, workflow state, review artifacts, receipts, evidence claims, raw provider/session traces)

Each already has a confirmed owner from this queue's prior items or from
already-read documents: review artifacts →
`revision-bound-review-artifacts`/`review-finding-bridge.mjs` (item 2);
evidence claims → `claim-evidence-service.md` itself; workflow state →
excluded consistently across items 1–3's control-plane and
organizational-execution-envelopes reconciliations. No conflict found.

Disposition: retire this section in full.

## The smallest remaining semantic consequence still lacking an owner

This residue is larger than a strict reading of "smallest" might suggest,
because the correction above removes a false retirement rather than adding
scope, and broadens the subject from "decisions" to "durable semantic
judgments" of which material decisions are one subtype: this remains a real
domain that current claim infrastructure and the sealed decision set only
*partially* and *partially* supply, not a domain either already
substantially owns minus a few edge types.

```text
already supplied by existing architecture (substrate, not ownership)
    revisioned identity mechanics (pattern proven by claim-evidence)
    evidence/provenance fields (pattern proven by claim-evidence)
    immutable history (pattern proven by claim-evidence)
    exact-revision reliance mechanics (pattern proven by claim-evidence)

existing narrow decision precedent (a consumer, not the whole domain's owner)
    sealed material decision set, for the subset of judgments an
        authorized owner seals into an explicit route selection

still distinct / unresolved
    durable semantic judgment identity (is it claim-shaped?
        decision-set-shaped? something else?) — of which explicitly
        authorized material decisions are one important subtype, not
        the whole domain
    judgment class + conclusion/selection
    judgment lifecycle (including whether "contradicted" is a state at all)
    active governing-judgment state (which judgments currently govern a
        role? owned state, or a projection over owned promote/retire/
        supersede transitions — neither owner demonstrated)
    typed judgment ancestry, all seven relations, endpoints undefined:
        PREMISE_FOR
        SUPERSEDES
        WEAKENS
        CONTRADICTS
        AFFECTS
        REOPENED_BY
        CHANGED_BECAUSE_OF
```

The open architectural question, preserved rather than resolved: **what owns
durable semantic judgment history, of which explicitly authorized material
decisions are only one class?** Is it a decision-domain use of a more
general revisioned-state primitive (the same "revisioned owned state +
admitted transition + fencing + authoritative successor" pattern already
observed independently in `claim-evidence`, `review-episode`,
`slice-campaign`, and decision-gated-compilation's own decision set), or
does the sealed decision-set architecture already provide the canonical
semantic object this trace should grow from — bearing in mind that the
sealed decision set only covers the route-selection subtype, not
interpretations, sufficiency judgments, assumptions, or placement judgments?
Neither this
reconciliation nor the prior pass it corrects decides that question.

On `contradicted` specifically: this reconciliation does not assume it is a
lifecycle *state* of one judgment at all. It may instead be a derived
condition read off a `CONTRADICTS` edge plus whatever adjudication/lifecycle
semantics a future judgment owner defines — that is left open, not resolved
toward either interpretation, and it is not the same thing as
claim-evidence's `contested` refresh outcome regardless of which
interpretation eventually wins.

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| Stable decision identity | **Not retired; broadened.** A judgment is not a claim (see claim/decision distinction above); no owner demonstrated. Reframed as "durable semantic judgment identity," of which material decisions are one subtype, not the whole subject. |
| Role and logical actor | Retired as a *field pattern* — producer attribution is a proven, reusable mechanic from `claim-evidence`, without claim-evidence owning judgment identity itself. |
| Work/proposal/slice identity, decision class, conclusion | **Not retired.** Removed from "coupled to a third claim-evidence domain profile" — no evidence found that claim-evidence intends to host judgments or decisions. Open, pending a judgment-identity owner. |
| Confidence, evidence references and cutoff, assumptions and limitations | Retired as *field/mechanic patterns* only — proven and reusable, not evidence that claim-evidence owns judgments or decisions. |
| Lifecycle status (active/stale/superseded/contradicted/resolved) | **Not retired.** `contradicted` is preserved as unresolved — not assumed to be a lifecycle state, and not mapped to claim-evidence's `contested`. |
| All seven relations (`PREMISE_FOR`/`SUPERSEDES`/`WEAKENS`/`CONTRADICTS`/`AFFECTS`/`REOPENED_BY`/`CHANGED_BECAUSE_OF`) | **Not retired.** Re-evaluated by endpoint type, not name. Three have exact-name but not exact-type matches in claim-evidence; three have no match at all; one (`AFFECTS`) has a differently-shaped near-match (`may_affect`). None safely retired without a judgment-identity owner to check endpoint types against. |
| Active decision set | **Not retired; reclassified.** Renamed "active governing-judgment state" — not called a projection. Whether a judgment is currently governing may itself be an authoritative promote/retire/supersede transition; no owner demonstrated for either that state directly or the transitions a projection would need to sit over. |
| Does not own a Work Dossier | Retired — confirmed correct, now grounded in `control-plane-causal-observability-ui.md` as a real example. |
| Other boundary exclusions | Retired — each already has a confirmed owner from this queue or from `claim-evidence-service.md` itself. |

**Result:** the old decision-trace idea does not survive as a proposal for a
new database or a new claim-evidence domain profile. But durable semantic
judgment history — of which explicitly authorized material decisions (the
sealed decision set) are only one subtype — remains a real, distinct domain
that current claim infrastructure and the sealed decision set only partially
supply (proven substrate patterns and one narrow consumer, not
ownership), with a genuinely open architectural question — general
revisioned-state primitive vs. the existing sealed decision-set
architecture — left for whoever takes this up next.

## Recommended status change to the idea file

Update `ideas/role-decision-trace.md`'s Status section to note that this
reconciliation exists, pointing to `root-ideas-reconciliation.md:100-138`
(the claim/decision distinction), `proposal-decision-gated-implementation-compilation.md`
(the existing, narrower-scoped "sealed decision set" precedent — a consumer
of part of this domain, not its owner), and `claim-evidence-service.md`
(proven substrate patterns, not a semantic owner). The remaining open scope
— durable semantic judgment identity (material decisions being one
subtype), judgment class, judgment lifecycle, active governing-judgment
state, and all seven ancestry relations with endpoints still undefined —
plus the open architectural question of whether judgment history belongs on
a general revisioned-state primitive or grows from the existing sealed
decision-set architecture, should be the only thing left active in that
idea file going forward.
