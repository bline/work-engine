# Rust programme inference cost, measured and projected

What the Rust work has cost in inference, and what the work still planned
should cost, denominated in weekly Codex quota. Measured 2026-10-18 from local
rollouts. Scope is the Rust campaigns in `docs/rust-*.md` and
[`experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md`](../experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md),
both what is implemented and what is currently planned. **The migration track
(`rust-migration-plan.md`, `rust-host-replacement-plan.md`) is out of scope by
instruction.**

The forward half is a forecast, pre-registered here so it can be scored rather
than reconstructed. The retrospective half is measurement.

## Units

A quota point is 1% of a weekly Codex window and is read directly from
`rate_limits.primary.used_percent` in the rollouts, so costs stated as a share
of a window carry **no multiplier assumption**. The plan tiers are Pro 100 =
5x, **Pro 200 = 10x** (this account) and Pro 500 = 25x, which also retires the
"assumed" qualifier the efficiency ledger has carried on Pro = 10x: the frozen
baseline independently verified 5x for what was then the Pro 100 era.

| unit | value |
|---|---|
| 1 Pro-200 week | 100 points = 1,000 Plus-units = **~162.3 Mweighted tokens** |
| 1 Pro-100 week | ~81.2 Mweighted |
| 1 Pro-500 week | ~405.8 Mweighted |
| weighted token | `uncached + cached/10 + 5 x output` |
| coefficient | **6.16 Plus-units per Mweighted**, fitted on 93 unclipped points over 151.0 Mweighted in the 10-15 window |

Window-to-window scatter on the coefficient is about +/-25% (6.16, 6.42, 6.81,
8.07 across four unclipped Pro-era windows), so treat every figure below as
carrying that band before any other uncertainty.


## Reading these numbers

Every figure here is one of three kinds, and the distinction is load-bearing:

- **Measured** — derived from rollout telemetry or repository state, reproducible
  by a named script.
- **Estimated** — derived from measured quantities under a stated assumption,
  which is given with the number.
- **Projected** — a forecast, or a saving that has not occurred. Each carries the
  prerequisite it is waiting on.

**The three classes are not sufficient, as this conversation demonstrated three
times.** A figure can keep its class label and still lose the relationship that
made it meaningful. Each of these happened within hours of the classes being
written down:

| restated as | actually | the slip |
|---|---|---|
| "62% of the programme" was rework | 49.3% of 953.3, and the iteration count never established iteration cost | wrong denominator, plus an assumption presented as a measurement |
| M22 delivers "~30% savings" | -30%, the measured *penalty* at today's 116K floor | sign inverted; a cost became a benefit |
| "56% projected efficiency improvement" | spend falling *to* ~56% (a 44% reduction) in a scenario model explicitly marked non-additive and untested; or the 56% throughput cut required to fit Pro 200 after 11-01 | reduction *to* read as reduction *by*, and a constraint read as a benefit |
| three absent scalar constraints in the RP2 contract | one genuinely absent; one whose magnitude is stated at contract line 291 as "1 MiB result/observation"; one misdescribing `INPUT_LIMIT`, a multiplier base used as `128 x` and `64 x`, as a 4 MiB combined cap | an absence search tested *spellings and constant names*, not magnitudes in prose units, and a self-reported claim was never checked against the code it described |
| "no 4 MiB combined input cap exists" | `startup.rs:357` enforces `compiler_document + compiler_options + input > INPUT_LIMIT`; the cap is real and the original report was right | an absence claim drawn from a grep truncated by my own `head -8`, which cut off the four lines containing the refutation |
| "all four arms inverted F2, citing defective code as exemplary" | one arm of four made a structural reference to the enum variant; one never mentioned it, one used it in a remediation, one cited its definition; the arm closest to the claim also filed a finding *against* that same process-side labelling | a four-model symmetry built from one sentence, then sharpened by a second reader into "treating its implementation as an example to follow" |

