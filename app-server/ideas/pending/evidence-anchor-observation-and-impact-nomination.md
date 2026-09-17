# Idea: Evidence-Anchor Observation and Impact Nomination

**Status:** Architecture idea; not accepted, prioritized, or authorized for
implementation. This supersedes an earlier draft of this idea
("Architecture-Fact Invalidation and Staleness"), which invented a
parallel, architecture-specific staleness system without first checking
whether Work Engine already owned this problem.
[`claim-evidence-service.md`](../../docs/claim-evidence-service.md) is an
*authorized* App Server design target (not an exploratory idea) with an
"Impact, refresh, and reliance propagation" section covering exactly this
ground, and root `ARCHITECTURE.md`'s "Claim-lineage backbone dogfood" proved
its core pattern — stable identity across revision, non-authoritative impact
nomination, both authorized refresh paths, and exact-revision reliance —
against real fixtures. This document narrows to the one piece that design
does not yet own: a deterministic boundary that observes whether a declared
dependency still matches its cited evidence, across evidence sources
including but not limited to codebase-memory-mcp.

**Scope:** A dependency-observation boundary that nominates possible impact
without claiming semantic consequence. It does not own claim lifecycle,
refresh judgment, or reopening — those remain `claim-evidence-service.md`'s.

**Authority:** Exploratory only. This document does not amend the migration
roadmap, admit an implementation, or authorize a claim-evidence contract
change.

```yaml
idea_status:
  architectural_supersession: partial
  superseded_by:
    - view: app-server/docs/architecture/substrates/evidence-anchor.md
      scope: "SS1-3, 8 (AnchorObservation schema, anchor-kind extensibility, five-state comparator, may_affect nomination boundary)"
  residue: present
  residue_ledger: "SS10, KIND: RESIDUE items (5 of 7: items 2, 3, 5, 6, 7) -- see inline tags. 2026-09-16 audit: all 5 confirmed open, 0 answered, 0 unchecked -> residue: present."
  backlog: present
  backlog_ledger: "SS10, KIND: BACKLOG items (2 of 7: items 1, 4) -- locator-choice investigation and shadow-mode review-surface sequencing. Both confirmed open, 0 unchecked -> backlog: present."
  audit_scope:
    - open-question-ledger
    - keyword-scan: full_document
    - close-read: "SS1-9"
  audit_completeness: complete
  status_as_of: 2026-09-16
```

```yaml
idea_provenance:
  origin: direct_capture
  supersedes:
    - "an earlier draft of this idea (\"Architecture-Fact Invalidation and Staleness\"), per this document's own Status line"
```

## Summary

> Work Engine needs a deterministic dependency-observation boundary that can
> nominate evidence-backed impact without claiming semantic consequence.
> Codebase-memory-mcp is one evidence backend for that boundary, not the
> boundary itself.

The corrected pipeline, with three responsibilities kept separate rather
than collapsed into one service:

```text
claim / architecture fact
    owns: "this fact depends on these anchors"
                |
                v
dependency / anchor registry   (owned by the claim, not by whichever
                                 adapter happens to observe it)
                |
                v
evidence-source adapter        (observes current state)
                |
                v
deterministic comparison
                |
                v
may_affect nomination
                |
                v
claim-evidence refresh pipeline   (owned by claim-evidence-service.md,
                                    not by this document)
                |
                v
domain owner judges consequence
```

## 1. Three responsibilities, three owners

- **Dependency ownership** — what does this claim or fact depend on. Owned
  by the claim/fact itself, at declaration time. This was the earlier
  draft's error: it proposed the CBM-backed producer own the anchor
  registry, which would put semantic dependency ownership in the evidence
  source that happens to observe it.
- **Observation** — what is the current state of that dependency. Owned by
  a source-specific adapter (§3).
