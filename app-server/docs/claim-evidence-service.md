# Claim Evidence Service

## Status and decision

This document defines the intended App Server claims architecture. It carries
forward the evidence-lineage semantics developed under
[`proposals/evidence-lineage`](../../proposals/evidence-lineage/family.md), but
changes their runtime placement:

> Claim custody and lifecycle are common runtime infrastructure. Deterministic
> evidence becomes claims through code-owned rules; irreducibly semantic
> judgments use bounded ephemeral inference inside that infrastructure.

Ordinary builders, planners, and reviewers are consumers and evidence
producers. They do not each operate a claim registry as part of their standing
role behavior.

This direction is authorized as the App Server design target. This document
does not by itself authorize a repository mutation beyond its own creation,
publish a production claim, accept a proposal or review, grant a writer
authority, or make the unfinished implementation under `skills/claim-evidence`
production state.

Repository evidence cutoff for this design: `fc254b4c8fd14cc4415a20184ca91485df96d05d`.

## Why placement changes

The existing claim skill asks a domain agent to select operations, construct
exact records, manage authority manifests, maintain lineage, inspect projection
completeness, and record reliance while doing its primary work. Those
distinctions are valuable, but their placement creates recurring cognitive
load and makes correct claim management depend on every role remembering the
same lifecycle.

The App Server foundation now provides better owners for those consequences:

- a stable host outside replaceable model context;
- retained logical role and thread bindings;
- attributed request, turn, tool, and lifecycle events;
- durable transactional SQLite state;
- revision fences, idempotent operation identities, and ordered input custody;
- ephemeral inference capabilities with bounded material loading; and
- runtime-manifest projections that can narrow authority and capabilities per
  role instance.

The placement rule is:

> A role remains responsible for its domain judgment. The claims service owns
> the mechanical preservation, validation, publication, lineage, discovery,
> and delivery consequences of that judgment.

This is the same division used by semantic context lifecycle management: models
judge meaning; code owns custody, integrity, fencing, actuation, and evidence.

## Goals

The initial service should make it possible to:

1. preserve evidence-backed statements independently of any model context;
2. derive deterministic facts from attributed, integrity-bound host
   observations without asking a builder to restate them;
3. compile semantic claim candidates from bounded attributed material without
   treating model output as authority;
4. verify and publish immutable claim revisions under an exact authority and
   predecessor fence;
5. discover and resolve claims without silently selecting a newest revision;
6. record reliance against an exact claim revision and decision scope;
7. nominate possible impact from repository or contract changes without
   declaring a claim false, stale, or causally changed;
8. project only relevant, provenance-bearing claims into a role request; and
9. remain correct across context replacement, process restart, executable
   generation replacement, multiple builders, duplicate delivery, and partial
   evidence.

## Non-goals

The service does not acquire authority to:

- accept implementation, proposals, plans, reviews, or roadmap changes;
- decide that a consumer should rely on a claim;
- turn retrieval rank or graph reachability into applicability;
- infer semantic causality from temporal proximity or touched files;
- reopen downstream work merely because an impact was nominated;
- activate an inactive role or create a scheduled obligation;
- convert a builder's prose into a canonical statement without verification
  and publication admission; or
- make its database, query index, transport, compiler, or verifier the owner of
  domain meaning.

Task ownership and repository-area claims used to coordinate builders are a
neighboring control problem. They may reuse identity, evidence, and lineage
mechanics, but they must not be confused with epistemic claims merely because
both use the word “claim.”

## Preserved semantics from the evidence-lineage proposals

The following properties remain foundational:

- A stable claim identity names a durable bounded question, not whichever
  conclusion is currently preferred.
- Every material revision binds the subject and evidence world actually
  examined.
- Claim revisions are immutable and independently addressable.
- Producer attribution and authority for the judgment are separate facts.
- Assumptions, limitations, support qualification, confidence where meaningful,
  evidence references, sensitivity references, and reopening conditions remain
  attached to the exact revision.
- Refresh, correction, supersession, composition, derivation, identity fork,
  and retraction remain distinct non-destructive relations.
- Discovery returns candidates, not automatic truth, freshness,
  applicability, sufficiency, acceptance, or authority.
- Reliance targets one exact claim revision, consumer revision, and decision
  scope. It never follows “latest” implicitly.
- Mechanical change may nominate `may_affect`; only an authorized semantic
  judgment can establish retained support, changed support, applicability, or
  `changed_because_of`.
