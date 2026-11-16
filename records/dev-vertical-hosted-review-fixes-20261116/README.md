# Dev vertical: hosted transport, Fable review remediation, 2026-11-16

Standing: bounded offline remediation of the
[independent Fable review](../../reviews/dev-vertical-phase4-piece-b-fable-20261116/REVIEW.md)
of piece (b) (subject `c66f2baf`, disposition *accept with required changes*).
Not acceptance, and not a re-review. Code subject: `286474df`.

## Dispositions

| Finding | Disposition |
| --- | --- |
| D1 gate evidence volatile and logged before currentness | Fixed. The gate logs an attempt only inside the store closure, once the parent grant is current, with its deadline. `coding-qualification` writes `coding-hosted-gate-attempts.json` on every exit. Each attempt record durably stores `gate_entered`, `deadline_ms` and `routes_sha256` before any upstream byte, and failures are distinguished as gate refused, refused inside admission, or dispatch failed after admission. |
| D2 relayed bytes discarded on protocol-error and abort paths | Fixed. An open attempt closed by a protocol error, foreign or out-of-phase frame, poll failure, run-loop exit or launch failure becomes `Uncertain`; its exact relayed bytes are retained and their digest recorded. The upstream connection is closed. |
| D3 retained files not verified at collection | Fixed. SSE, refusal bodies and relay evidence are written create-only, the directory is fsynced, and the files are registered as journal artifacts. `receipt()` verifies them. The unbound `.json` is replaced by the retained `relay.json`. |
| D4 response bound (design §6) | **Deferred to phase 6**, before any live provider use. The 1 MiB bound stays, and reaching it fails closed as `Uncertain` with the reservation kept. Bytes-per-token cannot be measured offline because it depends on the live backend's delta granularity. Measuring it and setting a profile bound are entry conditions for phase 6. |
| D5 bare refusals for foreign or out-of-phase frames | Fixed with D2. |
| D6 missing transport found after reservation | Fixed. `run()` refuses the hosted profile without a bound transport. The attempt path takes the transport before the gate and never drops the gate. |
| K-a 1 s drain after `response.completed` | **Open, operator question.** Pi releases right after `response.completed`, so a provider that closes late or sends trailing bytes ends `Uncertain`, with the reservation kept and the job failed. Is that conservative rule intended for live providers? Piece (c) will add delayed-close and trailing-byte variants either way. |
| K-b to K-h | Carried to piece (c) or phase 6: credential selection per payer and a header allowlist before phase 6; issuer-independent grants; bounds reconciled at admission; transaction-held connect for TLS; admission vestiges; labelling of container-reported usage. |

## Evidence

- Unit tests: manual-host lib 74 pass. New hosted tests cover protocol errors,
  foreign frames and run exit retaining bytes, and gate refusal and a missing
  transport being recorded without dispatch. Gate tests: 14 pass; the
  contracted-grant test now checks that only the currently admitted attempt
  is logged, with its deadline.
- Offline Docker run `r5` on `286474df` ([evidence.json](evidence.json), derived
  by [evidence.py](evidence.py) from a run of [run.py](run.py)): exit 0 in
  10.3 s, `component_passed`, `CompletedSettled`. Every attempt has
  `gate_entered` and a deadline. 9 hosted artifacts were registered and
  verified, and the gate log lists the 5 admitted attempts.
- The earlier run's `evidence.json`
  ([offline run record](../dev-vertical-hosted-offline-run-20261116/README.md))
  was derived by an uncommitted inline script. `evidence.py` now owns that
  derivation.

Review cost: $5.90 on API billing, launched directly with `pi --api-key`
because subagents do not inherit that override
([note](../../notes/pi-subagents-api-key-not-inherited-20261116.md)).
