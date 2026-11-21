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

## Resolved: response size (D4)

Resolved by `da0a54f8` ([record](../records/dev-vertical-h2-route-20261117/README.md)):
the receiver no longer forwards `message_update` on the live bridge, so the
container's output grows linearly with answer length. A 3 000-delta answer
that failed closed on the closed-milestone binary now settles. Remaining
answer limit for the coding profile: the 64 KiB container frame (the history
frame carries the conversation twice). Relay throughput for long answers was
raised by coalescing already-received bytes (`1051e660`).

## Resolved: overrun completions held at their raw cost (G1)

Handed over by the Pi-accounting lane (board 5863; operator 5864: the dev
vertical owns it). A completion whose raw-usage cost exceeded its reservation,
or whose catalog cost overflowed the 1e9 uUSD bound, failed the job but
recorded no amount, so the ledger held only the reservation. Now the ledger
records `raw_microusd` on the attempt (or the request, for single-attempt
profiles), saturating at 1 000 000 001 on overflow, and holds
`max(reservation, raw)`; the job still fails and the request stays unsettled.
Ledgers without an overrun serialize unchanged. Tests
`overrun_completion_holds_its_raw_cost` and
`hosted_overrun_attempt_holds_max_of_reservation_and_raw`.

## Hosted analysis profile (`work-engine.pi-analysis-hosted.v1`)

Used by the organization's hosted review (`work-engine-review-hosted`). It is
not the research path. Open: crash and cancel are not exercised offline for
this profile (the transport and ledger are shared with the coding profile,
whose matrix covers them).

## Pi Host lifecycle relay (`work-engine.pi-lifecycle-hosted.v1`): slice 4

Research runs on this profile: the Pi Host in controller mode
(`work-engine-pi-host serve`, protocol
`native-realization/PI_HOST_PROTOCOL.md`), driven by `lifecycle-service`'s
Pi Host runner. It was qualified live by H2 on `claude-sdk`, with three arms
accepted on 2026-11-19 (`records/research-lifecycle-h2-live-result-20261119.md`).
The relay context bound is 500,000.

Production composition ([plan](../planning/pi-host-production-composition-20261120.md))
landed locally through `fe4e42c3`. Pi Host exposes `prepare` and
`profile describe`; task host validates the pinned installation at startup and
on install check, and the lifecycle runner composes a digest-bound job and serve
config per attempt. Capture hardening (`72ac4f34`) binds the redacted request
head per attempt and the original Completions body when rewritten; the runner
checks account-identifier absence before retention (`099a4883`).

The selected task-host → Pi Host offline run settled a Messages turn and
retained eligible capture using controlled-proof loopback (board #6054, merge
#6056). That run does not qualify HTTPS for the installed runner or release the
production startup hold.

`bdf8f731` joins streamed Completions tool calls to relayed calls and results
in `serve` capture. Offline fixtures verify the join and ambiguity refusal;
this is not live OpenRouter qualification. The 8 MiB per-turn retained-body
bound remains a separate contract question for Research Lifecycle, and the
live-crossing frame remains on hold.

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
