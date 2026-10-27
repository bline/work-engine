---
name: slice-completion-commit
description: Safely create an explicitly approved user-visible Git commit from an accepted slice proposal.
---

# Slice Completion Commit

Own bounded real-branch Git mechanics after slice acceptance. The supervisor owns configuration, prompting, user authorization, and lifecycle consequences; the completing builder owns the semantic proposal. This adapter never creates authority.

Accept only a proposal bound to the accepted checkpoint tree and exact task-owned paths. A checkpoint may contain privately accepted changes that were never authorized for public history, so publishing its tree without comparing the complete parent-to-tree delta with the approval artifact can expose unapproved content. Before `create`, require the complete proposed commit delta to match the approved proposal paths, an attached branch, the approved branch and parent `HEAD`, the selected mode's index/worktree preconditions. Historical strict requests require a real index matching `HEAD` and a complete tracked/untracked projection equal to the accepted tree. Preserve the repository's configured user identity. Because signed commits and repository-specific hook behavior are deferred, safely refuse when commit signing is required or an active commit hook exists.

Create and verify the commit object before publishing it with an atomic expected-parent branch update. After publication, verify branch attachment and the selected mode's postconditions (global clean porcelain for strict requests). Never push, tag, infer scope, bypass hooks/signing, or modify private checkpoint refs.

The durable terminal consumer treats a `created` receipt as authoritative Git
evidence. A fabricated receipt would otherwise let terminal history claim a
mutation that never occurred. Expose read-only receipt verification that
re-establishes the repository, proposal binding, commit, parent, tree, message,
publication target, and resulting state before finalization persists that
claim. Later commits on the same attached branch do not invalidate historical
publication: the recorded commit must either remain the branch tip or be its
ancestor. A missing branch, different attached branch, or divergent tip fails
closed. Non-created states retain only their actual evidence boundary.

The supervisor owns any durable live completion offer separately from terminal
audit history. When publication may have succeeded before that offer was
finalized, expose read-only reconciliation from the durable request. Never
replay `create` to discover whether publication occurred; replay can turn an
already-published commit into a false refusal.

An explicitly declared operational path, such as an in-repository metrics
destination, is preservation state rather than commit authority. Create may
exclude such declared paths from accepted-tree and cleanliness comparison only
when they are safe relative paths disjoint from the approved proposal. They
remain uncommitted and untouched.

For an explicitly approved `preservation.mode: preserve_unrelated_v1` request,
read [the versioned contract](references/receipt-schema.md). The complete request
binds the full parent-to-accepted-tree delta, accepted lifecycle receipt,
source/index manifest, real-index digest and a fresh external attempt directory.
Unrelated ordinary staged, unstaged and untracked work is preservation state,
never publication authority. This mode does not permit `operational_paths`
exclusions or make an old non-main-relative accepted tree publishable. Coordinate
exact source owners before publication: an index lock does not fence worktree or
configuration writers. Unsupported flags, filters and file kinds refuse rather
than normalizing or discarding user state.

Branch CAS and index installation are separate effects. The adapter durably
records the prepared commit and original/replacement index before CAS, changes
only approved index entries and never writes working files. An interrupted
post-CAS operation can be `created` with postconditions `unestablished`; it is
not verified completion. Reconcile read-only using that exact durable attempt.
Never replay create, reset the branch or repair index/locks automatically.
Missing evidence cannot be replaced by observing a clean checkout later.
