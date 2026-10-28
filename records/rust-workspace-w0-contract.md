# W0 — fresh Rust canonical publication contract

Proposed 2026-10-18 under the user's W0 contract-formation authorization. This
settles the recommended bounded design following [C5](rust-compiler-c5-plan.md).
It is submitted for acceptance; it grants no implementation, deployment, user
commit or activation authority. W1 below is the resulting concrete code scope.

## Selected result and owners

The selected profile, `canonical-publication-local-v1`, lets a local operator
accept one exact checkpoint for publication, inspect its integrated result,
and separately authorize one canonical-ref update. It uses fresh private Rust
state and a local repository with an existing target branch. There is no old
state import, Node bridge, extension runtime, network fetch, push or tag.
The attached-user-branch [Python adapter](../skills/slice-completion-commit/SKILL.md)
is a separate contract and remains outside this profile.

| Owner | Owned meaning and implementation placement |
| --- | --- |
| `slice-campaign` | Exact checkpoint eligibility, operator acceptance, immutable offer and per-offer human decision, publication consumption/result reference. New `publication` module and private completion store; existing review outcomes retain their meanings. |
| New `workspace-publication` crate | Git-object/manifest inspection, isolated integration and validation records, operation-bound resource lease, physical canonical-ref CAS, publication result and reconciliation. No campaign acceptance decisions. |
| New `work-engine-completion-host` package | Small local executable composing those two owners, plus `work-engine-completionctl` client. Trusted startup installation, local operator ingress and the concrete Rust completion command route. No general capability dispatcher or provider. HP4 can later expose this same application route. |
| Operator | Semantic acceptance of the exact checkpoint and a separate `create` decision over the exact prepared offer. Review completion and agent requests supply evidence, never these decisions. |

This is the first bounded campaign publication profile, not whole-campaign
terminalization. Its acceptance record means **this exact checkpoint is accepted
for the optional publication described here**. It neither changes the campaign's
next-slice baseline nor declares the campaign objective complete. Publication
and uncertainty do not block checkpoint continuation (`INV-026`, `INV-029`).

Current [`Campaign::finalize_initial_completion`](../rust/crates/slice-campaign/src/initial_outcome.rs)
and [`Workflow::join`](../rust/crates/review-workflow/src/initial.rs) publish review
evidence. `Reported`, `Completed`, finding evaluation and `CompletionSelection`
do not supply either operator decision. W1 reads their exact owner records and
adds the bounded publication-owned records below; it does not reinterpret them.

## Trusted local ingress and qualification boundary

The production-intended ingress is a Unix-domain socket opened by the completion
host under an operator-installed private root. Startup pins the repository's
Git common-directory identity, canonical ref allowlist, completion/workspace
root identities, operator UID, validation program and policy digest, and host
build. Requests cannot install or override these values. The host obtains the
peer UID from the connected Unix socket; a request's claimed actor, `human`
field, receipt or boolean has no authorization effect.

`work-engine-completionctl` presents an exact offer or acceptance package and
sends an explicit `accept-checkpoint`, `create`, `decline`, or `cancel` command
with that displayed digest. The campaign owner receives the real connected
socket and independently checks its peer against the installed operator UID
before producing an operator decision record. There is no public
`approve: true` API and no agent-accessible operation that manufactures an
operator principal. The record attributes the decision to the authenticated
operator channel; it does not pretend that socket credentials prove physical
human presence.

Production use requires the operator channel and state roots to be outside the
agent's granted execution/writing authority. A same-UID unrestricted agent can
impersonate that operator or rewrite its records; such a deployment does not
establish human-authority isolation and is unsupported. Rootless W1 tests use
the real socket and peer check with the test runner's UID as a **fixture
operator**. A mismatched installed UID and forged actor fields must fail. This
tests ingress mechanics without new OS accounts or root; it establishes neither
production UID separation nor an actual human decision. Test-only authority
selection is unavailable from a production request and is labeled in every
fixture receipt. A later production installation must establish the missing
custody; W1 does not activate it.

## Data contract

All records below have a domain kind, schema version, owner root ID, immutable
record ID and SHA-256; references contain those values and no writable path.
New workspace/completion records use the named
`workspace-publication-rfc8785-v1` codec: restricted finite JSON values, UTF-8
RFC 8785 canonical bytes, no trailing newline. The digest input is ASCII
`work-engine/workspace-publication/v1`, NUL, the ASCII record-kind name, NUL,
then the canonical payload bytes; kind names cannot contain NUL. Unknown
versions fail. Git OIDs separately carry
their object algorithm. Existing campaign and evidence digests remain under
their existing codecs; no rehash or cross-domain interchange is implied.