The third propagated into a derived figure before being caught: a 2.27x capacity
multiplier, which assumed spend falling to 44%. The documented scenario gives
**1.79x** (1 / 0.56). Neither should be booked, because the underlying ~56% is a
non-additive prediction and not a result.

So a figure stated anywhere downstream of this note must carry:

1. the exact quantity **and its denominator**;
2. whether a percentage is a reduction **by** or **to**;
3. its basis - measurement, scenario, or forecast;
4. the conditions under which it holds;
5. the authoritative source **and revision**.

That list is not a new invention. It is what
[`evidence-and-claims.md`](../app-server/docs/architecture/evidence-and-claims.md)
already specifies - "a claim is never the thing it is about; it is a
materialization, a revision-bound, attributed copy of a fact some other
authority already established."

**Correction, and it is the fourth instance of the failure described above.** An
earlier version of this paragraph said that dimension was "designed and unbuilt,"
citing `authorization: design_work_authorized` / `implementation: none`. Wrong:
that block is §6's `status_override`, scoped to the *refresh extension*. The
page default, which governs §2-§5, is `design: accepted`,
`authorization: implementation_authorized`, **`implementation: implemented`** -
and §2 is titled "The Implemented Substrate Today." Verified in code: six
operations in `app-server/src/services/claim-evidence/service.mjs`
(`create_claim`, `publish_revision`, `publish_lineage`, `record_reliance`,
`retire_reliance`, `retract_revision`), three profiles in `contract.mjs`
(`proposal-research-v1`, `revision-bound-review-finding-v1`,
`production-path-v1`), and a mechanically enforced non-authority boundary that
throws `"production-path claim is self-authorized"` in
`production-path-contract.mjs:62`.

I read a component's status block as the dimension's status - a scope error, in a
correction about scope errors, against a page whose own status section says §6,
§7 and §8 "**must not be read as sharing this default**." The information was
labelled, scoped, and accompanied by an explicit warning, and it still did not
survive restatement. That is the strongest available evidence for the point this
section is making: understanding an invariant is not the same as enforcing one.

**What this changes practically.** The substrate for attributed, revision-bound
claims exists and runs. What is missing is a *domain profile* for quantitative
planning claims - §7's `planning-facts-v1` territory, which is `design: proposed`
/ `authorization: exploration_only` / `implementation: none`. So the available
move is a new profile on an implemented substrate, not a new mechanism, and the
five fields above are the profile's candidate schema: quantity with denominator,
direction (`reduction_by` versus `reduction_to`), basis, conditions, and
source revision. Derived values would carry lineage to the parent revision via
`publish_lineage`'s existing `derivation` edge, so a figure like the 1.79x
multiplier inherits its parent scenario's unvalidated status instead of
becoming an independent performance claim.

Carrying the five fields by hand remains the interim discipline, and this
section's own correction history is the argument for not leaving it there.

A number does not change class by being repeated. One figure in an earlier draft
of this note crossed from estimate to asserted fact between sections; it is
retracted below, with the reason, rather than quietly corrected.

## The 28-day promotion changes capacity, not price

OpenAI is running a 28-day campaign from **2026-10-05 to 2026-11-01**, giving a
feature or a quota reset most days. Today is day 14. **Eight resets are visible
in the rollouts** between 10-05 and 10-17, each starting a fresh 7-day window
from that moment with `used_percent` dropping to zero and `credits` remaining
`{"has_credits": false, "balance": "0"}` throughout.

This matters for reading any quota figure. It is a capacity gift, not a price
change: within each uninterrupted window `used_percent` is still monotone and
the coefficient above holds. Two readings it invalidates:

- Summing per-window spans across October over-counts; October received roughly
  seven windows of allowance, not the 2.6 a fixed weekly cycle would give.
- A sliding-window interpretation is **disproved**, not merely unused:
  regressing `used_percent` on trailing spend gives R2 between -0.5 and 0.0 at
  every lookback from 1 to 7 days.

## Measured: what the Rust programme has cost

