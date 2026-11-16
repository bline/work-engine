# Dev vertical: event-driven host stream session (phase 4, piece 1), 2026-11-16

Standing: bounded offline implementation result for piece (a) of the
[phase 4 decisions](../../planning/dev-vertical-phase4-decisions-20261116.md).
Not acceptance, not the Astra/xhigh review, not live qualification. The
decisions record asks for a focused design review before piece (b).

## What changed

- **`pi_stream::Session`** is an event-driven host side of one response stream.
  - The host loop calls `on_native(frame)` and `poll(wait)`. Both return the
    frames to send. Neither blocks on the provider; a separate thread owns the
    upstream connection, including reading the response head.
  - States: head → (refusal) → await-op ↔ upstream → close → done.
  - Dropping an unfinished session closes the upstream connection.
- **Cancel while awaiting the head.** The consumer may send `cancel(1)` before
  any head. The host confirms it with `response-cancelled` even if a head
  crossed it, then closes upstream. If the head cannot be read, the host sends
  `response-failure` (ordinal 0) followed by the normal single-`ack` close.
- **Refusal handling moved into the session.** A head status selected by the
  caller (`route::refusal_status`) is drained, bounded and screened, and is
  returned as `Progress::Refused`. Nothing reaches the consumer, which still
  awaits a head for the same stream; the caller may start another attempt.
- `relay()` now drives a `Session` over a blocking channel. The probe uses it,
  so every Docker control below exercises the state machine against actual
  Pi.
- **Receiver `openResponse`.** Head handling and the body share one cancel
  owner. The fetch `signal`, which carries Pi's head-timeout/abort, sends
  `cancel(1)`. `response-unavailable` and head-phase failure reject the fetch.
  Pi's head timeout is configurable through `init.headTimeoutMs` (default
  60 s).
- **Finding:** a 10 s head timeout is too short for hosted dispatch. Host
  admission per attempt (`PiOwner::enter` plus `EffectLease::check_current`)
  took 4–10 s here. `unavailable4` on the previous subject shows Pi cancelling
  during host admission, with the attempt correctly left uncertain.
  `failover5` only passed at 8.6 s. Lease-check latency in the existing
  admission owner is worth investigating separately.

## Evidence (one subject: [inputs.sha256](inputs.sha256); [evidence.json](evidence.json))

- Rust `pi_stream` 16/16. New tests cover:
  - a cancel while awaiting the head is prompt and the endpoint observes the
    close;
  - a cancel crossing a head is confirmed;
  - a pull before the head is refused;
  - a refused head is drained and never relayed, and a refusal body carrying
    the credential is refused;
  - an unreadable head produces failure (ordinal 0) followed by an ack.
- Node `openResponse` 8/8. New tests: abort during head → `cancel(1)`; a head
  crossing a cancel is discarded; unavailable/head failure reject with a single
  ack.
- Docker, actual Pi:

  | Run | Result |
  | --- | --- |
  | `stream14` | full workflow passes |
  | `failover6` | passes (primary 1 receipt; secondary 6 receipts) |
  | `unavailable5` | `route_unavailable_attempt_limit` |
  | `midquota5` | uncertain; secondary 0 receipts |
  | `fault-stall6` | body-phase abort |
  | `fault-truncate8` | `relay_upstream_truncated` |
  | `crash-connect-2c` | recovery gives refused + uncertain, `uncertain_predecessor` |
  | `fault-headstall3` | endpoint never sends a head; Pi's 10 s head timeout → `cancel(1)` → host closed the endpoint at 6.5 s of stall (host admission consumed the rest); Pi: "Codex SSE response headers timed out" |

- Regression: only the four known `claude_boundary` ProgramDrift failures and
  the two pre-existing Node `pi-memory-guard` failures.

## Open for piece (b) and beyond

- Integrate `Session` into the Pi coding Host loop and financial admission.
  Requirements: per-attempt identity and reservation beneath the logical
  request, with the financial owner authorizing any release, replacement or
  grant. Use a network-none inference container with no credential mount.
- Preserve the complete screened SSE stream as protected evidence, or obtain an
  acceptance decision narrowing that requirement. Today only digest, timing,
  usage and terminal information are retained.
- Recovery still needs an authenticated registry/sealed-reader join, and
  containers orphaned by a crash still need cleanup.
