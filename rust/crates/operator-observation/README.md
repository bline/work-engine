# Shared operator observation

`work-engine-observe` reads explicitly selected native sessions through the
existing pinned Events MCP compositor. It publishes the same bounded activity
snapshot consumed by the terminal UI and the authenticated web Work dashboard.
It launches no provider, issues no command, and makes no task, campaign,
completion, budget or settlement claim.

Build from `rust/`:

```sh
cargo build --locked -p operator-observation
```

Use an owner-only absolute [session-query profile](../session-query/README.md),
with `bridge: null`. The existing query owner pins remain enforced. In
particular, source selection requires an exact query-owner pin. U1 adds one
explicit current revision as a review candidate after controlled monitor checks;
its [source map and limits](../../../../records/unified-operator-interface-u1-source-map-20261113.md)
remain separate from production and remote-connector qualification. This
collector neither adds pins nor silently loads another source.

```sh
work-engine-observe /absolute/profile.json --label 'Current investigation' \
  --session 11111111-1111-4111-8111-111111111111 --human

work-engine-observe /absolute/profile.json --label 'Current investigation' \
  --session 11111111-1111-4111-8111-111111111111 \
  --output /absolute/private/observation.json --interval 4
```

The label is operator-supplied navigation text, not an owner-authored task
objective. Select up to 20 unique session IDs. Collection reads at most ten
entries per session with a requested 2,000-character window. Redaction and
window expansion remain with the Events owner; reasoning/system records are not
requested. The exported activity can still contain sensitive work content:
selection and optional remote publication are explicit operator choices.

The default is one observation. `--interval` repeats after a completed collection
and requires an absolute output path. Queries are sequential and individually
bounded by the existing client. Output is a mode-0600 file atomically replaced
in its selected directory; it contains no profile credentials or paths. A failed
session read is an unavailable component, not a fabricated empty transcript.
Invalid configuration prevents collection; later source/pin failure appears as
unavailable. Stop an interactive collector with Ctrl-C. No service is installed.

Open the same file in the terminal, from the repository root:

```sh
cargo run --locked --manifest-path rust/crates/terminal-ui/Cargo.toml -- \
  --observe /absolute/private/observation.json
```

The monitor rereads the private file every second, preserves the last successful
snapshot on failure, resets selection on scope changes, and exposes source age.
Left/right select a session, PageUp/PageDown scroll, `r` rereads, `q`/Esc exits.
It is an additional inspection mode; no fixture composer or send is enabled.

For web transport, follow `../devwits-auth/docs/operator-observation.md` in the
sibling repository. It is opt-in and uses the existing authenticated mailbox.
No local settings, live mailbox, deployment or persistent service is changed
by installing this source.

## Wire contract and limits

`work-engine.operator-observation.v1` has a closed shape: `schema`, `scope_id`,
`source_id`, operator `label`, collector `observed_at_ms`, and selected `sessions`.
Each session has its exact native `session_id`, local read completion time and
an `available` or `unavailable` observation. Available observations carry the
host, optional redacted title and bounded source entries. Entries preserve their
original IDs, roles, tool, source timestamp, text, truncation and redaction count.
Unavailable codes are `source_unavailable` or `invalid_owner_observation`.

`source_id` is SHA-256 of the exact protected profile bytes. `scope_id` is
SHA-256 of UTF-8 compact JSON `[source_id,label,sorted_session_ids]`. These are
presentation scope identities, not grants or independent authenticity evidence.
All timestamps in milliseconds are positive integers within JavaScript's Date
range. A snapshot is limited to one MiB. Unknown fields/versions, duplicate IDs,
unsupported roles and invalid scope identities are refused. The Rust-produced
fixture is shared byte-for-byte with the web tests.

This is a polling snapshot and a bounded transcript tail, not a complete event
history or atomic cross-session read. Per-session times and original entry times
retain different meanings. A source observation more than 120 seconds old, or
with a future timestamp, receives a client freshness warning; this is a display
cue, not expiry or authority. Fetch success does not refresh the source time.
The separate [selected-task presentation](docs/selected-task.md) composes TO2 and
exact native session links without changing this bounded session contract.
Campaign joins, event subscription/replay, budget/closure views and control
operations remain follow-up work.

## Controlled qualification

```sh
# From rust/:
cargo test --locked -p operator-observation
cargo run --locked -p operator-observation --example fixture
# From the repository root:
python3 rust/crates/operator-observation/tests/checkpoint.py \
  /absolute/work-engine-observe /absolute/work-engine /absolute/evidence-directory
python3 rust/crates/terminal-ui/scripts/operator_observation_smoke.py \
  /absolute/work-engine-terminal-ui /absolute/evidence-directory
```

The checkpoint reads exact accepted Events source from Git (`d44b1bbf`, digest
`933eda1c…`), creates disposable transcripts and private profile/state, then
executes the actual collector. It checks a source update, missing session,
redaction, output privacy and pin mismatch. It invokes no provider or network.
Its exported snapshot can be used by both actual UI smoke tests. This controlled
qualification does not establish current live deployment or workflow UI-0/UI-1
acceptance.

For an explicitly selected current owner, append its absolute source path and
full expected SHA-256 to the checkpoint command. The checkpoint refuses a
digest mismatch before invocation; the production query boundary independently
enforces its own allowlist and byte digest. Both forms retain private fixture
inputs, the selected profile, first/second observations and the deliberate
pin-mismatch observation under the evidence directory. Use an owner-only
directory, and retain failures alongside passing results.

The terminal checkpoint uses one PTY process and resize redraws for text
assertions; it does not emulate a terminal. It checks automatic source refresh,
an outage, exit restoration and two rendered frame sizes. The sibling web smoke
test accepts the checkpoint's `controlled-observation.json` through
`OPERATOR_SNAPSHOT`; it uses actual browser components and mocked authentication
and mailbox reads.
