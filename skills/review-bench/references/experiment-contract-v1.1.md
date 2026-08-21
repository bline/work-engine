# Review Bench Experiment Contract

Status: active for new cases and review attempts  
Contract version: `evidence-calibrated-review/1.1`  
Effective: 2026-08-20

This version inherits the frozen
[`evidence-calibrated-review/1.0`](experiment-contract.md) contract except for
the blocking-adjudication rule below. Existing 1.0 results retain their recorded
protocol identity and are not reinterpreted as 1.1 attempts.

## Caller-context blocking gate

Before classifying a finding as blocking, establish the relevant state path:

```text
producer -> validation or ownership boundary -> supported caller -> consumer
```

Identify which supported production caller can reach the defect, which caller
invariants exclude it, and whether a supported consumer can observe or act on
the resulting state. Distinguish a demonstrated supported path from direct
function invocation with malformed, synthetic, or contract-excluded input.

A finding may still block when the downstream consequence is bounded if it
directly violates an explicit acceptance criterion at a supported boundary.
Severity describes practical consequence; blocking describes whether the case
contract can be accepted. Do not infer one from the other.

If caller or consumer reachability could change acceptance and remains below
high confidence, gather proportionate evidence or return the material claim as
unverified. Record the bounded contradictory path that was checked.

## Protocol prompt addition

Append this paragraph to the 1.0 reviewer prompt for new 1.1 attempts:

> Before assigning blocking status, trace the relevant producer, validation or
> ownership boundary, supported caller, and consumer. Separate supported
> production reachability from malformed or contract-excluded direct
> invocation. A bounded practical consequence does not make an explicit
> acceptance-criterion violation nonblocking; report severity and blocking as
> separate judgments.
