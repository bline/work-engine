# Reconciliation: Problem-Derived Organizational Execution Envelopes

## Status

Reconciliation of `ideas/organizational-execution-envelopes.md` against
current app-server implementation and current prospective architecture.
Wave 1, item 3 of the sequel reconciliation queue.

Per instruction: the idea was not improved or modernized. Every clause was
checked against implemented code or an existing prospective-architecture
document. What is already implemented or already owned elsewhere is retired
below. This idea is larger in scope than items 1–2 and does not collapse to
a single tight residue; its central "Execution envelope" concept remains
genuinely unbuilt anywhere. Several surrounding components that looked
unowned on a first pass turned out to be either already decomposed into
existing owners (`RoleRealization`'s available/authorized/required/selected
vocabulary; most of "System environment") or coupled to an owner that has
already recognized and deliberately deferred the same question
(`role-compiler-proposal.md`'s composition/inheritance deferral covers
"Reusable role profile"; its structure/interface separation covers "Skill /
capability contract"). The residue that survives is one new organizational
level sitting above the role compiler, `RoleRealization`, and hierarchical
orchestration — not a restatement of the whole idea, and not a second
compiler stack beside them.

An earlier pass (`app-server/docs/root-ideas-reconciliation.md:287-328`)
placed this idea's layering correctly against `RoleRealization`; this
reconciliation re-verifies that finding directly against source and extends
it to the idea's full semantic-components list and its "Does not own"
boundary claims.

## Idea summary (unchanged)

`ideas/organizational-execution-envelopes.md` proposes treating roles as
reusable, parameterizable organizational functions and skills as reusable
capability machinery, compiling an immutable "execution envelope" (the
organization for one bounded problem) from a system environment, reusable
role profiles, role templates, and skill/capability contracts. It defines a
composition-authority model (available/authorized/required/selected),
states that deterministic assembly has no grant authority, and proposes a
first vertical: compile and explain the topology Work Engine already has,
before attempting dynamic team synthesis.

## What is already implemented

### Agent Environment Graph / skill-compiler (retires role-contract identity; confirms "no profile layer exists")

`docs/agent-environments.yaml` (root substrate, `status: verified_baseline`)
and `app-server/src/skill-compiler.mjs` together are the idea's own cited
"Current evidence." Direct inspection confirms the idea's own
characterization is accurate, not stale:

- `agent-environments.yaml`'s `roles:` section defines each role
  (`role.supervisor`, etc.) with a fully spelled-out `bound_by` invariant
  list (30+ entries for `role.supervisor` alone) and explicit
  `must_require` cross-role relations. There is no profile abstraction —
  no shared, named configuration block that multiple roles reference. Every
  role repeats its own list.
- `app-server/docs/structural-core-ownership.md:17` confirms the ownership
  boundary precisely: "Role identity, label, objective, context lifetime,
  relation names, and relation entry shapes" are owned by the "Agent
  Environment Graph validator," with "Skill-local experimental role
  profile" listed only as a *non-owner projection*.
- `skill-compiler.mjs`'s own `role_profile` field (`role_id`, `label`,
  `objective`, `context_lifetime`, `projection`, `relations`) is a
  **different sense of "profile"** than the idea's: it is the compiled
  shape of *one role*, not a "composable structural configuration shared by
  a declared class of roles" (the idea's definition). This is the same kind
  of vocabulary collision `claim-evidence-service.md` warns about for the
  word "claim" — worth flagging explicitly so a future reader does not
  assume `skill-compiler.mjs`'s `role_profile` satisfies this idea's
  "Reusable role profile" section. It does not.

Disposition: retire "role identity, label, objective, context lifetime" and
"role template" (single-role shape) as already owned by the Agent
Environment Graph / skill-compiler pair — exactly as the idea's own
"Current evidence" section already states. Confirm, not retire, the idea's
own claim that no reusable-profile layer exists: verified directly, true.

### `pre-indexed-capability-resolution-and-frozen-runtime-realization.md` (`RoleRealization`) (retires single-role concrete-binding layer; confirms no multi-role composition layer exists)

