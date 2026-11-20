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
     returning. `lifecycle-service` does not link the Host crate, so `pi_host_join.rs` reaches
     the same composition by running the binary.
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

- Profile path: absolute, a regular file (not a symlink), at most 64 KiB;
  echoed as given in the record.
- Root precondition, refused as `root` before anything is written:
  - an absolute path in which every component, ancestors included, is
    plain (`[A-Za-z0-9._-]`, no `.` or `..` segment), because the Host
    cannot mount `:`, `,` or similar. A `+`, `@` or space anywhere in the
    native-root path therefore refuses every launch;
  - its parent must be an existing directory, not a symlink;
  - it is either absent (created with mode 0700) or an empty, owner-only
    directory of the caller that is not a symlink. After it is resolved,
    the leaf is checked again.

  `prepare` creates `<root>/inputs` exclusively, so a concurrent `prepare` on
  the same root refuses as `root`. A root is single use: any refusal after
  that leaves it populated, and a retry needs a new root.
- Snapshot id grammar: `[A-Za-z0-9._:-]{1,128}`.
- Exit 0 prints one JSON object on stdout (`schema`
  `work-engine.pi-host-composition-record.v1`)
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

- It checks the profile by exactly the rules a launch applies: a full
  `prepare` into a private scratch root under the system temporary
  directory, removed afterwards. So it agrees with `prepare` by
  construction, including the launch-time pins:
  - the runtime manifest and the Pi version;
  - the Claude Code binary on `claude-sdk`;
  - the Docker and Host binary digests;
  - the catalog reservation.

  It takes about as long as `prepare`. It hashes the Host binary and the
  runtime tree: on the real `claude-sdk` runtime that is about 0.6–1.4 s in
  a release build and 32–41 s in a debug build (RL re-review, N2), so a
  caller with a short budget runs a release Host. It never opens a
  credentials file.
  - Its only environment input is `TMPDIR`, the scratch location.
  - Exit codes and refusal classes are as for `prepare`, except that `root`
    or `io` from `describe` means its scratch environment failed, not that
    the profile is invalid.
  - The agreement covers the profile, not a launch root. The root rules
    above apply to the caller's own root at `prepare`. A caller that
    allocates roots under a native root checks that native root itself
    (RL's predicate).
  - The description and the dry run must hash the same profile bytes;
    otherwise `describe` refuses as `profile`.
- Exit 0 prints:
  - `{schema, profile_sha256, provider, model, wire, routes:[{id, endpoint, payer}], payers:{<payer>:[kinds]}}`;
  - `payers` lists the credential source kinds that can serve every route
    the payer pays (one payer holds one credential). An empty list means a
    loopback-only payer, which needs no credential.
  - A payer whose https routes need incompatible kinds refuses as
    `profile`, and so does a payer on both loopback and https routes (in any
    order).
- `profile_sha256` in `describe` is the same value as `profile.sha256` in
  the record.
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
- `job_secs` (at most the relay deadline, 3,600);
- `turn_deadline_secs`, with
  `job_secs ≥ turn_deadline_secs + 300` (the runner's thread start, 180 s,
  and close, 120 s);
- optional `pi_settings`: an object of at most 64 KiB, copied verbatim to
  the container's public Pi settings, so it must be public content.

It carries no credential.

Deadline:

- `deadline_unix_seconds` is `prepare`'s start time (wall clock) plus
  `job_secs`, and the Host enforces it per turn and per frame.
- It is in the record and in `composition.json`, not in `pi-host.json`:
  the runner's `PiHostConfig` is strict, and the runner reads it from the
  record.
- Spawn `serve` right after `prepare`.
- Credential kinds match the credentials-file `kind` tags exactly. The
  OpenRouter kinds are `openrouter-api-key-env` and
  `openrouter-api-key-file`; the kebab-case spelling `open-router-...`
  stays accepted.

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

## Step 1 evidence (offline, 2026-11-20)

- `work-engine-manual-host` lib: 192 passed at fa536eee, 194 at bc565d41,
  and 195 on the merge `eeaa0e7e`. The step 1 tests are:
  - `prepare_writes_one_admissible_relay_job`;
  - `prepare_refusals_carry_their_class`, which covers every refusal class;
  - `prepare_refuses_roots_it_cannot_own_or_mount`;
  - `describe_agrees_with_prepare_and_names_credential_kinds`;
  - `allowed_kinds_parse` in `pi_coding_https`;
  - `tests/pi_host_prepare_cli.rs` for the binary's exit codes and refusal
    shape.
- Binary exit codes: a refused `describe` exits 1, a refused `prepare`
  exits 1, usage exits 2.
- Real-binary smoke run (`records/dev-vertical-pi-host-realization-20261118/prepare_smoke.py`):
  - The profile used the real `claude-sdk` runtime (`runtime-claude-sdk`)
    and the offline relay observer over two loopback routes. `describe` and
    `prepare` both exited 0.
  - `serve --config <root>/serve-config.json` reported the prepared
    `job_sha256` at `pi-host/initialize`.
  - Driven by `serve.py`'s controller, both relay turns completed and were
    eligible. `failures` was empty, the serve record was `served`, and no
    container was left. The output is in
    `~/.local/state/work-engine/dev-vertical-pi-host-h2-live-prep-20261118/slice4-smoke-1/`.
  - Repeated at bc565d41 (`slice4-smoke-2/`) with the same result.

## RL consumer review of step 1 (6020) and remediation

Review: `research-lifecycle-tool-registry-20261116/038-step1-consumer-review/REVIEW-65c4de02.md`,
verdict "ready with required changes".

| Finding | Disposition |
| --- | --- |
| F1 (P1): `describe` accepted profiles every `prepare` refuses. | `describe` now runs a full `prepare` dry run. An agreement test covers a bad model, `max_tool_calls` 100,000, invalid `pi_settings` and `max_requests` 0. |
| F4: invalid `pi_settings` refused as `io` after root creation. | Checked with the profile, as `profile`. |
| F2: a root containing `:` or `,` refused as `admission` after populating. | Path grammar is checked first, as `root`. |
| F3: symlinked root; parents created at the umask. | Symlink refused; parent must exist; leaf created with mode 0700; an existing root must be the caller's, empty and owner-only. |
| F5: a loopback-plus-https payer was order-dependent. | Refused in either order. |
| F6: OpenRouter kind tags. | Explicit `openrouter-api-key-*` with the `open-router-*` alias; each offered kind is parse-tested. |
| F7: atomicity. | Single-use root documented; exclusive `inputs` makes a concurrent `prepare` refuse as `root`. |
| F8: profile path. | Absolute, regular, not a symlink, at most 64 KiB. |
| F10: `pi_settings` is public. | Documented. |
| F11: test gaps. | Added tests for root shapes, every class including `io`, an unknown field, a zero turn, a symlinked profile, mixed payers, kinds and agreement, and the binary's exit codes and refusal shape (`tests/pi_host_prepare_cli.rs`). |
| F12: plan text and field names. | Fixed. |
| Deadline. | Semantics stated. The profile requires `job_secs ≥ turn_deadline_secs + 300`. The deadline is read from the record. |

## RL consumer re-review of bc565d41 (6038) and follow-ups

Review: `research-lifecycle-tool-registry-20261116/038-step1-consumer-review/REREVIEW-bc565d41.md`.
F1–F12 were resolved. Step 1 was merged as `eeaa0e7e` before this
re-review, on the operator's instruction. The follow-ups land on branch
`pi-host-prepare-followups-20261120`.

| Finding | Disposition |
| --- | --- |
| N1 (P2): a huge `turn_deadline_secs` overflowed the margin sum and exited 101. | Compared against `job_secs.saturating_sub(300)`, refused as `profile`; tested at `u64::MAX` and `u64::MAX - 215`. |
| N2 (P2): debug `describe` takes 32–41 s on the real runtime. | Documented with the release timings; RL's install check uses a release Host. |
| N3 (P3): `describe` reads `TMPDIR`. | Documented as its only environment input; `root`/`io` from `describe` mean a scratch failure. |
| N4 (P3): agreement covers the profile, not the launch root. | Documented, including that the grammar applies to every ancestor; symlinked-parent message fixed. RL checks its native root itself. |
| N5 (P3): check-then-use gaps. | `describe` compares the dry run's `profile.sha256` with its own digest; the resolved leaf is re-checked owner-only. The `symlink_metadata`/`read` gap on the profile remains; it is unreachable for an owner-only profile. |
| N6 (P3): stale plan evidence and `Refusal` comment. | Refreshed. |
| Nit: `/n/./x` accepted. | An interior `.` segment now refuses as `root`. |
| CLI test relink race in the shared build directory. | Not changed. The test runs the binary Cargo built for it, and another worktree can relink that path mid-run (board 6025, point 3). A rerun settles it. |
