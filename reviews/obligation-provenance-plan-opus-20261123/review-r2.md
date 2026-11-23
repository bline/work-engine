# Adversarial re-review: obligation-provenance safeguard plan, revision 2

Reviewer: Opus 5.5 (`claude-opus-5-5`), Claude subscription route. This was a read-only review. I made no edits outside this artifact, ran no builds, used no network and did no live inference. I did not approve any slice or release any skill.

## Subject verification (I ran these checks)

- HEAD is `faf8ff296b2e4894fdd2f5f45ee6ae3fea3425db` on branch `rescue-detached-20260911`. `faf8ff29` touches only the plan and the two design documents.
- Plan SHA-256 is `efa26f9ed1d275804cf3fbe81281067a32f0450e224433bc8ffecba85fb4ae13` (296 lines), which matches.
- I re-checked the packet artifacts: their revision-1 hashes match, and 041 is `c1c6090c…`.

## Verdict: `revise_and_resubmit`

Revision 2 is a real improvement:
- owner-only typing of parents;
- authenticated origin that excludes supervisor dispositions, board posts and general delegations;
- correct typed consumers;
- namespaces separating subject-level from review-level properties;
- honest protection levels per component;
- an instruction-level change first.

Four problems still block acceptance:

1. **`realization_conditional` is a loophole that exits 0, and F1a exercises it.** C1 does not check the conditionality test; the asserting party answers it. The plan's own F1a classifies the re-escalation review's "affirmative result" requirement as realization-conditional. That requirement already carries the causal-attribution seed that drove the incident (N1).
2. **F1's acceptance logic is inconsistent.** It pre-registers the dispositions, yet says the decisive 043 typing will be authored blind. The current authors cannot be blind to it, and a blind typist could read 043 as the reviewer did (N2).
3. **For the owners who actually exist, `owner_adopted` is either impossible or launderable.** RL records decisions as board posts. The operator records decisions as assent to agent-written recommendations (N4).
4. **Coverage by containment points the wrong way for obligations (N3).**

C0 is close to ready. Its remaining corrections, N5 and N6, are small. Whether to proceed with S0 separately is the operator's decision.

## Closure table for revision-1 findings

