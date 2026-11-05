# First complete local development workflow

Recorded 2026-11-05. Status: first joined offline system test implemented;
full development acceptance is **held**. This session owns the system test and
integration decision. Component owners retain their contracts and stores.

The user said “Let's proceed” at 19:07:26.983 UTC after the ownership proposal.
The supervisor set an initial limit of 20 API-equivalent USD adjusted for
subscription speed and two hours, including preparation. The absolute deadline
is 21:07:26.983 UTC; the first checkpoint is 6 USD or 19:52:26.983 UTC. This
preparation scope does not reactivate closed component budgets or admit a new
live-provider run. Its durable assignment and accounting are in
`/home/bline/.local/state/work-engine/local-development-integration-20261105T1907/`.

## Outcome

Complete one useful, explicitly authorized Work Engine development task through
its real supported runtime: request, authorization, edit, original tests,
recovery where claimed, independent review, acceptance and guarded Git
publication. Preserve exact operation identity and unresolved effects through
restarts. An isolated disposable-crate pass is a runtime prerequisite; it is
not acceptance of this complete development outcome.

The current [roadmap direction](../roadmap.md#current-strategic-route) already
selects one complete engineering job using existing Rust owners. This plan
selects its immediate qualification gates. It does not add a workflow store,
replace lifecycle authority or make a permanent external supervisor part of
the runtime architecture.

## Current source and fresh evidence

The preparation branch starts at `d44b1bbfafa9f78e4ab65ae732c2597501518da4`.
After the CLI owner completed publication, this branch admitted
`4463033789c905f80b59c6c59d4cbbe6e22192fd` for the joined process test.
The [bound candidate](../records/local-development-system-qualification-20261105/candidate.json)
records exact subjects, custody digests, failures and limitations.

| Component | Current standing and consequence |
| --- | --- |
| AH server and Web | Accepted selected AH `839b8c4e` and Web `28563885` synthetic composition. Fresh `common-disabled` and `common-mcp` scenarios both passed, including actual browser and shared query/task consumers. These are the exact older selected binaries and private source worktrees, not qualification of every later shared-branch change. |
| Events coding-job consumer | Accepted and merged in `d44b1bbf`, disabled by default. Fresh 17/17 tests passed with the real private command binary against a fake HTTP backend. This establishes transport/validation behaviour, not native coding execution. |
| Coding CLI and job API | CLI owner published `44630337`, integrating reviewed `b7ddfd4c` and the accepted Events owner's compatibility pin. Nine publication checks passed, including six parallel job-composition tests. The earlier repeated-submit failure remains unexplained; neither publication nor another passing run settles its cause or qualifies live activation. |
| Joined process test | The added Rust integration test sends actual HTTP MCP requests through the Events process, command process and real Rust job owner to its existing fake Host. It checks authentication presence, the four-tool surface, exact operation identity after a completed retry and server restart, native credential denial, one Host entry and unchanged pending acceptance/settlement. The verifier and Host remain explicit fixtures. |
| S10 continuity | Source `ea337a46`, shared merge `d5666c93`, is accepted. The actual live trial completed four provider requests, one tool call, two SDK sessions and one physical reset with exact recovery. The edit matched no source text, coding failed, and the validator did not enter. The owner confirmed that the producing binary exposes only `--execute`, with no supported ACK command; a compatible historical-receipt reconciliation route is unestablished. |
| Claims/tool continuation | The owners agreed that a later claims lane may add a separate opt-in profile. Existing v4 remains one initial workspace command with tools disabled after reset. Claims reads and further coding after reset are not implicitly enabled. |

All three original baseline custody runs exited zero with no remaining processes. Their
private records and source/binary inputs are referenced by the candidate.
The 102-entry historical AH candidate was hash-checked before replay; each
scenario created fresh disposable owner state and browser signatures. Historical
signed answers or stores were not reused as authority.

The [record and reproduction command](../records/local-development-system-qualification-20261105/README.md)
describe the new test, its final checks and the two initial harness failures.
The [collected S10 owner decision](../records/local-development-system-qualification-20261105/S10-OWNER-DECISION.json)
confirms that a next bounded pilot should include the exact seeded source while
retaining one tool call, one reset, two SDK sessions, four provider requests and
the 0.70 USD provider reservation. That consultation admitted no new run.

## Ownership and dependencies

| Gate | Accountable session | Required handoff |
| --- | --- | --- |
| D1: accepted coding-job path | CLI `01a1a84f-3760-7ce0-b46a-ad5b2f520c20` | Publication and compatibility pins are now available at `44630337`. Capture actual exit/stdout/stderr and owner response for the earlier parallel repeat failure; classify or repair it with source-bound evidence and owner acceptance. Another passing run alone does not establish the failed case. |
| D2: successful bounded runtime | S10 `01a1a2c0-7191-76f2-afa1-16d32d5c4883` | A newly admitted, exact seeded-source trial that produces the intended edit, passes the original validator and reconciles the producing Host receipt. Preserve the earlier failure and outstanding acknowledgement. |
| D3: task-to-real-job contract | Backend `01a195d4-b7a7-77f2-ba65-277669c0ec97`, with the S10 authority owner | An accepted operation/profile binding for the chosen real runtime. The current Events/CLI contract is `coding-job-offline-fixture-v1`; replacing its fake Host or setting an enable flag cannot qualify a live profile. AH owner `01a1ac61-2153-7271-b6cd-51551c318a78` owns any required thin server transport composition. |
| D4: complete task acceptance | Integration `01a1a7ed-45f5-7041-9db5-a6c9aa1cd3f3` | Select a useful actual Work Engine task that fits the admitted workspace/tool profile, bind all revisions and gates, collect the complete chain through review and publication, and report its actual readiness. |
| Later daily use | Integration, with claims `01a1a3dc-57d5-7273-bd12-86ae25aed635` and lifecycle owners | Qualify iterative tools after recovery and useful claims access under a separately versioned contract. Preserve all existing profiles and grant semantics. |

These are ownership and required-outcome proposals, not new implementation or
funding grants for the listed sessions. The integration scope funds only its
explicitly bound owner consultations. The CLI's active publication remains
under its separate direct user grant. No duplicate CLI patch is assigned here.

D1 and D2 may proceed independently under their own assignments. D3 consumes
their exact supported contracts. D4 consumes the accepted joins. A smaller
direct local runtime trial can establish D2 before the complete operator path
exists; its result retains that narrower standing.

## Qualification matrix

| Test | Required observable result |
| --- | --- |
| Fresh local startup | Exact selected executable/config/profile/source pins; separate liveness, owner readiness and writable capability; unsupported combinations refuse startup. |
| Request and authorization | CLI/MCP requests and the selected Web review refer to the same task, request and exact terms. Transport credentials and stored signatures do not mint execution authority. |
| Actual development | The model edits the admitted real task workspace; the original acceptance tests run against exported bytes. A zero-match edit or missing validator is a failed task. |
| Recovery | Committed checkpoint and exact task/result references survive the claimed restart/reset; no unapproved additional tool access or replay. |
| Repeat and lost acknowledgement | Exact retries observe one operation. Lost responses cause owner readback; unknown execution is retained rather than resubmitted with a new ID. |
| Cancellation and limits | Admission, consumption-time expiry/revocation and budget controls preserve their owner meanings; timeout is not proof of cancellation or settlement. Owned processes are collected. |
| Review and publication | The actual task output is the reviewed subject. Explicit acceptance and the existing guarded publication owner consume that subject; validator completion does not stand in for review or publication authority. |

Select a task only after its actual repository/workspace, tool count, test
toolchain and recovery needs fit the supported profile. If that requires a new
profile, place it with its existing owner and estimate that bounded extension.
No arbitrary-shell or universal workflow framework is implied.

## Next operating decision

The first checkpoint succeeds when an exact runnable route can support D4 with
current accepted contracts and its remaining work fits the protected budget.
Otherwise it returns the concrete missing joins and owner assignments before
new implementation or live entry. Preparation and passing component baselines
do not silently change any held gate to ready.

Remote multi-installation access continues under the separately reviewed
[A2 design](a2-connector-identity-and-policy-design-20261105.md). It need not
precede a properly bounded single-operator local trial. OAuth issuance/refresh,
remote deployment, user-state migration, hostile same-UID isolation and full
standalone release remain outside this preparation evidence.

Reconsider this candidate after the CLI publication/qualification result, a
successful S10 coding/receipt result, a changed task-to-job contract or a task
that cannot fit the supported tool/workspace profile. Bind actual new inputs
and refresh affected evidence at that time; retain these dated results.
