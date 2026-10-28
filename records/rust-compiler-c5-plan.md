# C5 disposition and proposed workspace/publication contract

Planning result, 2026-10-18, following C4 commit `58abbfd`. The user authorized
this bounded planning pass and removed backward compatibility and import of
previous work from its scope: the earlier code was never released and need not
run again. This document records the resulting C5 disposition and recommends
the next contract slice. It does not authorize implementation, Git publication,
runtime activation, historical-state changes or other sessions' work.

**W0 follow-up:** the user subsequently authorized contract formation. The
[proposed W0 contract](rust-workspace-w0-contract.md) now specifies the local
operator ingress, exact owner records, guarded publication/recovery boundary
and W1 file/acceptance scope. It awaits review and acceptance; extension runtime
remains deferred and W1 implementation is not authorized by that document.

## C5 disposition

**Defer extension runtime integration until a concrete workload and semantic
owner require it.** C4 provides a fresh Rust attachment compiler and verifier.
In the bounded compiler/CLI/review/task-host/lifecycle investigation, the only
non-test Rust callers of extension compilation and verification were the CLI
protocol. The existing review host consumes manifest projection; that is not an
extension attachment load, registry or dispatch path.

The evidence is specific: [extension.rs](../rust/crates/work-engine-compiler/src/extension.rs)
`compile_extension_bundle` and `verify_extension_attachment` (lines 345–556),
[extension_protocol.rs](../rust/crates/work-engine-compiler-cli/src/extension_protocol.rs)
(58–188), and [review startup](../rust/crates/review-workflow/src/startup.rs)
(759–772). The [C4 result](rust-compiler-c4-result.md) already separates checked
attachment integrity/admission from runtime authority. No new host is justified
solely to create a caller for that compiler.

Reopening this lane needs a named first-party workload, its host owner and its
admission/dispatch contract. A useful future acceptance would demonstrate a
fresh bundle compiled and saved, loaded after restart, verified against current
host admission, registered and dispatched through that host's effect mediator.
Tampered bytes, stale admission, denied capability/provider, duplicate names
and unavailable dependencies would remain distinguishable failures. These are
conditional consumer acceptance cases, not unfinished C4 compiler obligations.
No old attachment import, dual reader, Node bridge or legacy execution is needed
for this fresh-state direction.

## Why workspace/publication is the next contract candidate

