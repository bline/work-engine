# Slice 7 — Restore Roadmap Truth and Artifact Ownership

## Protected consequence

Future planners and campaign builders receive a truthful current map with one
canonical owner for each durable planning artifact. Historical evidence remains
attributed without being mistaken for current product state.

## Observed inconsistencies

- `roadmap.md` says the receipt validator accepts only `direct` and
  `falsified-placement`, while the current validator and tests accept any
  nonempty route identity.
- The same strategic handoff exists byte-for-byte under both `planning/` and
  `skills/planning/`; no consumer references either copy.
- `files.zip` is an unowned archive containing proposal documents.
- Large benchmark result and snapshot artifacts are present as untracked files;
  their intended ownership, publication boundary, and retention policy should
  be explicit before commit.

## Acceptance consequences

- Roadmap current evidence matches executable validators and current tests.
- Completed work is not left described as an unresolved gap.
- Each planning handoff has one canonical path and discoverable consumer or
  lifecycle reason.
- Convenience copies, exports, benchmark corpora, and binary snapshots are
  classified as source, generated evidence, intentionally retained fixtures,
  external artifacts, or disposable local files.
- Removing a duplicate does not discard the last durable representation of
  resume-critical information.
- Historical metrics and benchmark claims retain snapshot/protocol provenance
  and do not silently become production acceptance evidence.

## Required vertical proof

Trace every corrected roadmap claim to its owning source/test and verify all
repository references resolve to the selected canonical artifact paths. Confirm
that cleanup preserves required benchmark/proposal consumers and that no
unclassified archive remains in the intended commit boundary.

## Insufficient substitutes

- Editing only the roadmap status heading.
- Keeping duplicate files with a comment that one is canonical.
- Deleting benchmark evidence without checking its declared protocol consumer.
- Committing binary archives merely because they are small or generated.
