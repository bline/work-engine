# Idea: Service Plane and the Kernel/Domain Boundary

**Status:** Architecture idea; not accepted, prioritized, or authorized for
implementation. This document has been through steps 3–4 (seam map,
reconciliation) of its bounded intake session — see
[`service-plane-reconciliation.md`](../docs/service-plane-reconciliation.md),
reviewed and corrected before being carried forward here. Sections below marked
"(reconciled)" reflect that pass; the rest of this document predates it.

**Scope:** How Work Engine hosts and composes owned capabilities across
domains — not what any one domain (currently: software engineering) needs.

**Authority:** Exploratory only. This document does not amend the migration
roadmap, admit an implementation, rename anything in the codebase, or
authorize a refactor of any existing directory.

```yaml
idea_status:
  architectural_supersession: none
  architectural_supersession_note: "Confirmed: no canonical view under app-server/docs/architecture cites this document. Its own Coordinate/service_state content is explicitly named as one of work-engine-planned-architecture.md's 5 deliberately-deferred architecture items -- deferred, not absorbed."
  residue: present
  residue_ledger: "SS10 (11 'still open' items, batch-tagged KIND: RESIDUE, OPEN) -- already confirmed open by this document's own reconciliation pass. 3 other items in the same section are marked resolved and folded into SS1-9, not part of the open ledger."
  backlog: none
  backlog_note: "No dedicated staged-plan section; this document is a reconciliation-stage architecture direction, not an implementation rollout."
  audit_scope:
    - open-question-ledger
    - close-read: "SS1, SS10"
  audit_scope_completeness: partial
  audit_scope_completeness_note: "SS1-9 (the reconciled architectural content itself) were not close-read section by section this pass, only SS10's own ledger and the document's header."
  status_as_of: 2026-09-16
```

```yaml
idea_provenance:
  origin: direct_capture
  related_reconciliations:
    - app-server/docs/service-plane-reconciliation.md
```

## Summary

Work Engine should generalize the *way* it hosts and consumes services, not
the domain vocabulary those services carry. Concretely:

> **Generalize the envelope, not the payload.**

`CandidateTrajectory`, `CodeChangeProfile`, `ValidationDependencyMap`, and
`NativeReviewBoundary` should stay exactly as code-specific as they need to
be. What should become general is what Work Engine knows *about* a service
operation, independent of what domain it belongs to — its owner, its
determinism class, its authority requirement, its evidence shape, its
recovery semantics.

This is not a speculative generalization in search of imagined future
domains. [`service-plane-inventory.md`](../docs/service-plane-inventory.md)
found that the current implementation already needs this distinction to
describe itself accurately: `slice-campaign` alone turned out to be at least
six distinct units (campaign state, native-review coordination, capability
dispatch, legacy adapters, completion publication, and strategic
reconciliation) sharing one directory name, with different owners,
authorities, and lifecycles. The directory structure is no longer a reliable
representation of the service architecture. That is not a failure — it grew
organically — but it means the boundary this document proposes is diagnostic
of something real, not aspirational.

## 1. Three planes, not four (reconciled)

An earlier pass at this idea proposed four planes (kernel, service, workflow,
domain). That overstated domain's role. The inventory supports three planes,
with domain as a property of a service rather than a fourth execution tier.

Reconciliation found a second, orthogonal boundary that is easy to conflate
with a fourth plane but isn't one: these three planes describe **architectural
responsibility within production, governed execution**. `review-bench`
(offline evaluation tooling, explicitly carrying no production authority)
fits none of the three planes, but that is a **scope** question, not a
responsibility question — an evaluation system could itself someday have
internal kernel/service/workflow structure. The three-plane split applies
inside a boundary; auxiliary environments sit outside it entirely:

```text
production / governed execution substrate
    KERNEL, SERVICE, WORKFLOW

out-of-band / auxiliary environments
    evaluation tooling, developer tooling, migration tooling, diagnostics, ...
```