The following are the proposed public DTO shapes, not already-existing APIs.
`Ref<T>` means an exact owner reference. Fields listed as evidence are checked
by their owner; accepting their JSON shape does not make them authoritative.

| Type | Required content |
| --- | --- |
| `PublicationCandidate` | Existing `CampaignRef` and `CandidateRef`; plan and scope revision; checkpoint commit/tree/parent; exact review outcome refs and gate receipt refs; `AttributedManifest`. |
| `AttributedManifest` | Sorted unique repository-relative paths; before/after Git mode and blob OID or absence; raw content SHA-256 for existing sides; task attribution and its evidence ref. Rename is delete+add. The complete checkpoint-parent-to-tree delta must equal this manifest, including mode changes and deletions; untracked filesystem state is irrelevant. |
| `Eligibility` | Candidate digest, exact owner-verified evidence binding, operator acceptance record, acceptance scope `publication-only`, and active/revoked status with revision. |
| `PreparedPublication` | Workspace-allocated stable operation ID and preparation binding digest; eligibility ref; repository ID and canonical ref; observed expected parent; manifest digest; integrated tree; prepared commit, single parent and complete message; validation policy/result refs; operation-private worktree/artifact identity. |
| `CompletionOffer` | Eligibility and prepared-publication refs, exact `create` proposal digest, message, canonical target, expected parent and prepared commit/tree. State is `Open`, `Declined`, `Cancelled`, `CreateAuthorized`, `Consumed` or `Settled`; revisions use CAS. |
| `OperatorDecision` | Exact acceptance-package or offer digest/revision; action; installed authority-source digest; observed peer UID; request ID and server receipt time; decision record digest. `create` is bound to one offer and cannot authorize another preparation. |
| `PublicationRequest` | Exact offer, eligibility and prepared-publication refs plus the operation ID already allocated in that preparation. There are no caller-selected authority, replacement operation ID, holder, lease generation, paths or Git command arguments. |
| `WorkspaceOperation` | Stable operation ID, prepared-publication ref, state/revision and any exact admission/attempt/result refs. States are `Prepared`, `Admitted`, `AttemptStarted`, `ClosedNoEffect`, `PublishedObserved`, `PublishedUnconfirmed` and `Unresolved`; `ClosedNoEffect` is an irreversible tombstone for this operation. |
| `PublicationReceipt` | Request, preparation and admission binding digests; resource set; generation and admission ref or exact no-effect closure ref; exact commit/parent/tree/message and current target observation; effect state, limitation and recovery locator; fixture/production authority mode. |

Typed, privately constructed `CheckedEligibility`, `CheckedOperatorDecision`
and `MutationGuard` are process capabilities and do not implement Deserialize.
Exported views and saved references are data. After restart the owners re-read
their durable records and re-establish the checks; deserialization never restores
a capability. IDs are single-purpose and duplicate request IDs with different
bytes conflict; identical repeats return existing state, not a second effect.

## Entry, freshness and application route

The completion host is the W1 consumer. Its local API composes these proposed
domain methods, with command names also used by the client:

1. `inspect_candidate(candidate_ref)` obtains the exact current campaign
   candidate and review/gate records through owner APIs. Workspace inspects the
   immutable Git objects and full attributed delta. Gate evidence must bind the
   checkpoint tree, task patch, plan/scope and selected policy. Required evidence
   claims must be established; pending/conflicting owner writes cannot be
   accepted by a request flag. The operator receives the native review verdict,
   phase, findings, limitations and unresolved questions without a synthesized
   acceptance verdict.
2. `accept_checkpoint_from_peer(socket, package_digest, request_id)` rechecks
   that exact inspected package and its campaign candidate revision, then
   records the operator's semantic acceptance and returns `Eligibility`.
   `Reported` or a positive finding evaluation cannot take this transition.
   The acceptance package changes if any bound content changes.
3. `prepare_publication(eligibility_ref, target_ref, message, preparation_request_id)`
   first allocates and durably records a stable operation ID in workspace.
   Identical preparation request IDs/bytes return that same operation;
   different bytes conflict. It uses a detached, operation-namespaced
   integration worktree with its own index/HEAD. It
   integrates the accepted delta onto the observed canonical tip, refuses
   conflicts and any resulting parent-to-tree delta outside the accepted
   manifest, and runs the installed local validator against the actual
   integrated tree. W1 supports the nonconflicting case; semantic conflict
   repair returns to a new candidate. Successful validation produces the
   immutable prepared commit and `PreparedPublication` using the acyclic
   binding below. A publication request cannot allocate or replace this ID.
