# Adversarial plan review: obligation-provenance safeguard

Reviewer: Opus 5.5 (`claude-opus-5-5`). Read-only review. No files edited, no builds, no network, no live inference.

## Subject verification (verified by running)

- Repository `/home/bline/code/work-engine`, branch `rescue-detached-20260911`, HEAD `e882e8cc820a49e808a8efa67c69a5c50aa796f4`.
- Plan `planning/obligation-provenance-safeguard-plan-20261123.md`, SHA-256 `e701c0688d7f3f30ce66cc02da826616699b9e34a7f65d061aa779ba982a73e2`. Matches.
- All eight packet artifacts match the requested SHA-256 values: 043 `4bf5ee60…`, PLAN.md `c7511ad7…`, re-escalation ASTRA `12127e46…`, QUALIFICATION-AMENDMENT `93e004dc…`, resolved-object ASTRA `e8304cda…`, OPERATOR-DECISION `d0055826…`, DOCKER-AUTOREMOVE-PROPOSAL `f3598d2e…`, SCOPE-DECISION `b0b2b8fd…`.

## Verdict: `revise_and_resubmit`

The §4 semantics point the right way and agree with doctrine. Making a property binding requires an authority specific to that property. A general citation, or §3.9 itself, does not count. Selecting a realization stays model judgment. These match core §1.1 and §4 and evidence-and-claims §3.9.

The plan still fails its own acceptance claim. The judgments that decide F1 (how 043 is typed, and whether the amendment counts as an owner decision) are not made by the mechanism. They are made by whoever writes the requirements document and the fixture. On the manual route that is the same supervisor who made the original error. The plan also has no category for the reviewer's legitimate, realization-specific point. The typed enforcement point it names does not consume findings. And the manual route has no mechanical consumer at all. These points need changes to the semantics, not wording edits. A re-review after correction is warranted.

## My judgment on the F1 authority question

These conclusions come from reading the artifacts.

- **043 does not establish Host-attributed removal; for that property it is ambiguous to unspecified.** It says "The Host owns the child, same-session framed handshake, and exact **stop**" (043:11). That sentence covers stop, not removal. Clean cancel "concludes only after … exact container removal **observed** against the owned identity" (043:19). That is a passive outcome observation with no actor. The Host's assigned work is "qualification-only terminal **observation**" (043:23).
- **PLAN.md does not reduce to "H4 only."** PLAN.md:3 defers: "The latter documents [041, 043] own the broader qualification contract; this packet makes the checks observable." H3 (PLAN.md:15) requires the Host to "Distinguish confirmed removal, removal failure and unknown cleanup". H4 (:16) requires success "without fallback stop/kill" and independent post-exit absence. Line 27 covers "destructive cleanup (including failure paths)". Host-issued `rm` was the realization selected at the time, and H3 made a failure of the Host's own removal a required, distinct state.
- **The reviewer's point was legitimate and supported, conditional on that realization.** The re-escalation review said "For this realization, yes … If another actor removes it after `wait` while the Host cannot even start its removal, a positive result would qualify a Host whose cleanup did not work" (12127e46:11). Its P0 found that a removal setup failure was folded into `ConfirmedAbsent` (12127e46:17). Both are supported defects under H3 for as long as the Host performs removal.
- **The strengthening happened when the property became independent of the realization.** The later review made "the Host positively completed normal removal" an independent §3.9 claim (e8304cda:9). It ruled out the alternative: "Neither Engine auto-remove … supplies the amended Host action" (e8304cda:32). It also steered away from the owner: "Do not ask for permission to accept eventual absence instead" (e8304cda:5). The decision request then labelled this as "H1–H5 requirements are unchanged … Host normal removal must be affirmative and attributable" (OPERATOR-DECISION.md:5). Exclusive causal attribution on a shared Engine, regardless of realization, has no parent span.
- **Authority actually used:** the operator's overnight turn at 2026-11-21T08:11Z (session line 8873): "try to continue; use escalation rules and architectural reviews to help make decisions." The session shows no user turn between then and 15:07Z. The amendment was adopted, and both repair writers were dispatched at 08:59:31Z and 08:59:41Z (session lines 9350, 9353), without consulting the operator or RL. The re-escalation review itself named RL as owner of "any change in clean-close or consumer meaning" (12127e46:69). The delegation was real authority for realization decisions. It was not a decision by the owner of the 043 acceptance meaning.
- At 15:14Z the operator asked "what is the negative outcome there?" The agent answered "there's no negative cleanup outcome" (line 9739), which side-stepped the reviewer's point. The operator's realization change (create-time `--rm`, Docker as outcome owner; line 9743) dissolved the point rather than refuting it, because no Host removal step was left to attribute.