- **Consequence** — does the observed change alter the claim. Owned by
  `claim-evidence`'s own refresh-episode judgment
  (`claim-evidence-service.md`'s authority table: "Judge refreshed support
  or causality | Authorized claim-domain owner"), not by this document.

Concrete worked example:

```text
Architecture fact:
    "review-subject owns candidate/profile mediation"

Dependency:
    code symbol X
    service contract Y
    architecture direction revision Z

Observers:
    CBM adapter observes X
    service-state adapter observes Y
    document/revision adapter observes Z

Observed:
    X materially changed

Mechanical conclusion:
    may_affect fact F

NOT mechanically established:
    fact F is false
```

That last distinction — mechanical suspicion is never authority to declare
something false — is exactly what `claim-evidence-service.md` already states
for its code-change-analyzer example: "It can identify changed files,
symbols, contracts, tests, and structural reachability. It cannot publish
`changed_because_of`, advance canonical support, or reopen a consumer."

## 2. Anchor kinds — extensible by kind, not one universal shape

The earlier draft collapsed every code-referencing dependency into one shape
(`file_path`, `line_range`, `content_digest`). That is fine as a **text
citation** anchor, but lines move constantly, and some architectural facts
depend on structure rather than exact text — "service implementation
exists," "interface A is implemented by B," "module X calls Y," "symbol S
owns operation O," "dependency path A → B → C." Those are exactly where
codebase-memory-mcp's actual graph (`qualified_name`-based node identity,
`CALLS`/`IMPORTS`/`DEFINES` edges — confirmed present in its schema this
session) becomes valuable, because that identity survives a line shift in a
way a byte range does not.

Two anchor kinds, not one, with more expected to be added as evidence
justifies them:

```text
TextAnchor
    file
    range
    digest

CodeStructureAnchor
    repository/revision
    stable-ish structural locator   (the exact codebase-memory-mcp locator
                                      — qualified_name, node id, or
                                      something else — is future
                                      investigation, not decided here)
    expected relation / observation
```

Plus the two anchor kinds already grounded in existing mechanisms and
unchanged by this correction:

```text
ServiceStateAnchor
    owning_service, identity (claim_id | candidate_id | resource key),
    revision_at_declaration

ImplementationRevisionAnchor
    analyzer_or_service_identity, revision_at_declaration
```

**The anchor contract is extensible by kind.** This document does not
enumerate a final closed set, and does not invent the exact
`CodeStructureAnchor` locator now.

## 3. The `EvidenceAnchorObserver` family — one adapter per anchor kind

```text
EvidenceAnchorObserver
    code           -> codebase-memory-mcp / Git / code-change machinery
    document       -> file revision + digest
    service_state  -> owning service's own revision/CAS read
    implementation -> pinned implementation digest/version
```

Every observer produces the same shape, regardless of source — but "what is
the current state" is not safe to answer with a bare `current_observation`.
This session already surfaced the exact failure mode: codebase-memory-mcp's
watcher tracks live git HEAD and working-tree dirtiness, not an arbitrary
pinned historical commit, while Work Engine's own review/candidate model is
built entirely around immutable, exact, often non-HEAD checkpoints (private
refs, baseline/candidate commit pairs). An observer that answers "what does
the code look like now" without saying *which repository state it actually
looked at* can silently compare a declared dependency against the wrong
world. `AnchorObservation` must carry that binding explicitly:

```text
AnchorObservation
    anchor

    bound_observation
        subject_revision        (the state recorded at declaration time)
        observation

    observed_observation
        requested_subject       (the exact revision the anchor asks about)
        observed_subject        (the exact revision the source actually
                                  answered from)
        source_generation / implementation_revision
        coverage
        observation

    comparison
    evidence
```

