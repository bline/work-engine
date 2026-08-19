# Work Engine Run Review — Consolidated Findings
**Date:** 2026-08-19  
**Run:** `8f2ad7df-6e85-4030-bd78-421e7a8a8b56`  
**Slice:** `Normalize Visual Evidence authored observations`

## Executive summary

This run strongly validates the redesigned Work Engine.

A real Site2JSON slice crossed multiple semantic/UI/runtime boundaries, changed 13 task-owned files, passed focused checks and the full 602-test suite, received real-extension inspection, survived a falsified planning premise, and converged after two valid blocking review findings. The builder remained on GPT-5.6 Sol at low reasoning effort and did not require reasoning escalation.

The most important conclusion is:

> **The flexible objective-driven decision system worked. Most remaining problems are infrastructure, observability, artifact-lifecycle, and capability-affordance problems—not reasons to add more rigid procedure.**

The run also exposed a clear causal chain:

```text
large bootstrap + repeated repository/tool output
        ↓
builder reaches ~78% context before adversarial review
        ↓
implementation/review pushes peak context to ~88%
        ↓
large audit receipt is authored only inside model context
        ↓
context compacts immediately after task completion
        ↓
exact receipt bytes disappear from builder context
        ↓
metrics persistence requires transcript archaeology
        ↓
receipt production/persistence consumes ~21% of builder output
```

A separate cost path also emerged:

```text
many backgrounded subprocesses
        ↓
one model wakeup per completion/poll
        ↓
full large-context replay each time
        ↓
~4.0M input tokens of replay
        ↓
latency/turn cost without materially increasing resident context
```

These are different problems and should be optimized differently.

---

# 1. Guardrail: what this run does **not** justify changing

This section should stay near the top of any future review.

Do **not** respond to this run by adding:

- mandatory evidence ladders;
- mandatory review panels;
- mandatory context quotas;
- hard per-phase read limits;
- compulsory agent-per-seam decomposition;
- fixed model-escalation ladders;
- quota-exploitation logic;
- rigid browser-inspection recipes;
- more procedural rules merely because one infrastructure seam failed.

The architecture should continue to express:

```text
objective
+ invariants
+ acceptance conditions
+ available capabilities
+ evidence/cost state
→ model chooses route
```

The Work Engine succeeded because the model had room to adapt while still knowing what mattered.

The correct response is:

> **Improve the machinery around the reasoning so the model does not repeatedly pay for context, evidence, artifacts, or process state the system already possesses.**

---

# 2. What the run proved

## 2.1 Route flexibility worked

A planning premise was falsified:

> The existing picker/caller payload already supplied every field required by `validateObservation`.

The builder did not force the original plan and did not terminate unnecessarily.

It preserved valid evidence, marked dependent assumptions stale, and revised the route to:

> IR-owned normalization plus minimal caller-specific canonical projection and current-context plumbing.

This is exactly the intended behavior:

> **Routes are hypotheses. Invariants and acceptance conditions remain binding.**

## 2.2 Proportional validation made a good expensive decision

The slice touched:

- Detection;
- Graph;
- Visual Evidence;
- picker data;
- shared IR;
- provenance;
- ambiguity;
- user-authored state.

The workflow selected stronger validation because the consequence justified it.

Independent review found two real blocking defects:

1. A click without extractable interpretation could throw instead of becoming a truthful unresolved click.
2. An ambiguous Graph route could discard click evidence rather than remain unresolved while preserving the recorded click.

Both were fixed, regression-tested, and revalidated.

Therefore:

> **Independent review was valuable. Reconstructing independent review repeatedly was the waste.**

## 2.3 Low reasoning effort was enough

The builder remained on GPT-5.6 Sol / low effort for the full slice.

No reasoning escalation occurred.

This supports:

> **Start capable workers cheaply when evidence capabilities narrow the problem; escalate only from concrete evidence of need.**

## 2.4 Capability-oriented routing emerged naturally

The builder used different mechanisms for different questions:

- Claude + Codebase Memory for independent placement/review;
- direct Codebase Memory for targeted structural questions;
- literal source reads when graph evidence was ambiguous or exact behavior mattered;
- Chrome Vision for rendered-state observation;
- deterministic gates/tests for executable proof.

This is evidence that the new objective/capability model is producing useful autonomous routing.

---

# 3. Four distinct resource mechanisms

The run shows that “token efficiency” is too coarse a concept.

## 3.1 Context accumulation

This is resident information that grows the active builder context.

Examples:

- skill/reference bootstrap;
- large source reads;
- tool/interface descriptions;
- accumulated implementation state;
- direct repository exploration.

Effect:

> threatens context capacity, coherence, and compaction.

Observed:

- ~78% context occupancy before adversarial review;
- ~88% peak context occupancy.

## 3.2 Context replay

Every new model turn reprocesses the existing large prefix, even when almost all of it is cached.

Examples:

- subprocess completion/poll turns;
- repeated wakeups;
- small follow-up actions late in a large session.

Effect:

> token/latency/turn cost without necessarily increasing resident context.

Observed polling/wait cost:

```text
poll/wait turns:             28
mean input per turn:    ~142,673
replayed input:       ~3,994,862
share of builder input:    ~17.9%
```

Polling did **not** cause the context occupancy that led to compaction. It belongs to execution/turn economics, not the compaction causal chain.

## 3.3 Disposable external reasoning

This includes Claude repository evidence and independent review.

Recorded provider totals:

```text
provider output:           67,699
provider thinking:         31,401
provider cache reads:   3,192,067
provider cost:            $4.1001
provider wall time:       719.922s
```

This is where repo-search and persistent review can help.

## 3.4 Artifact lifecycle

This is reasoning spent creating, reconstructing, validating, or recovering durable artifacts that infrastructure could preserve.

The receipt episode is the clearest example.

Measured receipt production + persistence episode:

```text
input tokens:       ~736,370
output tokens:       12,845
reasoning tokens:     2,041
```

`12,845 / 60,673 ≈ 21.2%`

So roughly 21% of builder output for the slice was spent producing/persisting the receipt.

---

# 4. Builder-side spend is a first-class optimization target

The terminal receipt left builder metrics as `null`, but the raw rollout contains running telemetry.

Observed builder totals:

```text
builder input:             22,379,428
builder cached input:      21,944,320   (~98%)
builder output:                60,673
builder reasoning:             11,550
peak context:                 226,461   (~88% of 258,400)
observed builder turns:             9
wall clock:                    ~2,605s   (~43.4 min)
```

Builder output was approximately 47% of combined builder + provider output:

```text
60,673 / (60,673 + 67,699) ≈ 47%
```

Important precision:

> Do **not** convert this directly into a dollar-equivalence claim.

Provider dollars and Codex builder/cache economics are not directly comparable from this log.

The correct conclusion is:

> **External providers are not the only major reasoning consumer. Persistent builder context and output must be measured and optimized too.**

---

# 5. Builder metrics should be derived after completion

The receipt reported:

```text
builder_turn_count: 4
```

The complete rollout shows 9 builder turns.

Four was apparently truthful when the receipt was authored. Five later turns occurred during persistence/recovery.

This exposes a structural impossibility:

> **A builder cannot accurately report final metrics about work that happens after it produces the report.**

Therefore operational builder metrics should be harvested outside the builder after its terminal event:

- input tokens;
- cached input;
- output tokens;
- reasoning tokens;
- peak context;
- turn count;
- wall clock;
- tool-call count;
- tool-output volume;
- repository-access categories.

These should be deterministic telemetry, not model-authored claims.

---

# 6. Context growth is a P0 concern

The builder reached roughly 78% occupancy **before adversarial review began**.

Therefore review did not create the context crisis.

The largest resident-context contributors were instead:

- large batched `sed`/source reads;
- initial skill/reference loading;
- Codebase Memory capability/tool discovery;
- repository instructions;
- roadmap/design documents;
- repeated direct repository exploration;
- normal implementation evidence.

Raw analysis found roughly 745 KB of tool output across ~155 outputs, with the largest handful accounting for a disproportionate share.

The direct `npm test` call was **not** a major context problem because the harness limited returned model-visible output to ~3K tokens.

The right question is:

> **Which information must remain resident for the whole slice, and which can be loaded only when the current phase/decision needs it?**

Investigate before imposing limits.

Potential experiments:

- phase-lazy loading of receipt/reference docs;
- smaller skill bootstrap;
- less tool-interface discovery;
- avoid repeated large batched source reads;
- track repeated ranges;
- measure repo-search effect on builder-resident evidence.

---

# 7. Receipt persistence architecture is the clearest infrastructure defect

Actual order:

```text
receipt authored
→ task_complete
→ context compaction
→ first append attempt
→ schema rejection
→ second schema rejection
→ session-log archaeology
→ temporary receipt staging
→ append succeeds
```

The receipt was gone from builder context **before the first append attempt**.

So an append-retry wrapper is not enough.

The durable artifact must leave model context at production time.

The receipt was never lost from the system:

- it existed in the task-complete payload;
- it existed in the rollout log.

The builder later searched its own transcript to recover it.

That recovery was clever but should never be necessary.

## Preferred architecture

```text
builder
  ↓
returns semantic outcome / durable decisions / evidence references
  ↓
task_complete captured durably
  ↓
deterministic metrics harvester reads rollout/tool telemetry
  ↓
deterministic receipt assembler
  ↓
schema validation
  ↓
atomic append
```

At minimum:

```text
builder authors receipt
  ↓
exact bytes staged immediately
  ↓
task completes
  ↓
supervisor validates/appends staged artifact
```

Compaction must never be able to destroy the only exact copy.

---

# 8. Receipt schema/docs have avoidable ambiguity

Two schema failures occurred.

## 8.1 Unsupported `compatibility`

The builder added:

```json
{
  "provider": "claude-codebase-memory",
  "skill": "claude-recon-implementation",
  "compatibility": "legacy-combined-evidence-and-review"
}
```

The validator correctly rejected the extra key.

The builder was trying to preserve a useful fact, suggesting either:

- the closed identity shape was not salient enough; or
- provider-specific provenance lacks an obvious legal home.

Give provider-specific metadata one documented location.

## 8.2 `replacement_route` invited the wrong interpretation

The builder treated `replacement_route` as prose describing the new implementation approach.

The validator expected a route enum such as:

```text
falsified-placement
```

Prefer:

```text
replacement_workflow_route
```

and, if prose is useful:

```text
replacement_plan_summary
```

## 8.3 Canonical schema ownership

Audit:

- `receipt-schema.md`;
- builder receipt reference;
- supervisor skill;
- builder skill;
- `append_metrics.py`;
- examples/tests.

Check for:

- schema-version drift;
- stale examples;
- closed objects not clearly documented;
- enum names that imply free text;
- useful metadata with no legal home.

Canonical examples should execute as validator fixtures.

---

# 9. Reviewer persistence through remediation is strongly supported

Independent review totals:

```text
calls:                 3
output tokens:    35,657
thinking tokens:  21,718
cache reads:   1,993,625
cost:             $2.1483
wall time:          386s
```

Recommended lifecycle:

```text
fresh isolated reviewer
    ↓
initial whole-slice review
    ↓
finding(s)
    ↓
builder repairs
    ↓
same reviewer verifies delta + original concern
    ↓
repeat bounded remediation if needed
    ↓
accept
    ↓
discard reviewer
```

Freshness matters for independence **from the builder**.

Repeated amnesia between repair rounds does not create more meaningful independence.

Start fresh again only when:

- architecture/placement changes materially;
- the review premise changes;
- context becomes confused/oversized;
- a genuinely new independent perspective is valuable.

---

