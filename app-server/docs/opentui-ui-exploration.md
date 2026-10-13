> Promoted on 2026-10-07 to the maintained standalone app at
> [`rust/crates/terminal-ui`](../../rust/crates/terminal-ui/README.md).
> The exploration below records its design history; runtime integration remains pending.

# Standalone terminal UI exploration and initial design

## Status and scope

Exploratory assessment and proposed design, 2026-10-04 (America/Denver).
Requested by the operator: assess how much of the app-server migration can
support a UI, explore OpenTUI, then design the experience. This is not an
accepted architecture amendment or implementation plan. No provider was run,
runtime activated, or production UI implemented. The initial combined Rust-stack
evaluation used an isolated local harness outside the repository. A later,
operator-authorized interactive fixture prototype now lives in
[`rust/crates/terminal-ui`](../../rust/crates/terminal-ui/README.md), with its own
Cargo workspace and dependencies; engine/package dependencies are unchanged.

Expanded 2026-10-06 at the operator's suggestion to assess a standalone client
and Rust terminal UI libraries alongside OpenTUI. The existing filename is
retained for link continuity; OpenTUI is a candidate, not the selected stack.
The runnable prototype uses the Rust stack provisionally to evaluate the design,
not to settle the production implementation choice.

The intended product is provider-agnostic Work Engine. The directory name and
current Codex transport are implementation facts, not the UI's organizing model.
The operator selected work supervision as the leading experience: objectives,
progress, decisions, and intervention. Conversation and engine inspection support
the selected work; they do not determine the landing screen.

## Assessment

There is enough implemented substrate for a useful role interaction and runtime
inspection UI, and substantial durable domain state for a work supervision UI.
The inspected boundaries do not yet establish a complete provider-neutral UI
service. The architectural home already exists as `OperatorProjection`; the
remaining work is specifying its concrete client contract, joining owned state,
and presenting bounded actions and their outcomes. This is realization of an
existing boundary, not a proposal for another provider-neutral abstraction.
A migration-completion percentage would obscure those different readiness levels.

| UI need | Evidence in current implementation | Readiness consequence |
| --- | --- | --- |
| Discover roles, select an instance, inspect bindings | [OperatorSwitchboard](../src/operator-switchboard.mjs), commands `agents`, `attach`, `detach`, `status`, `threads` | Directly reusable behavior; selection does not itself start work. |
| Send a message and distinguish delivery from completion | `OperatorSwitchboard.startLine` returns delivery before its completion promise; completion exposes final output | Enough for a basic conversation. Streaming content needs an explicit normalized feed beyond this interface. |
| Explain queued input | Switchboard's closed-admission path returns queue ID, sequence, transition revision, and replay status | Can show “queued” without falsely showing “delivered”; subsequent queue disposition still needs a UI projection. |
| Show current runtime activity and token observations | [ObservableAppServerTransport](../src/observable-app-server-transport.mjs) has subscriptions and snapshots | Useful live diagnostics, not a durable workflow timeline. Default retention is 512 events; content is deliberately omitted. |
| Interrupt a running turn | [GenerationBoundAppServerTransport](../src/executable-generation-dispatch.mjs), `operatorControl`, exact target resolution and coalescing | Existing control lane is useful. Its acknowledgment is `interrupt_requested`, not proof the work stopped. |
| Disconnect the client independently | [AppServerProtocolProxy](../src/app-server-protocol-proxy.mjs) control lane | Supported independently of host shutdown. Current `/rpc` is a single-client Codex protocol relay, not a neutral Work Engine client API. |
| Inspect campaign state | [Slice campaign service](../src/services/slice-campaign/service.mjs) has revision-bound recovery and transitions; [SQLite store](../src/services/slice-campaign/sqlite-store.mjs) persists state | Domain foundation exists. Discovery across campaigns and an operator-oriented aggregate need a deliberate query boundary. |
| Inspect review history and exact evidence | [Review episode service](../src/services/review-episode/service.mjs) exposes revision reads/history; [claim evidence read service](../src/services/claim-evidence/read-service.mjs) exposes validated projection reads | Good drill-down sources. Their existence does not establish a joined UI timeline or universal permissions surface. |
| Retrieve retained output | [RetainedTurnOutputStore](../src/retained-turn-output-store.mjs) stores digest-addressed blobs and checks integrity | Useful artifact primitive; it does not by itself provide a complete searchable conversation history. |
| Explain authority, causal history, economics, and organization | Existing designs describe these outcomes | Treat as designed or partially supported, not as screens ready to connect. Each view needs source and completeness checks. |

The strongest immediate slice is therefore roles + message delivery + runtime
status + independent interruption. A work-centered landing view is also a
reasonable first product target, but requires more projection work. Neither
requires waiting for every future migration objective.

## Existing design lineage

[Provider Turn, Harness Runtime, and Operator Projection](../ideas/pending/provider-turn-harness-runtime-and-operator-projection.md)
explicitly separates three independently replaceable ports: `ProviderTurnPort`
for provider requests/results, `HarnessRuntimePort` for coding-agent sessions
and native tools/lifecycle, and `OperatorProjection` for human-facing state and
controls. Section 4 is the direct architectural home for this OpenTUI client.
It keeps logical role, workflow, context, and policy identity outside the UI
session. Replacing a client ordinarily leaves active execution unaffected;
an admitted operation that depends on that client for approval is an explicit
exception. The idea is partially incorporated into the architecture, not an
independent implementation authorization.

[Control Plane and Client Protocol](../../ideas/control-plane-and-client-protocol.md)
proposed revision-aware reconciliation, control discovery, client capability
advertisement, bounded intent, approval/input requests, and pending-obligation
visibility. Its control packets bind action, subject, revision, authority
context, and expected consequence. The accepted
[control-plane reconciliation](control-plane-and-client-protocol-reconciliation.md)
retired the client-protocol portion into `OperatorProjection`; the original
idea is historical lineage, not a competing protocol owner.

