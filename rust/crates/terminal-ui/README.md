# Work Engine terminal UI

For the product direction, implementation status and next work, start with the
[UI implementation roadmap](docs/implementation-roadmap.md). It maps this client
to the accepted [control-plane causal observability design](../../../app-server/docs/control-plane-causal-observability-ui.md).

A maintained standalone terminal application using **Ratatui + Crossterm +
tui-realm**, **rataflow** for the work graph, and **ratatui-textarea** for editing.
Work views use invented fixture data. The separate Lifecycle view is an authored
read-only reconstruction of a reported controlled run, with labeled design scenarios.
The conversation can also display a bundled archive of real retained live-proof output and published S3 lifecycle contract examples. An explicit S4 Unix socket connection can read lifecycle observations; it provides no provider or command authority, local persistence, or ambient endpoint discovery. Dependency
downloads happen during Cargo setup. Closing the app loses its local state;
nonempty drafts prompt before normal exit.

Promoted from `app-server/experiments/terminal-ui` on 2026-10-07.
The app retains its own Cargo workspace and lockfile until shared-workspace
integration is coordinated. This move does not establish runtime readiness.
Existing Python validation/export scripts remain transitional development tooling.

## Run

From the repository root, with Rust 1.88+ and Cargo:

```sh
cargo run --locked --manifest-path rust/crates/terminal-ui/Cargo.toml
```

For a compatible S4 lifecycle service, select both the socket and subject explicitly:

```sh
cargo run --locked --manifest-path rust/crates/terminal-ui/Cargo.toml -- \
  --connect /absolute/path/to/service.sock --subject subject-id
```

The app opens on a passive observation. The header names the selected subject and
socket. Ctrl-P offers `connection refresh`, `connection reconnect`, and
`connection disconnect`. Refresh and reconnect re-read with the last known
store, stream, and cursor; a changed identity or regressed revision is rejected.
This lifecycle-only mode has no automatic polling. A service outage or read timeout leaves any
prior observation labeled **last known only**. Explicit disconnect has its own
status; a later reconnect uses the same selected endpoint. Choosing a fixture,
contract example, or archive view invalidates pending replies and returns to
that local dataset; `connection reconnect` reopens passive inspection. Drafts
remain retained, but input and simulated sends are disabled while inspecting
the connection. The worker has one bounded request slot and a 600 ms transport
budget; it joins on exit.
The connection displays the service's typed snapshot fields, including explicit
unknown and unavailable values. It supplies no agent text or calculated context
percentage.

For a retained-task host supporting `task.observe.v2`, select a task instead:

```sh
cargo run --locked --manifest-path rust/crates/terminal-ui/Cargo.toml -- \
  --connect /absolute/path/to/service.sock --task task-id
```

Optional `--delivery delivery-id` and `--command command-id` qualify reads;
command observations retain their explicit **task binding not established**
status and have no graph edge to the task. Overview, Graph, Conversation, and
Lifecycle use the same observed components. Inputs, questions/answers, and
published results are grouped owner publications, not a chronological transcript.
Selecting a graph node scopes Conversation without changing the message recipient.

Task mode refreshes every four seconds after each completed read, with one read
in progress, one bounded request slot, and a 600 ms transport budget. The same
Ctrl-P connection commands refresh, disconnect, or reconnect. Disconnect stops
automatic reads and invalidates in-flight replies. Failures retain compatible
last-known components with stale labels; host identity or target changes clear
incompatible anchors. Unsupported versions are reported without v1 fallback.
Capacity refusal remains distinct from unavailability or an empty collection.
Per-component scope, revisions, completeness, host identity, and source references
remain inspectable in Context. A result publication does not imply settlement.
Drafts are retained and sends remain disabled. No real provider is launched.

Use a terminal around **128×40** for the graph and side panel together. At less
than 110 columns, Ctrl-P → `panel context` brings context forward; `panel main` returns to the main view. Short
or narrow graph areas use a list of the same subjects. Below 60×18, the app asks
for more space and retains drafts. No terminal required for a text preview:

```sh
cargo run --locked --manifest-path rust/crates/terminal-ui/Cargo.toml -- --snapshot 128 40
```

The snapshot command also accepts `history`, `conversation`, `commands`, `paste`, `compose`, `selection`, `colors`, `contract-failed`, `contract-historical`, `contract-unavailable`, or `contract-unsupported` after dimensions.
Use `--snapshot-ansi` instead of `--snapshot` to preserve RGB colors.
Saved previews: [dashboard](previews/dashboard-128x40.txt),
[compact dashboard](previews/dashboard-80x24.txt),
[history](previews/history-128x40.txt),
[conversation](previews/conversation-128x40.txt). These are plain-text Ratatui
buffers; the interactive app adds color and composer highlighting.
Color previews: [dashboard](previews/dashboard-128x40.png) and
[command picker](previews/commands-128x40.png). These use the actual Ratatui
cell colors and positions with approximate font rendering; terminal fonts may differ.

## Historical evidence replay

Use **Ctrl-P → replay live** for the retained 2026-10-07 live text proof: A's first
output, package capture, B's readiness, the actual second input, and B's recovered
fact. **replay rejected** shows the separate admission rejection that consumed
zero provider turns. **replay validation** shows the receipt's reported 30 default
tests and three controlled probes; those counts are not additional live turns.

The chat shows exact captured output with typed, colored system records. **F3**
expands provenance, including source paths, JSON pointers, SHA-256 hashes, and
native turn/effect/delivery identities. Drag and Ctrl-C copy source text; expanded
provenance is included only while expanded. Ctrl-P → `panel context` opens evidence on narrow terminals. Drafts are retained and input/recipient operations remain
disabled. **conversation** returns to the ordinary work fixture.

This is logical historical ordering, not timestamped playback or a running-engine
subscription. Context usage and HTTP inference request count are unavailable.
Text acceptance is scoped to the retained completion domain; process cleanup is
separate. The captured proof does not establish production readiness.

[Replay preview](previews/replay-128x40.png) ·
[Separate rejection](previews/replay-rejected-128x40.png).
Snapshot modes: `replay`, `replay-rejected`, `replay-validation`.

`replays/live-proof-2026-10-07.v1.json` uses the UI-owned
`work-engine-ui-history` archive format, version 1. This is **not a frozen runtime
wire contract**. It is bundled at build time; the binary reads no runtime database
or credentials. The parser rejects unsupported versions, unknown record types,
and missing or malformed provenance. It does not authenticate arbitrary external
archives or revalidate source files at runtime.

The offline exporter pins the parent receipt hash from board milestone 1127,
checks its retained source hashes, correlates outcomes and identities, and checks
exact output digests and carried-fact recovery. Run from the repository root:

```sh
python3 rust/crates/terminal-ui/scripts/export_live_replay.py --check
```

Without `--check`, it regenerates only the UI archive from the retained JSON
artifacts. It starts no process or inference and does not modify lifecycle-owner
artifacts. The Rust replay tests cover section separation, malformed archive
refusal, exact output copying, provenance expansion, draft retention, and disabled
operations.

## S3 lifecycle contract examples

Use **Ctrl-P → contract failed unsettled**, **contract historical complete current blocked**, **contract observation unavailable**, or **contract unsupported variant**. These are the published `lifecycle-wire` fixtures, parsed with its strict public parser and labeled **S3 CONTRACT EXAMPLE** throughout. They are neither live observations nor the retained historical replay. F3 expands the exact subject, context, store/stream cursor, delivery/effect/attempt scope, outcome, settlement, transition, current admission, provenance, references, and field availability. A failed outcome does not settle its effect; a completed historical transition does not grant current admission. An unavailable or unsupported result shows an explicit observation error. No agent transcript is supplied by S3.