2026-10-01 to 10-19, 198 threads after deduplicating the archived store against
the live one, **953.3 Mweighted total**. Root orchestration is 187.9 Mweighted,
**24.5% of all non-root work**, allocated back pro rata at a factor of 1.245;
that corroborates the efficiency ledger's standing "orchestration adds ~25% on
top" for M14 from an independent direction. Excluded from the Rust total:
schema14 (36.5, not Rust), coordination and readiness (23.0), unclassified
(8.5).

| slice / track | measured Mwt | + root share | Pro-200 weeks | status |
|---|---:|---:|---:|---|
| S0 foundation | 9.4 | 11.7 | **0.072** | done |
| S1 core + store | 12.8 | 15.9 | **0.098** | done |
| S2 async execution | 8.9 | 11.1 | **0.068** | done |
| S3 wire + client | 8.0 | 10.0 | **0.061** | done |
| S4 first service | 18.1 | 22.5 | **0.139** | done |
| S5 native provider | 64.4 | 80.2 | **0.494** | **reopened, active in A3** |
| S6+S7 semantic + admin | 48.6 | 60.5 | **0.373** | done |
| S8 retained workflow | 49.5 | 61.7 | **0.380** | done |
| S lifecycle shared | 8.3 | 10.3 | **0.064** | done |
| S9 fresh start + deferred migration | 109.9 | 136.9 | **0.843** | **active** |
| S10 replacement qualification | 33.1 | 41.2 | **0.254** | **active** |
| C compiler C0–C2, C4 | 48.4 | 60.3 | **0.371** | C3 open, C5 planning in A4 |
| R review episode R0–R2 | 26.6 | 33.1 | **0.204** | R3 open |
| RP review provider RP0–RP3 | 85.1 | 106.0 | **0.653** | **active in A2** (gate fix, live review) |
| SC slice campaign SC0–SC3d | 48.9 | 60.9 | **0.375** | SC3 remainder open |
| J owner join + completion | 67.7 | 84.3 | **0.520** | done |
| W workspace W0–W1 | 14.4 | 17.9 | **0.111** | **active in A4** (disk recovery, gate fix) |
| CE claim evidence | 10.8 | 13.5 | **0.083** | done |
| HP host HP1–HP3 | 9.3 | 11.6 | **0.071** | done |
| P replay regression | 7.2 | 9.0 | **0.055** | done |
| E0 / ER evidence | 6.2 | 7.7 | **0.048** | done |
| UI terminal | 1.7 | 2.1 | **0.013** | UI0 active in A3 |
| **Rust programme total** | **697.3** | **868.5** | **5.35** | |


Attribution is by `session_meta.agent_path`, which names the ordinal directly
(`rust_lifecycle_s4_20261007`, `rp1_sol_implementation`, `sc3d_sol_implementation`),
so it is exact rather than inferred from time intervals — interval attribution
fails here because several campaigns ran concurrently.

**Shape of the measured distribution.** Pure core slices cluster tightly at
**0.06-0.10 Pro-200 weeks** (S0 0.072, S1 0.098, S2 0.068, S3 0.061). Anything
that crosses a host, provider or evidence boundary costs three to eight times
that: S4 0.139, S6+S7 0.373, S8 0.380, S5 0.494, J 0.520, RP 0.653. The
cost driver is boundary crossing and review load, not lines of code — which is
why lines are reported nowhere in this note as a denominator.

## Planned: what remains

Open ordinals are those with a plan document and no matching result or
validation document, plus the two lifecycle slices in flight. Migration
excluded.

