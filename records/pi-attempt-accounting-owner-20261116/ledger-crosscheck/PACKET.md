# Read-only cross-check: production Pi ledger against S1 finding invariants

Requested by the operator (2026-11-17) through the Pi accounting owner, Pi session `01a1e569`.
You are a read-only reviewer. You did not write this code and own none of it. The code belongs
to the dev-vertical session (`01a1e2a4`), which decides what to change. Do not edit anything;
write only your report.

## Subject

A pinned snapshot of the repository at the commit named in `snapshot/COMMIT`:
`/home/bline/cbm-work/pi-accounting-s1-owner-20261116/ledger-crosscheck/snapshot/`.
Focus: the hosted Pi coding path's financial owner, the provider ledger in
`rust/crates/work-engine-manual-host/src/pi_coding_provider.rs`, and what feeds it
(`pi_coding_hosted.rs`, `pi_stream.rs`, `pi_route.rs`, `pi_coding_https.rs`), plus the lifecycle
gate (`rust/crates/pi-pressure-context-host/src/coding.rs`) and the admission owner
(`rust/crates/session-task-admission/src/`). Design context: `planning/dev-vertical-hosted-gate-contract-20261116.md`,
`planning/dev-vertical-phase4-hosted-transport-r2-20261116.md`, `docs/dev-vertical-open-items.md`.

## Invariants to check

These invariants were enforced on a separate experimental accounting specification (S1) after
repeated review failures there. Check whether the production ledger upholds them, in its own
terms. Treat them as properties to examine, not as presumed defects.

1. **Retained raw charge.** A provider-reported charge or raw-usage cost cannot be erased or
   lowered by later, mutually consistent records (settlement, rollups, budget held), and a
   reservation never replaces a larger raw charge.
2. **Malformed identity.** A missing, `null`, empty, wrong-type or mismatched model identity
   (requested model, response/backend model) cannot yield a complete, priced, eligible or
   favourable settlement; unpriceable usage keeps the conservative hold.
3. **No self-declared authority.** Admission, route/payer permission, grants, scopes or attempt
   ordering cannot be established by data the untrusted side supplies (the Pi container, the
   provider response, a caller-written field); only the trusted owner's records count. Check
   identity fields are present and non-empty before equality is trusted.
4. **Raw-interval placement.** Which request, attempt, objective or time window a charge belongs
   to comes from the raw observed events and their times, not from supplied labels or reasons.
5. **Distinct evidence, not amount cardinality.** Two distinct reservations or charges with equal
   amounts are not one; byte-identical duplicates are not two.
6. **Uncertainty is not release.** Any incomplete, interrupted, unrecognised or unparseable
   outcome keeps the full hold and blocks; release only on a definitive, evidenced refusal or a
   confirmed non-dispatch.

## Report

Write `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/ledger-crosscheck/REVIEW.md`. For
each invariant: `upheld` | `gap` | `not applicable` with evidence (file:line and reasoning).
For each gap: severity (P1/P2/P3), exact location, a concrete scenario showing the wrong outcome,
the expected behaviour, and whether existing tests or the dev-vertical open items already cover
it. Note anything the dev-vertical has deliberately decided (open items, operator decisions),
and do not re-raise a recorded decision as a defect. List what you read and what you did not.
Keep it concise. Cost target: about $8.
