---
name: independent-review-state
description: Preserve one authority-bound retained adversarial-review episode across same-session remediation and truthful reviewer replacement without creating an independence claim.
---

# Independent Review State

This profile owns only the unfinished operational state of one retained
adversarial-review episode. It preserves the exact reviewed subject, the
reviewer's attributed findings and observed remediation status, unresolved
questions, pending next action, exact evidence and claim references, and the
reviewer generation and resumable runtime-session reference.

The profile name is historical. The state is provider-neutral and does not
establish that a reviewer is independent, cross-model, or cross-provider.
Provider selection, isolation, model relationship, and evidence class remain
owned by the review configuration and receipt. A Codex same-model reviewer may
use this profile while remaining `accepted_same_model_review` with
`independence_claimed: false`; its manifest uses the `adversarial_reviewer`
reader role rather than implying independent-review status.

The profile does not own semantic acceptance, supervisor or implementation
state, proposal decisions, claims, packets, receipts, checkpoints, schedules,
or repository content. References to those owners are immutable reliance
pointers. Persisting a finding does not make it evidence or authorize another
owner's transition.

An MCP server exposes this profile only when launched with an episode-bound
authority manifest. The manifest constrains the episode, subject, writer
generation, readers, and authority provenance. It is not authentication:
caller admission remains the responsibility of the trusted process launcher,
operating-system permissions, and MCP tool configuration. A runtime session ID
is a resumable reference, not a credential or semantic owner.

The ordinary result lifecycle is:

```text
fresh initial review
  -> initial result
  -> remediation / same-session re-evaluation
  -> reported
  -> exact later remediation subject / same-session re-evaluation (zero or more)
  -> reported
```

`reported` publishes the current result; it does not retire the episode. While
the episode and its bound writer remain active, an exact later remediation
subject may enter same-session re-evaluation from either `remediation` or
`reported`. This continuation preserves the prior result in immutable history
and does not create a new freshness or independence claim. This ordinary chain
is not an exhaustive state machine: any non-retired phase may become uncertain
when continuity cannot be trusted, may receive an authorized writer replacement,
or may be retired. Uncertain continuity requires reconciliation, authorized
writer replacement, or retirement; retirement is terminal.

Replacement requires a separately issued successor manifest for the same
episode, the next writer generation, and the exact predecessor durable
revision. It is recorded as reconstructed continuation, never as a new claim
of fresh independence. Compare-and-swap revisions and transition identities
make retries idempotent and fence stale writers after replacement.

Historical and exact-revision reads are observational. They never resume a
provider, replay a review, mutate a referenced owner, or reactivate retirement.
Physical deletion and retention pruning require separate authority and are not
available through this profile.