| open item | plan | analogue | spent | **remaining, Pro-200 weeks** |
|---|---|---|---:|---|
| S9 fresh start + deferred migration | plan §S9, 93 lines | in flight, largest single consumer | 0.843 | **0.25 – 0.90**, central 0.50 |
| S10 replacement qualification | plan §S10, 25 lines | S8 0.380 plus review-heavy RP 0.653 | 0.254 | **0.40 – 1.00**, central 0.65 |
| C3 compiler | `rust-compiler-c3-plan` 406 lines | C2 at 0.284 (plan 399 lines) | — | **0.10 – 0.30**, central 0.20 |
| C5 compiler | `rust-compiler-c5-plan` 185 lines | roughly half C3 | — | **0.05 – 0.15**, central 0.09 |
| R3 review episode | `rust-review-episode-r3-plan` 458 lines | R2 at 0.149 (plan 447 lines) | — | **0.10 – 0.25**, central 0.16 |
| SC3 remainder | `rust-campaign-sc3-remediation-plan` 330 lines | SC3d already done at 0.159 | — | **0.08 – 0.25**, central 0.15 |
| **total remaining** | | | | **1.08 – 2.85, central 1.75** |

Ranges come from the measured spread of comparable completed ordinals, scaled
by plan length only where a same-track analogue exists. They are not derived
from a model of the work.


## Execution runs as four parallel avenues

The programme is not executed slice by slice; it runs as four concurrent root
sessions, deliberately parallel so that the promotional resets are absorbed
before they expire. Measured over the common window 2026-10-15 18:04 to
2026-10-19 03:31 (3.39 days):

| avenue | root | work | Mweighted | Pro-200 weeks | duty cycle |
|---|---|---|---:|---:|---:|
| A1 | `01a14128` | lifecycle S9 + S10 | 122.5 | 0.755 | 52% |
| A2 | `01a140bc-8707` | review provider RP0–RP3 | 94.2 | 0.580 | 67% |
| A3 | `01a140bc-d873` | S5 native provider + UI0 | 47.5 | 0.293 | 25% |
| A4 | `01a14d30` | workspace W1 + compiler C4/C5 | 25.7 | 0.159 | burst |
| **combined** | | | **289.9** | **1.79** | |

Duty cycle is distinct clock-hours containing at least one call, over the
avenue's own span. Combined sustained rate is **85.4 Mweighted/day against a
gift rate of ~100.6, or 85% utilisation.** Four avenues is close to the right
number: a fifth would exceed the reset cadence and buy time at 99% rather than
throughput. The remaining 15% is duty cycle, concentrated in A3.

## Reconciliation: the forecast is low, and by how much

Projected to 11-01 at the observed rate, the four avenues consume **1,196
Mweighted = 7.4 Pro-200 weeks**, against ~8.7 weeks of gift — **85% of
promotional capacity**. The per-ordinal forecast above totals 175-463
Mweighted, which at 85.4 Mweighted/day is only **2.0 to 5.4 days** of these
avenues running against 14 days remaining.

Both cannot be right. The burn rate is the better evidence, and the forecast is
low by roughly three to seven times. **The cause is that "done" is not
terminal and the analogue method priced first passes.** S5 is recorded above at
0.494 weeks and marked done, yet A3 has spent a further 0.293 weeks on it since
10-15 across `s5_attempt`, `s5_perf`, `s5_graph` and `s5_repair`; W1 is running
`w1_disk_recovery` and `w1_gate_fix`; RP is running `rp2_root_gate_fix` and
`rp3_astra_live_review`; C4 has a result document and `c4_builder` ran anyway.
Reopening and remediation are the dominant missing term, consistent with the
gpt-6-era measurement of 5.2 review findings and 2.6 fix iterations per slice.

**Revised reading.** Treat the per-ordinal table as a floor on first-pass cost,
not an estimate of completion cost. The defensible forward statement is the
burn-rate one: at the current configuration the programme consumes most of the
promotional gift by 11-01, and fits only while resets continue at the observed
cadence. Scoring after 11-01 should compare both methods, since the gap between
them is the measurement worth keeping.

## What that costs against the quota

Observed burn is **53.0 Mweighted per day**, or **2.28 Pro-200 weeks of quota
per calendar week** — a rate that only the promotion has made affordable.

