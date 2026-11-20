# Pi Host production composition (dev-vertical slice 4)

2026-11-20 · Operator-approved direction (option A below). The steps that touch
the research-lifecycle owner's crates need that owner's agreement.
Repository baseline: `d698c78d`.

## Objective

Let the task host run research on the Pi Host lifecycle relay
(`work-engine.pi-lifecycle-hosted.v1`, qualified live by H2), without
test-only composition. Today only `lifecycle-service`
`tests/pi_host_join.rs` (`prepare_host_job`, `config`) writes a Host job, and
the task host refuses the route.

## Facts that shape the design

- The task host's startup (`HostStartup`) is an installed, static file
  carrying the lifecycle `ServiceConfig`. Production grants are composed by
  the installer and do not expire (`selected_startup.rs`,
  `research_installation.rs`). Grants are not a per-launch problem.
- A Host job is per launch:
  - it has an absolute deadline (`deadline_unix_seconds`) and needs a fresh
    state directory;
  - its digest is bound into the session binary identity
    (`operations/pi_host.rs` `binary_identity`) and into every capture
    (`identities.job_sha256`);
  - the runtime admission (`work-engine.pi-coding-runtime-admission.v1`)
    binds the job id, the Host binary, model, provider, catalog, budget and
    route-table digest.
- The runner loads `<native root>/pi-host.json` at preflight
  (`validate_native_config`) and at launch (`prepare_launch`), checks the
  serve config and job by digest, and refuses non-https routes outside
  `controlled-proof`.
- The task host refuses `PiHostV1` at startup validation, because
  `configured_base_url` has no endpoint for it (`work-engine-task-host`
  `lib.rs:1810`). It also refuses it at the install check, which admits only
  the Codex ChatGPT route with its access-source isolation
  (`research_installation.rs:1355`).

## Design (option A)

1. **The Host composes its own job.** This is `work-engine-manual-host`,
   owned by dev-vertical.
   - New subcommand
     `work-engine-pi-host prepare --profile <composition profile> --root <fresh root> --snapshot-id <id>`.
   - The composition profile (`work-engine.pi-host-composition.v1`) is
     installed, and its digest is pinned by the operator through the task
     host's startup. It holds:
     - runtime root and manifest;
     - provider and model;
     - route table and attempts;
     - requests, budget, context bytes, tool calls;
     - turn and job durations;
     - public Pi settings.
   - `prepare` writes, under a root that must not already hold a job:
     - the inputs (shim and stream adapter from the Host's own constants, so
       they cannot drift);
     - the runtime admission, derived from the profile for this job id;
     - `job.json`, `serve-config.json` and `pi-host.json`;
     - `composition.json`, which records the profile digest and the
       derivation.
   - It validates the job by the Host's own admission rules before
     returning. `pi_host_join.rs` can then use the same library function
     instead of its ad hoc JSON.
2. **The runner calls `prepare` at launch.** This is `lifecycle-service`,
   owned by research-lifecycle.
   - `NativeConfig` names the composition profile by path and digest.
   - At launch, a per-attempt root under the native root is prepared with
     the Host binary.
   - Preflight validates the profile (by digest), not a pre-written job.
   - The existing digest, route and https checks then run on what `prepare`
     wrote, and the runner checks `composition.json`'s profile digest
     against the pin.
   - `lifecycle-service` keeps spawning the Host binary and does not link
     the Host crate.
3. **The task host accepts the route.** This is `work-engine-task-host`,
   owned by research-lifecycle.
   - Startup validation gets a `PiHostV1` branch: the profile pin instead of
     a base URL.
   - The install check gets a Pi Host credential-isolation predicate,
     equivalent to the ChatGPT access-source isolation:
     - the hosted credentials file is owner-only and regular;
     - it lies outside the repository, the state directories and the native
       root;
     - its payer kinds are allowed by the profile's routes.
4. **An offline end-to-end run through the real task host launch.** This
   uses the loopback observer under `controlled-proof`, as Q does.

Rejected:

- (B) `lifecycle-service` writes the job JSON. That duplicates the Host's job
  schema in another owner's crate.
- (C) `serve` composes internally. The runner would not know the job digest
  before the spawn, and that digest is bound into the session identity.

## Step 1 contract (RL 6015, 6016)

`work-engine-pi-host prepare --profile <composition.json> --root <root> --snapshot-id <id>`

- Root precondition: absolute; absent (created) or an empty directory. It is
  set to mode 0700, and anything else refuses. One job per root.
