# P7 run 1 — re-review of revision 2

**Verdict: `revise_and_resubmit`. Freeze: BLOCK.** The controls no longer give away their designation, and the question is now a defensible descriptive comparison of two case-informed packages. Keep those changes and the 66-run allocation. Two defects alone preclude freezing: the instrument still returns **`supported` for an obligation it did not assess**, and a **passing receipt does not establish the tool grant or even bind the scored output to the task that was launched**. Neither can be fixed by adding repeats or asking the operator to interpret the resulting scores more carefully.

**Verified subject:** `/home/bline/code/work-engine`, `rescue-detached-20260911`, HEAD `233821e11db00ad7db1510506b6fda006b6fa11f`; `PROTOCOL.md` SHA-256 `9ee81b51cfdad527e10dd5192d345c065dea31a5c26d48bb427929f210b35031`, `RUNBOOK.md` `58d028168f4e31b867da0f4f94fcfa792d5b478a768b6e567c03a3afbcad6ded`. Range reviewed against `29d84386`. Unless qualified, paths below are in `experiments/perspective-intent-research/p7-run1/`. Findings about branch behavior are from reading the pinned source; historical Pi files were inspected, not replayed.

**Check results:** `build_tasks.py --check` passed: 66 runs, 15 tasks, 8 leakage terms. Independent raw SHA-256 checks passed for all 15 shipped tasks; 22 cells × 3 repeats. **48/48 test bodies passed**, with an important qualification: 39 comparator tests and two build-check tests ran normally. The seven pipeline tests actually create and delete files (`test_pipeline.py:22–46,55–68`); `-B` only prevents bytecode writes. To respect “nothing written,” I also ran full unittest discovery with those temporary directories/files backed by *Python process memory*, leaving test bodies, production logic and repo reads unchanged. This tests the assertions, **not** real-disk persistence or native subprocess behavior. I did not run the unmodified filesystem-writing discovery command. No smoke test, agent install, child launch, model or network call by the reviewer, repository edit or commit.

## Closure of the eleven original findings

| # | Status | Evidence and residual |
|---|---|---|
| 1. Citation vs entailment vs authority | **Partially closed** | `M2-extract.md` in `prompts/`, lines 9–14, no longer gates candidate linkage on sufficient support. `test_comparator.py:163–184` separates specific-but-insufficient and general-but-comparable. But any specific reference clears citation deficiency for the entire entry, even one unrelated to the property; reference targets and quote text are not verified. All relevant candidates are treated as jointly binding constraints without a common subject/condition check. Authority is still decided entry-wide (B, C below). |
| 2. Comparisons and timing | **Closed (design)** | `PROTOCOL.md:19–36,69–88` makes A1 planner derivation, A2 local wording given the wider packet, A3 explicitly a retrospective audit, Case B design-informing, M1/M2 packages, and M3 a within-extraction pre/post comparison. `pipeline.py:278–299` really computes that pair. Shared M2 task text no longer substitutes for paired data. The six derived files are visible together (`tasks/A3-F.M2.md:193–396` delimiters); no earlier-firing claim. Scoring/collection defects remain under 5, 7 and 9. |
| 3. Controls reveal answers | **Closed for the stated package comparison** | Rendered NC tasks have `DOC-*` wrappers and no NC/“constructed negative control” labels; the NC-C M1 task still states automatic removal and the unchanged check (e.g. line 64), correctly, as contract evidence. `prompts/concepts.md:5–21` explicitly cues an engine, independent observation, actor attribution, exclusive causality and distinct failure; the same text occurs in M1 and M2 tasks including T1. The schema repeats it in token form. Not an unseen vocabulary or spontaneous transfer experiment, and §4 now says so. Ensure executor wrappers/filenames don't reintroduce labels (A). |
| 4. Comparator certifies weakness/non-assessment | **Partially closed; still blocking** | Complete actor widening, mixed item add/drop, proposals and some conforming/nonconforming choices now work (`test_comparator.py:86–142,195–235`). But omitted candidate keys, partial ambiguity and changing a field's *binding status* can still turn a loss or unknown into `supported` (`comparator.py:269–322`). See B. |
| 5. Validation and bounded fallback | **Partially closed** | Primary document role, entry quote and many status/enum checks now exist; successful fallback is restricted to a listed reading and an abstention preserves original readings (`comparator.py:93–182,375–429`). Other claimed spans/types are not validated; malformed shapes can crash downstream. An indexed fallback call can answer for a different target, and missing or alerting calls do not block comparison. See D, F. |
| 6. Rubric validity and parity | **Partially closed** | `prompts/rubric.json:8–11,27–39,45–87` splits baseline practice from obligation and E4a from E4b, protects the supported parts, scopes X5, and demands positive evidence for the controls. Those are improvements. But correct unspecified typing cannot fire some of the matching plan flags, and E4b still confuses missing proof with historical nonapproval. Typed positive credit vs negative/flag credit needs one explicit rule (C). |
| 7. Leakage, scoring and independent checks | **Partially closed** | `leakage_terms.txt:1–14` retires “Case F” and “obligation provenance”; alerts + pre-score adjudication, source-aware scoring, seeded human checking and explicit same-model bias are appropriate. But no effective leakage adjudication transition, scorer reply validation, or phase-local quote validation exists (E, F). A misspelled word or case-informed phrase is still an alert, not proof of exposure. |
| 8. Isolation | **Partially closed; still blocking** | Frontmatter `agents/{p7-arm,p7-scorer}.md:4–10` is valid for installed pi-subagents 0.89.0. Empty tools, extensions allowlist, no inherited context/skills, fresh context, user scope and empty scratch cwd are reasonable controls. The new observer checks use rather than grant and compares neither task nor configuration to an expected launch digest. The shared bridge change is unnecessary. See A. |
| 9. Execution and stopping | **Partially closed** | There is now a runbook, role assignment, seeded schedule, human sample, recorded raw arm output and immutable individual attempt JSON. But `final_attempt` does not implement the declared retry/finality rules, scorer material can be overwritten, and `report` doesn't consume a single scored/adjudicated item. Runbook steps relying on these states are not deterministic as written (E–G). |
| 10. Claims and limits | **Closed for external claims** | `PROTOCOL.md:27–36,219–239,253–263`; `../README.md:3–5,50`. Descriptive repeatability only; no blind schema, causal timing, held-out validation, general rates, savings or production authority. Freeze and execution approval are distinct. The specific *instrument* claims about isolation, invention and valid-run comparison still need correction. |
| 11. Build verification | **Partially closed (minor residual)** | `build_tasks.py:94–112,115–124,189–242` now checks the excerpt manifest and unique projected coordinate, preserves original source body text, and compares the task set and plan incl. asset digests. Stale-artifact tests `test_pipeline.py:159–176` pass. Not literally the byte-for-byte check claimed at `build_tasks.py:11–14`: `.md` read in universal-newline text mode accepts LF→CRLF, and parsed JSON equality permits differently serialized plan bytes. No mismatch in **this** checkout. Compare task bytes to the bytes hashed for dispatch; for the plan either require canonical bytes or explicitly promise structural equivalence plus a separately recorded file hash. |