```text
KERNEL / CONTROL
    identity, revisions, authority, fencing, history, admission, branching

SERVICE PLANE
    owned operations: state ownership, deterministic derivation, effects,
    projections, semantic realizations, evidence

WORKFLOW PLANE
    composes operations and sequences judgments toward an objective

domain = ownership/semantic namespace of a service, not another plane:

    domain: code       service: review-subject
    domain: runtime     service: reviewer-runtime
    domain: kernel      service: workspace-coordination
```

Nothing here requires code and a future research domain to share vocabulary.
They share the surrounding grammar (owner, revision, authority, input,
output, evidence, effect, determinism, history, recovery). A future research
domain could introduce `DatasetSnapshot`, `ExperimentRealization`,
`ObservationSet`, `AnalysisRevision` without the kernel needing to understand
any of them, the same way it does not need to understand what a Python symbol
or a Git tree is today.

## 2. Three working precedents already exist — do not copy any wholesale (reconciled)

The inventory found services that already look like partial instances of this
idea, but reconciliation corrected how they should be read:

- **`claim-evidence`'s ledger core** is the strongest precedent for the
  service plane at first glance: one generic
  `{operation_id, action, profile, expected_state, payload}` envelope already
  runs three unrelated profiles (`proposal-research-v1`,
  `revision-bound-review-finding-v1`, `production-path-v1`) with real
  idempotency (`operation_id` + payload digest) and CAS
  (`expected_state`/`revisionHeads`). Reconciliation reclassified its
  underlying mechanics — revision, CAS, idempotency, authority grant — as
  belonging to the kernel **semantically**, with the three profiles as the
  service-plane instances built on it. This is not a proposal to move, rename,
  or extract any code; `claim-evidence` remains exactly what it is today.