| Finding | Status | Evidence |
|---|---|---|
| **B1**: decisive judgment unowned; F1 passes by construction | **Partially closed** | Parent typing is now owner-only (§4.1), so that part is closed. Still open: (a) the asserting party declares `realization_conditional`, which exits 0 without any owner involvement (§4.4: "Who answers this test is part of the declared classification … not decided by C1") (N1). (b) F1's "blind" owner typing cannot be done by authors who have read the revision-1 review and Case F, and the expected table is fixed in advance (N2). (c) The coordinator owns `PLAN.md` (PLAN.md:32), so owner typing gives no separation for plan-level requirements (N7). |
| **B2**: `owner_adopted` without owner identity | **Partially closed** | §4.5 excludes supervisor dispositions, board posts and general delegations, and applies exclusion transitively. Still open: assent to a supervisor-authored request (041:5 "I go with your recommendations"; session line 9714 "The H1–H5 requirements are unchanged" sent to the operator). Operator prose does not name properties, so "names the property and value" cannot be checked mechanically. RL authentication is undefined (§9 Q1) (N4). |
| **H1**: no realization-conditional category | **Partially closed** | The category and the F1a/F1b split exist. F1a misstates the review and includes the causality seed (N1). The non-goal saying security defects need no citation still conflicts with in-vocabulary independence statements (N8). |
| **H2**: wrong typed enforcement point | **Closed** | C3 names the builder evaluation (`initial_outcome.rs` `BuilderEvaluationDecision`, all-done `!valid \|\| verified_resolved`) and the verdict rule, and removes `RemediationSelection`. It states the sidecar limitation. Minor wording: the reducer also uses unresolved questions and evidence admissions, not "the verdict alone" (`reducer.rs:283–327`). |
| **H3**: `OrganizationalRequirements` is not a subject parent | **Closed in the plan** | §4.2 adds namespaces; C1 does not depend on it. The design documents still contradict this (N11). |
| **H4**: no mechanical manual consumer | **Partially closed** | C2 and M3 protection levels are stated honestly. C0's U4-A claim ("would have shown owner: RL; decision: pending … holding the repair dispatch") is not established, because the owner column can be filled by its author (N5). Releasing the hold on RL-owned rows is undefined (N6). |
| **M1**: exit 0 included proposals | **Closed, subject to N1** | Exit 0 now requires `supported` or `realization_conditional`, and the latter has no checked anchor. |
| **M2**: F1/F4 contradiction | **Partially closed** | The dispositions are consistent with §4.6. The "narrowing" row cites only 043:11,17–25. The sentence (12127e46:13) has no inline citation; the section's citation (12127e46:9) also names `PLAN.md:15–17,24–27` and `041:13–17`. "`ambiguous`/`unspecified`" give different dispositions (`unspecified` → `requires_owner_decision`). 041 is missing from the F1 setup. |
| **M3**: parent precedence | **Partially closed** | §4.2 adds precedence. It does not stop a subordinate source from resolving the governing source's `unspecified`/`ambiguous` (N7). It does not represent the 041:15 ownership split (RL: receipt semantics; dev-vertical: Host controller and container boundary). |
| **M4**: property-level only | **Partially closed** | Value-level comparison and F9 were added. The containment direction is copied from `RealizationConstraint.allowed`, which runs the wrong way for obligations (N3). No value enumerations are defined. |
| **M5**: self-declared scope | **Partially closed** | Declarations can now be challenged and F11 was added. No trigger, owner, timing or consumer exists for a challenge, so F11 tests that a challenge is recorded, not that one happens. `subject.acceptance_evidence` is broad (N8). C2's builder re-run checks only what was declared. |
| **M6**: adopted amendments, narrowing | **Closed** (with N6) | §4.6 adds no-demotion, `contradictory` for narrowing, F10 and F12. The granularity of the hold is inconsistent with C0 (N6). |
| **M7**: cheaper intervention missed | **Closed in substance** | C0 and S0 come first. The rule for who may be named as owner and the scope of the hold need correcting (N5, N6). |
| **L1**: merged reviews | **Closed** | §1 items 3 and 5 attribute correctly. I verified 12127e46:11,17 and e8304cda:5,9,32. |
| **L2**: typed slice unjustified | **Closed** | S3 is conditional. |

## New findings

### N1 (high): `realization_conditional` exits 0 on the asserting party's word, and F1a lets the costly requirement through

**Plan text:**
- §4.4: "Who answers this test is part of the declared classification (§4.3), open to challenge, not decided by C1. C1 checks that the cited supported requirement exists, that the realization named is the one recorded as selected…"
- C1: exit `0` "only if every binding assertion is `supported` or `realization_conditional`".
- The "selected-realization record" is a C1 input with no stated author.
- F1a: "a removal setup failure must not be reported as confirmed absence; Host removal must be affirmative" → `realization_conditional`.

**Evidence (I read these):**
- The re-escalation review's requirement, 12127e46:13: "require an attributable normal operation … with an affirmative result under that operation's actual semantics … If a purported acknowledgement could equally mean 'already missing,' it cannot by itself establish this distinction."
- On Docker, a successful normal removal can follow concurrent disappearance (e8304cda:17). Distinguishing the Host's removal from "already missing" therefore means excluding concurrent removers. That is causal attribution, the very property that later proved unrealizable.
- The consequence the review named, "a Host whose cleanup did not work" (12127e46:11), is fully addressed by refusing a clean verdict when the Host's removal failed or never started. The plan's test ("would a different realization … satisfy the finding?") checks whether the requirement depends on the realization. It does not check whether the requirement goes beyond what that realization needs.
- F1a's first clause contradicts the review itself: "A failed verdict can truthfully carry `confirmed_absent`" (12127e46:29), and "Require `held_failed + confirmed_absent`" (12127e46:59).

**Why it matters:**
- A strengthening scoped to the current realization passes at exit 0 with no owner involved. The plan's own regression fixture encodes that false negative.
- C1's checks reduce to two things: a supervisor-authored assertion names a supervisor-authored realization record, and it cites some owner-typed requirement. C1 cannot check how the asserted property relates to the requirement it claims to protect.