This is illustrative, not a final schema. The invariant it exists to enforce:
**an observer must never compare a declared dependency against evidence whose
repository/service/implementation revision is merely assumed to be the
requested one.** For the codebase-memory-mcp adapter specifically: if the
anchor requests repository state `@R42` and codebase-memory-mcp's coverage is
current as of `@R47`, the observer must return `unsupported` /
`unavailable` / `mismatched_subject` — never a comparison computed as if
`R47` were `R42`. This is the same coverage discipline
`service-plane-and-kernel-domain-boundary.md` §10 (refinement 11) already
asks for, applied at the point of observation rather than only at the
coordinate layer.

Each observer is a narrow, deterministic operation in exactly the sense
`service-plane-and-kernel-domain-boundary.md` §3 already defines: determinism
`deterministic`, effect class `observe` only, no authority beyond read access
to its own source. None of them decide impact; §5 owns that.

## 4. The code-evidence boundary specifically: CBM as one backend, not the interface

This is the concrete answer to "maybe wraps CBM." Not a generic API wrapper —
a Work-Engine-owned contract that codebase-memory-mcp happens to implement
today:

```text
before:
    Work Engine queries CBM ad hoc, receives a graph, a model interprets it

after:
                CODEBASE MEMORY
                       |
            structural observations
                       |
                       v
              CodeEvidenceAdapter
                       |
         normalized owned observation (AnchorObservation)
                       |
                       v
                evidence services
```

This is the same move `provider-turn-harness-runtime-and-operator-projection.md`
already makes for provider/harness/UI: an external mechanism sits behind a
Work-Engine-owned port, and Work Engine retains the authority to interpret
what it means. If codebase-memory-mcp is ever replaced, forked, federated
with another graph, or supplemented by a home-grown structural engine,
`CodeEvidenceAdapter`'s consumers do not change — exactly the property that
document's ports already give provider and harness realization.

Confirmed this session, and unaffected by this design: codebase-memory-mcp's
own graph is deliberately read-only (its Cypher engine rejects `CREATE`/
`MERGE`/`SET`/`DELETE` explicitly). `CodeEvidenceAdapter` only ever reads from
it — this design does not ask codebase-memory-mcp to change, host anything,
or become anything other than what it already is.

## 5. `may_affect` nomination — the only claim this boundary makes, from a purely mechanical comparison

An earlier pass at this section said a nomination fires when "comparison
indicates relevant change." That phrasing hides a judgment leak: "relevant"
smuggles semantic materiality back into the observer, which is exactly the
authority this design exists to keep out of it. The comparator does not
decide whether a change matters — the claim author already decided that when
it declared the relationship as a dependency. The comparator only decides
whether the declared relationship still holds:

```text
anchor declaration
    includes mechanically decidable observation/comparator semantics
    (e.g. "relation CALLS(A,B) must exist")

observer
    observes

comparator
    returns exactly one of:
        matches
        differs
        unknown
        unsupported
        failed

declared dependency differs
    -> ImpactNomination(may_affect)
```

`unknown`/`unsupported`/`failed` never produce a nomination by default — they
are exactly the `mismatched_subject`/coverage-gap outcomes §3 requires, and
must be surfaced as such rather than silently treated as `matches`. Only a
mechanically established `differs` produces a nomination. This is what makes
`may_affect` "beautifully weak": it is a report that a declared, mechanically
checkable relationship no longer holds, nothing about whether that matters.

Per `claim-evidence-service.md`'s own authority table: "Nominate possible
impact | Deterministic analyzer or attributed producer" — this observer
family fills that role, generalized from one evidence producer (the
code-change analyzer, for candidate diffs) to a family of them, one per
anchor kind.

**Forming a nomination payload is not the same act as publishing an
authoritative nomination record**, and this document should not blur them:

```text
EvidenceAnchorObserver
        |
        v
AnchorObservation
        |
        v
deterministic nomination derivation   (this document's own work — read-only)
        |
        v
ImpactNomination candidate
        |
        v
claim-evidence admission/publication  (claim-evidence's own work)
        |
        v
durable may_affect record
```

