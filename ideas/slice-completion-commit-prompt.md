# Slice-Completion Commit Prompt

## Status

Idea / conceptual direction. This document is not a product contract,
implementation plan, or authority to create commits.

## Idea

After a slice reaches accepted completion, Work Engine should normally offer to
create a human-visible Git commit for that slice. The behavior should be
disableable by campaign or user configuration.

The model that performed the slice has unusually valuable context at that
moment. It understands:

- the intended consequence;
- what actually changed;
- why the chosen implementation belongs where it does;
- which alternatives or premises were rejected;
- what validation passed;
- what remains unresolved or deliberately deferred; and
- which changed paths belong to the slice versus the pre-existing workspace.

That is probably the best point in the lifecycle to compose an accurate commit
subject and body. Reconstructing the message later from a diff or terminal
receipt is possible, but loses semantic context and spends reasoning again.

## Distinction from slice checkpoints

The existing slice-checkpoint capability creates immutable Git commit objects
and private `refs/work-engine/checkpoints/` identities for review, acceptance,
recovery, and next-slice baselines. It deliberately does not move the user's
branch, use the real index, create tags, or publish refs.

A slice-completion commit would serve a different purpose:

```text
accepted checkpoint
    → immutable workflow and review identity

user-visible commit
    → authorized project history on the user's branch
```

The private checkpoint can establish the exact accepted content. It does not by
itself authorize placing that content in user-visible history.

## Intended consequence

Accepted slices can become coherent, reviewable units in ordinary Git history
without losing the model's best available explanation of what the slice
accomplished, while the user retains control over whether and how their branch
is changed.

## Possible interaction

At accepted slice completion, before the slice context is discarded, the model
could prepare:

```text
proposed commit subject
proposed commit body
accepted checkpoint identity
task-owned path set
included overlap or dependency paths
validation summary
unresolved or deferred consequences worth recording
```

When prompting is enabled, Work Engine would ask the user whether to:

- create the proposed commit;
- edit the message or included scope;
- leave the accepted checkpoint private and continue without a branch commit;
  or
- disable future slice-completion commit prompts for the campaign.

These are candidate affordances, not a required interaction design.

## Configuration direction

The useful initial policy appears to be conceptually:

```text
slice completion commit prompt: enabled | disabled
```

Enabled should mean “offer an authorized branch mutation,” not “silently
commit.” Disabled should suppress the prompt without disabling immutable slice
checkpoints, terminal receipts, or continuation.

An automatic-commit mode would create a materially different authority policy
and should not be inferred from this idea.

## Git and authority concerns

A later proposal must resolve how a user-visible commit interacts with:

- a dirty working tree or real index;
- user-authored baseline changes;
- pre-existing overlaps and reproducibility dependencies included in the
  accepted checkpoint;
- branch HEAD moving after the slice baseline was established;
- accepted checkpoint parentage that differs from the user's current branch;
- partial staging or a user-authored commit already in progress;
- repositories where commits must be signed;
- hooks, commit-message policy, and required attribution;
- an unattended campaign that otherwise has authority to continue; and
- a declined or deferred prompt when the next slice is ready.

The safe default should preserve the existing checkpoint boundary: if Work
Engine cannot prove that the proposed branch commit contains only authorized
content and is based on the expected branch state, it should retain the private
accepted checkpoint and avoid mutating user Git state.

## Information lifetime

The proposed commit message has value even when the user does not answer
immediately. The semantic message should therefore be produced while the slice
context is available and preserved with the completion state or another named
artifact. The later prompt can consume that proposal without requiring a new
agent to reconstruct the slice.

The proposed message is not itself a terminal receipt. It is a human-history
projection of accepted slice consequences.

## Relationship to campaign continuation

A branch commit should not be required for slice acceptance or for the next
slice to use the accepted private checkpoint as its baseline. Otherwise a
human-facing convenience would become a mandatory execution barrier.

If prompting is enabled, the campaign must make the pending interaction visible
and apply the configured authority policy truthfully. Whether continuation may
proceed while the prompt is pending is a product decision for a later proposal,
especially for unattended campaigns.

## Questions for proposal formation

- Should the prompt be enabled by default for interactive campaigns only, or
  for every campaign unless disabled?
- Is the proposed commit message stored in the checkpoint receipt, terminal
  receipt, compact handoff, or a distinct completion artifact?
- Can the accepted checkpoint commit itself be promoted onto the user's branch,
  or should the system create a separate human-history commit with the same
  accepted tree or task delta?
- How should the commit exclude user-owned overlap that was necessary for
  reproducible review but is not authorized for project history?
- What exact branch/index/worktree conditions make commit creation safe?
- Should declining once affect only the current slice, while a separate choice
  disables future prompts?
- What information belongs in the subject/body versus trailers or receipt
  references?
- Does a commit require a final freshness check when user Git state changes
  between acceptance and approval?
- How should signed-commit requirements and repository hooks be represented?

## Invalidation conditions

This direction should be revised if real usage shows that per-slice commits
produce poor repository history, that accepted slices do not align with useful
commit boundaries, or that safe branch mutation requires enough reconstruction
to eliminate the context advantage. In those cases, preserving a proposed
message or offering a later grouped commit may establish the useful consequence
more reliably.
