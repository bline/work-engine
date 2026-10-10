# Rust lifecycle multi-slice implementation plan

Status: reconciled for offline Rust continuation, 2026-10-07. S0–S3 are
accepted and locally published. The user authorized this plan reconciliation
and one retained Sol/high S4 integration builder, with UI preparation through
its existing owner. The exact S4 slice contract still binds its implementation
scope. S5 implementation, provider inference, legacy disposition, cutover and
ordinary publication remain outside that grant; this document grants none of them.

## Objective and governing inputs

Deliver the replacement in [DESIGN.md](DESIGN.md), preserving the requirements
and failure distinctions in [DESIGN_EVIDENCE.md](DESIGN_EVIDENCE.md). Follow the
repository's [Rust direction](../../docs/rust-development.md) and explicit
[model selections](../../docs/model-selection.md). The design/source manifest
binds the document revisions and inspected evidence; it does not grant authority.

The historical starting evidence remains the unchanged text-only Rust experiment
and the gate-failed schema-14 JavaScript worktree. Neither is a completed replacement.
The old experiment's three live turns are consumed; the remaining allowance is
zero. Its databases, receipts and source/build manifest remain historical inputs.
The production-intended workspace now exists under `rust/`.

Success means a retained workflow can preserve meaning, custody and authority
through context change and recovery using the actual service composition. It is
not enough for isolated modules, synthetic readiness or a UI replay to pass.

### Current progress and operating premise

The joined published base is `b2efdbfa0354a997b23ac4e183b89ca0dbf36cc2`.
These component exits are complete; M1's actual-service integration remains S4.

| Slice | Published result |
| --- | --- |
| S0 shared contracts/toolchain | `84be6695d716f026460fbf11aa703e73631e514e` |
| S1 lifecycle core/store | `e7c1d88ec0df20ea6213239b6dc2b36abd9f1872` |
| S2 owned executor/provider port | `b2efdbfa0354a997b23ac4e183b89ca0dbf36cc2` |
| S3 wire/client and U0 contract handoff | `634161c2f54dc4fcf9dcb53936951ecefe457235` |

Work Engine need not remain operational during development, and this effort
uses external development tools rather than depending on its own runtime.
Build the replacement offline and qualify a deliberate later cutover. Continuous
availability, dual running and a temporary Node bridge are not prerequisites.
Ordinary runtime custody, context continuity, uncertainty and recovery contracts
remain required. Inactivity does not settle an effect or dispose of saved state.

