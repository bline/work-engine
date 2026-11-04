# S10 and root/Rust convergence — Pi, Docker and existing MCP events

Status: user-selected planning direction, 2026-11-03. This amendment selects the
next integration route and first working-base milestone. It authorizes planning;
execution, subscription sign-in/inference, deployment and release still need
their own exact assignments. All earlier S10 and Docker allocations are closed.

The maintainer clarified the architecture: **a deterministic system launches
Docker containers for contract-bound inference; most local supervision stays
inside the container; context and budget pressure arrive from outside**. A
standing agent at the top level is explicitly not the target. Existing root
skills/workflows supply useful behavior and contract material, not another
always-running supervisor model.

Progress: the maintainer selected a host-first component within the original
attempt's cap. Its [accepted result](../records/rust-lifecycle-s10-pi-docker-host-component-result-20261103.md)
now supplies provider-disabled Pi/Docker admission, custody, cancellation,
recovery and receipts in `work-engine-manual-host`. The full working-base
milestone below remains unmet: real inference, engineering/review execution,
credential-bearing containment, actual event delivery and canonical lifecycle
integration were explicitly deferred, not waived. No automatic next allocation
follows from component acceptance.

## The first working base

Prove one complete engineering job driven by a small deterministic Rust host.
It admits a bounded contract, launches the selected Pi/Docker lane, observes
events and limits, invokes the required review/repair continuation under that
contract, and returns an attributed result for the authorized acceptance owner.
The container owns local planning, coding, testing and semantic judgment. The
outside host does not spend inference on routine polling, progress summaries
or tool-by-tool supervision.

This joins the demonstrated root workflow to Rust-owned execution and state.
It does not require first replacing every useful script or building a general
organization compiler. Deterministic parts take over mechanically defined
responsibilities; models retain the bounded judgment their contracts require.
Parsing a model's verdict is not an independent verification of its meaning.

The milestone is a usable **development base**, with supported capabilities
and limitations stated explicitly. A snapshot-only grant cannot authorize an
engineering mutation job. Its execution packet names the actual permitted
workspace/effects, review contract and acceptance owner. This milestone does
not silently satisfy the original full S10/M6 or standalone release conditions.

## Responsibilities and the initial small contract

| Owner | Responsibility |
| --- | --- |
| Deterministic Rust host, outside containers | Admit current job authority and limits; launch/bind roles; persist owner state; consume typed events/results; enforce no-new-work/cancel/collect decisions; invoke configured checks and report terminal standing. |
| Pi lane, inside its Docker boundary | Own the whole bounded inference job, useful retained conversation and local supervision. Apply selected root skills and tool contracts; return artifacts, evidence and unresolved obligations. |
| Docker transport/custody adapter | Realize role-specific filesystem/network/credential access and container lifetime; bind container, process, session and exported artifact identities; collect actual wait/exit/removal evidence. |
| Existing MCP event/delivery components plus the qualified lane join | Target addressed context, budget, control and completion delivery using existing source/pending/acknowledgment semantics. The unified inbox and Pi recipient path are unfinished; events create no task authority or effect settlement. |
| Existing Rust semantic owners | Retain task/grant, operation identity, review, lifecycle, uncertainty, acceptance and publication meanings. Adapters invoke those owners rather than creating competing stores. |

The initial job contract needs enough information to distinguish authorized
execution from an invalid result: exact input/source basis, selected instructions
and tools, permitted effects, role/subject identity, total budget/deadline,
required outputs/checks/review, continuation limits and acceptance owner. These
are contract consequences, not a universal schema to implement before the first
job. Exact storage/DTO placement belongs to the smallest justified integration.

The [verified Rust reuse map](s10-rust-reuse-map-20261103.md) makes
`work-engine-manual-host` the first mechanical reuse anchor: it already handles
admission, identity/budget binding, foreground observation, exact-turn waiting,
collection, receipts and ACK/recovery. Its current Codex/Python adapters and
outside-root decision boundary need adaptation. Existing task/lifecycle/review/
completion owners remain separate; their presence does not establish a joined
Pi/Docker path. Reuse them for the selected job rather than require every crate
or rewrite every script first. The
[earlier convergence reconciliation](work-engine-convergence-roadmap-reconciliation-20261103.md)
maps useful owners. Its root-directed top-level mechanism is superseded here
for this selected base by the maintainer's deterministic-host direction; its
domain ownership and original release distinctions remain useful.

