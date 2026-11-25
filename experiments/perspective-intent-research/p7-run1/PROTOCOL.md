# P7 run 1: pre-registration protocol (revision 3, draft)

**Status:** Draft, not frozen, not authorized to run. Revision 2 answered the review of revision 1 (§12). Revision 3 answers the re-review of revision 2 (§13), and moves launches to the Claude subscription container lane after the first smoke (§14); both reviews are in [`reviews/p7-run1-protocol-astra-20261124/`](../../../reviews/p7-run1-protocol-astra-20261124/README.md). The disclosed, uncounted calibration batch has run (§15): the receipt values are pinned and the quote policy is confirmed, and it found a schema presentation defect that is not yet fixed. Freezing needs that calibration and a passing review (§10). Running needs a separate operator execution go-ahead, which is distinct from the freeze.
**Date:** 2026-11-24
**Owning design:** [pilot P7 and Stage 1c](../../../design/ideas/pending/pilots/perspective-intent-pilot.md). Execution is specified in [RUNBOOK.md](RUNBOOK.md).

## 1. Operator decisions (2026-11-24)

| Decision | Value |
|---|---|
| Arm, fallback and scorer model | `claude-sdk/claude-opus-5-5`, effort `high`. Bills the Claude subscription through role_host's container lane, which has no fallback model and refuses rather than use extra usage (2026-11-25, §14) |
| Second arm model (`openai/gpt-6.2-astra`) | Deferred |
| Repeats | 3 fresh runs per arm, method and input |
| Run count | 66 arm runs |
| Quota cap | None; usage measured and reported |
| Independent protocol reviewer | `openai-codex/gpt-6.2-astra` at `xhigh` (ChatGPT/Codex subscription via OAuth) |
| Response to the first review | Option A: fix every finding and resubmit run 1 as designed |
| Response to the re-review | Fix logic and custody now; a disclosed, uncounted calibration batch (smoke plus one run per task) settles the receipt digests and quote policy; then one further review |

## 2. Question and estimands

On frozen documents, can a check (a) identify bundled or unassigned responsibilities at plan time, and (b) identify requirements a later document makes stronger, weaker or differently owned than its parents, at review time? How do two specified **packages** compare on that? The packages are:
- **M1:** a prose check under a concept list;
- **M2:** typed extraction under the *same* concept list, followed by a deterministic comparator.

Does a bounded fallback for fields the extractor marks `ambiguous` change the comparator's result for the same extraction, without inventing values?

These are descriptive estimands, fixed before any output exists:

- **Item hit rate** per input, method and rubric item: hits ÷ valid runs, reported alongside attempted runs and invalid runs.
- **False-flag census** per input and method: every flag classified as target, legitimate other issue (with source support), or false flag.
- **Weakening count:** outputs that propose dropping or weakening a plainly stated requirement. Every arm counts these as failures.
- **"More consistently":** a higher item hit rate over attempted runs (valid-run rate alongside), with no more false flags and no more weakening (§9).
- **"Adds coverage without inventing":** within each M3 output, post gains a target hit over pre, loses none, has no out-of-list fallback reply and no new false flag (§9). Reported as paired transitions, never by comparing separate M2 and M3 runs.
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
- **M2 typed:** extraction into schema `obligation-extract/v3` under the same concept list, then the comparator (§5).
- **M3:** M2's extraction task, then one bounded fallback call per field the extractor marked `ambiguous`, from a target list fixed before dispatch. Each M3 output is compared before and after its own fallback.

The concept list and the schema vocabulary are **case-informed**. They name concepts such as actor attribution and exclusive causality that the cases made salient, and are given equally to M1 and M2. The run therefore compares two representations of one checklist. It does not test open-ended discovery.

Every template states that text inside documents, including instructions addressed to someone, is material to analyse, not instructions to the model.

## 5. Schema and comparator

Schema `schema/obligation-extract-v3.json` uses case-neutral role tokens:
- acting component, managing system, independent observer, requirement owner, coordinator, reviewer, operator, any, other.

`any` records a party the text deliberately leaves open, and is never combined with another role. Evidence and path items are generic. Verification independence is a separate field, so verification can be recorded without independence.

