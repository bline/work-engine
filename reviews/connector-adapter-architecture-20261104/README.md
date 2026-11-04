# Connector adapter plan review — 2026-11-04

**Outcome: Astra high accepted revision 3 with no findings.** The user-requested
amendment makes Web-channel replaceability and conformance mandatory. Revision 2
remains an accepted historical subject, with both original findings resolved.
This is plan review, not implementation validation or deployment authority.

The [current plan](../../planning/connector-adapter-architecture-plan-20261104.md)
has SHA-256 `360e8fee417efe52dbab03489b5ab7ff21a808e854f9e41f06694a5d69fea65e`.
[The decision record](decision.json) binds that plan and its final review.
[The revision-2 decision](r2/decision.json) preserves its exact accepted subject.

| Pass | Exact subject | Review | Result |
| --- | --- | --- | --- |
| Initial fresh review | [Revision 1](r1/subject.md) | [Report](r1/report.json), [provenance](r1/provenance.json) | Changes required: one P2 and one P3. |
| Retained re-evaluation | [Revision 2](r2/subject.md), [delta](r2/changes.diff), [repair dispositions](r2/repairs.json) | [Final report](r2/report.json), [provenance](r2/provenance.json) | Accept; both findings resolved; no new findings. |
| Web replaceability amendment | [Revision 3](r3/subject.md), [delta](r3/changes.diff), [scope](r3/amendment.json) | [Report](r3/report.json), [provenance](r3/provenance.json) | Accept; no findings. Mandatory Web presentation/transport substitution and signer boundary. |

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

The reviewer was a fresh native top-level thread,
`01a1a810-adfd-75e0-b65b-75d5ac7d1870`, retained for the second and third passes. Requested
and recorded settings were `gpt-6-astra`, reasoning effort `high`, service tier
`default`; its recorded filesystem grant was read-only. The root independently
collected exact native final events and applied settings. Requested and observed
identity are retained in provenance rather than inferred from a role title.
Designer and reviewer both use OpenAI; no cross-provider review is claimed.

[The evidence manifest](evidence/manifest.json) binds 35 selected source and
handback snapshots. Work Engine discovery used coverage-aware graph retrieval;
external/private source was read directly. Clean coverage is best effort, and
handback outcomes remain attributed reports. No live connector, OAuth refresh,
Pi admission, confinement, provider execution or cutover was qualified by this
review. Documentation checks verified subject/snapshot hashes and local links;
implementation tests were not applicable to this documentation-only change.