- Canonical history and rebuildable query projections remain separate owners.
- Projection completeness, freshness, exclusions, failures, and source
  watermarks are visible results rather than hidden implementation details.

The two initial domain profiles also remain useful proving surfaces:

- proposal-research claims; and
- revision-bound review findings.

Their domain owners retain materiality, severity, review episode, synthesis,
support, outcome, and decision semantics. The shared service owns only the
common evidence and lineage boundary.

## Architecture

```text
App Server observations and canonical artifacts
  ├── role requests and responses
  ├── tool and test receipts
  ├── Git checkpoints and code-change profiles
  ├── review findings and proposal research
  └── human or domain-authority decisions
                │
                ▼
  attributed, integrity-checked evidence intake
                │
       ┌────────┴─────────┐
       ▼                  ▼
deterministic rules   semantic candidate compiler
       │                  │
       │                  ▼
       │           independent verifier
       └────────┬─────────┘
                ▼
      authority and revision fence
                ▼
     append-only canonical claim store
                │
       ┌────────┼───────────┐
       ▼        ▼           ▼
    queries   reliance   impact nominations
       │                    │
       ▼                    ▼
role projections       domain-owned refresh
```

### Stable service boundary

The claims service is common host infrastructure, not a peer role. Its stable
code boundary owns:

- operation correlation and idempotency;
- attributed evidence admission and integrity validation;
- closed record validation;
- authority-envelope verification;
- compare-and-swap publication;
- canonical history and integrity checks;
- projection construction and provenance;
- exact-revision query and reliance mechanics;
- impact-nomination custody; and
- durable episode, failure, and measurement evidence.

Replaceable executable generations may implement extractors, compilers,
verifiers, domain profiles, and query policies. A generation change must not
silently reinterpret already-published record meaning. Schema or semantic
changes require an explicit compatibility or migration result.

### Ephemeral semantic inference

Some statements cannot be derived mechanically. For those, the service creates
ephemeral, least-privileged compiler and verifier turns. They receive a bounded
host projection containing:

- the exact candidate subject and evidence baseline;
- attributed source material and integrity references;
- the applicable domain profile;
- the requested judgment scope;
- admitted authority references without private credentials;
- predecessor and competing claim revisions when relevant; and
- explicit omissions, unknowns, and projection completeness.

The compiler proposes semantic content. A distinct verifier challenges
identity continuity, evidence support, limitations, authority scope, reference
coverage, and conflicts. Neither model can publish, mutate the store, record
reliance, declare downstream reopening, or widen its evidence set.

Content does not acquire governing authority because it appears in a prompt,
tool result, retrieved document, prior claim, or model response. The host
preserves origin, trust class, producer, instruction applicability, and content
digest for every supplied material.

### Domain judgment and maintenance

The service does not require one universal persistent “claims agent.” When a
claim domain needs a semantic refresh, one of three routes applies:

1. an authority-admitted deterministic domain rule establishes the result;
2. an ephemeral compiler/verifier episode operates under an already admitted
   domain authority; or
3. a domain-specific authorized role or human supplies the judgment.

A persistent maintenance role is justified only when a domain contract truly
requires ongoing judgment, not merely because storage and lineage need an
operator. Such a role remains outside the shared service and cannot gain
authority from being selected by it.

## Record model

The first App Server schema should preserve the existing v1 core while
separating evidence intake and inference episodes from canonical claims.

### Evidence observation

An immutable host observation records:

- stable event and producer identities;
- origin and trust classification;
- exact subject and evidence baseline;
- content or artifact reference and digest;
- observed timestamp and provider sequence when available;
- completeness, exclusions, and collection failures; and
- the executable generation and adapter version that normalized it.

An observation says what the host received or derived. It is not automatically
a semantic claim.

### Claim candidate

A candidate binds:

- one proposed stable claim identity;
- one exact subject and evidence baseline;
- proposed proposition, support, assumptions, limitations, and references;
- compiler provenance and inference measurements;
- applicable profile and requested judgment scope;
- predecessor or competing revisions; and
- unresolved questions or blockers.

Candidates are temporary inference products. They are retained as episode
evidence but do not appear as published claim revisions.

### Verification

A verification binds the exact candidate and source projection revisions. Its
host-derived disposition is accepted or unresolved. It records every required
check and preserves disagreement or uncertainty. Verification is publication
eligibility evidence, not publication or domain acceptance.