**Derived entries record separately:**
- the references the document itself makes, quoted, marked `specific` or `general`, each with the properties it is offered for;
- per-property **support candidates:** every parent entry bearing on that property, whether or not it fully states it;
- optional `candidate_alignment`, marking a property whose several candidates apply to the same thing under the same condition;
- for a complete or unchanged entry, its **baseline:** the parent entries it replaces or restates;
- who the document says decided or adopted the requirement, not a party that merely states or reviews it;
- how it is presented (inherited, proposed, new, realization choice or unchanged), and its scope (partial or complete).

**Parent entries** may record a semantics owner, with every name the documents use for it, the document that says so, and a quoted span. The extractor never decides support.

**Validation** (`comparator.validate`) separates fatal shape errors from quote failures.
- **Shape errors** cover:
  - types, including malformed JSON values;
  - status and value consistency;
  - enumerations;
  - duplicate readings and `any` mixed with other roles;
  - document roles;
  - reference targets and properties;
  - candidates and baselines that resolve to parent entries.
- **Quote failures** are collected for every relied-on quote: entry spans, reference texts, owner spans, owner names and `other_text`. They use the named normalization `quote-norm/v1`: straight quotes, Markdown emphasis characters removed, whitespace collapsed, contiguous passage only.
- `QUOTE_POLICY` decides whether any quote failure invalidates the extraction. It is `strict`, confirmed from calibration (§15). Quote failures are recorded and reported either way.

An invalid extraction is a recorded invalid run, with no repair.

