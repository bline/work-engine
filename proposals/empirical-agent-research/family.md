# Empirical Agent Research Foundations

## Family identity

- Family ID: `work-engine.empirical-agent-research`
- Formation source: user-directed environmental-analysis, trace-retention, code-change-characterization, and Review Bench research in the 2026-08-23 Work Engine session
- State: formed candidate family; not reviewed, evaluated, accepted, prioritized, or authorized for implementation
- Decision owner: user or future explicitly authorized portfolio owner

This family preserves two independently decidable product candidates discovered
while asking whether Work Engine retains enough evidence to study agent
environments and reviewer effectiveness retrospectively:

1. [`raw-agent-execution-evidence-archive`](raw-agent-execution-evidence-archive/proposal.md)
   defines lossless, sensitive, provenance-bound preservation of provider traces
   and session evidence without making transcripts operational state or product
   truth.
2. [`code-change-characterization-profile`](code-change-characterization-profile/proposal.md)
   defines a reusable, recomputable description of an exact change subject so
   review and agent research can control for the work being performed.

The candidates are related but neither causally requires the other. Raw
execution evidence can support environment and behavior research before a
change profiler exists. Change profiles can support planning, testing, risk,
and review routing without retaining model transcripts. Their combination is
especially useful to Review Bench because it separates the reviewer environment
from the characteristics of the reviewed change.

## Shared research model

The family preserves the distinctions:

```text
task / problem profile
        +
exact change profile
        +
effective agent environment
        +
observable behavior
        +
outcomes over time
        +
resource use
```

No single artifact owns all of these facts. Proposal packets, checkpoints,
review artifacts, claims, environment graphs, provider traces, metrics, Git,
and later corrections retain their existing semantic authority. The proposed
artifacts bind and project those owners; persistence or correlation does not
make a claim true or authorize a workflow transition.

## Formation evidence

[`formation-evidence.md`](formation-evidence.md) records the direct repository
and installed-runtime observations, user directions, inferences, and remaining
limitations used during formation. [`future-question-inventory.md`](future-question-inventory.md)
preserves the research-question families and the minimum raw evidence needed to
keep future analysis possible.

## Why these are not one proposal

The archive owns exact recoverability, integrity, capture completeness,
sensitivity, retention, and provider binding. The change profile owns an
attributed and recomputable characterization of immutable repository subjects.
They have different authorities, information lifetimes, privacy risks,
consumers, and acceptance evidence. Either may be revised, rejected, or
implemented without deciding the other.

## Authority boundary

These packets preserve candidate meaning. They do not authorize collection of
sensitive transcripts, choose a storage service, approve a retention policy,
establish research consent, accept either proposal, change roadmap priority,
select reviewer models, or authorize implementation. Those decisions remain
with their named owners.
