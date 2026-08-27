# Claims Service Migration and Product-Development Layout

## Status

This document records the claims migration inventory and proposes a directory
direction for the App Server and the durable product-development workflow
before further implementation.

It is an implementation-planning document. It does not migrate records, move
repository paths, admit claim authority, accept a proposal, authorize an
implementation campaign, or make a proposed review coordinator production
machinery.

Assessment evidence cutoff: repository `HEAD`
`7c5893b818bf80ba6a817d90c14b44299ad869df`. Pre-existing uncommitted App
Server context-lifecycle work was preserved and was not treated as claims
implementation.

The target claims architecture remains
[`docs/claim-evidence-service.md`](docs/claim-evidence-service.md). This
document turns its migration section into a concrete placement and bootstrap
plan.

## Decisions

### Claims are a service before they are a role

The first production boundary is a stable claim-evidence service, not a
universal persistent claims agent.

The service owns mechanical consequences:

- attributed evidence admission;
- closed contract validation;
- stable identity and immutable revisions;
- authority and predecessor fences;
- idempotent publication;
- canonical storage and rebuildable projections;
- exact resolution, discovery, lineage traversal, and reliance; and
- durable operation, refusal, and measurement evidence.

Models retain semantic judgment. A later thin conditional claim skill or role
may discover, resolve, explain, or propose candidates through the service. An
ephemeral compiler and distinct verifier may support semantic publication.
Neither becomes the storage or lifecycle owner.

### Product-development artifacts share one machinery root

Raw ideas, intake assessments, proposal packets, proposal reviews,
implementation campaigns, slice plans, and implementation reviews are stages
of one product-development system. They should become discoverable beneath one
repository-level root.

The recommended eventual root is:

```text
development/
  ideas/
    raw/
    intake/
    history/
  proposals/
  reviews/
    proposals/
    implementations/
  campaigns/
  planning/
```

This is a common discovery and lifecycle boundary, not a common semantic
owner. Each stage retains its own contract, authority, identity, and
non-effects.

The root should be repository-level, not `app-server/development/`. These
artifacts can govern changes anywhere in the repository. App Server is their
runtime host and consumer, not their product-meaning owner.

### Do not move existing artifacts yet

Current contracts contain exact Git revisions, historical paths, local
references, integrity digests, and review subjects. A directory cleanup must
not silently retarget those identities or rewrite historical evidence.

Before moving current paths, the runtime and validators need:

1. a configurable development-artifact root;
2. discovery against both legacy roots and the new root during migration;
3. a closed migration manifest mapping each current canonical path to its new
   path and disposition;
4. validation of proposal-local and repository-relative references;
5. preservation of historical review subjects without rewriting them; and
6. an exact migration receipt followed by removal of legacy discovery only
   after all active consumers use the new root.

The current `app-server/ideas/AI_ACCESSIBLE_BROWSER_DESIGN.md` is a raw design
source, not App Server runtime code. It is a candidate for
`development/ideas/raw/` when its owner authorizes the migration; this document
does not move it.

## Existing product-development workflow

The repository already contains most of the workflow in separate surfaces.

### Raw ideas

Current raw and reconciled ideas live under `ideas/`. They are speculative
source material. They do not authorize implementation and do not remain the
current owner of meaning already promoted into a proposal.

### Intake intermediate format

The production-like intake layout is already:

```text
ideas/intake/<idea-id>/
  record.json
  assessment.md
  authority.md
  repository-observations.md
  source-checkpoint.json       # when separately materialized
  projection.json              # generated, non-owning handoff
  <other named evidence>
```

`record.json` is the closed canonical intake state. It binds stable idea and
assessment identities to an immutable Git source revision, blob, path, line
range, and content digest. It owns attributed assessment state, claim-level
dispositions, relationships, uncertainty, reopening conditions, evidence,
proposal references, and explicit downstream non-effects.

`assessment.md` owns nuanced interpretation. `projection.json` contains only
surviving candidates ready for proposal formation plus the bounded evidence
needed by that consumer. The projection is derived and may be empty; it does
not create proposal identity.