## Findings on the revised instruments

### A. Blocker (P0) — Wrong observation for a no-tools/no-extra-input claim

`pipeline.py:85–137,171–188,425–457`; `RUNBOOK.md:19–61`. On the actual retained run `d8af0eb0-8a22-4423-bf8f-ba20b238046a`, `status.json.steps[0]` has `context:"fresh"`, `toolCount:28`; `events.jsonl` has **28 tool execution starts but 22 tool definitions** in the initial system message's `toolsAdded`. On real same-provider run `95b437ab-8be5-4b55-b9cc-814aff7fe62b` these numbers are **16 starts, 6 declared**. Installed `src/runs/background/subagent-runner.js:2821–2840` increments `toolCount` on *execution start*; `async-status.js:250` omits it when undefined. These are two observations of **use**, not independent evidence of **availability**. A supplied-but-unused read tool need never increment this counter. `extract_receipt` ignores `toolsAdded`/`toolsRemoved` altogether. Conversely a missing count is not necessarily a nonzero count. A retained failed Opus launch `9b42d0e3-8d69-4211-a607-3366fb5d9659` has neither count; the code prioritizes `invalid_receipt` over provider failure. Don't fix this by equating missing with zero.

- **Real schema, not assumed incompatibility.** In `95b437ab…`, `steps[0].model` is `claude-sdk/claude-opus-5-5`, thinking `high`, `context:"fresh"`, skills `[]`; the assistant event is `provider:"claude-sdk", model:"claude-opus-5-5", thinkingLevel:"high"`. Those expected strings are plausible. The code compares only the **provider** in assistant events, passes an empty model set, accepts absent context and skills, accepts an empty/missing `sections` object in a system message, ignores later system messages and malformed JSONL lines. It does not check terminal state, exact cwd, extensions or their expected digest, effective definition or number of steps/user turns. Arbitrary extra content can be in the allowed `preamble` or `cwd` values. The three marker words are neither necessary nor sufficient for a bridge; benign provider prose isn't evidence of a bridge. Pi-Claude `src/prompt.ts:45–56` has a conditional custom-prompt path; don't prescribe or reject the provider's text by guesswork. Historical `9805c42a…` has assistant events but no system `message_end` in the retained stream; that is **unestablished** for this check, not proof of contamination. No actual P7 grant was observed.
- **Custody hole independent of tools.** The CLI accepts any `OUTPUT_FILE` and any `RUN_DIR`. It hashes the supplied bytes and copies `task_sha256` **from the planned row**; it doesn't check the observed user task or final assistant text against either, or record/check a unique native run/session ID. The same honest receipt could be reused for several repeats or paired with another row's answer. A scorer/fallback response has the same problem. Reading a retention-managed directory “immediately” (§3) does not establish a join or availability; the path/uid is hard-coded and may differ across runners. Save a bounded access-controlled receipt from the provided native paths before reclamation; do not archive private reasoning.
- **Bridge and keys.** Installed `docs/agents.md:19–34,479–488,504–508,514–531,563–580` documents these keys; the declared provider extension file exists. The absence of `memory` means no opt-in role memory in the base file. Required extensions/runtime overlays, override settings and additional prompt messages remain distinct. `options.agentScope:"user"` is documented in `docs/tool-reference.md:97`. `RUNBOOK.md:26–31,163` prefers editing a shared config, consulting a board snapshot and later restoring a backup. That can disable coordination in another session, overwrite an intervening legitimate change or leave the bridge off after a stop. Already-loaded extensions need not see the edit. The alternative (“proceed if no bridge markers”) is not a positive guard. Installed `docs/extension-api.md:295–315,419` explicitly supports a **per-launch** `intercomBridge:{mode:"off"}` on structured delegation and matching preflight. Use the supported surface on every stage; if unavailable on the selected launch interface, use an isolated configuration or stop. No global edit needed.

**Minimum:** freeze the expected launch contract; compare the native `launchContractDigest` to an **independently derived expected** digest; verify the realized tool declarations/deltas and system content at each request, not just keys/first preamble. Bind one fresh native run/step/session, raw task (including allowable wrappers), actual assistant provider/model/effort, terminal result/output and attempt id. Require the evidence to be present; separate unavailable telemetry/provider errors from a proven grant violation. Preflight alone omits some runtime annotations (`extension-api.md:309–315`); the actual receipt still matters. Offline tests should include unused tools, extra/later sections, missing events, wrong task/output and wrong actual model. Smoke must check a **successful** result under that contract, not merely the absence of entries in `problems`.

### B. Blocker (P0) — Supported is still reachable with unknown or lost binding requirements

