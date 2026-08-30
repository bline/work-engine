---
name: code-change-profile
description: Bind immutable Work Engine checkpoint subjects and derive recomputable deterministic physical code-change profiles.
---

# Code Change Profile

Use this capability to describe an exact repository change without assigning a
universal difficulty score or taking ownership of checkpoint, semantic, review,
or policy truth.

The analyzer consumes a versioned slice-checkpoint subject: version 1 retains
the historical full accepted/stopped lifecycle contract, while version 2 also
admits a canonical candidate receipt before review. It delegates candidate or
lifecycle validation to the corresponding `slice-checkpoint` validator, then verifies
the immutable base/result trees and patch digest before deriving observations.
It never reconstructs task scope from the current branch, index, or worktree.

The versioned contract is documented in
[`references/profile-contract.md`](references/profile-contract.md). Run:

```bash
python3 skills/code-change-profile/scripts/code_change_profile.py \
  profile --receipt <full-lifecycle-receipt.json>
```

Canonical JSON is written to standard output. Analysis is read-only.