**Smallest fix:**
- (a) Constrain the schema. A `realization_conditional` value must be the protected requirement's property and value applied to the selected realization's own mechanism, for example that its failure modes are reported inside the protected requirement's value space. It may not name or exclude other realizations or change an acceptance predicate, and it may not be cited as support for anything else.
- (b) Exit 0 for it only with a recorded challenge outcome from a party other than the asserting party. Otherwise C2 lists it as unchallenged and treats it as `21`.
- (c) Correct F1a to: "a Host removal that failed or never started is reported as removal failure, and the run is not `held_cancelled` on independent absence alone (`held_failed + confirmed_absent`)" → `realization_conditional` or F7 against H3. Add F1a′: "an affirmative result that distinguishes Host removal from already-missing" (12127e46:13) → `requires_owner_decision`, or `ambiguous_parent` depending on the typing. F1a′ must never be `realization_conditional`.

### N2 (high): F1 fixes its expected results in advance while claiming the decisive typing will be blind

**Plan text (§6):**
- "Owner typing … authored blind to the outcome: the operator's turn and the auto-remove documents are withheld."
- The table expects "`ambiguous_parent` (owner typed 043 removal attribution `ambiguous`/`unspecified`)".
- "A safeguard that does not produce these results … does not pass acceptance."

**Evidence:**
- Withholding the operator turn does not make the typing blind. The revision-1 review and the corrected research note both state the conclusion ("Exclusive causal attribution … has no parent span"), and the plan's authors have read both.
- A blind typist could read 043 as the Astra reviewer did (12127e46:9–11: "assigns the Host the child and exact stop … yes"). That typing gives `binding Host`, then `supported`, and F1 "fails".
- `unspecified` resolves to `requires_owner_decision` under §4.1 and §4.6, not to `ambiguous_parent`.

**Why it matters:** either the typing is tuned so F1 passes, which is not blind, or acceptance depends on how a stand-in reads ambiguous prose. Neither shows that the safeguard would have caught U4-A. That depends on RL's actual typing, which C1 cannot supply.

**Fix:**
- State F1's expectations as a function of the stand-in typing: `binding Host`, `ambiguous` and `unspecified`.
- Acceptance then means: for each typing, C1 returns the §4-correct disposition; F1-v1 and F1-v2 never return `supported`; F1a′ is never `realization_conditional`.
- If a blind reading is wanted as evidence, give it to P7 with an author who has not seen Case F, either review or this plan.

### N3 (medium-high): value containment runs the wrong way for obligations

**Plan text (§4.4):** "compared by equality or explicit containment, as `RealizationConstraint.allowed` already does."

**Evidence:** `RealizationConstraint { dimension, allowed }` (`work-engine-organization/src/schema.rs:251–257`) restricts a realization to the allowed classes: a realization satisfies it when its class is a member of `allowed`. Applied to an `inherited` obligation, that membership test would treat "`outcome_owner = Host`" as covered by a parent that allows `{Host, Docker}`. Yet the assertion excludes Docker, which is exactly the auto-remove exclusion (e8304cda:32).

**Fix:**
- An `inherited` assertion is covered only if the set it permits is a superset of the parent's binding permitted set, so it is no stricter than the parent.
- Membership applies only when checking a *selected realization* against a parent.
- Define per-property value enumerations in S1. F9 needs "Host exclusively causal" to be a distinct value.

### N4 (high): `owner_adopted` cannot be reached by RL and can be laundered through operator assent