## One pressure path and one continuity owner

Reuse the MCP event components already being developed for outside collaboration;
do not create an S10-specific notification service. Current Events MCP facade/
subscriptions/callback delivery and the local chatboard are separate surfaces.
The event owner's board4302 clarification states that the unified service-owned
board/context inbox, delivery without an agent-managed wait, consolidated budget/
completion route and Pi recipient are unfinished. The missing join therefore
includes actual recipient delivery and acknowledgment, not just attaching Pi
to an already unified push path. Reuse observers, delivery state and accounting
where their contracts fit; active funded assignments retain their owners.

The root's chatboard/control-wait path supplies Git-ref CAS messages and
session-bound observations to an active listener. `--wake` does not start a
session, and manual-host uses foreground readers/native dispatch rather than
an MCP subscription. None of these transports preserves semantic continuity
or authorizes a context switch. The selected deterministic service owns outside
observation and delivery; an agent-managed wait loop is not its deployed
supervisor. See the [current reuse/evidence limits](s10-rust-reuse-map-20261103.md#event-integration-standing).

Board4303 assigns the backend session's next contribution to the shared delivery
foundation, beginning with an isolated receiver. S10 retains Pi/Docker execution,
containment, session controls and lifecycle integration. Align the
[proposed receiver boundary](s10-rust-reuse-map-20261103.md#shared-delivery-contribution-and-receiver-boundary)
before source work crosses that interface. The split does not claim that either
join is delivered, select crate placement or create another supervisor.

The [Rust lifecycle replacement protocol](../experiments/context-lifecycle-rust/DESIGN.md#context-replacement-protocol)
owns durable continuity: fence new work, capture the safe boundary/unresolved
effects, compile and verify continuation, authorize/select a successor and
reconcile queued inputs/current authority. Pi supplies the actual session
operations through a qualified adapter. Its compaction event or lower context
reading cannot substitute for those owner transitions.

Use one authoritative lifecycle path. An independent Pi auto-compaction/reset
loop must not silently bypass the selected continuity contract. The exact first
profile establishes controlled session changes and truthful handling of
provider-initiated loss; until then, Rust/Pi lifecycle integration is unqualified.
No existing Codex-specific provider control is presumed compatible with Pi.

Outside pressure observations bind the job, container/session, source, epoch,
model/window and freshness. Missing or stale measurements remain unavailable.
The receiver records what reached the actual target and distinguishes transport
acceptance from agent observation and completed work. Pi's documented steering
arrives between tool batches and the next model call; it is not an immediate
interrupt of an in-flight request. A hard limit remains an outside-host admission/
cancel/collection responsibility, with reserves for delayed and in-flight usage.
No zero-tail or exact subscription-billing guarantee follows from cancellation.

These requirements preserve the existing
[session-control distinctions](../skills/manual-orchestration/references/context-control.md)
and [event-delivery ownership](work-engine-convergence-roadmap-reconciliation-20261103.md#existing-work-and-its-owners).
MCP is the collaboration interface; it is not a second authority ledger.

## Why Pi and Docker change the candidate route

The accepted [Docker mechanics result](../metrics/probes/manual-host-docker-20261103.md)
demonstrates retained builder/review/correction/rereview conversations and durable
exports. Its shared-container, unrestricted-network, readable-auth and reviewer
exposure limits remain unqualified. Reuse that work and its owner evidence;
there is no new S10 sandbox project or permission to assume production isolation.

Pi documents a selectable tool registry, pre-execution blocking `tool_call`
hooks, final provider-payload inspection and RPC session controls. These are
specific candidate interfaces for the control gap that stopped the
[native runner attempt](../records/rust-lifecycle-s10-runner-feasibility-stop-20261103.md),
not runtime acceptance. See [Pi extensions](https://pi.dev/docs/latest/extensions)
and [RPC lifecycle](https://pi.dev/docs/latest/rpc#run-lifecycle).

MCP is explicitly part of the supported candidate interface. The selected
server/tool inventory must cover direct and indirect exposure: a plain Pi
`--tools` list can leave MCP tools reachable. The existing Work Engine MCP read
surface does not grant transition or mutation authority. Actual task operations
stay with their owners. See [Pi tool selection](https://pi.dev/docs/latest/cli#tools).

Selected root skills remain available inside the lane. Controlling automatic
discovery is a way to freeze startup inputs, not a decision to discard those
skills. Pin the role-relevant instruction/tool set and inspect the actual final
provider request. A custom Pi prompt can still include structured sections such
as working directory/context unless configured otherwise. See the
[prompt builder](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/system-prompt.ts).

Subscription-backed authentication is documented, but the installed version,
selected route/account/model and usage treatment still need evidence. Pi labels
`openai-codex` legacy and also supports ChatGPT sign-in through `openai`.
[Pi authentication source](https://github.com/earendil-works/pi/blob/main/packages/ai/README.md#oauth-providers)
and [OpenAI plan-usage documentation](https://developers.openai.com/siwc/token-sharing-open-source)
do not establish this user's runtime access or task-attributed quota savings.

## One vertical proof toward release

The next execution packet should deliver one coherent host-to-container-to-result
path, with an early feasibility checkpoint and one total numeric ceiling covering
setup, adapters, inference, validation, review, repairs and closure. It should
consume qualified shared work, identify remaining owner joins and stop if its
premises or full forecast fail. Subtasks do not reset the limit or automatically
become separately funded prerequisite projects. No total remaining-release
estimate is established by this planning amendment.

The complete proof demonstrates:

- A real bounded engineering task with pinned inputs, actual permitted tools
  and effects, meaningful checks and the required subject-bound review. The
  container performs local supervision without an outside supervisor-model loop.
- Outside context/budget events reaching the correct active lane, plus unknown/
  stale data, cancellation, limits and collection with truthful dispositions.
- Restart/session continuation and one Rust-governed context transition with
  pending authority, queued input and uncertain effects preserved. Use the
  existing lifecycle/review consequences applicable to the exact claim.
- Durable raw events/session data, host-owned receipts, usage/context observations
  and artifact identities that survive container removal. `agent_end` is not
  settlement: the Pi adapter uses its documented `agent_settled` boundary and
  still performs required process/domain collection.
- Whole-episode cost and accepted useful output. Match model, task, tools, tier,
  cache conditions and retries before claiming an efficiency improvement; an
  account-wide usage-meter change cannot attribute one concurrent job's quota.

The separately identified retained-snapshot profile keeps its one-read,
two-replacement/restart, authority/FIFO and applicable recovery requirements.
Passing the base engineering job does not import those passes, and passing that
profile does not imply arbitrary engineering mutation is admitted. Original
17-predicate/T01–T37 meanings and all stronger unresolved history claims stay intact.

## Simplification and retirement

| Existing work | Treatment after exact parity is demonstrated |
| --- | --- |
| Codex-specific prompt/tool-surface workarounds and unbounded S10 observer repair route | Defer further expansion for this selected route; Pi/Docker supplies a candidate boundary to qualify. Preserve historical evidence and stronger original claims. |
| Session, event, context and process glue duplicated by accepted Pi/Docker interfaces | Replace the migrated consumers with a small Rust adapter and remove the exact redundant paths after consumer/receipt compatibility checks. No deletion count is promised. |
| Working root skills and engineering judgment | Project the useful role contracts into the container; deterministic host mechanisms own routine lifecycle/pressure bookkeeping. No standing outside model supervisor. |
| Task/grant, uncertainty, review, evidence, budget and publication owners | Reuse and join them. Pi statistics or completion events do not replace domain authority or acceptance. |
| Existing Codex/Claude routes and active consumers | Preserve their contracts until an explicit migration/retirement scope applies; no automatic commitment to three fully interchangeable backends. |

First ship the supported deterministic-host/container development base. Add
deterministic responsibilities incrementally where their inputs, checks and
transitions are sufficiently defined. Operational replacement still requires
applicable S9 disposition/compatibility/rollback, bounded live evidence and M6/
release-owner acceptance. This amendment neither publishes a release nor
waives those conditions.