| scenario | capacity | remaining programme at central 1.75 weeks |
|---|---|---|
| **Promotion, 10-18 to 11-01** | ~9.6 more windows at the observed 0.62 resets/day = **~1,560 Mweighted over 14 days** | per-ordinal forecast says 18%; **observed burn says 85%** — see Reconciliation |
| Pro 200 after 11-01 | 162.3 Mweighted per calendar week | **~1.8 calendar weeks of the entire quota**, with nothing left for anything else |
| Pro 100 after 11-01 | 81.2 Mweighted per week | ~3.5 calendar weeks |
| Pro 500 after 11-01 | 405.8 Mweighted per week | ~0.7 calendar weeks |

Two conclusions follow directly.

**Quota capacity is not what limits this programme before 11-01** — the
remaining work needs roughly a fifth of what the resets should hand over.
Finishing C3, C5, R3, the SC3 remainder, S9 and S10 before that date is the
difference between spending 18% of a gift and spending two full weeks of
standing quota.

**This is not a claim that the work completes by 11-01, and the distinction is
not academic.** Three constraints sit outside the aggregate:

- *Window capacity binds even when total capacity does not.* At the observed
  0.62 resets/day the sustainable rate is ~101 Mweighted/day against an average
  of 53, but bursts run far above that and then stall. Several October windows
  clipped at 99-100%. At the time of writing the account is at **99% with the
  nominal reset 5.8 days out**: ample aggregate capacity, no capacity today.
- *The remaining work is gate-bound.* S10 requires human replacement acceptance
  and a separately authorized bounded live proof, and S9's gating decision is
  unresolved. Neither is moved by quota.
- *The two largest forecast cells are the least reliable.* S9 and S10 are both
  in flight; measured per-slice spread is 4.6x and gpt-6-era slices carried 5.2
  review findings and 2.6 fix iterations each. For quota to bind, the pace would
  have to roughly double, the resets slow, or S9+S10 exceed the high end by
  about 3x.

**Pro 500 is the tier that matches current habits.** The observed 2.28
Pro-200 weeks per calendar week sits just under the 2.5x that Pro 500 provides.
At Pro 200 after 11-01, throughput has to fall by about 56% from the October
rate, which is a scheduling decision rather than an efficiency one — the
measured levers in
[`metrics/efficiency-ledger.md`](../metrics/efficiency-ledger.md) are worth tens
of percent, not a factor of two.


## Where engineering inference goes, by role (measured)

Same corpus and attribution, classified by `agent_path`, root allocated pro rata
([`metrics/scripts/rust_programme_roles.py`](../metrics/scripts/rust_programme_roles.py)):

| role | Mweighted | + root | Pro-200 weeks | share of non-root |
|---|---:|---:|---:|---:|
| build | 461.9 | 575.6 | 3.546 | 60.4% |
| other / coordination | 160.2 | 199.5 | 1.229 | 20.9% |
| review + remediation | 92.7 | 115.5 | 0.712 | 12.1% |
| planning + contract | 50.5 | 62.8 | 0.387 | 6.6% |

**Reviewer threads are 20% of builder spend.** That is the whole of what this
corpus establishes about the cost of correction.

## What this telemetry cannot establish (negative result)

An attempt was made to separate initial implementation from correction by
segmenting each builder thread at the first non-plan review of its own ordinal.
**It does not work, and the failure is more useful than the number would have
been.** Ordinals fall into two populations:

| pattern | ordinals | builder Mwt | spend after first review |
|---|---|---:|---:|
| build, then review | c4, ce3, j1, j2, s67, s8, sc3d, w1 | 137.3 | **27.4%** |
| review interleaved | ce1, r2, rp1, s5, s9, sc1 | 234.2 | **88.2%** |

Those are two **campaign styles**, not two correction rates. Where review runs
concurrently with ongoing planned work, "after the first review" captures the
entire build and says nothing about rework: a builder working after a review may
be correcting a finding, fulfilling an obligation not yet attempted, responding
to new evidence, or reconciling an integration consequence. Chronology does not
recover purpose.

