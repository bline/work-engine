# C0 instruction review: obligation-provenance safeguard plan, revision 3, §5 A–C

**Verdict: `accept_with_corrections`.** This holds only if the corrected A, B and C below (or equivalent wording) replace §5 and the §6.2 walk-through is rerun against them. If the owner rule (F1) or the change test (F2) is not adopted, the verdict becomes `revise_and_resubmit`, because the wording as proposed does not reliably produce §6.2's outcome on the U4-A route.

## Subject and applicability

- Repository `/home/bline/code/work-engine`, branch `rescue-detached-20260911`, HEAD `ae91cd1a5d9efc4a65e0c90da4c97ff829bea185`. Verified with `git rev-parse HEAD`.
- Plan `planning/obligation-provenance-safeguard-plan-20261123.md`, SHA-256 `bba3b025…1dead`. Verified. Subject: §5 A (line 66), B (line 70), C (line 74 plus the trailing sentence on line 76). §4 rules 1–8 are their intent and §6 their acceptance.
- Target files, all verified: `architectural-escalation.md` `0d8596e0…`, `worker.md` `bbe5985f…`, `supervisor-acceptance.md` `c615f787…`. Doctrine read: `instruction-design.md` `51161ccf…` (verified), `core.md` `359bf729…`, `evidence-and-claims.md` §3.9 and §12, `context-and-review.md`. Review method: `skills/agent-instruction-review/SKILL.md` `ab652b61…` and `references/finding-contract.md`.
- **Applicability: `applicable`.** The proposal contains exact normative text for runtime skill references, read by supervisors (A, C) and by workers and reviewers (B).
- This review is advisory. It does not edit the subject, approve implementation or release skills. The operator remains the decision owner (plan line 5).

## Correct (evidence)

- **The doctrine is not missing, and the references are the right owners.** Core §1.1 says "If an invariant must change, that is a contract change. It requires whatever authority the owning contract requires." Core §11 names "ownership choice, or approval" as decisions that need authority. Evidence-and-claims §3.9 says narrowing "requires the authority that owns the affected acceptance condition". A–C apply these to supervisor dispositions and reviewer findings, which instruction-design §2.3 allows: a runtime instruction "may specialize how an existing contract applies to a role". No doctrine edit is needed for C0 to be valid. (One observation for the doctrine owner, not a finding: §3.9 and §12 name only *narrowing*; strengthening is covered only by core §1.1.)
- **Nothing in A–C conflicts with core §4 or §10.** The hold is per row, and evidence-and-claims §3.9 says an unmet claim "does not independently halt unrelated work". Holding a change until its owner decides is ordering that crosses an authority boundary, so it is structural under instruction-design §3.5.
- **The existing text does not already cover this.** `worker.md:3` ("Report a discovered requirement gap, stale premise or ownership conflict to the supervisor…") stops silent scope expansion. It does not separate inherited requirements from interpreted ones, or forbid discouraging an owner question. `architectural-escalation.md:8` and `:65–66` ("the authorized owner's disposition") name one design owner, and in U4-A the coordinator filled that role itself: `QUALIFICATION-AMENDMENT.md:3` says "under existing operator authority and the overnight escalation request". What is new in A is the distinction between the design owner and the owner of each condition's meaning. I found no existing text on parent spans, "unchanged", or owner questions in `skills/manual-orchestration/**/*.md` or `skills/slice-supervisor/SKILL.md` (grep).
- **Each insertion is loaded when its rule applies.**
  - A: `supervisor-dispatch.md:17–20` loads `architectural-escalation.md` "when collecting implementation reviews and before dispatching repairs". `RESOLVED-OBJECT-ESCALATION.md:5` told the U4-A reviewer to read it. Both U4-A amendment points followed an escalation review.
  - B: the U4-A resolved-object reviewer "Used manual-orchestration reviewer and escalation references" (`resolved-object-review/ASTRA-ARCHITECTURE.md:62`, `e8304cda…`), so B reaches that route.
  - C: `supervisor-acceptance.md` loads when "stopping or reporting". `OPERATOR-DECISION.md:1` was a park ("U4-A parked — exact operator design decision required"), and `supervisor-acceptance.md:10` already asks for "the next owner or the decision needed".