Under the new root this becomes:

```text
development/ideas/intake/<idea-id>/...
```

Raw source remains a sibling at `development/ideas/raw/`, rather than being
rewritten into intake state.

### Proposal formation and packets

The proposal former consumes one bounded intake projection and named evidence.
It may produce zero, one, or several independently decidable proposal
candidates. It does not accept them or authorize implementation.

For every surviving proposal:

- `packet.json` owns stable identity and closed lifecycle metadata;
- the referenced narrative owns current proposal meaning;
- supporting files elaborate placement, relationships, evidence, and possible
  implementation shape; and
- Git owns history and review.

The proposal-packets capability is deterministic contract machinery, not a
semantic proposal role. Mechanical validation does not establish proposal
quality, placement, value, priority, acceptance, or implementation authority.

### Proposal review and authority decision

An immutable formed proposal can be reviewed by selected specialist
perspectives. Selection should follow the proposal's actual consequences and
uncertainty; capability availability alone does not justify a reviewer.
Specialist findings remain attributed and advisory. Synthesis must preserve
disagreement and distinguish coordinator inference from specialist findings.

The repository contains proposal-review artifacts and a formed adaptive-panel
coordination proposal, but no fully accepted, general production proposal-
review coordinator role. Existing review records are evidence of the intended
boundary, not proof that the entire stage is ready to port unchanged.

A named authority decision remains separate from review. Proposal review does
not accept the proposal, choose roadmap priority, or authorize implementation.
An implementation campaign begins only from an exact separately owned grant.

### Campaign and slice implementation

After implementation authorization, a campaign supplies the objective, work
source, effective configuration, limits, approval policy, validation profile,
and completion conditions to the slice supervisor.

The current implementation route is:

```text
authorized proposal or work source
  -> campaign preflight
  -> one bounded slice plan
  -> plan acceptance or escalation
  -> retained builder implementation
  -> deterministic gates
  -> immutable review candidate
  -> supervisor-selected specialist perspectives
  -> builder-owned finding evaluation and remediation
  -> accepted slice checkpoint and receipts
  -> next slice or terminal completion
```

Not every workflow step receives a review panel. Proposal review occurs after
a proposal has an immutable review subject. Implementation specialist
selection occurs after deterministic checks produce an immutable slice
candidate. Intake, formation, planning, and ordinary transitions retain their
own validation and authority boundaries instead of being treated as review
panels.

For implementation review, the slice supervisor owns specialist selection.
The builder supplies a bounded consequence projection, executes the selected
reviews through the configured provider, evaluates findings, and performs
authorized remediation. The supervisor does not inspect the implementation to
diagnose findings, and the builder does not select its own panel.

## App Server source layout

The current `app-server/src/` is intentionally flat foundation code. A broad
reorganization now would create churn and conflict with the uncommitted
context-lifecycle slice. New service work should establish a feature-first
layout without moving unrelated foundation files in the same slice.

Recommended growth boundary:

```text
app-server/
  docs/
  src/
    services/
      claim-evidence/
        contract.mjs
        identity.mjs
        validation.mjs
        service.mjs
        projections.mjs
        sqlite-store.mjs
        legacy-compatibility.mjs
      product-development/
        artifact-root.mjs
        intake-delivery.mjs
        proposal-delivery.mjs
        review-delivery.mjs
        campaign-delivery.mjs
    <existing foundation modules remain in place initially>
  tests/
    services/
      claim-evidence/
      product-development/
  runtime-manifest.yaml
  roles/                       # current vertical proof only
```

Feature-first placement keeps each service's contract, state boundary,
projections, and adapter close enough to verify as one semantic path. Shared
SQLite connection or migration primitives may later be extracted only after a
second service demonstrates a stable common interface.

`app-server/roles/` must not become a handcrafted JavaScript class per role.
The current strategic-planner files are first-vertical gateway and contract
code. Future roles remain skill or role-package defined and are composed by the
runtime manifest. Mechanically enforced boundaries belong in declarative
interfaces or generic runtime services.

