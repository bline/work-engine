# P7 run 1 — independent pre-freeze review

**Verdict: `revise_and_resubmit`. Freeze: BLOCK.** Keep the experiment, the source packets and the operator's 66-run budget. Do not freeze these instruments. A typed run can score well by identifying *missing citations* without detecting *stronger obligations*; a control tells the model it is a negative control; and a comparator can report `supported` when a replacement drops a binding protection. These are alternative explanations for the very results §2 asks the operator to interpret. More repeats would not fix them.

**Subject verified:** `/home/bline/code/work-engine`, `rescue-detached-20260911`, HEAD `29d843864667a4c5cdf9650750238ffbd67d4750`. `PROTOCOL.md` SHA-256 `a4de2ebc87d7d9ce7f69312e32da1ba64ea97294924613d37002038b400bc6e4`. Paths below are relative to `experiments/perspective-intent-research/p7-run1/` unless stated otherwise. F/B document IDs denote the **actual Stage 0 files** mapped in `PROTOCOL.md:28–41`; line references to them are source lines, not task-file offsets. Reviewer session metadata records `openai-codex/gpt-6.2-astra`, `xhigh`. This is a review of the design and local bytes, not a smoke test or run authorization.

## Correct and worth retaining

- **Verified by running:** the builder check printed `{"runs": 66, "unique_tasks": 15, "leakage_terms": 9}`; all **23 tests passed**. Independently hashing the 15 on-disk task files against the run rows passed; grouping those rows gave three repeats in each of 22 cells. The stored template, schema, comparator, rubric and fixture digests match the files here. I sampled the generated boundaries and byte-compared F-AMEND:7, F-DEC:5, B-1539's lock bullet and NC-C's acceptance line against their task copies. I did not read the large task texts end to end. The actual inputs are in much better shape than the *claims made for the check* (finding 11).
- The E2/E3 distinction is real and important: *report a failure of the selected Host removal even if the object is gone* is different from *produce evidence the successful removal was this Host's rather than someone else's*. The packet supports the former; a general doctrine of evidence does not by itself require the latter. The test of routing to the owner, rather than agreeing with the eventual Docker design, is worth doing. The draft correctly does not purport to authenticate owners or run a production gate (`PROTOCOL.md:20–22,93,107`).
- A single pre-specified arm provider/effort, no substitution on error, retaining ambiguous readings, and reporting outcomes conditional on the typing are sensible. See `docs/model-selection.md:49`, `docs/contracts/evidence-and-claims.md` §§3.9,12, and plan review-r2 finding N2:75–92. An arm model's parent typing is evidence **about that reading**, not an independent observation of what RL authorized.

## Findings and minimum corrections

### 1. Blocker (P0) — The typed procedure confounds citation form, semantic support and authority

`PROTOCOL.md:58,66,82–93,120–124`; `prompts/M2-extract.md:4–8`; `schema/p7-obligation-extract-v1.json:79–95`; `comparator.py:157–184`; `prompts/scorer.md:3`.

The instruction says to link a parent entry **only** when the derived document points to specific parent text **that states the requirement**. That asks the extractor to decide entailment to select the comparator's input. If the parent text is specific but does *not* entail the stronger property (precisely the experiment), a conscientious extractor has no good link under this instruction. `general` or `none` then flags before any field comparison. Conversely a correct restatement of a parent without a pinpoint can be flagged for exactly the same reason. M1 searches parents for support. These procedures do not answer the same question. F-RO:9 actually cites 043 lines and H3–H5 **as well as** §3.9 and the amendment; the issue is what those **specific** spans fail to establish. F-DEC:5 also contains both supported properties and the added attribution. A flag on the right sentence is not sufficient. The NC-C:15 true restatement cites “the cleanup requirement,” not a numbered span; the whole-document instruction risks forcing X9 on a truthful extraction. Conversely, F-RE:9,11,13,59 names ranges/authorities and argues from a realization; test `test_e2_supported_by_h3` supplies a hand-picked exact H3 link.

One entry-level `parent_entry`, not per-property evidence, cannot directly join an assertion supported in part by H3, H4 and 043, or distinguish “the amendment really says this” from “the original parent authorized this.” A derived target is simply “not found” (`comparator.py:167–169`). Under §7 it is **not** the 043 owner typing alone which determines E3–E6: general citations never consult it; `test_comparator.py:45–61,99–108` changes the **evidence and verifier too** to make E3 pass. Even with Host as owner, `actor_attributed_removal` can still lack a parent. Conversely splitting one obligation into two rows can change the worst disposition.

**Minimum:** Separate the reference actually made, the candidate supporting span(s) for each property (including specific-but-insufficient text), and the semantic comparison. Report deficient provenance, unsupported strengthening and current binding standing separately. Do not award E credit for “no citation” unless the output identifies the **particular added property**; require positive evidence on E2 and the unchanged parts. Record per-field reasons, actual linked spans/readings and conditional expectations over the fields and links **actually compared**, with extraction miss, comparator error, and genuinely uncertain ground truth distinct. If the single-parent prototype is retained, explicitly restrict it to atomic comparisons and make unrepresentable joins unassessable rather than wins.

### 2. High (P1) — The arms are useful as packages, not isolated causal tests

`PROTOCOL.md:18–22,52–71,136–145,156–167`; `prompts/{A1-planner,M1-plan,M1-review,M2-extract}.md` instructions; `build_tasks.py:65–75,129–150`; owning pilot, Stage 1c and P7:613–638.

