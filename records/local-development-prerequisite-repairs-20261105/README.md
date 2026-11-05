# Local development prerequisite repair round

Recorded 2026-11-05. Status: bounded component results accepted by integration;
local source publication is identified in the publication receipt. This is the
result of the [approved prerequisite budget](../../planning/local-development-prerequisite-repair-budget-20261105.md),
not approval for the subsequent live job profile or daily development rollout.

## Results

| Objective | Result | Evidence |
| --- | --- | --- |
| CLI failure classification and bounded repair | Confirmed a test fixture publication race; repaired atomic publication, retained raw failure diagnostics and added an actual-writer interleaving regression | [CLI result](CLI-RESULT.json), [validation](CLI-VALIDATION.json), [Astra review](CLI-REVIEW.json) |
| One seeded live Pi trial | Intended edit, original independent validator, physical reset/recovery and successful producer acknowledgement passed | [Runtime validation](RUNTIME-VALIDATION.json), [process custody](RUNTIME-CUSTODY.json) |
| Integration and accounting | Exact component turns collected; source, authority and evidence bound; future failure qualification recorded separately | [Evidence manifest](EVIDENCE-MANIFEST.json), [accounting snapshot](ACCOUNTING.json), [future test plan](../../planning/pi-unattended-development-qualification-tests-20261105.md) |

### CLI result

The fixture previously redirected JSON directly into its live Host journal.
A deliberately paused actual writer reproduced an empty journal, CLI exit 1
and native `owner_error: custody_lost`, with no operation ID. This is causal
evidence for the controlled reproduction. The earlier unexplained intermittent
failure remains **unattributed**.

The repair stages output in a unique sibling file and renames it after success.
The same interleaving test then preserved the existing operation ID, complete
journal and one Host invocation. Production owner/client behavior is unchanged.
The existing Events HTTP MCP process test remains byte-identical.

Accepted private commit: `09b2fc72612568ff7b95dc5b05962e9e3548ae51` on
`263402b3aa6a130df3aa49470358dc4f0c9b88af`; integration cherry-pick:
`6ac90de1`. Exact file SHA-256:
`fd08beba343dbc79a63bc3ddf0656e138be69474e86c74e638d92637f6090981`.
A fresh Astra/high/default reviewer with observed read-only filesystem and
restricted network accepted with zero findings. The same reviewer resumed
after a cost-guard interruption; this is one retained review episode, with no
provider-independence claim.

Ten source-bound execution gates passed. Composition and bounded parallel
runs each passed eight tests; saved-output validation covered 209 responses.
The command-unit gate ran zero tests. Default server library and owner binary
runs each left one ignored test; the saved-output gate included its ignored
test. The reviewer inspected custody and source without executing the tests.
Pre-build binary pins alone did not attest execution; before/after captures
and later rebuilt pins supplied the bounded comparison identity. No failed or
blocked attempt is counted as a passing gate.

### Runtime result

The unchanged pinned producer
`ac1a380070af5c930aeab8ebf0808683c04815b78c7e4570f2205f0b48d3a193`
ran job `runtime-seeded-20261105-repair-01` in **50.083 seconds**, under a
239-second job deadline. Integration owned its one foreground launch and process
collection; the runtime owner subsequently validated the saved evidence.

- The sole source change replaced the minute multiplier `1000` with `60000`.
  Tool execution and the original independent validator each passed three tests.
- Pi SDK 1.5.0 and `gpt-6-luna` completed four actual subscription-provider
  requests, one workspace tool call, one physical SDK reset and two sessions.
- The successor started with zero messages and recovered the exact checkpoint;
  the continued parent reached `CompletedSettled`. Tools after reset and
  automatic compaction were disabled.
- The existing producing process acknowledged receipt
  `454ecac0450f5b42734434e9037a80bb3dcd388ce44169eafdc5340b58a586d0`.
  Delivery is acknowledged; receipt acceptance remains `owner_pending` and
  domain effect settlement is not claimed. No new ACK interface was necessary.
- Hash-bound process records show all three owned containers absent and all
  processes collected; projected access files were removed. Validation did not
  perform fresh Docker reinspection. The idle tool container's final exit 137
  followed explicit stop; tool execution, validator and producer exited zero.

The historical failed runtime receipt remains pending and untouched. Before
this trial, an admission check refused entry because a previously resolved
documentation-addition marker had not been cleared. No producer, provider or
Docker entry occurred in that refusal. The incident and its repair are retained;
there was one actual live trial and no retry.

The new budget amendment reduced runtime from $12 to $10. A separately hashed
activation variant tightened that guard and bound the exact approval; the
prepared packet, producer, source, validator and provider bounds stayed fixed.
Credential verification examined custody metadata; credential contents were
neither printed nor included in this record.

## Budget and limitations

Original approval: 2026-11-05 21:09:51.931 UTC, $25 total split CLI $8,
runtime $12 including a $0.70 provider reservation, integration $5. At
22:00:45.387 UTC the user prospectively approved CLI $8, runtime $10,
integration $7, keeping the total, one-trial limit and deadlines unchanged.
Productive stop remained 22:24:51.931 UTC or $17; hard stop remained
23:09:51.931 UTC. The earlier first gate was collected separately.

At amendment approval, recorded integration cost was **$5.1885686**, exceeding
its former allocation by **$0.1885686**. That excess is preserved in the
[boundary record](AMENDMENT-COST-BOUNDARY.json); the later approval does not
retroactively change the old cap. The CLI review guard similarly observed
$1.506578 against its initial $1.50 allocation; a prospective internal move
within CLI $8 admitted completion under $2, with final observed review $1.665652.

[Accounting](ACCOUNTING.json) reports completed-call speed-adjusted
API-equivalent inference cost, all component actors once, and the separately
held $0.70 product reservation. The runtime's pinned-catalog cost calculation
was 1,280 microUSD. Neither value measures subscription quota or final billing.
In-flight and closing tails remain attributable rather than assumed zero.

The two direct-user documentation additions were reconsidered cumulatively:
the [research note](../../notes/pi-harness-benchmark-evidence-20261105.md) and
the future unattended failure-test plan. Both were accepted as small within the
same total and clock; no A/B test or extra live experiment was admitted.

Graph coverage was content-verified for the relevant public Rust paths, with
unresolved calls. Exact source and hash-bound private evidence govern these
bounded conclusions; no exhaustive graph or call-chain completeness is claimed.

## What this enables next

The [previous system qualification](../local-development-system-qualification-20261105/README.md)
proved the actual Events/MCP → CLI → job-server path using a fixture Host. This
round separately proves a direct, real-provider seeded runtime trial. It has
**not** connected that whole caller path to a newly admitted live job profile.

The next implementation budget should cover that profile and its smallest
useful task, with independent acceptance and selected timeout, truncation,
interruption and replay cases from the future test plan. Runtime owns effect
and continuity behavior; job/acceptance owns admission, identity and verifier
semantics; integration owns the full caller path, accounting and evidence.
The earlier $35–45 / three-hour figure remains an unapproved estimate, requiring
a concrete scoped proposal before spending.

A prescribed single-line edit is not evidence of autonomous coding quality.
The reset was deliberately triggered at low measured usage, not natural context
exhaustion. Multi-step coding after reset, unattended liveness, arbitrary jobs,
real OAuth onboarding and daily-use development remain outside this result.
