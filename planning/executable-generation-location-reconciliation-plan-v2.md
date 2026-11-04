# Executable-source maintenance with preserved role realization

## Status and lineage

Draft v2, 2026-09-12. Prepared under the operator's request to revise the plan
after fresh placement review. Not implementation acceptance, publication,
activation, campaign continuation, or incident-488 recovery authority.

Parent: [plan v1](executable-generation-location-reconciliation-plan.md), SHA-256
`d17f9ab7eb5fca8814bb27d43033215ad4396461d78f7eebe3c978b73569b02e`.
The parent remains unchanged. Its [review disposition](executable-generation-location-placement-review-disposition.md)
and chatboard 511 explain why it cannot be implemented as written. Code
baseline remains `ad99d46cd73996b580f1263496ccba75acc2c431`.

## Outcome and selected boundary

Run an explicitly admitted executable-source successor while preserving the
existing role realization: canonical identity paths, operational repository,
artifact destinations, provider working-directory semantics, retained bindings,
delivery history, and input custody. Candidate preparation must be inert with
respect to live execution and durable workflow state.

This revises v1's framing. We are **not** relocating the role's operational
environment or making all identity paths location-independent. We are separating
where executable bytes come from from the already-admitted environment they
operate on. Existing absolute identities remain meaningful historical identities.

For the motivating handoff:

| Dimension | Proposed value / owner |
| --- | --- |
| Executable source | An immutable reviewed successor of ad99d46, captured from its clean implementation/publication worktree |
| Identity base | Preserve the predecessor's admitted manifest identity base under `/home/bline/.local/state/work-engine/ppce-reconciliation-909c1bbe/app-server` |
| Operational repository | Preserve `/home/bline/.local/state/work-engine/ppce-reconciliation-909c1bbe`; do not move, reset, or overwrite its dirty PPCE work |
| Artifact destinations and provider options | Preserve the exact predecessor realization, verified before binding a live request; do not derive defaults from the new executable worktree |
| Durable role/lifecycle state | Existing v20 state root; no blank generation-state rollover or copy of bindings |
| Campaign and credential state | Existing independent owners and roots; unchanged |

The predecessor snapshot establishes identity inputs but does not alone prove
every actual provider option or capability-host destination. Bind the missing
observations before live admission. Unknown realization inputs are a blocker,
not permission to reconstruct a convenient equivalent.

## Why this placement

`runtime-manifest.mjs` already distinguishes identity paths from activated skill
paths. Its identity contract remains with the manifest owner. Bootstrap currently
derives executable capture, identity bases, requirements resolution, and live
repository selection from a shared workspace root. The proxy and capability-host
factory likewise couple executable and operational roots.

Generation lifecycle owns source staging and activation. Role-environment and
manifest owners establish identity compatibility. Artifact-root and capability
owners establish actual target semantics. The adapter retains its exact binding
fingerprint check. Context custody alone owns input release.

The selected route makes those existing dimensions explicit; it does not add an
alternate fingerprint or a blanket environment-migration exception. Reject the
alternative of silently rehashing bindings or introducing a role-fingerprint
equivalence alias. A genuine change to the role's operating environment requires
a separately planned binding/context successor, outside this repair.

## Work package A — explicit source and realization inputs

- Add a bounded host configuration separating executable source, admitted
  manifest identity base, operational repository, and artifact root. Preserve
  current defaults for ordinary same-root use.
- Bind non-default realization inputs to an exact predecessor generation and
  an explicit maintenance authorization. Verify repository identity, containment,
  symlink resolution, and the specific capability/target meaning of each path.
  Shared Git ancestry alone does not establish equivalent worktree state.
- Capture code, manifest, skills, profile bytes, and required dependency closure
  from the immutable candidate source. Record their actual origin. Identity
  projection may retain an admitted canonical path; it must never report that
  candidate bytes were read from the predecessor or dynamically import dirty
  predecessor code to satisfy a path.
- Keep requirements hydration and runtime verification consistent: use exact
  content and resolution evidence, not a predecessor pathname that silently
  selects different requirements. Classify path-bearing profile fields by their
  consumer; preserve identity where required and record physical origin separately.
- Route operational capabilities and artifact writes to the preserved admitted
  targets, not the new source worktree. Their implementation comes from the
  candidate executable closure. Observe effective provider cwd/options as well
  as configuration, without a domain turn.
- Produce compatibility evidence using the existing owning fingerprint logic.
  Require exact equality of generation environment and affected retained-role
  fingerprints. Preserve registry bytes, binding revisions, deliveries, and
  attachments. If equality cannot be achieved truthfully under the preserved
  realization, this plan fails; do not weaken either equality gate.

## Work package B — inert preparation and fenced activation

Candidate construction, describe, and validation may inspect immutable inputs
and read-only state projections. They must not start provider turns, lifecycle
timers/retries, queued-input recovery, service work, or writes/migrations against
live semantic stores. Candidate-owned scratch state is distinct from live state.

