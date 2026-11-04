# Executable-generation location reconciliation — bounded repair plan

## Status and authority

Draft v1, 2026-09-12. The operator authorized preparing this plan after the
activation and location-reconciliation attempts recorded in chatboard 508–510.
This is not implementation-plan acceptance or authority to implement, publish,
activate a repaired successor, resume PPCE, or replay incident 488 input.

Implementation is proposed out-of-band because the affected App Server cannot
currently start with the required baseline and preserved runtime state. Use an
isolated implementation workspace; preserve the ordinary rescue worktree, the
dirty PPCE worktree, and all live state. Independent placement and implementation
review remain to be selected and authorized; none has occurred for this plan.

## Objective

Allow an explicitly authorized relocation of executable source to a verified
repository location without discarding retained-role identity, delivery history,
or queued-input custody, when the relevant runtime contracts remain equivalent.
Keep source identity, location identity, generation compatibility, role-binding
compatibility, and permission to deliver input separately observable.

## Evidence cutoff and limitations

- Repair baseline: local-main commit
  `ad99d46cd73996b580f1263496ccba75acc2c431`.
- Existing v20 generation: `generation-109bdb1b2f1aa58a4e837d2b616b514d15d30ae0b6e1006f487c35b2bb1ca35a`.
- Rejected ad99d46 snapshot: `generation-3a2bdd1832fc516a7cbe3e3cdd0f41f4e6212bc90659645d7ab0155fa0dfd775`.
- Old source: `/home/bline/.local/state/work-engine/ppce-reconciliation-909c1bbe`.
  This worktree has staged, unstaged, and untracked PPCE changes. It must not be
  reset, moved, or overwritten as a deployment convenience.
- Candidate source: `/home/bline/.local/state/work-engine/legacy-wind-walker/context-lifecycle-bootstrap-recovery-20260910/publication/worktrees/main-publication-v1`.
- State root: `/home/bline/.local/state/work-engine/app-server-migration-generation-live-v20`.
  Operational state remains separately owned at
  `/home/bline/.local/state/work-engine/app-server-migration-934d50f`.
- The generated environment JSON differs only in
  `manifest.identityBaseDirectory`, `manifest.requirementsBaseDirectory`, and
  `semanticContext.profile.source.path`. This observation does **not** establish
  equivalence of all effective runtime inputs or executable behavior.
- Startup rejects the candidate with `environment_migration_required`.
  Bootstrap and generation-store startup checks both enforce compatibility.
- The maintenance-rollover helper intentionally excludes role bindings,
  attachments, and delivery history. Its raw SQLite-file copy is not a suitable
  custody-preserving handoff, particularly when WAL state may exist.
- Incident 488 still has two queued inputs behind closed admission and zero
  abort-release authorizations. The ordered complete queue-row projection from
  `sqlite3 -readonly ... 'select * from context_input_queue order by sequence;'`
  has SHA-256 `3f7f8da96f9db64ee11854c48ae124fcdd551a8599f7d2420d9bc8bb578a0872`.
  This diagnostic projection is not an authorization or a substitute for the
  queue's owned identities.

Evidence is direct source and bounded graph discovery, not independent review.
Graph coverage generation `2026-09-12T15:11:48Z` reported no recorded gaps for
the inspected bootstrap, generation store, role environment, adapter, binding
registry, and rollover helper. Relevant source was read directly; metadata
coverage is not proof of completeness or equivalence to a deployed subject.

## Proposed placement

Executable-generation lifecycle owns the maintenance transition and its durable
receipt. The existing role-environment and adapter owners supply compatibility
evidence; the binding registry remains the owner of retained bindings and
delivery history. Context-input custody remains the sole owner of admission
and release. No new credential, campaign, or queue-recovery service is proposed.

When a stopped host receives an exact authorized location-reconciliation
request, bootstrap validates the candidate and its compatibility evidence.
The generation store records the predecessor/successor transition using its
existing compare-and-swap ownership. The running host exposes the selected
source and transition evidence without delivering a domain input.

This is not satisfied by ignoring fingerprints, replacing stored hashes,
copying old bindings into a blank generation, stripping absolute paths from
every identity, or calling successful startup proof of resumed-role safety.

## Bounded implementation work

