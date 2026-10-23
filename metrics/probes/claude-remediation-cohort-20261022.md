# Remediation cohort, 2026-10-22: closing outside-review findings

**Registered before either builder's first write.** Labelled **not R2**. This is
an observational cohort, instrumented identically to R2 under amendment 2's
cohort rule, and it is not scored against B1–B6 as an R2 result.

## What this cohort is

Two builder slices repair the five findings an outside reviewer returned against
R1's S3 and the P1 port. Both are frozen at base `624ec60e`, both are Sonnet 5.5
high in isolated worktrees, both run under `skills/manual-orchestration`
unchanged.

- **fix-a** — `rust/crates/context-pressure/**`. EXT-001 (P1): replace the
  `format!("{:e}")`-derived number formatting with `ryu-js =1.0.3`, because Rust
  and ECMAScript break ties between equally short round-trip candidates
  differently. EXT-005 (P3): give the golden suite a guard independent of the
  fixture's own counts. Plus correcting the `lib.rs:12` parity claim to state its
  evidence domain.
- **fix-b** — `rust/crates/decision-readiness/**` and
  `skills/manual-orchestration/scripts/integration_readiness.py`. EXT-002 and
  EXT-004 (P2, provenance), EXT-003 (P2, instrument).

## Why it is worth instrumenting

The campaign has no measurement of **remediation** work, only of greenfield
slices. Three properties make this cohort informative:

1. **The defect targets came from outside the orchestration.** R1 amendment 2
   established that in-orchestration review at these tiers finds P3-class issues
   reliably and provenance- and contract-class issues unreliably. This cohort
   asks the next question: given a finding it did not find, can the orchestration
   close it, and at what cost per finding.
2. **Both packets require reproduce-before / reject-after evidence per finding.**
   That is a stronger evidence requirement than any R1 or P1 packet carried. If
   it holds, it is a candidate treatment change for R2's successor, not for R2.
3. **fix-a's defect is a formatting-tie class, not an input.** Closing it
   requires a differential test outside the corpus, which is the exact
   methodological gap the review exposed.

## Measured, per builder

Context at first code write (B1-comparable, adjudicated by reading the matched
command, never by detector); pre-write read calls (B6-comparable); weighted
tokens `uncached + cached/10 + 5*output`; added Rust lines and Mwt/krs; context
boundaries declared and whether a successor continued (outcome A); gate results;
and, as a cohort-specific quantity, **findings closed per builder with
reproduce-before evidence present**.

## Pre-registered limits

- n = 2 builders, one slice each. Every figure carries n.
- Remediation against a named defect is **not** comparable to greenfield build
  cost. No Mwt/krs figure from this cohort may be pooled with R1's or P1's.
- Added Rust lines will be small; `krs` denominators under ~0.3 make the ratio
  unstable, and that will be stated rather than smoothed.
- Quality here is not reviewer-relative in the usual way: the findings were set
  by an outside reader before the work began. Whether the repair is **correct**
  still needs a reader who is not the author, and that is `not established`
  until one runs.
- This cohort shares a branch with R2's subject. Measurement commits are held to
  my own `metrics/**` paths.

## Status at registration

Both packets dispatched; no result file exists for either. Every quantity above
is `not established`. P1-M-001 was applied to both packets before dispatch and
passed, 15 of 15 read-set paths present at base. R2-M-002 was raised against
fix-b's write scope and reported at chatboard sequence 2494; its resolution is
pending with the coordinator.
