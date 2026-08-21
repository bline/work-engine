# Git-Backed Slice Checkpoints

## Status

Idea / architecture proposal

This proposal develops the workspace-snapshot consequence described in:

- `durable-review-queue-and-scope-coordination.md`;
- `persistent-agent-state.md`;
- `persistent-agent-state-and-runtime-introspection.md`; and
- `persistent-strategic-planner.md`.

It focuses on one problem:

> **How can every slice, review attempt, repair, recovery, and downstream slice
> identify the exact repository state it is reasoning about without mutating or
> misattributing the user's Git state?**

---

## Motivation

A commit or tag created only after a slice is accepted is useful for campaign
continuation, but it does not preserve the code that an earlier review
examined.

That earlier state is often the most valuable one:

- it may contain a real defect found by review;
- it establishes whether the review result was valid for the inspected code;
- it allows a later reviewer to reproduce or challenge the finding;
- it supplies a pre-repair case for reviewer evaluation; and
- it distinguishes a stale review from a valid review of unchanged state.

Today, reconstructing that state may require correlating Git history, dirty
worktree observations, Codex sessions, Claude outputs, patch operations, and
terminal receipts. Reconstruction is useful forensic work, but it should not
be the normal source of truth.

The workflow can create immutable slice checkpoints when later reproduction,
review binding, or continuation requires durable content identity. The required
consequence is an immutable, attributable subject; Git checkpoints are the
current concrete route for providing it.

---

## Core consequence

Every consequential review and slice transition binds to an immutable content
identity.

Conceptually:

```text
accepted baseline
      ↓
builder mutation
      ↓
deterministic gate
      ↓
candidate checkpoint 0
      ↓
review 0
      ↓
repair, when required
      ↓
candidate checkpoint 1
      ↓
closure review
      ↓
accepted checkpoint
      ↓
next-slice baseline
```

If no repair is required, the accepted checkpoint may reference the same Git
tree as the reviewed candidate. It remains a separate lifecycle transition.

> **A review result establishes claims about one checkpoint. Acceptance and
> continuation must not silently detach those claims from that identity.**

---

## Checkpoint is broader than “commit everything”

The workflow should distinguish several concepts that ordinary Git usage often
combines:

### Baseline revision

The commit or accepted checkpoint from which the slice began.

### Workspace snapshot

The immutable content needed to reproduce the slice at a consequential
boundary. It may include declared pre-existing dirty dependencies or baseline
overlaps without claiming that the slice authored them.

### Task delta

The content change attributable to the slice's accepted mutation scope.

### Checkpoint commit

A Git commit object used as an immutable container for an allowed snapshot. It
does not by itself imply acceptance, authorship of every included byte, or a
move of the user's current branch.

### Checkpoint reference

A durable Git reference that keeps the checkpoint reachable and gives the
runtime a stable lookup identity.

### Human-visible tag

An optional annotated tag for an accepted or otherwise important milestone.
Tags are not required for every candidate or repair attempt.

These distinctions prevent “the code exists in a commit” from being confused
with “the slice owns every byte” or “the user asked to advance this branch.”

---

## Recommended Git representation

The initial implementation should prefer Git-native immutable objects while
leaving the user's branch, index, and working tree unchanged.

A checkpoint adapter can use a temporary index or isolated worktree to:

1. start from the declared baseline tree;
2. overlay only allowed task-owned paths and declared baseline dependencies;
3. write a Git tree;
4. create a commit object with structured checkpoint metadata; and
5. update a private Work Engine reference atomically.

Illustrative references:

```text
refs/work-engine/checkpoints/<run-id>/slice-<n>/candidate-<attempt>
refs/work-engine/checkpoints/<run-id>/slice-<n>/accepted
refs/work-engine/checkpoints/<run-id>/slice-<n>/stopped
```

The exact namespace is an implementation choice. The required properties are:

- candidate attempts remain distinguishable;
- accepted continuation has one authoritative identity;
- stopped or failed work cannot be mistaken for accepted work;
- old checkpoints remain reachable while referenced by review, recovery, or
  evaluation state; and
