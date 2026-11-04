# Running the bounded offline fixture

From the repository root:

```sh
CARGO_TARGET_DIR=/tmp/session-delivery-build cargo test --offline --locked --manifest-path rust/Cargo.toml -p session-delivery
```

The integration tests create disposable Git repositories, publish through the
actual board CLI, invoke the actual reader, commit a Rust inbox/cursor and spawn
isolated receiver processes. They include offline/busy/lost-ACK, crash/restart,
exact and conflicting replay, expiry, binding/ACK mismatch and backlog holds.
R1 regressions additionally reset/repopulate the same disposable repository to
the same/beyond cursor with changed IDs/bytes, remove retained history, and
exercise appended/reserved rows and non-message revisions across fresh CLI
processes. The source adapter receives both saved cursor and revision; missing
or incompatible history holds without adopting a new baseline.
No user board, subscription or receiver is enrolled.

`session-delivery schemas` prints the generated DTO schemas.
`schema-v1.json` is the matching checked-in generated artifact.
To inspect a standalone fixture, write a private mode0600 config in a private
mode0700 directory. `fixture_config` in the schema lists its required fields.
`fixture_only` must be true and `schema` must be
`session-delivery-fixture-config-v1`. The source paths select the existing
`skills/durable-state/scripts/codex_chatboard.py` and this crate's
`scripts/board_source.py`; repository identity is the canonical Git common dir.
Each target is a fixed Binding with `fixture_mode` equal to `observe`, `busy`,
`lost_ack` or `offline`; those are fixture behaviors, never production profiles.

The CLI takes `--config /absolute/private/config.json` followed by:

- `rpc`: one bounded JSON request on stdin; methods `subscribe`, `status`, `ack`.
- `observe`: one service-owned board observation committed to the store.
- `dispatch`: one oldest eligible delivery per recipient; collect each fixture child.
- `run N`: 1–100 foreground observation/dispatch cycles,50ms between cycles.
- `fixture-receiver NAME`: one fixture-only envelope on stdin, then exit.

Every RPC request uses `schema:session-delivery-rpc-v1`, a trusted configured
`principal`, and allowlisted `recipient`. Response discriminator is
`protocol:session-delivery-rpc-v1`, `schema_version:1`, with `ok` and result/error.
Subscription backfills retained inbox events matching the exact address.
Status accepts optional `after` from its prior `next_after`. ACK requires the
versioned observation DTO; no executable/path parameter exists in public tools.
The facade alone uses the descriptor-bound RPC-only `--config-fd` variant.

`SESSION_DELIVERY_FIXTURE_CRASH` is a deterministic fixture fault injector with
points `after_inbox_before_cursor`, `before_commit`, `after_commit`,
`after_prepare`, `before_send`, `after_send`, `before_ack_commit`,
`after_transport_ack`, `after_ack`. Exit86 is deliberate; prepared/unknown state
is retained. It is never a real recipient or delivery qualification mechanism.

No daemon is installed. `run` is a bounded foreground service, suitable for this
fixture qualification. A production deployment, pressure-producer enrollment,
Pi receiver and long-lived service host remain separate work owned by Root.