`comparator.py:185–246,264–354`; `prompts/M2-extract.md:6–15`. Branch traces on v2-valid inputs:

| Input | Current path and consequence |
|---|---|
| Partial inherited entry, `outcome_owner: ambiguous` with two readings and a valid parent candidate; no binding fields | `props` is empty at 274; neither `not_assessed` nor a provenance flag; 322 emits **`supported`**, even without refs. Same for `fields:{}`. The test at `test_comparator.py:143–149` is **complete** scope only. A fallback that sets the field binding can now appear to “add coverage” over a falsely supported pre-state. |
| Complete/unchanged parent requirement has `verifier=independent_observer` and `evidence=independent_check`; derived retains evidence only | If the extractor supplies a `verifier` candidate the drop is caught. Omit that candidate key but still link the very same parent via evidence: 275–278 never visits `verifier`. With a specific ref the entry is supported. Test lines 98–104 supplies the missing key for the comparator. Worse: retain the key and value but mark the derived verifier merely `selected_realization`; 287–295 compares its **value** as if still binding and can report equal for removal of its binding standing. |
| Choice vs parent | A narrower role selection inside a permitted set, extra items, complete item loss and weakened independence are handled sensibly in `_realization_conflict`. But a choice with no comparable properties/candidates becomes `realization`; `parent_unspecified` is indistinguishable from no lookup. Ambiguous parent, conflicting parents or `other` are all emitted as **`realization_conflicts`**, even when incompatibility was never established. A partly binding demand inside a choice can also pass as an innocuous implementation choice. |
| Multiple candidates / partial “changed” | Intersecting permitted role sets (wildcard neutral) and unioning required items is correct for **conjunctive constraints on the same subject, action and condition**; equality/conflict for independence presupposes the same thing. “Every parent entry that bears on that property” is not this predicate. The schema cannot tell complementary requirements from alternatives, a successful witness from a failure path or different concrete actors both typed `acting_component`. E.g. failure-reporting on the selected path and independent absence on the positive path can become one unconditional bundle. A binding candidate suppresses all ambiguous candidates. For overlapping role sets `{acting_component,managing_system}` → `{acting_component,operator}`, `_relation` returns `changed` even in partial scope: a flag is warranted for excluding the managing system, but the weaker component isn't a replacement loss. For partial supersets code calls the result `equal` rather than *entailed*. Neither label identifies the actual delta. |

The separation of citation form and comparison is useful. But `deficient_provenance` remains an entry-wide bit: one specific citation of any document clears it for every field. Conversely the NC-C unchanged acceptance can have equal semantic relations but overall standing `flagged` solely for a general citation; the real-document test expects this (`test_comparator.py:365–376`). A scorer must not turn that into an X9 failure or into evidence of strengthening. A proposal correctly creates no standing flags, but can still have relations in its record.

**Minimum:** distinguish assessment coverage, entailment/change, citation adequacy and standing in the report. Require a designated baseline obligation/set for complete or unchanged comparison; enumerate its binding properties independently of derived keys. Compare **binding status and value**. Empty, ambiguous, mixed or unlinked comparisons must not establish support or conflict. Require candidates to be jointly applicable to the property **under the same condition**, or record unresolved alignment; no general theorem prover needed. Emit added/lost values and the candidate spans; test these cases alongside the already-passing ones.

### C. High (P1) — E4b, E1 and E7 can reward the wrong thing

`comparator.py:83–90,249–261,307–322,357–372`; `PROTOCOL.md:133,141–146,163–183`; `prompts/rubric.json:15–17,25–39,43–59`; `prompts/scorer.md:7`.

- **Authority:** F-041:15 assigns RL receipt semantics, dev-vertical the Host implementation. A change to RL-owned evidence plus an *unchanged* property with dev-vertical as owner pools both owners; adopter `dev-vertical` then intersects the pool and incorrectly clears the change. Conversely any candidate with unknown owner forces the flag even if the relevant owner matches. `RL` and `Research Lifecycle` are different free-text strings here. Worse, comparing to an unspecified/ambiguous parent produces no `changes` member, so an imposed binding rule with no parent determination may skip authority checking altogether. A reviewer *asserting* an inherited requirement is not necessarily an *adopter*. “No decision by RL **shown** in the packet” is not “RL did not approve.” E4b is now representable in principle, not reliably inferred by this bit. Check authority **per changed property**, on an adoption event, with a source-backed owner identity (incl. document ID + span); distinguish matched, mismatched and not established. Require the correct RL property/routing for credit; do not require knowing the historical conversation.
- **Plan flags:** E1 is now specifically the unassigned **remover**, not the verifier. `F-043:19` can honestly yield owner `unspecified` rather than `ambiguous`; then no owner flag. B-1539:4584 requires verifying the execution stopped, with verifier and independence both unstated. With that faithful typing, line 368 emits `verifier_unassigned` (E7a); line 370 **cannot** emit `independence_unstated` (E7b requires verifier *binding* to trigger). No comparable defect with a fully typed NC-A (`not_required`, self-report) or B-DECREC item 2 (`required`, independent observer); those should remain positives, not be split. On multi-obligation packets, flags also depend on row grouping and whether evidence was typed binding. `plan_entries_evaluated` counts extracted rows, not gold opportunities. Specify whether an extracted status *without a matching flag* counts as detection; §7:178 says flags, scorer:7 allows records plus extraction. Give E1 an unresolved-owner observation distinct from an intentionally open choice, and record both verifier and independence on E7 without asserting independence was required. Test both honest typings and positive control evidence, not just “no flags.”

### D. High (P1) — Validation is strict on one quote but permissive on provenance