[Authority-Preserving Intent Projection](architecture/mechanisms/authority-preserving-intent-projection.md)
names the shared interaction mechanism: discover and render operations, collect
bounded intent, submit it to the owning domain, and display attributable
lifecycle feedback. The domain owns authorization, admission, and the meaning
of its outcomes. The architecture page records accepted design lineage and
authorized design work, with the concrete connective-layer schema still to be
specified. The switchboard is a partial precursor, not proof that this complete
mechanism is implemented.

[Control-plane causal observability](control-plane-causal-observability-ui.md)
already owns the runtime and execution-history design. Its five evidence classes
are observed, mechanically derived, semantic interpretation, counterfactual,
and recommendation. Its operational view can precede the economics and
counterfactual features.

[Studio reconciliation](work-engine-studio-reconciliation.md) places the broader
product around that design. Its accepted remaining design work is the interactive
connection between owned state and operator intent: discovering actions,
rendering inputs, submitting bounded requests, and showing authoritative
outcomes. Organization and design-to-runtime history also depend on their own
canonical sources. This proposal specializes presentation and interaction; it
does not create new owners for those concepts.

The reconciliation's historical missing-cross-reference finding is already
partly addressed: the current causal-observability document explicitly links
Studio and the reconciliation. Historical findings should not be copied as
current implementation gaps without checking their subjects.

## Standalone application boundary

The proposed deployment is a separately runnable operator client attached to a
Work Engine host through the provider-neutral `OperatorProjection` contract.
It can live in the same repository while having its own package, executable,
dependencies, and release lifecycle. A separate repository is not implied.

Closing or restarting the client ordinarily leaves engine work running. The
host retains workflow state and action admission; the client retains navigation,
drafts, and disposable view caches. Explicit client-dependent interactions such
as approvals still need disconnect behavior defined by their owner. A launcher
may offer to start a host, but host ownership and shutdown remain explicit.

This boundary makes Rust and TypeScript equally plausible client languages.
The wire contract needs version/capability negotiation, snapshots and update
reconciliation, bounded requests, and attributable outcomes. Client libraries
can use shared language-neutral schemas and fixtures; importing engine internals
or opening its SQLite stores directly would undermine the intended separation.
Local IPC is a reasonable first transport scope. Remote connectivity, host
discovery, authentication, and multiple simultaneous clients remain separate
design questions; the current single-client provider relay does not settle them.

## OpenTUI fit

