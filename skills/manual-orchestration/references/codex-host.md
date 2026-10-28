# Running on Codex

The owner selects Fast for manual supervisors and builders, and the standard
tier for Astra reviewers. These host mechanics specialize the generic child
launch and continuation wording in the role and context references.

- **Supervisors and builders:** `spawn_agent` with `fork_turns="none"` for a
  fresh assignment; `followup_task` for a retained continuation. Select model
  and effort from [model selection](../../../docs/model-selection.md). A child
  inherits the root's Fast tier ([observed probe](../../../metrics/probes/codex-manual-service-tier-20261028.json)).
- **Astra reviewers:** launch a separate top-level app-server thread through
  `<skill-dir>/scripts/role_host.py`, where `<skill-dir>` is this skill's
  directory. It has no runtime parent; explicitly pin its tier so shared
  defaults cannot select Fast:

  ```bash
  python3 <skill-dir>/scripts/role_host.py launch codex \
    --cwd <repo> --name <review-name> --prompt-file <packet> --parent <your-session-id> \
    --host-arg=-m --host-arg=gpt-6-astra \
    --host-arg=-c --host-arg='model_reasoning_effort="<selected-effort>"' \
    --host-arg=-c --host-arg='service_tier="default"'
  ```

- `--parent` records collection ownership only in the host ledger. Bind the
  full returned session ID; verify it with `role_host.py children <your-session-id>`
  and its rollout's `session_meta`. Spawned children instead use
  `<pinned context-status binary> agents --parent-thread <your id> --json`.
  Send the [identity binding](../SKILL.md#launching-a-child) after verification.
- Continue a retained reviewer with `role_host.py deliver codex <session-id>`
  (`--cwd`, `--message-file`) after the context check. Delivery refuses a busy
  thread; retain the pending binding or task until idle rather than replaying it.
  Collect through `wait_threads` when available or a verified completion route;
  `wait_agent` does not collect these top-level threads.
- Use `/fast` in the root for its supervisor/builder tree. The probed client
  also persisted `service_tier="fast"` into `config.toml`, so a separate thread
  alone did not preserve the reviewer's standard tier. The launch override
  above selects it explicitly. Record requested and observed tier in the
  existing assignment, including retained turns; the rollout's
  `thread_settings_applied` records Fast as `priority` and standard as `default`.
  An unobserved tier remains unknown. Include tier in the task-profile pilot's
  `--harness` value, for example `codex/role_host;tier=default`.

## Preserve root settings across continuation

Read this root's applied model, effort and service tier from its verified
rollout before preparing a handoff or an intentional same-session replacement:

```bash
python3 <skill-dir>/scripts/role_host.py settings codex <root-session-id> \
  --rollout <verified-absolute-rollout>
```

The reader uses the latest matching `thread_settings_applied` event and returns
its path, line, timestamp and event digest (SHA-256 of the UTF-8 line including
its line ending). Shared `config.toml`, an inherited
environment variable and the tier selected for a reviewer do not establish the
root's setting. This is recorded runtime configuration, not backend fulfillment
or billing; a later UI change may not yet have an applied record.

Carry the observation and any explicit user override in the continuation. Keep
the predecessor's model, effort and tier for the successor coordinator unless
the user's current selection or owning contract changes them. Preserve a known
user override; do not silently switch an observed root setting to a generic
role default. A genuine unresolved conflict remains with its decision owner.
The returned `successor_host_args`
make that request explicit for `role_host.py launch` or the rotation gate's
repeated `--host-arg` options; pass them as arguments, not executable shell text.
The [historical CLI 0.164.0 probe](../../../metrics/probes/codex-manual-service-tier-20261028.json)
and [live fixtures reporting runtime 0.165.0](../../../metrics/probes/outer-context-continuity-20261028.json)
support the same value mapping: observed `priority` maps to config
`service_tier="fast"`, and `default` maps to `service_tier="default"`.
The installed CLI version and a daemon's recorded runtime version are separate
observations; these cases do not qualify every version or ingress path.
An unknown tier or missing observation leaves preservation unestablished;
do not silently substitute a shared default or the reader's own settings.

Verify the successor's own applied record after launch/adoption and reconcile
a mismatch before accepting configuration-dependent work. Bind a recovery
observation to a record after the relevant launch/replacement boundary or to
the recovered turn. Reading the same pre-replacement event again establishes
only the last recorded configuration, not observed preservation. If no such
new observation exists, report preservation as unestablished. Same-session
compaction also needs this check rather than assumed preservation.
Separate reviewers retain their explicitly selected tier; carrying the root's
Fast setting does not override the reviewer contract above.

Configured campaigns retain their own launch and review-target contracts. Keep
`/fast` off in a campaign supervisor until that path supports separate reviewer
launch and binding; this manual adapter does not change campaign receipts.
