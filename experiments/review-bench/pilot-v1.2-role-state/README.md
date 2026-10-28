# Role-state paired review pilot

This pilot reconstructs two immutable checkpoints from the durable independent-review
episode-state slice and compares fresh `gpt-5.6-sol` low- and high-effort reviews.
Claude's historical review is retained as provenance, not converted into a current
result or used as the answer key.

## What the paired case established

The first checkpoint contains seven adjudicated truth findings: six blocking
implementation defects and one nonblocking test gap. The repaired checkpoint fixes
the advance-replay, remediation-evidence, new-finding-attribution, and focused-test
gaps, but it is not clean. Three blocking defects remain:

- identical `begin` retry is rejected after a successful creation;
- writer replacement accepts a successor manifest naming a different subject;
- ordinary transitions remain possible after `mark_uncertain`, producing
  contradictory lifecycle state.

All three surviving paths were reproduced directly against the exact candidate-2
archive. The original v1 truth is preserved and the corrected v2 truth records the
adjudication amendment.

## Descriptive comparison

| Configuration | Defect recall | Blocking recall | Precision | Severity overstatements |
| --- | ---: | ---: | ---: | ---: |
| Sol high | 0.60 | 0.667 | 1.00 | 4 |
| Sol low | 0.50 | 0.556 | 1.00 | 4 |
| Union | 0.80 | — | — | — |

The aggregate hides an important case-level reversal. On candidate 1, high effort
found substantially more. On candidate 2, low effort found all three surviving
defects while high effort found only begin replay. High effort also positively
verified the exact-subject criterion despite the reproduced replacement-subject
failure; scoring schema v1 does not currently penalize incorrect verified claims.

With two correlated checkpoints and one attempt per effort/case, these results are
descriptive only. They support keeping effort variants as potentially complementary,
not selecting one as the production reviewer.

These attempts are also explicitly a pre-claims baseline. As Work Engine is
built, each sufficiently identified slice should preserve immutable checkpoints,
its effective contract, deterministic evidence, and review/remediation provenance
as a later candidate. Once proposal, claim, reliance, reopening, and Codebase
Memory evidence can be supplied faithfully, selected snapshots should be rerun
under a new claim-aware protocol. After the system is complete, naturally occurring
reviews become a third, prospective production evidence class rather than being
pooled silently with reconstructed development cases.

## Relationship to the earlier visual case

The historical Visual Evidence development run did use Chrome Vision for rendered
state observation; the consolidated run review records 13 one-shot broker
constructions. The independent review findings themselves were semantic/runtime
normalizer defects: a null interpretation threw and an ambiguous Graph route dropped
recorded click evidence. The reconstructed benchmark packet contains source, tests,
and executable probes, but no screenshot or CDP evidence from the original review.

That makes the case suitable for code-review comparison, but not for measuring a
reviewer's ability to diagnose appearance, layout, or interaction defects from a
live browser. A true UI benchmark should freeze browser observations (URL/build,
viewport, screenshot, DOM/accessibility snapshot, interaction trace, console/network
evidence) as part of the case and give every reviewer the same packet.

## Artifacts

- `corpus.json` — frozen contracts and snapshot identities
- `truth.v1.json` — initial sealed adjudication
- `truth.json` — provenance-bearing corrected adjudication
- `provenance.json` — historical Claude and checkpoint provenance
- `results/` — four fresh result-v2 attempts
- `scoring/` — finding-level adjudication
- `report.json` — generated descriptive comparison
