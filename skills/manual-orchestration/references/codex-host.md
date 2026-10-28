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

Configured campaigns retain their own launch and review-target contracts. Keep
`/fast` off in a campaign supervisor until that path supports separate reviewer
launch and binding; this manual adapter does not change campaign receipts.
