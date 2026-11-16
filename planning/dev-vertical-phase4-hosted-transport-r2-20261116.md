# Phase 4 design, revision 2: hosted provider transport for the Pi coding path

Prepared 2026-11-16. This revision supersedes, for implementation, the
[original design](dev-vertical-phase4-hosted-transport-20261116.md), which
stays as written. It incorporates the [operator decisions](dev-vertical-phase4-decisions-20261116.md)
and the [independent Fable review](../reviews/dev-vertical-phase4-design-fable-20261116/REVIEW.md)
(accept with required changes).

Standing: a proposed implementation design. It is not acceptance, and not an
owner change to `session-task-admission`. Piece-(a) repairs are recorded in
`records/dev-vertical-stream-session-r2-20261116/`.

## Unchanged

- A new versioned profile, working name `work-engine.pi-coding-hosted.v1`.
- The inference container runs with `--network none`, has no `/auth` mount,
  and its CredentialStore holds a nonauthenticating marker.
- Access is projected and held on the host only, and is used only inside the
  dispatch closure.
- The existing provider reservation flow remains the single financial owner.
  `pi_route` provides route order, refusal-driven switching and provenance
  only.
- Old credential-mounted profiles are removed after the hosted profile
  qualifies.

## 1. Logical request and attempts (review R1, P1)

A coding `provider_request(id)` is the **logical request**. The container is
blind to attempts: it receives one grant and one stream.

**Ledger shape.** The existing `provider_requests[id]` gains an ordered list
of attempt records:
`Attempt { attempt (global counter), route, payer, reserved_microusd, state, http_status, raw_terminal, relay_digest, disposition }`.
Each attempt record is persisted **before** its upstream dispatch.

**Order for each attempt k:**
1. The financial owner reserves for attempt k under `id`. Attempt 1's
   reservation is made at `provider_request`, before the grant.
2. `router.next()` selects the route.
3. `PiOwner::enter(ProviderAttempt)` (see section 2).
4. `pi_stream::pre_dispatch`.
5. Dispatch.

**Release rule.** Only the financial owner applies it. An attempt's
reservation is released only when its disposition is:
- `Refused`: the head status is in `refusal_status`, zero bytes were relayed,
  and the body was drained to its framed end and screened; or
- `NotDispatched`: a head-phase cancel was confirmed by `pre_dispatch`, before
  any upstream byte was written.

Every other disposition keeps the full reservation: `Completed` until its
existing raw-usage settlement, and `Uncertain`.

**New grants.** A next attempt is reserved only after its predecessor
settles as `Refused`. `Uncertain` or unsettled attempts block the request,
and the existing "any unsettled reservation blocks new grants" rule stays.
The router's allowance check is disabled for this profile (set it to
`u64::MAX`), so the router can never refuse on budget and only the financial
owner admits money.

**Tools.** Tools are not affected. A switch happens only on `Refused`, when
nothing has reached the container. Tools run only after a relayed `done`.
`refusal_status` stays limited to 401/402/403/429; 5xx must never be added
(review K5).

## 2. Effect identity and lease span (review R2, P1)

- Add `Effect::ProviderAttempt(request, attempt)` in `session-task-admission`,
  keyed `provider:{request}:{attempt}`. This is a change in the admission
  owner and needs that owner's acceptance. Until then, the offline probe uses
  the global attempt counter, as it does today.
- The lease is entered before `pre_dispatch` and held through the end of the
  relay. Its `observed` digest is one of:
  - the relay digest, for a relayed stream;
  - `hash(status‖refusal body)`, for `Refused`;
  - `hash("not-dispatched:<attempt>")`, for `NotDispatched`.

  `Uncertain` leaves the lease unknown. This enforces "uncertain cannot
  authorize another" at the admission owner as well as in the router.
- The legacy IPC-grant lease, which today is observed with a constant digest,
  does not apply to the hosted profile.

## 3. Host-loop integration (review R4, R5, K2; focus 2)

