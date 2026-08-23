---
name: durable-state
description: Publish and read opaque durable values under stable keys with integrity-checked compare-and-swap revisions.
---

# Durable State

This capability owns durability mechanics only. Callers own payload meaning and
every transition they authorize. The store accepts an opaque stable key, opaque
bytes, and an expected opaque revision. Publication is atomic and reads verify
integrity. Revisions prove exact stored bytes; they grant no semantic authority.

Accepted publications retain an immutable predecessor link beneath the stable
current key. Callers may read an exact retained revision or list bounded pages
from the accepted predecessor chain in deterministic newest-to-oldest order.
The exclusive cursor is an opaque revision identity. Historical reads are
observational only: they do not publish, replay, authorize, or reactivate the
payload they return. Legacy values without predecessor metadata remain readable
and form an explicit history boundary rather than fabricated ancestry.
The Git adapter anchors each accepted revision with a private retention ref in
the same ref transaction that advances the stable current pointer. Git garbage
collection therefore cannot silently erase retained history; removing retention
refs requires a separately authorized retention policy.

The first adapter stores immutable blobs behind private Git refs without moving
a branch or modifying the worktree or index. Git is an available adapter, not a
required future storage technology.
