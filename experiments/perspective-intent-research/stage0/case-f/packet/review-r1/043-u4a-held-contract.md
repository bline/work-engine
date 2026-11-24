# U4-A design checkpoint — verify a held start without an activation path

Status: offline candidate, not installed admission or a release decision. Source `d410c9e1`; board 6071–6072 align RL and dev-vertical on the boundary. The scope remains C1 (pinned Docker and framed IPC) for the fresh-start, three-tool Pi analysis profile; D1 and production activation are not implemented by this slice.

## Placement

Select a separate Host-owned, controlled qualification arm **before** reading provider credentials, binding provider transport, wiring a task-effect relay, or servicing the controller. It shares the exact selected Pi receiver/container launch, registration, and checked-ready path with ordinary `serve`. Use the same provider/model *descriptor* and registration inputs but do not create any usable provider transport or task lease. A provider-less qualification profile with a different receiver/tool surface cannot substitute. Likewise, withholding `turn/start` from an otherwise enabled Host is not a hold: published observation leaves `turn/start` callable today. `native_startup.rs::publish` is Codex-specific and does not own this qualification. Existing installed Pi startup continues to refuse while held; this slice does not touch it.

## Evidence owner and usability

The Host owns the child, same-session framed handshake, and exact stop. The receiver `ready` frame supplies `session.getActiveToolNames()` (`receiver/pi-coding.mjs`); Host code validates the actual reported names against the pinned profile (`pi_coding.rs`), then the controller checks same-session idle (`pi_host_serve.rs`). Retain that actual validated list, not one recomputed from policy. It attests names at ready, not tool handler semantics, wire request tools, or any tool invocation. The pinned runtime/receiver and request-tool digest are different evidence.

Resolve the Docker container by owned identity and require a fresh pinned-client inspection after same-session ready while the no-effects hold still applies. Bind the observed exact container ID and checked image, labels, network mode, read-only root, user, limits/capabilities, mount destinations/modes, and no credential/task/repository mount to expected profile, Host/job/runtime/receiver/composition digests, session, and observation timestamp. Project safe observed fields and inspect digest; do not serialize raw environment, mount source secrets, or credentials. `observe_relay_container` already checks identity and containment; facts not in its emitted projection must remain under a named Host verification claim or be projected explicitly. Host-attributed Docker and receiver observations are not independent attestation of the daemon/kernel or of continuing liveness.

A readiness record can report the point-in-time observation and no usable capability was created. A terminal report can add the counts of early intent, refused intent, and entered dispatch/tool/snapshot effects through cancel/exit. An attempted forbidden intent is a qualification refusal even when nothing entered. Unavailable counts are not zero. Neither record is a grant; if an owner later needs to activate a **same** live process, that requires a separate design for attaching authority while held and re-observing currentness at the actual transition. This no-activation fixture is not evidence for that transition.

## Terminal cleanup

The explicitly accepted cancel for the owned, still-held session uses session-correlated `bridge_close` and concludes only after bounded receiver/attach exit collection and exact container removal observed against the owned identity. Report `held_cancelled` only after those checks; if process/container state is unavailable, retain a cleanup-unknown result and any known diagnostics. Repeated or mismatched cancel is not a new teardown authority. Controller EOF, malformed input, receiver failure, deadline, and failed containment stay distinct refusal/failure paths and retain diagnostics; none becomes `held_cancelled`. Keep ordinary serve completion rules unchanged.

## Next evidence and ownership

- Host owner: implement the isolated admission arm and qualification-only terminal observation; verify registration is the same selected profile before claiming scope. No change to ordinary serve or shared activation protocol.
- RL: verify bindings and refusal semantics as independent consumer only after the Host subject is pinned. No task-host installed-readiness receipt or activation in this checkpoint.
- Prove offline success followed by explicit clean cancel; no transport credentials/load/connect, task relay, or turn; observed pinned tools and Docker profile; exact cleanup. Exercise early intent, attempted start, mismatched ready/container/pin, disallowed profile, missing inspection, wrong/repeated cancel, EOF, and cleanup observation unavailable. A fake controller/receiver can test individual failure branches; only real selected receiver + Docker qualifies the positive fixture.

## Separate retention boundary

Board 6073 measured 8,388,608 logical body bytes accepted by Host/intake/store; one byte over refused. The retry reused one body file but was charged logically again. The exact-cap store artifact was 33,534,210 B; a synthetic inline task-host envelope was refused at the encoded 8 MiB acquire guard. This does not demonstrate actual actor/causal acquire at the cap. No change to either limit; lossless disk dedup would not alone fix inline transfer expansion.
