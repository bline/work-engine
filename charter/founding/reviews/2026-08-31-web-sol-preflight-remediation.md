# Web Sol Preflight 01 — Builder Assessment and Remediation

Status: local builder disposition; not reviewer re-evaluation or acceptance  
External review subject SHA-256:
`14b703dd2bcf61a82b6ea6fb2f290010a22fae3b79982e2650c872cf4217ae2d`  
Pre-remediation active seed SHA-256:
`2434560622809a8d3ec0e66521195fb121a2dc4480f5b67ee3239a10bbabc9d3`

## Applicability

Applicable. The reviewed package contains a system prompt and exact loading,
authority, and revision contracts for a prospective bridge model. The external
review is advisory and was supplied by the user; its provider identity and
unavailable `sandbox:` subjects were not independently verified here.

## AIR-WS01 — Temporary material control appeared as settled jurisdiction

Severity: high  
External finding: 1  
Outcome: `restate`  
Builder status: remediated; independent re-evaluation pending

The pre-remediation seed instructed the bridge to treat human statements as
inquiry “except when the human is explicitly exercising their authority over
scope, safety, resources, or real-world action.” Its consumer was the bridge
model at system-message precedence. The protected distinction is that material
control over resources and consequences must remain effective during founding
without converting that temporary fact into a permanent constitutional
allocation.

The old wording named a jurisdiction broader than its exposed causal basis and
bundled current control with future authority. The active seed now states the
material fact, why resource and real-world limits presently bind, and that the
future charter remains free to interpret or allocate authority differently.

## WS02 — Canonical active seed was absent from the reviewed packet

Severity: high  
External finding: 2  
Outcome: `bind exact subject`  
Builder status: mechanically remediated; package review pending

The repository had an active seed, but the supplied review packet contained
only v1. The finding is valid at the packet boundary. The current active seed
is `charter/founding/bridge-agent-seed.md`; v0, v1, and v2 are immutable lineage
artifacts. The genesis manifest must bind the active path and digest and include
the exact bytes in any review or launch packet.

## WS03 — Genesis dependencies lacked canonical artifact boundaries

Severity: high  
External finding: 3  
Outcome: `split and bind`  
Builder status: mechanically remediated; manifest validation pending

The following separately hashed artifacts now exist:

- `genesis/opening-question.txt`;
- `genesis/runtime-manifest.json`;
- `genesis/initial-authority-state.md`;
- `genesis/event-record-v1.schema.json`;
- `genesis/seed-revision-protocol.md`;
- `genesis/assurance-profile.json`.

Their existence does not freeze or validate the genesis bundle. That occurs
only when an exact manifest binds their digests and the accepted revision is
committed before launch.

## WS04 — Seed revision semantics were undefined

Severity: high  
External finding: 4  
Outcome: `define prospective transition`  
Builder status: remediated as proposed bootstrap mechanics; community authority
unresolved

`genesis/seed-revision-protocol.md` now distinguishes criticism, proposal,
participant judgments, acceptance, and context transition. A successor starts
a new model context, receives prior events as attributed history rather than
private memory, and cannot impersonate the earlier instance. The protocol does
not grant permanent amendment authority; a future charter may replace it only
prospectively.

## WS05 — No live external witness exists

Severity: critical for a highest-assurance claim  
External finding: 5  
Outcome: `retain open limitation`  
Builder status: open

No external commitment witness has been selected, and none can be retrofitted
after the first request. `genesis/assurance-profile.json` therefore classifies
the package as `operator_controlled`, forbids a highest-assurance claim, and
marks launch readiness blocked pending a witness decision and genesis freeze.
A local hash, Git commit, or self-signature must not be represented as closing
this finding.

## Validation boundary

Local validation may establish byte identity, JSON/schema syntax, internal
digest agreement, and absence of duplicate canonical paths. It cannot establish
the reviewer's acceptance, the external witness that remains absent, fairness
of the procedure, or successful community constitution.
