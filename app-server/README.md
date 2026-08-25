# Work Engine App Server Runtime

This boundary is the provider-specific runtime realization for Codex App
Server. It translates authorized Work Engine delivery intent into App Server
protocol effects while keeping role identity, authority, and canonical workflow
state outside provider threads.

The first vertical provides:

- a JSON-lines stdio transport for App Server;
- initialization and fail-closed capability/version negotiation;
- a durable logical-role-to-thread binding registry;
- a closed, digest-attributed runtime manifest that projects logical role
  templates, thread settings, and exact skill inputs;
- exact `SKILL.md` input resolution within configured roots;
- thread-scoped dynamic-tool declaration and dispatch; and
- idempotent turn delivery through client message IDs;
- bounded consumption of stable `turn/completed` notifications, including
  terminal failure propagation and final agent-output extraction; and
- deterministic parsing and request-binding validation of the strategic
  planner's version-1 YAML handoff.

Request-bound Work Engine context is compiled into a deterministic text input
item carried by the pinned `TurnStartParams.input` field. It is included in the
idempotency fingerprint. The runtime does not send an unpinned
`additionalContext` extension.

`threadId` is always a replaceable runtime binding. It is never used as the
logical role identity or as canonical workflow state.

Completion results are runtime observations, not canonical workflow state. A
live adapter retains a bounded result cache so fast notifications cannot race
their consumer. A replay whose completion is no longer retained fails with an
explicit reconciliation requirement instead of waiting for an event that has
already occurred.

## Protocol bindings

[`protocol-bindings.lock.json`](protocol-bindings.lock.json) pins the Codex CLI
version, generator mode, and the protocol roots consumed by this boundary. Run:

```bash
npm run generate:app-server-protocol
```

The generator fails if the installed CLI version differs from the lock. It
uses the CLI's own TypeScript generator and retains the complete dependency
closure of the selected protocol types under `generated/`.

The current foundation deliberately uses only stable APIs from Codex CLI
`0.149.1`. Experimental APIs must be added to the lock and capability profile
explicitly; their presence in a newer App Server does not silently enable them.

## Runtime manifest

[`runtime-manifest.yaml`](runtime-manifest.yaml) owns App Server runtime
composition for the currently ported roles. A role entry declares only variant
runtime structure:

```yaml
schema_version: 1
manifest_id: work-engine.app-server
roles:
  example-role:
    contract: ../skills/example-skill/SKILL.md
    developer_instructions: Keep the role inside its contract.
    thread_options:
      cwd: ..
      approval_policy: never
      sandbox: read-only
    skills:
      - name: example-skill
        path: ../skills/example-skill/SKILL.md
```

The loader rejects unknown fields, resolves paths relative to the manifest,
requires the named role contract to be present among the exact injected skill
inputs, binds the source bytes with SHA-256, and projects any named role plus
an instance ID into the adapter's role and skill inputs.
`ManifestRoleRuntime` provides the generic delivery boundary used to assemble
arbitrary roles for tests and future ports.

Each projected role also carries a deterministic environment revision derived
from its own contract, instructions, thread settings, and skill composition.
The binding registry refuses to resume an existing logical role thread when
that thread-scoped environment changes; the caller must perform an explicit
binding replacement. Changes to unrelated roles do not invalidate the role.

The runtime manifest does not own role authority, workflow state, or the
meaning of a skill. Those remain with their canonical contracts and durable
owners. Likewise, generated files under `docs/agent-environment-views/` remain
analysis projections and are not runtime configuration inputs.

## Tests

```bash
npm run test:app-server
```

The default suite uses a fake transport and temporary binding stores. The
optional local integration probe requires the pinned `codex` executable:

```bash
npm run test:app-server:integration
```

## Role verticals

The first role vertical is [`roles/strategic-planner.mjs`](roles/strategic-planner.mjs).
Its runtime composition comes from the checked-in manifest. The wrapper binds
each logical planning stream to a durable thread, supplies named evidence
cutoffs and canonical references, and validates the result against the exact
request. Its output is accepted only after the terminal turn succeeds and the
YAML handoff matches the requested objective, evidence cutoff, continuity
state, and version-1 shape. The handoff remains advisory planning state;
neither the wrapper nor its thread mutates roadmap or campaign authority.

## Planning documents

- [`docs/semantic-context-lifecycle-manager.md`](docs/semantic-context-lifecycle-manager.md)
  plans a shared code-owned lifecycle service that compiles, verifies, retires,
  and rehydrates model context across Work Engine root roles.
- [`docs/codex-agents.md`](docs/codex-agents.md) records the shared App Server
  daemon and operator-console behavior relevant to the new runtime topology.