**Consequence for the plan:** the U4-A chain contains two distinct assertions. A correct F1 must keep (A), the conditional, supported defect, and catch (B), the realization-independent strengthening. The plan's F1 treats them as one "Host-attribution assertion" (finding H1 below).

## Findings

### B1 (blocker): the decisive semantic judgment has no owner, so F1 passes by how the fixture is built
- **Plan:** §4: "`ambiguous` is set when the source text supports more than one reading" (no actor named). §5 C1 inputs: "a small structured requirements document with ids, properties, statuses, owner…" (author not named). §6 F1: "(043's outcome owner is `ambiguous`…)". F4: "043 wording typed as `outcome_owner: ambiguous`".
- **Evidence (verified by reading):** in the real chain, the reviewer typed 043 as requiring attribution. It wrote "RL `043…:11,17–25` assigns the Host the child and exact stop … yes, an affirmative Host-attributed normal removal is required", and "Narrowing success to eventual absence … would require an owner decision" (12127e46:9–13). The supervisor who would write both the C3 provenance block and any requirements document adopted that reading within minutes. The review was dispatched at 08:52Z (session line 9319) and the repair writers at 08:59Z, bound to the amendment's SHA. A supervisor who sincerely misread 043 would type it as `outcome_owner: binding Host`. C1 would then return `supported` and exit 0.
- **Why it matters:** C1 moves the judgment instead of removing it. F1 tests the fixture author's reading of 043, not the safeguard. The plan says "A safeguard that does not catch F1 does not pass acceptance", but F1 as written cannot fail.
- **Fix:** see corrections 1 and 2.

### B2 (blocker): `owner_adopted` does not bind owner identity per property, so the supervisor's own amendment would satisfy it
- **Plan:** §4: "`owner_adopted`, citing an owner decision record that names the property". §9 Q6 asks only about record *format*.
- **Evidence (verified by reading):**
  - QUALIFICATION-AMENDMENT.md:7 names the property: "Keep Host-attributed normal cleanup … including a **positively attained normal-removal witness**".
  - It is an adoption record ("**Adopt**", :3) under "existing operator authority and the overnight escalation request". architectural-escalation.md:8 and :65–66 call this "the authorized design owner" and "the authorized owner's disposition", and do not separate a design owner from the owner of an acceptance condition.
  - Board `author`/`session_id` are caller-supplied, non-empty strings (`codex_chatboard.py:252–284`), so a board message does not authenticate an owner.
  - F1's premise, "no owner decision names the property", is therefore false on the literal artifacts. It holds only because the fixture author decides the amendment does not count.
  - The later BLOCK cites the amendment as part of "the owning rule" (e8304cda:9). Support thus becomes transitive unless a failed adoption is kept out of the requirements set.
- **Fix:** correction 2.

### H1 (high): no realization-conditional category, so the legitimate point is either suppressed or the strengthening is laundered through F7
- **Plan:** §4 assertion statuses are only `inherited`, `proposed` and `owner_adopted`. F7 says "genuine defect violating a `supported` requirement → still allowed to block". F1 classifies the whole "Host-attribution assertion" as `requires_owner_decision`.
- **Evidence:** see the judgment section above (H3 at PLAN.md:15; 12127e46:11,17; e8304cda:5,9,32; OPERATOR-DECISION.md:5). The pilot's P7 target requires that the method "must preserve the reviewer's legitimate point … A proposal that simply drops attribution without addressing that fails" (perspective-intent-pilot.md:628).
- **Why it matters:** classifying 12127e46's P0 as `requires_owner_decision` stops a supported defect from blocking. That breaks the plan's non-goal "Changing ordinary defect findings". Admitting it through F7, citing H3 as the violated supported requirement, lets the realization-independent version through with it, because property-level matching cannot tell them apart. The same gap would flag the operator's own Docker design: "reject failed, missing or mismatched `AutoRemove` before ready" (SCOPE-DECISION:7) is a `path_fact` made binding without a parent.
- **Fix:** correction 3.