`comparator.py:79–119,122–182`; `schema/obligation-extract-v2.json:22–41`; `pipeline.py:186–202`. Only `entry.span` is checked as a whitespace-normalized substring. `references_made.text`, `target_document`, owner/adopter spans and `other_text` are not checked against the source. `test_comparator.py:39–51` even defaults to a specific “see parent” citation absent from the synthetic text. Owner names need not be strings; unhashable JSON values in status/token/candidate slots can raise `TypeError`, or a non-string name can raise `AttributeError` later, rather than recording a final invalid extraction. Duplicate identical ambiguous readings pass; mixed `any` plus a specific role can erase the restriction. `entries:[]` is accepted—fine as *zero coverage* only if all applicable items are explicitly counted, not evidence of successful assessment.

Whitespace normalization **does** allow line breaks/spaces inside a contiguous quote. It does **not** allow stripped Markdown delimiters, a changed curly quote, ellipsis or two noncontiguous clauses; e.g. F-AMEND:7 contains literal `**positively attained normal-removal witness**`. Crossing a delimiter without copying it fails. One such error invalidates the *entire* long extraction; M1 merely needs “flags” as a substring. This can measure transcription accuracy rather than obligation detection. The frequency is not established by a hand-typed test. Do not silently add fuzzy model repair: freeze a common reference convention (short literal anchors or offsets; named normalization), validate **all relied-on evidence** against its document, separate fatal shape errors from field/quote coverage failures, and report valid-only **and attempted-run** performance. Decide beforehand that e.g. 1/1 valid is not necessarily “more consistent” than 2/3 with no invalid runs. Test errors and legitimate multi-line/Markdown spans.

### E. High (P1) — Retry, adjudication and stop states are not the states the code selects

`RUNBOOK.md:65–90,133–156`; `pipeline.py:171–216,256–273,289–296,342–364,425–457`. `final_attempt` takes first `valid`, else last; no retry eligibility, cap, append-only void/clear event or terminal-invalid guard. Invalid extraction → another valid attempt will select the latter, contrary to “first-pass failure is final.” `test_pipeline.py:98–117` explicitly allows further attempts after valid/invalid. An arm leakage hit preempts parsing. §6's instruction to reclassify a coincidence `valid` has **no command or adjudication read**, and would bless malformed JSON/incomplete output unless validation resumes. Fallback hits are recorded but not given a suspected status. `record-fallback`/`record-score` can mask a bad receipt with a parse-failure status. The CLI generally exits 0 even when the recorded status calls for a stop. Provider failure before a normal assistant event may be categorized as isolation failure instead of eligible for a same-model retry. A1 has no actual completeness check; M1 text “I could not produce the flags” satisfies the substring test.

**Minimum:** a single append-only attempt/adjudication rule, including which failure classes have priority, terminal invalids, provider vs unestablished receipt, stop decisions and one authorized replacement. Evaluate leakage and validity independently; a clearance restores the independently assessed status. Select the only **eligible** final attempt, never the first successful answer among arbitrary retries. Enforce (or give the executor a fail-closed gate for) stops across stages, including repeated/erroring scores. Check M1's actual FLAGS section and track document coverage.

### F. High (P1) — Paired fallback exists, but the intervention and its score are not securely paired

`pipeline.py:226–299,310–389`; `comparator.py:375–429`; `prompts/scorer.md:3–14`.

- `record_fallback` stores parseable JSON with its file index, but never checks the response's `(entry,property)` against **that index's** target. `compare` silently skips absent/nonvalid response files and passes the remaining responses as a bag keyed by the **response's own** entry/property. An answer from the wrong span can resolve a different ambiguous field. Valid JSON like `[]` is accepted by the recorder then fails on `.get`. `status:"ambiguous"` accepts any supplied value; `reason:null` becomes the string `"None"`. An invented choice *among* the permitted readings (binding when the source still doesn't select one) is never an “invention” by the syntactic metric. Distinguish **out-of-list** from **not textually warranted**. Freeze the indexed target/attempt/task/output/receipt join, validate object/enum/status and nonblank string reason, and declare missing/void/invalid fallback outcomes. No post-success claim on an incomplete batch. Retain the actual post-extraction or per-field before/after values with reasons.
- Scorer tasks include concatenated original extraction + whole report; `post`/`fallback_log` directly identifies M3, not just “typed.” Scoring both in known order invites confirmation of improvement. A changed plan-field binding may be visible only via fallback log + disappearance of a flag; method labels removed is not phase blinding. Separate equivalent, shuffled phase packets with the effective extraction/records if claiming independent phase scores, or explicitly disclose paired, unblinded phase judging and have a blinded human compare material transitions. One same-model scorer is not independent corroboration.
- **The score can be empty yet valid.** `{"phases":[]}` passes `record_score`; no matching input ID, required phase, item IDs, enum, census classes, weakening answer or completeness check. `norm("") in scored` is always true, and the search is case-insensitive, whitespace-folded over **both phases plus extraction**. A post “hit” can quote pre-only text. `source_support` for legitimate-other flags and weakening quotes are not checked. `not_violated`/some misses are absence judgments for which a decisive quote may not exist. `human_sample` won't automatically select a structurally empty but “valid” reply, or necessarily catch a wholly missing reply. Require exact expected phase/item sets, legal answers/classes and evidence by phase/source/output; use an explicit “no supporting span” for genuine absence. Validate census coverage for enumerable typed flags. Uncertain/missing/invalid and consequential transitions go to human review; retain raw replies and adjudications.

### G. High (P1) — The advertised report cannot deliver the preregistered results

`pipeline.py:310–339,367–425`; `RUNBOOK.md:118–164`; `PROTOCOL.md:27–34,209–217`. `report()` never reads `scores`, scorer votes, the human sample or `adjudication.json`. `attempt_status` counts **one selected row per planned run**, not all attempts; tokens cover selected arm receipts only. No item hits/denominators, invalid-by-stage or void ledger, weakening, adjudicated false flags, or costs by stage. M3 “added” is raw flag-object set difference, not *target hits*; `targets` counts log rows (possibly duplicate or extraneous), and missing replies are not abstentions. The operator could compute all of this separately, but §12 calls the pipeline done, and §9's remaining “analysis” has no defined join/selection for multiple votes, unresolved votes or absent scores. `score_tasks` overwrites `task.md` and `scored-output.txt` **before** discovering `key.json` already exists. With changed eligible rows a second invocation can leave old scores bound to **different** output. `write_json` also prevents the prescribed in-place comparator recomputation after a re-freeze. These are custody issues, not conveniences.