**Retracted.** An earlier draft estimated in-builder rework at 354.2 Mweighted by
assuming the measured 2.6 fix iterations per gpt-6 slice represent 2.6 equally
expensive passes, and reported the total as "62% of the programme" against a
denominator that excluded coordination. Both are withdrawn: iteration *count*
does not establish iteration *cost*, a first pass and a small correction are not
interchangeable, reviewer inference is not all rework, and the correct shares of
the stated denominators were 49.3% and 54.1%. **The rework share is unknown**, and
the 27.4% / 88.2% figures above must not be read as bounding it.

The missing information is attribution of an execution attempt to the obligation
or finding that caused it. Conventional agent logs record what happened, not its
authoritative purpose — which is precisely the relationship
`implementation-contract-compilation.md` and `evidence-and-claims.md` are
designed to preserve.

## Caveats

- The coefficient carries +/-25% window-to-window scatter; every Pro-200-week
  figure inherits it before any forecast error.
- The cached/uncached price split remains an assumption, as the ledger records.
  Total spend estimates are robust to it; the mix-sensitive figures are not.
- Root orchestration is allocated pro rata, not measured per slice. A slice that
  consumed more supervision than average is undercharged here and its neighbours
  overcharged.
- Remaining-work ranges are judgement anchored on analogues. S9 and S10 are both
  in flight, so their "spent" column is a snapshot that will move.
- Reset count assumes the observed 0.62 resets/day continues to 11-01. The last
  four gaps average **2.45 days**, not 1.80, so the forward figure is the slower
  one; the cadence is irregular (observed gaps 0.36 to 3.08 days) and this is the
  largest single uncertainty in the forward budget.
- **Projected, not realised.** M22's handoff saving and the 1.43x multiplier it
  implies are replay-derived from traces, not production results, and they have an
  unmet prerequisite: `ContextPressureController` and `TokenUsagePressureProjector`
  exist only in `app-server/src/*.mjs`. **There is no pressure controller in the
  Rust host** — `lifecycle-core` is contracts/reducer/state, `lifecycle-runtime` is
  clock/executor/framing/process, and nothing in `rust/crates/` carries pressure
  bands or hysteresis. Until it is ported, M22's thresholds have nothing to
  configure, and no part of this saving should be booked when planning capacity.


## Pilot scoring rule (pre-registered)

The Plan IR pilot has two jobs. The first is the economic comparison. The second
is to demonstrate that a structured contract carries enough identity to attribute
inference to the obligation or finding that caused it — the thing the negative
result above shows cannot be recovered afterwards.

Headline outcome is **M14, estimated Plus-units per accepted slice**, fully
loaded: planning, implementation, review, remediation, integration, and the cost
of abandoned attempts. An arm costing more on its first pass wins if it needs
fewer corrections, and an arm scored on builder tokens alone would wrongly reject
it. Quality gates bind: a cheaper slice is not equivalent if it satisfies fewer
obligations or leaves defects undiscovered, so M9 (attempts), M10 (findings) and
the vertical/placement verdicts are reported beside every cost figure.

Attribution must happen **at dispatch or transition time**, not by classification
afterwards:

| quantity | required evidence |
|---|---|
| initial implementation | calls attributed to the initial bounded attempt |
| correction | calls attributable to a specific finding or failed obligation |
| continued planned work | calls fulfilling obligations not yet attempted |
| review | calls attributable to each review episode |
| reopening | work after an earlier acceptance |
| final outcome | accepted consequences and their verification |

Arms, using existing plan documents so that no implementation is required:
arm 1 is free (every builder run to date is Sol at high effort on current plan
representation); arm 2 is Sol against a structurally detailed plan; arm 4 is
medium effort against the same. Arm 2 against arm 1 isolates representation at
fixed effort; arm 4 against arm 2 isolates effort at fixed representation. Keep
every arm at slice level: the pilot must not silently settle whether Plan IR
represents the compiled implementation contract or the accepted branch plan, a
reconciliation its own source defers — "final role and artifact ownership should
be reconciled against the post-migration App Server architecture before
implementation."

## Scoring this later

After 11-01, re-run the attribution and compare actual against the central
column. The forecast is wrong in a useful way if the error is systematic by
bucket — core slices too cheap, qualification too dear — and uninformative if it
is scattered. Record the result in the efficiency ledger's measurement log
rather than editing the table above.