- Snapshot id grammar: `[A-Za-z0-9._:-]{1,128}`.
- Exit 0 prints one JSON object on stdout
  (`work-engine.pi-host-composition-record.v1`):
  - `profile` `{path, sha256}`, `root`, `job_id`;
  - `job_sha256`, `serve_config_sha256`, `pi_host_json_sha256`,
    `host_sha256`;
  - `snapshot_id`, `prepared_unix_seconds`, `deadline_unix_seconds`;
  - `routes`, `composition_sha256`.

  The same object, without `composition_sha256`, is written as
  `<root>/composition.json`.
- Exit 1 prints `{"status":"refused","class","error"}` on stderr. The
  classes are:
  - `profile`: the profile is invalid by the Host's rules;
  - `snapshot-id`;
  - `root`: not absolute, not fresh, or not creatable;
  - `admission`: the composed job fails the Host's fresh admission, for
    example a runtime pin or the context bound;
  - `io`.
- Exit 2: usage.
- Written under the root:
  - `inputs/`;
  - `pi-host-state/` (empty);
  - `job.json`, `serve-config.json`, `pi-host.json`, `composition.json`.

`work-engine-pi-host profile describe --profile <composition.json>`

- It checks the profile by the same rules: schema, durations, provider
  catalog, route table, each endpoint serving the provider's wire family,
  and credential kinds.
- It never opens a credentials file. Exit and refusal classes are as above.
- Exit 0 prints:
  - `{schema, profile_sha256, provider, model, wire, routes:[{id, endpoint, payer}], payers:{<payer>:[kinds]}}`;
  - `payers` lists the credential source kinds that can serve every route
    the payer pays (one payer holds one credential). An empty list means a
    loopback-only payer, which needs no credential.
  - A payer whose routes need incompatible kinds refuses as `profile`.
- Kinds per endpoint (`https::allowed_kinds`):

  | Endpoint | Kinds |
  | --- | --- |
  | `https:chatgpt-codex` | `chatgpt-pi-auth` |
  | `https:openai-responses` | `api-key-env`, `api-key-file` |
  | `https:anthropic-oauth` | `claude-code-auth` |
  | `https:anthropic-api` | `anthropic-api-key-env`, `anthropic-api-key-file` |
  | `https:openrouter` | `openrouter-api-key-env`, `openrouter-api-key-file` |
  | `loopback:*` | none |

The profile (`work-engine.pi-host-composition.v1`, strict fields):

- `runtime {root, manifest {path, sha256}}`, `docker`;
- `provider`, `model`, `hosted {routes, max_attempts_per_request}`;
- `max_requests`, `budget_microusd`, `max_context_bytes`,
  `max_tool_calls`;
- `turn_deadline_secs`, `job_secs` (at most the relay deadline, 3,600);
- optional `pi_settings` (object).

It carries no credential.

Host environment (RL 6015 (b)):

- `serve` reads only `WORK_ENGINE_HOSTED_CREDENTIALS`, plus the variable
  that a `*-env` credential source names. Production should prefer the
  `*-file` kinds, so nothing else needs to pass the allowlist.
- Docker is spawned with a cleared environment (`PATH=/usr/bin:/bin`,
  `HOME=/nonexistent`; `pi_docker.rs:610`).
- TLS uses the platform verifier, so system certificates are used unless
  `SSL_CERT_FILE` or `SSL_CERT_DIR` are set.
- `prepare` and `profile describe` read no environment.

Account identifier on non-Messages wires (RL 6015 (e)): the Host verifies
absence only on the Messages wire. Elsewhere it reports
`contains_account_identifier: true`, meaning not verified. With the consumer
refusing on any wire, Responses and Completions (OpenRouter) relay captures
are refused until the Host has an absence check for those wires. That check
is a separate Host item, not this slice.

## Invariants kept

- One fresh state directory per job, and the Host admission unchanged.
- Routes and payers only from the pinned profile, with https only outside
  `controlled-proof`.
- No credential in any composed file. The credentials file is named by the
  environment, as today.
- The protocol and `PROTOCOL_DIGEST` are unchanged.

## Open points

- **Admission authority.** The admission is derived per launch from the
  operator-pinned profile, not written by the operator per job. This keeps
  the K-c position (`docs/dev-vertical-open-items.md`): acceptable for
  single-operator local use.
- **Root allocation.** One root per attempt id under the native root. Also
  to settle: cleanup of finished roots.
- **Preflight.** Preflight runs before any per-attempt root exists, so it
  validates the profile and the Host binary, not a job.
