# S8 retained workflow implementation proposal

Prepared 2026-10-08 against `c3cfd7ec191a9cc06c718444d8855ae919e5d6ef`,
including published S6/S7 `d6ced89bfe6be964a095bc88cdac82cd69457085`.
This is **planning only**. It authorizes no implementation, provider execution,
live proof, saved-state operation, activation, publication or commit.

> **Later annotations.** Blocks headed *Current implementation* were added on
> 2026-10-11 against a working-tree snapshot based on `c72f7a5` (content
> manifest sha256
> `946887f3934c2d5541c41d54ce9fba8d7e1653460d853bac1a788dfa7fa2ac28`). They are
> not part of this plan and do not change its subject. Each points to where a
> symbol the section names is implemented in that later tree. *At subject* means
> a definition of that name existed in the crate at `c3cfd7e` (checked with
> `git grep`); it does not establish the same shape or meaning. *Introduced
> later* means none did.

The human selected **“Retained engineering/task workflow (Recommended)”** during
this planning turn. The concrete proposal is a permanent **retained
repository-analysis task host**, using the actual lifecycle service and its
bounded snapshot-read operation. The workflow family is selected; this exact
product profile and its new domain owner remain proposals. S8 admission is
**unresolved** until the host/lifecycle owners agree that concrete consumer,
profile, entry path and exact contract/source revision. The selection authorizes
neither implementation nor general editing. Current HP3 cannot supply S8 unchanged.