The [host replacement plan](../../docs/rust-host-replacement-plan.md) and
[generation plan's current disposition](../../docs/rust-generation-integration-plan.md#current-disposition)
inform the later joins. HP1 and independently owned review/claims/campaign work
do not wait for S4. S8's actual permanent Rust consumer and S9's permanent Rust
startup/deployment owner require an agreed handoff; the native-review host is
not implicitly the lifecycle host. A temporary bridge needs a named consumer,
a concrete need direct replacement cannot meet, bounded scope and an exit.

## Delivery shape and dependencies

M0–M6 retain the acceptance meanings in the design. S0–S10 below are work slices;
their numbering is not a requirement to execute unrelated work serially.

```text
M0 design decisions + implementation authority
                    |
             S0 shared foundations
                    |
          +---------+---------+
          |         |         |
       S1 core/  S2 owned   S3 wire/client ---- U0: UI schemas/fixtures
        store    executor      |
          +---------+---------+
                    |
             S4 service integration --------- U1: controlled live read-only UI
                    |
             S5 native/operation contracts
                    |
              +-----+-----+
              |           |
        S6 semantics   S7 recovery/admin
              |           |
        S8 retained       |
          workflow        |
              +-----+-----+
                    |
             S9 migration/rollback join
                    |
             S10 replacement qualification
                    |
       separate live-proof and operational authorities
```

S1–S3 can run concurrently after S0's interfaces are accepted. S6 and S7 can
overlap after S5; S8 can proceed when S6 is ready without waiting for S7 if their
edits remain independent. Integration of shared-state changes is serialized.
The graph retains the accepted dependency shape; the table above records which
component interfaces now exist without claiming their service composition works.

S4 now assembles the real store-to-executor-to-observer vertical from the accepted
S1–S3 revisions. The S0 contracts give it a stable target; discovered mismatches
return to their owner through an explicit interface change. Component acceptance
does not satisfy the integrated exit.

### Models, ownership and launch limits

- Planning/architectural adjudication: `gpt-6-astra`, starting effort `xhigh`.
- Each implementation lane: one retained `gpt-6-sol` builder, effort `high`,
  responsible for code, integration contributions, tests and ordinary remediation.
- One named Sol integration owner retains S0/shared-workspace responsibility,
  coordinates the joined build, and owns S4, S8 and the final joins. This is not
  automatic transfer of unfinished builder work to the supervisor.
  During the first parallel tranche this owner is the S1 builder, not a fourth
  implementation worker alongside S1–S3.
- Proposed adversarial reviewer: separate `gpt-6-astra`, effort `xhigh`, with
  required initial isolation and retained context through bounded remediation.
  This is not cross-provider independence and does not override an owning
  campaign that requires it. The reviewer and designer are separate roles.

The initial parallel limit is three implementation workers, plus the supervisor;
review starts after capacity is available rather than creating an unbounded fanout.
Explicitly authorize the concurrency and use a launch adapter whose contract
permits it. A sequential-only adapter is not silently reinterpreted as parallel.
No agent is launched by accepting the architecture alone.

Builders are not alone in the repository. Give each an isolated worktree or
non-overlapping owned paths and an exact base/interface revision. They preserve
other actors' changes. Only the integration owner changes the root Rust manifest,
lockfile, toolchain, shared type/encoding definitions and common gate inventory
during parallel lanes. Other builders request interface/dependency changes; they
do not invent local variants or silently relax a contract to make their code fit.
Use the repository coordination board for claims and transition handoffs.

## S0 — Shared foundation and executable slice contracts

Prerequisite: accepted architecture/placement and a bounded implementation grant.
Proposed paths: `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/rust-toolchain.toml`,
`rust/crates/work-engine-types/`, and minimal public contracts/manifests for the
S1–S3 packages. The integration owner freezes the exact creation/edit manifest
before launch; this proposed directory list is not permission for arbitrary files.

Deliver:

- Pin toolchain/MSRV, edition, dependency versions/features and production/test
  profiles. Review licensing/advisories; record unresolved dependency concerns.
  Preserve the existing experiment lockfiles and evidence.
- Specify IDs/revisions, digest domains/encoding, commands, effect plans,
  claim-entry/result handoff, outcome/settlement dimensions and typed errors.
  Define one operation contract sufficient for the controlled text profile;
  unsupported capabilities remain explicit, not successful placeholders.
- Specify clock samples, absolute authority expiry, monotonic wait budget,
  snapshot/cursor semantics, version negotiation and untrusted-to-domain conversion.
- Qualify the new wire codec with strict parsing and golden vectors. Define
  trusted domain/kind/version selection and preserve historical identity bytes;
  no universal `digest()` silently replaces legacy serialization. Include pinned
  claim-evidence/workspace counterexample vectors demonstrating their different
  bytes, without implementing either domain's migration. Legacy codec support is
  added only for an accepted import. Share public values, not entry permits.
- Define the S2 boundary between reusable process/task facts and lifecycle
  execution policy, and the S1 boundary between SQLite mechanics and domain-owned
  transactions. Select only the small interfaces needed by this vertical; new
  generic runtime/persistence crates are not prerequisites for the parallel lanes.
- Create the smallest compiling interface surface for the parallel lanes,
  plus compile-fail tests and a provider-disabled gate inventory. No duplicated
  reference controller or fake implementation advertised as production behavior.

Exit: agreed, compiled contracts; dependencies resolved; T03/T28/T36/T37 foundation
checks; accepted interface revision and exact manifests/test commands for S1–S3.
This is not M1 completion. No paid provider capability is enabled by the scaffold.

## S1 — Pure lifecycle core and transactional store

Depends on S0. Own `rust/crates/lifecycle-core/` and
`rust/crates/lifecycle-store/`, including their unit/property/migration tests.
Keep these coupled: durable validation and reducer semantics have one owner.

Deliver the pure subject reducer, normalized SQLite state/journal, atomic command
results and effect plans, authority/revision checks, ordered custody, lifetime
writer lock, artifact publication and coherent recovery load. Core performs no
I/O. Entry and result application are transactionally bound to the exact attempt.
Rejected command replay is stable; uncertain entered work is never automatically
resent. Trusted grant resolution is supplied by the accepted service composition,
not by keys embedded in an untrusted request.

Keep connection setup, transaction helpers and migration bookkeeping separable
from lifecycle schema, durability choices, writer fencing, validation and CAS
semantics. Any helper is subordinate to those domain-owned contracts; it cannot
expose a second writable path or span external effects with a local transaction.
Do not broaden this slice into other domains' persistence or a general framework.

Exit: T01/T03–T06/T09/T11/T18/T23/T24/T32 checks at the core/store boundary,
including actual competing processes where necessary. New migrations are owned
and monotonic; future migrations cannot be assigned conflicting numbers by lanes.
Module checks do not establish the service-level crash cuts required at S4.

## S2 — Owned asynchronous execution

Depends on S0. Own `rust/crates/lifecycle-runtime/` and the agreed
`rust/crates/lifecycle-provider/` port surface, with runtime-local tests/support.

Deliver Tokio task/process supervision, bounded queues and reserved result
capacity, clock sampling, immediate result ownership, safe framing across awaits,
and exact shutdown accounting. Execute only committed plans after the store's
entry claim. Record task failure, cancellation, panic, late evidence and actual
child exit separately. No second writable lifecycle cache or admission decision.

Separate low-level supervision/framing/correlation from the lifecycle binding
that claims effects and returns exact observations. Reusable mechanics accept
owner-bound identifiers and report facts; they cannot manufacture a lifecycle
permit or classify remote settlement. Provider-specific session and evidence
rules stay in their adapters. Begin within the authorized modules; later library
extraction must not make builders/reviewers import lifecycle core or store policy.

Controlled peers exercise the same port that real adapters will use. They may
control their own responses and external ledger, never mutate lifecycle state.
Use bounded acknowledgments at crash/cancellation boundaries rather than sleeps.

Exit: T06/T07/T18/T21/T22/T27/T32 component tests and a conformance test against
S0's entry/result contract. Peer results are not evidence that S1 and S2 compose;
that claim belongs to S4. Preserve every entered attempt through cleanup.

## S3 — Public wire and reusable read-only client

Depends on S0. Own `rust/crates/lifecycle-wire/`,
`rust/crates/lifecycle-client/` and their schemas/compatibility fixtures.
Initially no UI files and no production administrative command surface.

Deliver versioned DTOs, explicitly selected schema dialect, stable error codes,
strict command parsing, read compatibility rules and Rust client observation.
Separate read access from authenticated command submission. Implement bounded
wait, cursor-gap resync and reconnection without progressing lifecycle or blindly
retrying a mutation. Generated fixtures distinguish failed outcome, unsettled
effects, unavailable observation and historical completion/current admission.

Exit: T19/T28/T36/T37 contract checks and U0: an exact version/hash, schema bundle,
Rust package API, representative fixtures and unsupported-variant behavior for
the UI owner. These are contract fixtures, not fabricated live observations.
Actual socket authorization/redaction and store-backed resync are proved in S4.

## S4 — First integrated service and observation boundary

Depends on S1, S2 and S3. The named integration owner owns
`rust/crates/lifecycle-service/`, joined tests in `rust/tests/`, and common
`rust/crates/lifecycle-test-support/`. Lane owners fix their modules as needed
against the same interface revision; changes to shared contracts are explicit.

Deliver the permissioned local endpoint, trusted principal/grant resolver and
real core/store/executor composition. The controlled scenario enters through
public commands, disconnects its initiator, progresses the supported text
transition autonomously and observes its committed result using the shared client.
Provided continuation/controlled semantic responses establish only the bounded
profile, not the later semantic-quality claim. Unsupported production routes stay
disabled. Tests never edit lifecycle tables or invoke a hidden advance method.

Exercise actual-service crashes at all entry/result boundaries, stale and
conflicting commands, early success/failure, saturation, observation loss, clock
boundaries, process replacement and unsafe-close history. Derive proof facts from
this run's durable state and independent peer/child observations.

Exit: M1 and the read-only observation part of M2, with T01/T03–T07/T09/T11/
T18/T19/T21–T24/T27/T28/T32/T33/T36/T37 at the real composition. Freeze the tested
build, profiles, schema/client revision and evidence. U1 gives the UI owner a
controlled running service and reconnection contract. It is neither a production
proxy connection nor authority for UI commands. The UI owner separately accepts
and implements its integration; this plan does not claim or edit that workspace.

## S5 — Native provider and bounded owned operations

Depends on S4 and selection of the first operation profile and authorization
semantics. Own `rust/crates/lifecycle-codex/` and bounded provider-port/contract
extensions. Shared core/store changes go through the integration owner.

Preserve the pinned native protocol contract, raw ingress identity, history
association and operation-specific settlement. Test installed native execution
against isolated simulated inference where available, without credentials or live
network/provider entry. Preserve the native versus simulated evidence distinction.
No private/raw adapter path may bypass entry accounting.

Select one useful narrow tool profile, including its domain and execution owner,
entry-bound or commit-bound authority guarantee, outcome and settlement evidence.
Unenforceable revocation/descendant-effect guarantees disable that route; they do
not become a synthetic universal seal. Respect same-native-session requirements.
Normalize observed token/cache metrics without inventing measurements or adopting
a forecasting policy.

Exit: remaining M2 provider/profile evidence; T03/T06–T09/T15–T17/T28/T30/T33–T35.
Live provider verification remains separately authorized. This slice does not
adopt the unresolved general Authority Commit Gate proposal.

## S6 — Semantic preservation

Depends on S5; may overlap S7. Own `rust/crates/lifecycle-semantic/`, its material
ports, corpus and tests. The semantic verifier's quality/independence policy is
selected before this slice's implementation grant.

Build bounded canonical material projection, source/grant revisions, separate
compiler/verifier effects, adequacy challenges and successor continuation checks.
Use the same runtime/entry/result path as domain work; inference cannot create
authority. Include human answers arriving during capture, superseded decisions,
unresolved approvals and stale canonical references. Controlled responses test
orchestration and rejection, not the quality of an unrun live model.

Exit: M3's controlled composition and T13/T14/T15/T23/T35 evidence, with explicit
remaining live semantic-quality acceptance. A supplied digest echo is not success.
Corpus preparation can begin earlier without claiming this dependent slice done.

## S7 — Recovery, disposition and offline administration

Depends on S5; may overlap S6. Own `rust/crates/lifecycle-admin/`, admin scenarios
and proposed migrations. The integration owner serializes shared store/reducer
changes; concurrency never permits independently defined custody semantics.

Implement observation/reconciliation of uncertain entered attempts, exact-claim
authority-bound no-retry disposition, compatible startup, coherent backup/export,
inactive destination adoption and fenced source handoff. Preserve pending and
completed delivery identities in candidate-current rollback state. Ordinary
context cycling uses the stable store and never invokes offline migration.

Qualify only the historical codecs actually reached by the selected import,
using pinned old bytes/hashes and cross-language fixtures. Preserve old references
and explicit lineage to any new identity; reject unknown or mismatched codec
contracts instead of silently rehashing them. Claim/workspace compatibility
examples in S0 are not authority to migrate those services.

Exit: administrative components pass T21/T22/T24–T26/T35 using disposable state.
Production old-state import remains unestablished until S9. No actual queue,
including historical Q3/Q4, is retired, transferred or delivered by this slice.

## S8 — One retained workflow through the complete host path

Depends on S6 (therefore S5/S4); can overlap independent S7 work. The integration
owner composes the lifecycle service with a permanent Rust host and
`rust/crates/lifecycle-proof/`. Before slice admission, the lifecycle and host
owners agree the actual retained-workflow consumer, supported subject/profile,
entry path and exact source/contract revision. A proof-only host cannot replace
that consumer. Domain hosts remain external owners; this join neither absorbs
native-review authority nor grants edits to another host's paths. No legacy
bridge is assumed merely to keep the unused system running.

Run active work through pressure/yield, semantic capture, verified checkpoint,
successor readiness, custody and two queued inputs. Continue one delivery through
a second context change without replay. Exercise supported tools and a differently
named subject kind against the same kernel. Normal context replacement keeps its
host process alive when the selected provider contract permits it.

The controlled and later live proof use one installed service and scenario driver,
changing only authorized external-provider behavior and fault controls. Readiness
and final receipts project durable applied facts; caller completion, files and
test callbacks never advance production state. Validate before proof teardown;
perform any required offline preparation only after truthful quiescence.

Exit: M4; T02/T10–T17/T19–T22/T27/T29–T35 at the retained-workflow boundary.
All eight legacy proof responsibilities have an exercised supported path or an
explicitly justified replacement. Tool and semantic claims remain scoped to
the tested profiles; no real production work or live inference is implied.

## S9 — Recovery/migration integration and rollback qualification

Depends on S7 and S8. The integration owner joins the admin and runtime paths.
Before admitting this slice, the human/state owner chooses import or an explicitly
authorized archive/clean start for the actual saved state. No such choice has
been made. The following import, T25 and candidate-current rollback evidence
remain required by the existing acceptance contract until that owner accepts an
exact amendment; an archive choice cannot silently retire unresolved obligations.

Import a representative coherent legacy fixture into an actually absent target;
exclude concurrent launches, fence the source, commit the handoff and admit the
destination. Repeat crash cuts and idempotent retries without claiming safe retry
for uncertain effects. Roll back from the stopped candidate's current custody,
including changed pending/completed claims, into an actually absent compatible root.

Use the real packaged startup and old-executable compatibility checks. Do not
manually construct destination custody rows as a substitute for bootstrap adoption.
Actual live inventory refresh, authority packets and operational roots are separate.

Bind distinct executable/context/provider-session/process identities through the
selected permanent Rust startup/deployment contract. Its owner and actual
consumer are an explicit S9 handoff, not an assumed Node generation-manager
bridge. Bind the selected source/build/lock/toolchain, artifact/runtime identity,
captured configuration, saved path bases and persisted restart selection; reject
ambient build/PATH redirection and incompatible startup before effects.
Deployment owns build selection and launch/drain coordination; lifecycle owns
custody/admission and its executor reports actual
draining/settlement. Prove their composed fence and handoff, not
a lifecycle-local imitation of deployment. The whole executable-generation
manager and other control-plane services remain separately owned. Offline
exclusive transfer is sufficient for the development cutover premise; online
coexistence and hot rollover are not added requirements. Any necessary temporary
bridge needs the concrete consumer, need, owner, bounded scope and removal
condition described above. Preserve historical reference hashes through
import/readback as part of T25/T28.

Exit: M5; integrated T01/T02/T06/T11/T21/T22/T24–T26/T33/T35. Selection, startup,
disposition and rollback contracts agree on one schema and custody lineage.

## S10 — Replacement qualification and operational handoff

Depends on S9. Freeze the integrated candidate, complete the provider-disabled
production-composition gate and execute the selected review against that subject.
All T01–T37 have explicit evidence or an honestly unresolved applicability/coverage
decision; omission is not a pass and cannot weaken a required replacement claim.
Review both the core rules and the exact installed composition, not only the most
recent delta. Retained remediation returns to the same reviewer where appropriate.

The offline candidate can become ready for a separately authorized bounded live
proof. That authority specifies the exact build/profile, provider, quota, disposable
state, deadlines, permitted tool domain and semantic acceptance. A consumed attempt
is not reused. Any failure preserves the subject and stops advancement for diagnosis.

M6 completes only after the required live evidence and human replacement acceptance.
Publication, real legacy reconciliation/custody disposition, shutdown, activation
and rollback execution retain their own exact authority. An accepted code candidate
does not by itself stop the old proxy or authorize queued work.

## Slice admission, validation and evidence

The primary integrated acceptance owners below cover the entire scenario matrix.
Component versions may run earlier and later slices strengthen them; this table
does not claim that an early pass establishes every later operating profile.

| Primary integrated owner | Scenarios |
| --- | --- |
| S4 service/observation | T01, T03, T04, T05, T06, T07, T09, T11, T18, T19, T21, T22, T23, T24, T27, T28, T32, T33, T36, T37 |
| S5 provider/operations | T08, T15, T16, T17, T30, T34, T35 |
| S6 semantics | T13, T14 |
| S8 retained workflow | T02, T10, T12, T20, T29, T31 |
| S9 migration/rollback | T25, T26 |

Before launching a slice, bind its accepted objective/contract revision, exact
base and allowed paths, owner, dependencies, model/effort, supported profiles,
dependency pins, test inventory and resource limits. The tables above allocate
ownership; they are not an exhaustive source-file manifest. S0 produces those
manifests for the first parallel tranche; later slices refine theirs against the
accepted integrated base rather than guessing future filenames now.

An implementation grant can include normal focused tests, bounded diagnosis and
in-scope remediation; no new ceremony is needed for every compiler iteration.
Changed authority, shared semantics, materially expanded scope, external effects
or live-provider use returns to the appropriate decision owner. An unexpected
failure stops candidate advancement, not already-authorized investigation.

The intended Rust gate includes these commands once the workspace exists:

```sh
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace --locked
```

Those commands are proposed, not executed or sufficient alone. Each accepted
gate also names production-feature builds, compile-fail/property tests, schema
generation/checks, migration/compatibility tests, controlled actual-process/native
tests and packaged startup for its scope. Inventory discovery verifies expected
test names/counts and skipped reasons before relying on a selection. A shorter
selector or changed tracing profile is a changed gate input, not an invisible fix.
Run production and diagnostic configurations; do not blindly enable every feature
if that would combine mutually incompatible or live-only capabilities.

All default, focused, controlled and broad checks explicitly disable live-provider
entry. S0's gate contract records the exact supported selector/config names;
absence of a known selector list is not proof of provider-disabled execution.
Use isolated profiles without credentials and controlled endpoints; verify entry
counts against independent peer evidence. Any inherited legacy gate must disable
all of its live selectors explicitly. Offline admin/fixture capabilities are not
reachable through the production domain endpoint.

Capture task/claim/receipt and actual child-exit evidence before cleanup, plus
coherent database images where absence or durable state matters. Preserve raw
failures, effective configuration, exact source/build/lock hashes, and known
limitations. No rerun-until-green, assertion weakening, historical waiver reuse
or promotion of nonrecurrence into an explanation. Use separate temporary roots
per lane and no shared mutable provider profile.

## Decisions and next authorization

S0–S3 are complete at the published revisions above. The next authorized work
is S4's provider-disabled Rust composition under its exact accepted slice plan,
with one retained Sol/high integration builder. UI U0 preparation proceeds with
the existing UI owner; U1 still depends on the integrated service. Independent
HP1 work has no S4 dependency. None of those grants selects a later host,
saved-state disposition or operational cutover.

Before S5: select the first owned-operation profile and native-session policy.
Before S6: select semantic corpus and verifier quality/independence requirements.
Before S7/S9: select compatibility and disposition/handoff contracts, including
the human/state-owner import versus archive/clean-start decision and any explicit
change to T25/rollback scope. Before S8: agree the actual permanent Rust host
consumer. Before S9: agree its permanent startup/deployment boundary. Before M6:
settle retention/redaction, resource bounds and exact review/live-proof requirements.
These later decisions do not block independent, authorized foundation work, but
unsupported routes stay disabled until their owning requirements are established.

No estimates of elapsed time or percentage complete are evidence-backed yet.
Report progress by accepted exit conditions, failed/unrun checks, integration
state and remaining authority—not by generated file count or passing compilation.