# 10. Repo-search remains valuable, but benchmark total effect

Repository-evidence provider totals:

```text
attempts:            4
successful:          3
infra failures:      1
output tokens:  32,042
thinking:        9,683
cache reads:   1,198,442
cost:           $1.9517
wall time:       333.8s
```

Repo-search should reduce routine external repository reasoning and provide a cleaner evidence abstraction.

But do not expect provider savings alone to predict total Work Engine savings.

Benchmark:

- provider calls/output/cost;
- direct builder repository reads;
- builder context growth;
- total turns;
- wall time;
- review quality;
- accepted-result quality.

---

# 11. Repository exploration outside evidence packets must be derived

The receipt field:

```text
repository_exploration_outside_evidence_packets
```

was model-authored prose.

The raw rollout shows substantial direct repository activity throughout planning and verification.

That activity may have been justified.

The problem is telemetry.

If the architecture claims bounded evidence packets reduce durable builder exploration, the measure cannot be self-reported by the builder.

Derive repository access from the rollout:

```text
repository access
├── repo-search / bounded evidence capability
├── independent-review repository access
├── builder direct Codebase Memory
├── builder exact-source follow-up
└── other/unclassified direct exploration
```

Track:

- calls;
- files/ranges;
- bytes/tokens returned;
- repeated ranges;
- phase;
- context contribution.

---

# 12. Provider permission failure is a cheap, high-value fix

The first Claude call failed because the required permission mode was omitted.

Later calls used:

```text
--dangerously-skip-permissions
```

This was the run's one recorded infrastructure failure, and permission failures have appeared in earlier runs too.

If the documented invocation example still omits the flag:

> fix the example and add deterministic provider preflight.

This is exactly the kind of failure infrastructure should eliminate before production reasoning begins.

---

# 13. Chrome Vision: the full mystery is now mostly explained

This was not one bug. It was an affordance/configuration seam.

## 13.1 The builder did try the intended path first

The builder explicitly said it was using the `chrome-vision` skill.

It read the skill, which correctly said:

- use the repository broker;
- use bounded operations;
- the broker speaks NDJSON over stdin/stdout.

It then tried:

```bash
node scripts/site2json-chrome-vision.mjs --help
```

That returned exit code 0 but no useful output.

So the first attempt was not a thrown error, but it was an ergonomic dead end.

## 13.2 It discovered the Site2JSON adapter

`scripts/site2json-chrome-vision.mjs` contains:

- endpoint;
- Site2JSON aliases;
- extension name;
- recovery command;
- helper functions.

But it is not an interactive broker CLI.

## 13.3 It discovered the generic persistent broker

The generic CLI:

```text
work-engine/skills/chrome-vision/scripts/chrome-vision.mjs
```

expects:

```text
chrome-vision <config.json>
```

It reads the config once, creates one `ChromeVisionBroker`, then remains alive consuming NDJSON requests from stdin.

That is the intended persistent architecture.

## 13.4 The repository had no ready-to-use runtime broker config

The builder searched for a Chrome Vision config and found:

- schema;
- package metadata;
- observation schema;

but no actual runtime config file.

The config schema requires:

```json
{
  "version": 1,
  "endpoint": "http://127.0.0.1:9222"
}
```

with optional:

- limits;
- artifact directory;
- target aliases.

The Site2JSON adapter contains project configuration knowledge, but its profile shape is not directly the generic broker config shape.

So the model faced:

```text
A. persistent generic broker
   - correct architecture
   - requires config
   - no runtime config exists
   - requires persistent stdin/process handling

B. Site2JSON adapter
   - knows project aliases/recovery
   - not an interactive broker CLI
   - --help yields nothing

C. inline node -e
   - import broker
   - construct config inline
   - request()
   - works immediately
```

It chose C.

## 13.5 Thirteen one-shot brokers were confirmed

The log contains **13 separate `new ChromeVisionBroker(...)` constructions**, each in its own one-shot:

```text
node --input-type=module -e '...'
```

