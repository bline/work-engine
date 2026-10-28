# Slice checkpoints and the completion-commit offer

`<skill-dir>` is the slice-supervisor skill's directory.

When the campaign uses Git-backed slice checkpoints, request a candidate only
after the deterministic gate is ready for review. Give the checkpoint adapter
the declared baseline, exact attributed manifest, plan version, scope revision,
and gate-receipt digest. The adapter owns isolated Git mechanics; the supervisor
owns the lifecycle consequence. Pin review to the returned candidate commit,
tree, task-patch, plan, scope, and gate identities. A material repair requires a
new candidate and preserves the old candidate and findings.

Before acceptance, validate the review result with
`scripts/checkpoint_lifecycle.py`. Only an exact binding may create the private
accepted lifecycle checkpoint. Pass that accepted receipt to terminal
finalization so the durable audit receipt and next-slice recovery name the same
identity. Stopped or failed checkpoint refs preserve recovery state but never
advance acceptance. Checkpoint mechanics may update only private local
`refs/work-engine/checkpoints/`; they never move the user's branch, alter the
real index or working tree, create tags, or publish refs.

After private acceptance, apply `engine_config.slice_completion_commit.prompt`.
When disabled, suppress the offer and continue from the private checkpoint.
When enabled, prefer sending the accepted checkpoint and exact manifest back to
the completing builder before releasing useful context. Require its compact
schema-version-2 proposal to bind both and truthfully name its producer and
durable supporting evidence. If that context is unavailable, a replacement may
reconstruct only from the accepted checkpoint plus durable attributed manifest
and compact terminal semantic consequences. Insufficient evidence must produce
refusal or explicit uncertainty, never an unsupported equivalence claim. The
producer route and provider/model identity do not authorize the proposal; its
content and immutable bindings remain decisive. Then ask the user to create or
decline the ordinary commit. Only an explicit per-slice create decision authorizes
`$slice-completion-commit`; an open, declined, unavailable, or refused offer
never blocks checkpoint-based continuation. The supervisor owns one durable
live offer outside model context, terminal audit history, and the ordinary
worktree. The current implementation uses a private Work Engine ref as storage;
that mechanism is an affordance rather than invariant product structure. The
sibling adapter owns real Git preflight, mutation, and read-only reconciliation
after uncertain publication. Because resolving a stale open offer could
misreport an already-published commit, every terminal transition reconciles Git
first and fails closed when publication state is ambiguous. Persist terminal
offer transitions through the live owner, and retain the accepted private checkpoint—not current branch
HEAD—as the authoritative next-slice baseline.
