# Independent design review packet: phase 4 hosted Pi provider transport

## Reviewer authority and constraints

- Reviewer: `claude-fable-5-1` at `high` effort, fresh context (the operator approved lowering it from `max` on 2026-11-16 because of the cost cap; the `max` run was stopped about 30 s in, before producing output — see `aborted-max-run/`).
- The operator assigned this review the same independent review authority
  intended for the inherited Astra/xhigh review (operator instruction,
  2026-11-16). It is a same-provider review: the implementation was written by
  a Claude Opus session.
- **Read-only.** Do not modify any file, run builds, tests, Docker or network
  operations, or access credentials.
- **Scope.** Review the proposed integration design (piece b) before it is
  implemented, together with the piece-(a) code it will build on. Do not
  redesign the subsystem. Prefer the smallest changes that close a real gap.
- **Subject.** Commit `02f7a571` on branch `rescue-detached-20260911`. Exact
  blob ids are in `subject-manifest.txt`. Read files from the working tree only
  where they match those blobs; otherwise use `git show 02f7a571:<path>`.

## Read first

1. `planning/dev-vertical-phase4-hosted-transport-20261116.md`: the proposed
   design.
2. `planning/dev-vertical-phase4-decisions-20261116.md`: operator decisions and
   two binding requirements (per-attempt identity and reservation under the
   logical request; complete screened SSE evidence distinct from
   terminal-event accounting).
3. `records/dev-vertical-stream-session-20261116/README.md`: the piece-(a)
   result, including the admission-latency finding.

## Code to consult as needed

- Piece (a), new transport:
  - `rust/crates/work-engine-manual-host/src/pi_stream.rs`: `Session`,
    `relay`, `Upstream`, the `stream_hold` screening use.
  - `rust/crates/work-engine-manual-host/src/pi_route.rs`: route policy,
    failover and recovery.
  - `rust/crates/work-engine-manual-host/receiver/pi-native-tools.mjs`:
    `openResponse` and the fetch injection.
  - `rust/crates/work-engine-manual-host/examples/pi_native_tools_probe.rs`:
    the current offline host composition (lease use, routing loop).
- Existing live Pi coding path to integrate with:
  - `rust/crates/work-engine-manual-host/src/pi_coding.rs`: frame dispatch,
    around the `provider_request` and `initialize` handling.
  - `rust/crates/work-engine-manual-host/src/pi_coding_provider.rs`: provider
    reservation, grant, observation and complete.
  - `rust/crates/work-engine-manual-host/src/pi_coding_auth.rs`: access
    projection.
  - `rust/crates/work-engine-manual-host/src/pi_docker.rs`: Host loop and
    container arguments.
  - `rust/crates/work-engine-manual-host/receiver/pi-coding.mjs`: the live
    receiver.
  - `rust/crates/work-engine-manual-host/docs/pi-coding.md`.
- Admission owner: `rust/crates/session-task-admission/src/pi_pilot.rs`
  (`enter`, `check_current`, `observed`).

## Focus areas

1. **Financial correctness.** One logical request may have several
   independently authorized provider attempts. Check that:
   - this cannot duplicate coding or tool effects;
   - each attempt has its own identity and reservation;
   - no reservation is released, replaced or newly granted without the
     existing financial owner's authorization;
   - an uncertain attempt can never authorize another.

   Assess how the design's mapping onto `provider_request` → reservation →
   `provider_grant` → `provider_observation` → `provider_complete` should
   work, and where it is underspecified.
2. **Host-loop integration.** Check that:
   - stream progress cannot block the Host loop;
   - every cancellation race closes deterministically: head phase, overtaking
     an outstanding pull, crossing a terminal frame, and crossing
     `response-unavailable`;
   - tool or other frames cannot interleave while a stream is open.
3. **Evidence completeness.** Assess whether the design provides protected
   retention of the complete screened SSE stream, as opposed to
   terminal-event accounting. If it does not, state what minimal mechanism
   would, or what an acceptance owner would have to narrow.
4. **Admission latency.** Host admission took 4–10 s per attempt
   (`PiOwner::enter` plus `EffectLease::check_current`), and the session raised
   Pi's head timeout from 10 s to 60 s. Assess from source what makes
   admission this slow, and whether raising the timeout addresses the right
   problem.

**Specific question from the operator.** After the headers arrive, `openResponse`
removes its `abort` listener (`signal.removeEventListener`). Verify whether
body-phase aborts are still handled reliably by the other mechanisms. In
particular, Pi's `parseSSE` calls `reader.cancel()` on abort, which reaches the
stream's `cancel()` and then `release()`. Also check the idle timer and the
release-before-next-envelope rule. This is a review question, not a confirmed
defect. The pinned Pi SDK is under
`/home/bline/cbm-work/pi-artifact-handoff-20261113/pi/node_modules/@earendil-works/`.

## Required output

Write your review as your final answer, in Markdown, containing:

1. A **disposition**: `accept`, `accept with required changes`, or `reject`,
   for proceeding to the piece-(b) implementation.
2. **Findings.** For each finding give:
   - a severity (P1/P2/P3);
   - concrete evidence (`path:line` at `02f7a571`, or the design section);
   - why it matters;
   - the minimal recommended change.

   Separate confirmed defects from risks and open questions.
3. An answer to each of the four focus areas and to the operator's specific
   question.
4. Anything you could not verify, and why.

Be concise. Do not restate the design.