process.

This matches the background terminals seen during the run.

Persistence was structurally impossible in that invocation style.

## 13.6 AGENTS.md contains legacy manual-CDP route gravity

`AGENTS.md` correctly says generic observation belongs in Chrome Vision, but it also still contains the older manual CDP recovery recipe:

- enumerate port 9222 targets;
- identify page/service worker;
- call `chrome.sidePanel.open(...)`;
- reconnect manually.

That old detailed recipe remains behaviorally salient.

So the model received:

```text
new guidance:
use chrome-vision

old detailed fallback:
here is exactly how to do CDP manually
```

When the high-level path had friction, it already had a lower-level recipe in working context.

## 13.7 Correct Chrome Vision fix

Do **not** add more broker lifecycle theory first.

The immediate fix is:

> **Make persistent Chrome Vision the easiest path.**

Recommended:

1. Create one obvious Site2JSON Chrome Vision entry point.
2. Have it already know/derive the project config.
3. Start or attach to one persistent broker host.
4. Make request/reuse/release ergonomics obvious.
5. Remove/de-emphasize one-shot broker examples.
6. Remove the detailed old manual CDP recipe from always-resident AGENTS instructions; preserve it inside adapter/recovery docs/tests.

Conceptually:

```text
site2json-chrome-vision start
site2json-chrome-vision request '{...}'
site2json-chrome-vision request '{...}'
site2json-chrome-vision stop
```

or automatic start/reuse behind a single command.

The lesson:

> **When the intended abstraction has an incomplete seam, a capable model drops one level and completes the missing machinery itself.**

---

# 14. Background subprocess polling churn is confirmed

Measured:

```text
poll/wait turns:        28
mean replay input: ~142,673
replayed input:   ~3,994,862
share of builder input: ~17.9%
```

Important nuance:

- this is primarily **turn/latency/replay cost**;
- it is **not resident-context growth**;
- it did **not** cause the compaction.

The shape was also interesting:

> roughly one wait/poll per backgrounded subprocess, not pathological repeated polling of a few jobs.

So the problem is not “poll less often.”

The problem is:

> **Every backgrounded process currently creates another expensive large-context model turn when completion must be observed.**

If the execution substrate supports it, prefer completion delivery that does not require a model turn merely to learn that the process finished.

Measure before redesigning.

---

# 15. Direct `npm test` output is low priority

The command generated a large TAP stream, but the harness had:

```text
max_output_tokens: 3000
```

Only roughly 3K tokens entered model context.

Therefore wrapping every large command in `run_gate.py` is not currently a high-value optimization.

Keep the general bounded-output principle, but focus on measured large context contributors instead.

---

# 16. Supervisor/builder metrics ownership deserves a seam check

Intended ownership:

```text
builder → slice reasoning / implementation / validation / receipts
supervisor → campaign control / durable metrics / continuation
```

Actual recovery:

```text
builder authors receipt
→ compaction
→ supervisor reactivates builder
→ builder attempts/reconstructs/stages/appends metrics
```

This worked, but operational telemetry/persistence is probably better owned outside the builder.

Reasons:

- exact artifacts should survive builder context loss;
- final builder metrics cannot be self-measured accurately;
- persistence should not require reopening terminal reasoning.

---

# 17. Quota behavior: useful grace, never capacity

The run began at roughly 1% weekly allowance remaining.

The meter reached 100% used / 0% remaining while the active run continued for substantial additional work.

This appears to be graceful continuation behavior for an already-active run.

Operational invariant:

> **Benefit from grace if it happens; never budget around grace existing.**

Weekly remaining percentage is not a precise active-run compute budget.

Do not build quota-exploitation logic into Work Engine.

---

# 18. Revised priorities

## P0 — Fix receipt production/persistence architecture

Why:

- compaction happened before first append;
- exact receipt existed only transiently in model context;
- receipt episode consumed ~21% of builder output;
- self-authored final metrics are stale by construction.