- reference updates are atomic and idempotent.

The review should bind to the tree object identity as well as the checkpoint
commit identity. The tree identifies content; the commit also identifies
parentage and checkpoint metadata.

---

## Safe inclusion policy

A checkpoint must not indiscriminately commit the entire dirty workspace.

The default inclusion set should be:

```text
accepted baseline tree
+ task-owned paths
+ explicitly declared baseline overlaps
+ explicitly declared reproducibility dependencies
- ignored, sensitive, and unrelated untracked paths
```

Each included path should retain an attribution classification such as:

```text
task_owned
user_owned_baseline
pre_existing_overlap
generated_dependency
validation_dependency
```

If an undeclared dirty path materially affects the build or test result, the
checkpoint cannot claim full reproducibility. The supervisor must either add
that dependency through an authorized scope/baseline transition or record a
limitation and refuse any review or acceptance claim that requires an exact
snapshot.

Ignored files and likely secrets are excluded by default even when present in
the workspace. Including them requires an explicit policy and authority
decision.

---

## Role and authority boundaries

### Builder

The builder proposes task-owned files, baseline overlaps, and reproducibility
dependencies. It does not move branches, create public tags, push refs, or
silently widen the snapshot.

### Slice-checkpoint adapter

A narrow adapter performs Git mechanics from an explicit manifest. It:

- validates the baseline identity;
- uses isolated staging state;
- writes the immutable tree and commit;
- verifies the resulting content manifest;
- atomically updates the authorized private reference; and
- returns a structured checkpoint receipt.

It does not decide whether a slice is accepted.

### Campaign supervisor

The supervisor owns checkpoint timing and lifecycle consequences. With the
current Git-backed route it can:

- request candidate checkpoints after required deterministic evidence;
- pin review requests to candidate checkpoints;
- create replacement candidates after repair;
- validate review/checkpoint identity before acceptance;
- advance the private accepted checkpoint only after configured gates pass;
- start the next slice from that accepted identity; and
- preserve truthful stopped, failed, stale, and superseded lineage.

### Reviewer

The reviewer receives a read-only checkpoint identity and evidence manifest. A
review result names that identity and cannot mutate it.

### Human authority

Human authorization remains necessary for operations outside delegated
campaign scope, including moving the user's branch, publishing tags or refs,
committing unrelated user-owned work, or superseding protected work when the
supervisor lacks that authority.

---

## Checkpoint receipt

An illustrative checkpoint receipt includes:

```text
checkpoint_id
checkpoint_kind
campaign_id
run_id
slice_number
slice_attempt_id
candidate_attempt
created_at

baseline_commit_oid
baseline_tree_oid
checkpoint_commit_oid
checkpoint_tree_oid
parent_checkpoint_id

plan_version
scope_revision
task_owned_files
baseline_overlap_files
reproducibility_dependency_files
excluded_dirty_files
path_content_digests
task_patch_digest

gate_receipt_ref
gate_receipt_digest
review_request_ids
review_result_ids

state
supersedes
superseded_by
retention_class
publication_state
limitations
```

Unavailable facts remain unavailable rather than being inferred. A checkpoint
with material limitations may still be useful for forensics, but it cannot
support an unconditional reproducibility claim.

---

## Lifecycle rules

### Candidate creation

When the configured route uses a checkpoint to submit review, it creates the
candidate after the deterministic evidence that the review claim consumes. Its
receipt binds that evidence and accepted plan version to the checkpoint content.
Other timing is valid only when it preserves the same immutable subject and
does not misstate which evidence applies to it.

### Review repair

A material repair that changes the reviewed subject needs a distinct immutable
candidate before evidence is claimed for that new subject. The prior candidate
and its findings remain immutable. Findings may transfer only through an
applicability judgment, explicit lineage, and any revalidation required by the
changed assumptions or consequence.

### Acceptance

Acceptance requires every configured review result and deterministic gate to
refer to the accepted candidate or to evidence explicitly proven still valid
for it.

The accepted reference advances once. Duplicate finalization returns the
existing identity or fails without creating a second accepted history.

