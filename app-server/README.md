# Work Engine App Server Runtime

This boundary is the provider-specific runtime realization for Codex App
Server. It translates authorized Work Engine delivery intent into App Server
protocol effects while keeping role identity, authority, and canonical workflow
state outside provider threads.

The first vertical provides:

- a JSON-lines stdio transport for App Server;
- initialization and fail-closed capability/version negotiation;
- pinned provider-runtime negotiation for model-context replacement, requiring
  explicit `token_budget` configuration and target-model invocation semantics;
- a durable logical-role-to-thread binding registry;
- a closed, digest-attributed runtime manifest that projects logical role
  templates, thread settings, and exact skill inputs;
- exact `SKILL.md` input resolution within configured roots;
- thread-scoped dynamic-tool declaration and dispatch;
- idempotent turn delivery through client message IDs;
- bounded consumption of stable `turn/completed` notifications, including
  terminal failure propagation and final agent-output extraction;
- a bounded provider-neutral lifecycle evidence collector fed by a
  version-pinned Codex notification normalizer for token usage and context
  transition signals;
- deterministic `observed-context-v1` construction with closed trust and
  instruction-applicability vocabulary, explicit completeness limits, and
  Ed25519 host attestation;
- closed `continuation-state-v1` candidate validation with exact source and
  authority references, deterministic candidate revisions, and explicit human
  interaction state;
- a storage-neutral lifecycle-ledger record contract with constrained event
  truth statuses and a verifiable SHA-256 predecessor chain;
- a revision-fenced checkpoint publisher that requires accepted semantic
  verification and exact authority revalidation, then atomically compares
  source, binding, authority, checkpoint-predecessor, and ledger-predecessor
  revisions before storing a checkpoint and its publication evidence;
- a revision-bound in-process transition lease that validates the published
  checkpoint and ledger fence, records readiness, and gates adapter turn,
  dynamic-tool, and runtime-binding ingress around one sterile retirement
  control-turn request; and
- an event-driven transition/reconciliation path that subscribes before
  retirement, admits only a pinned matching compaction signal as a wake-up,
  injects the exact checkpoint once, and keeps domain work fenced until an
  exact predecessor-linked reconciliation receipt is accepted;
- a provider-neutral semantic inference harness that verifies projected source
  bytes, runs distinct compiler and verifier capabilities, and derives the
  verification disposition in host code; and
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

Lifecycle evidence follows the same ownership boundary. The Codex notification
source validates supported provider payloads and projects them into immutable
token-usage or context-transition observations. The in-memory collector assigns
a run-local sequence, bounds retention, and produces per-thread snapshots.
Provider `contextCompaction` items remain explicitly `unclassified`; neither
their names nor their presence establish semantic context replacement. This
collector is an observation surface, not the future durable lifecycle ledger.

The observed-context projector accepts only attributed content references; it
does not copy raw content or claim access to the provider's literal effective
prompt. Every projection retains mandatory unknowns for the effective model
input, provider instructions, and hidden reasoning state. Its source revision
binds the normalized role, runtime binding, visible-item inventory, governing
sources, activated skill digests, lifecycle snapshot, expected next work,
completeness state, and explicit omissions. A domain-separated Ed25519
signature authenticates the projector component and build revision. Successful
verification proves construction integrity for those declared inputs, not
projection completeness, semantic sufficiency, or retirement readiness.

Continuation validation establishes the shape and content revision of a
compiler candidate; it does not accept or publish that candidate, revalidate
its authority references, or establish retirement readiness. The lifecycle
ledger helpers distinguish observations, attempts, failures, unresolved facts,
and accepted decisions and reject event/status combinations that would turn an
attempt into a success claim. Their hash chain detects mutation of retained
records but does not provide durable storage, authenticate the writer, or make
the in-memory chain authoritative by itself.

