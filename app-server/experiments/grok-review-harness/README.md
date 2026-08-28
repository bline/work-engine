# External review harness experiment

This experiment asks one bounded question: can an external provider finish a revision-bound,
evidence-bearing implementation review without fabricating provenance?

The host constructs a clean packet from exact Git objects. Grok receives only
the packet, read/search capabilities, and a compact review contract. The host
rejects a result when its subject differs, its JSON is malformed, or a finding
cites a file or line range outside the packet. Accepted citations receive
host-derived evidence digests. The provider trace must also show a completed
direct read of every cited file, no undeclared tool, and no path outside the
packet.

The harness does not establish reviewer quality, independence, or production
acceptance. A completed receipt is only the prerequisite for later evaluation.
The Grok adapter is retained as failed experimental evidence. The Agy adapter
accepts Gemini Flash models only and rejects Pro model names before execution.
Model, reasoning effort, turn ceiling, and wall-clock timeout are case-local
execution settings so experiments can be tuned without changing the harness.
The CLI writes bounded elapsed-time progress to stderr while preserving stdout
for the final machine-readable result.

Run the preserved first case from the repository root:

```bash
node app-server/experiments/grok-review-harness/run-review.mjs \
  app-server/experiments/grok-review-harness/cases/claims-s6-candidate1.json
```

Run the same immutable packet with Gemini 3.7 Flash through Agy:

```bash
node app-server/experiments/grok-review-harness/run-agy-review.mjs \
  app-server/experiments/grok-review-harness/cases/claims-s6-candidate1.json
```