The observer stays read-only all the way through. `claim-evidence` remains
the only place a durable `may_affect` record actually comes into existence —
consistent with its own design, which already treats `may_affect` as a
distinct relationship from a nomination to an exact claim revision, and
already permits non-owning evidence producers (like the code-change
analyzer) to feed it without acquiring claim authority. This document does
not propose a second service for this; it is an ownership boundary worth
stating explicitly, not a new component.

## 6. The sequencing gap: what can be built now, what's blocked

Checked directly against `app-server/src/services/claim-evidence/contract.mjs`
this session: the implemented operation set is `create_claim`,
`publish_revision`, `publish_lineage`, `record_reliance`, `retire_reliance`,
`retract_revision`. There is no `nominate_impact` or refresh-episode
operation yet — the "Impact, refresh, and reliance propagation" section of
`claim-evidence-service.md` is authorized design, proven at the root/Condex
substrate via the dogfood, but not yet built in App Server.

That means:

```text
Evidence observer can exist
        |
        v
Impact nomination contract must land   <- claim-evidence's own work,
                                            not owned by this document
        |
        v
Claim-evidence accepts nomination
        |
        v
Refresh episode determines consequence
```

This document's own §1 correction — dependency ownership belongs to the
claim/fact, not the observer — has a consequence for this section that an
earlier pass missed: a **production** anchor registry is dependency-declaration
state, and dependency declarations are part of a claim's meaning. That state
needs the same kind of durable, revision-bound owner `claim-evidence`
already gives its other typed relationships (stable claim/revision identity,
separately versioned nominations and reliance edges). Putting a real anchor
registry in a side table owned by this observer service would quietly
recreate the exact ownership error §1 removed.

```text
Can be built independently now:
    EvidenceAnchorObserver implementations
    AnchorObservation contract
    shadow observations against explicit test/input anchors

Blocked on an owning dependency-declaration contract:
    production anchor registry
    durable impact nominations
```

Whether dependency declarations eventually live inside a claim revision
itself, or as a separately revisioned dependency artifact bound to an exact
claim revision, is not decided here — only that the registry is not
production-buildable as this observer's own state before that owner exists,
any more than durable nominations are before `claim-evidence`'s contract
grows the corresponding operation. Until both land, observers may run in
**shadow/report-only mode** — producing `AnchorObservation`s against
explicit test/input anchors, not a registry this service owns — without
inventing a competing durable claim-state store. Building a temporary
"architecture staleness database" as a workaround would create exactly the
competing lifecycle ownership this whole correction exists to avoid.

## 7. What narrowed from the earlier draft, and why

Removed or deferred entirely, because they duplicate what
`claim-evidence-service.md` already owns or has already designed better:

- An architecture-specific stale-state machine (the earlier draft's
  `current`/`drifted`/`contradicted`/`orphaned`/`unknown`). Superseded by
  `claim-evidence`'s own refresh-outcome vocabulary — `retained_unchanged` /
  `changed` / `inapplicable` / `insufficient` / `contested` / `deferred` /
  `superseded` — which is richer (it has genuine disagreement (`contested`)
  and claim-relative irrelevance (`inapplicable`), which the earlier draft's
  vocabulary had no way to express) and, per the root dogfood, partially
  proven rather than merely proposed.
- Architecture-specific consequence propagation (the earlier draft's
  "`drifted` converts a seam-map record's `ALIGNED` status to `UNRESOLVED`").
  This may still be a *correct downstream behavior* once a real refresh
  episode judges an architecture fact affected, but it is not this
  document's mechanism to own or mandate.
- Architecture-specific reopen semantics. Per `claim-evidence-service.md`'s
  own authority table, reopening downstream work belongs to "Downstream
  workflow owner" — whichever domain owns the affected work, not a
  standalone invalidation system.
- A duplicated refresh-outcome vocabulary generally. One vocabulary, owned
  by `claim-evidence-service.md`, used by every consumer.

Kept, because they are genuinely new and not owned elsewhere:

- The anchor contract, now extensible by kind (§2).
- The `EvidenceAnchorObserver` family and `AnchorObservation` shape (§3).
- `CodeEvidenceAdapter` as a Work-Engine-owned boundary in front of
  codebase-memory-mcp specifically (§4).
- `may_affect` nomination as the sole, deliberately weak output (§5).

## 8. Architecture facts become one consumer among several

Nothing about this boundary is architecture-specific. Once `claim-evidence`'s
impact-nomination contract exists, architecture facts are simply one
consumer of it, alongside whatever plan-claims, research-claims, or other
domain judgments eventually look like:

```text
                 impact nomination substrate
                           |
            +--------------+--------------+
            v              v              v
     architecture claim   plan claim    research claim
            |              |              |
            +---- domain-owned refresh ---+
```

`service-plane-and-kernel-domain-boundary.md`'s coordinate/service-state map
(§5, and its open "coverage, not just presence" refinement) is a candidate
consumer here, not a reason to build a separate architecture-only mechanism.

## 9. What this document does not decide

- It does not decide the exact `CodeStructureAnchor` locator against
  codebase-memory-mcp's schema — investigation, not invention (§2).
- It does not propose the `claim-evidence` contract change
  (`nominate_impact` or equivalent) that §6 identifies as required before
  nominations can be durably published. That is `claim-evidence-service.md`'s
  own authorized work to specify.
- It does not decide whether architecture facts become a new `claim-evidence`
  profile, a new consumer of an existing one, or something else — only that
  they are a consumer, not a co-owner, of the impact-nomination substrate.
- It does not authorize a shadow-mode implementation, though §6 describes
  one as the only mode available before the sequencing gap closes.
- It does not revise `claim-evidence-service.md`'s own design in any way.
- It does not decide where a production anchor registry's durable state
  lives (inside a claim revision, or as a separately revisioned dependency
  artifact bound to an exact claim revision) — only that it needs the same
  kind of owner `claim-evidence` already gives its other typed relationships
  (§6).
- It does not let the comparator (§5) judge whether a mechanically observed
  difference is semantically important. That authority stays with whoever
  declared the dependency and whoever later runs the refresh episode.

## 10. Open questions

1. [KIND: BACKLOG] [OPEN — `substrates/evidence-anchor.md` names this by its own admission: "investigation, not invention (source §2, §10 open question 1)."] What is the right `CodeStructureAnchor` locator against
   codebase-memory-mcp's actual node identity — `qualified_name` alone, or
   something that also survives a rename?
2. [KIND: RESIDUE] [OPEN — not addressed by the settled substrate, which explicitly does not own consumer-triggering mechanics.] Should `EvidenceAnchorObserver`s be invoked per the same trigger model
   `claim-evidence-service.md` §"Impact, refresh, and reliance propagation"
   already prefers (on-demand, when a real consumer needs a decision), or
   does a family of narrow adapters change that calculus?
3. [KIND: RESIDUE] [OPEN — `substrates/evidence-anchor.md` names this by its own admission: "who authors anchor registry entries, or where that state durably lives — open (source §6, §10 open question 3)."] Who authors the anchor registry entries for a given claim — the claim's
   own producer at declaration time, or a separate authorized step? This
   document assumes the claim/fact owns its dependency declarations (§1) but
   does not specify the mechanism, and §6 leaves open whether that state
   lives inside a claim revision or as its own revisioned artifact.