- **A1** is a planner asked explicitly “who must satisfy it,” not an unprompted implementation or generic review. It has B1–B3 observations rather than the review endpoints. It is useful for what the planner *derives*, not a measured false-negative rate of unaided review. **A2** has three parents, not P7's 043-alone test; H4 and the ownership split can help disambiguate the wider plan. Score a gap *in 043* separately from a gap in the whole packet.
- **M1** tells the model to split the three roles, look for missing independence, general citations, claims presented as unchanged and unsupported stronger rules. **M2** asks a different question, forbids judgment, exposes domain tokens, and then flags deterministically. Not a generic prose-vs-typing ablation. Naming the perspective is legitimate for the oracle-selected pilot. Either equalize the substantive information/goals across methods or call these *two specified prompt-and-check packages*, not an effect of typing alone.
- **A3** presents all six derived files in one call (`tasks/A3-F.M2.md:210,228,306,325,346,413`). The briefs are task inputs too, with instructions such as “Test whether intended success genuinely requires causal Host removal” (F-RE-BRIEF:13). Later articles discuss the precise fact an earlier reviewer would need to discover. Reporting “in order” is not a cut before adoption (pilot:613). Large exhaustive structured outputs also impose a different omission/truncation burden. For 66 runs the cheapest fix is to choose one genuinely pre-amendment primary cut, **or** explicitly make A3 a retrospective full-chain audit, not evidence it would fire at an earlier boundary; identify scored vs context-only documents. Treat packet imperatives as quoted data, not new assignments.
- **M2/M3** share *task text*, not necessarily *extracted data*. The plan specifies independent arm calls. Subtracting 3 independent M2 totals from 3 M3 totals confounds fallback with first-pass variation. Keep the 66 runs: for **each M3 output** compute/report the ordinary comparator both before and after its own fallback; attribute only the paired field/flag changes to fallback. Zero eligible fields means no fallback opportunity, not a successful interpretation. T1/T2 are informative pre/post examples from a second, design-informing case, not independent validation; the NCs test specificity, not field prevalence.

### 3. Blocker (P0) — Model-visible answer cues invalidate the specificity test as written

`fixtures/*.md:1–3`; `fixtures/NC-B-contract.md:13`, `NC-C-disposition.md:5–15`; `tasks/NC-A.M1.md:15`, `NC-B.M2.md:114,124`, `NC-C.M1.md:16,26,32,38`; `schema/p7-obligation-extract-v1.json:11–34,37–77,80–81`; `PROTOCOL.md:136–143`.

Every NC literally announces **“constructed negative control for P7 run 1”** to the arm. Even without understanding a contract, “FLAGS: none” is a good strategy. Titles, NC document IDs and “with one owner decision added” reinforce the cue. The explicit owner decision and revisability are *necessary evidence* and should remain; the experimental labels are not. Giving NC-C automatic removal is appropriate to its assigned task, not an across-arm disclosure if isolation holds. Do not treat that mechanism as evidence of leakage from F. F-RE:11 and F-RO:32 themselves discuss automatic removal.

M2 on **every input**, including T1, sees `actor_attributed_removal`, `exclusive_causality`, `independent_absence`, `independent_termination_verification`, actor names, failure-reporting and case-derived path items. Even the direction of set comparison and the treatment of `other` are in the model-visible schema. The bare `.md` extraction template need not name the answer for the **rendered** prompt to prime it. E7's only termination-specific evidence token asserts independence even though the source does not. `test_comparator.py:87–90` actually uses that token for T1. The positive isn't guaranteed—the model must still extract—but no claim of open-ended discovery or uninformed transfer is justified.

**Minimum:** Render neutral document IDs/headings and remove control labels/status metadata. Keep the facts needed to distinguish controls. Use a case-neutral vocabulary with a way to encode verification without independence, or declare the schema a case-informed checklist intervention and give prose the same concepts for a representation comparison. Audit **rendered** prompts (including any examples and fallback inputs), not just templates. Freeze new task hashes.

### 4. High (P1) — Comparator can certify a weakened, unassessed, or mispackaged obligation

`PROTOCOL.md:75–93,127–131`; `comparator.py:33–36,109–143,146–193`; schema:11–34,92–94. Following the branches on paper (no extra comparator runs):

| Case | Current result | Consequence |
|---|---|---|
| Binding actor parent `{host}`; **replacement** permits `{host,container_engine}` or `{any}` | `supported` (133–134), regardless of `replaces_parent` | The replacement now accepts an execution the parent forbids. For a partial, non-exclusive assertion a weaker consequence can be entailed; for replacement/equivalence this loses a constraint. |
| Parent requires `{independent_absence,removal_failure_reported_as_failure}`; complete replacement retains only `independent_absence` | `contradictory` for a strict subset **within that field**, correctly. But omit the entire property, or replace both with one old + one novel item | Omitted parent fields are never visited; mixed add/drop returns `requires_owner_decision` for the add before testing the loss. The protected evidence can disappear unreported. |
| `unchanged`, specific parent; derived fields empty, all ambiguous, or a proper subset with no `replaces_parent` | `supported`/“no binding fields asserted” or ordinary subset entailment | “Unchanged” adds a reason **only after failure**. A compatible partial assertion is not an equality check. Similarly a skipped ambiguous property can hide beside one supported property. |
| Whole entry `realization_choice`/`proposed` with some binding fields | Nonflag `realization`/`proposal` **before comparison** | Correct for a genuinely revisable conforming choice or an unadopted proposal, not evidence that a conflicting choice satisfies a binding parent, nor that an adopted rule was only a proposal. A model's own label is the decisive judgment. M2:4 doesn't even explicitly require entries for pure choices; omitting NC-C's choices can look like correct classification. |
| Required outcome but no nonempty binding `acceptance_evidence`, or owner assigned only as `selected_realization` | No plan flag on that basis | `plan_flags` fires only on binding **evidence** + unspecified/ambiguous owner or verifier; it ignores initiator, takes a chosen actor as enough and has no “not applicable.” Conversely any unrelated binding evidence with an intentionally open owner gets `unowned_outcome`. It runs on review parents too, including NC-C's deliberately unspecified removal mechanism. Not a general test of bundling or of all M1:3–8 criteria. |

