# Independent review packet: spec-r1 commit 2729107 (R8-OWNER-01, R8-SCHEMA-DOMAIN-01)

You are a fresh, read-only adversarial reviewer (Fable at xhigh, standing in for Astra xhigh by
operator direction; same provider as the root and builder, so this is same-provider review).
You did not author this change. Root: Pi session `01a1e569`. Do not modify any file except
your report.

## Subject

- Repository: `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/successor/subject`
  (git). Base `s1-base` (`d2551b9`, accepted predecessor manifest `5e594cac`); subject commit
  `2729107` on branch `spec-r1`. A clean checkout of the commit is at `/tmp/rootv-spec`.
  Review `git diff s1-base 2729107` and the resulting code as a whole, not only the delta.
- Built binary for the commit:
  `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/root-verify-target/debug/pi-accounting-spec-check`
  Predecessor binary: `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/successor/cargo-target/debug/pi-accounting-spec-check`
  You may run either against in-memory or `/tmp` mutants (`canonical SUBJECT /dev/stdin`,
  `evidence SUBJECT FILE`, `wire SUBJECT FILE`, `check SUBJECT --generated`). Use a scratch
  `CARGO_TARGET_DIR` under `/tmp` if you build anything.
- Builder's claims to challenge (do not trust):
  `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/spec-r1-evidence/RESULT.md`

## What must be true

1. **R8-OWNER-01 / C1-R8-01.** Only C constructs C-owned settlement validity
   (`SettlementDecision` or types carrying it), across every stage, boundary and writable
   constructor — not only S and V. S's typed forwarding of C's reference and C's own
   construction still pass. Original report:
   `/home/bline/cbm-work/pi-accounting-continuation-20261114/reviews/implementation-r8/C1-REVIEW.md` (C1-R8-01).
2. **R8-SCHEMA-DOMAIN-01 / C1-R8-03.** The published Draft 2020-12 schemas
   (`spec/*.schema.json`) and the Rust canonical/wire/evidence representations agree on every
   numeric and string domain, including representation (`1.0`, `1e0`, fractions, negatives,
   booleans, strings, u64 frontier precision), with exact checked conversion (no f64
   rounding), and every unsupported value gets a controlled error, never a panic/exit 101.
   Nano is checked i128 with nonnegative canonical decimal strings (`spec/ck.json` MONEY,
   SERIALIZATION). Original report: C1-R8-03 in the same file.
3. No regression: the predecessor's nine accepted gates and structural behaviour remain; no
   new financial policy; no edits to oracle-owned files; generated views match canonical
   source; dependency/impact and the 19 null acknowledgements are reported truthfully, not
   regenerated to make digests match.

Governing plan (binding):
`/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/full-realization-r1/planning/PLAN.md`
— rows R8-OWNER-01 and R8-SCHEMA-DOMAIN-01 and the numeric-representation paragraph after the
finding matrix.

## Concerns to examine (properties, not suspected defects)

- Completeness of the ownership check: could any construction path, alias, nested type,
  boundary declaration or generated rendering still let a non-C stage mint C validity?
- The new in-tree JSON number parser (`src/numbers.rs`): is it a faithful, total JSON parser
  for all inputs the CLI accepts (strings/escapes/unicode, nesting, duplicates, whitespace,
  huge exponents, negative zero), and does every consumer path actually use it? Is
  disagreement with `serde_json` possible on valid JSON?
- Schema/Rust agreement at every published bound and representation; any remaining
  `unwrap`/`as` cast/unchecked arithmetic reachable from input.
- Whether the new controls are discriminating (each negative hits its intended predicate;
  positives prevent blanket rejection).

## Report

Write `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/review-spec-r1/REVIEW.md`:
verdict `ACCEPT` | `ACCEPT_WITH_CHANGES` | `CHANGES_REQUIRED`, each finding with severity
(P1/P2/P3), exact location, executed counterexample or precise reasoning, expected behaviour,
and whether it is introduced by this commit or inherited. Then an `architecture_assessment`:
`bounded_repair_credible` | `local_design_correction` | `design_review_required`. State what you
read, executed and did not cover. Record that this is same-provider review.
