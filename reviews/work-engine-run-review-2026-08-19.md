# Work Engine Run Review

**Run:** `8f2ad7df-6e85-4030-bd78-421e7a8a8b56`  
**Slice:** Normalize Visual Evidence authored observations  
**Date:** 2026-08-19  
**Campaign:** Site2JSON roadmap  
**Builder:** GPT-5.6 Sol, low reasoning  
**Validation profile:** `engineering-proportional`

## Executive summary

This run is the strongest evidence so far that the redesigned Work Engine is behaving differently in the intended direction. A real, medium-risk Site2JSON slice crossed editor, picker, shared IR, tests, documentation, and live UI verification. The builder stayed at low reasoning, revised its route when evidence invalidated an assumption, completed implementation, ran focused and broad deterministic validation, used independent review where it mattered, repaired two real blocking defects found by review, and reached an accepted result with 602/602 tests passing.

The run also exposed several concrete efficiency and reliability seams. The largest avoidable cost was not review itself, but repeatedly reconstructing fresh review context during remediation. Repository evidence and independent review were still coupled through the same legacy Claude + Codebase Memory provider, so routine repository understanding consumed nearly half of the external-provider cost. The metrics receipt was generated correctly enough to contain rich evidence, but persistence failed twice because the probabilistically authored receipt did not exactly match the validator schema; compaction then made the transient receipt unavailable from model context. Recovery ultimately succeeded only after the receipt was recovered, corrected, staged to a temporary file, validated, appended, and the temporary file removed.

The important conclusion is not that the Work Engine needs another major redesign. The decision policy appears to have worked. The next gains are mostly infrastructural: preserve acquired context where independence is not lost, move deterministic data-shaping out of model memory, separate repository retrieval from independent judgment, and prevent large mechanical outputs from entering the reasoning context.

---

## 1. What the slice accomplished

The accepted slice normalized real Detection and Graph Visual Evidence selections into validated authored or unresolved session observations using canonical semantic-model identity and truthful picker/page provenance.

The implementation:

- added IR-owned canonical target projection and observation normalization;
- projected Detection fields from effective Semantic DOM nodes/findings/reports, including nested relationship paths;
- projected Graph paths from authored root/nodes/edges/property references without allowing aliases to become semantic identity;
- preserved ambiguity instead of selecting an arbitrary Graph route;
- propagated real picker version and current Page Analysis revision;
- distinguished authored click provenance from explicitly unresolved provenance;
- excluded raw selected values, live DOM references, opening tags, persistence effects, inference effects, and runtime effects;
- updated focused tests, broader tests, documentation, and the real extension path.

The final deterministic state was strong:

- 13 task-owned files;
- 602/602 full-suite tests passed;
- 12 focused Visual Evidence tests passed;
- initial ordered gate: 8/8 checks passed;
- two post-review repair gates: 5/5 each passed;
- `git diff --check` passed;
- real-extension Graph workflow inspected at wide/dark and narrow/light layouts;
- no final blocking review findings;
- two blocking high-severity findings discovered and repaired.

The one meaningful visual-validation limitation was explicit rather than hidden: the resumed draft exposed the Graph route but not blank-draft Detection, and opening Detection would have created additional user state. The run therefore used deterministic vertical and bridge tests for that path rather than mutating user state merely to satisfy a ritualized visual check. This is a good example of proportional validation rather than validation theater.

---

## 2. Evidence that bounded flexibility is working

The most important behavioral observation is that the system did not merely execute a fixed sequence more cheaply. It adapted its route while preserving the objective and invariants.

The builder began with a premise that the existing picker/caller payload already supplied every field required by `validateObservation`. Targeted evidence falsified that premise: semantic identity and document revision existed in authoritative owners, but were not projected into the Visual Evidence invocation. The builder preserved the valid evidence, marked the dependent approach stale, and replaced it with IR-owned normalization plus minimal caller-specific canonical projection and current-context plumbing.

That is exactly the behavior the Work Engine redesign was intended to produce:

> **Routes are disposable hypotheses; objectives, invariants, and acceptance conditions remain authoritative.**

