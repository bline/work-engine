# Slice checkpoint schema

Version 1 candidate requests contain repository, run and slice identity,
candidate attempt, baseline and optional parent checkpoint commit, accepted plan
version, scope revision, gate receipt digest, timestamp, and an explicit path
manifest. Each path names an `include` or `delete` action and one attribution:

- `task_owned`
- `user_owned_baseline`
- `pre_existing_overlap`
- `generated_dependency`
- `validation_dependency`

Candidate receipts bind the canonical request identity, baseline, commit, tree,
task-patch and attributed-manifest digests, gate/plan/scope identity, included path metadata, private ref,
and limitations. Accepted and stopped receipts retain that content identity and
record a distinct lifecycle commit and ref.

Review results used for acceptance must exactly match `checkpoint_commit_oid`,
`checkpoint_tree_oid`, `task_patch_digest`, `plan_version`, `scope_revision`, and
`gate_receipt_digest`. Acceptance uses compare-and-swap on the one private
accepted ref for the run and slice. A stopped ref never advances acceptance.

Private refs are local implementation state. This capability never moves user
branches, alters the real index or working tree, creates tags, or publishes.