6. [KIND: RESIDUE] [OPEN — not addressed; closely related to item 3's own open authorship question.] Who authors the mechanically-decidable observation/comparator semantics
   an anchor declaration must include (§5) — is that part of the same
   dependency-declaration act as naming the anchor itself, or a separate
   step with its own authority?
4. [KIND: BACKLOG] [OPEN — an implementation-sequencing question; no shadow-mode review surface has been designed anywhere.] What does shadow-mode output (§6) actually get consumed by, before a real
   `nominate_impact` operation exists — is an inert log sufficient evidence
   to justify building the contract change, or does shadow mode need its own
   minimal review surface?
5. [KIND: RESIDUE] [OPEN — `service-plane-and-kernel-domain-boundary.md`'s own `Coordinate`/`service_state` content is itself one of the capstone's 5 deliberately-deferred architecture items; this question is downstream of that unresolved upstream choice.] Does this boundary's `AnchorObservation` shape want to become part of a
   future `ServiceOperation`/`OwnedOperation` grammar
   (`service-plane-and-kernel-domain-boundary.md` §3) as a standard output
   type, or remain a separate concept?
7. [KIND: RESIDUE] [OPEN — the settled substrate keeps exactly the five-state comparator unchanged (Key Invariant 2); richness remains undecided, and `ai-accessible-browser-seam-reconciliation.md`'s own seam on this question is itself still `UNRESOLVED (deliberately)`.] Is the five-state comparator (§5: `matches`/`differs`/`unknown`/
   `unsupported`/`failed`) too flat? Seam reconciliation against
   `AI_ACCESSIBLE_BROWSER_DESIGN.md`'s independently-designed eight-state
   epistemic vocabulary (`ai-accessible-browser-seam-reconciliation.md`)
   found that vocabulary mixes at least four separate questions (provenance
   class, availability/coverage, temporal/correspondence, semantic
   relationship) that a flat comparator conflates — corroborating evidence
   for a richer state space, not a specific answer. That reconciliation
   explicitly declined to port the browser document's enumeration directly,
   flagging its `contradictory` state in particular as sometimes mechanical
   (a declared relation observed absent) and sometimes irreducibly semantic
   (cross-source disagreement requiring judgment) — exactly the ambiguity
   §5's `differs`/`unknown` split exists to avoid. Any richer comparator
   design should preserve that separation rather than adding a single
   `contradictory` state that blurs it.

## Relationships

| Direction | Relationship |
| --- | --- |
| [`claim-evidence-service.md`](../../docs/claim-evidence-service.md) | Owns claim lifecycle, the refresh-outcome vocabulary, and consequence judgment. This document proposes only the observation/nomination boundary feeding it, and identifies the contract gap (§6) blocking durable use. |
| root `ARCHITECTURE.md`, "Claim-lineage backbone dogfood" | The proven precedent for the core pattern (non-authoritative nomination, authorized refresh, exact-revision reliance) this document generalizes to non-code-change evidence sources. |
| [`provider-turn-harness-runtime-and-operator-projection.md`](provider-turn-harness-runtime-and-operator-projection.md) | `CodeEvidenceAdapter` (§4) is the same port pattern applied to structural code evidence: an external mechanism behind a Work-Engine-owned contract, replaceable without changing consumers. |
| [`service-plane-and-kernel-domain-boundary.md`](service-plane-and-kernel-domain-boundary.md) | Its coordinate/service-state map and open coverage-vocabulary question (§10, refinement 11) are a candidate consumer of the impact-nomination substrate once it exists — not something this document builds a parallel mechanism for. |
| codebase-memory-mcp | One evidence backend behind `CodeEvidenceAdapter` (§4), confirmed read-only and not asked to change. |
| [`AI_ACCESSIBLE_BROWSER_DESIGN.md`](AI_ACCESSIBLE_BROWSER_DESIGN.md), [`ai-accessible-browser-seam-reconciliation.md`](../../docs/ai-accessible-browser-seam-reconciliation.md) | Independently-designed browser-evidence domain whose own claims/epistemic-status machinery was narrowed to defer to this document's `may_affect` boundary; its richer (but not directly portable) epistemic vocabulary is corroborating evidence for open question 7. |
| `incremental-architecture-intake-and-seam-reconciliation.md` | A future architecture-intake session's baseline-verification step (§3 of that method) is a plausible consumer of this boundary once built, exactly as the retired draft of this document proposed — recorded here as a possible future connection, not owned by this document. |