The builder did not require a human restart, did not treat a corrected premise as failure, and did not blindly follow its first plan. It revised the path while keeping the accepted semantic goal intact.

A related observation appeared in the repository-evidence work around this run: the model began decomposing evidence questions into separate bounded agents without being told exactly how to optimize the decomposition. That suggests the global objective plus capability boundaries are giving the model enough direction to invent efficient tactics rather than forcing those tactics into the workflow specification.

This is evidence, not yet proof, that the Work Engine's new flexibility is producing the intended emergent behavior. It should be evaluated across more runs before being converted into additional hard procedure.

---

## 3. Independent review was expensive and demonstrably valuable

Independent review should **not** be removed based on this run.

The first adversarial review found a genuine blocking edge case: a rendered-page click on an element with no extractable interpretation could throw before the picker dismissed. The repair made the operation truthfully produce an unresolved observation while retaining click provenance.

The second review found another genuine blocking semantic defect: an ambiguous Graph route with a valid click interpretation needed to remain unresolved while still retaining the recorded click evidence. The repair preserved both ambiguity and action evidence and added a focused regression test.

These were not cosmetic findings. They were exactly the kind of semantic/provenance failures that a passing test suite can miss when the tests encode the same assumptions as the implementation.

The conclusion is:

> **Review was good expensive work. Reconstructing review context repeatedly was avoidable expensive work.**

### Review cost observed

The run used three successful independent-review calls:

- cache creation: 169,183 tokens;
- cache reads: 1,993,625 tokens;
- output: 35,657 tokens;
- thinking: 21,718 tokens;
- provider cost: about $2.15;
- provider wall time: about 386 seconds.

The average review round therefore paid again for a large portion of the same accepted boundary, prior findings, invariants, task-owned files, and architectural context.

### Recommended review lifecycle

Use a fresh isolated reviewer at the beginning of an independent review episode, then preserve that reviewer's context through bounded remediation rounds until acceptance or material premise change:

```text
fresh independent reviewer
        ↓
full initial review
        ↓
findings A / B
        ↓
builder repairs delta
        ↓
same reviewer verifies delta + prior concern
        ↓
repeat while converging
        ↓
approval
        ↓
discard reviewer
```

Start a new fresh review context only when the implementation boundary materially changes, architecture/placement is reopened, the reviewer becomes confused/context-heavy, a different independent perspective is warranted, or a later review episode begins.

This preserves the original independence from the builder while avoiding repeated acquisition of the reviewer's own knowledge.

---

## 4. Repository evidence and independent judgment are still conflated

The external-provider metrics divide almost exactly into two halves.

### Repository evidence

- 4 attempts;
- 3 successful;
- 1 infrastructure failure;
- output: 32,042 tokens;
- thinking: 9,683 tokens;
- cache reads: 1,198,442 tokens;
- provider cost: about $1.95;
- wall time: about 334 seconds.

### Independent review

- 3 successful calls;
- output: 35,657 tokens;
- thinking: 21,718 tokens;
- cache reads: 1,993,625 tokens;
- provider cost: about $2.15;
- wall time: about 386 seconds.

### Combined provider cost

- output: 67,699 tokens;
- thinking: 31,401 tokens;
- cache reads: 3,192,067 tokens;
- provider cost: about $4.10;
- wall time: about 720 seconds.

The run still identified both `repository_evidence` and `independent_review` as the legacy combined `claude-codebase-memory` / `claude-recon-implementation` capability.

This strongly validates the architectural split already underway:

```text
repo-search
    repository retrieval / tracing / coverage

independent review
    fresh judgment / falsification / adversarial synthesis
```

Routine repository understanding does not inherently require an independent language-model perspective. Codebase Memory, exact source, graph traces, and bounded fallback can often provide that evidence more cheaply. Independent model judgment should be purchased where the value of an independent perspective justifies it.

A useful experiment after `repo-search` lands is to compare otherwise similar slices on:

- external-provider output tokens;
- cache reads;
- wall time;
- supplemental evidence calls;
- placement reconsiderations;
- late semantic rejections;
- review findings;
- accepted defects discovered later.

