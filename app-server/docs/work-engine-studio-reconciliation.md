# Reconciliation: Work Engine Studio

## Status

Reconciliation of `ideas/work-engine-studio.md` against current
implementation, `app-server/ideas/pending/`, `app-server/docs/` (accepted
designs, not only pending ideas), and this session's own prior
reconciliations. Final item of the sequel reconciliation queue — closes the
entire 12-item, 4-wave queue Sol proposed.

Per instruction: the idea was not improved or modernized. Because Studio is
explicitly a presentation layer over nearly everything else this queue has
touched, this reconciliation leans more than any prior item on connecting to
already-completed work rather than independent investigation — checking
every relevant reconciliation this queue has already produced before
treating any of Studio's five views as unowned.

## Idea summary (unchanged)

`ideas/work-engine-studio.md` asks for a human-facing environment providing
five views — Contract/design (stable role/capability contracts, invariants,
ownership, mediation), Organization (the execution envelope and each role
projection), Runtime (provider/runtime realization, bindings, waiting
conditions), Control (only authorized controls), and Forensics (replay of
envelope revisions, workflow transitions, decisions, findings, evidence
changes, runtime events, checkpoints/receipts) — plus a "Design-time
diagnostics" list (missing obligation coverage, authority leaks, observation
gaps, ownership gaps, independence conflicts, redundant/orphan machinery,
unreachable required consequences, declared-vs-realized enforcement
mismatch). Its own boundary is explicit: Studio is never an alternate source
of truth; it renders and edits only through canonical owners and
authority-controlled transitions, and does not own role contracts,
execution-envelope semantics, control-plane authority, runtime execution,
workflow state, or proposal decisions.

## What a prior pass found, and what it necessarily missed

`root-ideas-reconciliation.md:329-333` found: "`operator-switchboard.mjs` is
a narrow CLI attachment/binding-view parser, nowhere near Studio's five
proposed views... No Studio UI, replay surface, or diagnostics engine found
anywhere in app-server. Genuinely unaddressed." This was accurate for what
it checked, but that pass could not have found
`app-server/docs/control-plane-causal-observability-ui.md` relevant context
from this session's own later work (items 8–9's investigation), and did not
check it. Re-checked directly here, this materially changes the picture.

## The central finding: an already-accepted design substantially overlaps Studio, with zero cross-reference either direction