The semantic inference harness is currently proven against recorded bounded
fixtures. It verifies the observed-context signature, admits exactly the
projected source references, checks every supplied content digest, and supplies
the same bounded material to compiler and verifier calls. The compiler can emit
only semantic continuation fields; the host attaches subject, timestamps,
inference provenance, and the candidate revision. The verifier must report all
five required checks and cited blockers or uncertainty, while the host derives
`accepted`, `rejected`, or `unresolved`. Distinct capability objects and
inference identifiers prove separate calls, not provider, model, or reasoning-
context independence. No live inference adapter, checkpoint publication,
retirement authorization, or actuation is implemented by this harness.

Human-interaction evaluation is part of the same bounded verifier pass rather
than a third role. The compiler must keep semantic status separate from loading
disposition. Host checks prevent ambiguous meaning from being compiled away,
open meaning from being omitted, active governing meaning from becoming
reference-only, and a claimed compiled consequence from lacking its durable
reference. The verifier must return one source-bound closure/loading evaluation
for every interaction, and its aggregate interaction check must agree with
those evaluations. `escalate` remains unresolved even when the underlying
classification is supported.

Checkpoint publication remains a code-owned consequence after semantic
verification. The publisher stores the complete continuation state and its
verification only after a destination authority owner returns evidence covering
the exact referenced authority set. The injected store operation must compare
the observed source revision, runtime binding, authority revision, predecessor
checkpoint, and predecessor ledger entry in one atomic write. The included
in-memory store proves that contract deterministically but is neither protected
nor durable. A rejected or failed attempt does not produce a
`checkpoint_published` ledger claim. This boundary does not establish
retirement readiness, acquire a transition lease, deliver an actuator turn, or
invoke `new_context`.

The transition-lease foundation consumes the complete published checkpoint,
its publication ledger entry, and the exact post-publication fence. Lease
acquisition appends accepted readiness evidence. Before the adapter delivers a
retirement control turn it verifies the active lease, requires the negotiated
target-model context-replacement capability, admits only the deterministic
host directive, and records an attempted actuation request. A competing domain
turn or binding change revokes readiness before proceeding; a dynamic-tool
effect is denied and revokes readiness. Once actuator delivery has begun,
competing domain turns, tool effects, and binding changes remain outside the
retiring revision. Failed delivery records failure rather than actuation
success.

The included gate serializes these boundaries only inside one process. It is
not protected durable lease storage, does not observe provider-native or
external input or tool-effect paths, and a compaction notification alone cannot
establish that `new_context` produced the required successor semantics. The
runtime subscribes before actuator delivery and waits on notifications rather
than polling. A pinned, same-thread compaction signal for the exact retirement
turn records only an unresolved transition observation and unlocks one
checkpoint-bound rehydration turn. The predecessor context-window ID is part of
the immutable lease subject; the successor receipt must report that exact
predecessor, a distinct current window, the exact lease/checkpoint/thread/nonce,
and supported continuation claims with no hidden uncertainty. Only then does
the gate record accepted reconciliation and release domain work.

The fake-transport tests prove event ordering, checkpoint injection, receipt
validation, and fail-closed admission. They do not prove live model compliance,
authenticate the model-reported context-window identifiers, or replace the
still-required live strategic-planner reconciliation probe.

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

Model-context replacement is not represented as an App Server request in the
generated protocol lock. The adapter therefore gates the provider-neutral
`model_context_replacement` capability through a separate pinned provider
runtime profile. Selection requires an explicit host declaration that
`token_budget` is configured, fixes the mechanism to model-invoked
`new_context`, and requires observed transition evidence. This declaration is
configuration evidence only: it does not prove that the tool was exposed or
that a transition occurred. Provider capabilities cannot be widened after
adapter initialization, and the selected provider profile participates in the
retained thread's environment fingerprint.

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
optional local integration suite requires the pinned `codex` executable. It
proves initialization, model visibility of a runtime-random request-context
value through a temporary manifest role, same-thread context-window replacement
with normalized lifecycle evidence, and an exact request-bound handoff through
the production strategic-planner role:

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