**Minimum:** bind versioned score packets/keys to immutable attempt and comparison digests before exposing scores; refuse an inconsistent existing batch *before* any write. Retain both versions on re-freeze. Add a small deterministic analysis (or an explicit tested operator table) joining all attempts, phases, final vote/adjudication and per-stage usage incl. smoke and retries, with missing values and zero-valid cells explicitly undefined. Predefine whether context-only flags enter the primary false-flag denominator (scorer:11 includes them in census), how ties/invalid runs affect consistency, and whether a lost target offsets gains. Test one mixed-status end-to-end batch through **reported item outcomes**, not just removal of one raw flag.

## Decision boundary

The minimum resubmission is not a larger experiment: fix the receipt/task join and per-launch bridge policy; correct empty/unassessed/complete/candidate and authority cases; align plan flags and rubric; make validity, leakage, fallback and final vote selection enforceable; demonstrate item-level reporting on canned complete **and** failed trajectories. Pin the resulting bytes. The model choice, 66 rows, controls, descriptive scope and separate execution approval can stand. No fixes were applied in this review.

**Unverified:** realized P7 isolation, exact successful clean Opus receipt/preamble and availability of the needed events after retention on the intended runner; provider billing/limits; frequency of quote rejections, model output length or scorer bias; historical owner assent; real-directory pipeline behavior in this no-write review. The historical non-P7 run evidence tests assumptions about field meanings and shapes, not that future P7 launches will pass or fail. No historical private reasoning is reproduced below.

## Command receipt

Key executed checks (all local): `git rev-parse HEAD`, `git branch --show-current`, `sha256sum` on both documents, range diff/stat/check, builder `--check`, 39 comparator tests, 2 build-check tests and full 48-test discovery with process-memory temporary I/O. Raw task hashes, rendered boundaries, actual status fields/system tool declarations, and provider/model events were independently inspected. Non-decisive probes included missing optional modules/paths and a `jq` aggregate which assumed `resolvedModel` was a string; the corrected query uses `step.model`. Detailed shell commands for this re-review follow. The final shell command writes only this artifact and verifies it exists.

<details><summary>Local commands submitted (batch-level tool result)</summary>

**returned**

```sh
cat docs/contracts/core.md >/dev/null; git rev-parse HEAD; git branch --show-current; P=experiments/perspective-intent-research; sha256sum "$P/p7-run1/"{PROTOCOL,RUNBOOK}.md; git diff --stat 29d84386 233821e1 -- "$P"; git diff --stat HEAD -- "$P"; ls -ld /home/bline/.pi/agent/sessions/--home-bline-code-work-engine--/subagent-artifacts/outputs/d8af0eb0-8a22-4423-bf8f-ba20b238046a/reviews/p7-run1-protocol-astra-20261124; ls -l "$P/p7-run1/"; ls -ld /home/bline/.pi/agent/npm/node_modules/pi-subagents; printenv PI_SESSION_FILE; pwd
```

**returned**

```sh
cd experiments/perspective-intent-research/p7-run1; nl -ba PROTOCOL.md; nl -ba RUNBOOK.md
```

**returned**

```sh
cd experiments/perspective-intent-research/p7-run1; nl -ba comparator.py; nl -ba schema/obligation-extract-v2.json
```

**returned**

```sh
cd experiments/perspective-intent-research/p7-run1; nl -ba pipeline.py; nl -ba test_pipeline.py
```

**returned**

```sh
cd experiments/perspective-intent-research/p7-run1; nl -ba build_tasks.py; rg -n 'def test_|tempfile|mkdtemp|open\(|write|patch\(' test_*.py; for f in prompts/*.md prompts/rubric.json agents/*.md fixtures/*.md leakage_terms.txt; do echo "==== $f"; nl -ba "$f"; done
```

