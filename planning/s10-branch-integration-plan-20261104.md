# S10 and connector branch integration queue

Date: 2026-11-04. Status: proposed merge plan, based on local Git and owner state
at approximately 21:17 UTC. The user requested a merge plan; no shared-branch
merge, push, deletion, deployment or implementation launch has been performed.

## Decision

Consolidate accepted work before admitting the proposed $15/90-minute runtime
pressure slice. That proposal remains unanswered; this ordering recommendation
neither approves it nor cancels it. Preserve the existing owners and their
in-flight grants while assembling the merge queue.

The proposed destination is the current shared working branch,
`rescue-detached-20260911`, pinned for this preflight at
`d86df598f8d140003c9ae2a803b2b7af79c759f5`. This is a local integration plan,
not a proposal to push that branch or choose a different release branch.
Refresh the destination, claims and owner dispositions before executing it.

## Evidence

The local repository has 68 branches and 239 registered worktrees. Of the
30 branches dated November 1 or later, 17 tips are not ancestors of the shared
destination. This is not a count of 17 independent merge tasks. Three of those
branches have only patch-equivalent commits already represented at the
destination, and several others are nested predecessors of one current branch.
Older branches were not audited for acceptance or retirement in this preflight.
No remote fetch or remote-state freshness claim is included.

Machine-readable inventory, ancestry, patch equivalence, dry-merge output and
dirty-path overlap are under
`/home/bline/.local/state/work-engine/s10-merge-plan-20261104/`.

The shared checkout currently has 99 changed/untracked path entries: 19 tracked
modifications and 80 untracked entries. These are preservation state, not merge
authority. In particular, `rust/Cargo.toml`, `rust/Cargo.lock` and four manual-host
paths overlap the signed-Pi branch's incoming changes. Three manual-host paths
already match that branch's bytes, while the other overlapping paths do not.
The CLI branch also overlaps both dirty workspace Cargo files.

## Queue and readiness

| Order | Change group | Standing and action |
| --- | --- | --- |
| 1 | `implement/pi-context-observer-20261104` | Ready for an explicitly authorized merge of the pinned branch. Source commit `6f0ecd4c` is independently accepted; the separate planning commit `c2f1b21e` records a proposal, not implementation authority. The preflight dry merge into `d86df598` was clean, and its eight incoming paths had no overlap with the shared dirty checkout. This queue document is a later proposal-only addition on that same branch. Re-pin its final tip before merge. |
| 2 | Signed-Pi/delivery lineage | Hold until the current backend owner finishes repair, independent recheck and final acceptance, then supplies the exact final commit and full publication delta. Current observed branch `private/dot-signed-pi-fixture-20261104` at `44ed91c2` is not yet accepted. Integrate its accepted lineage once, rather than merging each ancestor as a separate feature. |
| 3 | `implement/rust-shared-cli-read-20261104` | Hold until the query owner supplies final qualification/review acceptance and a frozen tip. It was actively changing during this read-only audit (`ce237f04` to `2fe2de43`); neither observation is a final publication pin. Reconcile its Cargo changes with the accepted backend tree. |
| 4 | AH application server | Planning only, owned by `01a1a862-034f-7750-b86b-de29e11cf327`. It should consume the accepted runtime/query boundaries and eventual common integration base. No AH implementation merge exists to schedule from this preflight. |
| Separate audit | Older planning, exploration and legacy runtime branches | Retain until their owners identify accepted, superseded, imported or still-needed content. Patch equivalence is evidence for avoiding duplicate changes, not deletion or abandonment authority. |

The backend and CLI merge order may be exchanged if final owner evidence makes
that more useful. Both need a combined Cargo reconciliation and affected checks;
neither should be merged merely because another feature's Git merge is clean.

The relevant ancestry is:

```text
private/session-delivery-foundation-20261103  07918fbc
  -> private/delivery-pressure-s10-20261104  f426efd6
  -> private/dot-s10-coding-20261104         0dc72776
  -> private/dot-signed-pi-fixture-20261104  44ed91c2 (repair pending)
```

`integrate/manual-host-docker-20261103` is already an ancestor of the current
shared destination and of these branches. The exploration branch is not an
ancestor by commit identity, but both its commits are patch-equivalent at the
destination. The same applies to both commits on
`fix/manual-host-large-state-20261103` and the one commit on
`codex/mcp-catalog-staging-repair-20261103`. Do not reapply those patches.

The accepted S10 current-Rust integration result is also a private source
overlay, not simply a clean branch tip to merge. The backend assignment names
that input manifest `496a28ff4566d2e0e2be09a573d765362e5dd20e84e8556bd414e3dc0f04fe18`.
Confirm how its task-owned changes appear in the accepted backend final tree;
do not separately replay an overlay already consumed by that lineage. Captured
unrelated Rust baseline files remain preservation state unless publication of
their changes is independently authorized.

## Known integration work

Read-only pairwise Git preflights found:

- Observer plus current signed-Pi candidate: no content conflicts.
- Observer plus observed CLI candidate: no content conflicts.
- Current signed-Pi plus observed CLI candidate: content conflicts in
  `rust/Cargo.toml` and `rust/Cargo.lock`.

Those tests used `git merge-tree --write-tree`; they did not update a branch,
index or working file. Clean Git merging is not semantic integration acceptance.
The conflict result is tied to the observed tips, not to future repair commits.

Resolve workspace membership and dependencies from both accepted change sets,
then regenerate/check the lockfile under the selected pinned offline toolchain
where supported. Do not choose one entire manifest/lockfile as “ours” or “theirs”
and drop the other component. Run the affected package and integration checks
against the actual merged source; retain the original source-bound evidence
without claiming it automatically qualifies the new composition.

## Execution boundary

1. Obtain the exact merge authorization and final accepted publication subjects.
   The user-visible adapter commit is authorized; advancing the shared branch
   is a separate effect that this planning request has not performed.
2. Claim the integration/publication resource and re-read active ownership.
   Pin the expected destination HEAD and complete incoming delta. Coordinate
   with backend, query and AH owners before affecting their input contracts.
3. Assemble conflict-heavy merges in a fresh integration worktree/branch.
   Preserve the shared dirty checkout. Resolve the owner/disposition of each
   overlapping dirty path before destination publication. Do not stash, reset,
   clean, mass-commit or overwrite the 99-path shared state.
4. For the observer-only merge, verify its final exact delta, existing pressure
   dependency bytes and lack of dirty-path overlap. Run checks only where the
   merged base changes an applicable validation premise; preserve the accepted
   five-test, strict-Clippy and cold-build qualification otherwise.
5. Publish only the accepted merged subject against the expected destination,
   preserving unrelated index/worktree state and normal repository commit
   behavior. Record the resulting commit and source/evidence identity. A merge
   does not activate a runtime profile, deploy a service or waive release gates.
6. Update consumer handoffs to the integrated revision. Retain branches and
   worktrees until a separate cleanup decision verifies their history and
   evidence obligations; no branch/worktree sweep is included.

The next implementation admission should start from that accepted integration
base or explicitly declare a still-private dependency. This makes accumulated
work usable without treating every private branch, accepted component and
active repair as equally ready for publication.