Reproduce the measured half with
[`metrics/scripts/rust_programme_cost.py`](../metrics/scripts/rust_programme_cost.py).

## The Claude-side meter, 2026-10-20

Added because the Claude pilots needed a denominator and I was initially using the
wrong numerator. Everything here is **measured**, with its attribution limits
stated.

### `total_cost_usd` is not money on a subscription

The Claude Code result object reports `total_cost_usd`. On a subscription plan
that figure is **list-equivalent usage**, not marginal spend: the runs cost
nothing additional until the plan's limits bind. Treating it as money is the same
category error as treating Codex weighted tokens as dollars.

Earlier in this session I wrote that "USD is the decision number" for
cross-provider comparison. **Amended:** list-equivalent USD is a *usage proxy*.
The decision number on either provider is **fraction of allowance consumed**, and
both providers now have one.

### The real meter is in the stream

`--output-format stream-json` emits `rate_limit_event` records carrying
`rate_limit_info.unifiedWindows`:

```json
{"five_hour": {"utilization": 0.78, "resetsAt": 1792489200},
 "seven_day": {"utilization": 0.42, "resetsAt": 1792555200}}
```

Two windows, a five-hour and a seven-day, each reported as a utilization
fraction with its reset time, plus `overageStatus` and `isUsingOverage`. This is
directly analogous to the Codex quota-point model and is the figure to track.

### Attribution requires serial runs

**Per-run quota attribution is invalid for concurrently executed runs.** The four
Test C generation-2 arms ran in parallel; all four observed a five-hour window
starting at 78%, and each one's final reading is a *cumulative* total including
the others, not its own contribution. The deltas 83/91/85/83 are therefore not
per-arm costs, and any table presenting them that way is wrong.

The only valid reading from a parallel batch is the aggregate:

| window | Test C gen 2, four arms together |
|---|---|
| five_hour | 78% -> 91%, about **13 points** |
| seven_day | 42% -> 44%, about **2 points** |

Measuring per-arm quota cost requires running arms **serially**. That is a real
experimental-design constraint on every future pilot, and it trades wall-clock
time for attributable measurement.

### What exhausted the account, measured rather than inferred

The two Test R generation-2 arms that terminated with `terminal_reason:
api_error` and "You've hit your session limit" died against the **five-hour**
window, which Test C had already driven to 91%. The seven-day window was at 44%
and was never the binding constraint. A limit event therefore does not imply
weekly exhaustion, and reading one as the other would overstate the programme's
cost by more than a factor of two.

### Calibration does not carry across the account change

The readings above belong to the account in use before 2026-10-20. The plan
changed that day, so those numbers calibrate neither the new plan's windows nor
its utilization baseline. Fresh readings must come from runs on the current plan,
and until several exist the correct statement about Claude-side capacity is that
**the denominator is not yet measured** - not an estimate carried over.

### Context economics, measured on the Test S builder and review arms, 2026-10-20

Prompted by a ChatGPT proposal that model identity should be fixed per compiled
execution unit to protect prompt-cache reuse, with switching allowed only at
checkpoints. The proposal's premise and my first rebuttal were **both** built on
a conflation, so the labels below are deliberate.

**Three different quantities, which must not be substituted for each other.**

| quantity | S-opus (build, 122 turns) | `haiku-5-5` (review, 66 turns) |
|---|---|---|
| **entry context**, first turn | **28,386** | **27,184** |
| **peak working context** (max per-turn read + creation + input) | **276,471** | **241,698** |
| mean per-turn context | 153,137 | 134,404 |
| **accumulated** cache creation, summed over the run | 265,357 | 231,052 |
| **accumulated** cache reads, summed over the run | 11,549,880 | 3,005,771 |

