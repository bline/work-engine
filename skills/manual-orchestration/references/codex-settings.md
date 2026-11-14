# Preserving Codex settings

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: preparing or recovering a same-session replacement, or preparing/adopting a successor root.

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
the predecessor's model, effort and tier for the successor supervisor unless
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
