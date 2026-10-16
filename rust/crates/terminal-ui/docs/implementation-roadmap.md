# Work Engine UI implementation roadmap

## Status and purpose

Reconciled 2026-10-15 (America/Denver; 2026-10-16 UTC) at repository
`f64bc61dd5b618d9531e7937e3494fe459dc1ea0`. The operator requested a bridge
between the accepted product design and the maintained terminal UI.

This is the current implementation roadmap and evidence map. The remaining
sequence is proposed work, not a new product-contract acceptance, campaign
launch, provider authorization, or commit approval. It preserves the accepted
UI-0–UI-4 outcomes and distinguishes implemented behavior from demonstrated
acceptance. No overall completion percentage is meaningful across these scopes.

## Design ownership and reading order

| Document | What it owns |
| --- | --- |
| [Control-plane causal observability UI](../../../../app-server/docs/control-plane-causal-observability-ui.md) | Accepted runtime and execution-history product design, five evidence classes, causal provenance, UI-0–UI-4 outcomes and acceptance exclusions. |
| [Studio reconciliation](../../../../app-server/docs/work-engine-studio-reconciliation.md) | Placement within the broader product; Contract/design, Organization, Control, and design-to-runtime correlation extend beyond causal observability. |
| This roadmap | Mapping to implemented increments, remaining dependencies, evidence gaps, and recommended delivery order. |
| [Terminal UI README](../README.md) | Current executable behavior, launch options and validation commands. |
| [Task observation reconciliation](task-observation.md) | Historical task-consumer proposal, with a current disposition pointing to the accepted host contract. It does not replace the broader product design. |
| [Standalone UI exploration](../../../../app-server/docs/opentui-ui-exploration.md) | Design lineage and interaction experiments. Historical stack alternatives and old integration status are not the current roadmap. |

The product remains provider-neutral work supervision: what is happening, what
needs attention, which evidence explains it, and which owner can act. The Rust
terminal client is a presentation of that product, not its semantic owner.
A retained task, campaign, slice, candidate, review episode, provider attempt,
and Git branch are distinct identities. Their joins require owned references.

## Established starting point

The terminal-ui subtree at the evidence cutoff is unchanged from the committed
UI integration `d2e5c530486151e9248848ad229b1719dd35e07f`. S8's client/host
integration was published separately at `4d99ded7`. These are bounded source
baselines, not proof of current deployment or live operation.