The owning scope is [S8](../experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md#s8--one-retained-workflow-through-the-complete-host-path),
[the lifecycle design](../experiments/context-lifecycle-rust/DESIGN.md), and its
[acceptance matrix](../experiments/context-lifecycle-rust/DESIGN_EVIDENCE.md).
This proposal does not amend those requirements.

## Decision and bounded outcome

One admitted task remains meaningful and correctly accounted for while active
work yields through two context generations. Two inputs queued behind that work
retain FIFO custody. Supported tools enter once under their actual owner, and
readiness and final results come from committed facts. The host process survives
ordinary context replacement under the selected provider capability. A separate
restart scenario proves recovery without passing off a new incarnation as the
same execution. The same installed service, host and scenario driver can later
be used with separately authorized live provider behavior.

The proposed first task is source analysis over an approved immutable repository
snapshot: answer an engineering question, preserve an unresolved approval and
canonical references, accept later clarification, and produce an attributed
analysis result. It requires no shell, repository mutation, campaign acceptance,
review independence, or automatic user-answer tool. An ordinary operator/Rust
client can submit such tasks to the permanent host outside a proof run. The
acceptance scenario supplies synthetic task content through that same interface.
If the product owner does not actually want this consumer, this route is invalid;
a new package named “task host” would not repair that missing product decision.

The proposed profile is `retained-repository-analysis-v1`, with a fresh-native-
thread-per-context contract and one active task delivery per subject. Subject
kind is owner metadata: repeat the scenario as `repository_analysis` and
`design_inquiry`, with equivalent contracts and the same lifecycle kernel.
These proposed names are not registered or authoritative yet. Task versus review
is no longer an unresolved human preference; the review alternative below records
the tradeoff considered before the selection.

Placement risk is high: task meaning, command admission, native execution,
semantic verification, custody, process lifetime and proof identity intersect.
The route is `falsified-placement`: bounded alternatives, an explicit conditional
recommendation, and one fresh targeted placement challenge. Retrieval is not
implementation review, and same-model fresh scrutiny does not establish provider
or reasoning independence.

The completed bounded challenge supports this conditional placement and falsifies
the narrower “host wrapper only” premise. Its findings are incorporated below:
native FIFO/yield/resume is real lifecycle implementation work; canonical task
meaning and its work cursor remain domain-owned; the current HP3 and passive UI
are not unchanged substitutes; snapshot-read supports a bounded analysis task,
not general editing. The challenge does not close the missing owner agreement.

## Evidence and alternatives

These are source observations, not new runtime results:

| Observed boundary | Planning consequence |
| --- | --- |
| `lifecycle-service/src/lib.rs:138` exposes `run_with_material_port`; `material_port.rs:19` requires projection, dependency observation and a currentness fence held over the short commit. | A permanent domain owner can supply canonical meaning without becoming lifecycle's writable store. The material adapter must be real owner readback. |
| `actor.rs:843–858` rejects native-profile commands other than `RequestReplacement`; `schedule_once:924` submits `ControlledPort` work. | Native queued delivery is a missing integration boundary. Wrapping the existing binary cannot establish S8's two queued inputs. |
| `operations/mod.rs:219–314` starts a thread and iterates a configured prompt/followup vector; it verifies `local_probe`. `actor.rs:341` launches that native task at startup. | Replace startup-script progression with durable admitted work and repeatable context allocation. Do not use configured followups as queue custody or a caller callback as progress. |
| `lifecycle-core/src/contracts.rs:193–238` defines controlled-text effect/input variants; `reducer.rs:418–510` prepares only the oldest eligible queued/prepared input. | Native delivery, yielded continuation attempts and their precise outcome join need lifecycle-owned representation. Preserve the existing FIFO owner rather than adding host scheduling truth. |
| `actor.rs:691–774` admits `workspace.snapshot.read.v1`, records exact child identity and returns committed results. `lifecycle-store/src/native.rs:870` binds native entry and `1381` qualifies settlement against ingress/history/children. | Reuse the bounded operation and its evidence contract; do not substitute terminal text for tool settlement. Durable native-turn evidence must be joined to the logical delivery. |
| `socket.rs:25–58` supports `read_only`, `controlled`, and feature-gated `native_simulated`; `lifecycle-service/Cargo.toml` declares the two proof features. | The accepted component profiles are not a general installed retained host profile. Production composition and provider selection require explicit qualification. |
| S6's material-port consumer is the test child `rust/tests/s6_semantic_preservation.rs::semantic_service_child`; the bounded trace of `run_with_material_port` reaches that caller. | Reuse S6 contracts and accepted evidence. This positive trace is not an exhaustive assertion that no other consumer exists. |
| HP3's proposed `review-host-controlled-initial-v1` is SC0-only, one obligation/attempt, with retained remediation deferred. Its actual child has unestablished review production-path claims. | Neither that proposal nor its future initial result establishes a retained-review consumer, lifecycle compatibility or independent review. |
| Lifecycle design §Context replacement distinguishes fresh-thread continuation from native same-session reviewer retention. | Selecting a reviewer must also resolve its native-session contract; persistence of an episode record is insufficient. |

| Candidate | Discriminating evidence / cost | Disposition |
| --- | --- | --- |
| Permanent repository-analysis task workflow and host | Avoids importing review/campaign authority and same-session reviewer semantics. Requires a real product decision, canonical task owner, permanent entry path and durable task records. | Conditional recommendation; no existing owner agreement claimed. |
| Owner-expanded permanent `review-workflow` / `work-engine-review-host` | Has an intended permanent consumer in the host roadmap, but current H0/H1 scope excludes retained remediation; needs accepted episode/campaign joins and a supported session/replacement contract. | Not selected: the human chose task workflow. A future review extension remains host-owned; S8 cannot absorb it. |
| Current HP3 H1 without extension | Narrow initial attempt cannot demonstrate retained work through two replacements and inputs. | Unsuitable for S8 unchanged. |
| Existing service component tests or a new proof-only host | Can exercise machinery but cannot supply actual domain admission or permanent consumer ownership. | Rejected as M4 substitute; useful only below the integration gate. |
| UI as host | UI's accepted boundary is a public read-only client; it does not own domain task meaning or lifecycle admission. | Keep as optional observer, not semantic host. |
| Temporary Node bridge or whole app-server migration | Offline development supplies no concrete need for the bridge; whole-root dispatch/deployment broadens S8 into separately owned work. | Not selected. |

No repository-wide absence or retirement claim is made. A new real consumer or
owner response can invalidate the conditional recommendation without changing
S8's acceptance conditions.

> *Current implementation* (later annotation; see the note under the title):
> - `rust/crates/lifecycle-service/src/lib.rs::run_with_material_port`, public as `lifecycle_service::run_with_material_port`; at subject.

## Provisional semantic-path certificate

**Verdict: unresolved at host admission; lifecycle placement is supported.**

| Element | Proposed binding |
| --- | --- |
| Trigger | An authorized task input, or a revision-bound pressure/yield request while that task is active. Waiting alone does not trigger replacement. |
| Producer | The permanent task owner projects exact objective/work position, governing material, completed consequences, decisions, commitments, uncertainty, questions/answers and approved source bytes. Native adapter produces bound observations; tool owner produces read evidence. |
| State owner | Task owner retains canonical domain records and their revisions. Sole lifecycle SQLite owner retains admission, inputs/deliveries, attempts, transitions, checkpoints, dependency bindings, custody and settlement. No host completion cache competes with it. |
| Consumer | Proposed `retained-task-workflow` library composed in permanent `work-engine-task-host`; ordinary operator/Rust client enters through its authenticated command interface. Lifecycle consumes its material port; successor and task result consumer use the verified continuation and exact task records. |
| Lifecycle | Admit input; own native attempt before send; quiesce/yield exact active attempt; capture and verify; allocate/rehydrate successor; verify semantic use and current dependencies; select successor; resume the same delivery under explicit authority; release later inputs only when earlier custody permits. Repeat without host restart. |
| Observable consequence | Correct task result and current canonical obligations after two replacements; stable delivery lineage and no repeated completed tool effects; truthful blocked/unknown/error state on failure. |
| Downstream proof | Installed permanent host and service, ordinary submit/observe path, controlled external native behavior, durable owner readback and independent entry ledger, then separately authorized same-build live semantic proof. |
| Insufficient substitutes | Test SQL edits, synthetic-owner journal in production, proof-only host, supplied readiness booleans, configured native followup vector, caller completion, files appearing, digest echo, or callback-driven advance. |

The remaining admission decision is concrete: **accept and assign the proposed
repository-analysis task owner/host and its exact profile/entry contract.** The
human's task-workflow selection settles the family, not that detailed binding.
Acceptance must name its host owner, lifecycle owner, source/contract revisions,
ownership, native-session policy and interface. Until that agreement exists,
this is a decision-ready plan, not an admitted S8 implementation.

## Host/lifecycle handoff before implementation

Freeze one small handoff record under the existing planning/acceptance owner,
rather than inventing another source of product authority. It identifies:

- The actual domain task/result consumer, host owner, lifecycle integration owner,
  exact source/contract revisions, allowed paths and ordinary entry command.
- Proposed permanent CLI `work-engine-task-host serve --startup <absolute-file>`
  and bounded authenticated task submission/observation protocol. Exact DTOs and
  profile revisions belong to that accepted host contract; these names are proposals.
- Trusted startup selection: exact executable/source/lock/toolchain/features,
  absolute path bases, approved snapshot and canonical-task roots, principal,
  grant issuer/scope/revisions, provider/profile, capability policy and limits.
  Request bytes cannot install roots, grants, executables or a material owner.
- Native execution and tool-loop ownership: one lifecycle execution binding over
  the pinned Codex adapter, one snapshot-read operation owner, no competing host
  provider loop. Domain host chooses meaning, not native settlement.
- Native-session policy: fresh threads allowed for the chosen task; same-native-
  session-required subjects refused unless their actual capability and authority
  are separately supported. Record host/process, executable, context and provider
  session separately.
- `CanonicalMaterialPort` implementation against durable task owner records:
  exact projection, current revision readback, and commit fence. Late human answers
  keep durable question/recipient/scope identities and invalidate only dependent
  meaning. This does not grant a model the ability to answer its own question.
- Native delivery/continuation contract: immutable input and logical delivery,
  work cursor, completed effects, yielded predecessor, successor transition,
  newly admitted resume attempt and unresolved effects. A new context grants
  neither replay nor overtaking. Recovery never reconstructs authority from a
  captured local permit or missing response.
- Cursor ownership is explicit: lifecycle binds the task owner's cursor reference
  and revision to custody/effect records; it does not become a second writer of
  domain progress. Native/provider waits must preserve the accepted host control,
  observation, shutdown and input-admission bounds.
- Separate host/domain and lifecycle recovery readbacks. No cross-store atomicity
  is claimed; exact durable locators allow idempotent reconciliation of actual
  owner commits. Lifecycle accepts task material only after owner publication.
- The task owner supports durable pending questions and authorized answer binding,
  and exact result publication. Propose result publication as a **commit-bound
  domain operation**, with its own authority revision/fence, entry/result identity
  and lost-reply readback. This writes task-owned records, not repository files,
  and provides a real commit boundary for T35. Lifecycle cannot attest that
  domain commit from its own transaction. Freeze this owner's concrete contract
  before admission; a read-only tool alone cannot prove the commit-bound branch.
- Chosen operating envelope, external-provider controls, proof consumption and
  identity contract. The current component profiles remain named and scoped.

The unselected review alternative would need a separately accepted route revision
and owner-issued retained-review handoff. Native episode, campaign, claims, E1,
reviewer session and review-independence contracts remain their owners'. Their
writable handles and path ownership do not transfer to this task workflow.

> *Current implementation* (later annotation; see the note under the title):
> - `rust/crates/lifecycle-service/src/material_port.rs::CanonicalMaterialPort`, public as `lifecycle_service::CanonicalMaterialPort`; at subject.

## Implementation boundaries and joins, conditional on admission

The future implementation configuration is separately accepted; the anticipated
builder is `gpt-6-sol/high`. This planning run requested/observed Astra/xhigh.
No model identity or implementation grant is inferred from those preferences.

| Lane / owner | Exact proposed path boundary | Required result / join |
| --- | --- | --- |
| Host/domain lane | New `rust/crates/retained-task-workflow/` and `rust/crates/work-engine-task-host/`, including their own `Cargo.toml`, `src/` and `tests/`; new `docs/rust-retained-task-host-contract.md` if accepted | Permanent normal consumer, bounded command entry, canonical task owner/material adapter, task-result readback and startup validation. No lifecycle store writes or second scheduler. These packages do not exist by virtue of this proposal. |
| Lifecycle integration owner | `rust/crates/lifecycle-core/src/{contracts,reducer,state}.rs`; `rust/crates/lifecycle-store/src/{lib,sqlite,native,semantic}.rs`; one new next-numbered store migration only if the accepted representation needs it; `rust/crates/lifecycle-service/src/{lib,actor,engine,admission,socket,semantic,semantic_control}.rs` and `src/operations/`; corresponding package tests | Join native entry/settlement to queued logical delivery, yield/resume and repeated context cycles; async owned results advance durable facts. Preserve accepted S67 semantic no-fallback/currentness, artifact and administrative recovery. A schema change needs S7 compatibility/export/adoption checks. |
| Provider/runtime portion, serialized by lifecycle owner | `rust/crates/lifecycle-codex/src/{protocol,harness}.rs`, `rust/crates/lifecycle-provider/src/`, `rust/crates/lifecycle-runtime/src/` only where a demonstrated required interface gap is confirmed; their exact tests | Supported interruption/terminal/history/profile and process ownership. These are conditional candidate paths, not permission for a broad rewrite; freeze the symbol/file delta at admission. Preserve one executor. |
| Public contract portion, serialized by lifecycle owner | `rust/crates/lifecycle-wire/src/`, `schemas/`, tests/fixtures and `rust/crates/lifecycle-client/src/` only for the accepted native enqueue/yield/observation contract | Compatible versioned command and observation boundary. Existing wire v1's qualified Unknown projection for internal Yielded remains truthful. If the consumer needs exact Yielded, add a versioned supported projection rather than silently changing v1. |
| Proof lane | New `rust/crates/lifecycle-proof/` with `Cargo.toml`, `src/`, `tests/` and scenarios; `rust/crates/lifecycle-test-support/` test-only peers/barriers; new `rust/tests/s8_retained_workflow.rs` if a workspace integration target is needed | One driver, ordinary host entry, immutable identity/receipt binding and owned setup/teardown. It cannot read writable internals or advance lifecycle. |
| Workspace/integration owner | `rust/Cargo.toml`, `rust/Cargo.lock`; affected lifecycle package manifests; `docs/rust-context-lifecycle-s8-implementation-plan.md` and an S8 result document | Additive dependency/test registration, exact manifests, integrated gate and handoff. Serialize all shared edits. |

The provider/runtime candidate paths are deliberately bounded but not all known to
require edits. The implementation owner refines them from the accepted contract
before touching them. New externally visible authority or schema semantics return
to their owner. No allowed path includes review/campaign/claims product changes,
terminal UI, unrelated skills or the legacy Node host. The shared workspace is
dirty; baseline status and 87 tracked dirty-file hashes are retained externally.
The planning task owns only this new document and its external planning directory.

Parallel work is optional after interface acceptance: the host/material adapter
and lifecycle delivery implementation can develop against agreed types while the
proof lane builds external process/identity scaffolding. One lifecycle owner
serializes core/store/schema/service/wire changes. The serial joins are (1) actual
host agreement, (2) native delivery/material-currentness contract freeze, (3)
smallest vertical proof, (4) complete two-cycle scenario, (5) fault/compatibility
qualification, and (6) immutable composed candidate and supervisor-selected review.
No lane's component pass establishes the final host boundary.

## Smallest vertical proof and complete scenario

First prove one ordinary task input through the permanent host: publish canonical
task records and an approved snapshot, obtain real lifecycle admission, execute
one accounted snapshot read, reserve a context transition, disconnect the client,
and let the installed service capture/compile/verify/rehydrate/assess/reconcile.
The successor continues that same task using the task owner's records and exact
checkpoint; a read-only reconnect sees the committed consequence. Fail this proof
if the host cannot supply actual meaning/currentness or the executor needs test
advancement. Do this before broad implementation or presentation work.

Then extend that exact path, without changing owners:

1. Start active delivery D0 and enqueue Q1 and Q2 behind it through normal host
   submission. Capture immutable payloads, command IDs, delivery IDs and sequences.
2. Exercise a supported snapshot read and preserve its completed consequence.
   Admit a revision-bound pressure/yield request targeting D0. Reserve transition A
   before interruption; also test the natural-terminal-wins race. The provider's
   actual outcome and effect settlement remain distinct from yield.
3. Freeze current semantic basis with unresolved approval, a superseded decision,
   a commitment, a governing instruction, a canonical reference and uncertainty.
   Run separate compiler/verifier effects and qualified successor readiness.
   Next domain input must not repeat the answers being tested.
4. Select successor A and transfer custody without thereby starting Q1. Resume D0
   under explicit continuation authority with its prior work cursor/effect receipts.
   Drive transition B while D0 is active, then resume and complete the same delivery.
   Do not replay its prior read or allow Q1/Q2 to overtake it.
5. Admit Q1 then Q2 only when prior custody permits. Hold Q1 after entry and prove
   Q2 cannot start. Final task/result projection consumes exact owner facts.
6. Repeat equivalent contracts under the second subject-kind label. Record one
   persistent host PID/birth/incarnation across ordinary context changes and exact
   distinct context/native session identities. Run declared restart lineage in a
   separate case with preserved custody, not inside the same-execution receipt.
   Keep the native harness process alive as well when its selected multi-thread
   capability permits it; finite compiler/verifier child lifetimes are distinct.
7. Validate the candidate receipt before teardown; close/drain/reap owned resources
   and preserve both primary and cleanup errors. Return proof success only after
   all required close evidence qualifies. Any offline preparation is later and
   follows truthful quiescence.

The controlled provider substitutes external native responses, tool requests and
fault schedules. It must not fabricate lifecycle tables, owner currentness,
readiness acceptance or final delivery records. Freeze one installed host/service
and scenario driver for controlled and later live use; authorized provider behavior,
credentials/quota and fault controls may differ. A changed binary or semantic route
requires a new controlled qualification. Controlled semantic fixtures prove
mechanical acceptance/refusal; live semantic adequacy remains unestablished here.

## S8 acceptance mapping

Every row is a future gate, not a pass claim. Component results may support a
specific unchanged clause but cannot replace its listed retained-host consequence.

| Requirement | Required retained-host evidence |
| --- | --- |
| M4 | Full installed host scenario above: active work, pressure/yield, verified checkpoint, successor, two queued inputs, tools, second replacement and separate restart. |
| T02 | Offer the same subject to selected Rust and ineligible predecessor paths; only current durable selection admits. Unavailable selected owner remains fenced. No dual-live-engine requirement or fallback bridge. |
| T10 | Drop initiating waiter at acknowledged entry; autonomous applied facts reach readiness and custody. Same driver/install for later authorized live profile. |
| T11 | D0/Q1/Q2 exact payload/sequence/delivery, custody acceptance distinct from delivery, Q1 hold blocks Q2, restart preserves lineage. |
| T12 | D0 yields through A then B under new authorized attempts while one logical delivery and completed-effect ledger remain stable; no replay/overtaking. |
| T13 | Successor uses the six realistic semantic obligations without them being supplied again; omitted/contradictory obligations fail. Controlled and live evidence classes remain distinct. |
| T14 | Forged material/invocation, relevant canonical/grant revision change and late answer cause refusal/re-evaluation; unrelated subject revision does not stale the package. Real host material port holds currentness at commit. |
| T15 | Selected task permits fresh native threads; a same-native-session-required subject is rejected before unsupported replacement. A reviewer cannot be silently converted to this profile. |
| T16 | Text success and the admitted snapshot read settle only from qualified evidence. Exercise an actual task-owned pending question/authorized-answer path; native completion cannot erase that accepted interaction. Denied tool request, running tool, child/remote effect and missing-descendant evidence remain distinct; unsupported effects do not become success. If the accepted interaction path is absent, report that required branch uncovered, not passed by an unsupported-tool refusal. |
| T17 | Wrong/unleased/stale/duplicate switch and unexpected compaction cannot be adopted by a later valid transition. Native observation lineage remains exact across both cycles. |
| T19 | Dropped/reconnected clients, missing notifications and cursor gaps do not advance or abandon execution; timeout is an observation outcome. |
| T20 | Independently mutate run/consumption/build/profile/root/incarnation and override attempts; each fails. Same-build process replacement invalidates same-execution proof. Setup already owns cleanup during delayed capture. |
| T21 | Fault discovery/startup/observation/receipt/close, including not-attempted/rejected/timed-out/late close; exact owned exits and primary error survive. |
| T22 | Acknowledge before-registration and spawn boundaries, close launch gate, prove no escaped child. Keep declared restart separate from invalid same-incarnation continuation; late resolution does not erase unsafe-close history. |
| T27 | Maximum accepted envelopes with diagnostics off/on; wrong/malformed identity, timeout, partial publication and absent acknowledgment fail boundedly. Cleanup allowance never extends proof authority. |
| T29 | Enumerate each operation actually registered by installed profile; exercise real admission/adapter entry, exact unsupported refusal, dependency closure and the eight responsibilities below. No direct-provider bypass. |
| T30 | Preserve observed raw native usage and disjoint normalized units, unavailable versus zero, no duplicate control/rehydration charging. Price revision and conditional payback require actual pinned inputs; no invented forecast or optimum. |
| T31 | Same kernel and equivalent contract under both subject kinds; no role-specific transition class. Host process survives normal replacement where provider permits. |
| T32 | Model permutations of queue/terminal/contradiction/deadline/restart/grant changes, plus actual-process disk/busy/full, result saturation and observation gaps. Assertions cover every committed state; Loom, if needed, only local synchronization. |
| T33 | Selected Linux socket/principal isolation, denied commands, permissions/redaction, frame/task/material/retention limits and owned cleanup. No same-UID hostile-code isolation claim. |
| T34 | Join harness-owned native turn and domain-owned snapshot read through one lifecycle accounting boundary. Wrong contract/implementation/executor or duplicate tool-loop ownership is refused before unauthorized entry. Host meaning never grants tool permission. |
| T35 | Race grant contraction with native/tool entry and with the proposed task owner's result-publication commit. The read's explicit entry-bound guarantee remains scoped; prior invalidation must win against a commit-bound task mutation at its actual owner. Kill/restart between entry, commit and receipt and recover exact result or uncertainty, never retry by absence. Reject unspecified validity/incomplete invalidation coverage. If no accepted real commit-bound operation can establish this branch, S8 reports it unestablished and cannot close that requirement; snapshot reads do not waive it. |

T18 timing checks, T23 artifact publication, T24/T28 compatibility and T36/T37
dependency/profile checks are supporting gates where the S8 delta reaches them.
This does not claim S9 migration or S10 replacement qualification.

| Legacy responsibility | Exercised path or justified replacement |
| --- | --- |
| `context_attestation` | Host canonical material projection + exact native predecessor/history identity, bound to the actual task. |
| `semantic_compiler` | Entered compiler invocation, source/input/output/provenance and applied result. |
| `semantic_verifier` | Distinct qualified verification effect and sufficiency findings; selected independence policy recorded without inventing independence. |
| `retirement` | Authorized quiescence, settled/accounted predecessor and exact fresh-thread switch. This replaces an in-place reset claim; no reset is asserted. |
| `rehydration` | Exact accepted checkpoint and governing bytes enter successor under narrowed control permissions. |
| `readiness` | Actual successor semantic use, verifier outcome, current dependencies and native identity jointly authorize readiness; digest echo fails. |
| `reconciliation` | Atomic lifecycle successor selection and custody projection; ordinary delivery still needs its own admission. |
| `blocked_probe` | Ordinary host task/tool attempt during transition is denied, with external entry count proving zero unauthorized dispatch. |

## Future runtime envelope and validation

Proposed first envelope, to be accepted with the profile: Linux, one trusted UID,
one subject per service instance, one active domain delivery, two pending inputs,
one native tool loop, one owned snapshot read at a time, and bounded isolated
semantic control invocations. Use existing limits as ceilings until measured:
128 KiB transport frame, 64 KiB input/material response where admitted, 30-second
maximum observation wait, and bounded snapshot ranges. Do not increase the
current 2 KiB native domain-prompt limit implicitly; select and validate the
needed new limit in the accepted profile, with separate material bounds.

The complete happy path budgets 13 native turns as an initial scenario estimate:
five domain turns (D0, two resumes, Q1, Q2) and four control turns per replacement.
This is not a provider allowance or claim about wire request count: tool-loop
continuations and any extra semantic challenge must be measured separately and
frozen before implementation acceptance. Use no automatic retries. Propose a
120-second controlled scenario deadline and separate 15-second cleanup budget;
if measured requirements differ, record the profile change before qualification.
Negative scenarios receive their own fixed counts/deadlines and fresh disposable
roots. Live provider, model, quota, deadline and semantic acceptance remain a
separate human-owned future grant; the old consumed live allowance is unusable.

All future commands below run with
`TMPDIR=/home/bline/code/.work-engine-tmp/s4-20261007/tmp` and
`CARGO_TARGET_DIR=/home/bline/code/.work-engine-tmp/s4-20261007/target` (or an
accepted successor location outside `/tmp`). No build/test command ran in this
planning task. The proposed new target names become exact only after manifests
are frozen; list their tests and validate the expected inventory before relying
on any selector.

```sh
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo test --manifest-path rust/Cargo.toml -p retained-task-workflow --locked
cargo test --manifest-path rust/Cargo.toml -p work-engine-task-host --locked
cargo test --manifest-path rust/Cargo.toml -p lifecycle-proof --test retained_workflow --locked
cargo test --manifest-path rust/Cargo.toml -p lifecycle-proof --test identity_and_cleanup --locked
cargo test --manifest-path rust/Cargo.toml -p lifecycle-proof --test recovery_and_faults --locked
cargo test --manifest-path rust/Cargo.toml -p lifecycle-service --features native-sim-proof --test s6_semantic_preservation --locked
cargo test --manifest-path rust/Cargo.toml -p lifecycle-service --features native-sim-proof --test s6_s7_s5_native_compat --locked
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace --locked
git diff --check
```

The command list is necessary but not sufficient. Freeze the production and
controlled feature combinations, exact installed binary/scenario hashes and
provider-disabled environment; enumerate intended tests and skips. The proof
targets launch actual installed processes rather than treating `cargo test`
return as host evidence. Add affected package property/compile-fail, schema
freshness, actual-process crash and S7 compatibility/export/adoption selectors
to the ordered gate manifest when the delta reaches those contracts. Do not run
unrelated historical gates merely to recreate already accepted S67 evidence.
No `--all-features` shortcut may accidentally admit live or admin capabilities.

The configured `engineering-proportional` requirements map as follows:

| Requirement | Gate |
| --- | --- |
| `semantic_proof` | Smallest real-consumer vertical, then complete M4 scenario and the explicit semantic negatives; controlled evidence labeled as such. |
| `risk_proportional_checks` | All S8 matrix rows above, shared runtime/store regression breadth, exact profile/startup/operation coverage, fault and compatibility checks; supervisor selects review after immutable candidate. |
| `workspace_integrity` | Exact baseline/attributed file manifest, unrelated index/worktree preservation, source/lock/build/profile hashes, generated-output freshness and `git diff --check`. |

Use the canonical Slice Builder gate runner with ordered argv arrays after an
implementation grant. Changed source, profile, schema or model/provider binding
invalidates dependent gate evidence. Review selection belongs to the supervisor;
normative host/semantic instructions will require an explicit instruction-review
disposition. Neither this plan nor the placement challenge accepts implementation.

## Risks, decisions and deferred work

The material decision is the exact permanent task consumer/profile and assignment
of its canonical owner. The human has selected the task-workflow family. The
proposed host must earn admission on ordinary task behavior, not on having enough
fixture knobs to pass M4. A future retained-review extension stays with its
existing host/domain owners and supported session contract.

The highest implementation risks are conflating native turn completion with
delivery success, resuming yielded work without explicit authority, capturing
domain meaning in a second lifecycle-owned journal, losing owner-currentness at
successor commit, and restarting the host to simulate ordinary replacement.
The vertical proof addresses those before the broad gate. Provider-qualified
interruption and real semantic adequacy remain observations to establish, not
guarantees inferred from Rust or the model name.

S9 retains actual saved-state import versus authorized archive/clean start,
current-custody rollback, selected permanent deployment/startup, old executable
refusal and historical-codec qualification. No actual state choice has been made.
S10 retains frozen complete replacement review, all T01–T37 coverage decisions,
separately authorized bounded live proof and human replacement acceptance.
Activation, shutdown of legacy production, queue disposition, deployment,
publication and rollback execution retain their exact authority boundaries.

## Planning provenance

Tier 2 Verify, repository-evidence provider `codex-codebase-memory` resolved from
the original version-2 campaign preflight; no provider/model substitution.
Graph project `home-bline-code-work-engine`, generation
`2026-10-08T21:22:09Z`, status ready. Exact lib/actor/operations/native searches
were untruncated; the relevant `run_with_material_port` and native-entry traces
were bounded and untruncated. Broad lifecycle and host searches were candidate
discovery only and were not exhaustively paginated; no absence claim rests on
them. Heuristic trace edges with unrelated type names were not treated as proven
calls; exact source controls those details.

Coverage for the 25 material repository paths reported matching metadata and no
recorded issue, a best-effort signal rather than completeness. One guessed
`lifecycle-store/src/queue.rs` path was missing; it is not evidence or an owned
edit target. Exact queue representation was read in the graph-discovered
`lifecycle-core/src/reducer.rs`. Direct document/config/source observation was
used for normative text, profile literals and concrete async/control behavior;
that evidence mode does not change retrieval-provider identity. Cargo lock and
toolchain hashes are identity-only inputs, not dependency audits.

Final planning-artifact coverage was recorded at generation
`2026-10-08T21:39:11Z`. The new plan, repository instructions and invoked repository
skills had matching metadata. `rust/Cargo.lock` was not tracked by the graph;
its exact bytes were read directly for identity hashing only, with no dependency
or structural claim inferred from the index.

The supervisor's one fresh Astra/xhigh challenge is
`placement-challenge-result-v1.md`, SHA-256
`c721a06881a62cc04e68542509fee38f7a8a29703f38874cbfb224b2afcb4cde`.
It separately checked 16 source paths, including the passive terminal UI, with
matching best-effort coverage. That attributed result supports the UI alternative
disposition; it is source-only planning evidence and makes no runtime or
independence claim. It preceded the human's task-workflow selection.

Baseline/source hashes, original preflight, authority, provider resolution,
coverage, selected challenge/disposition and the compact decision packet are
retained under
`/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-planning-20261008/`.
The earlier S67 compact handoff supplies accepted semantic consequences; its
publication-deferred wording is superseded by the two published commits above.
No S67 audit/transcript was re-consumed and no passing S67 gate was rerun.