Where evaluated: absent/`unspecified` or only selected parent field → owner decision; ambiguous parent → `ambiguous_parent`; general/none → owner decision; unresolved specific target or `other` → `not_established`; opposed disjoint actor sets → `contradictory`; strict actor restriction or extra required item → owner decision; compatible partial item subset → supported. These distinctions are often sensible. But §5:89 says “permit no **more**” while also calling for a superset: the *non-strengthening* partial claim permits no **fewer** actors; required items run the opposite way. Define partial entailment, replacement and incompatibility separately. The total order `contradictory > requires_owner_decision > ambiguous_parent > not_established > supported` is implemented, with textual reasons for compared fields. It is not specified in §5, and lower-ranked non-comparability is **not** evidence a field was assessed. Multiple independent flags must not collapse into one hit. Update §5's three `asserted_as` values and citation shape to match the implemented five and entry references.

**Minimum:** freeze a decision table for partial, complete and unchanged assertions, including dropped entire fields, mixed add/drop, wildcard/overlapping/disjoint actor sets, skipped/no-binding fields, and proposal/choice with mixed status. Never default “not assessed” to “supported.” Restrict plan flags to defined opportunities and record coverage. Add regression tests for those counterexamples and multi-field aggregation using representative extractions from the *actual* fixtures, not just friendly atomic entries.

### 5. High (P1) — Invalid or vacuous typed responses survive; M3's bound is not enforced end to end

`schema/p7-obligation-extract-v1.json:1–10,79–95`; `prompts/M2-extract.md:4–10`, `M3-fallback.md:3–17`; `comparator.py:51–106,205–231`; `test_comparator.py:178–201`.

The schema is a vocabulary and illustrative shape, not executable structural validation. The code checks schema ID, duplicate IDs, some keys, roles/status/enumerations, assertion/citation kinds. It accepts `entries:[]`, `fields:{}`, binding with empty/missing `value`, unspecified with values, ambiguous without readings, `other` without text, false spans, arbitrary document IDs/roles, `general` without text and `specific` without a usable target. No validation against the supplied documents. `replaces_parent:"false"` is truthy. Missing `entries` can pass `validate` then raise `KeyError`; wrong types can throw; JSON decoding occurs before the CLI's `try`. M2 says use `other` when nothing fits, but **actors** have no `other`; `component` is not a general unknown actor. A model that omits hard entries can appear both conservative and specific.