Re-verified directly (not just via the prior pass): `RoleRealization` binds
one role's `role_contract`, `policy_overlay_revision`, `provider_turn`,
`harness_runtime`, `tools`, and `context_transition`
(`pre-indexed-capability-resolution-and-frozen-runtime-realization.md` §5).
Its own "Relationship to existing Work Engine boundaries" section (§12)
independently confirms the same boundary this reconciliation needs:

- "The runtime manifest and compiler outputs own role requirements,
  capability and effect ceilings, static role environment, and deploy-time
  inputs" (§717) — per-role, not multi-role org topology;
- "The canonical Agent Environment Graph describes role contracts and
  effective configured environments. Its current baseline intentionally
  omits the live overlay" (§741) — confirms no dynamic, composed,
  provenance-bearing envelope exists, matching this idea's own "no general
  organizational compiler... exists today."

Disposition: retire "runtime binding" (the idea's own term for
provider/process realization) as already owned by `RoleRealization`. The
layering from the prior pass holds under direct re-check:

```text
Agent Environment Graph / skill-compiler (implemented)
    owns: one role_contract's identity/shape
        |
        +--> organizational-execution-envelopes.md
        |       composes MULTIPLE role_contracts + profiles + system
        |       environment into one execution envelope (unowned — see below)
        |
        +--> RoleRealization (prospective)
                materializes ONE role_contract's concrete provider/
                harness/tool binding (owned)
```

This is a `SUPPLIES` relation (`ExecutionEnvelope` → `RoleRealization`), not
competing ownership, exactly as the prior pass found.

### `role-compiler-proposal.md` and `skills-migration-plan.md` (couples "Reusable role profile" and "Skill / capability contract" to an existing deferred seam, rather than leaving them foreign/unowned)

These two documents were not checked in the first pass of this
reconciliation. Direct inspection changes two of the dispositions below.

`app-server/docs/role-compiler-proposal.md` already owns structured
per-role skill sources (`structure.yaml`, `interface.yaml`), shared catalogs
(global invariant/mechanism/capability/state/artifact/relation truth,
referenced by stable identity rather than duplicated), a generated skill
intermediate representation with source identity and provenance preserved
"for every resolved element" (§"Skill intermediate representation"), a
runtime-requirements renderer (required states, capabilities, effect
ceilings, prohibited effects, mediated transitions, context-lifetime
constraints — §"Runtime-requirements renderer"), and an explicit separation
between "structure" (semantic/interface) and "provider realization"
(§"Architectural principles" 2 and 4). Its own "Relationship to environment
projections" section explicitly reuses the Agent Environment Graph as
validation/projection backend rather than creating a second relation-truth
owner — the same boundary this reconciliation already confirmed above.

Critically, its own "Deferred questions" section explicitly names, as *not
yet decided but recognized*, exactly the territory this idea calls
"Reusable role profile":

> whether structure files should support composition or inheritance; how
> role variants are represented; how canonical content blocks may be shared
> across skills without obscuring ownership; which instruction-package kinds
> beyond the initial role profile need explicit structural profiles.