Accumulated cache reads are **not** a context: they are per-turn re-reads of a
growing one, 43.5x and 13.0x the creation figure respectively. Accumulated cache
**creation** is not a working-context size either - it is the sum of cache
writes, which includes incremental growth and any re-write of an expired
segment. Here it happens to approximate the peak (265K vs 276K) only because
context grew monotonically and nothing expired within the run. **Do not reuse
that coincidence as a rule.**

**The 12% figure, correctly labelled.** For S-opus the weighted split is cache
reads 52%, cache creation 12%, output 36%. That 12% is a **share of this run's
weighted inference**, not a measured model-switching cost and not a bound on
one. A switching measurement would have to establish what the successor can
reuse, what must be recreated at the transition, what extra repository
investigation the successor performs, and the provider-priced cost and quota of
all three. **None of that is measured. Reconstruction cost is unmeasured.**

**What the entry figure does establish, and it is the useful result.** S-opus
began with **28,386 tokens** of context - its prompt, the 83-line plan, and
startup material - and from that produced a 13-path, 5,407-insertion tree that
passed all seven configured gates. The 276K peak is **accumulated exploration,
not required context**. So the cost of discontinuity at a checkpoint is not the
peak; it is whatever re-exploration the successor must redo because the durable
contract did not carry it. On this slice that was evidently small: two builders
in different families, from the same 83-line plan and the same baseline, both
reached gate-passing trees with no shared context.

**A distinction that must be kept.** Fresh execution from a durable contract is
established as cheap here. **Resumption of an interrupted, partially implemented
increment with uncertain effects and no reasoning state is a different problem
and is not measured.** The five-hour window makes that case operational, not
hypothetical.

**Weighted tokens misprice tiers by an order of magnitude.** `haiku-5-5` 1.00
Mwt / **$0.5105**; `claude-opus-5` 2.23 Mwt / **$12.4955**. Weighted tokens
differ 2.2x, money differs 24x. Any switching or routing arithmetic computed in
tokens will mislead. Use provider price, and treat it as provider policy rather
than a technical property.

**Role asymmetry in output composition.** Haiku reviewing: 81,429 thinking
tokens, 87% of output. Opus building: 54,972, 34%. Review is
thinking-dominated, construction output-dominated. A router assigning tiers by
assumed difficulty would invert this.

**Context mass may be a property of the subject.** Two very different
responsibilities over the same tree accumulated 265K and 231K of cache creation
and peaked at 276K and 242K. That is **suggestive, not established** - n=1 per
role, one subject.

**The consequence for the compiler.** Compiling obligations alone does not shrink
context: the Test S decision package was the 76-line prompt plus the 83-line plan,
together on the order of 2,500 tokens, under 1% of the peak. The executor still
read `DESIGN.md`, `PHILOSOPHY.md` and the source it had to modify. The
opportunity is compiling **evidence projections** alongside obligations - and the
F3 result is the standing warning on that path, because an omitted observation is
exactly how a projection would fail while looking adequate.

**Two competing hypotheses, neither settled.**

- **A**: keep an expensive model's context alive as long as possible, because
  cache reuse is valuable.
- **B**: make each inference problem small and well-specified enough that cheap
  models, short contexts and cold starts become preferable.

The architecture points at B. The evidence does not yet decide. **The deciding
experiment is not Opus versus Sonnet**: it is the *same* model discharging the
*same* obligation under conventional repository exploration versus a compiled,
evidence-complete decision package, measured on cost, quota, and whether the
required finding survives the narrower evidence. If maximal cache reuse turns
out to be optimal, A wins; if a compiled projection lets a cheap model establish
what an expensive one established by exploring, losing the cache reuse is worth
paying for.

**One behavioural observation, bounded.** S-sonnet's stream contains one `Agent`
call where S-opus had none, on byte-identical input - the executor created a
sub-context on its own initiative. That is **harness behaviour, not an admitted
execution vantage**: no authority was granted, the delegation's bounds were not
specified by any contract, and its inference is consumed inside the arm's own
cost. It is evidence about when delegation appeals to a model, not evidence that
the delegation was necessary or correctly bounded, and it should not be cited as
support for compiled execution units.
