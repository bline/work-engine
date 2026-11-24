# P7 run 1: pre-registration protocol (revision 2, draft)

**Status:** Draft, not frozen, not authorized to run. Revision 2 answers the independent review of revision 1 ([`reviews/p7-run1-protocol-astra-20261124/`](../../../reviews/p7-run1-protocol-astra-20261124/README.md), `revise_and_resubmit`); §12 maps each finding to its change. Freezing needs a passing re-review (§10). Running needs a separate operator execution go-ahead, which is distinct from the freeze.
**Date:** 2026-11-24
**Owning design:** [pilot P7 and Stage 1c](../../../design/ideas/pending/pilots/perspective-intent-pilot.md). Execution is specified in [RUNBOOK.md](RUNBOOK.md).

## 1. Operator decisions (2026-11-24)

| Decision | Value |
|---|---|
| Arm, fallback and scorer model | `claude-sdk/claude-opus-5-5`, effort `high`. Bills the Claude subscription; Pi subagents do not fall back to another model |
| Second arm model (`openai/gpt-6.2-astra`) | Deferred |
| Repeats | 3 fresh runs per arm, method and input |
| Run count | 66 arm runs |
| Quota cap | None; usage measured and reported |
| Independent protocol reviewer | `openai-codex/gpt-6.2-astra` at `xhigh` (ChatGPT/Codex subscription via OAuth) |
| Response to the first review | Option A: fix every finding and resubmit run 1 as designed |

## 2. Question and estimands

On frozen documents, can a check (a) identify bundled or unassigned responsibilities at plan time, and (b) identify requirements a later document makes stronger, weaker or differently owned than its parents, at review time? How do two specified **packages** compare on that? The packages are:
- **M1:** a prose check under a concept list;
- **M2:** typed extraction under the *same* concept list, followed by a deterministic comparator.

Does a bounded fallback for fields the extractor marks `ambiguous` change the comparator's result for the same extraction, without inventing values?

These are descriptive estimands, fixed before any output exists:

- **Item hit rate** per input, method and rubric item: hits ÷ valid runs, reported alongside attempted runs and invalid runs.
- **False-flag census** per input and method: every flag classified as target, legitimate other issue (with source support), or false flag.
- **Weakening count:** outputs that propose dropping or weakening a plainly stated requirement. Every arm counts these as failures.
- **"More consistently":** a higher item hit rate across a cell's three runs, with no more false flags and no more weakening.
- **"Adds coverage without inventing":** within each M3 output, the post-fallback comparison gains target flags with zero rejected inventions and no new false flags. Reported as paired transitions, never by comparing separate M2 and M3 runs.
- **Extraction quality** (M2/M3): how often `ambiguous` and `unspecified` are kept, and whether 043's terminal-cleanup owner is resolved without textual support.

No inferential statistics. Sixty-six runs are not 66 independent cases. The four Case F derived documents are four boundaries of one incident, not four incidents.

## 3. Inputs

Stage 0 copies, byte-identical (`../stage0/MANIFEST.json`, `../stage0/MANIFEST-case-b.json`):