`app-server/docs/control-plane-causal-observability-ui.md` is not a pending
idea — it is an accepted App Server design document ("a product design and
architecture boundary, not an implementation plan or authority grant") with
a five-truth-class model (Observed / Mechanically derived / Semantic
interpretation / Counterfactual estimate / Recommendation), a causal graph
over exact identities (campaign/run → slice attempt → candidate → episode →
findings → checkpoint → completion), an explicit owner-join table (Slice
Supervisor, Slice Checkpoint, Independent Review State, Provider events,
Role Scheduler, Claim Evidence, Code Change Profile, Runtime Manifest), five
primary views (Operational, Causal timeline, Efficiency, Anomaly
explanation, Counterfactual, Recommendation), and phased delivery slices
(UI-0 through UI-4).

Checked directly and confirmed: **neither document references the other.**
`control-plane-causal-observability-ui.md` never mentions "Work Engine
Studio"; `ideas/work-engine-studio.md` never mentions this design. This is
the same failure mode items 8–9's `COUPLED_RECONCILIATION` flag existed to
catch, one step worse: there, two *pending* ideas risked independent
re-design; here, an *already-accepted* design and a still-*exploratory*
idea substantially overlap without either acknowledging the other, meaning
Studio's own text risks re-specifying territory that already has an
authorized, more rigorous design.

The overlap, checked view by view:

```text
Studio's Runtime view (binding, loaded/active state, current turn, runtime
    descendants, capability availability, waiting conditions)
    -> control-plane-causal-observability-ui.md's Operational view
       (active runs/slices, phase, builders/reviewers by role/provider/
       model/generation/session, waiting capabilities, quota/retry/
       schedule state) -- near-total overlap, and the accepted design is
       materially more precise (exact identity joins, explicit evidence
       classes) than Studio's own text specifies

Studio's Forensics view (replay envelope revisions, workflow transitions,
    decisions, findings, evidence changes, runtime events, checkpoints/
    receipts)
    -> control-plane-causal-observability-ui.md's causal graph + Causal
       timeline view covers the EXECUTION-HISTORY half precisely (typed
       edges: caused/supersedes/reviews/remediates/re-evaluates/replaces/
       waits-on/scheduled-as/supports/limited-by) -- but Studio's own
       uncompressed history frames Forensics as also comparing design
       intent with runtime behavior, a design/organization <-> runtime
       correlation the accepted design does not cover (see below); narrowed,
       not fully retired

Studio's Control view (only authorized controls)
    -> control-plane-causal-observability-ui.md's UI-1 slice explicitly
       defers this: "Begin read-only; invoke existing authority-bound
       controls only in a later slice" -- gestured at, not designed;
       genuine remaining overlap-adjacent gap, not a retirement

Studio's Contract/design view, Organization view
    -> NOT covered by control-plane-causal-observability-ui.md at all --
       that design is entirely about workflow EXECUTION (campaigns,
       slices, candidates, reviews), never about static role/capability
       CONTRACT structure or the execution envelope
```

Disposition: retire "Runtime view" fully, and "Forensics view" for its
*execution-history* scope, as substantially already covered by an accepted
design more rigorous than Studio's own text — not built, but authorized and
materially further along than "genuinely unaddressed" implied.

Forensics is narrowed rather than fully retired. The idea's own uncompressed
history frames Forensics mode as answering "what happened, why did it
happen, and what did the agent's environment look like when it happened,"
including "compar[ing] design intent with runtime behavior" — a cross-layer
join between design/organization state (a role-contract revision, an
execution-envelope revision) and runtime execution history, not only
execution history in isolation. `control-plane-causal-observability-ui.md`'s
causal graph is excellent, precise execution history (campaign → slice →
candidate → episode → findings → checkpoint), but it does not join that
history against role-contract or execution-envelope revisions — that
correlation depends on item 3's own execution-envelope compiler and
revision identity existing first, which item 3's reconciliation already
found unbuilt. This is not a second forensics engine to build; it is a
consumer-side extension over canonical histories that do not fully exist
yet, following the same shape as the Organization-view dependency below.

```text
execution/runtime forensics
    -> retired to control-plane-causal-observability-ui.md

design/organization -> runtime historical correlation
    ("compare design intent with runtime behavior")
    -> a future composition over item 3's execution-envelope revisions and
       the accepted causal-observability history, not a separate forensic
       owner; blocked on those canonical histories existing, not a new gap
```

The immediate actionable consequence is a documentation fix neither document
has made: `ideas/work-engine-studio.md` should point at
`control-plane-causal-observability-ui.md` as the accepted design for its
Runtime view and the execution-history half of Forensics, and that design
should note Studio as the broader product surface it sits inside, so neither
is developed twice independently.

## The idea's own un-compressed history, checked directly

`ideas/history/2026-08-22-pre-reconciliation/work-engine-studio-design-control-forensics.md`
is a 785-line pre-compression source for this exact idea (the same pattern
found for items 5, 6, and 10). It is not itself a live idea to reconcile
separately, but three passages from it directly sharpen this reconciliation
and were checked word-for-word against the archived text before use:

- "The UI does not decide what is authorized. It projects the existing
  authority model" (line 298) — grounds the Control-view correction below.
- "Edits should write back to the structured owners or produce reviewed
  change proposals" (line 727) — grounds the Contract/design split below.
- "What happened, why did it happen, and what did the agent's environment
  look like when it happened?" and "compare design intent with runtime
  behavior" (lines 577, 683) — grounds the Forensics-scope correction below.

## Contract/design view: split into a read projection (retired) and an authoring path (not retired)

### Read projection → Agent Environment Graph / `role-compiler-proposal.md`

Confirmed via item 3's own investigation this session: the Agent Environment
Graph already generates per-role structural views (role identity, label,
objective, context lifetime, relations), and `role-compiler-proposal.md`
already proposes a compiler pipeline rendering those into projections. This
is real, existing machinery — not packaged as an interactive UI, but the
underlying data and generation pipeline Studio's Contract/design *read* side
would render already exist.

### Authoring path — not retired; the idea's own product consequence is bidirectional

The first pass of this reconciliation retired the entire Contract/design
view on the strength of the read side alone. The idea's own uncompressed
history is explicit that Contract/design also includes creating/editing role
environments, assigning capabilities, binding invariants, defining
mutation/mediation boundaries and prohibitions, and generating structured
configuration — with edits writing "back to the structured owners or
produc[ing] reviewed change proposals." No document inspected anywhere in
this queue specifies that path: how an operator's edit becomes a candidate
structural change, passes validation, reaches the applicable authority/
decision boundary, and is admitted as an owner revision.

```text
Contract/design read projection
    -> substantially supplied by Agent Environment Graph + role compiler

Contract/design authoring path
    -> not retired: operator edit
           -> candidate structural change
           -> validation
           -> applicable authority/decision boundary
           -> admitted owner revision
```

This does not make Studio an owner of role contracts — the idea's own
boundary already forbids that, and this reconciliation does not weaken it.
The missing piece is the authority-preserving edit/admission path from UI
intent to the canonical owner, not a second contract store.

### Organization view → item 3's own still-open residue, not a new gap for Studio to solve

`ideas/work-engine-studio.md`'s "Organization view" field list — instantiated
roles, ownership/delegation, capability selections, information flow,
mutation boundaries, configuration provenance, baseline-versus-effective
differences — is, almost verbatim, item 3's (`organizational-execution-envelopes.md`)
own "Execution envelope" field list. Item 3's reconciliation already
established that no execution-envelope compiler exists anywhere, even for
the idea's own minimal first vertical. Studio's Organization view cannot be
built before that compiler exists to render — this is item 3's residue
surfacing again as a consumer-side dependency, not a second, independent gap
this reconciliation is finding for the first time.

## Control view: narrower than the first pass claimed — Studio does not own which controls exist or their authorization

Checked against `control-plane-and-client-protocol-reconciliation.md` (item
1) and `provider-turn-harness-runtime-and-operator-projection.md`'s
`OperatorProjection` (§4): both already supply *which operations exist* and
*whether they are currently authorized*. `OperatorProjection` presents
Work Engine-owned state, accepts bounded operator requests, may expose
administrative commands, and is explicit that "UI controls submit commands
or proposed overlay revisions to Work Engine; they do not directly mutate
active adapters." The idea's own uncompressed history states the same
principle even more directly: "The UI does not decide what is authorized.
It projects the existing authority model" — and describes a control surface
that becomes "authority-aware by construction" by deriving which actions to
expose from the environment it observes, not by making its own authorization
judgment.

The first pass of this reconciliation defined the open Control residue as
"which controls should be exposed, how a projection determines current
authorization" — that overclaims what Studio itself is missing. Those two
questions belong to the canonical control plane and `OperatorProjection`,
which already own them conceptually (though not yet built as an interactive
surface). What Studio is actually missing is narrower:

```text
canonical owner / control plane
    owns: what operations currently exist, subject + revision binding,
          authority requirement, admission/refusal, the resulting
          authoritative transition

Studio
    owns: discovering and rendering available operations, collecting
          bounded operator intent, submitting it without manufacturing
          authority, and showing proposed / pending / admitted / refused /
          completed / stale lifecycle feedback with attributable reason
```

This is the same missing piece as the Contract/design authoring path above
— not two parallel UI-mutation systems, one for runtime controls and one for
structural edits, but one **authority-preserving interactive command/edit
projection**:

```text
owned state
    |
    v
discoverable projections + admitted operations
    |
    v
Studio: display / edit / request
    |
    v
identity + revision + authority-bound intent
    |
    v
owning transition
    |
    v
authoritative result
    |
    v
Studio feedback
```

No document inspected in this queue, pending or accepted, specifies this
connective layer — not the "which controls exist" question (already owned
elsewhere), but the discovery/rendering/submission/lifecycle-feedback
mechanism itself. This is the genuinely Studio-specific missing piece.

## Design-time diagnostics: a consumer of items 8–9's specialists, not new diagnostic logic to invent

The idea's "Design-time diagnostics" list — missing obligation coverage,
authority leaks, observation gaps, ownership gaps, independence conflicts,
redundant/orphan machinery, unreachable required consequences,
declared-vs-realized enforcement mismatch — maps closely onto the joint
seam-review/architectural-review reconciliation's own vocabulary:
architectural review's "suspected ownership, placement, or decomposition
defects; affected contracts or invariants" (ownership gaps, authority
leaks, enforcement mismatch); cross-cutting seam review's boundary-mismatch
findings (redundant/orphan machinery, unreachable required consequences,
observation gaps). Studio does not need to invent this diagnostic logic —
it is the presentation surface for findings items 8–9's specialists would
produce, once those specialists exist. Neither is built; this is a
`SUPPLIES` relationship recorded for continuity, not a resolution.

