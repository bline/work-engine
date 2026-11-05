# Local development: prerequisite repair budget

Recorded 2026-11-05. Status: **proposal; implementation and new live entry are
not approved**. Only the first round below is proposed for immediate approval.

## Proposed first round: 25 USD, two hours

Resolve the two component prerequisites for the
[local development integration plan](local-development-integration-and-qualification-20261105.md):
the unexplained CLI parallel-submit failure, and a successful bounded runtime
coding trial with a supported receipt acknowledgement path. Integration owns
the combined evidence and next readiness decision; component owners retain
their semantics and stores.

| Package, including its actors, review, repair and collection | Maximum USD | Accountable owner |
| --- | ---: | --- |
| CLI failure classification and bounded repair | 8 | CLI owner `01a1a84f-3760-7ce0-b46a-ad5b2f520c20` |
| Runtime receipt support and one seeded-source coding trial | 12 | S10 owner `01a1a2c0-7191-76f2-afa1-16d32d5c4883` |
| Integration preparation, source admission, test refresh, accounting and final handoff | 5 | This session `01a1a7ed-45f5-7041-9db5-a6c9aa1cd3f3` |
| **Total** | **25** | One all-actor accounting scope |

Units are API-equivalent dollars adjusted for subscription speed. The runtime
allocation includes the existing 0.70 USD provider-admission reservation; it is
not an assertion of measured billing. All proposal preparation is included once
in the integration allocation. Earlier closed scopes and their unused ceilings
are not transferred or reopened.

The two-hour execution window starts at approval. Productive source work ends
at approval plus 75 minutes or aggregate spend of 17 USD, whichever comes first.
The remaining 45 minutes and at least 8 USD protect review, bounded remediation,
collection and closure. The first **5 USD or 30 minutes**, whichever comes
first, must establish a supported receipt route and a concrete failure-diagnosis
route. A branch that lacks such a route stops with its exact blocker and a
revised estimate; independent admitted work need not be discarded. There is no
automatic extension, clock reset or new live retry.

## Evidence and scope

The starting shared revision is
`4b290756abb99bde34562eb5a7dd4b2855357c53`, which contains published CLI
`44630337` and the new actual Events HTTP MCP → command → Rust job owner →
fixture Host test. The latest fresh checks passed seven job tests, seventeen
Events tests, two selected AH839/Web285 browser scenarios, strict Clippy and
formatting. Those results retain their
[bounded offline standing](../records/local-development-system-qualification-20261105/README.md).

The earlier CLI run returned a missing operation ID during parallel repeat
submission; serial and later parallel runs passed. Its cause is still
unestablished. The CLI package captures the actual failing CLI exit, stdout,
stderr and owner response, distinguishes a test-fixture issue from an owner or
transport defect, and repairs only the demonstrated cause. Another passing
rerun by itself does not close that evidence gap. A material source change
requires source-bound regression and separate review before acceptance.

The [collected runtime owner decision](../records/local-development-system-qualification-20261105/S10-OWNER-DECISION.json)
confirms that the actual live run completed context reset and continuation but
changed no source text; its validator never entered. The producing
`coding-qualification` binary exposes only `--execute`. The generic ACK adapter
refused its binary identity. The runtime package establishes a supported
producing-owner receipt interface and a safe disposition of the historical
pending receipt, using existing Host/receipt owners. It does not edit journal
state or relax receipt, binary, job or authority identity checks. If historical
reconciliation remains unsupported, that receipt stays explicitly pending with
its existing owner; it cannot become an acknowledged result by inference.

Once the source, review and exact authority packet permit entry, this package
admits **one new bounded real-provider trial**. Its task packet includes the
exact seeded `src/lib.rs` contents. It preserves the selected model/provider,
one workspace call, one physical reset, two SDK sessions and four provider
requests, with fresh job/state and original acceptance tests. The new trial
must produce the intended source change, run and pass its validator, and close
its own receipt through the supported route. A failed live trial ends the live
portion; prior unknown or failed jobs are never replayed as retries.

This round does not redesign lifecycle or add a general authority service. If
receipt support needs a changed owning contract or the selected trial cannot
fit the current workspace/tool profile, the early gate returns that fact before
broader implementation. The exact executable, packet, profile, sources and
limits are rebound before any admitted run; budget approval alone is not a
runtime admission token.

The integration allocation includes refreshed affected checks on the actual
accepted revisions, explicit failed/skipped evidence and guarded publication
when source acceptance permits it. One ledger attributes each funded call once,
including setup, failed launches, reviews, repairs and reporting. The two
component allocations include their own review and collection; integration
does not charge those again.

## Roles and admission

Existing CLI and S10 owners remain accountable for their component handoffs.
This session owns the system consequence, dependency admission and accounting.
Actual implementation actors are selected after scoped context checks rather
than automatically reusing their large former contexts. The proposed builder
default is `gpt-6.1-sol` at `medium`; bounded source review uses a separate
read-only `gpt-6-astra` at `high`, standard tier, subject to the applicable
user/provider contracts. Root settings are preserved. Actual identities,
capability grants, actor ceilings, result collection and writable paths are
recorded before their corresponding work is admitted.

Expected writes are narrowly bounded CLI owner/client or fixture repairs and
the runtime producer/receipt adapter with their affected tests and documentation.
The first gate binds exact paths after classification. Source work stays in
isolated worktrees; shared foreign edits remain untouched. No component role is
launched and no previous grant is renewed by publication of this proposal.

## Following budgets: estimates, not current authority

| Milestone | Provisional amount | Condition for a firm bid |
| --- | --- | --- |
| Accepted local live job profile and first useful Work Engine development task through tests, review, acceptance and publication | 35–45 USD, approximately three hours | Accepted component handoffs above, an owned task-to-real-job contract and an actual task that fits its workspace/tool/recovery profile. Include an early contract feasibility gate. |
| Repeatable daily use, including further tools after recovery and selected claims access | Re-estimate after the first real task | The existing v4 profile intentionally disables tools after reset. Any extension needs its own versioned contract and observed consumer needs. |

The forecast through the first useful development task is therefore 60–70 USD,
subject to the second bid, and does not include the later daily-use expansion.
There is no spending authority for either later milestone in this first round.
The broader A2 connector policy, remote installation rollout, deployment and
full A5 conformance remain separate work.

Accepting this proposal authorizes only the bounded first round, with the
stated caps, gates and exact-run admission requirement. Changed component state,
a failed early gate or an exhausted allowance returns a truthful terminal and
the next decision; it does not silently expand the objective.
