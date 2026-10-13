# Provisional task inspection

This UI-only slice implements the proposed projection described in
[task-observation.md](task-observation.md). These fixtures are **not an accepted
`task.observe.v2` protocol**, backend integration, or evidence about a live task.
The S8 advisory in board message 1664 informed the availability distinctions;
no S8 runtime, owner contract, transport, or database code changed.

Launch from the repository root:

```sh
CARGO_TARGET_DIR=/home/bline/.local/state/work-engine/ui-stack-evaluation-20261006/target \
  cargo +1.92.0 run --manifest-path rust/crates/terminal-ui/Cargo.toml \
  --offline --locked -- --task-fixture pending
```

Replace `pending` with `result`, `stale`, or `refused`. Ctrl-P exposes the same
`task fixture …` commands. `overview`, `graph`, `conversation`, and `lifecycle`
retain task inspection. `inspect <node>` selects a node without changing the
composer recipient. `work fixture` exits task inspection and restores the original
illustrative work UI. Existing replay, lifecycle examples, connection reads,
and `dashboard` snapshot alias remain available. The former Dashboard tab is
now labeled Graph.

The task graph draws connected, selectable nodes when the main viewport is at
least 82 columns by 24 rows. A compact relationship list handles smaller
viewports. Both use the same identifier-bound edges and inspection scope; the
selected node has a teal background and bright title. Edges are fixture
bindings, not inferred causality. A single
click scopes inspection and subsequent Conversation; a second click on the same
node within 450 ms opens Conversation directly. Ctrl-P `inspect` also works on
narrow terminals. Nodes without attributable text explicitly say so.

Overview reads in this order: objective, needs attention, latest result,
execution. It names pending questions, unavailable reads, stale publications
and capacity refusal where they affect understanding. Empty results explicitly
say no result was published. Technical component states, revisions and bindings
live in Context, with a quieter background. The persistent Inspecting label
names the selected node and its task.
Longer content lives in the grouped Conversation and Context. Mouse wheel or
Ctrl-P `scroll timeline down` scrolls the main task view; `panel context` and
`scroll context down` expose all details at narrow sizes. F3 expands source
references in Conversation. Dragging and Ctrl-C copy source Markdown. No overall
success status, progress percentage, agent graph or chronological transcript is
manufactured.

| Fixture | Distinction exercised |
| --- | --- |
| pending | Pending operator question while physical work remains running; result absent. |
| result | Recorded answer and published result while delivery is unsettled and admission closed. |
| stale | Owner and native reads unavailable, stale owner publications/result and segments retained, independently current lifecycle. |
| refused | Explicit proposed capacity refusal, with no native collection supplied; never interpreted as an empty collection. |

All cases also expose `not_requested`, `not_applicable`, and legacy `unknown`.
Components retain their own binding and revision domain; no native cursor is
invented from lifecycle. Fixture receipts A/B denote local read receipts, not
event timestamps or a shared commit. This is a static projection example: there
is no task refresh transport, response validation, host restart handling, task
ID input, or stale-response race claim. The existing lifecycle client remains
separate. Four scenarios share the single fixture task `task:ui-17`.

Drafts and their recipients survive every task view; send, paste, and recipient
retarget are disabled throughout task inspection. No native command is wired.

## Validation and previews

Five semantic/interaction tests supplement the existing 47 UI tests:
component freshness and grouped sources; mouse scoping/double-click, Markdown
copy, drafts and recipient preservation; normal/narrow stale-result rendering; visual edges, nonoverlapping node hit
regions and responsive identity; Overview hierarchy, absent result, refusal and
current execution.
The provider-free PTY check additionally drives direct launch, real SGR mouse
double-click, scenario changes, resize and terminal restoration.

```sh
export CARGO_TARGET_DIR=/home/bline/.local/state/work-engine/ui-stack-evaluation-20261006/target
cargo +1.92.0 test --manifest-path rust/crates/terminal-ui/Cargo.toml --offline --locked --bin work-engine-terminal-ui
rustfmt +1.92.0 --edition 2024 --check rust/crates/terminal-ui/src/task.rs rust/crates/terminal-ui/src/ui.rs rust/crates/terminal-ui/src/tests.rs
cargo +1.92.0 clippy --manifest-path rust/crates/terminal-ui/Cargo.toml --offline --locked --bin work-engine-terminal-ui -- -D warnings
cargo +1.92.0 build --manifest-path rust/crates/terminal-ui/Cargo.toml --offline --locked --bin work-engine-terminal-ui
python3 rust/crates/terminal-ui/scripts/pty_smoke.py "$CARGO_TARGET_DIR/debug/work-engine-terminal-ui"
python3 rust/crates/terminal-ui/scripts/task_pty_smoke.py "$CARGO_TARGET_DIR/debug/work-engine-terminal-ui"
```

Snapshot names: `task-pending`, `task-result`, `task-stale`, `task-refused`,
`task-graph`, `task-conversation`; both plain and ANSI snapshots work. The latter
two select the pending case in the named view. PNGs use the existing approximate
terminal font exporter and actual Ratatui buffer colors.

- [Pending Overview, 128×40](previews/task/pending-128x40.png)
- [Stale Overview, 80×30](previews/task/stale-80x30.png)
- [Bound Graph, 128×40](previews/task/graph-128x40.png)
- [Grouped Conversation, 128×40](previews/task/conversation-128x40.png)

The static fixtures do not qualify an accepted backend or prove the current
v1 `unavailable` means capacity. Actual host integration and additional response
identity/refresh cases remain the later owner-reconciled slice.

Additional refinement previews:

- [Pending Overview, 80×30](previews/task/pending-80x30.png)
- [Published result, 128×40](previews/task/result-128x40.png)
- [Published result, 80×30](previews/task/result-80x30.png)
- [Stale owner and native reads, 128×40](previews/task/stale-128x40.png)
- [Capacity refusal, 128×40](previews/task/refused-128x40.png)
- [Capacity refusal, 80×30](previews/task/refused-80x30.png)
- [Compact Graph, 80×30](previews/task/graph-80x30.png)

This refinement deliberately leaves the separately owned S8 library client and
host tests untouched. Binary-only validation and explicitly named rustfmt
inputs avoid claiming qualification of that concurrent work.
