# Review Provider Amendment 01

- Run: `3876601b-c43d-44c6-94a4-e22646d64257`
- Slice: `1`
- Timestamp: `2026-08-24T17:33:29-06:00`
- Authority: human approval in the active session

## Trigger

The preferred Claude provider returned HTTP 429 weekly-quota exhaustion during
placement falsification and supplied no review evidence. Claude is therefore
currently unavailable for this campaign obligation.

## Effective amendment

Replace the defaulted review-execution role for the remainder of this run:

```yaml
prior:
  independent_review:
    provider: claude
    skill: claude-recon-implementation
new:
  adversarial_review:
    provider: codex
    skill: codex-adversarial-review
    model: gpt-5.6-sol
    reasoning_effort: medium
    evidence_class: accepted_same_model_review
    isolation: fresh_process
```

The supervisor selected `medium` effort because this slice introduces shared
identity, authority, immutable history, reliance, and projection semantics
across several boundaries. No configured hard limit prevents that selection.

## Evidence boundary and provenance

The fallback must record the failed Claude attempt, quota reason, provider
transition, actual Codex model and effort, and fresh-process boundary. The
Codex reviewer must not inherit builder context and must report
`model_relationship: same_model` and `independence_claimed: false`.

This fallback is `accepted_same_model_review`. It does not establish
cross-provider, cross-model, statistically independent, or independent-reasoning
evidence and cannot satisfy an acceptance condition that explicitly requires
independent review.

## Human approval

> I officially authorize Codex Sol fallback if Claude is unavailable for
> whatever reason. Document fallback if it happens. Please proceed.