- **The dated incident sentence is acceptable in a reference.** Short dated incidents serve as causal reasons elsewhere in these files: `supervisor-acceptance.md:8` ("on 2026-10-22 one did…") and the S10 example in `worker.md`. Instruction-design §3.2 bars situational facts *as commands*, not as the failure mode a command explains.
- **The most effective sentence is the "unchanged" rule.** `OPERATOR-DECISION.md:5` reads "The H1–H5 requirements are unchanged: … Host normal removal must be affirmative and attributable". Requiring the parent quote beside that paraphrase would have put the mismatch in front of the operator even if the author sincerely believed nothing had changed. H3 in `review-r2/PLAN.md:15` only says "distinguish confirmed removal, removal failure and unknown cleanup".

## Findings

### F1 (P1): the owner rule leaves a sincere U4-A coordinator free to name itself, so §6.2 does not follow

- **Sentence.** A: "Take the owner from the source that governs the condition, not from who wrote the disposition." §6.2 claims: "A coordinator naming itself as owner violates rule 2, because 041 names RL."
- **Evidence (verified by reading).** At least three sources hand dev-vertical (the coordinator) a citable ownership claim, none of them the disposition's author:
  - `review-r2/PLAN.md:32` (`c7511ad7…`, the plan the amendment retained): "Dev-vertical owns coordination, result collection and the separate acceptance decision". Its H3 row (line 15) lists "Host owner" in the owner column.
  - `041:15` (`c1c6090c…`): "dev-vertical owns the actual Pi Host controller and rendered container boundary". The changed condition, a "positively attained normal-removal witness" of Host cleanup, is plausibly Host-controller evidence.
  - The reviewer, `12127e46:55`: "dev-vertical records its disposition … and freezes the amended acceptance rule". And `12127e46:69`: "Dev-vertical owns the local Host amendment and offline disposition. RL owns any change in clean-close or consumer meaning". The reviewer had already classified the strengthening as *not* a change in clean-close meaning: line 13 says narrowing to eventual absence "would require an owner decision".

  A only excludes the disposition's author. It does not rank a plan that defers (PLAN.md:3: "The latter documents own the broader qualification contract") below the source it defers to. It does not exclude a reviewer's ownership statement. And it gives no rule when the governing source splits ownership, as 041:15 does. The plan parks review-r2's N7 ("a subordinate source cannot settle what its governing source leaves open") as a comparator precondition (§7), but C0's owner rule depends on it.
- **Inference.** Under A as written, a coordinator who sincerely saw the change as Host-boundary evidence could plausibly record owner "dev-vertical (041:15; PLAN.md:32)" with this disposition as the decision, and no `pending` row. That is a counterfactual and cannot be verified.
- **Causal exposure.** The incident sentence gives the failure mode (a general delegation, the owner never asked) but not the mechanism (self-attribution through a split or deferring source).
- **Outcome: restate.**
  - Rank the governing source above any plan that defers to it and above reviewer statements.
  - When ownership is split or unclear, record the other party with `pending`. Settling the split in your own favour is itself the ownership choice core §11 reserves.
  - Restate §6.2 as conditional on this wording, and test it against PLAN.md:32, 041:15 and 12127e46:69.

### F2 (P1): the trigger depends on how the author classifies its own change

- **Sentence.** A: "When that decision changes or reinterprets an acceptance condition…"
- **Evidence (verified by reading).**
  - The disposition presented itself as preserving the requirements. `QUALIFICATION-AMENDMENT.md:3` ("Retain the exact cleared `review-r2/PLAN.md` … and H1–H5 / Host, fixture and RL ownership") followed `12127e46:3` ("retain H1–H5 and the Host/fixture/RL ownership split; require a local amendment to the qualification contract").
  - The changed text is headed "Amended qualification rule" (`QUALIFICATION-AMENDMENT.md:5`), not "acceptance condition".
  - `043:19` requires only "exact container removal observed against the owned identity". The amendment (`:7`) requires a "positively attained normal-removal witness … distinguishing normal removal from already absent".
- **Inference.** An author who believes the amendment only evidences H3 can sincerely decide it neither changes nor reinterprets anything, and write no row.
- **Outcome: restate.**
  - Give an objective test: a change is anything the quoted governing span would not by itself require or allow, including the author's reading of ambiguous text.
  - Name qualification rules explicitly.
  - Allow `none` as the parent span for a wholly new condition. The research note's Case F says "Exclusive causal attribution … has no parent span", so the column must not become an escape.
  - Friction note: the test also turns self-owned additions into rows (owner: self, decision: this disposition). That is the intended audit trail, not a hold.

### F3 (P1): C's baseline is undefined, it carries no reason, and it depends on a link for its row format

