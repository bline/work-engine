# Manual orchestration command inventory

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: selecting or invoking a helper script; read the operation contract before relying on its command.

## Scripts

- `scripts/provision_manual_host.sh [repo]` - build and pin the optional Rust manual host with its compatible adapter bundle; no global activation.
- `scripts/provision_context_status.sh [repo]` - build and pin the context-status binary outside the shared target; prints path and SHA-256.
- `scripts/child_context.py` - `check` one Codex session, `tree` a parent's children, or `claude` a Claude Code subagent transcript against the trigger and observed compaction band; writes a record to retain.
- `scripts/role_host.py` - host a role or child as a Claude `--bg`, Codex app-server or Pi session: `launch` (with `--parent` to record it in the ledger), `status`, `deliver`, `retire`, `children`, `whoami`. Pi launches name a lane with `--auth api|subscription` and may set `--service-tier` on the api lane; `--container` runs each turn without network behind a host egress proxy (`scripts/pi_container.py`); see [Pi host](pi-host.md). Codex also supports `rename`, `archive`, and `unarchive` without a model turn, plus `settings --rollout` to observe applied model/effort/tier for continuation. Detects and stops a Claude copy-on-resume fork.
- `scripts/auto_continue.py` - bounded foreground same-SID compaction and attributed recovery; use under [same-session continuity](same-session-context.md#foreground-automatic-continuation-on-the-codex-app-server).
- `scripts/root_rotation.py` - root transfer gate: `prepare` publishes continuation lineage and stops at `awaiting_authority`; `commit` or `watch` checks that the handoff is still available, launches the successor, waits for its typed `adopt` and retires or keeps the predecessor. `--title` provides a readable task name. See [context boundary](context-boundary.md#roots).
- `scripts/custody.py` - `run` a build or test, or a plan of them, recording PIDs, wait status, output and input digests and leftover processes; `freeze` a tree outside the repository into a manifest, archive and baseline diff. See [worker](worker.md#builds-tests-and-frozen-subjects).
- `scripts/scope_budget.py` - original scope, additions, the objective's sessions and its cost limits in the assignment record; `check` reports spend and time and says whether to reconsider or stop. See [supervisor](supervisor.md#scope-and-budget).
- `scripts/handoff_validator.py` - checks a root handoff's referents (paths, path-bound digests, labelled git refs, placeholders): 0 pass, 21 defect, 32 not established.