The goal is not fewer agents or fewer calls by itself. The goal is lower total cost to a trustworthy accepted result.

---

## 5. The receipt/metrics seam is currently too dependent on model memory

The run exposed a clear persistence-design problem.

The builder produced a rich schema-v4 audit receipt, but the first append was rejected because the two identity objects contained an unsupported `compatibility` key. The exact receipt then became difficult to retry safely because the shell was non-interactive and did not preserve history.

A later recovery attempt exposed a second schema mismatch: `route_revisions[0].replacement_route` contained a prose description of the replacement implementation approach, while the validator expected a workflow-route value such as `falsified-placement`.

Then context compaction occurred. After compaction, the builder correctly refused to reconstruct the exact large receipt from memory because doing so could silently change evidence.

The eventual successful recovery was revealing: the corrected receipt was staged to a temporary JSON file, passed mechanically through the locked validator/append path, appended successfully to `work-engine/metrics/roadmap.jsonl`, and the temporary file was removed.

That recovery path should become normal behavior.

### Recommended invariant

> **Once a terminal receipt has been produced, its exact bytes must survive model compaction, retries, and handoffs. Persisting or retrying it must never require reconstruction from model memory.**

### Recommended normal path

```text
builder produces semantic receipt
        ↓
stage exact candidate JSON
        ↓
validate staged artifact
        ↓
correct only validator-supported defects from evidence
        ↓
validate again
        ↓
append exact validated artifact
        ↓
remove/archive staging artifact
```

An even stronger design would move schema construction itself further into deterministic code: the model supplies semantic fields and flexible measurements, while a receipt builder normalizes closed objects, enum fields, defaults, nulls, and provider-specific extensions into the canonical schema.

---

## 6. The schema and skill contracts need a targeted audit

The immediate schema errors were model errors, but both were avoidable traps.

### `compatibility` in identity objects

The model was trying to preserve a truthful and useful fact: this run used the legacy combined evidence-and-review provider. It placed that fact inside the identity objects, but those objects have a closed schema.

This raises two questions:

1. Is the closed identity shape sufficiently explicit in the builder/receipt instructions?
2. Is there a clearly documented canonical home for provider-specific compatibility/provenance metadata?

If the fact belongs in `additional_metrics`, the skill should say so explicitly.

### `replacement_route` ambiguity

The model interpreted `replacement_route` in ordinary engineering language as "the replacement implementation route" and supplied a prose plan. The validator treated it as a workflow-route enum.

That field name is semantically ambiguous. A clearer contract would be something like:

```text
replacement_workflow_route: direct | falsified-placement
replacement_plan_summary: <optional prose>
```

The exact names are less important than removing the collision between workflow-routing terminology and implementation-plan terminology.

### Documentation/version drift

Earlier supervisor material still referred to schema version 3 while this run emitted schema version 4. Even if the live working copy has since been corrected, this is evidence that schema ownership can drift across skill docs, examples, and validator code.

### Recommended audit

Audit these as one seam rather than patching the two observed fields independently:

- `builder-receipt.md`;
- `receipt-schema.md`;
- `append_metrics.py`;
- slice-builder receipt instructions;
- slice-supervisor append instructions;
- all schema-v4 examples/tests;
- provider identity/provenance placement;
- every enum whose name could plausibly imply free text;
- schema-version references in skills and docs.

Every canonical receipt example should be executable as a validator fixture. The validator should remain the source of truth.

---

## 7. Deterministic output compression worked - except where it was bypassed

`run_gate.py` did what the workflow wanted: it converted broad mechanical validation into a small structured result. The initial gate ran eight checks and returned a compact pass/fail summary instead of flooding the builder context with test output.

Near the end of the run, however, the builder directly ran `npm test` and received roughly 3,500 lines of TAP output into its context even though the same class of evidence had already been handled through the deterministic gate.

This is a small but clean example of avoidable context pollution.

### Recommended principle

> **Bulk deterministic output should terminate at a compact deterministic boundary. Models should receive the result and only the failure detail needed for diagnosis.**