## What the idea's own boundary already gets right

"Does not own" (role contracts, execution-envelope semantics, control-plane
authority, runtime execution, workflow state, proposal decisions) and
"Boundary" ("Studio is never an alternate source of truth... renders and
edits through canonical owners and authority-controlled transitions") are
retired as correct and consistent with every non-authority framing this
queue has established: `claim-evidence`'s reliance/discovery split, item 5's
readiness-is-not-authority boundary, item 6's comparison-is-not-authority
boundary, `control-plane-causal-observability-ui.md`'s own identical framing
("do not make the UI a new owner of workflow truth or authority").

## The smallest remaining semantic consequence still lacking an owner

Decomposed rather than treated as one residue:

```text
already substantially covered by an accepted (uncross-referenced) design
    Runtime view (fully); Forensics view's execution-history half
        -> control-plane-causal-observability-ui.md

already real, packaging task over existing data
    Contract/design READ projection
        -> Agent Environment Graph / role-compiler-proposal.md

blocked on an already-identified dependency's own residue, not a new gap
    Organization view -> item 3's still-unbuilt execution-envelope compiler
    Forensics view's design/organization<->runtime correlation half
        -> also blocked on item 3's execution-envelope revisions

a consumer relationship, not new logic to invent
    Design-time diagnostics -> items 8-9's architectural/seam-review
        specialists, once built

genuinely unaddressed anywhere, even as a gesture -- one connective layer,
not two
    authority-preserving interactive command/edit projection: discovery,
        rendering, bounded-intent submission, and proposed/pending/
        admitted/refused/completed/stale lifecycle feedback -- covers both
        the Control view AND the Contract/design authoring path. The
        canonical control plane and OperatorProjection already own WHICH
        operations exist and their authorization; Studio's own missing
        piece is only the projection/interaction mechanism itself

a real, concrete documentation gap this reconciliation found directly
    Work Engine Studio and control-plane-causal-observability-ui.md were
        each written with zero awareness of the other, despite
        substantially overlapping in Runtime and part of Forensics
```

The single smallest, most concrete finding: the missing cross-reference
between `ideas/work-engine-studio.md` and
`app-server/docs/control-plane-causal-observability-ui.md`. The one
genuinely open design question is the authority-preserving interactive
command/edit projection — narrower than a full Studio, unifying what looked
like two separate gaps (Control view, Contract/design authoring), and not a
new architectural level unlike item 3's own execution envelope.

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| Runtime view | Retired — substantially covered by `control-plane-causal-observability-ui.md`'s Operational view, an accepted design more precise than Studio's own text. |
| Forensics view: execution-history half | Retired — substantially covered by the same design's causal graph and Causal timeline view. |
| Forensics view: design/organization ↔ runtime correlation half | **Not retired.** Blocked on item 3's execution-envelope revisions existing; a future composition, not a second forensics engine. |
| Control view | **Reclassified, narrower.** Studio does not own which controls exist or their authorization (already owned by the control plane / `OperatorProjection`); Studio's own missing piece is the discovery/rendering/submission/lifecycle-feedback mechanism, unified with the Contract/design authoring path below. |
| Contract/design view: read projection | Retired as a packaging task — Agent Environment Graph / `role-compiler-proposal.md` already generate the underlying data. |
| Contract/design view: authoring path | **Not retired.** The idea's own uncompressed history is explicit this is bidirectional ("edits should write back to the structured owners or produce reviewed change proposals"); no admission path specified anywhere. Unified with Control view above as one authority-preserving projection. |
| Organization view | **Not a new gap.** Blocked on item 3's own still-unbuilt execution-envelope compiler; matches item 3's field list almost verbatim. |
| Design-time diagnostics | **Not new logic to invent.** A consumer of items 8–9's architectural/seam-review specialists, once built. |
| Does not own / Boundary | Retired as correct — consistent with every non-authority framing this queue has established, including the idea's own "the UI does not decide what is authorized; it projects the existing authority model." |
| Missing cross-reference to `control-plane-causal-observability-ui.md` | **Not retired. The concrete, actionable finding of this reconciliation.** Neither document currently acknowledges the other. |

## Recommended status change to the idea file

Update `ideas/work-engine-studio.md`'s Status section to note that this
reconciliation exists; that its Runtime view and Forensics view's
execution-history half are substantially already covered by the accepted
(but previously uncross-referenced)
`app-server/docs/control-plane-causal-observability-ui.md`; that its
Contract/design view splits into an already-real read projection (Agent
Environment Graph / `role-compiler-proposal.md`) and a still-unspecified
authoring path; that its Organization view and Forensics' design-correlation
half are both blocked on item 3's own still-unbuilt execution-envelope
compiler, not separate gaps; that its Design-time diagnostics are a consumer
of items 8–9's architectural/seam-review specialists once built, not logic
Studio itself must invent; and that its Control view and Contract/design
authoring path converge on one genuinely open design question — an
authority-preserving interactive command/edit projection (discovery,
rendering, bounded-intent submission, lifecycle feedback) — the only
architecturally open piece remaining anywhere in this queue. Recommend
adding a reciprocal note to `control-plane-causal-observability-ui.md`
itself, pointing back at Studio as the broader product surface it sits
inside.

## Closing note for the queue

This closes the 12-item, 4-wave sequel reconciliation queue Sol proposed.
Across all thirteen idea files (two reconciled jointly in Wave 3), the
recurring shape has been: real substrate already exists or has already been
designed more often than the original ideas' own "Current evidence"
sections credited, and in every case but one (item 10,
`agent-instruction-structure-and-placement-review.md`, already fully built
and dogfooded) a narrow, precisely-placed residue survives — never the
whole original idea, and only once (item 3,
`organizational-execution-envelopes.md`) does that residue amount to a
genuinely new architectural level rather than a coupling to, or a missing
connective step between, capabilities this queue has already found real.

Studio itself compresses to almost the purest expression of that
architecture: it owns almost no domain semantics of its own. It reads
canonical structural state, renders canonical runtime state, submits bounded
intent to canonical owners, shows authoritative transition results, and
joins canonical histories for investigation. The UI is not another system —
it is a projection and interaction boundary over the system that already
exists.

## Acceptance

**Accepted 2026-09-14** (explicit user decision, after a closer-look review
of this reconciliation as part of the sequel queue's acceptance pass). This
document's findings and disposition are confirmed accurate. The stated
residue — the authority-preserving interactive command/edit projection
(discovery, rendering, bounded-intent submission, lifecycle feedback) — is
authorized for **design work only, not implementation yet**, pending that
connective-layer schema being specified.