The UI now depends on the public `lifecycle-wire` and passive `lifecycle-client` packages through its standalone lockfile. `src/contract.rs` projects their typed values into UI-owned rows. Controlled in-memory transport tests exercise the actual `ReadClient` snapshot, wait, cursor-gap reread, deadline, and reconnect behavior; this test port is not a production transport. The projection path retains last-known data only with an explicit health label. Invalid identity, regressed cursor, unsupported version or variant, and invalid observations display errors. No native command or simulated send is enabled in contract/client inspection. S3's raw `measurement: FieldV1<String>` has no kind, capacity, or context binding, so the UI never computes a context percentage from it.

The accepted S3 manifest SHA-256 is `f4c73a45602284989fc352f5529f421a5e2c73cdaece2c4678035825032727be` (published `634161c2f54dc4fcf9dcb53936951ecefe457235`). Verify its historical source bytes, current stable v1 schemas and fixtures, and the current S4 public client/wire source receipt with:

```sh
python3 rust/crates/terminal-ui/scripts/check_s3_pin.py
```

[U0 result and U1 needs](docs/u0-preparation.md) records the original supported fields and deferred service work. [The S4 source receipt](docs/s4-client-pin.json) binds the current public inputs to commit `41992a22d86b041d2db357d16345c477804af63a`.

## Agent output and lifecycle events in chat

Run **Ctrl-P → chat demo** to inspect a read-only conversation example. Agent messages
and typed lifecycle system events are interleaved: capture progress is lavender,
readiness/handoff is teal, and uncertain completion is amber. Explicit system
labels distinguish these records from agent prose. **F3** expands event details
along with conversation detail; returning to Summary collapses them.

The example contains invented agent output and an invented uncertain ending. It
is not captured output from the successful controlled run. Drafts stay retained,
sending/editing is disabled, and **conversation** returns to the normal work
fixture. Markdown and code highlighting, drag selection, and source-text copy
continue to work. Expanded event details join copied source text; labels do not.

The conversation header has an active-context usage indicator. It can format a
matching used/capacity measurement as a percentage, rejects invalid or wrong-context
measurements, and clears the measurement on context replacement. The fixture shows
**Context B · usage unavailable**: no measurement is supplied or inferred from the
length of displayed text. Historical views do not reuse current live usage.
Runtime identity binding, telemetry provenance/freshness, and actual streaming
subscription remain future read-only projection integration work. These UI types
are presentation state, not a proposed canonical lifecycle protocol.

[Chat preview](previews/chat-128x40.png); snapshot mode: `chat`.

## Read-only lifecycle timeline

Open **Ctrl-P → lifecycle**. Click a row or use **inspect step** commands to
inspect a step; **panel context** brings its context forward on narrow terminals.
Search **scenario** to choose the reported handoff or six illustrative failures.

The positive sequence is attributed to shared coordination-board milestone **1119**:
context A input 1 → captured package → context B readiness → successor activation
→ B input 2 once, under one coordinator. It is an authored logical sequence from
that provisional report, not an imported event stream or reconstructed timestamps.
The report used the real Codex CLI with simulated inference and explicitly left
parent validation pending. Its UI label preserves that qualification.

The evidence pane separates native observations, lifecycle outcome, next/blocking
condition, and source limitations. Missing evidence, contradiction, restart after
entry, stale commands, unsupported interactions, and missing readiness each have
**DESIGN FIXTURE** views; they are not captured failures or claims of passing tests.
The semantic reference is
[the experimental native contract](../../../experiments/context-lifecycle-rust/NATIVE_VERTICAL.md).
A native Completed status, process exit, or package existence cannot alone become
a successful lifecycle outcome.

This view has no runtime connection and no native command wiring. Input is disabled;
existing drafts are retained. Sending, retargeting, and work time-scope commands
are unavailable here. View/navigation commands remain usable. Return to a work
view before editing a draft or simulating a send.