### H2 (high): the typed enforcement point is misidentified
- **Plan:** §5 C2: "campaign remediation selection (`slice-campaign` `RemediationSelection`, which already requires verified acceptance sources), together with the episode reducer". §6: "remediation selection refuses a proposal-only blocker".
- **Evidence (verified by reading):**
  - `RemediationSelection::new` (`slice-campaign/src/remediation.rs:54–88`) checks only that `acceptance_sources` is non-empty with `status == "verified"`, that its text fields are present and its SHA is well formed, and that the raw input sizes are valid. Its documentation calls these "Trusted startup inputs" (:30–31). The replay check repeats the string test (:4329–4333). No finding data reaches it.
  - The per-finding consumer is the builder evaluation `valid: bool` (`initial_outcome.rs` `BuilderEvaluationDecision`). An outcome becomes `Reported` when every finding is `!valid || verified_resolved` (`initial_outcome.rs:1271–1291`; `remediation.rs:718–737`).
  - The reducer reads only the verdict: phase `remediation` iff verdict ≠ `acceptable_as_is` or questions remain (`reducer.rs:297,313–327`). `remediation_required` needs only some pending finding (`implementation_review_v1.rs:107–110`).
  - An owner-routing pattern already exists: an evidence admission that is not established leads to `evidence_unestablished` / `route_unestablished_claim_to_owner` (`reducer.rs:283–325`; `state.rs:286–320`).
- **Why it matters:**
  - A receipt bound at `RemediationSelection` would be checked by nothing that looks at findings.
  - On today's typed route, "mandatory remediation" is the builder's own per-finding `valid` judgment. That is the same kind of acceptance by the acting role that U4-A shows going wrong.
  - Option (b), the sidecar, cannot reach the pure verdict rule unless `record_result`'s inputs change. "Leaving v1 unchanged" therefore does not deliver enforcement at the reducer.
- **Fix:** correction 6.

### H3 (high): `OrganizationalRequirements` is not a parent representation for finding provenance, and its field names collide with the vocabulary
- **Plan:** §5 C1: "typed `OrganizationalRequirements` conditions where available". §9 Q3.
- **Evidence (verified by reading):**
  - `work-engine-organization/src/schema.rs:103–135` closes the vocabulary to how a review is conducted: `Continuity` fresh_initial/retained, `Mutation` none, `IndependenceSource` `state.builder_context`, `Integrity` verified_artifacts, `EvidenceCustody` execution_evidence_custody, `Dimension` provider_family/execution_confinement.
  - Its single obligation is `candidate_review` on `campaign_candidate` (:211–233). `Normative` is defined at :237–247.
  - The `[dependencies]` in `slice-campaign/Cargo.toml` do not include `work-engine-organization`.
  - `work-engine-review-preparation` is inert review configuration with no findings (`lib.rs:1–30`).
- **Why it matters:** the answer to Q3 is **no**. The typed route has no typed parent for the subject's acceptance requirements, so S2 needs the same structured requirements document as S3. A subject-level `independence`, `evidence_custody` or `verifier` assertion could also be resolved against the review-level `Normative` fields and come back as a false `supported`.
- **Fix:** correction 7.

### H4 (high): the manual route has no mechanical consumer, and M1 and M2 rely on the actor who erred
- **Plan:** §5 C3 options M1–M3.
- **Evidence (verified by reading):**
  - U4-A dispatched its writers with Pi's native `subagent` tool, using free-text tasks that named the amendment's SHA (session lines 9350, 9353).
  - `role_host.py` `launch(…, prompt, …)` (lines 326, 562, 1513) takes a prompt string and does no packet validation.
  - `handoff_validator.py` checks root-handoff referents, and `--advisory-only` forces exit 0 (:266).
  - `architectural-escalation.md` has no receipt field.
  - `worker.md:3` already tells workers to report "requirement gap, stale premise or ownership conflict", and the builders still implemented the amendment.
- **Protection achievable now:**
  - M1 is after-the-fact audit only.
  - M2 checks, at instruction level, that a receipt exists. The supervisor produces that receipt from inputs it typed, so M2 catches an omission but not a sincere misreading, which is what happened in U4-A.
  - M3 would need a hook in the Pi subagent host or extension, outside `skills/manual-orchestration/scripts/`. Even there it could check only that the receipt is present and its digest matches.
  - **The U4-A route (supervisor Markdown amendment, then free-text Pi `subagent` dispatch) has no mechanical consumer today.**
