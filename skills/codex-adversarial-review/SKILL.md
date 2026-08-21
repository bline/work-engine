---
name: codex-adversarial-review
description: Perform a read-only, fresh-process Codex adversarial review when a campaign explicitly accepts same-model review. Do not use when model- or provider-independent review is required.
---

# Codex Adversarial Review

Produce evidence and a review verdict without mutating the repository.

This adapter represents `accepted_same_model_review`. It does not provide
cross-provider, cross-model, statistically independent, or independent-reasoning
evidence.

The caller launches the reviewer in a fresh process with `fork_turns: "none"`
and the configured model and reasoning effort. Do not pass builder-session
history. The reviewer must directly read the repository instructions, full
`DESIGN.md`, and full `PHILOSOPHY.md` before evaluating the change.

Review adversarially against the accepted objective, invariants, placement
certificate, implementation, and available validation evidence. Inspect source
directly and return concise findings with severity, evidence locations,
consequence, and remediation guidance, followed by an acceptance or blocking
verdict. Return evidence rather than hidden reasoning.

Remain read-only: do not edit files, apply patches, regenerate artifacts,
commit, or perform external mutations. Ordinary read-only test observation is
permitted only when the caller's gate contract authorizes it.

Record an accepting result with evidence class `accepted_same_model_review`;
the corresponding human-facing label is `accepted same-model review`. Report the observed
model, reasoning effort, fresh-process isolation, absence of inherited builder
context, same-model relationship, and `independence_claimed: false`. Stop if
the configured provenance cannot be observed or if an acceptance requirement
demands independent review.