- One active `pi_stream::Session`, or `pi_stream::Unavailable`, per container
  stream. Host-loop tick:
  1. `session.poll(screen, 0)`;
  2. if `session.accepts_native()`, route that stream's
     `pull`/`cancel`/`ack` frames to `on_native`;
  3. otherwise route frames to the existing coding dispatch.
- `stream_interleaved_frame` and `stream_foreign_frame` from a session become
  "attempt `Uncertain`, reservation retained, job failed with the attempt
  recorded". They are never a bare protocol error.
- No permitted attempt: `Unavailable::answer` is sent. The next native frame
  is offered to `on_next`, which confirms a crossing `cancel(1)` and returns
  any other session envelope to the coding dispatch.
- Release after a terminal event drains in a polled `Draining` state. Native
  handling never blocks on the provider.
- `provider_observation` is produced by the **host** for the attempt that
  reached the container, from `Relay`. Container-sent `provider_observation`
  is refused in this profile.

## 4. Host-produced raw terminal (review K3)

- `rawTerminal` is the full data payload of the last `response.completed`
  event in the forwarded bytes, including `response.status`, and is checked by
  the existing settlement predicate.
- `backendModel` comes from `response.model`; `httpStatus` from
  `Relay.status`.
- Missing or inconsistent values keep the reservation, as today.

## 5. Complete screened SSE evidence (decision 4, review R3)

Terminal-event capture is settlement evidence only; it does not close this
item.

**Proposed mechanism.** Retain `Relay.forwarded` for each relayed attempt as a
create-only checked store artifact, `stream-<request>-<attempt>.sse`. These
are the exact screened bytes already forwarded. Bind each artifact to the
`forwardedSha256` in the attempt's control record. Retain refusal bodies the
same way.

**Capacity.** From source: 1 MiB per stream maximum and six requests give
about 6 MiB, within roughly 10 MiB of non-closure store capacity. The store
budget must be rechecked when this is implemented. If the operator or
acceptance owner prefers to narrow the requirement instead, the narrowing
must state that only digest, chunk timing, normalized usage and the terminal
event are evidence, and that a byte-level replay cannot be reconstructed.

## 6. Bounds and timeouts (review K1)

- **Response size.** The profile's `MAX_RESPONSE` must cover the catalog's
  `maxTokens`. Measure SSE bytes per output token on a synthetic large
  response before fixing the bound.
- **Hitting a bound** leaves the attempt `Uncertain` with its reservation
  retained.
- **Read timeouts.** The upstream idle read timeout is set by the profile, not
  inherited from probe constants. It must not exceed the job's consumption
  deadline.
- **Pi's head timeout** is set by the host in `init.headTimeoutMs`. It must
  cover host admission plus the permitted failover attempts. The 60 s default
  is a stop-gap (section 7).

## 7. Admission latency: a dependency, not absorbed here (review focus 4)

The review counted about 13 pinned-Python subprocess runs per attempt in
`PiOwner::enter` plus `check_current`. Each run re-hashes its pins. That
structure explains several seconds per attempt. The fix belongs to the
admission owner: one `current()` per `enter`, reuse of the evidence collected
by `begin_current`, and the board dump cached within a single call.
`pre_dispatch` (section 1, step 4) makes a cancel that arrives during this
time release cleanly instead of wasting a request.

## Implementation order for piece (b)

1. Add the ledger attempt records and release rule in `pi_coding_provider`,
   with unit tests modelled on the existing reservation tests.
2. Add the hosted profile schema, container arguments (network none, no
   `/auth`) and receiver fetch injection, reusing `openResponse`.
3. Wire the Host loop as described in section 3, with host-side
   `provider_observation`.
4. Run the offline Docker coding workflow on a synthetic hosted route, and
   inspect `NetworkMode=none` and the absence of a credential mount.

Then piece (c): qualification across failover, crash recovery, cancellation,
accounting, exports and evidence.