**returned**

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -B skills/durable-state/scripts/codex_chatboard.py --repository . read --limit 20 --active-claims | jq -c '{claims:(.claims|length),messages:(.messages|length)}'; d=d5209c61f8d7f8bc185531f316876ce6f5630fdecdef0cdc46e992d0acdeff2a; b=$HOME/.local/state/work-engine/tools/context-status/$d/work-engine-context-status; if test -r "$PI_SESSION_FILE" && [ "$(sha256sum "$b"|cut -d' ' -f1)" = "$d" ]; then "$b" pi --session "$PI_SESSION_ID" --session-file "$PI_SESSION_FILE" | jq -c '{session_id,epoch,recorded_model,latest_request_status,tokens:.latest_request.input_tokens}'; fi; rg -n 'wind-walker' docs/contracts/context-recovery.md | head -4; ls -a ~/.pi/agent/sessions/--home-bline-code-work-engine--/2026-11-23T05-47-09-629Z_01a206f1-a5b7-729e-b3e2-b53250edde5a/d8af0eb0-8a22-4423-bf8f-ba20b238046a/run-0/
```

**returned**

```sh
P=experiments/perspective-intent-research/p7-run1; PYTHONDONTWRITEBYTECODE=1 python3 -I -B "$P/build_tasks.py" --check; python3 -I -B -c 'import importlib.util as i;print({k:bool(i.find_spec(k)) for k in ("pyfakefs","fsspec","fs","pytest")})'; sed -n '1,120p' skills/wind-walker/SKILL.md; ls -d /tmp/pi-subagents-uid-*/async-subagent-runs; R=/tmp/pi-subagents-uid-$(id -u)/async-subagent-runs; ls -d "$R"/d8af0eb0*; rg -l '"agent"\s*:\s*"(reviewer|p7-arm|p7-scorer)"|claude-sdk/claude-opus-5-5' "$R" -g status.json | head -25
```

**returned**

```sh
cd experiments/perspective-intent-research/p7-run1; nl -ba test_comparator.py; rg -in 'negative control|NC-[ABC]|transfer test|case [BF]|exclusive causality|verifier claims|observed against|Unchanged from the cleanup requirement' tasks/{NC-C.M1,NC-A.M2,NC-B.M2,T1.M1,A2-F.M1}.md | cut -c1-210 | head -35; rg -n '^=== (PARENT|SOURCE|DERIVED) DOCUMENT' tasks/A3-F.M2.md; cd ../../..; git diff 29d84386 233821e1 -- experiments/perspective-intent-research/README.md; jq -r '.runs|group_by(.input_set,.method)|map(length)|group_by(.)|map({n:.[0],cells:length})' experiments/perspective-intent-research/p7-run1/runs/plan.json
```

**returned**

```sh
Q=$HOME/.pi/agent/npm/node_modules; S=$Q/pi-subagents; jq -r '.version' "$S/package.json"; ls -l "$Q/pi-claude-subscription/src/index.ts"; rg -n -e 'defaultContext' -e 'inheritGlobalContext' -e 'extensions.*Omitted' -e 'tools:.*empty' -e 'explicit.*skills' -e 'agentScope:.*both' "$S/docs/agents.md" | cut -c1-420; rg -n 'intercomBridge|agentScope|retention|run directory' "$S/docs/"{tool-reference,extension-api,configuration}.md | cut -c1-340 | head -55; rg -n 'toolCount\s*[:=]|\.toolCount\+\+' "$S/src/runs" | cut -c1-300 | head -100; rg -n 'provider:|promptMode|section|thinkingLevel' "$Q/pi-claude-subscription/src/index.ts" | head -65 | cut -c1-280
```

**returned**

```sh
R=/tmp/pi-subagents-uid-$(id -u)/async-subagent-runs; D=$R/d8af0eb0-8a22-4423-bf8f-ba20b238046a; ls -lah "$D" | head -35; jq -c '{id,status,context,toolCount,cwd,step:(.steps[0]|{agent,model,thinking,skills,context,toolCount,exitCode,hasTools:has("tools"),hasSkills:has("skills"),hasContext:has("context"),ext:.launchResolvedExtensions|keys?}),keys:(.steps[0]|keys)}' "$D/status.json"; jq -c 'select(.type=="message_end" and .message.role=="system")|{sections:(.message.sections|keys?),contentsz:(.message.content|length),keys:(.message|keys),mark:(.message.sections.preamble|tostring|test("intercom|orchestrator|contact_supervisor";"i"))}' "$D/events.jsonl" | head -3; jq -c 'select(.type=="message_end" and .message.role=="assistant")|.message|{provider,model,thinkingLevel,stopReason}' "$D/events.jsonl" | head -2; rg -l '"model": ?"claude-sdk/claude-opus-5-5"' "$R" -g status.json | head -6; rg -n 'tool_execution_start|toolCount|launchTaskDigest' ~/.pi/agent/npm/node_modules/pi-subagents/src/runs/background/subagent-runner.js | tail -35 | cut -c1-275; df -h /dev/shm
```

**returned**

```sh
Q=$HOME/.pi/agent/npm/node_modules/pi-subagents; rg -n 'intercomBridge:' "$Q/src" --glob '*.js' | grep -E 'Type\.|schema|params|options\.' | head -20 | cut -c1-340; nl -ba "$Q/docs/extension-api.md" | sed -n '264,277p;295,316p;412,421p'; rg -in 'rotat|truncat|retention|oldest|bounded' "$Q/src/runs/background" -g '*event*' | head -30 | cut -c1-295; rg -n 'role: "system"|message\.sections|systemPromptSections' "$Q/src" | head -25 | cut -c1-320; rg -n 'function applyClaudePrompt|registerProvider\(|provider:' $HOME/.pi/agent/npm/node_modules/pi-claude-subscription/src | head -32 | cut -c1-260; git diff 29d84386 233821e1 -- experiments/perspective-intent-research/p7-run1/agents/; rg -n 'proposed unit|Case F' docs/contracts/context-recovery.md; wc -c experiments/perspective-intent-research/p7-run1/tasks/A3-F.*
```

**returned**

```sh
P=experiments/perspective-intent-research/p7-run1; PYTHONDONTWRITEBYTECODE=1 python3 -I -B -m unittest discover -s "$P" -p 'test_comparator.py'; PYTHONDONTWRITEBYTECODE=1 python3 -I -B -m unittest discover -s "$P" -p 'test_pipeline.py' -k BuildCheck; cd "$P"; jq -r '.runs[]|[.task_sha256,("tasks/"+.task_id+".md")]|join("  ")' runs/plan.json | sort -u | sha256sum -c -; rg -in 'constructed negative control|synthetic fixture|\bNC-[ABC]\b' tasks/*.md | cut -c1-160 | head -10
```

**returned**

```sh
R=/tmp/pi-subagents-uid-$(id -u)/async-subagent-runs; D=$R/d8af0eb0-8a22-4423-bf8f-ba20b238046a; jq -c 'select(.type=="message_end" and .message.role=="system")|.message|{n:(.toolsAdded|length),names:[.toolsAdded[]?.name][:7],secnames:(.sections|keys)}' "$D/events.jsonl"| head -2; for f in $(rg -l '"model": ?"claude-sdk/claude-opus-5-5"' "$R" -g status.json | head -5); do d=${f%/status.json}; echo "run: ${d##*/}"; jq -c '.steps[0]|{model,thinking,context,skills,toolCount,exitCode}' "$f"; test -f "$d/events.jsonl" && jq -c 'select(.type=="message_end" and (.message.role=="system" or .message.role=="assistant"))|.message|if .role=="system" then {sections:(.sections//{}|keys),added:(.toolsAdded|length),rem:(.toolsRemoved|length),preambleLen:(.sections.preamble|length),bridge:(.sections.preamble|tostring|test("contact_supervisor|intercom|orchestrator";"i"))} else {provider,model,thinkingLevel,stopReason} end' "$d/events.jsonl" | head -2; done; find /usr/lib /usr/local/lib /home/bline/.local/lib -type d \( -name 'pyfakefs' -o -name 'fsspec' \) -print 2>/dev/null | head -12; rg -n 'export const PROVIDER|startsWith|return' $HOME/.pi/agent/npm/node_modules/pi-claude-subscription/src/{prompt,models,constants}.ts | tail -25 | cut -c1-230
```

**returned**

```sh
Q=$HOME/.pi/agent/npm/node_modules/pi-subagents; rg -n 'intercomBridge' "$Q/src" -g '*schema*.js' -g '*params*.js' | head -45 | cut -c1-300; ls -al $HOME/.pi/agent/extensions/subagent/ | head -12; rg -n 'ready|scope|reload' "$Q/docs/configuration.md" | rg 'config\.json|reload' | head -20 | cut -c1-300; L=experiments/perspective-intent-research/stage0; rg -n 'after bounded receiver|H3 —|H4 —|Owner checkpoints' "$L/case-f/packet/PLAN.md" "$L/case-f/packet/review-r1/"* | cut -c1-400; rg -n 'termination actor' "$L/case-b/packet/CONTROLLED-BUILDER-ADMISSION-REVISION.md" | cut -c1-570; jq -r '.entries[]|select(.source_line==4584)|.text' "$L/case-b/session-excerpt.json" | rg -o '.{0,60}it verifies.{0,65}'; cat docs/contracts/context-recovery.md; RG=docs; rg -n '^## |^### ' "$RG/contracts/"{evidence-and-claims,instruction-design,context-and-review}.md | head -20
```

**returned**

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -I -B - <<'PY'
# Run unittest discovery with only temporary fixture I/O redirected to Python memory.
import builtins, io, itertools, os, posixpath, runpy, sys, tempfile
from unittest.mock import patch
P='experiments/perspective-intent-research/p7-run1'; root='/__p7_readonly_review_mem__'; files={}; dirs={root}; count=itertools.count(); orig_open=builtins.open; orig_ls=os.listdir; orig_exists=os.path.exists; orig_isdir=os.path.isdir
mem=lambda p:isinstance(p,str) and (p==root or p.startswith(root+'/'))
class Temp:
 def __init__(self): self.name=root+'/'+str(next(count)); dirs.add(self.name)
 def cleanup(self):
  for k in list(files):
   if k.startswith(self.name+'/'): del files[k]
  dirs.difference_update([k for k in dirs if k==self.name or k.startswith(self.name+'/')])
def mkdir(p,*a,exist_ok=False,**kw):
 assert mem(p)
 if p in dirs and not exist_ok: raise FileExistsError(p)
 while mem(p): dirs.add(p); p=posixpath.dirname(p)
def op(p,m='r',*a,**kw):
 if not mem(p):
  assert not any(c in m for c in 'wa+x'); return orig_open(p,m,*a,**kw)
 if 'r' in m and p not in files: raise FileNotFoundError(p)
 b=io.BytesIO(files[p] if 'r' in m else b''); f=b if 'b' in m else io.TextIOWrapper(b,encoding='utf-8'); close=f.close
 def end():
  if not f.closed and 'w' in m: f.flush(); files[p]=b.getvalue()
  close()
 f.close=end; return f
def ls(p):
 if not mem(p): return orig_ls(p)
 if p not in dirs: raise FileNotFoundError(p)
 return sorted(posixpath.basename(k) for k in (dirs|set(files)) if posixpath.dirname(k)==p)
sys.argv=['unittest','discover','-s',P,'-p','test_*.py']
with patch.object(tempfile,'TemporaryDirectory',Temp), patch.object(builtins,'open',op), patch.object(os,'makedirs',mkdir), patch.object(os,'listdir',ls), patch.object(os.path,'exists',lambda p:(p in files or p in dirs) if mem(p) else orig_exists(p)), patch.object(os.path,'isdir',lambda p:p in dirs if mem(p) else orig_isdir(p)):
 runpy.run_module('unittest',run_name='__main__')
PY
```