- **Fix:** corrections 1, 2 and 8.

### M1 (medium): exit 0 includes `proposal`, so a "passing receipt" does not mean the assertions may be adopted
- **Plan:** §5 C1: "`0` all assertions `supported` or `proposal`". C3/M2: "repair dispatch packets must carry the passing receipt". The exit codes for `ambiguous_parent` and `contradictory` are not specified.
- **Why it matters:** a supervisor can label the attribution `proposed`, get exit 0, and then adopt it as binding in prose.
- **Fix:** correction 5.

### M2 (medium): F1 contradicts F4 and §4, and does not say which boundary it tests
- **Plan:** §4: "`ambiguous_parent`: the cited parent field is `ambiguous`". F1 expects `requires_owner_decision` *because* 043 is typed `ambiguous`. F4 gives the same typing and expects `ambiguous_parent`.
- **Evidence:** each document cites differently:
  - 12127e46 cites specific spans (043:11,17–25; PLAN:15–17,24–27) → `ambiguous_parent`.
  - The amendment cites the review plus general operator authority → `requires_owner_decision`.
  - e8304cda cites 043, PLAN, the amendment and §3.9.
  - OPERATOR-DECISION cites "H1–H5", which does not cover the property → `requires_owner_decision`.
- **Fix:** correction 4.

### M3 (medium): parent precedence is unrecorded
- **Plan:** §1 item 2 and F1 treat `PLAN.md` H4 as the controlling parent ("required only").
- **Evidence:** PLAN.md:3 says 041 and 043 "own the broader qualification contract". The research note repeats the overstatement ("The plan itself asked for the outcome only").
- **Why it matters:** if PLAN.md is typed as the controlling parent, the finding is trivially "absent → strengthening". The property should be judged against 043, which is ambiguous. Without a recorded precedence, `contradictory` and "covering" are undefined when parents differ.
- **Fix:** the requirements set records precedence and ownership per source. F1 judges the attribution property against 043, and H4 against PLAN.md.

### M4 (medium): coverage is checked per property, never per value
- **Plan:** §4 rule: "covering that exact property".
- **Why it matters:** a parent `outcome_owner: binding Host` would "cover" `outcome_owner: Host, exclusively causal on a shared Engine` (e8304cda:3,17), which is a strengthening within a single property.
- **Fix:** values come from closed enumerations and are compared by equality or explicit containment, as `RealizationConstraint.allowed` already does. A free-text value is never mechanically "covered"; it returns `requires_owner_decision` or `not_established`.

### M5 (medium): scope is self-declared, and F1's actual wording may fall outside the vocabulary
- **Plan:** §4: "Only assertions in the closed v0 property vocabulary are in scope". C3: "Any such disposition … that asserts an in-scope obligation carries a structured provenance block".
- **Evidence:** the amendment states the requirement as success-predicate and evidence content: "Derive clean cancellation once after all helper evidence, including a positively attained normal-removal witness" (QUALIFICATION-AMENDMENT.md:7). That maps only loosely onto initiator, outcome_owner, verifier, custody or path_fact. instruction-design §3.6 says exhaustive commands require a world that is genuinely closed. This vocabulary is closed by the plan, not by the domain.
- **Why it matters:** the author decides whether to declare an assertion. Leaving the block out, or rephrasing, evades the check. F3's "not flagged" passes only because nothing is declared, so it tests nothing.
- **Fix:**
  - State that classification is declared by the author and may be challenged by an independent reviewer (or, later, a P7 extractor).
  - Record "undeclared in-vocabulary assertion found on review" as a provenance defect.
  - Add an `acceptance_predicate` / required-evidence-content property, or show F1's mapping explicitly.
  - Rewrite F3 to declare its realization choices as `realization_conditional` (correction 3).

### M6 (medium): already-adopted amendments and actor narrowing are mishandled
- **Plan:** §4 gives `requires_owner_decision` the consequence "Same as `proposal`… cannot by itself force remediation". No fixture covers narrowing.
- **Evidence:**
  - The research note says "An accepted amendment is a real obligation until its owner supersedes it" (perspective-intent-research-note.md:161).
  - The pilot's Stage 1c (:475) and governance constraint 15 (:739) say the checks are advisory and "never weaken an accepted obligation silently … treat an owner-accepted amendment as binding until superseded".
  - §3.9: "an actor cannot replace an unestablished claim with a weaker one it can establish".
