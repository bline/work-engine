---
name: codex-adversarial-review
description: Perform a read-only, fresh-entry Codex adversarial review with retained remediation context when a campaign explicitly accepts same-model review. Do not use when model- or provider-independent review is required.
---

# Codex Adversarial Review

Produce evidence and a review verdict without mutating the repository.

This adapter represents `accepted_same_model_review`. It does not provide
cross-provider, cross-model, statistically independent, or independent-reasoning
evidence.

The caller launches one reviewer in a fresh process with `fork_turns: "none"`
and the configured model and reasoning effort at the start of the review
obligation. Do not pass builder-session history. The reviewer must directly
read the repository instructions, full `DESIGN.md`, and full `PHILOSOPHY.md`
before evaluating the change.

Retain the returned reviewer identity through the bounded remediation loop.
After a valid finding is fixed, use `followup_task` on that same reviewer with
the exact new subject or delta, affected gate evidence, prior finding
identities, and remediation summary. Do not spawn another reviewer or resend
the full initial research merely because one repair occurred. Label each pass
as continuation of the initial fresh entry; do not claim new freshness or
independence.

Reset only when the subject's architecture, placement, scope, or review premise
materially changes, the context becomes degraded or oversized, or model
judgment identifies a renewed fresh-perspective need. Record the reason and
preserve applicable prior findings. If the retained identity is unavailable,
record a replacement or reconstructed reviewer rather than claiming continuous
context.

Review adversarially against the accepted objective, invariants, placement
certificate, implementation, and available validation evidence. Inspect source
directly and return concise findings with severity, evidence locations,
consequence, and remediation guidance, followed by an acceptance or blocking
verdict. Return evidence rather than hidden reasoning.

Remain read-only: do not edit files, apply patches, regenerate artifacts,
commit, or perform external mutations. Ordinary read-only test observation is
permitted only when the caller's gate contract authorizes it.

Record an accepting result with evidence class `accepted_same_model_review`;
the corresponding human-facing label is `accepted same-model review`. Report
the observed model, reasoning effort, initial fresh-process isolation, absence
of inherited builder context, continuation or replacement provenance,
same-model relationship, and `independence_claimed: false`. Stop if the
configured provenance cannot be observed or if an acceptance requirement
demands independent review.