### Canonical claim history

Canonical history contains:

- stable claim identities;
- immutable claim revisions;
- typed lineage relations;
- exact-revision reliance records;
- admitted authority references and publication receipts; and
- later impact, refresh, support-selection, and obligation records when those
  phases are implemented.

No current-state field may erase the immutable transition that produced it.
Current heads, support selections, active reliance, unresolved impact, and
pending obligations are derived projections over canonical records.

## Authority model

Authority is supplied by a trusted lifecycle owner and validated at the final
host boundary. It is never inferred from:

- possession of a skill, dynamic tool, or MCP connection;
- Git authorship or filesystem access;
- a role name declared in model output;
- compiler or verifier agreement;
- successful schema validation; or
- persistence of an earlier operation.

The runtime manifest and role binding identify an acting logical role instance
and narrow its capability ceiling; they are not by themselves authentication
or publication authority. A trusted host lifecycle and domain authority
profile must still admit the exact semantic transition and decision scope.

| Consequence | Default owner |
| --- | --- |
| Record an attributed, integrity-bound host observation | App Server host |
| Derive a closed deterministic fact | Named deterministic extractor |
| Propose a semantic claim candidate | Ephemeral compiler or authorized domain producer |
| Verify candidate support | Distinct verifier |
| Admit publication authority | Trusted launcher or domain lifecycle owner |
| Publish an immutable revision | Claims service after all fences pass |
| Decide to rely on a revision | Consumer's decision owner |
| Nominate possible impact | Deterministic analyzer or attributed producer |
| Judge refreshed support or causality | Authorized claim-domain owner |
| Reopen or change downstream work | Downstream workflow owner |

Publication binds one stable operation identity, exact payload, actor, domain
profile, authority reference, expected predecessor or branch state, and source
projection revision. Exact replay is idempotent. Conflicting reuse fails
closed. Branch creation is possible only when the profile explicitly permits
it.

## Builder experience

The normal builder path should require no manual claim bookkeeping.

```text
builder performs accepted work
  → host captures attributed effects and receipts
  → code-change analyzer profiles the immutable checkpoint
  → deterministic rules emit factual observations
  → semantic inference proposes and verifies material claims when scheduled
  → service publishes only admitted revisions
  → host-issued handoff evidence references resulting exact revisions
```

Relevant existing claims arrive as a bounded request-context projection. The
projection states why each candidate was selected and carries revision,
authority, limitations, freshness, completeness, and unresolved-impact state.
The builder may consider the projection as candidate evidence. Any
consequential reliance remains an explicit, exact-revision decision of the
consumer's authority owner; presence in model context grants neither reliance
nor decision authority.

A thin conditional claim skill may remain for tasks that genuinely require a
model to:

- discover claims outside the supplied projection;
- inspect exact lineage or reverse reliance;
- propose a candidate that cannot be extracted from ordinary work receipts;
- explain an unresolved conflict; or
- request a domain-owned applicability or refresh judgment.

That skill describes result meaning and the narrow interaction interface. It
does not teach every role to operate storage, construct authority manifests,
rebuild projections, or maintain the claim lifecycle.

An initial role interface could express:

```yaml
claims:
  projection: relevant_exact_revisions
  discover: allowed
  resolve: allowed
  propose_candidate: allowed
  publish_revision: denied
  record_reliance: denied
  refresh_support: denied
```

Runtime manifests may narrow these capabilities. They cannot widen the domain
profile's authority ceiling.

## Multi-builder behavior

One shared service supports many builders. Evidence intake remains attributed
to each logical role instance, thread, binding revision, accepted slice, and
repository checkpoint. Claim identity and lineage are global within their
declared namespace rather than copied into each builder's context.

When builders work on different areas:

- each produces independently attributed observations and checkpoints;
- shared or overlapping claims may receive several `may_affect` nominations;
- neither builder can declare another builder's governing claim refreshed;
- cross-builder dependencies reference exact claim or artifact revisions;
- incoming claim notifications pass through the target role's ordinary input
  admission and context-lifecycle custody; and
- a context reset cannot erase published claims, reliance, pending impact, or
  delivery state.

Repository work allocation, overlap prevention, and merge authority remain
with claims/ownership control specifically designed for work coordination or
with isolated-worktree orchestration. Epistemic claim publication does not
reserve files or accept another builder's changes.

## Context lifecycle integration