### Stopped or failed work

Stopped or failed work may receive a recovery checkpoint when preserving it is
useful and authorized. That checkpoint must not advance the accepted baseline.

### Next-slice start

The current continuation route uses the predecessor's accepted checkpoint as
its attributable baseline rather than inferring the baseline from branch HEAD
or the mutable working tree. Another route is valid only if it preserves the
same exact baseline identity, authority, and attribution consequences.

---

## Review queue integration

The durable review request should replace an ambiguous
`workspace_snapshot_id` with a checkpoint identity and content digest.

At minimum it should bind:

```text
checkpoint_id
checkpoint_commit_oid
checkpoint_tree_oid
task_patch_digest
accepted_plan_version
scope_revision
gate_receipt_digest
reviewed_claim_ref
material_assumptions_ref
established_consequence_ref
limitations_ref
review_lineage_ref
```

Before review begins and before its result is accepted, the runtime verifies
that the queued identity still exists and that the result names the same
checkpoint.

If later work produces a different candidate tree, the old review request and
result remain truthful about their original checkpoint and are never silently
retargeted. Consequential use against the later tree requires an applicability
judgment. A material conflict requires refreshed, composed, or
provenance-bearing superseding evidence; a non-conflicting change may retain the
old result with that applicability recorded.

This makes delayed review reproducible and makes duplicate-review suppression
reliable across restarts.

---

## Overlap detection and plan changes

Git checkpoints strengthen mechanical overlap evidence:

- path changes can be compared between exact trees;
- renames and deletions can be observed from immutable diffs;
- content-digest changes can invalidate queued review deterministically;
- two plans can compare task deltas against protected checkpoints; and
- accepted ancestry can distinguish continuation from unrelated branch
  movement.

They do not replace semantic overlap judgment. Different files may still alter
the same state owner, contract, producer, or consumer.

When a plan changes mid-flight, the current coordination capabilities include:

1. retain the prior plan version and checkpoint lineage;
2. revise a mutation reservation transactionally when an actual concurrency or
   authority contract requires one;
3. compare the revised consequence and assumptions with relevant reviews;
4. create a new candidate only after the revised work and gates complete; and
5. mark obsolete candidates stale or superseded without deleting them.

---

## User commits, pushes, and branch movement

An external commit or push is an event, not proof that the campaign should
reinterpret its work.

Because campaign state names exact checkpoint and baseline identities, the
supervisor can distinguish:

```text
user branch moved
campaign accepted checkpoint unchanged
candidate checkpoint unchanged
working tree changed
```

The supervisor then evaluates whether the event changes the slice's declared
baseline, scope, or reproducibility dependencies. It does not automatically
claim the user's commit as task output or abandon an otherwise valid review.

Integrating an accepted checkpoint into the user's branch is a separate,
authorized operation.

---

## Persistent agent state integration

Persistent supervisor state should reference, rather than copy:

```text
current accepted checkpoint
active candidate checkpoint
candidate attempt
checkpoint creation state
pending review bindings
protected scope
integration state
retention obligations
```

Checkpoint creation itself needs idempotency. After interruption, recovery
must determine whether the tree, commit, and reference were created before
retrying. A repeated request with the same manifest should return the same
checkpoint consequence or a deterministic conflict.

Raw sessions remain forensic evidence. Normal continuation should be possible
from durable state, checkpoint receipts, Git objects, review state, and gate
receipts.

---

## Relationship to reviewer evaluation

Candidate and accepted checkpoints naturally create paired evaluation cases:

```text
candidate before review
review findings
finding dispositions
candidate after repair
accepted checkpoint
```

This supports meaningful comparison of review providers, models, and harnesses
without reconstructing mutable historical worktrees.

An evaluation case can bind the original contract and evidence to an exact
candidate, replay multiple blinded reviewers, and determine:

- whether each reviewer found confirmed defects;
- whether it produced unsupported findings;
- whether different harnesses made complementary errors;
- whether closure review verified the actual repair; and
- which review classes can safely use a same-model fallback.

Checkpointing supplies the artifacts. Human or executable adjudication still
establishes whether findings are correct.

