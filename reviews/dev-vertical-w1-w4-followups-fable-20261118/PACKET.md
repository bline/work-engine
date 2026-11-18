# Review packet: W1-W4 follow-ups (pre-dispatch subscription check; thinking blocks)

Subject: `ab4a22db` and `f84d4afb` against `ed30ad1f`, the W1-W4 head accepted by the earlier
review ([subject.diff](subject.diff)). The reviewer is read-only. Use `git show f84d4afb:<path>`
for exact bytes. No Docker, network or provider calls. Prior review:
[W1-W4](../dev-vertical-w1-w4-fable-20261118/REVIEW.md), [recheck](../dev-vertical-w1-w4-fable-20261118/RECHECK.md).

## Authority

1. **Pre-dispatch check (operator decision 2026-11-18).** On `https:anthropic-oauth`, dispatch only
   requests whose first two system blocks are Claude Code's billing header (`cc_entrypoint=sdk-ts`)
   and the Agent SDK identity. Other requests are neither reserved nor sent, and the job fails. The
   receiver clears `PI_CLAUDE_SDK_PROMPT`. Operator constraint: a third-party request on the
   subscription may draw extra usage.
2. **Thinking blocks (research-lifecycle owner decision, board 5886).** Six conditions, quoted:
   (1) closed set as observed; (2) non-semantic: never a tool call, the answer or a projection, and
   a thinking-only final refuses; (3) retained byte-exact, and in outgoing requests carried
   verbatim outside the two-rule canonicalization, so prefix extension compares them exactly;
   (4) thinking tokens count in settlement and bounds as output; (5) tests: a positive turn with
   thinking, text and a tool call, a negative with an unknown type, a negative with a
   thinking-only final; (6) record the effective thinking setting. The change may touch only
   `pi_analysis_proof.rs`; no native settlement or Codex path changes.

## Questions

- Is each of the six conditions met exactly, without widening the closed grammar or touching the
  coding grammar (`pi_coding_proof.rs` `text()` is shared and must be unchanged)?
- Does the continuity change keep both canonical rules unchanged for every other block, and
  compare thinking blocks exactly (including when the string-content rule applies)?
- Pre-dispatch check: can a non-Claude-Code request still reach the subscription route? Can a
  genuine Claude Code request be falsely refused? Is the refusal fail-closed and free of ledger
  side effects (no reservation, router settled, job failed)?
- Record: are the live-c2 claims supported by the run files under
  `~/.local/state/work-engine/dev-vertical-h2-route-20261117/live/live-c2`?

Severity P0-P3 with file and line, and a verdict. Output the review as a markdown document in
your final message.
