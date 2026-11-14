# Worktree graph observation

Scope: manual-orchestration role guidance; authority remains with the active assignment and owning contracts.
Load when: allocating or recovering a worktree, or relying on graph freshness after edits.

## Worktrees and Codebase Memory

A lasting Codebase Memory watch belongs to a live MCP connection rooted at that worktree; querying or indexing another worktree does not register one, and the main repository and a slice worktree are distinct projects. At allocation or recovery, check the exact project root, `index_status` watch state and the current store generation. A watch marked active does not prove the latest edit is indexed: after edits, check candidate-file coverage and use direct source for unverified ranges. A manual `index_repository` with `persistence=false` avoids repository writes but keeps no watch. Tool availability can change mid-session, so rediscover the available MCP tools before falling back to CLI or source. Workspace cleanup does not authorize stopping a shared daemon or another agent's connection. [Repo-search](../../repo-search/SKILL.md) owns query, coverage and fallback rules.