Fallback checks that the *current* target is ambiguous; it deep-copies and revalidates enum membership. It accepts **any** schema status (including `unspecified` contrary to M3's return instructions), does not require a reason, original-reading membership or one resolution per originally ambiguous field. An `ambiguous` response discards the original `readings`; a second response to the same still-ambiguous target can then bind it. Out-of-scope requests can be logged as not applied without failing the run. No component verifies what spans the caller actually sent. Fallback has only one entry-wide quote, not necessarily the context needed to establish a field; corroboration absent from that snippet is not evidence of a new binding duty.

**Minimum:** validate exact shape, field/status/value/readings consistency, literal quotes against pinned docs, permitted roles and reference targets (distinguish invalid ID from absent authority); preserve original extraction. Fix the list of fallback targets **before** dispatch; exactly one bounded response per target, no new fields, no retry-by-repetition, preserve readings on abstention, log inputs/reason/provenance/delta. Specify whether a novel value is legal and how inventing it is scored. Test rejection/abstention and invalid JSON as well as happy paths. The 23 passing tests are a useful unit suite, not representative acceptance tests.

### 6. High (P1) — Some rubric properties need narrowing; the methods cannot earn the same credit

`PROTOCOL.md:95–132,172`; `prompts/rubric.json:5–63`; `prompts/scorer.md:3–12`. Source audit:

| Items | What the documents support; fair scoring boundary |
|---|---|
| **B1–B3; E1; X1/X2** | F-043:11,19 says “exact stop” and “exact container removal observed”; :23–25 assigns Host implementation/terminal observation and RL consumer verification. F-PLAN:3,15–18 distinguishes removal failure/unknown, requires an independent post-exit query of the original full ID on the launch daemon, and separates R1. B2/B3 and preserving H4/H3 are sound. B1 is an **observation**, not a correct answer: distinguish a planner describing the selected Host path from one inferring universal Host credit. E1 is the ambiguity of **043's wording**, not proof no verifier is assigned anywhere. Respect a defensible contextual reading. |
| **E2/E3; X3** | F-RE:9–13,17–19,59 explicitly limits the first argument to the selected path, disclaims forensic proof of the sole remover and requires `held_failed + confirmed_absent` on setup failure. H3 supports not converting the failed attempt into clean cancel, **not** an acknowledgement distinguishing “already missing”; H4 is still owed. Credit the split, not a blanket flag on attribution or a blanket rejection of the review. |
| **E4; X4** | F-041:15 assigns RL “lifecycle admission/receipt semantics” and dev-vertical the Host boundary; F-PLAN:32 gives dev-vertical local coordination/acceptance. F-RE:69 distinguishes local amendment authority from clean-close meaning. F-AMEND:3,7,16 **adopts** the positive witness citing existing authority; it does not show RL approved that meaning. “No property-specific owner decision shown in this packet; route a change to RL, don't ignore the adopted rule” is observable. Historical non-consultation/delegation scope is not proved by the amendment. The research note, Case F step 2, labels scope an interpretation. §7 requires stronger **and** adopted without owner; rubric:25 uses **and/or**. There is no typed adoption-authority/decision/owner-of-semantics field: `outcome_owner` is who effects cleanup, **not** who can change the receipt. M2/M3 can catch added evidence, not independently return the ownership finding. Split E4 into a common strengthening item and an authority/routing item (not comparable without adding fields); never award full credit for both based on one generic disposition. |
| **E5/E6** | F-RO:3,9,32 lists Host-positive removal separately and excludes Engine auto-remove under the **amended** rule. It cites specific parent spans; what is lacking is support *there* for unconditional Host attribution—not lack of all specific citations. F-DEC:3,5 adopts a hold and calls H1–H5 “unchanged”; the attribution part, not non-target safety or independent observation, is the addition. Score the scope/part, not merely the words “auto-remove” or “unchanged.” `asserted_as`/statuses allow a rough approximation, not an explicit realization-conditional binding rule. |
| **E7; X5** | B-REV:83–85,133–143 is an admission-lifetime/design decision, not a killer-must-prove-kill contract. B-1539 (`session-excerpt.json`, `source_line:4584`) says “until it verifies that the covered execution has stopped”; the system must verify, **who** and independence are open. Do not read `independent_termination_verification` back into T1. B-DECREC:14, item 2, now names the designated termination actor and independent observer; :21 still asks where the implementation can establish this. T2 negative is right **on item 2**, not “any flag” on the entire decision record (§7:117). |
| **X6–X9** | NC-A:11 expressly owns, deletes and self-reports, with “no independent check … required”: X6 correct. NC-B-contract:11,13 adds the explicit owner requirement **and** independent absence; NC-B-review:7 restates both: X7 correct (the fixture is a short adaptation, not literal 043 plus one sentence). NC-C-parent:7 and disposition:7–15: engine removal and runner issuing stop are choices (X8); independent full-ID absence after exit on the launch engine is preserved as necessary, not redefined as sufficient (X9). A real flag on a different issue is not an X failure. |

These are mostly properties, not demands to reproduce a fix; no Docker proposal should be necessary for credit. But `and/or` in E1/E4/E5/E7 makes partial vs complete hits unclear; no rubric item requires **positive** correct NC-B `supported` or NC-C classification. Empty extraction/“FLAGS: none” can pass all must-nots. Conversely the binary scorer cannot distinguish a legitimate extra finding from a false flag. **Minimum:** freeze scorable spans, atomic outcomes including “missed target / not assessed,” compatible alternatives, and scope of negatives. Require both coverage and correct classification on the NCs. Report target-specific performance separately from a census of off-target flags; count no-source weakenings as failures across all arms. Do not score an inexpressible obligation equally on prose and typed outputs.

### 7. High (P1) — Literal leaks and non-blind scoring can select the result

`PROTOCOL.md:142–152`; `leakage_terms.txt:1–14`; `build_tasks.py:154–168`; `prompts/scorer.md:1–18`. The lexical check establishes only absence from rendered arm texts and presence *somewhere* in the withheld corpus. “obligation provenance” is an ordinary combination of M1's own vocabulary; “Case F” is inferable from F-043; substring “change row” is not a fingerprint. Honest outputs can be selectively voided, especially longer provenance reviews. §8 lists `C0` but the frozen file does not (appropriately: case-insensitive substring `c0` occurs inside task hashes). A paraphrase of privileged information can pass all nine tests. Neither a hit nor its absence proves isolation.

A fresh Opus scorer is useful for first-pass property coding, not an independent reference judgment. Same model/likely shared interpretive and style preferences across all arms, prose vs recognizable JSON, case-known rubric and visible input IDs make differential credit plausible (not demonstrated). It gets output and rubric, **not** the source against which to classify arbitrary false flags/weakening. One vote cannot “disagree” with itself. Its JSON has no per-flag correctness inventory, no item for invented fallback values and no clear evidence convention for *absent* claims. §7 promises all three measurements.

**Minimum:** use high-specificity sentinels as alerts; retain and log every attempt, adjudicate suspected exposure against recorded inputs/config **before unblinding scores**, with a predeclared void/replacement rule; don't reroll plausible language. Scope scan to actual arm/fallback model text, not scorer rubric, IDs or harness wrappers. Freeze a source-anchored scoring sheet (raw/pre/post typed evidence), validate cited output quotes, and require separate scoring of field inventions, coverage and false flags. One fresh scorer per output plus independent blinded human checks of a preselected cross-method sample, all uncertain/invalid items and consequential mismatches is feasible without changing the arm model. Define disagreement, retained original vote, adjudication authority and handling of unresolved votes. Mask labels/order where possible; do not call this model-blind validation.

### 8. High (P1) — Agent files request isolation; they do not establish it

`agents/{p7-arm,p7-scorer}.md:1–10`; `PROTOCOL.md:136,145`; locally installed `pi-subagents` **0.89.0**, `docs/agents.md:19–34,401–417,479–488,504–508,514–532,554–559,563–586,620–626,670`. Empty `tools:` really is an empty declared allowlist, unlike omitted `tools`; replace + no project context/skills is appropriate for a **fresh** child. No `memory` field means no opt-in role memory in these definitions; if enabled it can inject 200 lines even into a read-only agent. User installation is a legitimate location; declining project trust disables project discovery, but default `both` lets a trusted project's same-name agent win. A fork retains the parent's prompt/history. Absent `extensions:` allows ambient background extensions; required extensions, refinements, explicit skills and launch overrides are separate from the base file.

There is also a default supervisor bridge (`docs/configuration.md:628–636`): the installed `src/intercom/intercom-bridge.js:166–192` preserves an explicitly empty tools array at this rewrite but **appends instructions** when active, after the definition digest. Runtime-injected tools/settings are not captured by hashing the Markdown. The provider's `pi-claude-subscription/README.md:41–48,65–78` describes its preamble and Pi prompt/tool transport. No evidence here that any specific answer was injected; “nothing else is accessible” is **unestablished**.

**Minimum:** freeze the launch contract for arm, fallback and scorer: explicit `fresh`, user scope, provider/model/effort, provider-compatible extension policy, bridge off, no memory/refinement/fork/continuation, skill injection or project instructions. Check **effective** model-visible tool list, sanitized system/task digests and fresh session identity from outside the model on the actual route; refuse unexpected additions. A self-report of “no tools” from the first arm, or a digest of the installed file alone, is not a capability receipt. Don't run any of these checks live until authorized.

### 9. High (P1) — There is no specified end-to-end execution or analysis contract

`PROTOCOL.md:125–167`; `runs/plan.json:39–702`; `build_tasks.py:140–151,171–196`; `comparator.py:205–231`; pilot §§9.1,12. A human could conduct calls; `runs/plan.json` is **not a runner**. Nobody is assigned to collect full outputs, render bounded fallbacks (including no-citation/parent cases), invoke the comparator, project the correct item subset to the scorer, validate replies, or link adjudications to immutable attempts. The A3 tasks are 90,593 / 94,644 bytes and ask for *every* requirement and exact quotes. Incomplete output, omitted documents, truncation, provider errors, malformed JSON and invalid spans are different outcomes. Discarding format failures or silently asking M2 for repairs advantages one treatment. Adjudication after seeing the scores could change denominators. A provider substitution is rightly forbidden, but lifecycle retries on the **same** model are possible (`pi-subagents/docs/models.md:18`).

**Minimum:** a small offline driver **or a deterministic operator runbook**, exercised on canned valid/invalid outputs, with an assigned executor and independent scorer/adjudicator. Bind full input/output digests, run/attempt/parent IDs, source version, model/effort and observed tools; record ordinary and fallback comparisons, decisions/abstentions, error stage, truncated/invalid/void runs and why. Count tokens (including cache policy), calls and latency by extraction/fallback/scoring/smoke/retry; mark unavailable values unknown. Use access-controlled evidence, final texts or stable pointers, not private reasoning logs (pilot:385–401). Freeze an order or interleaving, completeness/length checks, JSON failure policy (e.g. first-pass failure, no discretionary repair), and how far a stop invalidates prior rows, partial-result reporting and re-freeze after a defect. Keep stop-on-provider/isolation/harness failure, no fallback provider and no quota cap. State the descriptive estimand: per-item hits out of eligible attempted/valid runs, invalids, individual false flags, paired M3 transitions and extra calls; define “more consistently” and “adds coverage without inventing” before seeing outputs. Do not pool four F boundaries as four independent incidents.

### 10. Medium (P2) — Claims need one more boundary

`PROTOCOL.md:18–22,169–183`; `../README.md:3–5,40–50,64–70`. §10 correctly disclaims general rates, production and cross-model generality; the README correctly calls B prevention, not another amplified contract. Add case-informed vocabulary, labeled synthetic/easy controls (until fixed), retrospective look-ahead, bundled prompt effects, and same-model judge. Even perfect 3/3 cells show repeated detection **on these bytes under these prompts**, not that the checker would have intervened in time, the typing is authoritative, B is held out, fallback improved over its own baseline without a paired comparison, or any engineering time/incident would have been saved. A null result may be format/coverage failure, not absence of ambiguity. Sixty-six runs are not 66 independent cases. README:50's “No session record before …” should be limited to the reviewed/captured evidence unless a full-session audit is referenced; :3's “captured evidence only” no longer describes the containing directory. Keep separate freeze and operator execution approval (§9 currently includes go-ahead among *freeze* prerequisites).

### 11. Medium (P2) — `--check` does not check the shipped build against its pins

`PROTOCOL.md:26,40,169–175`; `build_tasks.py:42–50,87–111,171–196`; `runs/plan.json:3–38`. With `--check` the code builds in memory and checks fixed historical file hashes, run count and leak terms. It **never reads** `tasks/*.md` or the existing `runs/plan.json`; it computes but does not compare the fixture digests (`expected=None`), templates or comparator against the stored pins. For B-1539 it compares one copied **digest string** and uses `entry["text"]`, not a recomputation/authentication of the raw line. Rendered source bodies also use `rstrip()`: bytes of captured files vs bytes sent to the model are different claims. I verified the current B excerpt itself hashes to its Stage 0 manifest (`70a224bf…`), and the projected text to `22bb19f2…` as in the plan. I found **no current mismatch**. Future edits/deletions of shipped outputs could still give a green check.

**Minimum:** in check mode compare the expected render, every on-disk task, exact run rows and frozen asset digests (including agents, leak list and schema/rubric) without writing. Verify the Stage 0 excerpt hash from its manifest, unique entry coordinate, and projected text hash; distinguish that from verifying the original raw session. Pin the full freeze by commit and protocol digest. Add a stale-artifact rejection test.

## Freeze decision and verification limits

**Do not reject the question.** Remove answer labels; fix citation/entailment and replacement/unknown handling; align the rubric and comparable endpoints; pair fallback; specify and offline-test isolation, failure, scoring and receipt handling. These can preserve the operator's model, effort, repeats, no-cap decision and 66 planned arm runs. Submit the revised hashes and negative test results **before** any smoke/arm call. **Fixed in this review: none.**

I did not launch a model, agent, runner or network operation, install either agent, regenerate tasks, edit the repository or commit. I did not authenticate the historical owner's decisions, read the original private raw transcripts, verify subscription billing/availability, establish the future runtime's effective isolation, test a model's JSON/length behavior or measure scorer reliability. Comparator counterexamples above are branch traces from reading; only the two authorized P7 checks were executed as programs under test. No experiment outcomes are reported.

## Command receipt

From `/home/bline/code/work-engine`:

```
PYTHONDONTWRITEBYTECODE=1 python3 -I -B experiments/perspective-intent-research/p7-run1/build_tasks.py --check
# exit 0; 66 runs / 15 unique tasks / 9 leakage terms
PYTHONDONTWRITEBYTECODE=1 python3 -I -B -m unittest discover -s experiments/perspective-intent-research/p7-run1 -p 'test_*.py'
# exit 0; 23 tests, OK
```

`-B` is intentional in addition to the requested environment setting: `-I` ignores `PYTHON*` environment settings. SHA comparisons of the task and asset rows all returned `OK`; four independent `cmp` checks returned 0. Other shell calls were local inspection. Directory probes initially found the designated report parent absent; an oversized documentation listing was truncated by the tool, and relevant sections were read by line; `.pi` and one conventional reader path were absent. Neither absence is evidence of runtime isolation. The explicit context check used an existing digest-verified reader; no listener was started. The final command created only the necessary report directories and this report, appended the command-only receipt below, and checked the result.

<details><summary>Inspection commands (grouped as actually submitted; status is the shell tool result for the batch)</summary>

**returned**

```sh
pwd; git rev-parse HEAD; git branch --show-current; sha256sum experiments/perspective-intent-research/p7-run1/PROTOCOL.md; git diff --stat HEAD -- experiments/perspective-intent-research; ls -ld /home/bline/.pi/agent/sessions/--home-bline-code-work-engine--/subagent-artifacts/outputs/d8af0eb0-8a22-4423-bf8f-ba20b238046a/reviews/p7-run1-protocol-astra-20261124; ls -R experiments/perspective-intent-research
```

**returned**

```sh
cat docs/contracts/{core,instruction-design,evidence-and-claims,context-and-review}.md
```

**returned**

```sh
nl -ba experiments/perspective-intent-research/p7-run1/PROTOCOL.md; cat experiments/perspective-intent-research/README.md
```

**returned**

```sh
cd experiments/perspective-intent-research/p7-run1 && cat -n build_tasks.py && cat -n test_comparator.py
```

**returned**

```sh
cd experiments/perspective-intent-research/p7-run1; nl -ba comparator.py
```

**returned**

```sh
cd experiments/perspective-intent-research/p7-run1; for f in prompts/* agents/* schema/* fixtures/* leakage_terms.txt; do echo "=== $f"; nl -ba "$f"; done
```

**returned**

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -I -B experiments/perspective-intent-research/p7-run1/build_tasks.py --check; PYTHONDONTWRITEBYTECODE=1 python3 -I -B -m unittest discover -s experiments/perspective-intent-research/p7-run1 -p 'test_*.py'
```

**nonzero**

```sh
ls -la ~/.pi/agent/npm/node_modules/pi-subagents/docs; nl -ba ~/.pi/agent/npm/node_modules/pi-subagents/docs/agents.md; namei -l /home/bline/.pi/agent/sessions/--home-bline-code-work-engine--/subagent-artifacts/outputs/d8af0eb0-8a22-4423-bf8f-ba20b238046a/reviews/p7-run1-protocol-astra-20261124/review.md
```

**returned**

```sh
rg -n 'P7|9\.1|Stage 1c|^## 12|Case [BF]' design/ideas/pending/pilots/perspective-intent-pilot.md; rg -n 'N2' reviews/obligation-provenance-plan-opus-20261123/review-r2.md; rg -n 'Experiment arms|pre-registered' docs/model-selection.md; rg -n 'Case [FB]' design/ideas/pending/perspective-intent-research-note.md; cd experiments/perspective-intent-research/stage0; wc -l case-f/packet/{*.md,*/*.md} case-b/packet/*; ls -l MANIFEST*; head -n 40 MANIFEST-case-b.json
```

**returned**

```sh
cd experiments/perspective-intent-research/stage0/case-f/packet; for f in review-r1/*.md PLAN.md; do echo "=== $f"; nl -ba "$f"; done
```

**returned**

```sh
cd experiments/perspective-intent-research/stage0/case-f/packet; for f in ARCHITECTURE-REESCALATION.md architecture-reescalation-review/ASTRA-ARCHITECTURE.md QUALIFICATION-AMENDMENT.md; do echo "=== $f"; nl -ba "$f"; done
```

**returned**

```sh
cd experiments/perspective-intent-research/stage0/case-f/packet; for f in RESOLVED-OBJECT-ESCALATION.md resolved-object-review/*.md; do echo "=== $f"; nl -ba "$f"; done
```

**returned**

```sh
cd experiments/perspective-intent-research/stage0/case-b; for f in packet/ORG-ASTRA-DESIGN-REVIEW-RESULT.md packet/CONTROLLED-BUILDER-ADMISSION-REVISION.md; do echo "=== $f"; nl -ba "$f"; done; jq '.entries[]|select(.source_line==4584)|{text,source_line,raw_line_sha256}' session-excerpt.json
```

**returned**

```sh
p=design/ideas/pending/pilots/perspective-intent-pilot.md; nl -ba "$p" | sed -n '385,403p;460,484p;611,640p;682,718p'; nl -ba reviews/obligation-provenance-plan-opus-20261123/review-r2.md | sed -n '75,98p'; rg -n 'scope|project trust|shadow' ~/.pi/agent/npm/node_modules/pi-subagents/docs/agents.md | head -28; rg -n 'contact_supervisor|supervisor channel' ~/.pi/agent/npm/node_modules/pi-subagents/docs/{agents,tool-reference,configuration}.md | cut -c1-400 | head -32; sed -n '122,149p' design/ideas/pending/perspective-intent-research-note.md
```

**returned**

```sh
p=$HOME/.pi/agent/npm/node_modules/pi-subagents/docs; nl -ba "$p/configuration.md" | sed -n '627,639p'; nl -ba "$p/agents.md" | sed -n '18,39p'; rg -ni 'task.{0,44}(max|limit|bytes|chars|truncat)|(?:max|limit|bytes|chars|truncat).{0,35}task|supervisorBridge' "$p/tool-reference.md" "$p/configuration.md" | cut -c1-450 | head -50; ls -d ~/.pi/agent/npm/node_modules/*claude*; rg -n 'claude-sdk|Experiment arms' docs/model-selection.md | head -20 | cut -c1-550; ls -d ~/.pi/agent/npm/node_modules/pi-subagents/* | head -30
```

**returned**

```sh
cd experiments/perspective-intent-research; head -n 65 stage0/MANIFEST.json; tail -10 stage0/MANIFEST-case-b.json; nl -ba README.md | sed -n '1,12p;40,71p'; cd p7-run1; rg -n '^=== (PARENT|DERIVED|SOURCE) DOCUMENT|positive.*normal-removal witness|The H1–H5 requirements are unchanged|The system should not release the execution lock' tasks/A3-F.M2.md tasks/T1.M1.md | cut -c1-240; rg -n 'negative control|affirmative, attributable|automatic-removal|Unchanged from the cleanup requirement' tasks/NC-?.M?.md | cut -c1-180 | head -36; jq -r '.runs | group_by(.input_set,.method)[] | [.[0].input_set,.[0].method,length] | @tsv' runs/plan.json; jq -r '.runs[]|[.task_sha256,("tasks/"+.task_id+".md")]|join("  ")' runs/plan.json | sort -u | sha256sum -c -; wc -c tasks/*.md; jq -r '.case_b_session_excerpt.sha256' ../stage0/MANIFEST-case-b.json; sha256sum ../stage0/case-b/session-excerpt.json; jq -j '.entries[]|select(.source_line==4584)|.text' ../stage0/case-b/session-excerpt.json | sha256sum; jq -r '.input_digests["B-1539"]' runs/plan.json
```

**returned**

```sh
rg -n 'def .*read|sqlite3|open\(|init_db|ensure_schema|connect|mkdir|socket|urlopen' skills/durable-state/scripts/codex_chatboard.py | head -55; sed -n '1,115p' skills/manual-orchestration/references/context-control.md; rg -n 'reviewer' skills/manual-orchestration/SKILL.md | head -10; rg -n '^## ' docs/contracts/system-design.md; rg -n 'memory|inline|clarify|question' ~/.pi/agent/npm/node_modules/pi-subagents/docs/tool-reference.md | cut -c1-250 | head -25; git log -1 --oneline --stat | head -55
```

**returned**

```sh
s=$HOME/.pi/agent/npm/node_modules/pi-claude-subscription; rg -n -i 'tools: \[\]|disable.*tools|allowedtools|settingSources|system prompt|claude-sdk' "$s/README.md" | cut -c1-350 | head -32; nl -ba ~/.pi/agent/npm/node_modules/pi-subagents/docs/configuration.md | sed -n '623,633p'; rg -n 'contact_supervisor' ~/.pi/agent/npm/node_modules/pi-subagents/src | cut -c1-260 | head -35; rg -ni 'maxTask(Length|Bytes)|MAX_TASK_(LENGTH|BYTES|CHARS)|task.{0,12}too long' ~/.pi/agent/npm/node_modules/pi-subagents/src | cut -c1-230 | head -20; sed -n '86,143p' skills/durable-state/scripts/codex_chatboard.py; sed -n '1,28p' skills/manual-orchestration/references/worker.md
```

**returned**

```sh
cd experiments/perspective-intent-research/p7-run1; jq -r '((.template_digests|to_entries[]|[.value,("prompts/"+.key+".md")]),([.schema_digest,"schema/p7-obligation-extract-v1.json"]),([.rubric_digest,"prompts/rubric.json"]),([.comparator_digest,"comparator.py"])) |join("  ")' runs/plan.json | sha256sum -c -; sha256sum fixtures/*.md agents/*.md; jq '.fixture_digests' runs/plan.json; sed -n '1,64p' runs/plan.json; rg -in 'C0|obligation provenance' tasks/A3-F.M2.md | head -2 | cut -c1-160; rg -n 'no .*fall.?back|fall.?back' ~/.pi/agent/npm/node_modules/pi-subagents/docs/models.md | cut -c1-300 | head -10; rg -n 'source_line.*4584' ../stage0/case-b/session-excerpt.json
```

**nonzero**

```sh
nl -ba ~/.pi/agent/npm/node_modules/pi-claude-subscription/README.md | sed -n '3,8p;35,112p'; p=$HOME/.pi/agent/npm/node_modules/pi-subagents; nl -ba "$p/docs/configuration.md" | sed -n '614,627p'; nl -ba "$p/src/intercom/intercom-bridge.js" | sed -n '127,196p'; cat "$p/package.json" | jq -r '.version'; nl -ba "$p/docs/models.md" | sed -n '10,25p'; jq -r '.subagents|{intercom,intercomBridge,forceTopLevelAsync,asyncByDefault,defaultExtensions,defaultInheritGlobalContext}' ~/.pi/agent/settings.json; ls -A .pi 2>/dev/null
```

**returned**

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -B skills/durable-state/scripts/codex_chatboard.py --repository . read --limit 20 --active-claims | jq -c '{ok:(has("error")|not),keys:keys,claims:(.claims//.active_claims//[]|length)}'; ls -l ~/.local/bin/work-engine-context-status; printf 'Binding supplied? session:%s file:%s\n' "${PI_SESSION_ID:+yes}" "${PI_SESSION_FILE:+yes}"; nl -ba docs/contracts/system-design.md | sed -n '1,24p;159,208p'
```

**returned**

```sh
psrc=$HOME/.pi/agent/npm/node_modules/pi-subagents/src; rg -n 'bridge\.active|intercomBridge\.active' "$psrc/runs/shared/child-tool-plan.js" "$psrc/runs" | cut -c1-260 | head -40; rg -n 'intercomBridge' ~/.pi/agent/npm/node_modules/pi-subagents/docs/tool-reference.md | cut -c1-530; rg -n 'status.*pi|binary|work-engine-context-status' skills/manual-orchestration/scripts/provision_context_status.sh | head -28; printf 'transcript binding exists? '; test -r "$PI_SESSION_FILE" && printf 'readable\n' || printf 'unavailable\n'; head -1 "$PI_SESSION_FILE" | jq -c '{type,id,cwd}'; rg -n 'input_tokens|PINNED|context-status' skills/manual-orchestration/references/context-measurements.md | head -12 | cut -c1-190
```

**returned**

```sh
p=experiments/perspective-intent-research; cmp -s <(sed -n '418p' "$p/p7-run1/tasks/A3-F.M2.md") <(sed -n '5p' "$p/stage0/case-f/packet/resolved-object-review/OPERATOR-DECISION.md"); echo "F-DEC:5 cmp=$?"; cmp -s <(sed -n '313p' "$p/p7-run1/tasks/A3-F.M2.md") <(sed -n '7p' "$p/stage0/case-f/packet/QUALIFICATION-AMENDMENT.md"); echo "F-AMEND:7 cmp=$?"; cmp -s <(sed -n '38p' "$p/p7-run1/tasks/NC-C.M1.md") <(sed -n '15p' "$p/p7-run1/fixtures/NC-C-disposition.md"); echo "NC-C:15 cmp=$?"; cmp -s <(sed -n '187p' "$p/p7-run1/tasks/T1.M1.md") <(jq -r '.entries[]|select(.source_line==4584)|.text' "$p/stage0/case-b/session-excerpt.json" | sed -n '7p'); echo "B-1539 para cmp=$?"; rg -n 'from subagent|required tool|preflight' ~/.pi/agent/npm/node_modules/pi-subagents/docs/extension-api.md | rg -i 'digest|tools|launch contract' | head -10 | cut -c1-280; git status --porcelain=v1 --untracked-files=normal -- experiments/perspective-intent-research
```

**returned**

```sh
sed -n '1,42p' skills/manual-orchestration/scripts/provision_context_status.sh; printf 'header matches env? '; head -1 "$PI_SESSION_FILE" | jq -r --arg id "$PI_SESSION_ID" '.id==$id'; jq -c 'select(.type=="model_change" or .type=="thinking_level_change") | {type,provider,modelId,thinkingLevel}' "$PI_SESSION_FILE" | tail -4; jq -c 'select(.type=="message" and .message.role=="assistant")|.message|{model,provider,time:.timestamp,usage:(.usage|{input,cacheRead,cacheWrite,output})}' "$PI_SESSION_FILE" | tail -1; jq -r 'select(.type=="message" and .message.role=="assistant")|.message.content[]?|select(.type=="toolCall")|.name' "$PI_SESSION_FILE" | sort | uniq -c | head -12; pwd; ls -ld /home/bline/.pi/agent/sessions/--home-bline-code-work-engine--/subagent-artifacts/outputs
```

**returned**

```sh
p=$HOME/.pi/agent/npm/node_modules/pi-subagents; rg -n 'contactSupervisor|CONTACT_SUPERVISOR_TOOL_NAME|supervisorTool' "$p/src/runs/shared/child-tool-plan.js" "$p/src/runs/foreground/" | cut -c1-300 | head -34; nl -ba "$p/docs/extension-api.md" | sed -n '303,316p'; nl -ba "$p/docs/tool-reference.md" | sed -n '109,123p'; rg -n 'bridge.*(supervisor|tool)|contact_supervisor' "$p/src/runs/shared/child-tool-plan.js" | cut -c1-240
```

**returned**

```sh
s=$HOME/.local/state/work-engine/tools/context-status; ls -d "$s"/*/work-engine-context-status | head -8; jq -r 'select(.type=="message" and .message.role=="assistant") | .message.content[]?|select(.type=="toolCall" and .name=="bash")|.arguments.command[0:120]' "$PI_SESSION_FILE" | head -5; nl -ba ~/.pi/agent/npm/node_modules/pi-subagents/docs/tool-reference.md | rg -i 'intercomBridge|inline' | head -8 | cut -c1-365; nl -ba experiments/perspective-intent-research/stage0/README.md 2>/dev/null | head -2; git rev-parse --short HEAD; sha256sum -b experiments/perspective-intent-research/p7-run1/PROTOCOL.md
```

**returned**

```sh
d=d5209c61f8d7f8bc185531f316876ce6f5630fdecdef0cdc46e992d0acdeff2a; b=$HOME/.local/state/work-engine/tools/context-status/$d/work-engine-context-status; if [ "$(sha256sum "$b" | cut -d' ' -f1)" = "$d" ]; then "$b" pi --session "$PI_SESSION_ID" --session-file "$PI_SESSION_FILE" | head -c 2000; fi; jq -c 'select(.type=="message" and .message.role=="toolResult")|{toolName:.message.toolName,toolCallId:.message.toolCallId,isError:.message.isError}' "$PI_SESSION_FILE" | tail -2; nl -ba experiments/perspective-intent-research/p7-run1/runs/plan.json | tail -14
```


</details>