Actions:

- capture/stage semantic receipt at production/task-complete time;
- preferably assemble final audit receipt deterministically after return;
- harvest builder metrics from rollout telemetry;
- validate before atomic append;
- make retries independent of model memory.

## P0 — Investigate/reduce builder context growth

Why:

- ~78% occupancy before review;
- ~88% peak;
- context pressure caused compaction;
- ~22.4M builder input replayed over the run.

Actions:

- attribute context/tool output by source and phase;
- inspect top contributors;
- phase-lazy-load references where appropriate;
- measure repeated source/range reads;
- measure repo-search effect on resident builder evidence.

Do not impose arbitrary limits first.

## P0/P1 — Harvest builder metrics deterministically

Record after terminal completion:

- input/cached/output/reasoning;
- peak context;
- turns;
- wall clock;
- tool calls/output volume;
- repository-access categories.

## P1 — Audit receipt schema/docs

- align schema versions;
- remove ambiguous field names;
- document provider-metadata home;
- turn examples into validator fixtures.

## P1 — Fix provider invocation/preflight

- correct permission-mode example;
- deterministic provider preflight;
- eliminate known permission failures.

## P1 — Persist reviewer context through remediation

- first review fresh;
- reuse same isolated reviewer through bounded fixes;
- start fresh only after material premise/boundary/context change.

## P1 — Complete and benchmark repo-search

Measure total effect, not only Claude savings.

## P1 — Fix Chrome Vision entry-point/config integration

- one canonical Site2JSON command;
- project config available/derived automatically;
- persistent broker reuse is easiest route;
- remove/de-emphasize lower-level one-shot/manual route;
- move manual CDP recovery detail out of always-resident AGENTS.

## P1/P2 — Derive repository-exploration telemetry

Replace self-reported prose with rollout-derived measurement.

## P2 — Reduce subprocess completion replay if measured worthwhile

The current cost is real (~17.9% input replay), but it is execution-substrate overhead rather than context-growth cause.

## P3 — Additional bulk-command wrapping

Low measured value in this run.

---

# 19. Suggested measurements for future slices

For each accepted slice, capture:

```text
quality
- late semantic rejection
- review findings
- fix rounds
- tests
- real UI inspection where applicable

builder
- input
- cached input
- output
- reasoning
- peak context
- turns
- wall clock
- tool output bytes/tokens

repository evidence
- repo-search / direct CBM / source-read calls
- files/ranges
- repeated ranges
- builder-direct exploration

review
- fresh review episodes
- remediation continuations
- output/cache/cost
- findings by severity

browser evidence
- broker starts
- broker reuses
- one-shot processes
- Chrome reconnects/restarts
- observation packet count

infrastructure
- provider failures
- background subprocess count
- wait/completion turns
- compactions
- receipt production/persistence cost
```

Compare before/after:

- repo-search;
- persistent reviewer remediation;
- phase-lazy reference loading;
- deterministic receipt assembly;
- corrected Chrome Vision persistent entry point.

Success metric:

> **lowest total cost to a trustworthy accepted result without increased late defects or weakened evidence.**

---

# 20. Final assessment

This run is evidence that the Work Engine redesign was directionally right.

The model:

- revised a falsified route;
- preserved valid evidence;
- used different capabilities intelligently;
- stayed at low reasoning effort;
- selected independent review where justified;
- fixed real defects found by that review;
- passed deterministic validation;
- inspected the real UI;
- recovered from compaction and schema failures without corrupting the durable record.

The remaining failures cluster around infrastructure:

- context accumulation;
- context replay;
- exact-artifact lifetime;
- self-measurement;
- schema ergonomics;
- reviewer reconstruction;
- provider invocation hygiene;
- repository-access telemetry;
- Chrome Vision configuration/entry-point affordance;
- background subprocess completion cost.

The decision system itself should remain flexible.

> **Do not add more rules to solve infrastructure problems. Make the correct, efficient capability path easier than the lower-level workaround.**