- **Sentence.** C: "…present every changed or reinterpreted acceptance condition as a change row, as in architectural escalation, never as unchanged."
- **Evidence (verified by reading).** `OPERATOR-DECISION.md:5`'s "unchanged" was literally true relative to the coordinator's own `QUALIFICATION-AMENDMENT.md:7`, which the decision request treated as the baseline (`OPERATOR-DECISION.md:3`, and `e8304cda:9`: "The owning rule is RL `043` …, cleared `review-r2/PLAN.md` H3–H5 … and `QUALIFICATION-AMENDMENT.md` §Amended qualification rule").
- **Inference.** C does not say what "changed" is measured against, so the U4-A request could have passed C as worded.
- **Causal exposure.** C has no reason at `supervisor-acceptance.md`'s loading boundary. That contradicts the plan's own line 62 and instruction-design §3.1. C also depends on `architectural-escalation.md` for the row's fields, which may not be loaded when stopping or reporting.
- **Outcome: restate.**
  - Measure "changed" against what the owner last decided, not against the supervisor's or a reviewer's own disposition.
  - Name the row fields inline.
  - Add the failure mode.

### F4 (P2): the decision-reference sentence admits a general delegation as a decision

- **Sentence.** A: "A decision reference may be the owner's own message, or its board post with session id."
- **Evidence.** The amendment acted "under existing operator authority and the overnight escalation request" (`QUALIFICATION-AMENDMENT.md:3`, verified). The 08:11Z text is quoted in the research note ("try to continue; use escalation rules and architectural reviews to help make decisions"); I did not verify it in the session file. Any message from the owner qualifies, including one that never addresses this change.
- **Outcome: restate.** Accept the owner's message or board post (with session id) only if it decides the change or expressly delegates it. This keeps the operator free to delegate explicitly, so it does not narrow operator authority under core §11.

### F5 (P2): the "unchanged" sentence presupposes rows for unchanged requirements and is too broad

- **Sentence.** A: "Call a requirement unchanged only when its row quotes the unchanged parent span".
- **Evidence (verified by reading).** The table lists "each change", so an unchanged requirement has no "its row". Read broadly, every "retain plan X by digest", such as `QUALIFICATION-AMENDMENT.md:3`, would require verbatim quotes of all of H1–H5. The failure actually observed was a *paraphrase* labelled unchanged (`OPERATOR-DECISION.md:5`).
- **Outcome: restate.** Target requirements that are restated or paraphrased as unchanged, and require the parent quote beside them. A table is not needed for this.

### F6 (P2): the hold binds in one direction only

- **Sentence.** A: "Until the owner decides, do not treat a `pending` row as an acceptance condition or as required repair."
- **Evidence.** The plan's non-goals (§3) include "Suppressing findings or concerns, including proposed new obligations".
- **Inference.** For a row that reinterprets ambiguous parent text, the pending meaning is unresolved in both directions. As worded, a supervisor could accept on the narrower reading and let the row disappear. `supervisor-acceptance.md:8` ("Record open obligations") covers this only indirectly.
- **Outcome: restate.** Until the owner decides, a pending row is neither required repair nor resolved.

### F7 (P2): the realization-choice exemption duplicates existing text and opens a labelling escape

- **Sentence.** A: "Other rows, unrelated work, diagnosis, design review and realization choices you make and label as your own continue."
- **Evidence (verified by reading).** "Unrelated work, diagnosis, design review" repeats `architectural-escalation.md:44–46`. A realization choice is not a change to an acceptance condition, so it never produces a row. The exemption is redundant for the hold but invites an owner requirement to be relabelled as "my realization" and then presented to others as required. In U4-A the requirement began as realization-scoped ("For this realization, yes…", `12127e46:11`) and became an "independently required path claim" (`e8304cda:9`).
- **Outcome: remove** the duplicate clauses. **Restate** the exemption as a definition: a choice you present as yours to revise is not a change, and you must not present it to others as a requirement.

### F8 (P2): the insertion point breaks the antecedent of the next sentence

- **Evidence (verified by reading).** A goes between `architectural-escalation.md:65–66` and `:67` ("Bind that decision to the history it considered…"). After roughly 190 inserted words ending in owner decisions, "that decision" on line 67 no longer clearly means the design decision. A's own opening, "When that decision…", depends on the preceding sentence.
- **Outcome: move.** Make A self-contained and place it as its own paragraph after line 72 ("…escalates again."), the end of "Review and release of the hold".