The proposed role compiler may eventually make decomposed role packages the
canonical source and generate `SKILL.md`. That migration is independent of the
service layout: App Server consumes the generated role projection and declared
interface rather than acquiring a second role definition.

## Claims implementation reuse inventory

### Port as production semantics

The current claim-evidence implementation already proves:

- closed transport-safe JSON validation;
- canonical SHA-256 identities;
- stable claim identity from namespace, subject kind, and stable subject ID;
- immutable exact revision identities;
- exact references and unresolved-status vocabulary;
- typed authority grants with actor, profile, permission, scope, and verified
  authority references;
- idempotent operations and conflicting-operation rejection;
- current-head predecessor fencing;
- immutable typed lineage and cycle rejection;
- exact-revision reliance and non-destructive retirement;
- deterministic projection identity, freshness, completeness, exclusions,
  failures, unresolved references, branches, and conflicts; and
- bounded discovery, exact resolution, traversal, and direct or reverse
  reliance queries.

The schemas and tests are compatibility inputs. The JavaScript service should
reproduce these meanings or record an explicit migration disposition for each
difference.

### Adapt rather than copy

- Python functions become provider-neutral JavaScript contracts and service
  operations.
- Filesystem locking and atomic JSON replacement become SQLite transactions.
- CLI arguments become closed host requests and bounded receipts.
- Root-initialization-only authority becomes a separately owned host authority
  admission lifecycle.
- Generated JSON files become rebuildable query projections and optional
  human-readable exports.
- The full skill becomes a thin conditional discover, resolve, explain, and
  propose interface.

The existing SQLite App Server store supplies proven operational patterns:
schema migration, `BEGIN IMMEDIATE` transactions, integrity checks, strict
tables, restart persistence, full synchronization, and competing-writer tests.
Its current tables and private transaction method are context-lifecycle-
specific. Claims need a separate schema and adapter rather than being inserted
into context tables.

### Preserve as migration evidence

- `skills/claim-evidence/scripts/claim_evidence.py` remains the compatibility
  oracle during migration.
- `skills/claim-evidence/tests/test_claim_evidence.py` supplies parity and
  adversarial cases.
- the closed schemas and representative profile fixture remain wire-contract
  evidence; and
- the evidence-lineage dogfood remains historical evidence for unchanged
  versus changed refresh, exact reliance, authority-domain forks, and truthful
  limitations.

### Do not port as production placement

- filesystem paths or raw SQL as model-facing interfaces;
- builder-owned registry, projection, or authority-manifest maintenance;
- implicit newest-revision selection;
- proposal-dogfood-specific topology as the general schema;
- automatic publication authority for imported records; or
- a custom runtime class for every role using claims.

### Known compatibility hazards

The legacy `apply_operation` function can mutate its in-memory store before a
later validation failure. The CLI avoids publishing that state, but the new
service must validate before mutation or guarantee transactional rollback.

Legacy publication accepts only a current head. The target design permits a
branch only when a domain profile explicitly authorizes one. Branch admission
therefore needs a new closed rule rather than accidental compatibility.

Legacy authority grants are admitted only at filesystem-root initialization.
That is adapter behavior, not the future authority model.

Import validation establishes representability only. It does not authenticate
the producer, admit authority, publish current support, or make a legacy record
applicable to a new consumer.

## Role-port assessment

### Suitable early role ports

`idea-intake` and `proposal-former` are comparatively small semantic roles.
Their meaning already lives primarily in skills and contracts:

- idea-intake adds one deterministic validator/projector;
- proposal-former is primarily model judgment over a bounded intake
  projection; and
- proposal-packets is deterministic validation and transition machinery, not
  another persistent role.

An early App Server vertical can declare retained logical instances for idea
intake and proposal formation while temporarily invoking the existing Python
validators as bounded capabilities. Model-facing writes must still be mediated
through exact artifact requests and host-owned authority checks; read-write
filesystem access alone is not publication authority.