| Increment | Evidence and observed capability | Limit |
| --- | --- | --- |
| Standalone shell and interactions | Baseline `d4716fc`; [README](../README.md). Persistent composer, independent Ctrl-P palette, source-preserving selection/copy, responsive views and retained drafts. | Invented work data and historical replay remain labeled modes. Shell usability is not workflow observability. |
| Lifecycle observation | [U0/U1 and S5 validation](u0-preparation.md). Explicit passive lifecycle connection; earlier populated service/UI proof shows handoff and recovery-fence snapshots. | Snapshot reads do not supply complete event history, an agent transcript, or qualified usage. Earlier proof is not a fresh qualification of today's full stack. |
| Task fixture projections | `007e849`; [fixture record](task-fixtures.md). Overview, identifier-bound Graph and scoped Conversation explore pending/result/stale/refused states. | Fixtures establish interaction behavior, not an accepted wire contract or a real task. |
| Task observation v2 client | [TO2 host contract](../../../../docs/rust-retained-task-host-contract.md#to2-independent-task-observation), [client](../src/task_observation.rs). Five independent typed components, bounded decoding, identity checks and scoped stale anchors. | v2 only; no atomic snapshot, native revision, task-command edge, transcript or command authority. Native capacity refusal is explicit. |
| Connected task views | `d2e5c53`; [worker/projection](../src/task_live.rs), [UI model](../src/model.rs), [Rust regression tests](../src/task_live_tests.rs). Explicit task selection, background refresh, disconnect/reconnect, late-reply fencing and host/target anchor invalidation. | Controlled socket tests and local PTY evidence; the installed-host test was not rerun for this UI commit. This is a one-task observation path, not UI-1 campaign visibility. |

At `d2e5c53`, recorded validation was 10 library plus 58 binary Rust tests,
all-target Clippy with warnings denied, formatting, build, and three PTY
checks: existing shell, task fixtures, and a local synthetic v2 socket journey.
The [installed-host client test](../tests/task_observation_host.rs) remained
ignored in the default test run. It calls the production client but does not
render the terminal UI; passing it alone would not close the rendered-UI gap.
These are prior results, not tests rerun during this documentation reconciliation.

Settled interaction behavior carries forward: Overview leads task inspection;
Graph selection scopes inspection and double-click opens Conversation; neither
changes the draft recipient. Read-only connections disable sends and retain
drafts. Attributable task publications remain grouped rather than presented as
a fabricated chronological transcript. Context exposes technical provenance;
unsupported progress measurements remain unavailable.

## Qualification update — 2026-10-16

The [installed-host validation](installed-host-validation.md) now has a repeatable
Rust integration test, [`task_refresh_host.rs`](../tests/task_refresh_host.rs).
A real pinned S8 host and finite loopback simulator drive pending question →
authorized answer → published result; one running terminal UI updates its
Conversation screen without manual refresh. Owner readbacks also establish the
completed delivery/native segment, and the test checks terminal restoration and
child cleanup. This is test-owned mutation in disposable state; UI commands stay
read-only. The opt-in installed test passed, separately from the default suite.

This narrows the rendered-host gap; it does not close the complete qualification
matrix below. Multiple segments, restart/partial failure, capacity refusal,
draft/recipient preservation and receipt-age/segment presentation retain their
separate evidence requirements. One-off four-view checks are prior evidence,
not proof that this repeatable test covers every view or failure mode.

The [S5 source map](s5-source-map.md) identifies the broader design's actual
case: `skills-migration-app-server-20260828`, slice 6. It binds five candidates,
acceptance, all 14 linked revisions across three review episodes, five
hash-matched gate results, eight distinct provider result envelopes, the exact
quota retry item, the recorded metrics row and continuity repair. The replacement
authority explicitly names the original reported episode. Candidate 5's
constructed receipt lists five passed checks while frozen metrics says eight
stages for the same digest. The historical metrics producer copied the prior slice record and manually
entered eight with the new S5 digest; no separate eight-stage source appears in
that producer path. The published count still needs an owner-bound correction
or explanation. A new uncommitted, locally qualified
[historical read host](../../historical-evidence-read/README.md) admits the exact
checkpoint, gate-capture and review records through their legacy owners and
renders a first terminal investigation. Its current grant and captured proof
are external to the repository and await implementation review. Provider,
scheduler and metrics-owner reads, a broader public access policy and full
UI-0 acceptance remain open.

## Mapping to the accepted delivery slices

“Not established” below means the bounded UI evidence above does not establish
the slice's acceptance. It is not a repository-wide claim that every supporting
backend capability is absent.

| Accepted slice | Reusable work today | Remaining consequence and status |
| --- | --- | --- |
| UI-0 — Event and owner projection | Passive client patterns, component scope/revisions, safe failure states, exact-source inspector, and an uncommitted locally qualified checkpoint/gate/review S5 vertical. | Provider/scheduler/metrics owner reads, full versioned evidence classes and owner registry, broader confidentiality policy and the complete golden trace remain **not established**. |
| UI-1 — Operational control plane | Overview/Graph/Conversation shell and one-task read-only inspection. | Actual campaign/slice state, roles, candidates, gates, review obligations, limits and scheduler waits need public projections and joined UI proof. **Partially reusable presentation; slice acceptance not established.** |
| UI-2 — Causal timeline and drill-down | Identifier-bound task graph, expandable source details and offline replay presentation. | Exact candidate/episode/finding/remediation lineage, justified causal edges, historical cutoffs and traversable owner references need a qualified source graph. **Not established.** |
| UI-3 — Efficiency and anomaly explanation | Existing source/detail presentation can display qualified measurements. | Non-overlapping attempt attribution, comparable frozen baselines, missing-measurement handling and evidence-backed S5 explanation. **Not established.** |
| UI-4 — Counterfactuals and recommendations | Accepted design specifies the outcome and authority limits. | Versioned alternatives over a frozen trace, uncertainty, recommendation decisions and post-adoption evidence. **Not established; later work.** |

Task graphs show supplied bindings. They are not the accepted causal graph over
campaign → slice → candidate → reviewer episode → finding → remediation.
Repeated snapshots likewise do not establish a causal timeline or complete
history. The five evidence classes remain the accepted design's classes:
observed, mechanically derived, semantic interpretation, counterfactual
estimate, and recommendation.

## Source readiness and ownership

The accepted design's owner table remains authoritative for semantic placement.
The following is a bounded implementation starting map, not a new registry or
a claim that any listed library is already a public UI service.

| Needed meaning | Starting evidence / owner | Connection still to establish |
| --- | --- | --- |
| Retained-task publications and lifecycle/native observations | Accepted [task host contract](../../../../docs/rust-retained-task-host-contract.md#to2-independent-task-observation), consumed by the current UI. | Rendered installed-host proof and reconciliation of task provenance/receipt-age presentation with the original consumer proposal. |
| Campaign phase, accepted boundary, candidate and pending obligations | [`slice-campaign::Campaign::read`](../../slice-campaign/src/application.rs) and [`Snapshot`](../../slice-campaign/src/contract.rs) expose owner state. | A bounded read-only host/query surface, supported discovery scope and exact joins to candidate/review evidence. A library method is not an admitted public UI endpoint. |
| Review episode state and history | [`review-episode::Application`](../../review-episode/src/lib.rs) implements admitted, bounded reads/history at revisions. | Projection access, confidentiality, immutable subject/finding lineage and host wiring. UI code does not open the owner's database. |
| Checkpoints, gate results, acceptance and completion | Slice Checkpoint and supervisor receipts named by the accepted design. | Resolve exact references from an agreed run, distinguish independent owner revisions and expose refusal when a reference cannot be validated. |
| Provider cost/latency and scheduler obligations | Provider events/normalized metrics and Role Scheduler named by the accepted design. | Exact eligible sources, units, coverage, access and history for the selected run. No inference from rollout byte counts, session liveness or a due schedule. |
| Claims, physical profiles and role realization | Claim Evidence, Code Change Profile and Runtime Manifest named by the accepted design. | Select the required read contracts and preserve each owner's limits; do not copy payloads into UI-owned authoritative state. |

The October Rust owner APIs are candidate sources; the 2026-10-04 exploration's
Node inventory is historical evidence, not a mandate to bind new UI code to
those old paths. New first-party implementation and reusable test infrastructure
follow the [Rust direction](../../../../docs/rust-development.md). The UI stays
outside writable stores and provider execution. A host-side projection can use
owner readers; its placement and transport are resolved against the first
concrete consumer, without inventing a universal event platform first.

## Recommended work sequence

### Close task-view qualification

Demonstrate the committed task-view path against an exact compatible installed
host in disposable, provider-disabled state. This completes the missing proof
for the increment already built; it does not count as UI-0 or UI-1 completion.

The result should bind host/client source and binary identities, request scopes,
actual UI frames and source readbacks. The journey covers task absence and
known-empty collections, pending/answered questions, published result with
unsettled execution, multiple native segments, capacity refusal, partial failure,
restart, stale replies, disconnect/reconnect, and draft/recipient preservation.
Classify cases by evidence source: actual host/UI, controlled peer injection,
or unit test. A case unavailable through the installed host remains an explicit
gap, not a synthetic pass relabeled as host evidence.

Reconcile the original proposal's visible last-read age and delivery/segment
inspection expectations with actual frames. Preserve per-component source
scope and label local receipt time separately from event time. Any required
repair remains bounded to this consumer or returns to the owner of the failed
contract. No live provider call, host activation, command send or custody
migration is needed to establish this read-only consequence.

### UI-0: bind the evidence before expanding screens

First prepare a source map for one explicitly selected campaign/run and the
accepted design's **S5 review-amplification** case. Bind exact candidate, gate,
reviewer episode, finding/remediation, replacement, quota, schedule and repair
references, their owners, revisions, accessible evidence and missing coverage.
The [source map](s5-source-map.md) now binds the campaign identity, linked review
history, five hash-matched gate results, eight provider result envelopes, a quota
event and an acknowledged scheduler item. Candidate 5's gate count discrepancy
remains open. The first local read admits checkpoint/gate/review records only;
provider, metrics and scheduler reader paths remain open. A similarly named lifecycle S5 handoff or
metrics cohort is not the same case without an established identity join.

Complete the remaining source-map coverage before broader projection implementation. Its useful deliverable
is a reviewable producer-to-consumer path for each displayed fact and edge,
including the read/access boundary and a proposed vertical test. It determines
which owner capabilities can be reused and which public projections are missing.
The physical query index, transport and retention choices stay open until those
facts distinguish the alternatives. A missing required source is reported to
its owner; UI-0 is not declared complete using a substitute narrative.

Then implement the smallest read-only projection that can reconstruct that
bound trace while preserving the accepted event envelope, evidence classes,
integrity references, confidentiality, cutoff and completeness. Storage/indexing
is disposable derived state, not another workflow authority. Deterministic
checks cover incorrect joins, missing or corrupted references, owner revision
changes, incompatible generations and omitted/private evidence. This is the
UI-0 acceptance boundary, rather than merely defining an envelope schema.

### Prevent copied gate counts at the metrics boundary

The S5 evidence map now retains the exact receipt and deterministic
`gate_check_count = length(checks)` derivation, separate from historical
`stage_total`. Future gate/metrics owner work should expose an explicitly named,
versioned measurement with units, candidate scope and source receipt digest.
The count should be derived from the validated receipt; publication should
reject a supplied count that disagrees. Missing or incompatible evidence should
produce an unavailable measurement rather than a copied scalar. If stages are
a separate concept, their owner needs an explicit stage list and counting rule.

Acceptance includes a valid five-check receipt, a supplied eight-count mismatch,
a wrong candidate/digest, missing evidence, and nested test counts that do not
inflate the check count. Historical records retain their original values with
revision-bound corrections where warranted. This is proposed gate/metrics owner
work, not a validator implemented in the UI; production enforcement remains open.

### UI-1: make one actual campaign understandable

Connect one selected campaign's operational view: objective/boundary, phase,
current candidate, gate/review state, pending obligations and evidence-backed
waiting reasons. Add roles, limits and schedules where the owning read can
establish them; unavailable fields remain visibly unavailable. Scope/discovery
expands only with an owning query boundary.

The vertical journey is operational attention → selected work/candidate → exact
owner evidence → back to the same investigation. It preserves source revisions,
read freshness, selection, drafts and independent recipient. Reconnect and source
changes cannot silently retarget inspection. A first useful campaign view can
land incrementally, but the full UI-1 result still accounts for every required
operational dimension or records its unmet condition.

This does not wait for economics or live provider integration. Saved or
controlled owner records can qualify the projection with their evidence class
visible; that proof does not claim an operational production deployment.

### UI-2: traverse cause and evidence

Render the qualified UI-0 graph in timeline lanes and drill-down. Every causal
edge resolves to an owning transition, exact reference or attributed semantic
interpretation. Candidate changes, reviewer continuation/replacement and
remediation remain distinct. Missing history and evidence cutoffs are explicit.

Acceptance follows the S5 investigation from an operational anomaly through
replacement/finding/remediation to the responsible contract revision, without
manufacturing causes from timestamps. Existing task bindings remain available
as a different graph scope. UI-1 and UI-2 can evolve together once their needed
source joins are established; numbering does not require a new rendering stack.

### UI-3 and UI-4: explain cost, then evaluate alternatives

UI-3 follows when provider attempts can be attributed without overlap and the
baseline cohort is named and comparable. Display original units, coverage and
unknowns before adding versioned semantic explanations. The S5 acceptance case
separates useful findings from avoidable review amplification.

UI-4 follows qualified historical traces and explicit policy/assumption owners.
Estimates keep mechanically avoidable overhead separate from uncertain discovery
quality. Recommendations remain advisory; later human decisions and observed
results do not rewrite the original estimate. Neither stage blocks earlier
operational visibility or grants authority to change production routing.

## Dependencies, deferred scope and decisions

Task qualification and the UI-0 source map are independently useful next work.
The former closes a consumer proof gap; the latter prevents broader screen work
from assuming sources and joins that have not been established. UI-0's qualified
joins feed UI-1 and UI-2; those identities and histories support UI-3; frozen
traces and explicit alternative-policy semantics support UI-4. Exact placement
and slice boundaries can change with evidence while those consequences remain.

Answers, sends, interruption, creation and other controls are a separate
owner-admitted track. The [intent-projection mechanism](../../../../app-server/docs/architecture/mechanisms/authority-preserving-intent-projection.md)
is its design lineage. They need operation discovery, exact target/revision,
authority, bounded input and attributable outcomes; availability in a view is
not admission. They are neither implied by this roadmap nor prerequisites for
the read-only slices. Streaming agent text and qualified context usage likewise
need their own source contracts. Studio's design/organization views and remote
or multi-client hosting remain outside this first sequence.

Before starting the broader UI-0 implementation, resolve candidate 5's gate count discrepancy,
the owner-admitted historical read paths, initial campaign scope, public
projection placement and access policy. The operator/product owner retains scope
and priority choices; each
semantic owner retains contract/access decisions. This roadmap does not alter
concurrent lifecycle, migration, review-provider or campaign commitments.

Revisit the sequence when the golden corpus proves unavailable, a required owner
read changes, rendered qualification exposes a missing contract, or the operator
prioritizes interactive control over read-only supervision. Record the consequence
in this roadmap and link the evidence rather than silently changing what a
completed slice means.

## Evidence scope of this reconciliation

The accepted design, Studio/exploration lineage, host contract and UI documentation
were read as documents. Current task wiring and the candidate campaign/review
read boundaries were checked through Codebase Memory Tier 2 and targeted source.
Project `home-bline-code-work-engine`, recorded generation
`2026-10-16T02:25:33Z`, reported content-verified/no recorded issue for the nine
code/test paths relied on. This is best-effort coverage, not a repository-wide
absence audit or runtime proof. Broad owner discovery was narrowed to positive
reader examples; no exhaustive backend-completeness claim follows.

The completion/validation statements above are bound to their named prior
increments. The original planning pass changed documentation only and ran no runtime or
provider qualification. The later 2026-10-16 update above records the separately
authorized installed simulator test and bounded historical source discovery.
