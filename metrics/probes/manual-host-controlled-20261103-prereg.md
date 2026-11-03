# Current manual host versus manual orchestration: prospective pilot

Status: **prospective registration, revision 1; not activated or funded**.
The owner approved preparing the registration and budget on 2026-11-03 after
the six-pair recommendation. The registration receipt binds the frozen protocol;
it is not a funding or execution grant. No arm has been launched. Approval of the concrete
budget and successful qualification below permit execution; this document does
not claim either has happened. The decision is recorded separately in
`records/manual-host-controlled-pilot-decision-20261103.json`.
The local registrar receipt is
[`manual-host-controlled-20261103-registration.json`](manual-host-controlled-20261103-registration.json).
Its authority is caller-asserted, as the registrar reports; it does not attest
human funding, native exposure, enforced isolation or trial execution.

## Question and scope

Does the current deterministic outer host reduce the root's modeled inference
cost for an otherwise equivalent orchestration episode, while preserving the
required checks, evidence and judgment? The contrast is **host automation versus
ordinary manual use of the same underlying scripts**, not two host versions.

This is a controlled lifecycle-workload pilot. Six pairs mix two real-source
reviews with four small experimental workloads; it is not production dogfood
or an estimate for 52-hour supervisor campaigns. It cannot establish the
original 40% root-time hypothesis, defect-recall equivalence, cross-provider
independence, long-context behavior or Dot grant handling.