Make the lifecycle explicit through existing worker/bootstrap owners:

1. Prepare and validate an inert worker, including required initialization
   compatibility and non-delivering role-fingerprint projections.
2. Fence admission and verify the expected generation, binding/state revisions,
   relevant file identities, and absence of conflicting live work or transitions.
   An unresolved closed queue is preserved, not treated as permission to recover.
3. Record the exact admitted successor through the generation store's existing
   CAS transition. Worker readiness before this point cannot enable effects.
4. Enable only the effects allowed by the committed transition and current
   operating authorization. Maintenance-only activation exposes read-only status;
   it does not enable domain delivery or consume deferred recovery work.

The exact mechanics may vary, but no precommit or crash window may run a
candidate as an authoritative worker. Recovery reads the recorded transition
and completes or reports it truthfully; it must not silently revert the store
after successor effects. Record a durable maintenance-only disposition so a
restart cannot accidentally convert activation into queue-release permission.
The activation interlock gates execution; it neither creates nor replaces the
custody owner's release authorization.

Ordinary authorized startup/recovery must remain available under its existing
contracts, but only after admission. The new behavior must cover rejected
candidates and compatible reload candidates as well as this maintenance path.

## Proposed implementation boundary

Candidate owners and corresponding focused tests:

- `app-server/scripts/app-server-proxy.mjs`: explicit host inputs and status.
- `app-server/src/executable-generation-bootstrap.mjs` and
  `executable-generation-store.mjs`: capture, validation, fencing, durable
  maintenance disposition, compatible activation and recovery.
- `app-server/src/default-executable-generation-worker.mjs` and
  `executable-generation-worker-runtime.mjs`: inert versus admitted lifecycle.
- `app-server/src/executable-generation-role-environment.mjs` and
  `runtime-manifest.mjs`: separate physical resolution from existing identity;
  no change to historical identity interpretation.
- `app-server/src/codex-app-server-adapter.mjs`: reusable non-delivering
  fingerprint/compatibility observation if not already exposed.
- `app-server/src/retained-role-live-host.mjs` and
  `local-semantic-live-host.mjs`: explicit recovery activation rather than
  construction-time effects, including avoiding premature live-store writes.

Inspect worker wrapper and service-factory wiring as consumers. Final exact
paths require placement confirmation and an accepted implementation plan;
expanding into a new state owner or unrelated service semantics requires revision.
Do not change claim admission, campaign acceptance, reviewer selection,
credential refresh, queue-release authority, or the general runtime-port design.

## Acceptance evidence

Use fixtures and fake provider transports, not live incident data:

- With a distinct executable source and preserved realization, both owning
  fingerprints equal the predecessor, existing bindings/deliveries remain exact,
  and a later separately authorized synthetic input resumes the original thread.
- Verify actual capability repository/artifact targets and executable module
  origins; equality of a supplied hash is not sufficient evidence.
- Changed instructions, requirements, skills, provider options, access grants,
  targets, unknown fields, stale source/authorization/state, symlink escape, or
  genuine role identity change reject without live effects.
- A rejected candidate and a validated-but-uncommitted candidate perform zero
  provider requests, recovery deliveries, timers, or semantic-store writes.
  Include recoverable authorized queue items to expose premature recovery, not
  only an empty queue or incident 488's currently unreleaseable inputs.
- An incident-488-shaped closed queue with no release authority remains exact
  through maintenance activation and repeated restarts. Existing failure and
  scheduler state remain owned and visible; no new authorization is fabricated.
- Exercise crashes before/after durable activation, lost acknowledgments,
  concurrent CAS writers, and duplicate activation requests. Admission stays
  closed until the runtime and durable identities agree. Retry is idempotent.
- Preserve normal compatible reload, rejection of real environment changes,
  and authorized post-admission lifecycle recovery. Report known unrelated
  failures separately in focused and proportionate integration results.

Independent review must cover identity/resolution, custody, early side effects,
and crash recovery. This v2 has not yet received that placement review.

## Live consequence and stopping conditions

Implementation remains out-of-band in an isolated worktree. After independent
placement confirmation, exact plan acceptance, implementation validation/review,
and separately authorized publication, request the live maintenance handoff for
the resulting source revision. An executable repair creates a new revision; do
not report that repaired runtime as the unchanged ad99d46 commit.

Verify running snapshot identity and preserved state with read-only status before
any domain continuation. PPCE's unpublished delta still requires its own baseline
reconciliation; this plan does not promise that loading main restores it.
Incident 488 replay requires exact recovery evidence and separate approval.

Stop for a plan decision if preserving actual targets conflicts with the desired
operation, provider-retained path references cannot remain valid, either owning
fingerprint changes, or inert staging requires another semantic authority owner.
Those are observable scope boundaries, not permissions to improvise rebinding.
