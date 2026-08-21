# Review Bench Experiment Contract

Status: active for new cases and review attempts  
Contract version: `evidence-calibrated-review/1.2`  
Effective: 2026-08-20

This version inherits
[`evidence-calibrated-review/1.1`](experiment-contract-v1.1.md), including its
caller-context blocking gate, and changes only reviewer output classification.
Existing 1.0 and 1.1 results retain their recorded protocol and schema identity.

## Separate defects from verification

New attempts return `review_bench_result_v2` and classify each evidence-backed
item exactly once:

- `findings`: claimed defects or material risks that could change acceptance,
  violate an invariant, cause a regression, mislead a user, or create a
  significant maintenance defect. Findings carry severity and blocking status.
- `verified_claims`: acceptance-critical conclusions established to high
  confidence. Each identifies the acceptance criteria it supports. Verified
  claims do not carry severity or blocking status.
- `observations`: real non-defect improvements, documentation notes, or
  out-of-contract facts worth retaining. Observations do not carry severity or
blocking status.

Do not use a finding merely to narrate correct behavior. Do not hide a defect in
an observation. Do not duplicate one claim across arrays. An empty `findings`
array is the normal representation of a clean review; verified claims explain
why acceptance is justified without distorting defect metrics.

Verified claims must be high confidence. An `accepted` verdict must cover every
case acceptance criterion through verified claims and contain no blocking
finding. A `rejected` verdict must identify at least one blocking finding. Use
`blocked_unverified` when material criterion coverage remains below high
confidence.

## Protocol prompt addition

Append this paragraph to the 1.1 prompt for new 1.2 attempts:

> Return result schema v2. Put only defects or material risks in findings. Put
> evidence-backed acceptance conclusions in verified_claims and identify the
> acceptance criteria they support. Put real non-defect improvements or
> out-of-contract notes in observations. Verified claims and observations have
> no severity or blocking label. Classify each claim once.
