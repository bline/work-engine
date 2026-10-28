# Targeted reconnaissance call

For the high-assurance route, after Codex provisionally selects a placement, invoke a new Claude process with no session persistence. Give it the original objective and work source, the placement candidates, the provisional certificate, rejected alternatives and reasons, and applicable repository instructions. Explicitly require it to assume the certificate may be wrong.
Provision the same read-only evidence capabilities as the placement call. If a
certificate clause depends on an external or unindexed artifact, independently
observe that bounded artifact in this fresh process instead of relying only on
the earlier call's interpretation.

For the direct route, invoke one compact process with the objective, supported boundary, provisional certificate, and applicable instructions. Require observed evidence for every certificate clause and require it to report any competing owner, hidden consumer, lifecycle conflict, or broader consequence. Discovery of any such condition escalates the route before implementation.

This call explores only the selected boundary and the callers or dependencies necessary to prove or falsify it. Require a JSON schema with:

- `verdict`: `confirmed`, `conflict`, or `unresolved`.
- `failed_certificate_clause`: the failed clause for a conflict, otherwise null.
- `facts`: at most eight directly observed facts that prove or falsify the certificate, including premise conflicts rather than silently redefining the task.
- `ranges`: at most twelve minimum code, test, and canonical-document ranges with path, numeric start/end lines, and reason.
- `wiring`: at most five registration, call-path, lifecycle, test-bootstrap, or environmental facts Codex cannot cheaply derive from those ranges.
- `commands`: exact vertical, precheck, focused, full-suite, and check-only freshness commands; do not execute them during reconnaissance.
- `blockers`: at most three unresolved decisions or missing-context risks.
- `statistics`: available call statistics.