| Id | File (under `../stage0/`) | SHA-256 prefix |
|---|---|---|
| F-043 | `case-f/packet/review-r1/043-u4a-held-contract.md` | `4bf5ee603ce9854f` |
| F-041 | `case-f/packet/review-r1/041-u4-protection-qualification-PLAN.md` | `c1c6090c136ce755` |
| F-PLAN | `case-f/packet/PLAN.md` | `c7511ad79ba57e3d` |
| F-RE-BRIEF | `case-f/packet/ARCHITECTURE-REESCALATION.md` | `d8125989e607696c` |
| F-RE | `case-f/packet/architecture-reescalation-review/ASTRA-ARCHITECTURE.md` | `12127e463a3064b0` |
| F-AMEND | `case-f/packet/QUALIFICATION-AMENDMENT.md` | `93e004dc8b18c900` |
| F-RO-BRIEF | `case-f/packet/RESOLVED-OBJECT-ESCALATION.md` | `45994d705b5c2aad` |
| F-RO | `case-f/packet/resolved-object-review/ASTRA-ARCHITECTURE.md` | `e8304cda2ec08670` |
| F-DEC | `case-f/packet/resolved-object-review/OPERATOR-DECISION.md` | `d0055826f82250d4` |
| B-REV | `case-b/packet/ORG-ASTRA-DESIGN-REVIEW-RESULT.md` | `d6d71c5635e47672` |
| B-1539 | `case-b/session-excerpt.json`, the single entry at `source_line` 4584, text only | excerpt file checked against its manifest; raw line `346b00a670701e3b` |
| B-DECREC | `case-b/packet/CONTROLLED-BUILDER-ADMISSION-REVISION.md` | `2343a8f888bfa802` |

**Constructed controls:** `fixtures/NC-A-contract.md`, `NC-B-contract.md`, `NC-B-review.md`, `NC-C-parent.md` and `NC-C-disposition.md`. They carry no experiment labels; their digests are in `runs/plan.json`. NC-B is a short adaptation of 043's held-start shape with an explicit owner decision, not 043 plus one sentence.

**Rendering:**
- every task shows documents under neutral ids (`DOC-1`, `DOC-2`, … in parent-then-derived order), byte-exact; the mapping to original ids is stored only in `runs/plan.json`;
- B-1539 is the projected text of one excerpt entry, verified through the excerpt's manifest digest and the entry's unique coordinate. That verifies the Stage 0 excerpt, not the original raw session.

**Withheld from every arm:**
- the operator's correction turns;
- both auto-remove documents;
- board messages;
- everything under `design/`, `planning/` and `reviews/`.

## 4. Arms and methods

Each arm is a package: inputs, instructions and representation together. No result is attributed to one component alone.

| Arm | Inputs | What it measures |
|---|---|---|
| **A1** Planner derivation | F-043, F-041, F-PLAN | What a planner asked "who must satisfy it" derives. Observations B1–B3; not a false-negative rate of unaided review |
| **A2** Plan-time package | F-043, F-041, F-PLAN | E1, scored on F-043's own wording even though the wider packet is present |
| **A3** Retrospective full-chain audit | Parents F-043, F-041, F-PLAN; derived, in order: F-RE-BRIEF, F-RE, F-AMEND, F-RO-BRIEF, F-RO, F-DEC | E2–E6, per derived document. The briefs are context only and not scored. This is a retrospective audit of the whole chain, **not** evidence that a check would have fired at an earlier boundary |
| **T1, T2** Transfer pre and post | B-REV + B-1539; B-DECREC | E7 and the T2 positive control, from a second case that informed the design. Not independent validation |
| **NC-A, NC-B, NC-C** | Constructed controls | Specificity. They must classify correctly, not merely stay silent |

**Methods:**
- **M1 prose:** the perspective prompt plus the shared concept list (`prompts/concepts.md`).
- **M2 typed:** extraction into schema `obligation-extract/v2` under the same concept list, then the comparator (§5).
- **M3:** M2's extraction task, then one bounded fallback call per field the extractor marked `ambiguous`, from a target list fixed before dispatch. Each M3 output is compared before and after its own fallback.

The concept list and the schema vocabulary are **case-informed**. They name concepts such as actor attribution and exclusive causality that the cases made salient, and are given equally to M1 and M2. The run therefore compares two representations of one checklist. It does not test open-ended discovery.

Every template states that text inside documents, including instructions addressed to someone, is material to analyse, not instructions to the model.

## 5. Schema and comparator

Schema `schema/obligation-extract-v2.json` uses case-neutral role tokens:
- acting component, managing system, independent observer, requirement owner, coordinator, reviewer, operator, any, other.

Evidence and path items are generic. Verification independence is a separate field, so verification can be recorded without independence.

