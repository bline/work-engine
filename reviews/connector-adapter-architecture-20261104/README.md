# Connector adapter plan review — 2026-11-04

**Outcome: Astra high accepted revision 4 with no blocking findings.** The
user-requested amendment adds an explicit thin Rust application-server package
and proposed ownership. Revision 3's mandatory Web replacement proof and the
earlier resolved findings remain intact. Review is not implementation or
deployment authority.

The [current plan](../../planning/connector-adapter-architecture-plan-20261104.md)
has SHA-256 `026fca8e02c317fa60997d20a04c71625f006e8194aad00584a751e634765da9`.
[The decision record](decision.json) binds that plan and its accepted review.
[The revision-3 decision](r3/decision.json) preserves its exact accepted subject.
[The revision-2 decision](r2/decision.json) preserves its exact accepted subject.

| Pass | Exact subject | Review | Result |
| --- | --- | --- | --- |
| Initial fresh review | [Revision 1](r1/subject.md) | [Report](r1/report.json), [provenance](r1/provenance.json) | Changes required: one P2 and one P3. |
| Retained re-evaluation | [Revision 2](r2/subject.md), [delta](r2/changes.diff), [repair dispositions](r2/repairs.json) | [Final report](r2/report.json), [provenance](r2/provenance.json) | Accept; both findings resolved; no new findings. |
| Web replaceability amendment | [Revision 3](r3/subject.md), [delta](r3/changes.diff), [scope](r3/amendment.json) | [Report](r3/report.json), [provenance](r3/provenance.json) | Accept; no findings. Mandatory Web presentation/transport substitution and signer boundary. |
| Shared application-server amendment | [Revision 4](r4/subject.md), [delta](r4/changes.diff), [scope](r4/amendment.json) | [Report](r4/report.json), [provenance](r4/provenance.json) | Accept; no P1/P2. Explicit AH host package, proposed ownership and unchanged active grants. |

The material repair defines authorization throughout asynchronous delivery:
token lifetime differs from a delivery grant, invalidation is ordered with
disclosure, restart cannot restore revoked access, and queued/replayed payloads
remain subject to current scope. The other repair gives enriched operation and
result views an explicit observation owner and requires parity between MCP and
CLI. The revision also assigns the Codex execution-port extraction and requires
caller identity to come from verified evidence.

Revision 3 requires the Web's existing read/review/signed-answer workflow without
Dot/MCP, a replacement presentation and a substituted local transport adapter.
Signing remains an explicit capability under the same verification contract;
RP/origin and trust pins are preserved. A5 now requires A4 for overall completion.
Already-admitted narrower query or Pi work retains its original scope. The review
notes preserve the existing ordinary-question/typed-task distinction, complete
review terms and history, and require fixture evidence to name which presentation,
transport and signer were present. Fixture success is not live qualification.

Revision 4 names AH as the shared Rust application-server package, with backend
integration proposed to own a separately admitted slice. It reuses existing
domain/runtime owners and requires process-level MCP/Web/CLI composition. The
review's three nonblocking notes concern exact per-operation runtime/profile
mapping, observable drain timeout/owner-unavailable outcomes, and local
caller/listener protection at the first checkpoint. None grants implementation
authority or changes another role's budget.

The initial reviewer was a fresh native top-level thread,
`01a1a810-adfd-75e0-b65b-75d5ac7d1870`, retained for the second and third passes. Requested
and recorded settings were `gpt-6-astra`, reasoning effort `high`, service tier
`default`; its recorded filesystem grant was read-only. The root independently
collected exact native final events and applied settings. Requested and observed
identity are retained in provenance rather than inferred from a role title.
Designer and reviewer both use OpenAI; no cross-provider review is claimed.

Revision 4 used a fresh replacement Astra high/default reviewer,
`01a1a857-cb13-7bd0-9d96-3d84c9d8770b`, because the earlier context had grown.
It reviewed this amendment and selected source, using earlier reports for
unaffected regions. Its native final, settings and read-only grant are recorded
in [revision-4 provenance](r4/provenance.json); uninterrupted continuity or a
repeated full-source audit is not claimed.

[Notification receipts](r4/notifications.json) record addressed board notice
4392 to backend integration, the auth/UI predecessor, the adopted CLI/query
root and S10. Native messaging also accepted notices for the three current
backend/query/runtime sessions. The completed predecessor was not reactivated.
Delivery acceptance is not a read acknowledgement or authority to expand work.

[The evidence manifest](evidence/manifest.json) binds 35 selected source and
handback snapshots. Work Engine discovery used coverage-aware graph retrieval;
external/private source was read directly. Clean coverage is best effort, and
handback outcomes remain attributed reports. No live connector, OAuth refresh,
Pi admission, confinement, provider execution or cutover was qualified by this
review. Documentation checks verified subject/snapshot hashes and local links;
implementation tests were not applicable to this documentation-only change.