OpenTUI provides a native Zig rendering core with TypeScript APIs and Core,
React, and Solid composition options. Its component set covers panes, selection,
multiline input, scrolling, Markdown, code, and diffs. These are a good fit for
role conversations, an operational list, and evidence inspection. A causal
timeline or relationship navigator is still application design work.
Sources: [introduction](https://opentui.com/docs/),
[components](https://opentui.com/docs/components/).

Markdown supports incremental updates; that makes streamed output practical
once the engine supplies an appropriate content feed. The renderer cannot turn
metadata-only transport observations into a transcript.
Source: [Markdown](https://opentui.com/docs/components/markdown/).

Current documentation specifies Bun 1.3+ or Node 26.4+ with experimental FFI.
This checkout declares Node >=22 and this session has Node 22.23.2, with no Bun
on PATH. OpenTUI is therefore not a drop-in dependency for the current runtime.
A separately packaged TUI process using Bun is a plausible starting choice;
it avoids making the engine's runtime migration a prerequisite for UI work.
Exact release, OS, native dependencies, and packaging remain spike decisions.
Source: [runtime support](https://opentui.com/docs/getting-started/runtime-support/).

Within the OpenTUI option, Core is sufficient for a small renderer experiment.
Solid is a plausible composition choice; React is also viable. There is no measured
performance basis here for choosing between them. Decide after a representative
prototype with long output, focus changes, resizing, and bursty updates.

OpenTUI's in-memory test renderer supports frame capture and simulated input.
That can test focus, navigation, stale-state feedback, and layout without
starting model work. Real terminal checks remain necessary for terminal cleanup,
copy/paste, keyboard compatibility, Unicode, and SSH behavior.
Source: [testing](https://opentui.com/docs/core-concepts/testing/).

## Rust alternatives and comparison

Official project documentation reviewed 2026-10-06 supports this shortlist:

- **Ratatui with Crossterm:** the leading Rust candidate for a custom dashboard,
  history browser, and runtime conversation. Ratatui supplies rendering and
  widgets; application state, event routing, focus, and navigation need an
  application architecture. Its documentation discusses Elm, component, and
  Flux patterns, and its backends include an in-memory `TestBackend`.
  Sources: [Ratatui](https://github.com/ratatui/ratatui),
  [application patterns](https://ratatui.rs/concepts/application-patterns/),
  [backends](https://ratatui.rs/concepts/backends/).
- **tui-realm:** a framework over Ratatui that adds reusable stateful components,
  messages/events, mounting, focus, and event forwarding. Worth evaluating if
  its component model reduces our multi-view interaction work without making
  shared input and nested drill-down awkward.
  Source: [tui-realm](https://github.com/veeso/tui-realm).
- **Cursive:** another Rust option with composed views, dialogs, callbacks, and
  an application event loop. My assessment is that it deserves a secondary look
  if forms and dialogs dominate; Ratatui is the more direct first experiment for
  this custom operational workspace. This is a fit judgment, not a measured
  capability or performance ranking.
  Source: [Cursive](https://github.com/gyscos/cursive).

The Rust ecosystem includes `ratatui-textarea`, with multiline editing,
undo/redo, selection, and optional search. A composer therefore need not start
from raw key handling. Compatibility among Ratatui, backend, and widget versions
still needs checking in an actual prototype.
Source: [ratatui-textarea](https://github.com/ratatui/ratatui-textarea).

| Decision factor | OpenTUI candidate | Rust / Ratatui candidate |
| --- | --- | --- |
| Composition | TypeScript with Core, React, or Solid | Explicit application state/update/render model, optionally tui-realm |
| Conversation content | Documented Markdown streaming, code, and diff components | Rich text, Markdown, and diff integration need a concrete widget/dependency assessment |
| Custom operational views | Flexible component layout; application-specific timelines and relationships | Flexible widget rendering; application-specific timelines and relationships |
| Shared composer and focus | Built-in textarea and framework interaction APIs; full UX still ours | Textarea widget available; focus/event orchestration belongs to the app or an added framework |
| Distribution | JS runtime/native renderer assets, possibly bundled into an executable | Target-specific compiled executable; packaging and system dependencies still require target checks |
| Maintenance | Fits the existing JS/TS toolchain, but client/host isolation still matters | Adds a Rust toolchain; typed protocol models and shared fixtures can limit cross-language drift |
| Testing | In-memory renderer and simulated input | TestBackend plus state/update tests and suitable input fixtures |

Both can implement the proposed standalone architecture. Native Rust packaging
is attractive for a long-lived operator tool; OpenTUI's integrated rich-content
components are attractive for the unified conversation. Neither observation
establishes a winner, and no relative memory, latency, or development-speed
claims have been measured.

The proposed comparison uses the same small fixture and interaction design in
OpenTUI and Ratatui: dashboard → historical branch detail → streaming process
conversation, with a persistent multiline draft, detail toggles, resizing, and
disconnect/reconciliation. Assess implementation burden, input correctness,
scroll stability, Unicode/terminal behavior, frame latency, memory under bounded
history load, and distribution on selected target platforms. A runnable parity
prototype would supply selection evidence. The bounded headless Rust probe below
is an initial compatibility/interaction check, not that full parity prototype.

### Combined Ratatui + Crossterm + tui-realm evaluation

At the operator's request, evaluated the three as one stack on 2026-10-06.
Their responsibilities compose cleanly: Crossterm supplies terminal input/output,
Ratatui renders widgets and layouts, and tui-realm organizes components, focus,
subscriptions, and application messages. tui-realm re-exports Ratatui, allowing
the client to use the version selected by the framework instead of introducing
an incompatible independent dependency.
Sources: [tui-realm API](https://docs.rs/tuirealm/4.1.0/tuirealm/),
[package manifest](https://github.com/veeso/tui-realm/blob/main/crates/tuirealm/Cargo.toml).

**Executed evidence:** built an isolated fixture harness with Rust 1.92.0 on
Linux. Its lockfile resolved tui-realm 4.1.0, Ratatui 0.30.2, and Crossterm 0.29.0.
The harness lives at
`/home/bline/.local/state/work-engine/ui-stack-evaluation-20261006/`, with source,
Cargo.lock, README, `test-results.txt`, `dependencies.txt`, and `frame.txt`.
Reproduce with `cargo test --locked` or render the fixture with
`cargo run --locked --quiet` from that directory. It is local evaluation evidence,
not a portable repository artifact or production implementation.

Five checks passed:

1. Multiline draft state survived fixture view changes, detail toggling, and
   rendering at 80x24, 120x40, and 160x50.
2. One hundred sequential acknowledged activity updates reached a subscribed
   content component while input focus remained on the composer.
3. A focused component also subscribed to an event received it once, avoiding
   duplicate processing through the two paths.
4. Crossterm key and paste event values converted and reached the composer.
5. A characterization test confirmed that batched dispatch uses the focus in
   effect before application-level update messages are applied.

The fifth finding matters: Tab followed by typing can reach the previous field
if both events are dispatched before the application applies the focus change.
The proposed integration processes each input event and its resulting updates
before routing the next. This is an event-loop design concern, not evidence that
tui-realm cannot support correct focus behavior.

Source inspection of the installed 4.1.0 package also found an unbounded standard
MPSC channel in `src/listener/mod.rs` and dispatch-before-update behavior in
`src/core/application.rs`. For high-volume engine updates, the adapter therefore
needs bounded buffering/coalescing and explicit gap/resnapshot behavior rather
than assuming the framework supplies backpressure. Lossless operator commands
and reconstructible display updates need different treatment. No overload or
memory-growth benchmark was run.

**Assessment:** retain the combined stack as a leading candidate for the next
interactive design prototype. tui-realm supplied useful focus and subscription
behavior in the probe; its cost is component/attribute/message plumbing and
careful integration of event ordering. Keep it confined to presentation: typed
Work Engine client models, revision reconciliation, and request results should
remain independent of framework component state. Its event subscriptions do not
replace durable engine history or the `OperatorProjection` protocol.

**Limits and next decision evidence:** the probe changes view labels over mounted
fixture components; it does not implement full branch/history screens. Its
composer only appends text, so it does not evaluate a production textarea's
cursor, selection, or undo behavior. Rendering is in memory; real keyboard/IME,
paste negotiation, mouse, SSH, terminal restoration, and platform compatibility
are untested. Sequential updates are not a throughput benchmark. Markdown/diff
rendering, long-history scrolling, async network ports, and reconnect need the
next interactive prototype. The favorable result supports continuing evaluation,
not selecting the stack or claiming superiority to OpenTUI.

## Multi-view experience design

The first screen should answer: what am I trying to accomplish, what is
happening, what needs my attention, and what can I do next? Provider details
remain inspectable attributes of a role's runtime binding.

The operator's subsequent direction (2026-10-05) expands this into a multi-view
workspace: a possible dashboard entry point, several upper-level perspectives,
historical actions and branches, and drill-down to a runtime chat-style view
with unified input and configurable detail. The views below develop that
direction as design candidates, not a fixed navigation taxonomy or a claim
that all supporting projections exist.

| Candidate view | Principal question |
| --- | --- |
| Dashboard | What is progressing, waiting, completed, or asking for my attention across the selected scope? |
| Objectives and work | How does this objective relate to its constituent work, decisions, dependencies, and outcomes? |
| History | What happened, what caused it, and what was known at that point? |
| Branches and comparisons | Where did work diverge, what changed between alternatives or revisions, and what was accepted or left unresolved? |
| Roles and runtime | Who is doing what, with which realization, and what can I observe or communicate to them? |
| Process conversation | How can I follow and participate in the selected process through one attributed conversation/activity stream? |

An evidence inspector is shared across these views. It opens artifacts,
findings, contracts, observations, and exact references without making the user
leave the subject they were investigating. Multiple upper-level views can
coexist: a portfolio overview, dependency map, attention queue, and outcome
comparison answer different questions over the same owned state.

### Connected views and historical scope

Scope, time, and detail are independent view choices. For example, an operator
can inspect one completed run in summary form, compare two candidate revisions,
or follow all roles in a live run at activity-level detail. Moving between views
preserves the selected subject and historical cutoff where meaningful; a
breadcrumb and back navigation preserve the investigation path.

“Branches” needs further product definition. Git branches/worktrees, candidate
revision lineage, alternative plans, and branching execution paths are distinct
relationships. A branch view can connect them through explicit owner references,
but should label the relationship rather than treat them as one tree. The first
design experiment can compare Git/candidate history with work/decision lineage
before choosing which belongs in the initial product.

Historical inspection shows the available state at a named cutoff and makes
missing history visible. It does not imply that the engine can reconstruct any
arbitrary past moment. Viewing old work is separate from rerunning or resuming
it; those actions require current owner admission and an explicit target.

### Unified process conversation and input

The process conversation combines operator messages, role responses, decisions,
and expandable activity entries for the selected scope. Every entry retains its
speaker or semantic owner, role/run identity, timestamp, and source reference.
Parallel activities can be grouped by work item or role instead of implying
that visual adjacency establishes a causal sequence.

One composer is available across views. It retains a visible recipient or
process scope, draft, and any attached evidence references. From an overview,
the operator can address the responsible coordinating role, where one is
defined; from runtime detail, they can address a specific role. Navigation does
not silently change the recipient of a draft. Unified input does not itself
grant broadcast authority or invent a universal coordinating agent.

Natural-language input remains useful at every level. If interpreting a
process-level request requires model judgment, that belongs to an authorized
role rather than a UI routing heuristic. Structured controls remain available
for exact actions, with their subject and consequence visible. The interface
shows whether input is drafted, submitted, queued, delivered, or unresolved.

### Configurable detail

Proposed presets are starting points for a prototype:

- **Summary:** objectives, outcomes, decisions, waits, and material changes.
- **Activity:** role messages, handoffs, tools, review activity, and transitions
  for which a suitable source exists.
- **Evidence:** exact revisions, receipts, bindings, telemetry, and limitations.

Detail can be changed for a view or an individual entry, with filters for role,
event kind, and time. Expanding a summary preserves its subject and source
cutoff. Mechanically grouped entries and model-authored summaries remain
distinguishable; a model summary is attributed interpretation, not owner state.
Changing presentation detail neither changes retention nor acquires access to
otherwise unavailable content. Pending operator decisions, stale data, and
unknown action outcomes remain visible even at the lowest detail setting.

Attention-first ordering is a dashboard preference, not a workflow priority
decision. A provider change remains a runtime-binding change, not a new logical
role. Organization and authoring views can extend this workspace as their
canonical sources and admission paths become available.

Illustrative layout; all rows below are fictional fixture data:

```text
Work Engine / example-repository                 Connected · current at 14:32
Dashboard   Work   History   Branches   Runtime                 / Search
──────────────────────┬────────────────────────────────────────────────────
Needs attention       │ UI migration                 Waiting for decision
> UI migration        │ Objective: independent operator interface
  Review cleanup      │ Next: choose initial interaction scope
                      │
Active work           │ Activity   Conversation   Evidence
  Claim refresh       │ 14:29  Candidate published       [Observed]
                      │ 14:31  Review available          [Observed]
                      │ 14:32  Operator choice requested [Observed]
                      │
                      │ Decision details / exact subject / available actions
──────────────────────┴────────────────────────────────────────────────────
Message to planner:main                                            [Compose]
Tab Focus · Enter Open · Esc Back · Ctrl+P Commands · ? Help
```

This sketch shows one dashboard/detail arrangement, not the layout of every
view. Use view-specific main panes with a shared scope/time/detail bar, composer,
and optional evidence inspector at wide widths. At 80 columns, use
list → detail → inspector navigation rather
than squeezing three columns. At 120+ columns, show list and detail together.
These are prototype starting points, to be tested at 80×24, 120×40, and 160×50.

Quiet separators, generous line spacing where possible, and a small status
palette keep the hierarchy legible. Status always has a text label; color is
supplementary. Exact revisions and verbose provenance live in the inspector,
while freshness, uncertainty, and consequential action scope stay visible.

### Interaction details

Selecting a work item changes the view. Selecting a role does not silently
retarget an unfinished message: the composer shows and retains its recipient.
Opening the composer focuses a multiline editor; Enter inserts a newline and
an explicit Send command submits. Drafts survive navigation and reconnection;
their persistence policy is a product choice still to settle.

Messages progress through locally drafted, submitting, queued or delivered,
and completed/failed states according to owner responses. A timeout after
submission is “outcome unknown,” not automatic failure or automatic retry.
An interrupted provider turn does not imply a cancelled campaign.

Actions are presented with their subject and effect. “Interrupt turn,”
“Disconnect UI,” and a future “Pause campaign” are different actions. The first
two have inspected implementations; the third should appear only if a canonical
owner supplies it. No generic Stop button should conceal these distinctions.

Disconnected views retain their last snapshot and label its age. Reconnection
refreshes owned state before re-enabling consequential controls. A stale
revision refusal preserves the operator's input and shows what changed. Missing
history or telemetry is explicitly unavailable, never an empty-success state.

The evidence inspector displays source owner, subject, revision, evidence class,
cutoff/completeness, and limitations. An interpretation can sit beside an
observation with clear attribution; it does not replace the observation.

## Standalone client realization of OperatorProjection

```text
Terminal client (OpenTUI or Rust) — an OperatorProjection realization
    ↕ Work Engine queries, subscriptions, bounded operator requests
Work Engine-owned projection endpoints and intent admission
    ↕ existing owner reads and admitted transitions
Workflow / evidence / role / runtime owners
    ↕ ProviderTurnPort / HarnessRuntimePort realizations
Codex and other provider/harness implementations
```

This diagram is a proposed realization of the existing ports and intent-projection
mechanism, not a new semantic layer. The concrete transport and placement of
projection endpoints remain design decisions. The chosen TUI stack renders the operator
experience; Work Engine retains the client contract and domain authority.

The UI consumes Work Engine identities and capability descriptions. Codex
thread/turn identifiers remain adapter bindings exposed in runtime inspection,
not the top-level information architecture. The engine owns authorization,
revision checks, idempotency, and domain results; the UI owns view state, focus,
draft interaction, and presentation.

The connective design specializes the existing control-packet model and
intent-projection lifecycle into three response shapes, with exact wire schemas
deferred until the first journeys are agreed:

| Shape | Information needed |
| --- | --- |
| View snapshot and updates | View/version, subjects, owner references and revisions, evidence cutoff, completeness, limitations, and a reconnect cursor or explicit resnapshot requirement. |
| Available operation | Owner-defined operation identity, target/revision, bounded input description, expected consequence, availability/refusal reason, and any existing authority requirement. Submitted intent binds action, subject, revision, authority context, and expected consequence. Display availability is advisory until the owner revalidates submission. |
| Request outcome | Stable request identity, owner acknowledgment, queued/admitted/refused state where applicable, eventual result reference, and explicit unknown outcome when acknowledgment is lost. No generic UI state machine replaces domain states. |

A snapshot-plus-update design needs a race-free handoff and explicit gap
handling. A new session or expired cursor can require a fresh snapshot. The
bounded transport event ring alone cannot promise restart-safe replay. For a
first read-only view, snapshot refresh is acceptable if freshness is visible;
an event system is not a prerequisite for every useful screen.

Keep interruption reachable independently of normal message delivery, as the
current control lane demonstrates. Avoid exposing the raw provider relay as
the permanent UI contract. Keep raw transcript access separate from content-free
diagnostics and carry source confidentiality limits through the projection.

## Design work and feasibility experiments

The design should compare three real journeys using fixtures:
understand a waiting run and its next decision; converse with a role while
work continues; investigate a finding through its exact candidate and evidence.
Extend those journeys across views: dashboard → work detail → process
conversation → runtime/evidence detail, and historical branch comparison →
decision lineage. Check that changing view or detail preserves subject, time,
draft recipient, and the ability to return to the starting overview.
Include disconnected, stale, queued, ambiguous-target, and unknown-outcome
states, not only successful activity.

Use a navigable terminal prototype for those journeys before wiring live
mutations. Its purpose is to decide information hierarchy, keyboard behavior,
attention management, readable density, and how the process conversation and
shared composer work alongside each upper-level view. Fixture data must remain visibly
identified. The first interactive experiment below now exercises the waiting
objective journey, parallel work, role conversation, evidence inspection, and
historical branch browsing. Disconnected, stale, queued, and unknown-outcome
interaction designs still need explicit fixture coverage.

Technical experiments can then answer bounded questions: streaming/scrolling
under load without stealing focus; resizing without losing selection or drafts;
native renderer packaging and cleanup; reconnect without duplicate requests;
and substitution of a second provider fixture without changing UI components.
An actual second provider integration is a separate backend readiness question.

The design is ready for implementation planning when the operator can review
the journeys and failure states, every visible field/action maps to an owner or
an explicit backend gap, and the first delivery boundary is small enough to
validate. Full Studio authoring, organization, cost simulation, and recommendation
features need not be part of that first delivery.

### Interactive supervision prototype — 2026-10-06

At the operator's request, built a standalone fixture app in
[`rust/crates/terminal-ui`](../../rust/crates/terminal-ui/README.md).
Run from the repository root:

```sh
cargo run --locked --manifest-path rust/crates/terminal-ui/Cargo.toml
```

The binary combines Ratatui 0.30.2, Crossterm 0.29.0, tui-realm 4.1.0,
rataflow 0.1.0, and ratatui-textarea 0.9.3. It has no engine/provider client or
persistence. Every screen labels the fixture context; simulated send only
appends a local timeline entry and does not acknowledge delivery, accept a
decision, or complete work. Normal exit prompts before discarding drafts.

Implemented views and interactions:

- **Dashboard:** a five-node work graph and equivalent list. Follow a waiting
  objective to its review dependency and unresolved decision; inspect the
  parallel build and evidence. Pan, zoom, and fit the graph with the keyboard.
- **Selection context:** a side panel with Overview, Evidence, and History.
  Status, wait, responsible role, revision references, and limitations follow
  selection. Below 110 columns, focusing context brings it into the main area.
- **History:** inspect main r18 and a superseded approach-a r11 fixture with
  different status/evidence for the same subject. This is a sample comparison,
  not Git branch discovery or a durable replay implementation.
- **Conversation:** a shared process timeline with role attribution and
  Summary/Activity/Evidence detail. Opening a selected role highlights its
  entries without automatically changing the composer recipient.
- **Unified composer:** one visible editor with explicit recipient, multiline
  input, cursor/selection, undo/redo, and separate session drafts per role.
  Retargeting is explicit. Historical snapshots disable simulated send and
  retargeting; returning to live preserves the draft and recipient.

Saved text previews: [dashboard at 128×40](../../rust/crates/terminal-ui/previews/dashboard-128x40.txt),
[compact dashboard at 80×24](../../rust/crates/terminal-ui/previews/dashboard-80x24.txt),
[historical branch](../../rust/crates/terminal-ui/previews/history-128x40.txt),
and [conversation with context](../../rust/crates/terminal-ui/previews/conversation-128x40.txt).
The interactive renderer adds color and visible focus. In this layout the graph
is useful at 128×40; shorter/narrower areas use the work list. More space helps
the relationship view, while the context panel carries the detailed explanation.

**Findings:** the combined Rust stack supports this interaction shape. rataflow
provides real rendering and viewport operations, but this experiment uses
authored positions and application-owned linear/dependency selection. It does
not establish automatic-layout quality, mouse usability, large-graph behavior,
or cross-platform compatibility. tui-realm supplies component/focus dispatch;
whether its structure pays for itself as the app grows remains open. Dispatching
one input event and applying its resulting focus change before the next event
is necessary for correct routing. Wrapped-text scroll bounds currently depend
on Ratatui's explicitly enabled unstable rendered-line-info API.

**Validation:** eight Rust tests pass, covering the supervision journey,
history refusal, target/draft preservation, selection/recipient separation,
progressive detail, framework event ordering, Unicode editing, and resized
layouts. Formatting and Clippy pass. A Unix PTY smoke test exercised startup,
journey keys, bracketed paste, resize, draft-exit cancellation and confirmation,
and terminal mode restoration. Text frames were inspected at 128×40 and 80×24;
automated resize checks include 60×18, 40×12, and 160×50. Execution used Linux and
Rust 1.92.0; the package declares Rust 1.88+. These checks establish prototype
behavior, not live migration readiness or renderer performance under load.

The next useful step is an operator walkthrough of this artifact. Resolve the
landing-view hierarchy and whether conversation should filter or highlight the
selected work, then define a small read-only OperatorProjection connection.
The fixtures suggest the following required information without prescribing
its wire schema:

| Prototype need | Production boundary still needed |
| --- | --- |
| Work graph and context | Discoverable subjects, typed relationships, attributable owner state and revisions; no inferred dependency from activity order. |
| History and branches | Owned branch/revision identities, evidence cutoff and completeness, explicit relation to current state; not a UI-created history ledger. |
| Shared conversation | Attributable entries with stable subject/role identities, content access limits, ordering and gap handling. |
| Explicit recipient | Discoverable live target and admission state; selection alone is insufficient to bind delivery. |
| Available intervention | Owner-defined operation/target/revision and authority context, current availability/refusal, admission and eventual outcome. |
| Freshness and reconnect | Visible stale/disconnected/partial state and a race-safe snapshot/update handoff or explicit resnapshot. |

The prototype's `Work`, `Entry`, and UI enums are invented local fixtures, not
new domain contracts. Approval, interruption, reconnect, persistent drafts,
streaming pressure, full evidence navigation, and provider substitution remain
unimplemented here. Distinct adapter labels in historical/live fixtures show
that the presentation has no provider-specific component, not that a second
provider has been integrated. No production architecture decision is implied.

### Visual treatment and slash-command follow-up

The operator found the responsive collapse and persistent composer useful and
requested a less boxed-in visual treatment plus slash autocomplete. The fixture
prototype now uses charcoal surfaces, plum context, a teal composer, and amber
attention highlights. Most panel outlines are removed. Brighter titles/text and
a small title marker indicate keyboard focus; tinted selection remains separate.
Graph node outlines remain because they help explain the relationships.

A command picker opens with `/` in an empty composer or outside it; Ctrl-P opens
it alongside an existing draft. Its query is separate from message text. It
supports ranked prefix/keyword/abbreviation matching, category tags, argument
suggestions for detail, scope and recipient, descriptions, and visible reasons
for unavailable commands. Tab completes; Enter executes; Esc cancels. Commands
are typed local UI actions only, and F8 cannot send while the picker is open.
Historical recipient changes remain unavailable. Existing draft text and every
role's saved draft survive command navigation. Literal slashes in existing text
and pasted slash-prefixed messages remain text.

Color previews: [dashboard](../../rust/crates/terminal-ui/previews/dashboard-128x40.png)
and [command picker](../../rust/crates/terminal-ui/previews/commands-128x40.png).
Colors and cell positions come from the actual Ratatui buffer; font rendering is
approximate. Narrow layouts keep the composer visible and stack the command
preview when space permits. The current palette targets dark RGB terminals;
alternate themes and accessibility evaluation remain open.

The follow-up passes twelve Rust tests, formatting, Clippy, and the updated PTY
smoke check including command opening/completion/cancellation. This adds no live
operation catalog or production authority. Richer typed argument pickers,
subject-specific consequence previews, recent commands, and unified work/evidence
search remain promising design directions.

### Chat input convention follow-up

The operator requested standard chat input: Enter sends and Shift-Enter adds a
newline. The prototype now applies that convention in the composer, preserving
historical-send refusal and the command picker's separate Enter behavior. F8
remains a send alias. Bracketed paste is inserted literally with normalized line
endings, does not execute slash commands or submit trailing newlines, and is
undoable. Paste feedback reports the inserted line count.

Unix startup requests enhanced keyboard reporting and pops that mode during
cleanup. Shift-Enter depends on distinct modifier reporting by the terminal;
Alt-Enter is available as a newline fallback. This uses
[Crossterm's keyboard enhancement support](https://docs.rs/crossterm/0.29.0/crossterm/event/struct.PushKeyboardEnhancementFlags.html).
Terminals without bracketed-paste reporting cannot supply the same literal-paste
guarantee. Fourteen Rust tests pass; the updated PTY check exercises enhanced
Shift-Enter, legacy Alt-Enter, Enter send, paste, and terminal-mode restoration.
Sending remains simulated and does not contact an engine.

### Folded pastes and growing composer follow-up

The operator requested compact large-paste handling and a composer that grows
with the draft. The prototype now soft-wraps ordinary text, grows from five rows
up to roughly one third of the terminal (maximum twelve rows), and scrolls
internally beyond that cap. Resizing and deletion recompute the height; the
surrounding work/context view remains navigable.

Pastes of six or more lines or at least 1,000 Unicode characters become labeled
objects in the draft. Full normalized text remains owned by the role's draft;
labels are only display projections, not replacement message content. Sending
expands the original payloads in their original positions. Cursor/selection,
deletion and undo/redo treat each block atomically. Literal lookalike labels or
private Unicode input cannot alias another block. Ctrl-O provides read-only
full-draft inspection. Editing inside a folded block and folding sent timeline
entries are not implemented in this experiment.

Previews: [growing composer](../../rust/crates/terminal-ui/previews/compose-80x24.png)
and [folded paste](../../rust/crates/terminal-ui/previews/paste-80x24.png).
Twenty Rust tests pass, including folded content retention and editor behavior;
formatting, Clippy, and the extended PTY folding/inspection smoke check pass.
This remains session-local fixture UI behavior, not model-context compression
or a live attachment protocol.

### Mouse interaction and source-Markdown selection follow-up

The operator accepted paste folding and composer growth and requested mouse
selection with clean Markdown copy. The prototype now captures mouse input:
visible panels and view tabs accept focus/navigation clicks, work lists accept
subject selection, and the wheel scrolls text/editor panes or changes work
selection. Existing keyboard navigation remains available; no click retargets
a message or performs a domain operation.

Conversation rendering now maps display graphemes to original Markdown byte
ranges. Drag selection can cross soft wraps and messages; copy omits role labels,
subject IDs, presentation padding, and simulated-send metadata. Original message
line breaks and syntax remain, with two newlines between messages. Message
recipient metadata is separate from authored text. Unicode graphemes remain
whole, selection survives resize/scroll, and source changes invalidate offsets.
This is literal Markdown display, not a formatted Markdown renderer yet.

Ctrl-C requests copy for an active selection in the main pane; Esc clears it.
Clipboard transport is terminal OSC 52, with no acknowledgment of clipboard
acceptance, so status explicitly says “copy requested.” Terminal support and
permissions determine whether it succeeds. Native clipboard fallbacks, double-click selection, and graph/editor mouse positioning are
future refinements. Modals block mouse click-through; no clipboard read occurs.

Twenty-four Rust tests, formatting, Clippy, and the real-PTY mouse/clipboard
smoke check pass. Tests capture and decode clipboard requests in an isolated
PTY; they do not overwrite the host clipboard or certify emulator support.
[Selection preview](../../rust/crates/terminal-ui/previews/selection-128x40.png).

### Edge-drag fix and syntax-highlighting direction

The operator found that selection stopped at the visible history viewport.
The fix tracks the held pointer and advances selection on an independent 80 ms
UI timer. At the top/bottom content row it scrolls one row per tick; dragging
farther beyond the edge accelerates up to five rows. It bounds scrolling to
available history and keeps the source anchor stable. Release, keyboard input,
resize, view/focus changes, or an overlay stop the gesture; moving back inside
or horizontally outside the content stops auto-scrolling. Stationary-pointer
regressions cover both directions, reaching full source, and cancellation.
Twenty-five Rust tests and the real PTY check pass; the PTY copies text that
was initially above the viewport after holding a stationary pointer at its edge.

The next requested investigation is syntax highlighting and detection. Recommended
first implementation: preserve the current source and selection model, add
Markdown and code styles as a separate cached display layer, then assess richer
rendered Markdown. This was the researched direction; the first implementation is recorded below.

- [pulldown-cmark 0.13.4 OffsetIter](https://docs.rs/pulldown-cmark/0.13.4/pulldown_cmark/struct.OffsetIter.html)
  emits Markdown events with source ranges. Use it to identify headings, emphasis,
  links, inline code, and fenced blocks. Event ranges are a starting point, not a
  complete rendered-character map: escaped characters, entities, hidden delimiters,
  and link labels still need explicit selection mapping and tests.
- [syntect 5.3.0 SyntaxSet](https://docs.rs/syntect/5.3.0/syntect/parsing/struct.SyntaxSet.html)
  supports syntax lookup by token/extension and first-line patterns. Prefer a
  fenced language tag; next use actual filename metadata if present, then a
  recognized shebang/modeline. An explicit unknown language should remain plain
  text. First-line matching is not a general-purpose arbitrary-code classifier.
- Start with source-preserving color: Markdown structure plus fenced code tokens,
  using colors compatible with the existing charcoal/plum/teal palette. Selected
  text must remain legible over every syntax color. Copy must still return original
  Markdown, never ANSI escapes, display labels, or newly inserted line breaks.
- Verify the actual grammar inventory for Rust, JavaScript/TypeScript, JSON,
  shell, and diffs before promising support. Cache parse/highlight output per
  message revision and syntax choice rather than parsing on every frame. Incomplete
  streaming fences, multiline strings, unknown languages, and Unicode are required
  cases; parser failure should fall back to readable source.

This interpretation treats “detection” as recognizing Markdown/code structure and
choosing a language for highlighting. It does not infer executability, execute
code, open detected links, or claim semantic understanding of pasted content.
The user has not yet specified richer detection or automatic attachment typing.

### First syntax-color implementation and selection redraw work

The operator authorized the color pass and reported selection lag. The conversation
now uses pulldown-cmark 0.13.4 source ranges and syntect 5.3.0 with bundled grammars,
a custom palette, and the pure-Rust regex backend. Markdown structure, inline code,
and fenced code receive styles without hiding or rewriting source characters.
Teal/lavender/amber colors fit the existing surfaces; blue distinguishes links and
functions, rose distinguishes deletions, and code receives a subtle background.
Selection overrides foreground/background for readability. Copy still returns
exact source Markdown, including escapes, entities, links, fences, and Unicode.

Explicit code fence hints take precedence. Unlabeled blocks can use supported
first-line hints such as shebangs; unknown labels remain plain. Rust, JavaScript,
Python, JSON, shell, and diff grammar lookup are tested. No arbitrary-code language
classifier or filename detection is implemented; the fixture has no filename
metadata. Composer and full-draft preview coloring are outside this first pass.

Redraw previously rebuilt every wrapped transcript row. The transcript now caches
source annotations and wrapped layout: mouse selection, scrolling, and activity
redraws reuse both; resizing reflows without reparsing syntax. Visible content
changes invalidate the cache. The event loop idle poll timeout is now 16 ms.
Tests prove cache reuse and preserved selection; this does not constitute an
end-to-end latency measurement or universal terminal responsiveness guarantee.

Twenty-eight Rust tests, formatting, Clippy with warnings denied, and the real-PTY
paste/selection/edge-scroll/clipboard journey pass. A dedicated
[color preview](../../rust/crates/terminal-ui/previews/colors-128x40.png) shows Rust,
JSON, and diff highlighting. This remains a local fixture presentation experiment.

## Evidence and validation limits

Follow-up lineage check: the operator requested incorporating the existing
provider-neutral interface proposals. The four linked port, client-protocol,
reconciliation, and mechanism documents were read directly; coverage checks
reported matching metadata and no recorded gaps at graph generation
`2026-10-05T05:52:30Z`. This revision corrects the initial omission of
`OperatorProjection` by name; it does not claim additional implementation or
change the supervision-first product direction.

Repository HEAD at inspection: `7583e2095725957470e28cc11fc69404386fec63`.
The working tree contains unrelated in-progress documentation; this assessment
does not reconcile or modify it. This is a UI-focused assessment, not an audit
of total migration completion or the currently activated host generation.

Repository retrieval used Tier 2 verification in graph project
`home-bline-code-work-engine`, generation `2026-10-03T17:48:17Z`, with direct
source verification of the material boundaries. Targeted class, service,
control-method, and test-file queries were complete; broad discovery was
narrowed before reliance. The inbound switchboard trace identifies the CLI and
executable-generation role environment as its two non-test construction callers.
Coverage checks reported no recorded gaps and matching metadata for relied-on
existing files; this is best-effort evidence, not proof of global completeness.
This newly authored document was absent from the index and verified directly.

Validation executed:

```sh
node --test app-server/tests/operator-switchboard.test.mjs \
  app-server/tests/observable-app-server-transport.test.mjs \
  app-server/tests/app-server-protocol-proxy.test.mjs
```

Result: **27 passed, 0 failed, 0 skipped**. These exercise local/fake transport,
switchboard, and proxy behavior, including control access during an in-flight
request and disconnect independence. They do not prove live provider parity,
production lifecycle readiness, OpenTUI rendering, or end-to-end UI integration.
OpenTUI conclusions come from official documentation inspected during this
session; no OpenTUI runtime benchmark or installation was performed.


## Experimental Rust lifecycle timeline follow-up

The operator supplied and accepted the provisional controlled handoff milestone.
Shared board message 1119 is the UI sequence source; the lifecycle owner's
`experiments/context-lifecycle-rust/NATIVE_VERTICAL.md` supplies meaning and limits.
The source remains worker-reported and parent-validation-pending at that message.
A retained result artifact was also inspected, but its coordinator identity differs
from message 1119; the UI does not combine those runs or copy their identities.
No native command or live-provider validation was performed by the UI session.

A fourth, read-only Lifecycle view now presents A input 1, package capture,
B readiness, activation, and B input 2 as an authored logical sequence. It does
not pretend to be a live event stream or supply invented timestamps. Six separate,
explicitly labeled design fixtures explore missing completion evidence, conflict,
restart uncertainty/no resend, stale command rejection, unsupported interaction,
and missing readiness. Native observation, lifecycle outcome, next/blocking
condition, and provenance have separate context sections. Native Completed and
process exit never serve as generic success indicators.

The composer remains visible but disabled, drafts survive, and send/recipient/time
scope controls cannot act from this view. Narrow-terminal context switching still
works. No lifecycle-crate file was changed and no unstable Rust DTO dependency was
added. The parent lifecycle session still owns contract validation and later
read-only projection integration. Thirty UI tests, formatting, Clippy, and the
existing real-terminal smoke test validate this presentation-only increment.


### Lifecycle within the conversation

The operator requested agent output and lifecycle events in one conversation,
with percent context usage. `/chat demo` now shows an explicitly illustrative
mixed timeline: agent Markdown/code, typed system capture progress, a context
handoff, then an uncertain-completion event. State labels and lavender/teal/amber
styles distinguish system records from prose. F3 expands their details. This
example's alternate uncertain ending is not attributed to the successful run.

The header formats usage only from a valid matching-context used/capacity sample;
replacement clears it and stale-context samples cannot become successor usage.
No actual usage is available, so the example displays unavailable, not a guessed
percentage. The UI has no runtime subscription, genuine streaming output, or
freshness-validation contract yet. Those remain Rust projection integration work.
Drafts are retained and input is disabled in the illustrative example; normal
work-fixture conversation remains available through `/conversation`.

Thirty-three Rust tests cover the existing interface plus context-bound usage,
replacement/invalid-data behavior, typed mixed entries, event expansion and copy,
and read-only example draft preservation. Formatting, Clippy and the existing
real-terminal smoke check pass. No Rust lifecycle-owner files were changed.


### Retained live proof in the conversation

The 2026-10-07 bounded live proof now has a separate offline archive in the UI.
`/replay live` displays exact captured agent outputs and typed system records for
A's first input, package capture, B's readiness, and B's second input. F3 reveals
source hashes, JSON pointers, and turn/effect/delivery identities. The context
pane carries provenance and scope limitations. Context usage stays unavailable.

`/replay rejected` keeps the prior zero-turn admission rejection separate;
`/replay validation` presents reported tests and simulated-inference probes.
Historical records are labeled RECORDED, scoped accepted text turns TEXT ACCEPTED,
and rejected admission REJECTED. Closure and process cleanup do not imply a broader
settlement result. This is a logical sequence, without invented event timestamps.

The version-1 `work-engine-ui-history` archive is presentation-owned and bundled
at build time. A reproducible offline exporter checks the parent milestone's
receipt hash, source hashes, identity/outcome agreement, and output digests. This
archive does not freeze the lifecycle runtime contract. A versioned read-only
runtime projection remains the next integration boundary; no commands, inference,
database access, or provider connection were added. Lifecycle-owner files remain
untouched. Input is disabled and retained work drafts survive inspection.

Thirty-seven UI tests, formatting, Clippy, archive reproduction, and the existing
PTY smoke test cover this increment. The UI test count is separate from the
lifecycle proof's reported 30 tests and three controlled probes.


### Independent command palette and composer-only keyboard input

The operator approved replacing slash commands and persistent panel focus with an
independent **Ctrl-P** palette near the top of the terminal. A visible Commands
control opens it by mouse. Slash is now ordinary message text, including at an
empty draft; former single-character navigation keys also type normally. Tab no
longer cycles panel focus. Palette Escape preserves the draft insertion point.

Mouse inspection, selection, and scrolling do not steal keyboard input or change
the message recipient. Historical views disable messaging while retaining command
navigation. Searchable commands expose existing work items, lifecycle scenarios
and steps, and visible conversation events. Panel main/context and viewport scroll
commands keep keyboard-only inspection possible in compact layouts. Detail
expansion preserves the inspected event anchor. No run creation, branch mutation,
provider action, or new backend capability is implied by these local commands.

This supersedes the earlier slash/focus interaction descriptions in this design
log. The prototype README and in-app help describe current controls. Thirty-nine
UI tests plus formatting, Clippy, terminal smoke checks, and regenerated previews
validate this increment; lifecycle runtime ownership and integration boundaries
remain unchanged.