The [host replacement plan](rust-host-replacement-plan.md#subsequent-slices-and-composition)
names completion/publication as an HP4 dependency, alongside terminalization and
supersession. Its current concrete source consumer is Node
[`createCompletionPublicationService`](../app-server/src/services/slice-campaign/completion-publication.mjs)
(307–355), which calls workspace preparation, sealing, promotion, inspection and
reconciliation. This establishes a real domain use case. The successor Rust HP4
consumer remains planned and unqualified.

The missing seam is earlier than a generic lease or Git wrapper:

| Current Rust boundary | Meaning for publication |
| --- | --- |
| [`Campaign::prepare_initial_completion`](../rust/crates/slice-campaign/src/owner_readback.rs), 1113–1248, records `CompletionIntent` from checked execution evidence. | This prepares the review evidence join, not a Git completion offer. |
| [`Campaign::finalize_initial_completion`](../rust/crates/slice-campaign/src/initial_outcome.rs), 905–990, returns `InitialOutcomeProgress`; `InitialOutcomeView` at 174–187 carries verdict, phase, findings and exact evidence. | A committed review outcome is not an accepted checkpoint or human `create` decision. |
| [`Workflow::join`](../rust/crates/review-workflow/src/initial.rs), 676–716, returns that outcome through the actual Rust review consumer. | The observed route ends with review outcome/readback, not canonical Git publication. |
| [`CompletionSelection`](../rust/crates/slice-campaign/src/initial_outcome.rs), 19–141, installs finding/reliance authority; [`EvaluationRecord`](../rust/crates/slice-campaign/src/completion.rs), 624–635, records exact finding evaluation. | Neither grants Git mutation authority. Their publication fields concern evidence and evaluation. |

The [J2 contract](rust-j2-completion-implementation-plan.md#entry-outcome-contract-and-authority)
explicitly distinguishes `Reported`/`Completed` from review or campaign
acceptance. HP4 also leaves terminalization/acceptance and workspace publication
unavailable until their owners' complete contracts join. Treating one of those
existing review states as permission to commit would invent authority.

## W0 — form the exact campaign-to-publication contract

**Recommended next bounded deliverable:** an owner-reviewed contract for one
fresh Rust canonical-ref publication profile, plus the exact W1 implementation
scope and acceptance cases. W0 resolves the missing eligibility and human
decision boundary before choosing persistence mechanics. It produces no crate,
runtime, ref change or new grant.

The recommended profile takes an exact accepted checkpoint and separately
recorded human `create` decision, integrates in a separate worktree, validates
the integrated tree, and advances one configured canonical ref with fencing
and an expected parent. It refuses checked-out canonical targets, semantic
merge conflicts and unsupported profiles. This is a proposed profile choice
for the campaign and HP4 owners, not a claim that they have selected it.

W0's inputs are the current Rust review outcome/readback contracts above, the
Node completion/publication boundary as behavioral evidence, the configured
campaign acceptance and human approval authority, and the selected repository,
canonical-ref and validation policy. The latter configuration is not supplied
or accepted by this planning result.

W0's concrete output should settle these interface decisions in one contract:

| Owning boundary | Proposed information and observable result | Decision W0 must resolve |
| --- | --- | --- |
| Campaign completion | Owner-read eligibility tied to campaign identity/revision, accepted checkpoint commit/tree, checkpoint parent and complete attributed change manifest; exact acceptance evidence. | Which campaign transition establishes publication eligibility, and which Rust API returns it? Review completion alone is insufficient. Preserve any separate terminalization meaning. |
| Human decision, recorded by the completion owner | An exact offer revision/digest, decision identity and attributable `create` authorization bound to the selected checkpoint, repository and canonical target. Denied, deferred, stale and superseded decisions confer no effect authority. | Name the authority source, record owner, decision ingress/readback and invalidation rules. An agent-provided boolean or copied receipt is not a replacement. |
| Workspace/publication owner | Preparation bound to that eligibility and decision; observed target parent, integrated tree, validation policy/result, prepared commit and durable operation identity. | Freeze the request/response types, freshness checks, Rust-owned format/identity and recovery locators. Choose package placement with the integration owner. |
| Publisher and campaign readback | Physical effect receipt distinguishes no effect, published with admission evidence, conflicting state and unresolved outcome; campaign records that exact result without changing its meaning. | Define consumption, reconciliation and retry semantics across both owners. Publication evidence cannot manufacture acceptance or human approval. |
| HP4 composition | One named Rust entry route calls the campaign and publication owners under an installed profile; other profiles are refused before effects. | Identify the composition owner and offline qualification fixture. A standalone lease crate does not close this route. |

Publication changes the canonical branch, so authorization must cover the exact
attributed change and selected target. Integration can change the resulting
tree, so the selected validation must cover that tree. A stale worker can race
another publisher, so resource-matched fencing and expected-parent mutation
must agree at the effect boundary. A crash can hide a successful ref update, so
durable operation/admission evidence and observed Git state must support
reconciliation without blindly repeating the effect. These are the protected
outcomes; storage layout and method names remain implementation choices within
the accepted contract.

W0 is complete when the campaign, human-decision, workspace and HP4 boundaries
have named owners; their request/readback types and authority sources are
reviewable; and a paper trace binds one fresh accepted checkpoint plus one
human decision through successful publication, stale refusal and uncertain
post-effect recovery. Every required input must have an identified authoritative
producer. Any producer or authority that cannot yet be supplied is an explicit
unsatisfied prerequisite, not a W1-ready claim. The resulting W1 file/surface
manifest and validation criteria then support a concrete implementation decision.

## W1 candidate and proof boundary after W0 acceptance

If W0 selects the profile, the smallest useful implementation is the joined
fresh Rust campaign completion/publication path. Domain ownership would stay
split: campaign owns eligibility, offer and decision recording; workspace owns
resource admission, isolated integration and physical publication records;
HP4 owns trusted composition and route exposure. Exact crates and shared Cargo
edits remain the integration owner's decision. No compiler extension runtime
is a dependency.

Controlled qualification would use a fresh local Git repository and private
Rust state, exercising the accepted APIs through the selected Rust process:

- An exact human-authorized checkpoint reaches the canonical ref; its complete
  parent-to-tree delta equals the attributed approval manifest, and the actual
  integrated tree has the selected successful validation evidence.
- Missing/denied/stale authorization, wrong checkpoint or manifest, stale or
  wrong-resource lease, concurrent parent movement, checked-out target and
  semantic conflict refuse before the canonical mutation.
- Restart after the ref update but before result finalization reconstructs the
  operation from durable authority, admission and Git evidence. It does not
  invent confirmed success from the ref value alone or repeat an uncertain
  publication blindly.
- The attached user branch, real index, worktree and unrelated files retain
  their exact state. Qualification performs no push or tag and requires no
  historical import, legacy execution or dual runtime.

These are proposed W1 proof criteria, not tests run or gates passed by C5/W0.
They would establish one joined fresh profile, not full HP4 cutover, provider
qualification, all campaign mutators or retirement of other implementations.

## Separate publication contracts and coordination

The proposed canonical publisher follows the separate-worktree boundary in
[`git-publisher.mjs`](../app-server/src/services/workspace-coordination/git-publisher.mjs)
(151–181), which refuses a checked-out target. The
[`slice-completion-commit` adapter](../skills/slice-completion-commit/SKILL.md)
instead creates an approved commit on the attached user branch while preserving
unrelated index/worktree state. They share Git mechanics but have different
inputs, effects and authority contracts; W0 does not replace or merge them.

Fresh-state authority removes a compatibility/import obligation from this
candidate. It does not authorize deleting old state, changing another owner's
contract, bypassing exact approval or claiming that previous production was
retired. Lifecycle S9/S10, RP0 review startup and UI repair work retain their
owners. The shared Cargo/lockfile and HP4 route remain coordinated resources;
this planning pass claims neither.

## Evidence and limits

The retained reconnaissance packets are
`/home/bline/.local/state/work-engine/c5-20261018/consumer-evidence.md` and
`workspace-option.md` in that directory. They contain bounded source scopes,
exact traces and coverage results for the extension and Node workspace lanes.
No repository-wide absence of extension demand or Rust publication code is
claimed. Their broad `rust/crates` coverage query reported 72 gaps that were not
inspected and supplies no exhaustive evidence.

The additional seam check used Tier 2 Codebase Memory project
`home-bline-code-work-engine`, index `2026-10-18T05:12:49Z`, store generation
`u936ff96c90197408g7`. Relevant symbol pages were complete; the depth-one
non-test inbound trace for `finalize_initial_completion` returned the single
`review-workflow::Workflow::join` caller. Exact snippets and bounded source
reads establish the meanings above. Coverage for the four cited Rust seam
files, J2/HP4 plans, compiler plan and governing documents returned matching
metadata, verified content and no recorded issue on all 11 paths; that is a
best-effort signal, not completeness proof. No tests, builds, provider calls,
credentials, commits or activation were used for this documentation result.
