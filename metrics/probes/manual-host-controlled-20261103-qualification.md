# Host-versus-manual pilot qualification, 2026-11-03

**The prospective protocol passed review; the current launch path did not
qualify for trial entry. No treatment was allocated and no trial episode ran.**
There is no new efficiency estimate from this work.

The owner approved the exact registered packet and $200 maximum at
2026-11-03T19:43:10.031Z. The
[approval record](../../records/manual-host-controlled-pilot-approval-20261103.json)
binds the source message and hashes. The approved plan limits setup and
qualification to $20 and explicitly stops qualification when the existing route
cannot establish the required separation without building a new isolation
platform. That condition, rather than a budget exhaustion or rejected design
review, stopped admission.

The [machine-readable result](manual-host-controlled-20261103-qualification.json)
and [bounded probe evidence](manual-host-controlled-20261103-qualification-evidence.json)
preserve the observations and limitations. The frozen preregistration remains
unchanged; its original pending-funding fields are historical. The later approval
and this result own the subsequent state.

| Qualification item | Outcome |
| --- | --- |
| Registered inputs, exact subject and tool pins | Verified; 44 bundle files |
| Fresh Astra/xhigh/default protocol review | Ready, no material findings; accepted for protocol design |
| Reviewer exact-turn identity/settings and native filesystem read-only policy | Verified before source gate |
| Per-run readable-path allowlist | Unestablished on the frozen launch route |
| Fully frozen startup instructions | Incomplete: a global AGENTS source was loaded |
| Actual tool/connector/network allowlists | Unestablished for the registered experiment |
| Allocation and episode entry | Not performed |

The [review result](../../reviews/manual-host-controlled-pilot-20261103/protocol-review-result.json)
and [provenance](../../reviews/manual-host-controlled-pilot-20261103/protocol-review-provenance.json)
are preserved byte-bound to the exact subject. The reviewer cost $2.970928 of
its $4 allowance. Its completed, unloaded turn was collected through the pinned
host, settings matched, and the task was closed and acknowledged. That evidence
establishes neither wider tool isolation nor trial qualification.

## Runtime observations

A deterministic probe used study-owned sentinel files and the same legacy
`readOnly`, network-disabled sandbox shape returned by the native launch:

- Reading the declared allowed sentinel succeeded, exit 0.
- Reading the excluded sentinel also succeeded, exit 0.
- Writing to the excluded location failed with a read-only-filesystem error,
  exit 1.

This was the native command executor under that policy shape, without a model
turn. It was not a probe of every connector or a demonstration of an escape
from a configured read allowlist: **no such allowlist was established by this
legacy policy**. It shows why a native read-only write restriction cannot stand
in for the registered readable-input restriction. The sentinel contained only
a controlled public test string.

The actual launch also reported loading `/home/bline/.codex/AGENTS.md`, whose
SHA-256 was absent from the registered environment. The repository AGENTS file
matched its registered hash. Merely copying the global file afterwards would
not prove that a future arm loaded only its materialized frozen inputs.

A bounded compatibility check then supplied a named read-restricted profile in
per-thread configuration. With the frozen route's explicit legacy sandbox
selection, the response still reported legacy read-only and no active named
profile. The direct named-profile selector was rejected because the ordinary
client connection had not negotiated `experimentalApi`. No model turn was
started by these probes; the one empty thread created was archived.

These observations agree with the documented precedence of legacy sandbox
selection over default permission profiles. Named profiles provide more
specific filesystem rules, but require a compatible selection path.
[Official OpenAI permissions documentation](https://learn.chatgpt.com/docs/permissions).
Filesystem restrictions also leave HTTP MCP traffic under separate controls.
[Official OpenAI evaluation example](https://developers.openai.com/cookbook/examples/partners/harness_aware_plugin_evals/harness_aware_plugin_evals).

The frozen host request carries a `sandbox` string and its native preparation
always emits `-s` through `role_host`; the current request does not select a
named native permission profile. No global managed-policy or user configuration
was changed to override this. This is a finding about the exercised frozen
route, not a claim that Codex cannot provide isolation through other qualified
realizations.

## Cost and accounting

At **20:22:31Z**, the canonical reader observed **$7.6849086** in modeled,
speed-adjusted API-equivalent cost since the original preparation boundary:

| Actor | Calls | Observed cost |
| --- | ---: | ---: |
| Study controller: preparation and qualification | 56 | $4.7139806 |
| Fresh protocol reviewer | 20 | $2.9709280 |
| Total | 76 | $7.6849086 |

There were no unpriced calls in that snapshot. Later controller reporting and
closure calls are a separate tail; this is not a settled bill or an exact final
total. The twelve nominal episodes, retry, quality-scoring and comparison-analysis
allocations were unused. The small probe threads produced no model calls.

Preparation before scope initialization was preserved, not reset. The frozen
pre-init charge is $1.7109256, already included in the controller row above.
Because default runtime control starts at initialization, its aggregate ceiling
was tightened to $18.2890744 and the controller ceiling to $14.2890744, keeping
the unchanged $20 qualification and $16 controller authorizations inclusive of
that earlier cost. The offset must not be added a second time to the complete
cost tree, whose explicit start is 19:18:20.262Z.

## Remaining boundary

Before this comparison can run, it needs a qualified common launch path with
the required read/tool/network restrictions and a complete frozen startup-input
inventory. That realized path must be bound prospectively before allocation;
changes to the accepted arms or source pins require the registration's own
revision and approval-applicability decision. A ready protocol review cannot
supply missing runtime evidence.

The registered six task pairs remain unsolved and unallocated. The qualification
review is not a seventh task or an experimental observation. No acceptance
threshold was weakened, no role was silently substituted, and no production
source, global configuration, skill deployment or peer work was changed. The
bounded review is collected; remaining experiment conditions are recorded as
unestablished rather than successful.

The qualification budget record is closed as abandoned because the registered
execution prerequisite is unmet. The deadline notification was cancelled after
closure. No owned model collector or schedule listener remains. Observed
controller reporting calls through the final snapshot are attributed explicitly;
subsequent reporting tails remain unsettled.
