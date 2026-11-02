# Phase 0 validation and allowance

Phase 0 validates a design proposal and data fixtures. It does not validate
an executable task authority or a live writable session.

## Checks performed

- Both ASCII and UTF-8 request strings exactly match the existing Python
  `owner_answer.request_json` reference codec and `request_digest` result.
- Python and Node SHA-256 agree for both request digests and both exact-answer
  WebAuthn challenges. An added answer-space changes the challenge; a changed
  budget changes the request digest, in both vectors.
- Sixteen UI and sixteen backend semantic obligations have unique IDs within
  each set and nonempty preconditions/expected consequences. They are data
  cases for future implementation tests, not 32 executed runtime tests.
- The collected backend contribution matches its source owner's receipt hash.
  Seven selected work-engine source hashes still match that contribution's
  recorded evidence. Graph coverage for nine relevant paths was content-verified
  at generation `2026-11-02T20:19:44Z` / `u4959e4d8df4ea0abg2`, with no recorded
  issue. This is bounded evidence, not proof of repository completeness.
- devwits-auth is unavailable in that graph; the UI findings use bounded direct
  reads of its protocol, approval UI, API and laptop verifier/sync sources.

No valid passkey proof is included. No runtime admission, native coding task,
new model actor, deployment or global activation occurred. Source-owner
concurrence does not substitute for the next slice's independent review.

## Admission and budget basis

User approval: `2026-11-02T19:55:07.033Z`, for the recommended phase 0 in
completion-plan commit `9e4d7849c4a5e0d186eceaba563845cb1b49c864`.
First funded own response: `19:55:20.161Z`; productive boundary `20:55:20.161Z`;
hard boundary `21:10:20.161Z`. Cap $10 / 75 minutes, productive $8 / 60 minutes,
protected closure reserve $2 / 15 minutes, across both participating roots.

Closed productive/collection window: `2026-11-02T20:32:07.177Z`, about 37 minutes
after first funding. All required backend output and final concurrence were
collected. The coordination/publication reporting tail is separately attributed.

Observed conservative interval upper estimate at `2026-11-02T20:32:07.221Z`:
**$4.4008**, 56 recorded Codex calls, none unpriced. These are speed-adjusted
API-equivalent proxy amounts, not bills or measured subscription quota.
The backend interval conservatively begins at dispatch `19:59:22.241Z`; the peer's exact
phase 0 source boundary is `20:03:06.937Z`. Its earlier independent OAuth
verification grant is not an additive phase 0 charge. Subsequent reporting
is not included in this snapshot; final tail remains observable in the ledger.

Canonical live allowance/collection record is
`/home/bline/.local/state/devwits-auth-rollout-20261102/session-management-phase0/assignment.json`;
the measurement is `cost-current.json` beside it. Reader identity is
`metrics/scripts/usage_ledger.py`, SHA-256
`ed026f40f93801c9fffab4a9d5d5fc32745fd1d94a6137edd22ed875b5a67291`.
Other contemporaneous objectives remain attributed to their own owners, not
added to this cap or assumed to cost zero when attribution is unavailable.

No phase 1 implementation allowance, pilot permission or optional bookkeeping
grant is inferred from this closeout. The [next bid](first-slice-bid.md) is a
concrete separate proposal.