The claims service state lives outside model context. A semantic continuation
checkpoint records exact claim revisions and unresolved obligations on which
the role currently depends; it does not copy or silently advance claim history.

During context replacement:

1. the context manager closes role input admission;
2. claim notifications or cross-agent messages are queued outside the retiring
   revision;
3. the checkpoint preserves exact relied-upon claim revisions and relevant
   projection watermarks;
4. the successor reconciles those references; and
5. queued claim events are released through normal role delivery only after
   accepted reconciliation.

The claims service may continue recording host observations while a role is
fenced. It must not inject semantic work into the retiring model context.

## Storage and projection

The first App Server adapter should use transactional SQLite because the
foundation already provides restart-safe local state, compare-and-swap
publication, competing-writer tests, and stable controller paths outside
executable snapshots.

SQLite is the initial canonical publication boundary for this local runtime,
not the semantic owner. The schema and service contract own meaning. The store
should support:

- append-only records and operation receipts;
- unique stable identities and content digests;
- predecessor and authority fences;
- atomic canonical publication plus outbox consequences;
- exact replay and conflicting-operation detection;
- integrity checks and schema migration;
- backup/export into human-readable provenance-bearing records; and
- projection rebuilding from canonical tables.

Query indexes, search embeddings, graph projections, MCP views, and generated
Git exports are replaceable. Every projection exposes its build version,
canonical watermark, source content set, freshness, completeness, exclusions,
and failures. An unavailable or partial projection cannot masquerade as an
empty result.

## Transport and security

The stable host mediates all mutation operations. Model-facing tools expose
closed requests and bounded receipts rather than filesystem paths or raw SQL.
Read projections may be exposed through App Server dynamic tools or MCP, but
transport does not change identity, authority, or completeness semantics.

Required controls include:

- authenticated host construction and authority admission;
- least-privileged compiler and verifier threads;
- destination-specific write authorization;
- prompt-injection-safe trust classification;
- secret and confidentiality filtering before inference;
- digest and provenance validation for every loaded material;
- retention limits for inference inputs and raw provider events;
- append-only or equivalently protected publication evidence; and
- explicit refusal when source inventory, authority, or projection integrity
  is unavailable.

## Impact, refresh, and reliance propagation

Maintenance is a later layer over the production core, but the core must not
make it impossible.

```text
repository or contract event
  → may_affect nomination
  → optional domain-owned refresh episode
  → retained_unchanged / changed / inapplicable
     / insufficient / contested / deferred / superseded
  → optional new claim revision
  → exact reverse reliance projection
  → downstream-owner applicability and reopening judgment
```

The code-change analyzer is an evidence producer for the first edge. It can
identify changed files, symbols, contracts, tests, and structural reachability.
It cannot publish `changed_because_of`, advance canonical support, or reopen a
consumer.

The service should initially prefer on-demand refresh when a real consumer
needs a decision. Continuous semantic maintenance is justified only by observed
latency or correctness needs; it must not become standing model inference by
default.

## Observability and feedback

Every claim episode should carry one identity across:

- evidence capture;
- deterministic extraction or compiler inference;
- verification;
- authority validation;
- publication or refusal;
- later query and reliance; and
- impact or refresh outcomes.

Useful measurements include:

- evidence bytes and source count;
- deterministic versus semantic candidate rate;
- compiler and verifier tokens and latency;
- verification disagreement and unresolved rates;
- publication refusal reasons;
- duplicate and conflicting operation rates;
- query candidate count and projection completeness;
- claim reuse across builders, reviews, and decisions;
- impact nominations resolved unchanged, changed, or unresolved;
- time from nomination to required consumer judgment; and
- repeated work avoided by exact claim reuse.

Metrics may tune extraction, scheduling, query ranking, and inference budgets.
They cannot relax evidence, authority, exact-revision reliance, or integrity
requirements.

## Migration from the current skill-owned implementation

The existing artifacts are bootstrap evidence, not waste. Preserve:

- the shared core and domain-profile schemas;
- stable identity and immutable revision algorithms;
- exact-reference status vocabulary;
- authority-manifest and operation-envelope tests;
- lineage, branching, projection-completeness, and exact-reliance fixtures; and
- the dogfood records demonstrating unchanged and changed refresh paths.

Reclassify their placement:

- `skills/claim-evidence/scripts/claim_evidence.py` is a reference adapter and
  migration oracle, not the target runtime service;