### F9 (P2): B's trigger is narrower than how the U4-A reviewers actually spoke, and its "Never" is over-broad

- **Sentences.** B: "quote the exact parent span for any requirement you call inherited…" and "Never advise against asking the owner whether a requirement applies: your reading becomes binding only through the owner's decision."
- **Evidence (verified by reading).**
  - The reviewers did not call the requirement "inherited". They called it "required" (`12127e46:11`), an "independently required path claim (§3.9)", and named the amendment itself as part of "The owning rule" (`e8304cda:9`).
  - `12127e46:9` did cite `043:11,17–25` with line numbers, so a quote clause alone would have been satisfied by quoting a span that does not state the requirement.
  - The "Never" sentence matches `e8304cda` exactly ("Do not ask for permission to accept eventual absence instead"; Case F step 3). But it also forbids pointing out that an owner has already decided the question, and its reason is wrong when a requirement is stated verbatim in an accepted contract: such a requirement binds through that contract, not through a new owner decision.
- **Outcome: restate.**
  - Trigger on any requirement the reviewer presents as required by a source.
  - Have the reviewer label anything beyond what the quoted span states as a proposed change for that condition's owner.
  - Allow citing an existing owner decision as the one exception to the owner-question rule.
- **Placement.** Retain B in `worker.md`. The "As a reviewer" prefix scopes it, and workers lose only a few lines.

### F10 (P2): A's reach is wider than the plan states, in one direction and narrower in another

- **Evidence (verified by reading).**
  - Wider: `architectural-escalation.md:3–4` describes itself as "shared by manual orchestration and configured slice supervision", and `skills/slice-supervisor/SKILL.md:39` loads it. A would bind configured campaigns, but plan §1 and §2 say "manual route", and delivery there would go through configured adapters (`slice-supervisor/SKILL.md:39–43`: "Use configured specialists and adapters").
  - Narrower: as worded ("that decision"), A covers only post-design-review dispositions, while §2's objective is "every change to an acceptance condition".
- **Outcome: restate scope in the plan.** Either state that A applies to both routes and to every disposition this file governs (the self-contained corrected A below is read at every repair dispatch via `supervisor-dispatch.md:17`), or narrow §2 to escalation dispositions.

### F11 (P2): the trailing approval sentence in C has uncertain status and would overreach if inserted

- **Sentence.** `plan:76`, outside the blockquote: "An approval adopts only what the request presented as changes." The sentence is not quoted, so whether it would be inserted is ambiguous.
- **Inference.** Read literally, it would deny adoption of anything the owner adds in its own reply, which constrains the operator rather than the supervisor's interpretation (core §11).
- **Outcome: restate**, as in corrected C: assent to a request does not adopt a change the request labelled unchanged.

### F12 (P2): the acceptance examples in §6 are weaker than the rules they test

- **Evidence (verified by reading).**
  - The §6.2 row cites `043 "Terminal cleanup"`, a section heading without lines or digest, so it fails rule 1's own format. The span would be `043-u4a-held-contract.md:19`, `4bf5ee60…`, "exact container removal observed against the owned identity".
  - §6.3 calls create-time `--rm` "the supervisor's realization choice". It was the operator's change, and RL aligned with it (Case F, board #6173/#6176), so it is a weak negative control for supervisor self-labelling.
- **Outcome: restate** the example row. Add a counter-check of a supervisor-labelled realization choice that is later presented as a requirement (F7).

### F13 (P2, limitation): B reaches only reviewers told to load manual-orchestration

- **Evidence.** `SKILL.md` ("Launching a child") instructs children to load the skill. A Pi `reviewer` launched under another skill or prompt, as this review was, does not receive `worker.md`.
- **Outcome: retain.** Record it as a known reach limit. It held for the U4-A route (`e8304cda:62`).

## Minimum corrected text

**A.** Insert into `architectural-escalation.md` as a new paragraph after line 72 ("…escalates again."):