4. `open_offer(prepared_ref)` records an immutable offer in the campaign
   completion owner. `decide_from_peer(socket, offer_ref, action, request_id)`
   records a separate exact operator `create`, decline or cancel decision.
   Preparation and validation precede this decision so the human-authorized
   proposal includes the final tree and expected parent.
5. `publish(request)` re-reads the active eligibility, exact offer/decision,
   prepared artifacts and validation through their owners. It consumes the
   create decision for one operation, obtains the fenced critical lease below,
   and attempts the exact canonical-ref CAS. `inspect(operation_id)` and
   `reconcile(operation_id)` return checked owner state; reconcile is read-only
   with respect to Git.

Acceptance survives unrelated later campaign continuation because it binds an
immutable accepted checkpoint. Explicit revocation, changed preparation,
changed target parent or changed validation policy invalidates an unconsumed
offer. The accepted checkpoint is still preserved. Before admission, the
operator may cancel/revoke via the same authenticated channel; the campaign
owner's CAS determines whether cancellation or consumption wins. After
consumption, the decision cannot be revoked retrospectively or reused. A
cancellation request is then recorded as too late while reconciliation owns
the effect result. No caller can replace an unresolved operation with a new one.

Consumption is durable before workspace admission. If the process stops or
pauses in between, absence of admission on a read is not a no-effect result:
a delayed publisher could still arrive. Recovery calls workspace
`close_no_effect(operation_id, consumption_ref)`, which takes the same OS guard
used for admission and checks the exact healthy preparation and consumption
binding. A workspace transaction changes `Prepared`, or `Admitted` with no
`AttemptStarted`, to irreversible `ClosedNoEffect` and returns its durable
closure receipt. If an attempt has started or required records are missing or
conflicting, it returns recovery evidence instead of a closure. The campaign
may settle `NoEffect` only after verifying that closure receipt through the
workspace owner. A crash after closure but before campaign settlement leaves
the tombstone in force and recovery records that same result.

Every admission and physical-attempt entry rechecks the operation state under
that same continuously held guard. A delayed publisher observing
`ClosedNoEffect` returns its existing result and cannot admit or mutate, even
if it checked campaign consumption earlier. Another attempt needs a new
preparation/operation ID, new offer and new create decision; a closed operation
is never reopened. Records in two databases are not claimed to commit atomically.

### Acyclic preparation and authority bindings

Workspace owns the precommit preparation binding. After integration and
validation, it hashes the record kind `preparation-binding` with exactly:
profile/version, workspace root ID, allocated operation ID, eligibility ref,
repository ID, canonical ref, expected parent, attributed-manifest digest,
integrated tree OID, validation policy digest, tree-validation receipt ref,
and proposed message bytes. The validation receipt binds the integrated tree,
not the future commit. The prepared commit's message embeds only the operation
ID and this preparation digest as fixed metadata alongside the proposed
message. Neither its own commit OID nor a later offer, create decision,
consumption or admission is an input to that digest. `PreparedPublication`
then binds the resulting commit OID and complete message to that preparation.

The campaign owner subsequently creates the offer over that exact
`PreparedPublication`, records the operator decision over the offer, and
records consumption of that decision for the preallocated operation. At
admission, workspace hashes kind `admission-binding` over exactly: operation
ID, preparation digest, prepared-publication ref, offer ref, create-decision
ref, campaign-consumption ref, complete resource set and assigned generation.
This later authority binding belongs in the durable admission and receipt; it
is never inserted into or used to regenerate the already-approved commit.
Verification follows both chains from owner records. Commit metadata supplies
an exact preparation link, not a self-authenticating create grant.

## Physical fence, lifetime and custody

This profile uses a **critical-section lease with no wall-clock expiry while
held**, rather than checking a TTL and later writing Git. The resource set is
the canonical Git ref, repository worktree/checkout topology, and the specific
integration worktree/private artifact namespace. One stable OS lock file per
Git common directory serializes the relevant owner mutations. Its inode is
never deleted/replaced during normal use. All enrolled ref and checkout writers
use this same owner/lock; a bare repository is convenient for fixtures but does
not itself exclude linked worktrees or unmanaged writers.

Under the exclusive OS guard the owner obtains a monotonically increasing
generation, checks that the operation is not closed or previously attempted,
checks the exact offer consumption and prepared binding, and
durably records its operation-bound lease/admission in SQLite before touching
the canonical ref through a one-time `Prepared` to `Admitted` CAS. A repeated
entry seeing `Admitted` or a later state returns inspection/recovery evidence;
it cannot resume another holder's physical attempt. The original guarded call
alone advances its admitted state to `AttemptStarted` and the effect. It checks
the current generation, expected parent and empty
target checkout set while retaining the guard. Canonical ref comparison and
mutation execute **in the same process while the guard is held**, using a
pinned Git library's ref transaction; no child that can survive the holder
performs that mutation. Other preparation/validation subprocesses have no
canonical write authority. The guarded ref transaction checks the expected old
OID as part of the update, not through a prior observation alone.