Previews: [reported handoff](previews/lifecycle-128x40.png) and
[missing evidence](previews/lifecycle-uncertain-128x40.png). Snapshot modes are
`lifecycle` and `lifecycle-uncertain`. `lifecycle.rs` owns these presentation
fixtures, not a canonical lifecycle DTO or adapter. The next integration boundary
is a separately versioned read-only projection once the Rust owner announces it.

## Try the first journey

1. Open **Ctrl-P**, search `follow dependency`, and press Enter. Repeat to move
   from **Supervise work** through **Interaction review** to **History action scope**.
   Or search `inspect` and choose the work item directly.
2. Use `context evidence` or `context history`. At narrow widths, context occupies
   the main area; `panel main` brings the main view back. `scroll context` commands
   move it without touching the composer cursor.
3. Run `conversation`; `detail evidence` expands activity and source details.
   Search `event` to jump to a visible message. Event inspection survives F3 expansion.
4. Run `to reviewer:main` to explicitly choose a recipient. Type a response;
   Shift-Enter adds a line and Enter simulates send. This creates no owner receipt.
5. Run `dashboard`, then `inspect build:prototype`. Selection changes; the draft
   recipient and insertion point do not.
6. Use `history` and `scope historical` to inspect the earlier fixture branch.
   Messaging and retargeting are disabled, but Ctrl-P remains available.
   `scope live` restores the live fixture scope.
Each explicitly selected recipient keeps a separate draft. `F7` saves the old
role's editor state and restores the selected role's draft; it never migrates
text between recipients. This is session-local UI state, not a durable input
queue. Simulated messages name their actual recipient and remain live-scope
entries. Inspecting history never reclassifies those messages as historical.

## Chat input and paste

**Enter sends; Shift-Enter inserts a newline** in the composer. F8 remains an
alternate send shortcut. In the command picker, Enter runs the selected command. Historical
scope and empty-input checks apply equally to Enter and F8.

Bracketed paste is inserted as text, including multiple lines and slash-prefixed
content. Trailing newlines do not submit it. CRLF and CR line endings normalize
to LF; tabs and other content remain in the draft. A paste notice distinguishes folded blocks from ordinary insertion; Ctrl-Z
undoes the insertion (Ctrl-Y restores it). Pasted text never
opens or executes a command. The app requests bracketed-paste mode; terminals
that do not report bracketed paste cannot distinguish pasted keystrokes from
typing, so that guarantee depends on terminal support.