**Evidence (I read these):**
- The operator records decisions as assent to recommendations written by agents. 041:5 reads "Operator, 2026-11-20 MST: 'I go with your recommendations' in response to `/tmp/decisions.md`". Session line 8013 is "proceed". Line 8261 is "Great, let's do that now".
- At 09:43:22Z the coordinator told the operator "The H1–H5 requirements are unchanged" (session line 9714). An operator "yes" to that request would be an authenticated user entry adopting a mislabelled change.
- §4.5 requires the record to name "property and value". Operator prose does not name either. The asserting party maps prose to property and value, and C1 can verify only the entry id and digest.
- RL records decisions as board posts (#6173 request, #6176 alignment), which §4.5 excludes.
- RL is the `pi-research-lifecycle` session `01a1e56d-7ceb-71fc-9c46-c9fcd9618234`. It replied only after the operator resumed it ("ok, I just resumed that session", 15:35:19Z, line 9798).

**Why it matters:** no RL decision can currently reach `owner_adopted`. Operator decisions qualify only through the supervisor's mapping, which reopens B2 through assent. With no owner-typed sets either, every in-vocabulary binding assertion resolves to `requires_owner_decision`, and `realization_conditional` is unavailable too because it needs a binding owner-typed requirement. On real documents the mechanism is "always 21" until the operator changes how they work. S2's gate acknowledges this; the plan should state it.

**Fix:**
- An operator `owner_adopted` record is either (i) the operator's own text naming the change, or (ii) assent that names the digest of a typed decision record. In that record every change must appear as a change row (as in C0), never as "unchanged", and the record must contain no `requires_owner_decision` rows.
- Generic assent adopts nothing that was not presented as a change.
- Consider an authentication root for RL: the operator binds the RL session identity once, in an authenticated user entry.
- State the consequence for RL-owned workstreams until §9 Q1 is answered.

### N5 (medium): anyone can fill C0's owner column, so the U4-A walk-through does not follow

**Plan text (C0):** "Owner of that condition", with no rule on who may be named; and "Applied to U4-A, the table would have shown 'owner: RL; decision: `pending`' at 08:59Z."

**Evidence:**
- 12127e46:69: "Dev-vertical owns the local Host amendment."
- 041:15: "RL owns task-host and lifecycle admission/receipt semantics; dev-vertical owns the actual Pi Host controller and rendered container boundary."
- The amendment was written 62 seconds after the review (file times 01:58:10 and 01:59:12 MST), and dispatch followed at 01:59:31 (08:59:31Z, session line 9350).
- A coordinator who sincerely saw the change as Host-boundary evidence could name itself (citing 041:15) and record no `pending` row.
- C0 covers escalation dispositions and reviewer references only. It does not cover decision requests to the operator (OPERATOR-DECISION.md; session line 9714), which §1 names as a point of failure.

**Fix:**
- The owner is whoever the governing source names for the *meaning* of that acceptance condition. Here that is the `held_cancelled` receipt semantics, owned by RL (041:15, 043). It is never the disposition's author unless the source names it.
- A `pending` row is delivered to its owner when it is created.
- The table and the "unchanged" rule also apply to decision requests.
- State the U4-A walk-through as conditional on these rules.

### N6 (medium-high): the hold's scope, how it is released and the cost claim

**Evidence:**
- §4.6 holds "repair or acceptance that depends on it", meaning the whole amendment. C0 holds work that "depends on a `pending` row".
- QUALIFICATION-AMENDMENT bundles independent repairs (durable evidence, the fixture caller contract; 12127e46:31–51) with the attribution rule. A per-amendment hold stops all of them.
- If C0's "owner decision reference" must meet §4.5, RL rows can never be released. Only the operator could release them, which makes the operator a bottleneck and in effect bypasses RL's ownership.
- On cost, the session shows causality work running from about 08:58Z to 09:43Z. From 09:43Z (line 9714) U4-A was parked awaiting the operator until 15:07Z, and RL was not available until the operator resumed it at 15:35Z. A C0 hold would also have waited for the operator. Its benefit is roughly 45 minutes of engineering and a properly framed owner question, not the "6.5 hours … went to proving exclusive cleanup causality" in §1 item 6.
- Doctrine: no direct conflict. `architectural-escalation.md:44–47` already holds ordinary repair pending "the authorized owner's disposition". §3.9 says an unmet claim "does not independently halt unrelated work". Core §10 allows waiting where authority requires it.
- Deadlock: I found no structural deadlock, because diagnosis and design review continue under escalation (:44–46). Owner unavailability does cause stalls, as above.

**Fix:**
- Hold per row only.
- The hold covers treating a row as an acceptance condition or as required repair. It does not cover realization work that the model chooses for itself and presents as such (core §4).
- C0 decision references are instruction-level and may cite the owner's board post with its session id, which is auditable. §4.5 authentication applies to C1 and C2.
- Correct §1 item 6 and §8 using the measured park.

### N7 (medium): a subordinate source can settle what its governing source left open

**Evidence:**
- §4.2: "where the two differ, the owning source governs."
- PLAN.md:32: "Dev-vertical owns coordination, result collection and the separate acceptance decision." PLAN.md:3 defers to 041 and 043.
- If 043 is `unspecified` and the coordinator, as owner, types H3 as `outcome_owner: binding Host`, nothing defines whether those "differ". An inherited assertion citing PLAN.md would then be `supported`.

**Fix:**
- A subordinate source may not make binding a property that its governing source types `unspecified` or `ambiguous`; only the governing owner can resolve it.
- Add 041 to the F1 setup. It is a governing source, it is cited at 12127e46:9, and it records the ownership split at 041:15.

### N8 (medium): `subject.acceptance_evidence` is too broad, and the security non-goal is still contradicted

**Evidence:**
- §3: "A panic, race or security defect needs no contractual citation."
- §4.3: only statements *outside* the vocabulary are ordinary findings.
- Most qualification findings state required evidence content, for example the caller-contract rows at 12127e46:45–51 on digest verification and response finality. They now fall inside the vocabulary.
- "The verifier must be independent" falls under `subject.independence`.
- Without owner typing (N4), such findings resolve to `requires_owner_decision` and are held.

**Fix:**
- An in-vocabulary statement is an ordinary defect when it names a concrete consequence on the selected realization, and any realization that removes the consequence satisfies it without changing an acceptance predicate.
- Narrow `acceptance_evidence` to changes in an acceptance predicate's required evidence.

### N9 (low): mechanics

- "Typing older than the assertion" cannot be proven without an ordering source. Prove it by requiring the assertion to cite the requirements-set digest.
- "Withdrawn when the selection changes" is an obligation on the document; C1 can only detect a mismatch.
- "Supported requirement" mixes terms: requirements are `binding`, while assertions are `supported`.
- The consequence of `ambiguous_parent` for dependent work is not stated.

### N10 (low): the scope of S1

- S1 builds the comparator before its inputs exist. No owner-typed sets exist, RL authentication is undefined, and §9 Q1 and Q3 are open.
- Make S1 explicitly fixture-only, or gate it on resolving Q1 and Q3 for one real workstream.
- Otherwise the growth is driven by the findings: F1–F12 plus variants, the `acceptance_evidence` property, and C0–C3. It does not drift toward a general engine, provided the value enumerations stay small and C1 never types prose.

### N11 (low-medium): accuracy of the design-document corrections (`git diff 14a248cc faf8ff29 -- design/`)

**Accurate (checked against the packet):**
- the quotes from 12127e46:11, :17 and :69, and from e8304cda:5, :9 and :32;
- OPERATOR-DECISION:5;
- the file times 01:58:10 and 02:41:36 MST;
- dispatch at 08:52Z and 08:59Z (session lines 9319, 9350, 9353);
- the overnight delegation text (line 8873);
- the agent's 15:14Z answer (line 9739);
- moving the "Host normal removal must be affirmative and attributable" quote off the resolved-object review. It belongs to OPERATOR-DECISION:5, and the correction gets that right.

**Inaccurate or incomplete:**
- (a) The heading "Review, in two steps" introduces three items, and item 2 is the coordinator's adoption, not a review.
- (b) "That delegation covered realization decisions, not the acceptance meaning RL owns" is stated as fact. It is an interpretation, because the 08:11Z text sets no scope.
- (c) The note quotes the 15:14Z answer "H4 currently asks for proof that the Host performed cleanup" without flagging that this misstates PLAN.md H4, which asks for independent absence. Like the 09:43Z "The H1–H5 requirements are unchanged" (line 9714), it carried the "unchanged" laundering into the conversation with the operator.
- (d) "Legitimate under H3", in the note and in the pilot P7 row, gives the review too much credit. 12127e46:13's affirmative-result requirement goes beyond the consequence it named (N1).
- (e) Not corrected: pilot Stage 1c (line 473) and §14.2 (lines 771–777) still say `OrganizationalRequirements` Condition fields (independence, evidence custody, observer class) "compare directly", which contradicts plan §4.2. Their disposition lists also lack `realization_conditional`. P7 "may evaluate this safeguard" and would inherit the contradiction.

## Specific questions

- **Owner-only typing and authenticated origin: can they be realized?** Not with the decision practices that exist today (N4). The mechanism is "always 21" until owners publish typed sets and authenticate. S2's gate covers this, but the plan should say so, and S1 should be fixture-only or gated (N10).
- **Is `realization_conditional` a loophole?** Yes. It exits 0, the asserting party answers its test, and F1a exercises it (N1).
- **Can the hold deadlock?** I found no structural deadlock and no conflict with core §10, evidence-and-claims §3.9 or `architectural-escalation.md:44–47`. It can stall when the owner is unavailable, and that happened in U4-A anyway. Releasing RL rows is undefined, and the hold's scope is inconsistent between §4.6 and C0 (N6).
- **Is F1 accurate?** The "narrowing" sentence does not cite 043 inline. The section cites 043, PLAN.md and 041 (12127e46:9). F1a's first clause contradicts 12127e46:29 and :59. H3 supports only "report removal failure distinctly and do not certify a clean cancel over it". It does not support "Host removal must be affirmative" in 12127e46:13's sense (N1, M2).
- **Is C0 the smallest change, and does it fit instruction-design?** It is small. The reason behind each rule is present (§3.1, owner authority under §3.9), it does not prescribe a method (§3.4), and its sequencing is structural because adoption crosses an authority boundary (§3.5). It needs the owner rule and per-row scope (N5, N6), and should also cover decision requests. The plan explains its rules, but the released wording must keep each reason next to its command.

## Minimum remaining corrections

1. Constrain `realization_conditional` and require a challenge before it can exit 0. Fix F1a and add F1a′ (N1).
2. State F1's expectations per stand-in typing, with acceptance defined over all of them. Drop the claim of blind typing, or give that reading to P7 (N2).
3. Set the containment direction (superset for `inherited`, membership for realizations) and define the value enumerations (N3).
4. Operator `owner_adopted` comes from the operator's own text or from assent naming the digest of a typed decision record that has change rows. Generic assent adopts nothing presented as unchanged. State the gap for RL (N4).
5. In C0, the owner comes from the governing source and is never the disposition's author unless named. Deliver `pending` rows to their owners. Cover decision requests (N5).
6. Hold per row. Exempt realization work the model chooses and labels as such. C0 references are instruction-level. Correct the 6.5-hour cost claim (N6).
7. Add the rule that a subordinate source cannot settle what its governing source leaves open, and add 041 to F1 (N7).
8. Define the boundary for in-vocabulary defects and narrow `acceptance_evidence` (N8).
9. Make S1 fixture-only or gate it on §9 Q1 and Q3 (N10).
10. Design documents: fix (a)–(e) in N11.

## What I could not verify

- The content of board posts #6173, #6176, #6184 and #6185, and whether RL was contacted between 08:58Z and 08:59Z. My "did not consult RL" rests on the 62-second gap and RL being unavailable until 15:35Z.
- Whether any hook exists in the Pi `subagent` host for M3.
- How RL could authenticate decisions, beyond board posts.
- Whether other operator workstreams record property-level decisions. I sampled only this session and 041:5.
- No code was built or run. Every source claim comes from reading.

## Commands run

All were read-only. Results are inline above.

- `git rev-parse HEAD`, `git branch --show-current`, `git log --oneline -5`, `git diff --stat 14a248cc faf8ff29`, `git show --stat faf8ff29`, and `git diff 14a248cc faf8ff29 -- design/`.
- `sha256sum` on the plan, on the eight revision-1 packet artifacts (all matched again this run), on `review-r1/041-u4-protection-qualification-PLAN.md` (`c1c6090c…`) and on `review-r2/PLAN.md` (`c7511ad7…`). `TZ=America/Phoenix stat -c '%y'` on the six packet files.
- `read` of the full plan. `cat -n` of 041.
- `grep`/`sed` of the pilot (lines 473, 739 and 771–777) and the research note (line 295) for `OrganizationalRequirements` and `realization_conditional`.
- Bounded `python3 -I` scans of session `01a1e2a4…jsonl`: lines 9600–9730 (tool-call counts by hour and `subagent` dispatches), lines 9700–9729 (assistant text at 09:43Z and 10:38Z) and lines 9790–9800 (RL session id and the 15:35Z resume).
- Packet artifacts and source seams read in revision 1 (12127e46, e8304cda, QUALIFICATION-AMENDMENT, OPERATOR-DECISION, 043, PLAN.md, `schema.rs`, `reducer.rs`, `initial_outcome.rs`, `remediation.rs`, `architectural-escalation.md`, `codex_chatboard.py`) were reused from that verified reading; their hashes were unchanged.
- No Cargo, no tests, no network, and no writes except this artifact.