- **`slice-campaign`'s capability dispatch** (`capability-contract.mjs` +
  `host-effect-runtime.mjs`) is generic in shape but workflow-private, has no
  independent authority or recovery semantics of its own, and has exactly one
  consumer (`SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL`, confirmed used only by
  this workflow's own `executable-generation-*` files).
- **`workspace-coordination`'s core** is a kernel precedent: typed resources
  already spanning non-Git kinds (`port`, `database`, `review-budget`),
  fencing-token authority, CAS-checked mutation. Its `admitMutation` operation
  additionally demonstrates that a kernel primitive can expose a bounded
  operation consumed by a service-supplied callback **without** kernel and
  service-plane membership overlapping — see section 3's grammar note.

The right move is not to promote any one of these into *the* `ServiceOperation`
abstraction. `claim-evidence`'s envelope carries semantics appropriate to a
revisioned ledger specifically; `admitMutation` is a kernel operation, not a
service operation at all; other operations may be stateless derivations,
effects, projections, or semantic realizations that need none of this. The
task is to extract the **common boundary grammar** underneath all three
precedents together, not to copy whichever one currently looks most generic —
this matters more after reconciliation than before it, since `admitMutation`
suggests the common grammar may need to cover kernel operations as well as
service operations (section 3).

## 3. Candidate `ServiceOperation` grammar — a hypothesis, not a schema

This is explicitly a starting hypothesis for the reconciliation step (section
10, question 2) to test against the 20 units `service-plane-inventory.md`
already classified — not a settled design:

```text
ServiceOperation
    identity
    contract revision
    implementation revision
    semantic owner / domain
    input type / owned refs
    determinism class        (deterministic | semantic | hybrid)
    required authority
    effect class              (observe | derive | mutate | orchestrate)
    evidence / result type
    recovery / idempotency semantics
    coordinate contribution
```

**`implementation revision` was added by reconciliation, not hypothesized in
the original draft of this document.** It is distinct from `contract
revision`: a stable contract can have two implementations that produce
materially different derived observations, which matters for historical
reconstruction even when no model is involved. This is not speculative —
`review-subject` already pins an analyzer/checkpoint-validator digest before
every invocation (`legacy-backend-adapter.mjs:57-106`); that digest pair *is*
an implementation revision, already present in running code, just not
previously named as a grammar field. It is also distinct from **semantic
realization** (which model/provider/harness/context performed a judgment) —
see section 8. A coordinate should eventually be able to answer three
different questions about one result: what operation was promised (contract
revision), what deterministic machinery produced it (implementation
revision), and what model performed any semantic judgment involved (semantic
realization).

**Reconciliation also surfaced a more open question about the grammar's own
name.** `workspace-coordination`'s `admitMutation` (section 2) is a real
operation with identity, authority, effects, and recovery semantics, but it
is a *kernel* operation, not a service operation. That suggests the shared
grammar this document is looking for may not ultimately be called
`ServiceOperation` — it may need a broader supertype that both kernel and
service operations instantiate as distinct kinds:

```text
OwnedOperation
    owner plane, contract revision, implementation revision, authority,
    effects, evidence, recovery, ...

    KernelOperation   (e.g. admitMutation)
    ServiceOperation  (e.g. review-subject.create_physical_profile)
```

This is not a schema proposal. It is one data point suggesting the ten-field
grammar may describe operations beyond the service plane — left open for the
next reconciliation pass, not decided here.

Illustratively, for a service the Candidate Trajectory family already
proposes:

```text
service: code.candidate_trajectory
operation: derive_delta

owner: code-domain/candidate-trajectory
inputs: immutable candidate refs
determinism: deterministic
effects: none outside durable observation publication
evidence: content-addressed trajectory result
recovery: recomputable under a bound implementation revision
```

Work Engine's kernel does not need to know what `symbols_modified_again`
means. It needs to know that this operation is deterministic, produces no
effect outside a durable observation, and is recomputable — exactly the
`ServiceOperation` fields above, none of which are code-specific.

Whether ten fields is the right number, whether `determinism class` and
`effect class` are truly independent axes, and whether every one of the 20
inventoried units can honestly populate all ten fields are open questions for
the reconciliation step, not settled by this document.

## 4. Resolving the `capability` naming overload

`service-plane-inventory.md` confirmed three existing uses of the word
"capability," not the two originally suspected:

1. Operation identity — `capability.checkpoint_lifecycle/bind_candidate`
   (`slice-campaign`'s dispatch key).
2. A role/realization's granted permission —
   `capability.external_information_retrieval`,
   `capability.visual_artifact_observation` (the ports and
   pre-indexed-capability-resolution family).
3. A model-facing dynamic-tool-manifest shape —
   `proposalCapabilityDefinitions` (`product-development`), structurally an
   instance of sense 1 represented differently.

This document proposes reserving **capability** for sense 2 — what a
role/realization is permitted and able to use — because that concept is
already the richer one (it already carries policy, admission, and grant
semantics in the ports and capability-resolution families). A different term,
**service operation**, should carry sense 1 (and sense 3, which is a
representation of sense 1, not a fourth meaning):

```text
capability grant
    ↓ permits
service operation invocation
```

For example: a role's granted capability to retrieve external information
might permit invoking a `web.search` service operation; the grant and the
operation are related but distinct, and only one of them should be called
"capability" going forward.

This document does not propose renaming anything in the current codebase. It
proposes the vocabulary a future formalization should use, so that work does
not inherit the collision.

## 5. Coordinates as an open service-state map

`service-plane-inventory.md` found coordinate-shaped fragments already
scattered across services that were never designed with this in mind:

- `review-episode`: identity + revision + handled transitions, with
  writer-generation takeover.
- `claim-evidence`: claim/revision/operation identity.
- `workspace-coordination`: resource + fencing token + lease identity.
- `review-subject`: subject/profile/digest chain.

`revisioned-research-and-execution-architecture.md`'s coordinate (§2) already
lists "world-state basis and coverage" as one field in an otherwise fixed,
enumerated property list. This document proposes concretizing that field as
an **open map of revision-bound service-state references** rather than a
closed list the coordinate schema must be revised to extend:

```yaml
coordinate:
  service_state:
    claim-evidence:
      claim_id: ...
      revision_id: ...
    workspace-coordination:
      resource: ...
      lease_id: ...
      fencing_token: ...
    code.review-subject:
      subject: ...
      profile_revision: ...
    review-episode:
      episode_id: ...
      revision: ...
```

A future research domain could add `research.observation_set: ...` and
`research.experiment_spec: ...` without the coordinate kernel changing at
all. The kernel does not need to interpret any owned service-state reference
— it only needs to know which revision-bound owned service states
constituted the world at that coordinate.

## 6. Not everything under `/services` should become a first-class service

The inventory's negative findings matter as much as its positive ones:

- `skills-migration-integrity` is a stateless function with no identity,
  authority, or independent persistence, called inline from one workflow
  file. This document's working conclusion is that it should probably remain
  an ordinary utility owned by whatever workflow uses it, not be promoted to
  a registered service merely because it lives in a `services/` directory
  today.
- `operational-coordination` looks reusable and domain-neutral, but its
  recovery mechanism is a heuristic regex match against board state, not a
  durable CAS index the way `review-episode`'s or `claim-evidence`'s are.
  That is a strengthening question, not a generalization question — it needs
  better recovery semantics before it should be relied on for coordinate
  reconstruction, regardless of what the service plane eventually looks like.

The service-plane project should not become "register everything as a
service." Some things are ordinary library code; some things are
service-shaped but currently under-evidenced.

## 7. A sharper rule for where inference belongs

A workflow should not ask "can I compute this?" It should ask the service
plane: "does an admitted service operation own this fact?"

```text
if yes:
    service operation -> fact

if no service owns it and the question is semantic:
    projection -> model judgment -> owning transition
```

This is not a new principle invented for this document — it is the same
boundary [`candidate-trajectory-remediation-delta-for-native-review.md`](candidate-trajectory-remediation-delta-for-native-review.md)
and [`candidate-trajectory-builder-side-consumption.md`](candidate-trajectory-builder-side-consumption.md)
already apply narrowly to one workflow (native review remediation). This
document proposes it as a general question any workflow should be able to
ask of the service plane, not a fact specific to Candidate Trajectory.

## 8. Relationship to provider/harness/operator realization (reconciled)

A service operation classified as `semantic` or `hybrid` (section 3) requires
a **semantic realization** to execute; a `deterministic` operation does not.
"Realization" here means specifically a model/provider/harness/context
realization — a `deterministic` operation is not realization-free in every
sense. It still depends on an **implementation revision** and sometimes an
execution environment: a historical Candidate Trajectory result is
reproducible only under a bound analyzer version, Git semantics, and parser
version. These are two different concerns and should not share a name:

```text
semantic realization:            model / provider / harness / context
service implementation revision: service code / version / environment
```

```text
semantic service operation
        |
        v
required realization
        |
        v
provider + harness + tools + context
```

This does not change anything
[`provider-turn-harness-runtime-and-operator-projection.md`](provider-turn-harness-runtime-and-operator-projection.md)
already owns — realization admission, capability grants, and port ownership
remain exactly as that document describes. What this section adds is a
reason a service operation needs a realization at all: `reviewer-runtime`
(named in the inventory as a partial precursor to `HarnessRuntimePort`/
`ProviderTurnPort`) is a `hybrid` service operation whose semantic half
requires exactly this. A deterministic operation like `review-subject`'s
profile derivation never needs one.

## 9. What this document does not decide

- It does not refactor `slice-campaign`'s directory structure, even though it
  identifies six distinct units inside it. Directory structure diverging from
  service-plane structure is acceptable; ownership and authority must still
  be correctly separated in the contracts themselves.
- It does not mandate that every inventoried unit become a registered,
  first-class service (section 6).
- It does not freeze the `ServiceOperation` schema in section 3.
- It does not rename `capability` anywhere in the codebase.
- It does not decide which existing services are "true" semantic owners
  versus derivations, adapters, or coordinators beyond what
  `service-plane-inventory.md` already recorded — that classification is
  itself part of the open reconciliation work.
- It does not authorize implementation of anything in this document.

## 10. Reconciliation questions — resolved and still open

The original list of eight questions, plus four refinements raised during
review, went through one reconciliation pass
([`service-plane-reconciliation.md`](../docs/service-plane-reconciliation.md)).
Some are now resolved (folded into sections 1–9 above); most are not. This
section separates the two rather than presenting all twelve as equally open.

### Resolved by reconciliation

- **Implementation revision** (originally refinement 9): resolved. Added to
  section 3's grammar and section 8's realization discussion, grounded in
  `review-subject`'s existing digest-pinning mechanism — not merely
  hypothesized.
- **Can kernel primitives expose bounded operations consumed by services?**
  (originally refinement 12, retitled on resolution): resolved, but not in
  the direction first proposed. `workspace-coordination`'s `admitMutation`
  demonstrates exclusive kernel ownership plus a boundary, not overlapping
  kernel/service membership — see sections 2 and 3.
- **Scope boundary for auxiliary tooling** (not originally a numbered
  question, surfaced by `review-bench`): resolved into section 1 — the three
  planes apply within production/governed execution; auxiliary environments
  sit outside plane classification entirely.

### Still open

**[KIND: RESIDUE, batch] [OPEN — all 11 items below are architectural placement/ownership questions (kernel-vs-service boundary, operation-envelope shape, coverage vocabulary, domain-tag granularity), already confirmed open by this document's own reconciliation pass against `service-plane-reconciliation.md`. Consistent with this content being one of `work-engine-planned-architecture.md`'s own 5 deliberately-deferred architecture items (Coordinate/service_state).]**

1. What belongs in kernel/control versus an ordinary service, as a general
   rule? Reconciliation found a repeated shape across three independent
   units — `claim-evidence`'s ledger, `review-episode`, and `slice-campaign`'s
   campaign state all combine durable revisioned state, an admitted
   transition, optimistic concurrency, and an authoritative successor
   identity. That is evidence toward an answer, not a settled rule; three
   independent implementations converging on one shape is exactly the kind of
   evidence such a primitive should eventually be extracted from, not a
   reason to design it prospectively now.
2. What is the minimal common operation envelope? Sharper than before:
   evidence now suggests the answer may not be `ServiceOperation` alone but a
   broader `OwnedOperation` supertype covering both `KernelOperation` and
   `ServiceOperation` as distinct kinds (section 3). Retain all three existing
   precedents (`claim-evidence`, `admitMutation`, `slice-campaign`'s
   dispatcher) as evidence when eventually forming this — do not template on
   whichever currently looks most generic.
3. Which existing services are true semantic owners versus derivations,
   adapters, or coordinators? Answered unit by unit in the reconciliation
   pass; no unit resisted this classification once plane assignment was
   separated from it.
4. Which workflow-private units should remain workflow-local rather than
   become services? Confirmed for `slice-campaign`'s native-review host and
   legacy adapters. `review-bench` turned out not to be this kind of question
   at all — see the resolved scope-boundary item above.
5. How does service state participate in coordinates and reconstruction,
   concretely? Still open — refinement 11 (coverage vocabulary, below) is the
   concrete gap.
6. How are provider/harness realization services represented without leaking
   their implementations upward? Still open. `reviewer-runtime` and
   `slice-campaign`'s legacy adapters are both honestly
   `inherited_transitional_state` (borrowing
   `authority-backed-architecture-directions-as-workflow-inputs.md`'s
   adoption vocabulary) pending
   `pre-indexed-capability-resolution-and-frozen-runtime-realization.md`'s
   admission mechanism — that is outside this document's scope to resolve.
7. How do capability grants authorize service operations without
   reintroducing the naming conflation section 4 identifies? Still open — no
   unit's implementation forced a decision either way.
8. Which current representations duplicate another owner's state and need an
   explicit projection relationship? One confirmed instance
   (`completion-publication.mjs`'s checkpoint-field redeclaration); no
   exhaustive sweep has been done for others.
9. **A third inference case**, still open. Section 7's rule (`owned →
   service fact`, `unowned + semantic → model judgment`) is missing a middle
   state: mechanically knowable but currently unowned by any service. That
   should be recognized as an architectural gap or a temporary fallback with
   explicit provenance, not silently folded into model inference:
   ```text
   mechanically knowable + owned    -> service operation
   mechanically knowable + unowned  -> architectural gap / temporary fallback
                                        with provenance
   not mechanically decidable       -> semantic judgment
   ```
   The closest existing precedent is `code-change-profile`'s
   `unsupported`/`failed` measurement states — per-field, not
   per-fact-ownership, but establishing the right discipline to build from.
10. **Coverage, not just presence**, still open. Section 5's illustrative
    coordinate map only answers which owned states are present. The concept it
    concretizes is explicitly "world-state basis *and coverage*"
    (`revisioned-research-and-execution-architecture.md` §2). A coverage-state
    vocabulary per service-state reference is needed — candidates include
    captured / not_applicable / intentionally_omitted / unavailable / stale /
    unknown — so absence is never ambiguous between "none existed," "not
    relevant," and "reconstruction failed to capture it." This is
    `code-change-profile`'s own measurement-state discipline, extended from
    per-measurement to per-coordinate-reference coverage. Rather than a
    bespoke coverage vocabulary invented here,
    [`claim-evidence-service.md`](../../docs/claim-evidence-service.md)'s
    authorized refresh-outcome vocabulary
    (`retained_unchanged`/`changed`/`inapplicable`/`insufficient`/
    `contested`/`deferred`/`superseded`) is the better candidate, reached via
    [Evidence-Anchor Observation and Impact Nomination](evidence-anchor-observation-and-impact-nomination.md)'s
    proposed anchor contract and observation boundary. Not accepted or
    applied.
11. **Domain-tag granularity**, newly surfaced. `product-development`'s
    delivery adapters do not fit the coarse "code domain" label used for
    triage in the inventory — `domain` needs to be a namespaced tag
    (`product-development`, `code.review`, `code.candidate-trajectory`, …)
    rather than a flat kernel/runtime/code/other split.

## Relationships

| Direction | Relationship |
| --- | --- |
| [`service-plane-inventory.md`](../docs/service-plane-inventory.md) | The implemented baseline this document reasons from. Every claim above that cites a specific service traces to that inventory. |
| [`service-plane-reconciliation.md`](../docs/service-plane-reconciliation.md) | Steps 3–4 of this document's intake session, now complete and reviewed. Section 10 above reflects its resolved and still-open questions; this document's sections 1, 2, 3, and 8 were revised on that basis. |
| [`provider-turn-harness-runtime-and-operator-projection.md`](provider-turn-harness-runtime-and-operator-projection.md) | Owns realization, admission, and port semantics; this document only adds why a semantic/hybrid service operation needs one (section 8). |
| `revisioned-research-and-execution-architecture.md` | This document concretizes its coordinate's "world-state basis and coverage" field (§2) into an open service-state map (section 5), with a coverage-state vocabulary still needed (section 10). |
| Candidate Trajectory family (`candidate-trajectory-*.md`) | An early, concrete instance of a code-domain service family this architecture is meant to host without absorbing its semantics into the kernel. Its reviewer/builder inference-boundary work (section 7) generalizes here. Reconciliation additionally found a `COUPLED_DECISION`: whether `slice-campaign`'s campaign-state persistence should separate from campaign orchestration (mirroring `review-episode`'s separation of state from consumer) directly affects where Candidate Trajectory's predecessor/successor relationship should durably live. Neither this document nor `candidate-trajectory-durable-foundation.md` should resolve that independently. |
| The broader multi-idea architecture-direction synthesis (discussed earlier this session, not yet started) | Deliberately sequenced after this document's reconciliation, which is now complete — the vocabulary this session settled will change how that synthesis describes other pending ideas' service needs. |

This document does not own the final `ServiceOperation`/`OwnedOperation`
schema, the coordinate schema, the campaign-state coupled decision, or any
implementation authority. One reconciliation pass against the implemented
baseline is reflected here; further passes (a second seam-map iteration,
proposal decomposition per
`incremental-architecture-intake-and-seam-reconciliation.md` §13) remain
future work.