The Unix terminal setup requests enhanced keyboard reporting and restores it on
exit. Shift-Enter requires a terminal that reports the modifier distinctly;
legacy terminals may send the same bytes for Enter and Shift-Enter. **Alt-Enter**
is the newline fallback in those terminals. See the pinned
[Crossterm keyboard protocol documentation](https://docs.rs/crossterm/0.29.0/crossterm/event/struct.PushKeyboardEnhancementFlags.html).
The PTY check injects enhanced Shift-Enter and legacy Alt-Enter sequences and
checks mode restoration; it is not certification for every terminal emulator.

## Growing composer and folded paste blocks

The composer starts at five rows, grows with typed lines and soft-wrapped text,
and shrinks when text is removed or sent. It is capped at about one third of the
terminal height (at most twelve rows); arrows and PgUp/PgDn navigate/scroll
inside it beyond that point. The work/context area and footer remain available.
Height estimation is approximate; the editor owns exact wrapping and scrolling.

A paste with **six or more lines, or at least 1,000 Unicode characters**, folds
into a label such as `[Paste #1 · 40 lines · 1750 chars]`. These thresholds are
prototype presentation choices. This is visual folding, not summarization or
compression of the text sent to the recipient. Line endings normalize as above;
the full payload is retained and expanded in place on simulated send.

- Type before or after a block normally. Multiple blocks retain their ordering.
- Cursor movement, selection, Backspace/Delete, and undo/redo treat a folded
  block as one object. Ctrl-Z can restore a deleted block and its full payload.
- **Ctrl-O** opens a read-only view of the full current draft. Arrows/PgUp/PgDn
  scroll it; Home/End jump; Esc or Ctrl-O closes it. Input and sending are disabled
  while inspecting. This first version does not edit inside a folded block;
  delete and paste a replacement to revise that payload.
- Each recipient retains its own blocks. Historical sending stays disabled.
  Typing or pasting a lookalike label never substitutes another block's payload.

`draft.rs` owns each role's editor and retained paste objects. Private internal
markers make ordinary editor selection and undo atomic; literal input in that
Unicode range is escaped into separate literal objects. Display labels and send
text are separate projections. Retained objects remain available for undo/redo
until that role's draft is sent or the app exits; there is no disk persistence.
After sending, the fixture conversation currently displays the expanded text.

Previews: [growing composer](previews/compose-80x24.png) and
[folded paste](previews/paste-80x24.png), both at 80×24. Additional tests cover full
payload delivery, ordering, deletion/undo/redo, literal-label/Unicode collisions,
read-only inspection, recipient/history preservation, growth/shrink/resize,
soft-wrapped navigation, and stable internal scrolling. The PTY smoke test also
pastes and inspects a folded block.

## Mouse selection and Markdown copy

- Click a top view tab to change views or a work list row to inspect that subject.
  These clicks never change the recipient or move keyboard input out of the composer.
  The visible **Commands Ctrl-P** control opens the command palette.
- Wheel over the conversation or context to scroll it; over a work view to
  change the selected subject; over the composer to scroll its editor.
- In **Conversation**, drag with the left button to select message text.
  **Ctrl-C** copies the selection; **Esc** clears
  it. Without a selection, Ctrl-C retains its existing quit behavior.
- Selection can cross messages. It copies their original Markdown, joined by
  two newlines, omitting role/subject labels, simulated-send labels, padding,
  and soft-wrap breaks. Unicode graphemes remain intact. The prototype displays
  source Markdown with syntax colors; hiding delimiters and richer rendered
  Markdown layout remain separate follow-ups.
- Selection survives scrolling and resizing; changed visible source clears it
  rather than applying old offsets to new text. Hold the pointer at the top or
  bottom edge while dragging to scroll and extend selection into offscreen history.
  This continues without mouse movement; moving farther beyond the edge speeds
  it up. Release or move back inside to stop. Keyboard input, resize, view changes, and modals cancel held dragging. The wheel remains available too.
  Double-click word selection, graph node clicking, and mouse cursor placement in the editor
  are not implemented in this first pass. The composer remains the keyboard target.
- Help, the command picker, the full-draft preview, and exit confirmation block
  mouse click-through. Help and full-draft preview accept wheel scrolling.

Copy uses [OSC 52](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html), which
requests the terminal's clipboard using base64-encoded UTF-8. The status says
**copy requested** because the protocol supplies no acceptance acknowledgment.
Your terminal must support and permit OSC 52; multiplexers may need configuration.
This prototype has no native clipboard-helper fallback. Terminal-native selection
(often Shift-drag) bypasses the app and does not provide its clean Markdown copy.
Mouse capture is restored on normal exit and panic alongside the other modes.

[Selection color preview](previews/selection-128x40.png).
`mouse.rs` maps rendered graphemes to source byte ranges; `clipboard.rs` writes
only explicit copy requests. Simulated message recipient metadata is now stored
separately from authored text. Selection is temporary view state, not engine truth.

## Markdown and code colors

Conversation messages now style headings, emphasis, bold text, links, inline code,
quotes, task markers, and fenced/indented code. Markdown punctuation stays visible.
The palette adds blue functions/links and rose diff deletions to teal strings,
lavender keywords, amber constants, and a subtle code background. Selection uses
its own contrasting foreground/background, so syntax colors cannot obscure it.

`pulldown-cmark` 0.13.4 identifies Markdown source ranges; `syntect` 5.3.0 highlights
code with bundled grammars and a custom theme. Explicit fence language labels win;
unlabeled blocks may use recognized first-line hints such as shebangs. Unknown
labels and ambiguous content remain plain. Rust, JavaScript, Python, JSON, shell,
and diff grammar lookup are tested. This is not arbitrary-code classification;
filename metadata is not present in this fixture schema, so no filename-based
detection is wired. No detected link or code executes.

Source text never passes through a Markdown-to-text conversion. Escapes, entities,
URLs, fence markers, original line breaks, and Unicode are copied verbatim from
source ranges. Incomplete fences remain readable and can be highlighted. Composer
input and the full-draft preview retain their current editing/inspection behavior;
this first color pass applies to the conversation.

The transcript caches syntax annotations and wrapped rows. Selection, scrolling,
and sample-activity redraws reuse them; width changes reflow without re-highlighting.
Content/detail changes invalidate the visible-content cache. This removes repeated
whole-transcript layout work from mouse selection. Event polling also uses a shorter
16 ms idle timeout; perceived responsiveness still needs terminal-specific feedback.
No end-to-end latency benchmark is claimed.

Open Ctrl-P → `conversation` for the styled fixture recommendation,
or export the dedicated [color preview](previews/colors-128x40.png):

```sh
cargo run --locked --manifest-path rust/crates/terminal-ui/Cargo.toml -- --snapshot-ansi 128 40 colors
```

## Color and command-picker experiment

The shell uses borderless charcoal surfaces, a plum context panel, a teal
composer, and amber attention highlights. Selection has its own tinted background;
inspection and composer delivery identity remain separate. Graph nodes retain
outlines to explain relationships. This assumes a dark, RGB-capable terminal;
terminal palette/accessibility modes remain future work.

**Ctrl-P** or the clickable **Commands** control opens an independent palette near
the top of the terminal. Its query never enters a draft. It stays above the composer
at wide and compact sizes; a description appears where space permits. The composer
is the sole persistent keyboard target. `/`, `q`, `d`, digits, and other unmodified
characters are ordinary message text. Tab no longer cycles panel focus.

- Search command names, description keywords, or abbreviations. Prefix matches
  rank first; short queries stay limited to name/token prefixes.
- Up/Down selects; **Tab completes without executing**; **Enter runs**;
  **Esc** or Ctrl-C cancels, preserving the draft and insertion point.
- Commands include `dashboard`, `history`, `conversation`, `detail evidence`,
  `context evidence`, `scope historical`, `scope live`, and `to reviewer:main`.
- Current work, lifecycle steps/scenarios, and visible conversation entries produce
  searchable `inspect`, `scenario`, and `event` results. These inspect local fixtures
  or bundled evidence; they do not discover running engines or create branches.
- `panel context` / `panel main` choose the compact viewport. `scroll context` and
  `scroll timeline` offer up/down/start/end, so keyboard inspection needs no panel focus.
  `graph` commands retain viewport fit, zoom and pan; `quit` retains draft confirmation.
- Historical views disable messaging and retain drafts. Palette navigation remains
  usable; unavailable recipient/scope commands show a reason and cannot execute.
- F8 cannot send while the picker owns input. Paste never executes a command.
  A typed or pasted leading slash is always ordinary composer content.

These are local UI commands. This is not a shell, a provider command list, or
live operation discovery. The command catalog in `commands.rs` is typed and context-sensitive; a future owner-provided catalog must retain availability,
target, authority, and outcome distinctions. Richer argument pickers, exact
subject previews, recent commands, and consistent search across work/evidence
are useful follow-on design experiments, not implemented features.

## Keys

| Key | Behavior |
| --- | --- |
| Ctrl-P / click Commands | Open independent top command palette |
| Up/Down, Tab, Enter, Esc in palette | Select, complete, run, cancel |
| F1 | Help; arrows scroll, Esc closes |
| F2 | Cycle views without changing recipient |
| F3 | Summary → Activity → Evidence; retains inspected event anchor |
| F4 | Context Overview → Evidence → History |
| F5 | Live fixture ↔ historical fixture scope |
| F6 | Graph/list preference |
| F7 | Explicitly target selected work owner in live scope |
| Enter / F8 | Simulate send when messaging is available |
| Shift-Enter / Alt-Enter | Newline / compatibility fallback |
| F9 | Advance sample activity counter |
| Arrows, Shift-arrows, PgUp/PgDn, Home/End | Composer cursor, selection, scrolling |
| Ctrl-O | Inspect full draft including folded pastes |
| Ctrl-Z / Ctrl-Y | Undo / redo |
| Drag / Ctrl-C / Esc in conversation | Select Markdown / request copy / clear selection |
| Ctrl-C without chat selection | Exit; y discards drafts, n cancels |

Tab and Shift-Tab outside the palette leave keyboard focus unchanged. Graph nodes
cannot be edited; graph mouse selection and editor mouse cursor placement remain
unimplemented. Persistent drafts, native clipboard fallbacks, IME/platform coverage,
and richer target discovery remain future work.

## Architecture and dependencies

- The pinned stack composes into one running binary: Ratatui 0.30.2, Crossterm
  0.29.0, tui-realm 4.1.0, rataflow 0.1.0, ratatui-textarea 0.9.3. Cargo.lock
  records transitive versions.
- A shared selection connects graph, list, context, history, and conversation.
  Selection, explicit delivery recipient, view, time scope, and detail are
  separate UI state. No provider type organizes these components.
- A context panel can carry the explanation and exact evidence references while
  the graph stays sparse. A list is more readable at 80×24. The current graph
  uses authored positions for five nodes; no automatic-layout or large-graph
  conclusion follows.
- tui-realm mounts the three rendering panels and routes keyboard input through
  the composer. The model temporarily directs input into the palette or other
  modals. Its value beyond event dispatch and mounting remains a question for
  the next iteration.
- rataflow provides actual node/edge rendering and viewport pan/zoom/fit. Clicking
  a visible node inspects its work item without changing the recipient, draft cursor
  or viewport. Hit areas follow zoom/pan and are clipped to the graph canvas.
  Node dragging, spatial navigation, automatic layout, and minimaps were not evaluated. Linear selection and dependency navigation belong to this
  prototype. No graph edits become workflow operations.
- Accurate wrapped-text scroll bounds currently use Ratatui's opt-in
  `unstable-rendered-line-info` API. Keep that compatibility dependency explicit
  as the app evolves.

`fixture.rs` owns invented projection data; `model.rs` owns temporary UI state
and simulated interactions; `ui.rs` owns rendering and tui-realm components;
`main.rs` owns terminal lifetime and event dispatch; `connection.rs` owns a single
bounded passive lifecycle socket worker; `task_live.rs` owns the task worker and
UI projection over S8's `task_observation.rs` reader. These UI types are **not**
a proposed canonical engine schema. The app lives separately from the Node app-server and
has its own Cargo workspace and lockfile.

## Validation

Run from the repository root:

```sh
cargo fmt --manifest-path rust/crates/terminal-ui/Cargo.toml --check
cargo clippy --locked --manifest-path rust/crates/terminal-ui/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path rust/crates/terminal-ui/Cargo.toml
cargo build --locked --manifest-path rust/crates/terminal-ui/Cargo.toml
python3 rust/crates/terminal-ui/scripts/pty_smoke.py \
  rust/crates/terminal-ui/target/debug/work-engine-terminal-ui
cargo build --locked --manifest-path rust/Cargo.toml -p lifecycle-service
python3 rust/crates/terminal-ui/scripts/connection_smoke.py \
  rust/crates/terminal-ui/target/debug/work-engine-terminal-ui \
  rust/target/debug/lifecycle-service
```

If using `CARGO_TARGET_DIR`, pass that directory's binary to the PTY script.
The PTY check requires a Unix host and Python 3, with no third-party modules.
Rust tests cover the full dependency-to-conversation journey, historical
send refusal, per-recipient drafts, selection/recipient separation, Unicode
editing, framework event order, progressive detail, and resize/composer-target retention. Additional command checks cover completion versus
execution, unavailable targets, draft preservation, literal slashes, unknown
queries, and compact layouts.
Source-selection checks also cover wrapped Markdown, Unicode, reverse and cross-message
selection, resize/scroll stability, source invalidation, modal isolation, and
stationary edge-drag scrolling in both directions with release/edge-exit stops.
Color checks cover source fidelity, token styles, conservative grammar detection,
unfinished fences, and cache reuse/invalidation across redraws and resizing.
The original PTY check captures an OSC 52 request from actual SGR mouse input, including stationary edge scrolling into previously
offscreen history, without
touching the host clipboard. It also exercises real terminal entry, key input, bracketed paste, command-picker completion/cancellation, resize,
exit/cancel, and normal terminal restoration. The connection PTY test starts
only an isolated disposable `read_only` service with an empty configured subject,
reads its initial snapshot,
stops and restarts it against the same owned store, and checks refresh,
unavailability, reconnect, explicit disconnect, and cleanup. Tests and
previews were checked on Linux with Rust 1.92.0. This is not cross-platform
certification, a latency benchmark, or proof of controlled-turn history or
cursor-gap behavior in a running service. The U0 controlled client tests cover
cursor-gap mapping; the S4 service owns the source proof. This smoke establishes
the UI/client/service read join and its outage/restart behavior only.
Unrelated engine tests are outside this app.

To regenerate a color preview, the optional developer script requires Pillow:

```sh
python3 rust/crates/terminal-ui/scripts/preview_png.py \
  rust/crates/terminal-ui/target/debug/work-engine-terminal-ui \
  commands 128 40 rust/crates/terminal-ui/previews/commands-128x40.png
```

## Next planned work

The [implementation roadmap](docs/implementation-roadmap.md) distinguishes the
committed one-task observation path from the broader accepted UI-0–UI-4 slices.
The next proposed work is to close installed-host-to-rendered-task-UI
qualification and bind the exact owner sources for the UI-0 S5 review-amplification
trace. These can proceed independently. Broader campaign visibility and causal
drill-down follow qualified source joins; economics and counterfactuals come later.

Overview is the task landing view; Graph and scoped Conversation are supporting
views. Their interaction behavior is already implemented. The
[design and migration exploration](../../../app-server/docs/opentui-ui-exploration.md)
remains historical lineage, rather than the current implementation status.

## Lifecycle client boundary

[UI observation requirements and U0/U1 handoff](docs/u0-preparation.md) maps the
UI's needs to lifecycle S3 and S4, including explicit gaps for agent text, usage
and cross-domain history. U0 contract fixtures and passive client behavior are
integrated; U1 connects the explicit S4 Unix socket read transport. Agent text,
usage telemetry, and broader cross-domain projections remain separate work.

Conversation presentation types live in `src/presentation.rs`. One selected
presentation supplies rendering and command-palette event inspection for work
fixtures, authored chat and retained archives. Source changes discard source-local
selection/anchors while drafts and recipients remain independent. The lifecycle
owner retains wire contracts, cursor semantics and authoritative state.

The [S5 populated observation validation](docs/u0-preparation.md#s5-populated-observation-validation)
joins this UI to an isolated actual service and simulated native generation
handoff, including a verifier-failure recovery fence. It validates current
snapshots; native operation outcomes and full event history still need public
projections. The reusable `scripts/s5_populated_smoke.py` harness can retain
socket snapshots and terminal frames for review.


### Repeatable installed-host refresh check

[Installed-host validation](docs/installed-host-validation.md) describes the
opt-in Linux Rust test and required pinned host/template environment. It runs
one actual UI process through pending question, authorized answer and published
result using the local simulator, asserting the evolving terminal screen with
no manual refresh. The default suite compiles but skips this environment-dependent
test. Test-only provider/PTY dependencies do not enter the UI application.

The [S5 source map](docs/s5-source-map.md) binds the historical review-amplification
case and records remaining source/access gaps for the broader causal UI.
