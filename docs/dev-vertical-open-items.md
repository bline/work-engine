# Dev vertical: deferred and monitored items

Kept current. Owner of the items below: the dev vertical (hosted Pi coding
profile, `work-engine.pi-coding-hosted.v1`). Qualification and history:
[the qualification record](../records/dev-vertical-hosted-qualification-20261117/README.md).
Close an item here when it is resolved, with the commit or record that resolves it.

## Deferred by operator decision (2026-11-17)

| Item | Decision | Revisit when |
| --- | --- | --- |
| **Subscription route registration and permission.** The completion plan (phase 6) says reusing Pi's Codex OAuth access "without qualification is insufficient" and asks for the actual registration, permission and account-model checks. The Host's primary route (`https:chatgpt-codex`) reuses that access today. | Deferred. Do not forget: required before calling the subscription route qualified for anything beyond this operator's own use. | Before wider use of the subscription route, or if OpenAI registration for open-source token sharing becomes available. |
| **API route price parity (R-c).** Settlement and the budget apply the pinned `openai-codex` catalog rates on every route, so the 700000 µUSD cap bounds a proxy, not actual API spend. | Fine as is; tune later. | When API spend matters or the catalog and API prices diverge. |
| **Unexercised live failures.** A real quota refusal (429/402), a genuinely revoked or expired credential, and a live cancel or crash against a real route. | Monitor and log; revisit with more evidence. | When the observation log below has real occurrences. |

## Monitoring log

Run after live hosted jobs (it appends only new events):

```sh
python3 metrics/probes/hosted_live_observations.py \
  metrics/dev-vertical-live-observations.jsonl <run-or-state-dir>...
```

It records, per attempt, any refusal, uncertain or not-dispatched outcome, any
401/402/403/429, and completions without the framed end, plus jobs that ended
cancelled or failed. It copies nonsecret journal fields only (refusal bodies by
digest). Entries with payer `chatgpt-revoked` are the deliberate synthetic
invalid-token transitions from qualification, not real events.

## Other open items

- **Grants are self-issued (K-c).** The producing binary (`coding-qualification`)
  issues the lifecycle grants for its own job. Authority today rests on the
  operator-pinned runtime admission, the job profile and the preparation
  authority; the grant adds no independent authority. Operator (2026-11-17):
  acceptable for single-operator local use; revisit (independent grant issuer)
  before jobs can be submitted by anyone other than the funder.
- **Owner acceptance.** Every result is `owner_pending`: the Host and server
  never accept a job's output; adopting the exported change is the owner's
  action. Operator (2026-11-17): no change for this vertical; unattended
  acceptance would need its own policy and owner.
- **Follow-ups from review 3:** remove the Sol-high grammar from
  `lifecycle-store`; remove the retired subscription switch from
  `session_events.py` (digest-pinned by `session-query` `OWNER_SHAS`, so it needs
  a coordinated pin update).
- **Not yet exercised offline:** pre-dispatch cancel and a gate refusal between
  attempts in the Docker flow; a crash during the release drain; deadline
  expiry mid-stream; non-chunked framing.