Direct full-suite commands should normally be wrapped by `run_gate.py` or an equivalent bounded-output mechanism. Raw output should be pulled only on failure and only for the failing portion.

---

## 8. Chrome Vision may not yet be reusing its persistent transport as intended

During the run, `/ps` showed 12 background Node terminals, many associated with `ChromeVisionBroker` invocations and prior observation packets.

This does not prove that 12 live CDP/WebSocket connections were leaking; the Codex terminal UI may retain completed/background entries. But it is strong enough to investigate.

The intended Chrome Vision architecture is:

```text
one persistent broker/session
        ↓
many bounded observations
```

not:

```text
observation
→ launch persistent broker
observation
→ launch another persistent broker
...
```

Before changing anything, inspect actual process/socket state and broker lifecycle. If connections are truly accumulating, add broker discovery/reuse, lease semantics, or deterministic shutdown. If the terminals are merely retained UI records, document that and leave the architecture alone.

This should remain an evidence-driven investigation, not a speculative rewrite.

---

## 9. Metrics are already useful, but builder-side spend remains partially invisible

The run captured external-provider spend in considerable detail, including provider roles, cache creation/read, output tokens, thinking tokens, wall time, cost, failures, evidence modes, fallbacks, retrieval counts, placement calls, and review-fix iterations.

However, these fields remained null:

- `builder_input_tokens`;
- `builder_output_tokens`;
- `builder_context_usage`;
- `builder_wall_clock_seconds`.

That means the metrics can answer questions such as "which external evidence route was expensive?" much better than "what did the entire accepted slice cost end-to-end?"

Capturing builder-side usage would improve optimization analysis, but it should not become a blocker for engineering work. Missing measurements should remain `null`, not estimated.

---

## 10. Quota behavior: useful reliability knowledge, not usable capacity

This run began when the Codex weekly meter displayed roughly 1% remaining. The slice consumed substantially more work than that number would intuitively suggest before the account eventually displayed 0% remaining with the scheduled reset unchanged.

The practical observation is that active work may continue across the point where the displayed weekly allowance reaches zero rather than being terminated immediately.

This should **not** become part of the Work Engine's resource budget or scheduling strategy.

Treat it as graceful service behavior:

> **Benefit from quota grace if it occurs; never budget around it existing.**

The run also clarified that two scarcity dimensions are distinct:

- weekly allowance pressure: whether future work can be initiated/continued;
- active context pressure: whether the current worker still has enough coherent reasoning context.

At the end, the weekly allowance was 0% while the active Codex session still had roughly 81% of its 258K context window left. Weekly percentage therefore should not be treated as a precise token budget for the current worker.

If Work Engine ever receives resource-state awareness, it should be a coarse route-selection/resumability signal only. It must never weaken acceptance conditions.

---

## 11. Context persistence is becoming a general optimization pattern

Several discoveries from this run point to the same deeper principle:

> **Do not repeatedly pay to reacquire knowledge that remains valid, but do not preserve context whose freshness or independence is the reason it exists.**

Examples:

- **Builder:** persistent across the slice because accumulated implementation context is durable and useful.
- **Independent reviewer:** fresh at episode start for independence; persistent through remediation because its own review context remains valid.
- **Repository scouts:** disposable when their exploratory history is low-durability; return bounded evidence packets.
- **Chrome Vision:** persistent transport, disposable/bounded observation packets.
- **Receipts:** persist exact bytes outside model context once produced.
- **Handoffs:** persist compact durable conclusions, not raw exploration.

This is a more precise optimization target than simply "use fewer tokens" or "use more agents."

---

## 12. What should *not* be changed based on this run

The run provides no evidence that the new decision policy should be tightened into more procedure. In fact, the adaptive behavior was one of the strongest successes.

Do not respond to these findings by adding a mandatory reviewer ladder, rigid evidence-agent sequence, fixed query plan, or large new supervisor checklist.

The following choices were validated by this run and should remain unless later evidence contradicts them:

- objective + invariants + acceptance conditions are authoritative;
- routes remain revisable hypotheses;
- Sol low is a credible default builder effort when evidence capabilities bound the problem;
- independent review is conditional on consequence/uncertainty, not universally removed for cost;
- deterministic gates establish observed test state;
- real UI inspection is proportional to what can be inspected truthfully without corrupting user state;
- review findings must be repaired and revalidated when they expose real semantic defects;
- missing metrics remain null rather than invented;
- user attention is reserved for actual judgment/authority boundaries, not ordinary route corrections.

---

## 13. Prioritized follow-up actions

### P0 - Make receipt persistence mechanical

Stage terminal receipts outside model context before validation and append. Ensure exact bytes survive compaction/retry. Add regression coverage for the two observed schema failures.

### P0 - Audit schema/skill correspondence

Check receipt docs, schema docs, validator behavior, examples, enum names, provider provenance, and version references as one correspondence seam. Clarify or rename `replacement_route` semantics.

### P1 - Persist reviewer context through remediation

Keep the initial independent review fresh, then reuse that isolated reviewer until acceptance or material boundary change. Measure token, cache-read, cost, wall-time, and finding-quality differences against fresh-review-every-round runs.

### P1 - Complete and benchmark `repo-search`

Move routine repository evidence away from the legacy combined Claude evidence/review provider. Preserve independent model review as a separate capability. Compare total accepted-slice cost and defect yield, not call count alone.

### P1 - Verify Chrome Vision broker lifecycle

Determine whether the observed background terminals correspond to live broker/session accumulation. Fix only if actual transport reuse is failing.

### P2 - Keep full deterministic output out of builder context

Route full suites and other high-volume mechanical checks through `run_gate.py`; expose raw failure output only when diagnosis requires it.

### P2 - Capture builder-side usage when available

Populate builder token/context/wall-time metrics from observed system data if a reliable source exists. Do not infer missing values.

### P2 - Treat quota pressure only as coarse state

If quota awareness is eventually exposed to the decision policy, use it to favor cheaper credible routes and clean resumability. Never treat displayed remaining percentage as precise available compute and never lower the acceptance bar.

---

## 14. Experiments worth running over the next 20-30 slices

Do not optimize from this single run alone. Use it to define measurements.

Track:

- accepted-slice end-to-end wall time;
- builder effort level and escalations;
- builder context usage when observable;
- repository-evidence calls and external-provider cost;
- review episode count versus review call count;
- review findings by severity and whether they were genuinely task-caused;
- review repair iterations;
- late semantic rejections;
- placement reconsiderations;
- deterministic gate reruns;
- UI inspection coverage and limitations;
- metrics append failures/retries;
- context compactions during active slices;
- repeated evidence acquisition that did not change a decision;
- defects discovered after acceptance.

The most useful qualitative annotation remains:

> **Why did it do that, and did the extra cost change the decision or increase trustworthy acceptance?**

That question will reveal where the Work Engine needs another capability, where an existing capability is being overused, and where apparent expense is actually buying quality.

---

## Final assessment

This run did not expose a failed Work Engine design. It exposed a successful decision architecture sitting on top of several still-expensive mechanical seams.

The builder adapted rather than obeying a brittle script. Independent review found two real defects. Validation remained truthful when live Detection inspection would have required unwanted user-state mutation. The system reached a strong accepted result while the builder remained at low reasoning effort.

The remaining inefficiencies are unusually actionable:

```text
knowledge reacquired unnecessarily  → persist bounded context
repository retrieval uses judgment  → separate repo-search from review
receipt exists only in model memory → stage exact artifact mechanically
schema wording invites interpretation → tighten semantic ownership/names
bulk deterministic output hits model → terminate it at run_gate
persistent browser transport may duplicate → verify actual lifecycle
```

The direction should therefore remain the same:

> **Give the model clear objectives, invariants, consequences, and capable tools; let it choose the route. Then make the infrastructure cheap enough that it does not have to repeatedly pay for knowledge or mechanics it already owns.**

This run is strong evidence that the flexibility is producing useful behavior. The next phase should improve the seams around that behavior without proceduralizing it back out of existence.
