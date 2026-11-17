# R2 reconciliation: root summary for the operator (2026-11-17)

Inputs: `design/RECONCILIATION.md` (Opus xhigh, $3.59) and `review/REVIEW.md` (Fable xhigh, $3.20,
`revise_before_owner`, 1 P1 + 5 P2 against S-C/N3/N4 wording; S-A and S-B sound). Pass cost $11.01 against the
$10 cap: $1.01 over, caused by root context re-reads after the subscription's 5-hour limit interrupted the session.

## Root position after review

- **Confirmed:** the hosted Host + lifecycle-store + `CodingContinuity` already own per-effect currentness (the
  transaction is held across the send), the durable consumed-before-entry record (Host journal) and settlement. A
  spawn-boundary host in review-delivery would be a parallel mechanism (N1a: don't build it). Today's lifecycle
  grants carry no independent authority (K-c holds in source).
- **Held:** S-C (independent issuer on the hosted path). The review shows the issuer/revocation channel cannot reach
  the per-job single-writer lifecycle store as designed (P1-1), its decision content for a coding job is undefined
  (P2-2), and the operator already accepted self-issued grants for single-operator use until non-funder submission
  is wanted (P2-1). Revisit only when non-funder submission is wanted.
- **Next work (fixture standing, no live spend):**
  - **S-A admission core** (pure crate: records with 6.1 F4/F5, reservation-sealed observation incl. the
    same-observation/later-boundary negative, Gate O subset O3/O4/O5/O8/O9, Gate R R1–R4, five states, 6.1 §6
    oracle classes). `GrantProjection` and `realization_digest` move out (their only consumer is held S-C).
    Estimate **$22–35**.
  - **S-B consumption seam and real discharges** (review-delivery's total discharge function returns S-A outcomes
    for separation / component-binding / authority / realization-admission / observer-identity; mapping kinds stay
    assumed; realization checks stay undischarged; regate before consumption against a fresh fixture observation;
    close the R2-S P3s on ungated `run_attempt*` and mintable `AssumedDischarges`). Estimate **$19–29**.
  - Total **$41–64**; recommended grant **$65** (new objective). Stop rule as before.
- **Owner decisions** (recommendations): N1a do not build the spawn-boundary host — yes. N1b production
  delivery home is Host + lifecycle — yes, conditional on a hosted review profile (N7), which stays deferred until
  mapping-rule owners exist (O7). O4 wording: "as amended by N1": the fixture path keeps the R2-S pre-spawn gate and
  the O10 single-use gap; no reservation-sealed delivery admission exists for reviews until N7. S-C/N2–N4/N6 held.
- R2's exit after S-A + S-B: met on fixture standing **as amended by N1/O4**, not as originally worded.