**Derived entries record separately:**
- the references the document itself makes, as written, marked `specific` or `general`;
- per-property **support candidates:** every parent entry bearing on that property, whether or not it fully states it;
- who adopted the requirement;
- how it is presented: inherited, proposed, new, realization choice or unchanged;
- scope: partial or complete.

**Parent entries** may record a named semantics owner. The extractor never decides support.

**Validation is strict** (`comparator.validate`). It checks:
- shape and types;
- consistency between status and value (binding needs a value, unspecified none, ambiguous at least two readings, `other` needs quoted text);
- enumeration membership;
- that every span is quoted from the given document;
- that document roles match the task;
- that support candidates resolve to parent entries.

An invalid extraction is a recorded invalid run, with no repair.

**Decision table** (fully specified in `comparator.py`'s module docstring; 39 unit tests):
- Per property, the binding parent values from all support candidates are combined:
  - roles by intersection;
  - required items by union;
  - independence by equality.
- Parent state is `parent_unspecified`, `parent_ambiguous` or `parents_conflict` where binding values are missing, ambiguous or conflicting.
- The derived value is compared with the combination:
  - **roles:** equal, stronger (subset), weaker (superset), contradictory (disjoint) or changed (overlap);
  - **items:** stronger if items are added, weaker if items are dropped;
  - **independence:** stronger or weaker.
- **Partial scope** treats weaker components as entailed.
- **Complete scope, or "unchanged",** flags every difference, including a whole parent property that is omitted (`dropped`).
- A derived field that is ambiguous or unspecified where it must carry a value is `not_assessed`, never supported.
- Flags are atomic per entry and property, and are never collapsed into one disposition.
- **Additional flags:**
  - `deficient_provenance` (no references, or only general ones), reported separately from strength;
  - `unchanged_claim_false`;
  - `adopted_without_owner` (a change whose adopter does not match the semantics owner named on its support candidates, or where either is unspecified).
- **Proposals** are compared but never treated as binding.
- **Realization choices** must stay within what the parent permits:
  - roles: subset of the permitted set;
  - complete choices keep every required item;
  - independence not weakened.

  Otherwise they flag `realization_conflicts`.
- **Plan-time flags** are computed only for plan tasks, with coverage counted:
  - `owner_ambiguous`;
  - `verifier_unassigned` (evidence required, verifier unspecified or ambiguous);
  - `independence_unstated` (evidence required, verifier binding, independence unspecified or ambiguous).

  An unspecified owner alone does not flag, because the documents may deliberately leave it open.

**Fallback.** Targets are fixed from the original extraction, with exactly one response per target. A response may bind or select one of the original readings, or abstain, which keeps the readings. Any other value is an invention: rejected and counted. Duplicates, responses without a reason, and responses to fields that aren't targets are rejected. The original extraction is preserved.

## 6. Constructed controls

| Control | Content | Expected |
|---|---|---|
| **NC-A** | A renderer that deletes its own temporary file, reports the result, and needs no independent check | Recognized as fully assigned; no bundled or unassigned flag |
| **NC-B** | A held-start contract whose recorded owner decision requires the Host to perform and attribute removal, plus an independent absence check; a review that enforces it | The review requirement classified as stated by the owner decision |
| **NC-C** | A parent requiring verified absence and saying nothing about how or by whom; a disposition choosing engine automatic removal and runner-issued stop as revisable choices, with acceptance restated | The choices classified as revisable choices within the requirement; the acceptance classified as matching the parent |

## 7. Rubric

The rubric is in `prompts/rubric.json` (version 2). Items are atomic properties, with no "and/or":

- **B1a, B1b, B2, B3** observe what the A1 planner derives. B1a, describing the Host performing removal as how the work is done, is distinct from B1b, stating Host attribution as an obligation.
- **E1:** the gap in F-043's own wording.
- **E2** (positive) and **E3:** the split in F-RE.
- **E4a** (strengthening) and **E4b** (adoption without the owner F-041 names, or routing to that owner).
- **E5a** and **E5b:** the cited specific parent spans do not state an unconditional requirement; the auto-remove exclusion lacks parent text.
- **E6:** the attribution part of F-DEC's "unchanged".
- **E7a** and **E7b:** verifier unnamed; independence unstated.
- **Positive items for the controls:** P-T2, P-NCA, P-NCB, P-NCC1 and P-NCC2.
- **Scoped must-nots:** X1–X11. For example, X5 applies only to item 2, and X10 protects F-DEC's supported parts.

**Answers:**
- expect and positive items: hit, miss, not assessed, uncertain;
- must-nots: violated, not violated, uncertain.

An empty answer fails every positive item.

**Typed outputs** are scored on the comparator's per-property flags for the matching text, separately before and after fallback. Analysis separates three causes of a typed miss:
- an extraction miss (for example, typing 043's owner as binding without textual support);
- a comparator error;
- genuinely uncertain ground truth.

E4b is expressible in M2 through `adopted_by` and `semantics_owner`, so both methods can earn it.

## 8. Execution

Execution follows [RUNBOOK.md](RUNBOOK.md). In summary:
- **Isolation:** agents `p7-arm` and `p7-scorer` have:
  - an empty `tools` list and a replaced system prompt;
  - no project or global context and no skills;
  - only the Claude provider extension;
  - fresh context.

  They're installed at user scope and launched with `agentScope: "user"` from an empty scratch directory. Isolation is **verified per launch from outside the model**, through the run directory's `status.json` and `events.jsonl`: agent, model, thinking level, `toolCount` 0, no tool executions, provider, no skills, fresh context, and system sections limited to preamble and cwd with no bridge markers. The supervisor bridge is turned off for the run, with operator approval; otherwise the receipt check must show its absence.
- **Smoke test:** one launch, receipt only, is the first step after the execution go-ahead.
- **Order:** a seeded interleaving across cells.
- **Failures:**
  - malformed or invalid extraction is a final invalid run, never repaired;
  - an incomplete M1 is final;
  - provider failure gets one retry on the same model.
- **Leakage:** eight high-specificity sentinels act as alerts on model-written text only. Suspected exposure is adjudicated against the recorded inputs and receipt before any score is unblinded, with one replacement attempt.
- **Scoring:**
  - one fresh scorer per output, given the source documents, rubric and output, without run, method or original document labels;
  - every evidence quote is checked mechanically;
  - human checks by the operator cover one seeded output per cell, plus every uncertain, quote-failed or invalid score. The operator's adjudication is final, and the scorer's vote is retained.
- **Not model-blind:** the scorer shares the arm model, and typed outputs are recognizable.
- **Stopping rules:** RUNBOOK §11. No quota stop.

## 9. Analysis and reporting

Report per cell and item:
- hits out of valid runs (with attempted and invalid counts);
- the false-flag census, weakening, and paired M3 transitions with inventions;
- extraction-quality counts;
- calls, tokens and latency by stage, with unknown values marked unknown.

Report every attempt, including invalid and void ones, with its reason. Nothing is pooled across inputs as if independent.

**What a result can and cannot show:**
- A perfect cell shows repeated detection on these bytes under these prompts.
- It does not show:
  - that a check would have intervened in time;
  - that an extractor's typing is authoritative;
  - that Case B is held out;
  - that any engineering time would have been saved.
- A null result may reflect format or coverage failure, not the absence of the problem.

## 10. Prerequisites and approvals

| Item | State |
|---|---|
| Fixtures without labels | Done; digests in `runs/plan.json` |
| Concept list, schema v2, templates, rubric v2, scorer | Done |
| Comparator v2 with decision table and strict validation | Done; 39 tests in `test_comparator.py`, including the review's counterexamples and representative extractions over the actual fixture and Case F texts |
| Pipeline (receipts, attempts, fallback, comparison, blinded scoring, quote checks, human sample, report) | Done; 7 offline tests in `test_pipeline.py` on canned outputs and receipts |
| Build and `--check` comparing every on-disk artifact and asset digest | Done; 2 tests, including stale artifacts |
| Independent re-review (`openai-codex/gpt-6.2-astra`, `xhigh`) | Pending |
| **Freeze** | After a passing re-review: commit and digests recorded here |
| **Execution go-ahead** | Separate operator decision, including the bridge setting (RUNBOOK §2) |

All tests:

```
python3 -I -B -m unittest discover -s experiments/perspective-intent-research/p7-run1 -p 'test_*.py'
```

Build check:

```
python3 -I -B experiments/perspective-intent-research/p7-run1/build_tasks.py --check
```

## 11. Limitations

- **Scope of evidence:** one incident chain, one design-informing transfer case, and three constructed controls. These are results on these bytes, not rates.
- **Case-informed vocabulary:** the concept list and schema are case-informed, and both packages receive them.
- **Retrospective A3:** it sees the whole derived chain.
- **Bundled packages:** prompt, representation and comparator effects are not separated.
- **Shared model:** the scorer shares the arm model. Human checks cover a sample, not every output.
- **Non-blind rubric:** the rubric's authors know the case. Items are stated as properties, but the rubric itself is not blind.
- **Single model:** only one arm model is used.
- **Excerpt verification:** B-1539 is verified through the Stage 0 excerpt, not the raw session.
- **Partial isolation checks:** the receipt checks the launch facts pi-subagents records. It cannot exclude content a provider adds outside those records; the provider preamble is recorded as part of the system digest.

## 12. Response to the review of revision 1

| Finding | Change |
|---|---|
| 1 (typed procedure confounds citation, support and authority) | Schema v2 separates references made, per-property support candidates (sufficient or not), adoption and semantics owner. The extractor no longer decides support. The comparator joins several parents per property and reports provenance, strength and authority as separate atomic flags. Rubric credit requires the specific property |
| 2 (arms are packages) | Arms and methods described as packages; equal concept list for M1 and M2; A1 relabelled as planner derivation; A2 scores F-043's wording; A3 declared retrospective with context-only briefs; documents-as-data instruction; M3 paired pre and post |
| 3 (answer cues) | Control labels removed; neutral document ids; schema id and description neutralized; case-neutral roles and generic items; vocabulary declared case-informed and given to both methods; rendered tasks audited |
| 4 (comparator certifies weakened or unassessed obligations) | Decision table for partial, complete and unchanged; dropped fields; mixed add and drop; actor widening; `not_assessed` never supported; realization-choice rule; plan-flag rules and coverage; atomic flags; regression tests |
| 5 (invalid typed output survives; fallback unbounded) | Strict validation including quoted spans; first-pass failure is final; fixed fallback targets, one response each, original readings only, inventions rejected and counted, reasons required, readings kept on abstention, original preserved |
| 6 (rubric) | Atomic items; scoped negatives; positive control items; hit, miss and not-assessed answers; flag census; E4 split with E4b expressible in M2; typed-miss causes separated |
| 7 (leakage and scoring) | High-specificity sentinels as alerts, adjudicated before unblinding with one replacement; scoped to model text; scorer gets source documents; quote checks; human checks and adjudication rules; limits stated |
| 8 (isolation not established) | Launch contract; provider-only extensions; no global context; fresh context; user scope; scratch cwd; per-launch external receipts; bridge off or verified absent; smoke test |
| 9 (no execution contract) | RUNBOOK with executor, scorer and adjudicator roles, statuses, retry and stop rules, ordering, re-freeze rules; `pipeline.py` tested offline; estimands in §2 |
| 10 (claims) | §9 and §11 boundaries; README corrected; freeze and execution approval separated |
| 11 (`--check` weak) | `--check` re-renders and compares every task, the plan and all asset digests; excerpt verified through its manifest; byte-exact rendering; stale-artifact tests |