---

## Retention and publication

Private references must have an explicit retention policy because an
unreferenced Git object may be garbage-collected.

Possible retention classes include:

```text
active_campaign
pending_review
accepted_lineage
recovery_required
evaluation_corpus
expired
```

A checkpoint remains reachable while any active campaign, queued review,
unresolved finding, recovery path, or retained evaluation case references it.

Private checkpoint refs should not be pushed automatically. Durable sharing
may use an authorized remote namespace, bundle, artifact store, or repository
policy. Publication must preserve provenance without exposing excluded or
user-owned material beyond its authority.

Human-visible tags are optional. If used, annotated tags should normally mark
accepted checkpoints rather than every candidate attempt.

---

## Initial implementation boundary

A convincing first version does not require automatic branch integration,
remote publication, semantic merge, or general Git hosting support.

It needs:

1. a versioned checkpoint manifest and receipt;
2. baseline and previous-accepted identity validation;
3. an isolated temporary index or worktree;
4. allowlisted inclusion with attribution classes;
5. private candidate and accepted references;
6. atomic and idempotent reference updates;
7. immutable candidate identity when review or later reproduction requires it;
8. distinct candidate identity when repair changes the reviewed subject;
9. review and gate binding to exact checkpoint identities;
10. next-slice continuation from the accepted checkpoint;
11. stopped/failed checkpoints that cannot advance acceptance; and
12. retention sufficient for queued review and recovery.

The adapter may eventually be exposed as a `slice-checkpoint` skill or
capability. The supervisor remains the lifecycle owner; the adapter performs
bounded Git mechanics.

---

## Evidence of completion

The initial capability is convincing when executable tests demonstrate:

- checkpoint creation does not change the current branch, working tree, or
  user index;
- unrelated dirty and untracked files are excluded and unchanged;
- declared baseline overlaps remain reproducible without becoming task-owned;
- a candidate review result cannot be applied to a materially different tree
  without an applicability judgment;
- a repair creates a new candidate while preserving the old one;
- acceptance advances one authoritative private reference exactly once;
- the next slice starts from the accepted checkpoint;
- stopped or failed checkpoints cannot become accepted implicitly;
- restart during tree, commit, or reference creation recovers idempotently;
- a conflicting later integrated change cannot reuse review evidence whose
  material assumptions no longer apply;
- a non-conflicting later change can continue with recorded applicability;
- a delayed review remains truthful about its immutable checkpoint despite
  later worktree changes;
- user branch movement does not rewrite checkpoint attribution; and
- a pre-review/post-repair pair can be exported as a blinded evaluation case.

---

## Metrics worth preserving

Useful observations include:

- checkpoints created by kind and lifecycle state;
- candidate attempts per accepted slice;
- checkpoint creation failures and recovery outcomes;
- excluded dirty paths and material reproducibility limitations;
- applicability conflicts detected across tree, patch, or material-assumption
  changes;
- duplicate checkpoint operations suppressed;
- storage retained by lifecycle class;
- time from candidate checkpoint to review result;
- accepted checkpoints integrated into user branches; and
- evaluation cases recovered without transcript archaeology.

Metrics inform architecture and retention. They do not determine acceptance or
grant Git publication authority.

---

## Open design questions

- Should the first adapter use a temporary index, isolated worktree, or both?
- Does an accepted campaign checkpoint become the parent of the next private
  checkpoint even when the user's branch has moved independently?
- Which declared dirty dependencies may be included in a private snapshot, and
  what policy prevents accidental secret capture?
- Should accepted checkpoints receive annotated tags, private refs only, or a
  configurable publication policy?
- What storage owner records checkpoint receipts and reference transactions?
- How should checkpoint retention coordinate with review-queue cleanup and Git
  garbage collection?
- When may a review finding transfer across trees without full re-review?
- How should repositories without Git provide an equivalent immutable content
  identity?

---

## Compact principle

> **Bind every result to an immutable, attributable subject and preserve a
> truthful continuation identity. The current Git-backed route supplies those
> consequences without taking ownership of the user's Git state.**
