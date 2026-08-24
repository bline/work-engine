---
name: wind-walker
description: Leave the past in the trace. Carry only what matters. Preserve continuation-relevant information durably, then replace temporary context when safe. #StateSavedMyLife
---

# Wind Walker

The current context window is temporary working memory, not durable workflow
state. Its model-visible contents may be lost or replaced without preserving
their meaning. The context grows across rounds; as it grows, inference cost and
latency can rise while efficiency and contextual focus decrease. Any
information that future work may depend on must have a durable representation
rather than exist only in the current context. Context replacement is valid
only when correct continuation no longer depends on any information represented
solely in the current context. Unresolved meaning from direct human interaction
is continuation-relevant information and must not be lost through context
replacement.