> Changing an acceptance or qualification condition is a contract change that needs its owner's authority (core contracts §1.1). So for each change a disposition makes, record: the condition as changed; the parent span it changes, quoted with path, lines and digest from the source that governs the condition (`none` for a new condition); the owner of the condition's meaning; and that owner's decision or `pending`. A change is anything the quoted span would not by itself require or allow, including your reading of ambiguous text. A realization choice you present as yours to revise is not a change, but do not present it to others as a requirement. Take the owner from the governing source, not from the disposition's author, a reviewer, or a plan that defers to that source. If the source splits ownership or leaves it unclear, name the other party and record `pending`: settling that in your own favour is the decision you lack. On 2026-11-21 a supervisor adopted a reviewer's stronger cleanup requirement under a general delegation, and the requirement's owner was never asked. Send each `pending` row to its owner when you record it. Until the owner decides, the row is neither required repair nor resolved; other rows proceed. A decision is the owner's own message, or its board post with session id, that decides the change or expressly delegates it. Do not call a requirement you restate or paraphrase unchanged unless you quote its parent span beside it: a change described as unchanged hides it from the owner who must decide it.

**B.** Append to the opening paragraph of `worker.md`:

> As a reviewer, quote the governing source's exact span for any requirement you present as required by it. Label a reading of ambiguous text, or anything beyond what the span states, as a proposed change for that condition's owner, not as inherited. If a requirement holds only for the current realization, say so and limit it to the consequence you name, since other realizations may not need it. Do not discourage asking the owner whether a requirement applies unless you cite that owner's decision on it: a requirement its source does not plainly state binds only through its owner's decision.

**C.** Append to the reporting paragraph of "Acceptance and result" in `supervisor-acceptance.md` (line 10):

> When a decision request to the user or another owner restates requirements, show each changed or reinterpreted acceptance condition as a change, with its quoted parent span, owner, and decision or `pending`, as in [architectural escalation](architectural-escalation.md). Judge "changed" against what the owner last decided, not against your own or a reviewer's disposition. An owner shown a change labelled unchanged cannot decide it, and assent to that request does not adopt it.

Drop the trailing sentence on plan line 76. Corrected C replaces it.

**Expected U4-A behaviour under the corrected text (inference, not demonstrated):**
- 08:59Z: `043:19` does not by itself require a positive witness that distinguishes normal removal from "already absent", so it is a change. PLAN.md defers to 041/043, so PLAN.md:32 cannot settle ownership, and neither can the reviewer at `12127e46:69`. 041:15 splits the Host controller from receipt semantics, so the row is `pending` and goes to RL.
- 09:43Z: "changed" is measured against 043 and PLAN.md, not the amendment, so the operator sees a change row.

This remains instruction-level. A supervisor could still deny that the witness goes beyond `043:19`, and nothing mechanical prevents that (§2).

## Not verified

- No session transcript was read. The 08:11Z delegation text, the 08:59:31Z dispatch time and the 09:43Z message are taken from the research note and README spot checks.
- Whether the U4-A coordinator would have behaved differently is counterfactual inference.
- Released skill copies were not compared with the repository copies; the task asserts they are identical.
- Effectiveness on other workstreams is unknown; the claim rests on a single incident.
- I did not establish a context-pressure listener for this short read-only review; context headroom stayed ample.

## Commands run (all read-only)

- `git rev-parse HEAD; git branch --show-current; git status --short`: HEAD `ae91cd1a…`, branch `rescue-detached-20260911`. Unrelated dirty and untracked files were present; none are subject files.
- `sha256sum` of the plan, the three targets, `instruction-design.md`, `core.md`, `SKILL.md` and `agent-instruction-review/SKILL.md`: the plan and all targets match the task's pins.
- `grep -n` over the plan (lines 55–75) and over the targets for line numbers.
- `grep -rn architectural-escalation` across `skills/` and `docs/`: loaded from `supervisor-dispatch.md:17` and `slice-supervisor/SKILL.md:39`.
- `grep -rn -i "parent span|unchanged|inherited requirement|owner question|acceptance condition"` across `skills/manual-orchestration/` and `slice-supervisor`: no existing coverage.
- In `/home/bline/cbm-work/pi-host-u4a-acceptance-20261121/`: `cat -n` of `QUALIFICATION-AMENDMENT.md` (`93e004dc…`), `OPERATOR-DECISION.md` (`d0055826…`), 041 (`c1c6090c…`) and 043 (`4bf5ee60…`). Then `grep`/`sed` over both `ASTRA-ARCHITECTURE.md` files (`12127e46…`, `e8304cda…`), `RESOLVED-OBJECT-ESCALATION.md`, `ARCHITECTURE-REESCALATION.md`, and `review-r2/PLAN.md` (`c7511ad7…`, lines 3, 15, 32).
- Read the review record's README and review-r2 (N5, N6, N7), and research note Case F (lines 122–166).
- `codex_chatboard.py read --limit 5 --active-claims`: startup board read; nothing relevant to this subject.