**Decision table** (fully specified in `comparator.py`'s module docstring):
- **Assessed properties:**
  - every property the derived entry types as binding, selected or ambiguous;
  - for a complete or unchanged entry, also every property its baseline binds. These are enumerated from the baseline, independently of the candidate keys the extractor supplied. Without a baseline the entry is `baseline_missing`.
- **Parent value** of a property, from its candidates (plus the baseline for complete or unchanged entries):
  - any ambiguous candidate gives `parent_ambiguous`; a binding candidate does not hide an ambiguous one;
  - no binding candidate gives `parent_unspecified`, or `no_candidates` when nothing is linked;
  - several binding candidates combine only if they all belong to the baseline or the property is marked `same_condition`, and otherwise give `alignment_unresolved`;
  - roles combine by intersection (`any` is neutral), required items by union, independence by equality. A failed combination gives `parents_conflict`.
- **Comparison** with a binding derived value records the added and lost values:
  - **items:** added are stronger, lost are weaker;
  - **roles:** losing a permitted party narrows (stronger), adding one widens (weaker), and disjoint sets are contradictory;
  - **independence:** stronger or weaker.
- **Partial scope** marks weaker components `entailed`. **Complete scope or unchanged** flags every difference, including a baseline property the entry leaves unspecified (`dropped`).
- **Status changes:**
  - a parent's binding value restated as a revisable choice is `status_weakened`;
  - a binding derived value with no binding parent is `parent_unspecified`, counted as a change: an addition without parent text;
  - an ambiguous derived value is `not_assessed`.
- **Reported separately per entry:**
  - **Coverage:** properties assessed and established.
  - **Entailment:** `supported` only when every component is equal or entailed; `changed` when any change exists; otherwise `not_established`. An entry with nothing assessed is `not_assessed`, never supported.
  - **Citation per property:** `adequate` (a specific reference to a parent document offered for that property), `unresolved_target`, `general_only` or `none`. Anything but adequate flags `deficient_provenance`. Citation never changes entailment.
  - **Authority per changed property:** owners come from the candidates that bind that property, never pooled across properties. The outcome is `matched`, `mismatched` (`adopted_without_owner`), `owner_decision_not_shown` (owners known, no adopter stated) or `not_established` (`authority_not_established`). "No decision shown" is not "the owner did not approve".
- **Other flags:**
  - `unchanged_claim_false` (an unchanged entry with any change).
  - **Proposals** are compared and recorded, and never flagged.
- **Realization choices:**
  - Binding fields inside a choice are compared as requirements.
  - A selected value must stay within the parent: roles a subset of the permitted set; a complete choice keeps every required item; required independence not weakened.
  - A parent that leaves the property unspecified permits any choice. Unlinked, ambiguous, conflicting, unaligned or `other` comparisons are not established, never conflicts.
  - Standing is `realization`, `realization_conflicts`, `realization_not_established` or `realization_not_assessed`.
- **Plan-time flags** (plan tasks only; `plan_entries_extracted` counts extracted rows, not opportunities):
  - `owner_unresolved`: outcome owner ambiguous, or unspecified on an entry that binds evidence or a path fact. A deliberately open party is typed `any` and does not flag.
  - `verifier_unassigned`: evidence required, verifier unspecified or ambiguous.
  - `independence_unstated`: evidence required, independence unspecified or ambiguous, whatever the verifier's status. It records that independence is unstated, not that it was required.

**Fallback.** Targets are fixed once from the original extraction and bound to its attempt. Each reply is paired with the target at its own index, and validated:
- the same entry and property;
- a legal status;
- string values;
- a non-blank reason.

Each target's outcome is one of:
- `resolved_within_list` (records the before and after values and the reason);
- `abstained` (readings kept);
- `out_of_list` (an invention: rejected and counted);
- `invalid_response`;
- `missing`.

A comparison needs a recorded reply for every target. A resolution among the listed readings is not syntactically an invention, but may still lack textual warrant. Every resolution therefore goes to human review (RUNBOOK §10).

## 6. Constructed controls

| Control | Content | Expected |
|---|---|---|
| **NC-A** | A renderer that deletes its own temporary file, reports the result, and needs no independent check | Recognized as fully assigned; no bundled or unassigned flag |
| **NC-B** | A held-start contract whose recorded owner decision requires the Host to perform and attribute removal, plus an independent absence check; a review that enforces it | The review requirement classified as stated by the owner decision |
| **NC-C** | A parent requiring verified absence and saying nothing about how or by whom; a disposition choosing engine automatic removal and runner-issued stop as revisable choices, with acceptance restated | The choices classified as revisable choices within the requirement; the acceptance classified as matching the parent |


## 7. Rubric

The rubric is in `prompts/rubric.json` (version 3). Items are atomic properties, with no "and/or":

- **B1a, B1b, B2, B3** observe what the A1 planner derives. B1a, describing the Host performing removal as how the work is done, is distinct from B1b, stating Host attribution as an obligation.
- **E1:** the gap in F-043's own wording.
- **E2** (positive) and **E3:** the split in F-RE.
- **E4a** (strengthening) and **E4b:** the change is not shown to be decided by the owner F-041 names. It is adopted by another party, no owner decision appears in the documents, or it is routed to that owner. Credit does not require knowing whether the owner approved it outside the documents.
- **E5a** and **E5b:** the cited specific parent spans do not state an unconditional requirement; the auto-remove exclusion lacks parent text.
- **E6:** the attribution part of F-DEC's "unchanged".
- **E7a** and **E7b:** verifier unnamed; independence unstated.
- **Positive items for the controls:** P-T2, P-NCA, P-NCB, P-NCC1 and P-NCC2.
- **Scoped must-nots:** X1–X11. For example, X5 applies only to item 2, and X10 protects F-DEC's supported parts.

**Answers:**
- expect and positive items: hit, miss, not assessed, uncertain;
- must-nots: violated, not violated, uncertain.

An empty answer fails every positive item.

**Typed outputs** earn an item only through a comparator flag or record on the matching text and property. A field typed without a matching flag does not count. The typed matches are:
- E1: `owner_unresolved`;
- E7a: `verifier_unassigned`;
- E7b: `independence_unstated`;
- E4b: an authority outcome naming the owner F-041 names.

`deficient_provenance` concerns citation only. It never decides an entailment item or a must-not; for example, it is not an X9 violation for NC-C's truthful restatement.

Analysis separates three causes of a typed miss:
- an extraction miss (for example, typing 043's owner as binding without textual support);
- a comparator error;
- genuinely uncertain ground truth.

## 8. Execution

Execution follows [RUNBOOK.md](RUNBOOK.md). In summary:

- **Isolation:** every launch is one fresh, single-turn Pi session on role_host's Claude subscription container lane (2026-11-25, §14):
  - a networkless container with a placeholder credential; the host egress proxy injects the real one;
  - only the pinned Claude provider extension;
  - no tools, no skills and no context files;
  - the system prompt replaced by the agent's prompt file (`p7-arm` or `p7-scorer`);
  - one request at most;
  - an empty scratch cwd.
- **Receipts:** every launch is checked from outside the model, through the role_host records (RUNBOOK §2). Findings are sorted into violations, provider failures and missing evidence (never read as zero).
  - **Launch contract:** Pi's exact argv, from the host-only turn spec.
  - **Tool availability:** the empty tool list on the request to Anthropic, from the host proxy, and no tool executions.
  - **Requests:** exactly one request admitted and settled.
  - **Actual call:** requested and served model, effort and subscription billing verdict, from the host proxy; provider, model and effort on the assistant message.
  - **Context:** system sections and preamble from Pi's events. The request's system-prompt digest and message count are compared to the values pinned from calibration.
  - **Custody:** one user turn carrying the planned task text. The output is the final assistant text, taken by the pipeline from the record, and each session id is bound to one record.
- **No subagent machinery:** the lane runs no subagent extension, supervisor bridge or acceptance contract, which were the defects of the first smoke (§14).
- **Smoke test, then a disclosed calibration batch.** The smoke launches each agent once. The calibration batch is one uncounted arm run of each task. Together they settle the pinned receipt values and the quote policy, and are followed by a re-freeze. Calibration outputs are reported as calibration and never scored.
- **Order:** a seeded interleaving across cells.
- **Failures:**
  - invalid extractions and incomplete prose (no real FLAGS section, or for A1 no numbered list) are final;
  - a provider failure or a leakage void gets **one** replacement;
  - the final attempt is the last recorded.
- **Leakage:** eight high-specificity sentinels act as alerts on model-written text. Validity is assessed independently. An adjudication is written once: coincidental restores the assessed status; exposure voids the attempt.
- **Fallback** replies are paired by index and validated. A comparison needs every target answered.
- **Scoring:**
  - one fresh scorer per output, in a versioned batch bound to attempt and comparator digests;
  - full reply validation;
  - phase-local quote checks;
  - the human check list of RUNBOOK §10.

  The operator's adjudication is final, and the scorer's vote is retained.
- **Not blind:** the scorer shares the arm model, and typed outputs are recognizable. M3's pre and post phases are judged together, in known order, and are **not** blinded from each other. Every M3 transition and every fallback resolution is human-checked.
- **Stopping rules:** RUNBOOK §11, enforced by the pipeline gate. No quota stop.

## 9. Analysis and reporting

`pipeline.py report` computes the joins. It reports, per cell, phase and item:
- answer counts;
- hit rates over valid runs and over attempted runs. A rate is undefined, not zero, when its denominator is zero.

It also reports:
- the census by class;
- weakening;
- the attempt ledger, including invalid, void and replacement attempts with reasons;
- calls, tokens and latency by stage, including smoke, calibration and replacements, with unknown values counted.

Rules fixed before any output exists:
- **Unresolved answers:** an item without a valid scorer answer or an adjudication is unresolved. It is reported, and not counted as a hit or a miss.
- **False flags:** the primary false-flag count excludes `context_only` census entries, which are reported separately.
- **"More consistently":** a higher hit rate over **attempted** runs, with the valid-run rate alongside, no more false flags and no more weakening. One valid run out of one does not beat two out of three with no invalid runs. A tie is reported as a tie.
- **"Adds coverage without inventing"** holds for an M3 output only if:
  - post gains a target hit over pre;
  - no target hit is lost (a loss is not offset by a gain);
  - no fallback reply is out of list;
  - post has no new false flag.

  Missing and invalid fallback replies are reported, and are not abstentions.

Nothing is pooled across inputs as if independent.

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
| Concept list, schema v3, templates, rubric v3, scorer | Done |
| Comparator v3: decision table, coverage, entailment, citation and authority reported separately, quote checks | Done; 56 tests in `test_comparator.py`, including every counterexample in re-review findings B–D and representative extractions over the fixture and Case F texts |
| Pipeline rev 3: receipts and custody, attempt rules, adjudication, stop gate, fallback pairing, versioned scoring, reply validation, report joins | Done; offline tests in `test_pipeline.py`, including a mixed-status batch carried through to item outcomes |
| Receipts on the container lane (§14) | Done; offline tests on synthetic host records, one per violation, provider-failure and missing-evidence class |
| Build and `--check` comparing raw task bytes, canonical plan bytes and asset digests | Done; 2 tests, including CRLF and reserialized plans |
| Smoke and disclosed calibration batch | Done 2026-11-25 (§15) |
| Pinned receipt expectations and quote policy | Done (§15) |
| Schema presentation fix (§15) | Pending the operator's decision |
| Independent re-review (`openai-codex/gpt-6.2-astra`, `xhigh`) | Third review after calibration |
| **Freeze** | After a passing review: commit and digests recorded here |
| **Execution go-ahead** | A separate operator decision |

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
- **Shared model:** the scorer shares the arm model; M3 phases are judged unblinded; human checks cover a sample plus every flagged case, not every output.
- **Non-blind rubric:** the rubric's authors know the case. Items are stated as properties, but the rubric itself is not blind.
- **Single model:** only one arm model is used.
- **Excerpt verification:** B-1539 is verified through the Stage 0 excerpt, not the raw session.
- **Candidate alignment:** the comparator does not reason about conditions. Several binding candidates combine only when the extractor marks them aligned or they form a designated baseline, so an extractor's alignment marking is trusted as typing.
- **Owner aliases:** names match only as listed by the extractor; a missed alias can read as a mismatch.
- **Receipt limits:**
  - Host-only records establish the launch, tools, request count, models, effort and billing. The user turn and the final text come from Pi's events file, written inside the container; the model has no tools to alter it, but it is not host-written.
  - The proxy logs digests, not content. The pinned system digest and message count come from the calibration observation, not from an independent derivation, and cover Claude Code's identity block and Pi's `cwd` section as well as the agent prompt.
  - The billing verdict is the provider's response header, not a quota measurement.
- **Quote strictness:** under `strict`, one transcription error invalidates a whole extraction. Attempted-run rates are reported for that reason.

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

## 13. Response to the re-review of revision 2

The re-review ([`review-r2.md`](../../../reviews/p7-run1-protocol-astra-20261124/review-r2.md), `revise_and_resubmit`) raised findings A–G. The operator chose to:
1. fix the logic and custody findings now;
2. settle the data-dependent parts with a disclosed, uncounted calibration batch;
3. then hold one further review.

| Finding | Change |
|---|---|
| A (receipt observes use, not availability; output not bound) | The receipt checks `toolsAdded` at every system message. It checks provider, model and effort on every assistant call. System content and the user wrapper are compared to digests pinned from calibration. Findings are sorted into violation, provider failure and missing evidence, with missing never read as zero. The planned task must appear in the single user turn, and the output file must equal the final assistant text. A native run id binds to one record. Bridge: no global config edit. The installed tool has no per-launch override, so positive tool and content evidence excludes the bridge, and calibration stops the run if a bridge appears. **Data-dependent:** the exact digests, pinned after calibration. *Superseded by the route change in §14, which rebuilds the receipt on host records* |
| B (supported reachable with unknown or lost requirements) | A baseline is required for complete and unchanged entries, and enumerated independently of candidate keys. Binding status is compared (`status_weakened`). Empty, ambiguous, unlinked or unaligned comparisons are never supported or conflicts. Several candidates need declared alignment. Ambiguous candidates are not hidden. Roles are decomposed into narrowed and widened parts, with added and lost values. Coverage, entailment, citation and standing are reported separately. Citation is per property, and a general citation no longer makes NC-C's restatement "flagged" |
| C (authority pooling; plan flags) | Authority is checked per changed property, with source-quoted owners. Outcomes are matched, mismatched, decision not shown or not established. The adopter means who decided, not who stated. E4b is reworded so it does not assert historical non-approval. `owner_unresolved` fires on an unspecified owner where an outcome is required, while `any` records deliberate openness. `independence_unstated` fires whatever the verifier's status. Typed credit comes through flags only. Tests cover honest typings and positive controls |
| D (provenance validation) | Every relied-on quote is checked under the named `quote-norm/v1`. Quote failures are separated from shape errors. Malformed values become extraction errors, not crashes. Duplicate readings and `any` mixed with roles are rejected. Reference targets and properties are validated. **Data-dependent:** the quote policy, confirmed in calibration. Attempted-run rates are reported |
| E (retry, adjudication, stops) | One authorized replacement, only after a provider failure or a leakage void; the final attempt is the last recorded. Leakage and validity are assessed independently, and a write-once adjudication restores the assessed status or voids the attempt. A stop gate blocks all recording, with exit 22. Real M1 FLAGS and A1 list checks |
| F (fallback pairing; scorer validation; phase blinding) | Fallback replies are validated against their own indexed target. Every target gets one logged outcome, a comparison needs a complete batch, and before and after values are recorded. Scorer replies are fully validated: input, phases, items, answers, census classes and coverage, weakening. Quotes are phase-local, `NO SUPPORTING SPAN` covers absence, and phase material is delimited. Paired, unblinded phase judging is disclosed, and every transition and resolution is human-checked |
| G (report; scorer custody) | Score batches are versioned and bound to attempt and comparator digests. An inconsistent batch is refused before any write, and old batches are kept. Comparator outputs are named by digest, so recomputation is verified and a re-freeze keeps both. `report` joins the ledger, usage by stage, final replies, adjudications, item rates, census, weakening and M3 item transitions. A mixed-status batch is tested end to end |
| Build residual | `--check` compares raw task bytes and canonical plan bytes |

## 14. Launch route change (2026-11-25)

The first smoke (`runs/smoke/1.json`, one paid call, not counted) ran through pi-subagents and failed its receipt:
- the supervisor bridge was active;
- an acceptance contract was injected;
- the system message recorded no tool manifest.

It also exposed a pipeline defect: the system digest hashed an empty `content` field rather than the sections.

The operator approved moving launches to role_host's Claude subscription container lane, after its offline qualification and a live smoke ([S3 record](../../../records/pi-container-claude-sdk-s3-20261125/README.md)). That smoke included a request with Pi's system prompt replaced, which was served on the subscription (`allowed`, overage `rejected`).

Changes:
- **Launch:** RUNBOOK §2's role_host command replaces the `subagent` call.
- **Agent files:** now hold only each agent's system prompt.
- **Receipt:** rebuilt on the host records, as summarized in §8. Tool availability now comes from the request itself, and the system check uses the sections and the proxy's request digest.
- **Commands:** record commands take the host state root and session id, and the pipeline takes the output from the record.
- **Smoke:** launches both agents.

The experimental design, inputs, rubric, comparator and scoring are unchanged.

## 15. Calibration results (2026-11-25)

Operator-approved, run on the lane of §14:
- a smoke of each agent (`runs/smoke/2.json`, `3.json`);
- one uncounted arm run of each of the 15 tasks (`runs/calibration/`).

Driver and step log: `runs/calibration/driver/`. These are 17 requests on the Claude subscription, none counted or scored.

**Receipts.** All 17 were clean in calibration mode:
- one request each, billing verdict `allowed`;
- the served model verified as `claude-opus-5-5`, effort `high`;
- an empty tool list.

Each observed value took one value across every launch, and is pinned in `receipt_expectations.json`:

| Value | Pinned |
|---|---|
| `p7-arm` request system digest | `d0d24752b108bb79…` (16 launches) |
| `p7-scorer` request system digest | `c1c3df083ce19969…` (1 launch) |
| Request message count | 2 |
| User wrapper digest | `9ab8802b2f26e424…`: the user message is the task text itself |

The environment fingerprint differs on every launch and is recorded, not pinned. The real credential occurred in no file under the host state root or `runs/`.

**Outputs.**
- All 8 prose tasks (A1, M1) were valid.
- Of the 7 typed (M2) tasks, the 4 plan-kind tasks were valid with 0 quote failures.
- The 3 review-kind tasks (`A3-F`, `NC-B`, `NC-C`) were `invalid_extraction: bad asserted_as`.
- There were no leakage alerts.
- Output sizes ranged from 1,192 to 47,994 output tokens; A3-F.M2 was the largest.

**Schema presentation defect.** The schema lists the derived-entry keys under a `"derived_only"` heading, and all three review-kind extractions nested them in a `derived_only` object. The comparator requires them on the entry itself. Flattened in a diagnostic outside the records, all three validate with 0 quote failures.

This is an instrument defect, not a model failure, and it would invalidate every review-kind typed run. The records are unchanged: they stay `invalid_extraction` as recorded.

**Quote policy.** No quote failure occurred in any of the 7 typed extractions, including the 3 flattened in the diagnostic. `strict` is confirmed.
