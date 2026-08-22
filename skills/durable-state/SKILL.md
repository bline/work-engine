---
name: durable-state
description: Publish and read opaque durable values under stable keys with integrity-checked compare-and-swap revisions.
---

# Durable State

This capability owns durability mechanics only. Callers own payload meaning and
every transition they authorize. The store accepts an opaque stable key, opaque
bytes, and an expected opaque revision. Publication is atomic and reads verify
integrity. Revisions prove exact stored bytes; they grant no semantic authority.

The first adapter stores immutable blobs behind private Git refs without moving
a branch or modifying the worktree or index. Git is an available adapter, not a
required future storage technology.

