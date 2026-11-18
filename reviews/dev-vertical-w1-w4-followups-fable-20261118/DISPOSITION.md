# Disposition: Fable review of the W1-W4 follow-ups (verdict: accept)

| Finding | Disposition |
| --- | --- |
| P2-1 the check does not separate Pi's prompt mode | Fixed rather than only reworded. The check also requires the third system block to begin with the extension's Claude prompt (`You are Claude, working as a coding agent in pi, a terminal coding harness.`); the probe's Pi-prompt body (run1 0005) now refuses, and all six live bodies pass. The test has both a Pi-prompt-mode case (same first two blocks) and a `no_claude_code_blocks` case. Code comment and record now say what it is: a misconfiguration guard over container-authored bytes, not an identity proof; prompt mode is also held by the env delete and the pi-config allowlist, billing by the response guard. |
| P2-2 no durable reason; request-level reservation still held | Fixed: `Reservation.not_dispatched` records the code; a request with no attempt settles at 0 (nothing was written), so it is neither charged nor unsettled; `provider_failed` still fails the job. The receiver's 502 now carries the relay error message (the code). The test asserts all three. `ledger_refused` is unchanged (pre-existing; it can occur with prior attempts, and settling it needs its own analysis). |
| P3-1 exact thinking comparison not exercised live in a prefix | Recorded as a known untested condition for the first job with four or more requests. |
| P3-2 file count; body 3 positional check | Record says 94 files scanned = 92 run files + harness summary and error log; all bodies checked positionally with the tightened predicate. |
| P3-3 test label | Renamed ("no attempt reserved"), and the request-level assertions were added. |
