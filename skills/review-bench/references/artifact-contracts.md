# Review Bench Artifact Contracts

The deterministic validator in `scripts/review_bench.py` is authoritative for
field names and closed vocabularies. These notes explain the separation of
responsibilities.

## Candidate lifecycle and evidence eras

A preserved review candidate is not yet a corpus case. During development, a
slice may be retained for later use when it has:

- an immutable pre-review, candidate, repaired, or accepted checkpoint;
- the exact slice contract and acceptance criteria effective at that snapshot;
- deterministic evidence and artifact identities;
- original reviewer and remediation provenance when available; and
- explicit reconstruction limitations for anything not checkpointed directly.

Candidate capture must not invent expected findings or verdicts. Corpus
admission and truth adjudication remain separate later acts.

Keep three evidence eras distinguishable:

1. **Pre-claims development baseline** — the reviewer receives the snapshot,
   contract, and then-available evidence without the future claim system.
2. **Claim-aware replay** — the same snapshot is reviewed with a frozen,
   snapshot-appropriate proposal, claim, reliance, and Codebase Memory evidence
   packet under a newly versioned protocol.
3. **Post-build production evidence** — naturally occurring reviews use the
   completed system and produce prospective claim and reliance evidence as part
   of ordinary work.

Do not backfill future claims into an old baseline attempt or describe a
claim-aware replay as historical production behavior. Compatible analysis may
pair attempts by snapshot, but reports must preserve evidence era, protocol,
harness, graph generation, and supplied artifact identities.

## Corpus

`review_bench_corpus_v1` contains the shared protocol and blinded cases. Each
case records:

- review class and risk;
- one coarse route class plus multi-label semantic domains, topology, novelty,
  evidence types, consequences, and optional diff size;
- goal, acceptance criteria, and excluded scope;
- snapshot identity, digest, kind, reconstruction confidence, and limitations;
- artifact locators and roles; and
- deterministic evidence available to every reviewer.

Do not put known defects, historical review findings, or expected verdicts in
the corpus.

Keep cases governed by different reviewer protocol versions in separate corpus
artifacts. Do not change a corpus prompt after results have bound themselves to
it merely to make later cases use a newer protocol; aggregate across compatible
corpora only through tooling that preserves each result's protocol identity.

For Git-exportable cases, `snapshot.digest` is the SHA-256 of the deterministic
`git archive --format=tar.gz` bytes for `snapshot.identity`. Export fails if the
resolved ref no longer produces the recorded digest.

## Truth

`review_bench_truth_v1` is withheld from reviewers. It exactly covers the
corpus cases and records the adjudicated expected verdict and confirmed
findings with direct evidence and an adjudication basis. Case-level notes retain
material rejected claims and limitations. A historical Claude finding is not
truth until executable evidence or human adjudication establishes it.

When later evidence establishes that sealed truth was wrong,
`review_bench_truth_v2` preserves the correction without erasing the original.
It identifies the superseded v1 artifact by contained relative path and SHA-256,
records provenance-bearing adjudication amendments with exact before/after
targets, and exposes the corrected case truth used for scoring. CLI validation
checks the referenced artifact digest and verifies each amended value against
both versions. This corrects gold truth; it does not change the protocol or
reinterpret what a reviewer reported.

## Result

`review_bench_result_v1` is one reviewer attempt. Reviewer identity separates:

```text
provider
model
harness
inference_family
fresh_context
tool_access
reasoning_effort
review_protocol
pass_count
aggregation_strategy
aggregation_members
```

The result binds to the corpus snapshot digest and uses one of:

```text
accepted
rejected
blocked_unverified
stale_snapshot
```

Every finding records severity, blocking consequence, category, claim, direct
evidence, and calibrated confidence. The reviewer must not receive or emit
truth identifiers.

`review_bench_result_v2` preserves the v1 identity, verdict, timing, and defect
finding fields while adding separate `verified_claims` and `observations`
arrays. Findings are claimed defects or material risks and remain the only
items included in defect precision, recall, severity, blocking, and
false-positive metrics. Verified claims bind evidence to one or more stated
acceptance criteria quoted verbatim from the case contract. Observations hold real non-defect improvements or
out-of-contract notes. Neither v2 array accepts severity or blocking fields,
and one identifier cannot appear in more than one array. Do not migrate or
rewrite historical v1 results merely because their provider used the old
`findings` field for positive verification statements.

Verified claims require high confidence. An accepted v2 result must cover every
case acceptance criterion through verified claims and cannot contain a blocking
finding. A rejected v2 result requires at least one blocking finding. Use
`blocked_unverified` when material acceptance coverage cannot reach high
confidence.

## Scoring

`review_bench_scoring_v1` is created after review by an adjudicator. It maps
every reported finding to exactly one disposition:

```text
true_positive
false_positive
duplicate
nonblocking_observation
```

It also records whether the finding's evidence is valid and exactly which
truth findings were missed. This explicit mapping avoids unreliable lexical
matching between differently worded findings.

Scoring v1 does not adjudicate the correctness of `verified_claims`; it only
counts them. Claim-aware evaluation must add an explicit disposition for false,
stale, inapplicable, insufficiently supported, and correctly verified claims
before those results inform reviewer policy. A false high-confidence verified
claim may be more consequential than an ordinary missed finding and must not be
hidden by defect precision or recall.

## Report

`review_bench_report_v1` reports descriptive configuration metrics,
route-class stratification, resource use, and pairwise conditional and joint
miss measurements. Configuration identity includes reasoning effort, protocol,
tool access, pass count, and aggregation; those results must not be pooled.
The report identifies the truth artifact type and applied amendment IDs.
For adjudicated true-positive mappings it also counts severity and blocking-label
overstatements and understatements; these calibration counts remain distinct
from verdict-level false acceptance and false blocking.
For v2 results it reports verified-claim and reviewer-observation counts
separately from `reported_findings`. Legacy v1 items adjudicated as nonblocking
observations remain visible under their legacy count.
The report intentionally does not claim statistical equivalence or
independence. Repeated attempts from the same case are correlated.