The protocol follows the prospective comparison rules in
[experiment-plan-20261026.md](../experiment-plan-20261026.md#2-rules-for-every-experiment)
and Round 2's screening, original-episode attribution and failure retention.
For this new comparison the pricing source is the frozen canonical usage reader
identified in the environment manifest, rather than the older experiment's
historical price-file path. One identical pricing scenario applies to both arms.

## Unit, arms and fixed conditions

One episode is a **fresh supervisor session** from its first inference through
task preparation, child launch, identity and settings verification, context and
budget observation, results, review, ordinary repairs, acceptance or abandonment,
acknowledgment, closure and its final report. It owns no unrelated work.
Children and continuations remain assigned to that episode's original arm.

- **H, hosted:** use frozen bundle `b03ad3a899b7323ed911a828d83bc87b5a34d95a579822776e2dbb488fd74569`,
  binary SHA-256 `b25c9bdff5593b4e08ce892a3f87f0924cb519210ed2659d9b326ba78a15b329`,
  with compact receipts. The host performs its supported deterministic work.
- **M, manual:** use the same bundle's underlying role host, scope-budget,
  context, custody and coordination scripts directly. The root may batch calls,
  reuse those helpers and write small adapters as it ordinarily would. It does
  not call the outer manual host or obtain equivalent orchestration from an
  unmeasured helper. New root-authored adapters count toward its cost.

Both arms retain decomposition, packet content, result judgment, acceptance,
scope and integration decisions. Both perform the same required checks; neither
is rewarded for omitting settings, custody, budget, context or closure evidence.
Automatic state and receipt formats may differ; the required information and
correctness remain equivalent. Both get common tool documentation, and only
the condition instruction differs. Every loaded instruction is materialized
from the manifest, never from live skill symlinks.

Requested configuration, fixed for both members of each pair:

| Role | Model | Effort | Tier |
| --- | --- | --- | --- |
| Episode root | `gpt-6.1-sol` | `xhigh` | `default` |
| Implementation/artifact worker | `gpt-6.1-sol` | `medium` | `default` |
| Initial source reviewer and episode reviewer | `gpt-6-astra` | `high` | `default` |
| Protocol review before activation | `gpt-6-astra` | `xhigh` | `default` |
| Masked final quality assessor | `gpt-6-astra` | `high` | `default` |

These are experiment selections, not a change to production model defaults.
Root settings match the observed comparison roots; worker identity is held
constant rather than adding a builder-model comparison. Observed model, effort,
tier, exact turn and filesystem grant must be recorded; request text alone is
insufficient. A required mismatch or missing observation invalidates the
configuration-dependent comparison, with its cost retained. No model fallback.

## Six frozen tasks and allocation

The exact task definitions are in
[`manual-host-controlled-20261103/tasks.json`](manual-host-controlled-20261103/tasks.json).
The product source baseline is Git commit
`8e0ff12daae60073a7836028d6fa78698ac348c2`. The experimental Rust fixture is a
separate dependency-free package, not a product change. Its public tests and
all task documents are hashed in the environment manifest.

| Pair | Workload | Required shape |
| --- | --- | --- |
| P01 | Codec contract source review | One fresh read-only reviewer; root judges and closes |
| P02 | Identifier contract source review | One fresh read-only reviewer; root judges and closes |
| P03 | UTF-8 line summary | One implementation worker, public tests, fresh review |
| P04 | Portable relative-path predicate | One implementation worker, public tests, fresh review |
| P05 | Tag counting and checked merge | One worker across two explicitly bounded turns, then fresh review |
| P06 | 12 MiB artifact and manifest | One artifact worker, source/snapshot verification, fresh review |

Before allocation, freeze eligibility and the public task/environment hashes
for all six pairs. Every task must be feasible in both arms under the same
authority. Register exclusions before any arm outcome; do not replace a task
after observing results. A task that cannot qualify leaves this registration
unactivated; a changed task set needs a new prospective revision.

Each eligible pair receives one independent operating-system random bit:
0 means H then M; 1 means M then H. Store all six bits in one exclusive,
write-once allocation record **before any episode root starts**; print/reveal
the allocation only after durable publication. Reopening the record returns
the existing allocation; uncertainty requires reconciliation, never a redraw.
There is no forced balancing or rerandomization. Report the actual order balance.
Pair order is P01 through P06; runs are serial, with at most one episode root
active. Within an episode, workers and reviewers need not overlap.

Both members receive byte-identical common/task inputs, with condition and
necessary run-local path/identity substitutions recorded separately. Roots are
fresh, receive no study-author conversation and cannot read the other arm's
outputs or future source. P05 retains only its own worker between the specified
turns. It is a planned extension, not a fabricated review-driven repair.

There is **no hidden answer key** in these tasks. The implementation requirements
and examples are public contracts; the review tasks have no predetermined bug
labels. Final semantic quality is assessed against source and contract by a
separate masked assessor. This avoids reading any existing sealed benchmark
keys, including S2 or usage-export batteries. It does not measure known-defect
recall. If a hidden key is later introduced, its author/custodian must be
separate from arm-prompt authoring and execution, under a new revision.

## Measurement and complete accounting

**Primary outcome:** paired root API-equivalent inference cost. Each root SID is
exclusive to one episode, so count all its calls from creation through terminal
report, including startup and closure. No substring-based bookkeeping-call
classifier defines the primary outcome. Host-induced context growth or reduction
is part of the treatment; do not adjust it away. Record initial/common prompt
identity, token/cache counts and execution order to expose remaining variation.

For pair i let M_i and H_i be cumulative root cost, including any permitted
infrastructure retry assigned to that arm. Report each pair and
`saving_i = 1 - H_i / M_i`, its median, arithmetic cost totals and the ratio of
totals. A missing price, nonpositive manual denominator or unclosed cost tail is
unknown, not zero and not silently dropped.

Secondary outcomes: all-actor cost; root calls; authored inline Python bodies
and characters with extraction limitations; other adapter authorship where
observable; dispatches; pre-review fixes; repair cycles; escalations; elapsed
time from first root entry to terminal disposition; explicit interruptions;
review verdicts and evidence failures. Latency and accumulated turn duration
are reported separately and are **not isolated active model time**.

The existing usage ledger supplies deduplicated calls, observed tiers, token
counters and frozen pricing identity. Keep one non-overlapping actor row for
study preparation/qualification, each root, each worker, each reviewer, retries,
quality assessment, intervention, analysis and final closure. A child SID is
exclusive to one episode; retained P05 turns stay on that row. Each call ID is
charged once. Actor sums reconcile to the study total. Subtree totals are not
added to their direct-cost components. API-equivalent cost is not an invoice
or a subscription-quota measurement.

The study controller supplies the same minimal initial launch/identity delivery
in both arms and owns process collection, not free engineering coordination.
Routine episode bookkeeping remains inside the assigned root. Root rescue or
unexpected operator intervention is recorded as an outcome, with its actual
cost attributed to the arm as well as the study total. It cannot disappear into
common overhead. Common qualification/scoring/reporting cost is shown separately
and in an all-in total; it is not used to dilute the primary arm contrast.

After terminal reports, deterministic collection waits for quiescent owned
turns/processes and reconciles root/child token counters. Two unchanged usage
reads 30 seconds apart, within a five-minute collection bound, are evidence of
observed stability, not a billing-settlement guarantee. Missing or later tails
remain explicit. Do not send another paid root turn merely to obtain a snapshot.

## Quality, masking and failure handling

Episode acceptance uses the public task contract and the usual root judgment.
Review results distinguish accept, changes-required, blocked-evidence and
blocked-scope; missing results are never passes. A repair is a review-driven
resubmission cycle, not every continuation. Record zeros and every enrolled
episode, including abandoned, withdrawn and pending episodes.

Final quality assessment receives neutral candidate IDs, exact subjects, public
contracts and verification evidence, without host receipts, root narratives or
condition labels. Source defects and lifecycle evidence are scored separately.
A second evidence view may expose condition-specific artifacts; record that
view as unblinded. Record observed reviewer identity and exposure. Freshness and
native filesystem read-only alone establish no wider tool isolation or
provider independence. The study author does not score its own subjects.

No paid replacement for a behavioral failure: refusal, task misunderstanding,
giving up, a missing report or budget exhaustion are outcomes. An objectively
recorded infrastructure failure may have one fresh retry of the same arm/task,
only after unknown effects are reconciled. Maximum **two retry episodes in the
entire study**, no new task pairs. Preserve the original attempt and all its
cost; if retry capacity is unavailable, retain the missing outcome. A failure
caused by the host's behavior is a treatment outcome, not an excuse to rerun it
as unrelated infrastructure.

A critical authority violation, duplicate unresolved launch/effect, acceptance
of mismatched settings or corrupted evidence, or lost collection ownership
stops new study admissions immediately in both arms. Preserve responsibility
for collecting work already admitted. Ordinary experimental-code review defects
are repaired within their existing episode allowance; they are not themselves
evidence that the orchestration trial must halt. A pause never resets money,
clocks, randomization or original-arm attribution. Resumption after a critical
pause belongs to the owner.

## Analysis fixed before results

The sample is **six nominal pairs / twelve episodes**, with no adaptive
replication. All screened/enrolled tasks, invalid attempts, withdrawals and
missing outcomes remain in the report. Accepted-only cost summaries are
supplementary. A permitted retry adds to its assigned episode's cumulative
cost; it does not create an extra independent observation.

The primary directional effect is the mean paired log cost ratio
`mean(log(M_i / H_i))`. Enumerate the 64 possible first/second assignments from
the six independent order bits under the sharp no-host-effect null. Reassign
the observed first/second outcomes within each pair, use the absolute statistic,
and report the exact two-sided randomization p-value including ties. This tests
the randomized order assignment under that null; it does not eliminate provider
cache or time interactions. Report all six effects, range and n, not just p.

Result classes apply only within this pilot:

- **Confirmed advantage:** six complete valid pairs, exact two-sided p <= 0.05,
  positive primary effect and at least 20% median paired root-cost reduction;
  no critical hosted lifecycle failure, no hosted loss of contract/evidence
  success relative to its paired manual run, and hosted all-actor cost no more
  than 105% of manual all-actor cost. The 5% allowance is an experimental
  efficiency decision threshold, never permission to weaken a product contract.
- **Confirmed disadvantage:** six valid pairs with p <= 0.05, negative primary
  effect and at least 20% median paired root-cost increase; or a demonstrated
  critical hosted orchestration violation under the common contract, reported
  explicitly as a correctness disadvantage rather than a cost estimate.
- **Inconclusive:** valid observations that do not meet either class, including
  small effects, conflicting quality/cost signals or behavioral noncompletion
  that prevents the advantage criterion. No equivalence claim follows.
- **Invalid experiment:** required environment/isolation/configuration evidence,
  allocation, masking provenance or cost coverage cannot support the registered
  comparison. Report the exact affected scope and all costs; no silent deletion
  of an invalid pair to manufacture a smaller confirmatory study.

The 20% threshold is proposed as a useful operational benefit worth investigating,
not a prediction. Six pairs have low power and limited representativeness.
Even a confirmed pilot advantage is not a population-wide savings claim.

## Proposed budget and time

All amounts are modeled API-equivalent USD, not spend targets. Existing
preparation under the user's instruction is included in the setup row and
reported from its actual authority boundary; approval does not reset that cost.

| Envelope | Hard allocation |
| --- | ---: |
| Registration, environment/recorder qualification, protocol review | $20 |
| Twelve nominal episodes, maximum $10 each | $120 |
| At most two infrastructure retries, maximum $10 each | $20 |
| Separate masked quality assessment | $18 |
| Final analysis and reporting | $12 |
| Protected study collection/closure | $10 |
| **Whole-study maximum** | **$200** |

Within an episode: root $5 (substantive stop $3.50, protected tail $1.50),
implementation/artifact worker(s) $2 combined, reviewer(s) $2.50 combined,
unallocated episode margin $0.50. Read-only tasks use no worker allocation.
Initial startup, failed admissions, pre-review fixes and ordinary repairs all
count; no allowance is reset on continuation. Root may assign unused episode
headroom before new work, preserving its tail, within $10 and the frozen model
choices. No cross-episode or category transfer without an explicit study-budget
revision; never consume the final $10 collection reserve for another task.

These are authorization/admission limits, not a claim of provider-enforced
invoice caps. Before new inference, include startup and in-flight exposure in
the remaining-headroom forecast. Delayed usage and any observed overshoot stay
in the original actor/episode row; they are not erased by a stop or reclassified
as a fresh grant. Qualification must show usable forecast/control headroom, or
return an infeasible execution result within the setup allowance.

Planning estimate: $100–150 total, with substantial startup/repair uncertainty;
there is no current matched-task estimate. The hard cap is $200 regardless.
Per episode: 30 productive minutes and 45 minutes to hard return/collection,
measured from initial root entry. Setup/qualification at most two active hours;
whole study at most 16 active hours and 48 elapsed hours from funding. A cap
closes admissions and preserves unfinished outcomes; it does not erase already
incurred cost or authorize unbounded emergency inference. Quiet deterministic
waits own live process collection; no paid idle availability.

## Qualification and activation

The accompanying environment JSON freezes the available baseline, bundle,
source, fixture, instruction and pricing hashes. It truthfully leaves runtime
bindings and controls unestablished until observed. Prior repairs and the older
tiny pilot are background, not current-trial qualification or observations.

Before drawing any allocation or launching episode roots:

1. Record approval of this exact protocol/environment/task revision and $200
   maximum, with original funding/time boundaries, in a separate decision.
2. Under the $20 setup cap, collect a fresh Astra/xhigh protocol review; resolve
   material comparison defects prospectively. A changed subject needs a new
   exact revision/approval-applicability decision before arm entry.
3. Materialize separate single-baseline exports with no later refs or reflog,
   the hashed fixture overlay and frozen role instructions. Verify actual
   readable/writable path, tool, connector and network allowlists. A request for
   native filesystem read-only does not establish the wider allowlist. If the
   production-equivalent route cannot establish the required separation within
   setup scope, stop qualification and report the unmet property; do not silently
   weaken it or build a new isolation platform inside this experiment.
4. Qualify the common outer launcher without beginning a benchmark task: root
   identity/settings/parent accounting, dollar controls, owned cleanup, receipt
   and cost-tail collection, arm instruction selection and no cross-arm access.
   Any model entry for this check is setup cost and a separately identified
   non-scored qualification attempt, not a thirteenth nominal observation.
5. Reuse the existing Rust experiment-control registrar and its durable-state
   transport for frozen specifications and controlled artifact/receipt history;
   registration creates no native roles and proves no global tool isolation.
   Demonstrate allocation recovery/exclusive publication and accounting joins on
   deterministic synthetic records, with no answer-key or trial-outcome access.
   Reuse the usage ledger and existing controls. The Round 2 recorder's
   `cmd_enroll` chooses Terra/Sol and has Terra-specific limits; it is not an
   unmodified host/manual allocator. Any small new reusable executable adapter
   is Rust, bounded by setup scope and independently checked before use.
6. Freeze the resulting realized-environment addendum, exact launch commands,
   observed tool versions/settings and allowed input inventory. Changes may fill
   declared runtime bindings; changes to arms, tasks, thresholds or limits require
   a new registration revision, never an outcome-driven update. Publish its
   digest on the coordination board and retain the owner's approval binding.

Only then publish/recover the allocation record and start the twelve episodes.
Final results belong in a new dated result record linked from the efficiency
ledger, with complete costs and explicit result class. This registration and
its old revisions remain historical. No product source integration, skill
release, deployment, live session reset or production-board experiment is
authorized by this pilot.