The lease stays current and unexpired through the physical effect because its
lifetime is that guarded operation, not elapsed time. No renewal/reassignment
or managed checkout can cross the guard. This gives `INV-036`–`INV-039` their
mutation-boundary meaning without weakening “unexpired” into “was unexpired
earlier.” It is not the existing Node TTL lease implementation. A queued
operation cannot take a newer generation while a prior admitted operation is
unresolved, even after the OS lock becomes available. Reclaim requires the old
holder's termination/guard release and read-only reconciliation; a timeout or
chatboard claim is insufficient.

The strong claim requires **selected managed repository custody**: no
writer can mutate the target ref, lock inode, operation records or checkout
topology outside this owner. W1's private fixture tests actual enrolled actors
and their enforced critical sections; same-UID fixtures do not establish
exclusion of arbitrary outside writers or hostile validation programs.
Production startup needs independently established filesystem/process
custody for the selected repository and its Git common directory, including
linked worktrees. A cooperative registration list or a `git worktree list`
snapshot is not proof. If custody is unavailable, the route refuses to claim
fenced publication and remains unqualified for production. In particular,
unmanaged `A→B→A` changes bypass generation fencing and pass an OID-only CAS;
unmanaged checkout after inspection can race a publisher. W0 does not hide
either limit behind a lease token.

## Recovery and result truth

SQLite, Git and campaign readback are separate durable effects. Before the
physical attempt, workspace commits `Admitted`, then `AttemptStarted` with the
immutable request, both binding digests, generation and holder identity. Both
transitions use the same guard/state exclusion as `close_no_effect`. A crash in
`AttemptStarted` makes that operation recovery-only; `publish` never repeats
the CAS to discover what happened. The prepared commit contains the operation
and precommit preparation digest; later human authority is verified separately
through the admission binding and its owner records.

| Durable/read-only evidence | Result and allowed consequence |
| --- | --- |
| Durable `ClosedNoEffect` receipt, created under the admission guard before `AttemptStarted` | `NoEffect` with reason; settle the consumed offer. The tombstone prevents a delayed publisher from entering the effect. No reuse of the operation or decision. |
| Admitted attempt plus exact target commit/parent/tree/message and authority binding, under established writer custody | `PublishedObserved`; persist the exact observation and settle the offer. Successful process return alone is insufficient. |
| Target equals prepared commit but admission or custody evidence is absent | `PublishedUnconfirmed`; retain uncertainty, never manufacture admission or mark created. |
| `AttemptStarted` but target is expected parent or another commit, or any required record is missing/conflicting | `Unresolved` with exact observations. Current ref value does not establish zero history; no retry, new generation or cleanup of required artifacts. |
| Publication receipt exists but campaign result recording was interrupted | Reverify workspace/Git binding and record that same result in campaign; no Git mutation. |

Inspection can record a known expected-parent mismatch as `NoEffect` only when
the guarded owner has not entered the physical attempt and durably closes the
operation before returning the receipt. After an uncertain
attempt, even apparent parent mismatch remains historical uncertainty. Operator
acknowledgement may record a recovery disposition but cannot fabricate success,
absence of effect or permission to repeat the same create. A future exceptional
repair route is outside W1; truthful unresolved state is a supported terminal
qualification result for those cuts.

## Exact W1 scope and acceptance

W1 is one joined offline domain delivery, not a generic lease crate. Proposed
file ownership is bounded as follows; the integration owner serializes the
shared manifests. Names are proposed new files unless marked existing.

| Owner | File/surface manifest |
| --- | --- |
| Campaign | `rust/crates/slice-campaign/src/publication.rs`, `publication_store.rs`, `publication_ingress.rs`; `tests/publication.rs`; existing `src/lib.rs` for exports and `Cargo.toml` for exact dependencies. The completion store is a fresh schema/root under this owner, referencing existing campaign readbacks; no modification/import of existing campaign rows. |
| Workspace | New `rust/crates/workspace-publication/{Cargo.toml,src/lib.rs,src/contract.rs,src/codec.rs,src/store.rs,src/git.rs,src/lock.rs,src/validation.rs,tests/publication.rs}`. Schema SQL may live beside `store.rs`; no shared universal persistence package. |
| Composition/client | New `rust/crates/work-engine-completion-host/{Cargo.toml,src/lib.rs,src/main.rs,src/bin/work-engine-completionctl.rs,tests/joined.rs}`. The library is the HP4-callable application boundary; the executable is the concrete W1 consumer. |
| Integration/docs | Existing `rust/Cargo.toml` and `rust/Cargo.lock` for member/dependency registration; new `docs/rust-workspace-w1-result.md` for exact candidate, profile and proof limitations. No changes to lifecycle, current review startup, UI, Python completion adapter or production dispatcher. |