**returned**

```sh
R=/tmp/pi-subagents-uid-$(id -u)/async-subagent-runs; for d in "$R"/{d8af0eb0-8a22-4423-bf8f-ba20b238046a,95b437ab-8be5-4b55-b9cc-814aff7fe62b,9805c42a-d7d1-470c-bee1-1dc9f716c73a}; do echo "${d##*/}"; ls -lh "$d"/events*; jq -s -c '{system:[.[]|select(.type=="message_end" and .message.role=="system")]|length,adds:[.[]|select(.type=="message_end" and .message.role=="system")|(.message.toolsAdded|length)][:4],user:[.[]|select(.type=="message_end" and .message.role=="user")]|length,starts:[.[]|select(.type=="tool_execution_start")]|length,head:(.[0]|{type,seq}),tail:(.[-1]|{type,seq})}' "$d/events.jsonl"; done; Q=$HOME/.pi/agent/npm/node_modules/pi-subagents; rg -n 'events\.jsonl' "$Q/src" -g '*.js' | grep -Ei 'max|rotate|retention|bounded|5 \*|FILE|compact' | head -30 | cut -c1-300; rg -n 'intercomBridge' "$Q/src/extension" "$Q/src/tools" -g '*.js' | head -25 | cut -c1-270; nl -ba $HOME/.pi/agent/npm/node_modules/pi-claude-subscription/src/prompt.ts | sed -n '20,65p'; jq -sc 'map(select(.type=="model_change" or .type=="thinking_level_change"))|map({type,modelId,provider,thinkingLevel})|.[-2:]' "$PI_SESSION_FILE"; test ! -e /home/bline/.pi/agent/sessions/--home-bline-code-work-engine--/subagent-artifacts/outputs/d8af0eb0-8a22-4423-bf8f-ba20b238046a/reviews/p7-run1-protocol-astra-20261124/review.1562e9c3.md
```

