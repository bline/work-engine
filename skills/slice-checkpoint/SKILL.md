---
name: slice-checkpoint
description: Create immutable, attributed Git checkpoints without changing the user's branch, index, or working tree.
---

# Slice Checkpoint

This adapter owns bounded Git mechanics, not slice acceptance. It consumes an
explicit versioned manifest, constructs a tree with a temporary index, writes
immutable commit objects, and updates only `refs/work-engine/checkpoints/`.

The campaign supervisor owns when candidates are requested, whether review and
gate evidence matches a candidate, and whether an accepted or stopped lifecycle
transition is authorized. Never move a branch, use the repository's real index,
create tags, push refs, or infer additional workspace dependencies.

Every included path has an attribution classification. Reject paths outside the
repository, ignored paths, sensitive-looking paths, unsupported file types, and
undeclared dirty content. Exact retries are idempotent; a conflicting private-ref
update fails closed.

The schema and lifecycle contract are documented in
[`references/checkpoint-schema.md`](references/checkpoint-schema.md).