1. **Establish the actual compatibility boundary.** Produce a deterministic,
   read-only comparison of the two exact environments. Include generated
   configuration, resolved requirements and skill bytes, instructions, tool
   specifications, provider capabilities/options, role environment revisions,
   repository access, and artifact destinations. Derive per-role fingerprints
   using the owning implementation without starting or resuming provider turns.
   Separate accepted executable-code differences from environment differences.
   Confirm that old paths are locators rather than retained semantic references
   before treating their relocation as harmless. Reject unclassified changes.
2. **Add an explicit, narrowly admitted maintenance transition.** Bind the
   exact predecessor, candidate, old/new locations, compatibility projection
   revision, evidence, operator authorization, and expected state revisions.
   Preserve original generation identities and record a successor, rather than
   making the old fingerprint mean new bytes. The transition is unavailable
   during active work or conflicting maintenance/context transitions.
3. **Preserve retained-role and custody state.** Prefer leaving bindings and
   their revisions unchanged when their actual fingerprints remain identical.
   A changed role fingerprint or need to rewrite retained context invalidates
   this plan's location-only premise: report it before implementation scope
   expands. Do not manufacture a binding successor or equivalence alias.
   Preserve deliveries, terminal-output references, attachments, queue IDs,
   input revisions, admission state, and lifecycle failure state.
4. **Make activation recoverable and observable.** An interrupted transition
   must recover to the recorded predecessor or exact admitted successor; no
   interval may admit work under mixed identities. Reuse the generation store's
   transition ownership rather than coordinating independent raw file edits.
   Startup maintenance must not invoke delivery recovery as a side effect.
   Expose enough read-only status to bind the running worker to its immutable
   source snapshot, compatibility decision, and startup outcome.

The placement is provisional. A fresh placement challenge must test especially
whether preserving role fingerprints is feasible and whether any startup
consumer acts before maintenance admission. A failed premise requires a plan
revision, not a broader migration hidden inside implementation.

## Candidate file boundary

Expected owners are `app-server/src/executable-generation-bootstrap.mjs`,
`executable-generation-store.mjs`, `executable-generation-role-environment.mjs`,
and the proxy's administrative entry/status surface. Reuse adapter fingerprint
logic if a non-delivering compatibility projection needs to be exposed.
Focused tests belong with generation bootstrap/store/worker and retained-role
integration tests. A small generation-owned compatibility module is permissible
if needed to avoid duplicating identity rules.

The builder must confirm exact paths before implementation acceptance. Changes
to binding semantics, context-input release authority, campaign acceptance,
review selection, credentials, or the general runtime-port architecture are
outside this boundary. Existing owner code may be read or exercised in fixtures
without granting authority to change its contracts.

## Acceptance evidence

- Positive fixture: relocate an exact environment with unchanged relevant
  contracts; activate the admitted candidate and verify its source identity.
- Negative fixtures: changed instructions, tools, capabilities, requirements,
  artifact destination semantics, provider options, unknown fields, stale
  authorization, changed predecessor, and changed role fingerprints reject.
- Retained-role fixture: preserve exact binding/delivery identities and show
  that a later **separately supplied** input uses the expected retained thread.
  This synthetic proof does not authorize a live turn.
- Incident-488-shaped fixture: closed admission, two queued inputs, and no
  release authorization survive startup and restart unchanged. No delivery or
  provider-turn request occurs during maintenance, including startup recovery.
- Crash/conflict fixtures: stale CAS, concurrent mutation, and failures around
  transition publication/worker activation never admit a mixed or unrecorded
  state. Retry is idempotent and retains predecessor evidence.
- No regression to normal implementation-compatible reload, incompatible
  environment rejection, or legitimately authorized abort-release recovery
  outside the maintenance-only boundary.
- Focused tests and proportionate App Server integration checks pass against
  an immutable candidate. Report unrelated failures separately. Selected
  independent review must cover identity, custody, and startup side effects.

## Subsequent live handoff — separately authorized

After acceptance and publication, bind the repaired source revision (which will
be a successor of ad99d46, not ad99d46 itself), exact maintenance request, and
current durable state. Activate under the reviewed boundary and obtain running
revision evidence plus unchanged custody/binding evidence. No provider work is
needed to establish activation.

PPCE candidate reconciliation and continuation remain subsequent work. Loading
main does not establish that preserved, unpublished PPCE changes are present
in the executable runtime. Incident 488 recovery additionally requires its own
exact recovery evidence and separate operator approval, even if maintenance
and every synthetic test succeed.