**returned**

```sh
Q=$HOME/.pi/agent/npm/node_modules/pi-subagents; rg -n 'intercomBridge' "$Q/src" -g '*.d.ts' | rg 'params|run|delegat|workflow|tool|contract' | head -36 | cut -c1-275; rg -n 'intercomBridge' "$Q/bundle" -g '*.js' -l | head -8; R=/tmp/pi-subagents-uid-$(id -u)/async-subagent-runs; jq -csr 'map(select((.steps[0].resolvedModel//.steps[0].model//"")|contains("claude-sdk")))|map(select((.steps[0].toolCount//0)==0))| map([.runId,.steps[0].agent,.steps[0].model,.steps[0].context, (.steps[0].toolCount|tostring),(.steps[0].exitCode|tostring)] | @tsv)|.[:14][]' "$R"/*/status.json | cut -c1-190; rg -n 'intercomBridge' "$Q/index.d.ts" | head -10; git diff --check 29d84386 233821e1 -- experiments/perspective-intent-research; d=d5209c61f8d7f8bc185531f316876ce6f5630fdecdef0cdc46e992d0acdeff2a; b=$HOME/.local/state/work-engine/tools/context-status/$d/work-engine-context-status; if [ "$(sha256sum "$b"|cut -d' ' -f1)" = "$d" ]; then "$b" pi --session "$PI_SESSION_ID" --session-file "$PI_SESSION_FILE" | jq -c '{epoch,tokens:.latest_request.input_tokens,latest_request_status}'; fi
```

**returned**

```sh
p=experiments/perspective-intent-research/p7-run1; rg -n 'digest|privacy|private|retention|adjudicat|latency|invalid' "$p/RUNBOOK.md" | tail -32; rg -n 'F1a|N2' reviews/obligation-provenance-plan-opus-20261123/review-r2.md | head -6 | cut -c1-240; rg -n 'receipt|covered|observation' docs/contracts/evidence-and-claims.md | head -8 | cut -c1-300; jq -r '.runs[0]|keys' "$p/runs/plan.json"; ls -al "$p/runs"; git --no-optional-locks status --short -- "$p"; rg -n 'async-subagent-runs|retention' ~/.pi/agent/npm/node_modules/pi-subagents/docs/agents.md | rg -i 'retention|cache|sweep|delete|tmp' | head -10 | cut -c1-320
```

**returned**

```sh
R=/tmp/pi-subagents-uid-$(id -u)/async-subagent-runs; jq -csr 'map(select((.steps[0].model//""|tostring)|contains("claude-sdk")))|map(select((.steps[0].toolCount//0)==0))|map([.runId,.steps[0].agent,.steps[0].model,.steps[0].context,(.steps[0].toolCount|tostring),(.steps[0].exitCode|tostring)]|@tsv)|.[:16][]' "$R"/*/status.json | head -16; Q=$HOME/.pi/agent/npm/node_modules/pi-subagents; rg -n 'intercomBridge' "$Q/src" --glob '*.js' | rg -i '(tool-parameters|tool-api|public-tool|options\.intercomBridge|\["intercomBridge"\]|schema|normalize|surface|type\.Optional|additional)' | head -30 | cut -c1-330; jq -c '{state,runId,agent,step:(.steps[0]|{launchModel,resolvedModel,requestedModel,processTerminal:(.processTerminal|keys?),status,turnCount}),keys:keys}' "$R/95b437ab-8be5-4b55-b9cc-814aff7fe62b/status.json" | cut -c1-1250; ls -l "$R/9805c42a-d7d1-470c-bee1-1dc9f716c73a"/events*; jq -c '.steps[0]|{context,skills,toolCount}' "$R/9805c42a-d7d1-470c-bee1-1dc9f716c73a/status.json"; nl -ba experiments/perspective-intent-research/stage0/case-f/packet/QUALIFICATION-AMENDMENT.md | sed -n '3p;7p' | cut -c1-330; rg -n '^## |^### ' reviews/p7-run1-protocol-astra-20261124/review.md | head -21
```

**returned**

```sh
R=/tmp/pi-subagents-uid-$(id -u)/async-subagent-runs; jq -sr 'map(select((.steps[0].toolCount//0)==0 and .steps[0].exitCode==0)) | map([.runId,.steps[0].agent,.steps[0].model,.steps[0].context,(.toolCount|tostring),(.steps[0].toolCount|tostring)]|@tsv)|.[:8][]' "$R"/*/status.json; D=$R/9b42d0e3-8d69-4211-a607-3366fb5d9659; jq -c '{state,toolCount,step:(.steps[0]|{model,thinking,context,toolCount,exitCode})}' "$D/status.json"; Q=$HOME/.pi/agent/npm/node_modules/pi-subagents; nl -ba "$Q/src/runs/background/subagent-runner.js" | sed -n '2818,2842p'; nl -ba "$Q/src/runs/background/async-status.js" | sed -n '238,254p'; rg -n 'Type\..*IntercomBridge|intercomBridge.*Optional|IntercomBridgeSchema' "$Q/src" "$Q/index.js" -g '*.js' | head -12 | cut -c1-260; rg -n 'retention|events\.jsonl' "$Q/docs/async-runs.md" "$Q/docs/lifecycle.md" 2>/dev/null | head -12 | cut -c1-300; rg -n 'case_b_session_excerpt' experiments/perspective-intent-research/stage0/MANIFEST-case-b.json; rg -n '^## ' skills/manual-orchestration/references/worker.md | head -6
```


</details>
