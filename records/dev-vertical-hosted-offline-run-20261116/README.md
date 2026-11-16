# Dev vertical: offline hosted-profile Docker run, 2026-11-16

Standing: piece (b) step 4 of the
[phase 4 design r2](../../planning/dev-vertical-phase4-hosted-transport-r2-20261116.md).
This is a bounded offline run on synthetic loopback routes with a synthetic
access token held by the Host. It is not acceptance, not independent review
and not live qualification. No live credential, provider inference, push or
deploy was involved.

Subject: `3c1948b1`. The release `coding-qualification` binary was built from
the working tree that was then committed unchanged as `3c1948b1`. Steps 1–3 are
`9131a20d`, `9d24449c`, `5df2d1c4` and `1445a648`.

## What ran

[run.py](run.py) prepares a `work-engine.pi-coding-hosted.v1` job. The job
uses the pinned Pi runtime, the offline rustc toolchain and the qualified node
image, plus a route table with two loopback routes. The runtime admission
binds that route table's digest. The harness starts
`examples/pi_hosted_coding_observer.rs` and runs
`coding-qualification --execute`. The observer's primary route refuses its
first request with 429. Its secondary route serves the four-request
continuity flow: one `workspace_exec` call, a final text, recovery after the
reset, and the continued turn.

## Result (run `r4`, [evidence.json](evidence.json))

- Exit 0, `bounded_live_continuity_completed`, 14.8 s. Host receipt
  `component_passed`. Lifecycle `continued_parent: CompletedSettled` with one
  physical reset. The validator container passed the independent
  `acceptance.rs` (exit 0).
- Inference container: `NetworkMode=none`, read-only rootfs. Its mounts are
  `/config`, `/export`, `/pi`, `/profile`, `/receiver/receiver.mjs` and
  `/receiver/pi-native-tools.mjs`. There is no `/auth` mount and no access
  file, and `private-access` is empty.
- Failover: request 1, attempt 1 went to `primary` (synthetic-subscription),
  which refused with 429. The attempt's reservation was released and the
  refusal body was retained. Attempt 2 went to `secondary` (synthetic-api)
  and completed. Requests 2–4 went directly to `secondary`, because a refused
  route stays unavailable for the rest of the job. The gate admitted exactly
  these 5 attempts under one policy digest.
- Settlement: every request settled at 266 µUSD. The status, backend model
  and raw terminal usage behind that settlement were produced by the Host from
  the relayed bytes; the container sent no observations. Each attempt's
  retained SSE or refusal body matches the digest in its attempt record.

## Defects found by earlier runs (fixed in `3c1948b1`)

| Run | Finding |
| --- | --- |
| r1 | The SDK zstd-compresses Codex SSE request bodies, so the Host refused the fetch. Now the receiver decompresses, with output bounded to 45000 bytes. The Host refuses any `content-encoding` and forwards only plain JSON. |
| r2 | With a debug build, the Host's repeated self-hashing took about 268 s, and the gate's 5-minute budget freshness lapsed at request 2. This was a harness problem; later runs use a release build. Request 1 had already completed with failover and Host-produced settlement. |
| r3 | Lifecycle settlement refused the gate's `pi.provider_attempt` ingress records (gate defect from `9d24449c`). The gate now reports attempts instead of recording them. The rehydration turn's bare `grant:new` was also corrected. |

## Open

- Independent (Fable) review of steps 1–4.
- Piece (c): qualification across failover variants, crash recovery,
  cancellation, accounting, exports and evidence.
- Public HTTPS transport (phase 6), then removal of the credential-mounted
  profiles once hosted qualifies.
- Pi's head timeout is still the 60 s stop-gap (design section 7).
- Known pre-existing failures, unchanged: two `tests/lifecycle.rs` timing
  tests, and the `pi-signed-subscription` node harness (3 pass / 8 fail).
