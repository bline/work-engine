# Idea: Executable-Generation Maintenance Rollover

**Status:** Post-migration proposal input with an emergency helper implemented

Repeated migration repairs change the executable environment fingerprint and
therefore require a fresh generation-state root. The semantic context should
survive that maintenance boundary, while the predecessor generation identity,
role bindings, attachments, and delivery state must not be copied forward.

The immediate deterministic helper is:

```bash
npm run app-server:prepare-generation -- \
  --from "$WE_MIGRATION_GENERATION_STATE" \
  --to "$HOME/.local/state/work-engine/app-server-migration-generation-live-vNEXT" \
  --socket /tmp/work-engine-migration.sock
```

It refuses a live proxy socket or an existing target, creates a private target,
copies only `semantic-context.sqlite3`, verifies the source stayed stable and
the destination digest matches, and emits a machine-readable receipt. It does
not activate the generation, rewrite shell variables, copy bindings, or start
the proxy.

Post-migration proposal work should make this a first-class maintenance
operation owned by executable-generation lifecycle rather than a shell-level
deployment convention. That operation should stage, attest, activate, and
surface rollback/continuation evidence while preserving ProviderTurnPort,
HarnessRuntimePort, and OperatorProjection as architectural seams.

## Additional reload evidence: 2026-09-08

An in-process reload reached `active_unexercised` and retired its predecessor,
but the successor's first `turn/start` failed with `App Server adapter is not
initialized`. The stable client had initialized the predecessor before the
candidate existed. Candidate validation and activation did not carry that
negotiation state into the fresh internal adapter, so durable generation state
claimed activation while the role environment was not exercisable.

The emergency repair makes the stable bootstrap retain the bounded App Server
initialization exchange and project it into each later role-generation worker.
The successor adopts the already-negotiated response before activation; it does
not initialize the provider process a second time. A regression test now runs a
role turn, replaces the generation in-process, and runs another role turn
through the successor using the single original client initialization.

The first-class maintenance operation should generalize this requirement:
`active_unexercised` must mean that every stable-session prerequisite needed by
the candidate has been projected and validated, not merely that its worker
process passed static generation validation. First exercise should remain a
separate receipt, but it must not be the first point at which missing bootstrap
state can be discovered.