The builder chooses/pins the in-process Git dependency and supported local
filesystem features within this contract; it must support atomic expected-old
ref transactions while the caller retains the OS guard. If the chosen library
cannot satisfy that, the implementation must change before claiming W1 passed;
substituting an unfenced child process is not a conforming shortcut. W1 records
the dependency/build and filesystem assumptions with its evidence.

Acceptance exercises the actual Rust host, owner APIs and reopened stores in a
fresh local repository. Git fixture creation supplies candidate objects; no
direct success-path table writes or hand-authored acceptance/decision receipts
substitute for those APIs. Required cases are:

- Exact owner review/gate readbacks plus fixture-operator checkpoint acceptance,
  isolated integration and integrated-tree validation, separate exact `create`,
  physical ref CAS, readback and campaign settlement all join successfully.
- Reported/incomplete review alone, forged actor/approval DTO, wrong peer UID,
  missing required claim, wrong checkpoint/manifest, stale offer, revoked
  eligibility and changed policy/parent are refused with their own reasons.
- Two actual publisher processes and a managed checkout actor cannot supersede
  the live critical lease. Pause the holder across a would-be TTL interval;
  generation remains current until release/reconciliation. A stale token never
  reaches canonical mutation. Different-parent CAS fails.
- Actual-process crash cuts cover durable consumption, admission, attempt
  start, physical CAS and pre-result finalization. Recovery distinguishes the
  table above and never performs a second canonical mutation. Expected-parent
  readback after an uncertain cut remains unresolved.
- Pause a publisher after it has checked campaign consumption but before
  workspace admission. A concurrent reconciler must durably close the
  operation under the admission guard before campaign `NoEffect`; resuming
  the publisher cannot mutate. Also cover publication winning the guard first,
  and a crash after the closure commit before campaign settlement.
- Recompute the precommit preparation digest from its named inputs, the exact
  commit and its metadata, and the later admission authority binding after
  reopen. The operation ID is stable from preparation through settlement;
  altered prepared/offer/decision references fail without rewriting the commit.
- Tampered/missing admission and a deliberately unmanaged ref/checkout actor
  demonstrate the custody limitation; they do not become successful fence
  evidence. An external ABA cannot be reported as detected by Git's OID CAS.
- The attached user branch/index/worktree, unrelated paths and accepted
  checkpoint refs remain byte/identity-equal; no push, tag or legacy execution
  occurs. The selected fixture validator performs no canonical mutation; its
  actual process capabilities and the resulting custody limit are reported.

W1 can qualify this controlled profile without root or a real operator account.
Production authority isolation and managed-repository custody remain explicit
deployment prerequisites, as does later HP4 dispatcher integration. They do
not block offline W1 implementation; they prevent its tests being mislabeled
as production activation or full host cutover. Contract acceptance and a W1
implementation grant remain the user's decisions after review of this document.

## Evidence scope

[C5](rust-compiler-c5-plan.md) and its retained reconnaissance packets establish
the missing Rust publication handoff and the existing Node domain consumer.
Additional source checks covered campaign identity/candidate types, workspace
mutation admission and promotion, shared Cargo declarations and
[`INV-025`–`INV-029`, `INV-036`–`INV-039`](../docs/workflow-invariants.md#checkpoints-and-publication).
The physical analysis is retained at
`/home/bline/.local/state/work-engine/w0-20261018/physical-boundary.md`; it also
records Git CAS, ABA and checkout limits. This design chooses the nonexpiring
guarded critical lease rather than that packet's tentative TTL reservation.

Tier 2 Codebase Memory project `home-bline-code-work-engine` was ready/watched;
additional coverage at `2026-10-18T05:26:52Z`, store `uc55b7fd5ab5d0cd7g2`,
reported verified content, matching metadata and no recorded gap on all seven
newly relied-on paths. Parent and retained packets cover the remaining exact
source paths. Relevant query pages completed. These are bounded static source
observations, not runtime or whole-repository absence evidence. No tests,
builds, provider calls, credentials, commits or activation were performed.