- the current full skill is a source for extracting the future thin
  interaction contract, not the lifecycle owner;
- filesystem JSON stores and generated projections are import/export fixtures,
  not assumed canonical App Server state; and
- proposal-local dogfood remains historical evidence rather than production
  data.

Migration should compare the old and new validators on representative records
and explain every semantic difference. Import never grants publication
authority merely because a legacy record validates.

## Implementation sequence

This is a dependency-aware bootstrap hypothesis. Evidence may change mechanisms
or combine adjacent steps, but it may not collapse the ownership, authority,
evidence, publication, reliance, or semantic-impact distinctions above.

1. Inventory the existing schemas, validator behavior, fixtures, authority
   assumptions, and unfinished maintenance consequences.
2. Define provider-neutral App Server contracts for evidence observations,
   candidates, verification, canonical claim records, operations, and query
   receipts.
3. Port the production core into a transactional SQLite adapter with integrity,
   idempotency, migration, backup, and competing-writer tests.
4. Implement exact resolve, bounded discovery, lineage traversal, and direct or
   reverse reliance as service operations and read-only projections.
5. Connect deterministic evidence producers: immutable Git checkpoints, test
   receipts, App Server effects, and code-change profiles.
6. Run semantic claim compilation and distinct verification in shadow mode
   without publication.
7. Publish one authorized proposal-research claim and one revision-bound review
   finding through the new host boundary and compare them with the legacy
   contract.
8. Generate a bounded relevant-claims request projection for one builder and
   prove that ordinary implementation requires no claim-management behavior.
9. Produce claims automatically from one builder handoff, then let a separate
   reviewer discover and rely on an exact revision.
10. Add `may_affect` nomination from the code-change analyzer and exercise one
    retained-unchanged and one changed refresh without granting the analyzer
    semantic authority.
11. Replace the heavy builder-loaded claim skill with the thin conditional
    interaction skill and declarative role-interface capabilities.
12. Only after real use, evaluate continuous maintenance, external MCP
    projections, and richer query indexes.

## Acceptance evidence

The base claims service is ready for ordinary development only after proving:

- restart-safe canonical publication and exact replay;
- rejection of unauthorized, stale, conflicting, partial, or tampered writes;
- rebuildable queries with truthful freshness and completeness;
- exact revision resolution and reliance without implicit newest selection;
- one deterministic claim path requiring no model inference;
- one semantic claim path with bounded compiler and distinct verifier evidence;
- one builder that performs domain work without operating claim mechanics;
- one review consumer that discovers and relies on an exact revision;
- context replacement preserving claim references and pending input custody;
- multiple role instances producing attributed evidence without ownership
  collapse; and
- legacy fixture parity or an explicit migration disposition for every
  difference.

## Open decisions

- Which authority-admission lifecycle should replace the legacy root-init-only
  authority manifest?
- Which exact claim classes are safe for deterministic publication rather than
  deterministic observation plus semantic verification?
- Which builder handoff fields provide enough semantic candidate input without
  recreating claim bookkeeping in the builder contract?
- What is the smallest relevant-claims projection that avoids both omitted
  dependencies and indiscriminate context loading?
- Which claims require active canonical-support maintenance versus immutable
  history and on-demand applicability assessment?
- When should a semantic verifier use a different model or provider rather than
  merely a distinct ephemeral invocation?
- Which confidential evidence may be stored canonically, passed to inference,
  exported, or exposed through MCP?
- How should task-ownership claims used for multi-builder coordination relate
  to, or remain separate from, evidence-backed semantic claims?
- Which outcomes should notify an active role, queue a normal input, create a
  domain-owned obligation, or only remain queryable?
- When does observed query scale justify a graph or search projection without
  moving canonical ownership?

## Relationship to neighboring designs

- [`semantic-context-lifecycle-manager.md`](semantic-context-lifecycle-manager.md)
  owns safe replacement of role working context and exact queued-input custody.
- [`role-compiler-proposal.md`](role-compiler-proposal.md) owns the proposed
  decomposed role source and generated model-facing projection.
- [`proposals/evidence-lineage`](../../proposals/evidence-lineage/family.md)
  remains the semantic and historical proposal lineage for claims, refresh,
  reliance, and maintenance.
- [`skills/claim-evidence`](../../skills/claim-evidence/SKILL.md) is the current
  skill-owned reference implementation to be decomposed and migrated, not
  silently edited into the new service.
