# Unified operator interface: first implementation result

Recorded 2026-11-13 UTC for the user's 2026-11-12 request to take over unified
interface development. Browser and terminal access have equal priority.
The [development plan](../planning/unified-operator-interface-20261112.md)
defines this increment and its remaining work.

## Result

Explicitly selected native sessions now have one bounded read projection shared
by CLI, TUI and the authenticated web Work dashboard. The Rust
`work-engine-observe` collector calls the existing pinned Events MCP dispatcher
through `session-query`, preserving its transcript parsing, redaction and source
pin checks. It publishes an atomically replaced private file. The terminal reads
that file every second; the optional laptop transport adds it to the existing
authenticated web mailbox payload. Both consumers preserve exact identities,
source read times and per-session unavailability.

The projection reports session observations only. It cannot establish task
execution, completion, campaign membership, settled spend or command authority.
A successful source read with no matching entries differs from an unavailable
read. A mailbox refresh does not reset source age. Unsupported or malformed
snapshots are refused, and failed refreshes visibly retain last successful data.

Implementation locations:

- [Rust collector, contract and usage](../rust/crates/operator-observation/README.md).
- [Terminal consumer](../rust/crates/terminal-ui/src/observation_view.rs) and
  [PTY checkpoint](../rust/crates/terminal-ui/scripts/operator_observation_smoke.py).
- Sibling web repository: [consumer and opt-in setup](../../devwits-auth/docs/operator-observation.md),
  [activity view](../../devwits-auth/src/operator-activity.jsx), and
  [browser checkpoint](../../devwits-auth/tests/ui-operator-observation.smoke.mjs).

The terminal's historical view also has three mechanical Clippy corrections
(`or_else` to `or`, and two collapsed conditionals); its behavior is unchanged.

## Qualification

Final focused checks passed:

- Rust: six observation tests, collector build, terminal binary build, package
  formatting check, and both scoped Clippy checks with warnings denied.
- Web: 82 Node tests, 31 laptop-sync Python tests, and production build.
- Controlled actual Events parser to Rust collector: available and unavailable
  sessions, source update, credential redaction, private output and pin mismatch.
- The same collector-produced snapshot through actual desktop and phone browser
  views: bounded text, partial source failure, retained data during a refresh
  outage, invalid schema refusal and sign-out clearing.
- One actual TUI process: first observation, automatic refresh after atomic file
  replacement, visible failure after file removal, successful exit with all
  terminal settings restored, and 128×40 / 60×24 rendered frames. The PTY harness
  forces resize redraws for text assertions; it does not emulate a terminal.
- Existing web dashboard smoke: 14 layout checkpoints passed.
- The final collector also read this development session locally: one available
  session with ten bounded entries. Transcript content was not published to the
  web mailbox or copied into this record.

An early controlled transcript used the wrong Events filename layout and an
early PTY assertion searched raw cursor-delta bytes. Both harness issues were
corrected before the final passing runs. Earlier scratch evidence is retained.

Final evidence is under
[`/home/bline/cbm-work/unified-interface-20261113/final/`](/home/bline/cbm-work/unified-interface-20261113/final):

- [Implementation file and binary digests](/home/bline/cbm-work/unified-interface-20261113/final/implementation-manifest.json).
- [Collector proof](/home/bline/cbm-work/unified-interface-20261113/final/collector-checkpoint.json),
  [browser proof](/home/bline/cbm-work/unified-interface-20261113/final/browser-checkpoint.json),
  [terminal proof](/home/bline/cbm-work/unified-interface-20261113/final/terminal-checkpoint.json),
  and [local session read metadata](/home/bline/cbm-work/unified-interface-20261113/final/local-read-checkpoint.json).
- [Phone rendering during the controlled outage](/home/bline/cbm-work/unified-interface-20261113/final/observation-390.png)
  and [desktop rendering](/home/bline/cbm-work/unified-interface-20261113/final/observation-1440.png).

All three controlled proofs bind the same snapshot SHA-256
`adda11ef16045395fe738c425e3366e0df3b8279df08cb8804a390fe5d586f52`.
The Rust-emitted fixture is byte-identical in both repositories, SHA-256
`235768ba4f03aeaa809d97955ba70c9b5567424344eb3e8688db00baf4be8a61`.
These are local uncommitted worktree qualifications, not immutable accepted
workflow checkpoints or production UI-0/UI-1 acceptance.

Repository discovery used Tier 2 graph evidence for project
`home-bline-code-work-engine`, with exact-path coverage checked at generation
`2026-11-13T05:28:12Z`. Changed metadata and unresolved callsites were checked
against relevant source; graph call counts are not treated as complete. The
sibling web paths are outside that graph and were checked directly. This is a
task-directed verification, not an exhaustive repository audit.

## Operational limits and continuation

The accepted retained Events source used here is `d44b1bbf`, SHA-256
`933eda1c2af1f3f76819fbe2099269c6a04c33fcdb8adf52969cdf80f5beeefd`.
The current working-tree Events source differs from the client's accepted pins.
No pin was added or bypassed. Qualifying the current Events revision belongs
with the query owner before selecting it in a monitor profile. These checks do
not qualify the current remote Events connector, which required reauthentication
during exploration.

Collection is sequential polling of a bounded tail, not atomic cross-session
history or event subscription/replay. The browser retains its existing
60-second visible/online refresh and the laptop sync adds its own delay. The
120-second freshness warning is a display cue. The browser qualification used
mocked authentication/mailbox reads with nonlocal traffic blocked; real Redis
storage and production delivery were not exercised.

No live configuration, deployment, mailbox publication, persistent collector or
sync service was activated. No provider was launched or approval signed. Both
repositories remain uncommitted. Work Engine's observed base was `697d66213b3dd89e57a4f28a20d13dff8dbe41a0`;
the web base was `285638858097d656797709fb376aecc41d5b0240`.
Concurrent research/task-host edits were preserved. The shared root Cargo lock
contains both this increment's new package and a concurrent `research-register`
dependency change; the latter is outside this increment's ownership.

Next development should qualify the current Events read source, join sessions
to tasks/campaigns using owner-supplied identities, and then add event wakeups,
replay and faster browser delivery. Budget/closure views and owner-admitted
controls remain separate open work. Transcript titles, recency and prose must
not be used to invent those relationships or states.