This is not foreign territory the role-compiler direction is unaware of —
it is a question that document already knows it must eventually answer, and
deliberately declines to answer prematurely ("The bootstrap should not
decide these prematurely... The slice-builder decomposition should provide
evidence for these decisions rather than forcing the first schema to
predict them"). A future "Reusable role profile" mechanism cannot be
designed without reconciling it against this owner's eventual answer to
composition/inheritance/shared-fragment representation — building it
independently risks creating the second relation-truth owner the
role-compiler proposal explicitly warns against.

`app-server/docs/skills-migration-plan.md` independently confirms a
classification scheme that already separates "role-like behavior" from
neighboring concerns as a required migration consequence (`:711-712`):
"Role-like behavior is distinguished from agent skill, deterministic
capability, service state, provider adapter, experiment, and projection."
Its own responsibility table (`:218`) already lists "optional role profile"
as one distinguished category within skill structure, alongside "semantic
instructions, objectives, authority meaning, causal explanations" — this is
the migration plan's own placement for exactly the skill/capability
separation this idea's "Skill / capability contract" section asks for,
distinct from "provider deployment choices or server state."

Disposition:

- **Reusable role profile**: reclassify from "unowned" to *unimplemented,
  but a `COUPLED_RECONCILIATION` with an owner that already recognizes the
  question and has explicitly deferred it* — not simply ownerless. Any
  future design here must reconcile with `role-compiler-proposal.md`'s
  eventual answer to composition/inheritance/shared-fragment
  representation rather than invent a parallel mechanism.
- **Skill / capability contract**: reclassify from "not addressed" to
  *substantially supplied, prospectively, by the role-compiler and
  skills-migration classification scheme*. The role-vs-capability
  separation and closed runtime-requirements boundary already have
  prospective owners. Reusable capability *composition at the
  organization level* (assembling several already-separated capability
  contracts into one problem's envelope) may remain an input this idea
  consumes, but is not itself an unowned gap this idea introduces.

### `hierarchical-planning-and-multi-supervisor-orchestration.md` (adjacent, not competing — re-verified, not just asserted)

This idea's "work decomposition vs. role/capability assembly" distinction
was checked directly against `hierarchical-planning-and-multi-supervisor-orchestration.md`
rather than re-asserted. That document's "Does not own" list
(§§ "Owns"/"Does not own" under Preplanner, Orchestrator, Branch planner,
Supervisor) never mentions role profiles, capability contracts, or
system-environment composition. Its "Durable state" section (§16) tracks
"planner identities" and "supervisor identities" as opaque references, not
compiled envelope output — it assumes roles already exist and decides
*which run when and report to whom*, never *what a role is made of* or how
its structural definition was assembled from reusable parts. Its topology
is hard-coded per workflow shape (preplanner/orchestrator/branch-planner/
supervisor/builder), with no reusable-profile mechanism of its own — it
would itself be a *consumer* of an execution-envelope compiler if one
existed, not a substitute for it.

Disposition: confirm, not retire — this remains a `CORRESPONDS`/adjacent
relationship, re-verified rather than merely carried forward from the
earlier pass.

### Decomposing "System environment" and "available/authorized/required/selected" rather than leaving them whole

The first pass of this reconciliation left both of these components alive
in full because no single mechanism implements either one as the idea
describes it. That understates how much of each is already decomposed and
owned elsewhere; only a narrower residue survives once each is broken into
its actual parts.

**System environment** bundles six distinct things (global invariant
references; authority boundaries; context non-authority; common provenance;
globally available machinery; defaults with explicit override authority).
Checked individually:

- global invariant/mechanism references and their resolution: owned by the
  Agent Environment Graph's shared catalogs, now confirmed further owned in
  the compiler direction by `role-compiler-proposal.md`'s "Shared catalogs"
  section (global invariant/mechanism/capability/state/artifact/relation
  truth, referenced by stable identity);
- authority boundaries and effect ceilings: owned by `RoleRealization`'s
  role contract plus operator policy overlay
  (`pre-indexed-capability-resolution-and-frozen-runtime-realization.md`
  §1, "authority grants," "pins") and by
  `role-compiler-proposal.md`'s runtime-requirements renderer (effect
  ceilings, prohibited effects);
- context non-authority (model context is ephemeral, non-authoritative):
  already an established Work Engine-wide invariant, not something this
  idea introduces or must supply;
- common provenance: owned by the skill compiler's IR ("preserve source
  identity and provenance for every resolved element") and by
  `RoleRealization`'s realization-identity/evidence section;
- globally available machinery: owned by the capability inventory
  (`pre-indexed-capability-resolution-and-frozen-runtime-realization.md`
  §"Capability inventory as a dependency graph").

What does not decompose into an existing owner: **shared organizational
defaults or configuration applied across multiple role definitions in one
problem's envelope, with explicit source and override authority at the
organization level** (as opposed to one role's own contract or one
realization's own policy overlay). That narrower object — not the whole
"System environment" section — is what remains in the residue.

**Available / authorized / required / selected** likewise already
substantially exists, but at the *realization* layer, not the
organizational layer:

```text
available    -> capability inventory
authorized   -> role contract + operator policy overlay
required     -> role requirements / effect ceilings
              (role-compiler runtime-requirements renderer)
selected     -> admitted RoleRealization
```

`pre-indexed-capability-resolution-and-frozen-runtime-realization.md`
already separates exactly these as distinct authoritative inputs feeding
resolution and admission into an immutable realization, with receipts
binding requirements, authority/policy, capability observations, and the
selected provider/harness/mechanisms. If this idea's envelope retained an
undifferentiated available/authorized/required/selected state model, it
would quietly reacquire runtime-selection authority this reconciliation
otherwise retires to `RoleRealization`. The surviving, organization-level
version of this vocabulary — distinct from the realization-level version
above — would be:

```text
available organizational components   (which roles/profiles exist to draw on)
authorized organizational composition (what a problem's authority permits assembling)
required organizational consequences  (what this problem's org must guarantee)
selected organizational structure     (the organization chosen or established
                                        under the applicable decision authority)
```

`selected organizational structure` is not the same thing as "whatever the
compiler produced." The idea itself already draws this line: deterministic
assembly may resolve references, apply declared composition rules, verify
mechanically decidable constraints, preserve provenance, and emit immutable
candidate envelopes — it may not invent authority, roles, exemptions,
ownership, or other semantic decisions; judgment and human authority remain
separate inputs. So the *selection* and the *envelope* are distinct:

```text
selected organizational structure
    = the organization chosen or established under the applicable
      decision authority

ExecutionEnvelope
    = the immutable materialization of that selection, plus its
      provenance and role-scoped organizational projections
```

For the idea's own first vertical this distinction is close to trivial,
because the topology being compiled is the one Work Engine already runs —
there may be no open selection judgment at all:

```text
existing authoritative topology
        |
        v
deterministic compilation
        |
        v
ExecutionEnvelope
```

It stops being trivial the moment more than one valid organizational
composition exists for a problem:

```text
requirements + available components
        |
        v
candidate organizations
        |
        v
authorized judgment / selection
        |
        v
ExecutionEnvelope
```

Keeping this distinction explicit is what keeps a future envelope compiler
from slowly becoming an organizational planner. "Selected runtime/
provider/harness/tool realization" stays exactly where it already is:
`RoleRealization`, unaffected by either organization-level state.

## What the idea's own "Does not own" boundary already gets right

Checked each exclusion directly:

- **Control plane** — owned by `role-scheduler` (root) /
  `operator-switchboard.mjs` / `OperatorProjection`, per this queue's item 1
  (`control-plane-and-client-protocol-reconciliation.md`).
- **Runtime adapters** — owned by
  `provider-turn-harness-runtime-and-operator-projection.md`'s
  `ProviderTurnPort`/`HarnessRuntimePort`.
- **Role-owned durable state** — `proposals/agent-state/role-owned-durable-operational-state/proposal.md`
  is a real, formed proposal (confirmed to exist, not fabricated by the
  idea's citation); `app-server/src/context-transition-lease.mjs` is a real,
  already-implemented partial realization of context-transition recovery.
- **Review artifact semantics** — owned by
  `revision-bound-review-artifacts` / `claim-evidence`'s
  `review-finding-bridge.mjs`, per this queue's item 2
  (`review-scope-coordination-reconciliation.md`).
- **Proposal research** — `proposal-research-maturity-and-freshness.md`
  (queue item 5, not yet reconciled) already points at
  `claim-evidence-service.md` per `root-ideas-reconciliation.md:266-274`.
- **Studio UI** — `work-engine-studio.md` (queue item 12, not yet
  reconciled); `root-ideas-reconciliation.md:329-333` already confirmed no
  Studio implementation exists anywhere in app-server.

Disposition: retire this entire section as accurate and already consistent
with every other reconciliation this queue has produced so far. No
correction needed.

## The smallest remaining semantic consequence still lacking an owner

The corrections above narrow the residue considerably from the first pass.
`role-compiler-proposal.md` owns one role's structural source, shared
catalogs, and role-local projection/provenance, with composition/inheritance
explicitly on its own deferred-questions list rather than foreign to it.
`RoleRealization` owns one role's concrete runtime binding, including the
full available/authorized/required/selected vocabulary at the realization
layer. `hierarchical-planning-and-multi-supervisor-orchestration.md` owns
which work branches run and the supervisory execution topology, hard-coded
per workflow rather than compiled. None of the three, individually or
together, answers these questions for one concrete problem:

```text
which existing roles participate?
what are their relationships?
what information/mutation/authority boundaries connect them?
what problem-specific organizational constraints apply?
what exact organizational revision was active?
what role-scoped organizational projection did each role receive?
where did every organizational choice come from?
```

**Residue:** a durable, provenance-bearing, problem-level multi-role
organizational envelope and its role-scoped organizational projections,
initially over Work Engine's already-existing fixed topology (supervisor →
builder, with independent review and deterministic gates, per
`agent-environments.yaml`), without inventing new roles, authority, or
runtime realizations.

This survives as **one new organizational level**, not as a second compiler
stack beside the role compiler:

```text
role compiler (role-compiler-proposal.md)
    owns one role's structural meaning
    + shared catalogs
    + role-local projection/provenance
    (composition/shared-fragments: deferred, not foreign — COUPLED)

RoleRealization
    owns one role's concrete runtime binding
    (available/authorized/required/selected, at the realization layer)

hierarchical orchestration
    owns which work branches run
    and supervisory execution topology

        |
        v  missing
Organizational authority / problem specification
    owns what organization is permitted or required
    and any open selection judgment among valid candidates
    (available/authorized/required/selected organizational vocabulary,
    distinct from the realization-layer vocabulary above)
        |
        v
ExecutionEnvelope
    the immutable materialization of that selection for one problem:
    instantiated multi-role organization + provenance + role-scoped
    organizational projections — not itself the authority that selects it
```

Two things that were treated as part of the residue in the first pass are
now explicitly *not* part of it, to avoid overclaiming:

- **Reusable role profile composition** is not this idea's residue to solve
  independently. It is a `COUPLED_RECONCILIATION`: the envelope needs
  profile composition, and `role-compiler-proposal.md` has already
  recognized (and deferred) the same question. Whoever eventually builds
  either must reconcile with the other rather than build in isolation.
- **Skill / capability contract separation** is not this idea's residue
  either — it is substantially supplied, prospectively, by the
  role-compiler/skills-migration classification scheme (see above). The
  envelope consumes already-separated capability contracts; it does not
  need to invent the separation itself.

Everything beyond the fixed-topology first vertical — dynamic team
synthesis, inventing new roles, synthesizing novel authority, restructuring
active campaigns — remains explicitly self-deferred by the idea's own
"Adoption boundary" section, not retired here as already solved by
something else.

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| System environment | **Partially retired.** Global catalog references, authority boundaries, context non-authority, provenance, and globally available machinery each decompose into an existing owner (Agent Environment Graph / role-compiler catalogs, `RoleRealization`, capability inventory). Only "shared organizational defaults/configuration across multiple role definitions, with explicit source/override authority at the organization level" survives, folded into the residue. |
| Reusable role profile | **Reclassified: `COUPLED_RECONCILIATION`, not unowned.** Unimplemented, but `role-compiler-proposal.md`'s own "Deferred questions" already names composition/inheritance/shared-fragment representation as a recognized, deliberately deferred question. A future design must reconcile with that owner's eventual answer rather than build independently. |
| Role template | Retired — role identity/label/objective/context-lifetime already owned by the Agent Environment Graph / skill-compiler. |
| Skill / capability contract | **Reclassified: substantially supplied, prospectively**, by `role-compiler-proposal.md`'s structure/interface separation and runtime-requirements renderer, and by `skills-migration-plan.md`'s explicit role-vs-agent-skill-vs-capability-vs-service classification. Not this idea's gap; consumed as an input. |
| Problem specification | Not owned by anything inspected, but explicitly an upstream input this idea consumes rather than produces — not this idea's own gap. |
| Execution envelope | **Not retired.** The idea's central concept; no compiler exists. This is the residue: one new organizational level above the role compiler and `RoleRealization`, not a second compiler stack beside them. |
| Role projection | Not owned as an envelope-relative concept (a role's view of the multi-role structure it sits in); `RoleRealization`/`reviewer-projection.mjs` project single-role state, not org structure. Folded into the residue rather than listed separately, since it is only meaningful once an envelope exists. |
| Runtime binding | Retired — owned by `RoleRealization`. |
| Profile composition and configuration authority (available/authorized/required/selected) | **Partially retired.** This vocabulary already exists at the realization layer (`RoleRealization`: available=capability inventory, authorized=role contract+policy overlay, required=runtime requirements, selected=admitted realization). Only the organization-level analogue (available organizational components / authorized organizational composition / required organizational consequences / selected organizational structure) survives, folded into the residue — explicitly distinct from, and must not duplicate, the realization-layer vocabulary. |
| Organizational assembly has no grant authority | **Not a separate gap; preserved invariant.** Any realization of the surviving envelope capability must retain the idea's existing no-grant-authority boundary: deterministic assembly may materialize and validate authorized organization but may not create the authority or unresolved semantic judgment that selects it. Nothing about this is automatic — it must be carried forward explicitly by whoever builds the residue. |
| Upstream prerequisite / provider-neutral packet framing | Consistent with `RoleRealization`'s own "provider choice does not redefine role semantics" framing (confirmed via `hierarchical-planning-and-multi-supervisor-orchestration.md` §15, "Provider realization"). No conflict; not a gap. |
| Adoption boundary | Retired as the idea's own correct self-scoping — re-verified, not re-derived, in this reconciliation. |
| Does not own | Retired in full — every exclusion checked and confirmed to already have a real owner elsewhere in this queue's findings. |

**Smallest remaining semantic consequence still lacking an owner:** a
durable, provenance-bearing, problem-level multi-role organizational
envelope and its role-scoped organizational projections, initially over
Work Engine's already-existing fixed topology, without inventing new roles,
authority, or runtime realizations — one new organizational level, not a
second compiler stack. Reusable role-profile composition is a coupled,
deferred seam with the role compiler rather than part of this residue
directly.

## Recommended status change to the idea file

Update `ideas/organizational-execution-envelopes.md`'s Status section to
note that this reconciliation exists; that its layering against
`role-compiler-proposal.md`, `RoleRealization`, and
`hierarchical-planning-and-multi-supervisor-orchestration.md` is confirmed
(not just asserted); and that its residue is a durable, provenance-bearing,
problem-level multi-role organizational envelope over the already-existing
fixed topology — one new organizational level above those three owners, not
a second compiler stack beside them. Reusable role-profile composition and
skill/capability-contract separation are not part of this residue: the
former is a coupled, deferred seam with the role compiler; the latter is
already substantially supplied by the role-compiler/skills-migration
direction. Dynamic team synthesis remains explicitly out of scope until the
fixed-topology first vertical exists.

## Acceptance

**Accepted 2026-09-14** (explicit user decision, after a closer-look review
of this reconciliation as part of the sequel queue's acceptance pass). This
document's findings and disposition are confirmed accurate. The stated
residue — the execution-envelope compiler — is authorized for **design work
and proposal formation only, not implementation yet**. It is coupled to
`role-compiler-proposal.md`'s own explicitly-deferred composition question;
building the compiler before that question is answered risks creating a
second, competing relation-truth owner, per that document's own warning.
Full implementation authorization is deferred until that design question is
resolved.

## Candidate answer for the organizational-authority layer (proposed 2026-09-15, not yet accepted)

The residue diagram above names an explicit "missing" layer between
existing orchestration and `ExecutionEnvelope`: "Organizational authority /
problem specification — owns what organization is permitted or required and
any open selection judgment among valid candidates." A separate, later idea
document —
`app-server/ideas/pending/deterministic-authority-projection-and-adaptive-organizational-topology.md`,
Part 5.2 ("Three ownership levels for an organizational decision") —
independently arrived at the identical layering while reasoning about a
different problem (adaptive context/vantage topology), then, on reconciling
against this document directly, found the match and is surfacing a candidate
answer here rather than adopting it silently in its own text. That routing
is deliberate: this document is this residue's authorized owner, and
leaving a candidate answer to sit only in the document that happened to
discover it would recreate the exact ownership problem this architecture
exists to prevent.

**The candidate, restated precisely — one correction already applied before
surfacing it (per review), so this is not a same-day draft.** An earlier
version of the source idea's Part 5.2 bundled "organizational authority
admits" and "`ExecutionEnvelope` materializes" into one phrase
("ExecutionEnvelope/workflow authority: admit the organizational
consequence... then the ExecutionEnvelope receives a new revision"). That
reads as the artifact itself being an actor, which contradicts this
document's own framing of `ExecutionEnvelope` as "the immutable
materialization of that selection... not itself the authority that selects
it." Corrected: organizational authority is the authority domain that
admits a selection; `ExecutionEnvelope` only records the consequence.

**Second correction, applied on review before this section's first version
was ever accepted.** The first version of this candidate mapped
`available`/`authorized`/`required`/`selected` onto evidence, constraints,
and decision mechanism directly — conflating properties of an organizational
candidate with the mechanisms used to derive or discover them. Corrected
below. The clean version keeps each of the four vocabulary terms describing
a *property a candidate organization either has or lacks*, never a
mechanism:

- **available** — which organizational realizations can actually be
  *constructed* from currently available role primitives, capabilities,
  runtime realizations, and resources. Not `VantageSeparationEvidence`
  itself — that is evidence bearing on *required*, below, not a definition
  of what is constructible.
- **authorized** — the subset of available realizations the current
  authority ceiling, delegation rules, workflow policy, and effect
  boundaries actually *permit*. Genuine lawfulness constraints only:
  non-transferable authority forbids transfer; an atomic transition forbids
  splitting; workflow policy freezes topology.
- **required** — the properties the organization must *satisfy* for this
  work: independence, continuity, effect separation, semantic obligations,
  capability needs. "Independence requires separation" belongs here, not
  under `authorized` — it is a property the organization must satisfy, not
  a question of whether one lawfully may. Some required properties are
  mechanically established directly; an unresolved remainder may need the
  active semantic role's bounded judgment — but that judgment is a
  *mechanism* for resolving an unresolved requirement, never itself
  identical to "required."
- **selected** — the specific candidate organizational authority actually
  admits. Consistent with, not a departure from, `RoleRealization`'s own
  established gloss of "selected" as "admitted realization" at the
  realization layer — no fifth state is needed to keep "chosen" distinct
  from "materialized." `ExecutionEnvelope` records what gets selected; it
  is never the authority that selects.

**The candidate decision procedure, corrected to match:**

```text
accepted work / current organization
        |
        v
derive candidate organizational realizations
        |
        v
AVAILABLE: filter to realizations constructible from current role
primitives, capabilities, runtime realizations, and resources
        |
        v
AUTHORIZED: filter further to what the authority ceiling, delegation
rules, workflow policy, and effect boundaries actually permit
        |
        v
REQUIRED: derive the properties the organization must satisfy
   -- mechanically known requirements resolve directly
   -- an unresolved remainder, if any, is what VantageSeparationEvidence
      feeds into the active semantic role's bounded judgment
        |
        v
intersect: available (cap) authorized (cap) satisfies(required)
        |
        +-- 0 candidates survive  -> organizational gap
        +-- 1 candidate survives  -> mechanically determined selection,
        |                            no judgment needed at all
        +-- N candidates survive  -> the only place a genuine selection
                                     judgment can actually live
        |
        v
organizational authority admits the surviving candidate
        |
        v
ExecutionEnvelope records the admitted selection as a new revision
   -- the materialization, never the authority that produced it
```

This handles the no-inference case directly: when the intersection reduces
to exactly one candidate, nothing is left to judge. Model-level inference
belongs only at the genuine N-candidate branch, or at deriving an
unresolved required property — never at simply picking among options that
were never actually narrowed.

**Status: proposed, not accepted.** This candidate — corrected once already
before being formally proposed — has not been evaluated against this
document's own acceptance discipline (the same explicit-user-decision
process that accepted the residue itself on 2026-09-14). It is recorded
here so a future decision has a concrete candidate to accept, reject, or
modify — not treated as settled by either document.