### Not a trivial role port

The slice supervisor is a substantial workflow system. It owns campaign
preflight, effective configuration, phase state, active-slice recovery,
checkpoint lifecycle, review selection, metrics, receipts, completion offers,
and stop decisions across many scripts and contracts. It should first run
through a compatibility adapter or existing skill execution path. A native App
Server rewrite should occur only after its state and effect boundaries are
declared and tested.

The general adaptive proposal-review coordinator is also not yet a settled
production role. Proposal review and implementation review share useful
selection doctrine, but they have different subjects, authorities, and
downstream decisions. App Server placement must not merge them merely because
both select specialists.

## Relationship between claims and product development

The product-development workflow consumes the claims service; the claims
service does not own the workflow.

Examples:

- intake evidence can create attributed observations and semantic claim
  candidates without letting claims decide a candidate is ready for formation;
- proposal research and review findings can publish exact revisions without
  letting publication accept the proposal;
- campaigns and builders can rely on exact claim revisions without letting
  reliance authorize implementation;
- code-change profiles can nominate `may_affect` without establishing semantic
  causality; and
- review and slice outcomes can become evidence without letting the claims
  service accept the reviewed subject.

This separation allows all development stages to share identity, evidence,
lineage, and query machinery while preserving their decision owners.

## Incremental delivery plan

### Slice 1: provider-neutral claims core

- Create `app-server/src/services/claim-evidence/`.
- Port canonicalization, identities, exact references, closed core records,
  authority envelopes, operation envelopes, lineage, and reliance validation.
- Run the legacy fixtures through both implementations.
- Require exact parity or an explicit semantic-difference receipt.
- Do not add SQLite or a model role in this slice.

### Slice 2: transactional canonical store

- Add a dedicated SQLite schema and migration history.
- Implement atomic publication, exact replay, conflict rejection, predecessor
  fences, integrity checks, export, and competing-writer tests.
- Ensure failed operations leave no partial state.

### Slice 3: read service and projections

- Implement exact resolve, bounded discovery, lineage traversal, direct and
  reverse reliance, and truthful projection metadata.
- Keep search indexes and generated exports rebuildable and non-owning.

### Slice 4: evidence intake and deterministic observation

- Define immutable evidence-observation records.
- Connect one exact Git checkpoint or test receipt as an evidence producer.
- Publish only a claim class whose deterministic authority has been explicitly
  admitted; otherwise retain the observation without semantic promotion.

### Slice 5: semantic candidate shadow path

- Add bounded ephemeral compiler and distinct verifier turns.
- Retain candidate, verification, disagreement, tokens, latency, and refusal
  evidence without publication.
- Review all compiler, verifier, and thin claim-interface text with
  `agent-instruction-review` before loading it.

### Slice 6: first authorized claims vertical

- Publish one proposal-research revision and one revision-bound review finding
  through the host authority fence.
- Compare both with the legacy contract.
- Project one bounded relevant-claims input to a builder without teaching the
  builder to operate the registry.

### Slice 7: initial development roles

- Add manifest roles for idea-intake and proposal-former.
- Supply one exact raw idea and one validated intake projection.
- Invoke existing deterministic validators through bounded host capabilities.
- Persist outputs under a configurable artifact root, initially the legacy
  paths until the development-root migration is separately authorized.

### Later: artifact-root migration and native campaign execution

- Introduce `development/` through the dual-root migration contract.
- Move current artifacts only with an exact reviewed migration manifest.
- Bridge the existing slice supervisor before considering a native rewrite.
- Add proposal-review coordination only after its authority and production
  placement are decided.

## Acceptance boundary before more claims implementation

The next claims slice may begin when the user accepts this layout direction.
It must remain additive:

- new claims code goes under
  `app-server/src/services/claim-evidence/`;
- new tests mirror that service boundary;
- current flat foundation files are not reorganized in the claims slice;
- canonical development artifacts remain at their current paths until a
  separately reviewed migration; and
- agent-facing role or inference text receives revision-bound instruction
  review before loading.