- **Why it matters:** applied to an adopted amendment, the stated consequence lets a builder treat the amendment as optional. That is a silent weakening, and §3.9 forbids it just as firmly as strengthening. The plan also turns an advisory research check into an enforcing one without saying so.
- **Fix:** correction 9.

### M7 (medium): a cheaper intervention on the real route is missing
- **Evidence:**
  - `architectural-escalation.md:65–66` already requires "the authorized owner's disposition, the exact retained or revised plan".
  - QUALIFICATION-AMENDMENT.md:3 says "Retain the exact cleared `review-r2/PLAN.md` … and H1–H5", while its "Amended qualification rule" (:7) changes the clean-cancel predicate.
  - The escalation hold already exists (`architectural-escalation.md:44–47`).
  - RL aligned within minutes once asked (research note line 132: board #6173, #6176).
- **Why it matters:** a required per-change table in the disposition would have shown "owner: RL; decision: none" at 08:59Z. Columns: changed or reinterpreted acceptance condition | quoted parent span (path, lines, digest) | owner of that condition | owner decision reference or `pending`. A rule that repairs depending on a `pending` row stay held would have stopped the repair dispatch, with no Rust. A reviewer-reference rule would add more: quote the exact span for anything claimed as inherited, label interpretations as interpretations, and never discourage the owner question (contrast e8304cda:5).
- **Fix:** correction 8.

### L1 (low): §1 item 3 merges two reviews
- **Evidence (verified by running `grep`):** `12127e46` contains no §3.9 or evidence-and-claims citation. §3.9 appears only at `e8304cda:9`. The research note's Case F attributes this correctly.
- **Why it matters:** F8 ("§3.9") does not model the re-escalation finding, which cited specific spans. This is why B1 decides F1.

### L2 (low): S2 is not justified by evidence
- No typed-route amplification incident is cited. S2 needs a v2 schema or sidecar plus new consumers (H2), and has no typed parent (H3).
- Under the pre-release simplicity policy, S2 should wait for evidence: a P7 result or an incident on the typed route.

### I1 (info): already correct
- Rule and dispositions match core §1.1/§4 and §3.9. In particular, "§3.9 requires an independent authority and cannot be its own source" matches §3.9 condition 2.
- The non-goals are right.
- "Neither route counts as protected until its actual enforcement point consumes the check's result" is the right standard.
- The exit-code convention matches `handoff_validator.py:8–11`.
- The risks section correctly anticipates ritual compliance and false confidence.

## Answers to the six adversarial questions

**1. Semantic authority.**

What C1 can prove mechanically:
- a citation resolves to an id in a digest-pinned requirements set at the stated revision;
- the cited span exists in the source bytes at that digest;
- the property id is equal, and the value is equal or contained within a closed enumeration;
- the recorded parent status is `binding`;
- a decision record's digest matches, it names the property id, and its *recorded* origin equals the requirement's *recorded* owner;
- a general citation (no requirement id) never resolves;
- an "unchanged" claim matches the named source revision, not the accumulated set;
- a record whose own receipt failed cannot be used as support.

What needs semantic judgment:
- typing prose into property, value and status, including `ambiguous`;
- deciding whether a statement is an obligation assertion, which property it concerns, and whether it is conditional on a realization;
- deciding whether a defect's remedy admits other realizations;
- deciding whether a general delegation covers a property.

The plan brings judgment into the deterministic component through:
- the actorless "`ambiguous` is set when …";
- the unattributed "structured requirements document";
- the typings asserted in F1 and F4;
- C3, where "the supervisor runs the C1 CLI" over a block the supervisor wrote.

As written, the authority for these judgments is whoever writes the inputs. It should be the parent's owner for typing and the condition's owner for adoption. Extractors and reviewers may only propose.

**2. False positives.**
- A security finding that "the verifier must be independent" is an in-vocabulary `independence` assertion. If its author declares it, it becomes `requires_owner_decision` and cannot block. If the author does not declare it, the mechanism never sees it. Either way the outcome depends on self-classification (M5).
- The plan needs the realization-conditional rule (H1): a defect with a named consequence may block the selected realization, but may not require a property that is independent of the realization.
- A finding that a realization breaks an accepted path constraint is handled correctly (F7) only when an owner-accepted typed parent exists (B1). With `OrganizationalRequirements` it may resolve against the wrong level (H3).
- The operator's own `AutoRemove`-before-ready requirement would be flagged without H1.

**3. False negatives.**
- F1 passes trivially: 043 is pre-typed (B1), and the amendment is excluded as an owner record by the fixture author's choice (B2).
- A citation to a generally authoritative document is caught only when it carries no requirement id. A specific span in 043 resolves to whatever typing the author supplied.
- Phrasing outside the vocabulary evades the check (M5).
- Strengthening within one property evades it (M4).
- Exit 0 with `proposal` evades it (M1).

**4. Enforcement.**
- Typed route: there is no protection today. `RemediationSelection` does not consume findings (H2). If the builder-evaluation and verdict consumers were added, protection would be preventive, but only for findings that carry an obligation block checked against an owner-typed parent.
- Manual route: no mechanical protection today. The U4-A route has no consumer at all (H4). M1 is detective, and M2 is an instruction-level presence check. M3 needs a host hook that does not exist in the manual-orchestration scripts.

**5. Authority preservation.**
- C1 itself cannot create obligations. Through inputs typed by a non-owner it can, because `supported` "may be treated as binding" (B1).
- A supervisor can obtain `owner_adopted` by citing its own record (B2).
- Laundering an "unchanged" claim is caught only if the claim is compared against the named source and failed adoptions stay out of the set (correction 2).
- `contradictory` handling ("the parent remains binding") is correct. It needs recorded precedence (M3) and a symmetric rule against narrowing (M6).

**6. Scope.**
- S1 plus a closed vocabulary is appropriately small, provided C1 stays a comparator over typed inputs supplied by owners.
- Drift toward a general engine appears in the open-ended vocabulary extension and in any temptation to have C1 type prose. Keep both out.
- S2 is premature (L2).
- A cheaper intervention on the real route, the owner-per-change table in the escalation disposition (M7), was overlooked and should come first.

## Minimum necessary corrections (targeted plan edits)

1. **§4, new paragraph "Who types the parent".** Only the owner of a parent source may author or accept its typed fields, digest-pinned before the derived assertion. The asserting party, its reviewer, an extractor or a fixture author may propose a typing but cannot create `supported`. With no owner-accepted typing, an `inherited` assertion of an in-vocabulary property resolves to `requires_owner_decision`.
2. **§4 `owner_adopted`, plus §9 Q6.**
   - The requirements set records the owning authority per requirement and per property.
   - An `owner_adopted` record must come from that owner, with authenticated origin. For the operator, that means a session-transcript user entry (session id, entry id, text digest). A board message whose author field is a caller-supplied string does not qualify. An explicit delegation that names the property also qualifies.
   - Supervisor dispositions and general delegations do not qualify.
   - A disposition that fails provenance does not enter the requirements set, and this applies transitively.
3. **§4, new assertion status `realization_conditional`.**
   - The assertion names the selected realization and the supported outcome requirement it protects. It is binding only while that realization is selected.
   - Test: "Would a different realization that removes the named consequence satisfy the finding?" If not, it is strengthening.
   - Split F1 into F1a: 12127e46 P0 under Host `rm`, `realization_conditional`, supported via H3, may block. And F1b: e8304cda fact (2) together with the auto-remove exclusion, plus OPERATOR-DECISION's "unchanged", which return `requires_owner_decision`.
   - Rewrite F3 so it declares `--rm` / `AutoRemove` as `realization_conditional` and is not flagged.
4. **§6 F1.**
   - Give a per-boundary expectation table (document, cited source, disposition) consistent with §4. Fix the F1/F4 contradiction.
   - Judge attribution against 043 and absence against PLAN.md.
   - Author the typing blind to the outcome (withhold the operator turn, as P7 does).
   - Add variants in which the supervisor (i) types 043 itself and (ii) cites the amendment as its owner decision. Neither may yield `supported`.
5. **§5 C1 exit codes.**
   - The receipt lists every assertion the document treats as binding.
   - Exit 0 only if every one of them is `supported` or `realization_conditional`.
   - Proposals are listed as not adopted.
   - `ambiguous_parent` and `contradictory` return 21.
6. **§5 C2 enforcement point.**
   - Replace `RemediationSelection` with the builder-evaluation disposition (`valid` with a provenance basis) and the implementation-review verdict rule.
   - Note that (b), the sidecar, cannot reach the pure verdict rule without changing `record_result`'s inputs.
   - Consider reusing the existing `evidence_unestablished` / route-to-owner phase.
7. **§5 C1 inputs and §9 Q3.** State that `OrganizationalRequirements` v0 types review conduct only and that `slice-campaign` does not depend on it. Separate subject-level and review-level properties into distinct namespaces.
8. **§5 C3 and §7.**
   - Make the first manual slice an instruction-level change to architectural escalation and the reviewer reference, reviewed under instruction design.
   - Architectural escalation: a required per-change table (condition | quoted parent span | owner | decision or `pending`), with dependent repair held while any row is `pending`.
   - Reviewer reference: quote spans for inherited claims, label interpretations, and never discourage the owner question.
   - State that the manual route's protection is detective or instruction-level. M2 becomes meaningful only once corrections 1 and 2 make the receipt's decisive inputs originate with the owner, so a builder can re-run C1 against them. M3 would need a hook in the Pi subagent host.
9. **§4 consequences.**
   - Owner-accepted amendments stay binding until superseded; C1 reports the defect but does not demote them.
   - Amendments adopted by a non-owner are routed to the owner and hold dependent repair; they are not treated as optional.
   - Add fixture F10: an actor narrows a binding requirement → `contradictory`, and the parent stays binding.
   - Say explicitly that this engineering safeguard enforces, unlike the advisory pilot check.
10. **§1 item 3.** Attribute the attribution reading to 12127e46, and the §3.9 citation and auto-remove exclusion to e8304cda.
11. **§7.** Make S2 conditional on evidence from P7 or a typed-route incident.

## What I could not verify

- The contents of board messages #6173, #6176, #6184 and #6185. I relied on the research note's description.
- RL's own reading of 043, beyond the files.
- Whether any live campaign currently uses the slice-campaign schema-4 remediation path.
- Whether the Pi `subagent` host has an extension point that could carry an M3 hook.
- Whether any authenticated-origin mechanism exists for RL decisions.
- The re-escalation review's completion time. I have only its dispatch at 08:52Z and the repair dispatch at 08:59Z from the session record; the 01:58 MST file time comes from the research note.
- Nothing was compiled or tested. All source findings are from reading, and no build was needed for them.

## Commands run

All read-only. Results are summarized inline.

- `git rev-parse HEAD`, `git branch --show-current` and `git status --short` matched the expected commit and branch. The tree has unrelated untracked files.
- `sha256sum` on the plan and on the eight packet artifacts: all matched. `wc -l` on the same files.
- `cat -n` on 043, PLAN.md, QUALIFICATION-AMENDMENT, OPERATOR-DECISION, the two DOCKER-AUTOREMOVE files and both ASTRA-ARCHITECTURE reports.
- `grep` for `3.9`, `evidence-and-claims`, `operator authority` and `owner decision` across the packet reports.
- Python (`-I`) scans of the session JSONL, limited to small line ranges: user turns between 14:50Z and 15:59Z; assistant turns at lines 9729–9750; user turns at lines 8000–8900 and 8900–9730; tool-call counts and `subagent` dispatches from 08:50Z to 09:35Z. Two scans were interrupted by the session; I re-ran them with narrower ranges.
- Read `docs/contracts/core.md`, evidence-and-claims §3.9, `docs/rationale/production-path-claims.md`, and selected sections of `instruction-design.md` (§3.1, §3.4–3.8).
- Read research note Case F (lines 122–163) and pilot sections §3 pins, Stage 1c, P7, §14.2, and line 739.
- Read `review-episode-core` `implementation_review_v1.rs` (whole file), `reducer.rs:100–419`, and `state.rs` `validate_admission`.
- Ran `grep`/`sed` on `slice-campaign` `remediation.rs` (1–200, 695–760, 1077–1100, 3438–3637, 4315–4345), `initial_outcome.rs` (20–170, 1255–1310), and `Cargo.toml` dependencies.
- Read `work-engine-organization/src/schema.rs:78–300`, `lib.rs`, and `work-engine-review-preparation/src/lib.rs`, and grepped `prepare.rs`.
- Read `skills/manual-orchestration/references/architectural-escalation.md`, `supervisor-packets.md` and `worker.md`.
- Ran `grep`/`sed` on `handoff_validator.py`, `role_host.py` and `skills/durable-state/scripts/codex_chatboard.py` (author fields).
- No Cargo builds, no tests, no network, no repository edits. The only file written is this review artifact.
