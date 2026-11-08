# Knowledge Oracle S0: result (2026-11-08)

**Status:** accepted by the user on 2026-11-08 as a completed feasibility
study. An independent session reviewed the results and returned **"acceptable
with the listed corrections"**; all of them are applied below. S0 is a
feasibility experiment for the exploratory Knowledge Oracle
(`design/ideas/pending/knowledge-oracle-question-resolution-adapter.md` at
`14031bc2`, §12). It is not production qualification.

**Accepted result (user decision, 2026-11-08):**

| Arm | Correct / partial / incorrect | False confident reuse |
|---|---|---|
| A, unassisted | 8 / 2 / 2 | 1 |
| B, existing claims readers | 8 / 3 / 1 | 1 |
| C, curated projection | 12 / 0 / 0 by the primary scorer; the second scorer rated C/M1 partial | 0 |

- **M5 adjudication.** The user adopted the independent reviewer's correction:
  B/M5 counts as false confident reuse and is incorrect rather than partial.
  The primary scorer's original score and the reviewer-adjudicated score are
  kept as separate records. The frozen score files are unchanged; the decision
  is a new record, `private/scores/adjudication-user-20261108.json` (section
  13).
- **What the result means.** The pre-registered result remains "proceed to S1",
  met by arm B with zero safety margin. It is a fragile feasibility result. It
  does not establish a general cost advantage, improved safety, or production
  readiness.
- **What it does not authorize.** No S1 implementation, live claims
  publication, deployed skill change or push.

**Protocol:**

- approval packet revision 2: `planning/knowledge-oracle-s0-experiment-20261108.md`;
- frozen answer key, sha256 `1f8f3e6a75446d4668bd8274d37c95500326544be273bb0a3a5f57fa4c784054`;
- stage 2 rules, registered before any stage 2 run or score, sha256 `82aaccd4…`.

**Data (off-repo, under `~/cbm-work/oracle-s0-20261108/`):**

- sealed outputs: `sealed/`;
- scores: `private/scores/`;
- analysis: `results/analysis.json`;
- the reviewed draft: `results/s0-results.md`;
- the review: `results/interpretation-review.md`.

**Run setup:** `claude-sonnet-5-5` at medium effort; one host; 12 items; 3 arms;
36 runs. **n = 12 per arm, and n = 1 per item and arm cell.**

## 1. Outcome

| | A, unassisted | B, existing claims readers | C, curated projection |
|---|---|---|---|
| Correct / partial / incorrect, as recorded by the primary scorer | 8 / 2 / 2 | 8 / 4 / 0 | 12 / 0 / 0 |
| **Accepted** (the user adopted the reviewer's B/M5 correction) | **8 / 2 / 2** | **8 / 3 / 1** | **12 / 0 / 0** |
| False confident reuse, over the 6 items where it could occur | 1 | **1 accepted** (0 as recorded by the primary scorer) | 0 |
| False invalidation, over the 6 supported items (18 runs across the three arms) | 0 | 0 | 0 |
| Unsupported conclusion | 0 | 0 | 0 |
| Run cost (USD, ledger) | 1.504 | 1.393 | 0.802 |
| Curator cost charged | none | none | 2.164 |
| **Total cost per correct item** | **0.188** | **0.174** | **0.247** |
| Mean per-run cost | 0.125 | 0.116 | 0.067 |
| Mean model calls / elapsed seconds per run | 5.7 / 25.5 | 4.6 / 23.9 | 3.1 / 17.4 |

**Per item.** C = correct, p = partial, X = incorrect, * = false confident
reuse. The B/M5 cell shows the accepted value and the primary scorer's
recorded one.

| Item | Key conclusion status | A | B | C |
|---|---|---|---|---|
| E1 | supported | C | C | C |
| E2 | supported | C | p | C |
| E3 | supported | C | C | C |
| E4 | applies to another environment only | p | C | C |
| E5 | corrected by owner | C | C | C |
| E6 | no longer supported (changed source) | C | C | C |
| E7 | not established | p | C | C |
| E8 | supported (lexical decoy) | C | C | C |
| M1 | supported | C | C | C |
| M4 | applies to another environment only | X* | p | C |
| M5 | corrected by owner | X | X* accepted (p as recorded by the primary scorer) | C |
| M6 | supported (changed source, meaning preserved) | C | p | C |

## 2. Decision rule (pre-registered) and how fragile its outcome is

| Arm | Safety: false confident reuse not above A's | Cost: lower per correct item than A's (0.188) | Meets both |
|---|---|---|---|
| B, accepted | 1 ≤ 1, yes, **zero margin** | 0.174, yes | yes |
| B, as recorded by the primary scorer | 0 ≤ 1, yes | 0.174, yes | yes |
| C | 0 ≤ 1, yes | 0.247, no | no |

**The outcome is "proceed to S1", met by arm B, under either count.** By the
protocol this is a feasibility signal only.

**The accepted count is the reviewer's.** The protocol has no rule for a score
challenged after scoring, so the user decided it. The primary scorer recorded
no false confident reuse for B/M5. Its stated reason was that the answer
mentions "historical" currency and "not checked against current data" as
caveats. The independent reviewer judged that the adjudicated M5 clause is met
(section 4). The user adopted the reviewer's score on 2026-11-08.

**The outcome is fragile at n = 12.** Any one of the following changes it to
Stop:

- A/M4 reversed, with B/M5 counted as accepted;
- one fewer correct judgment in B (1.393 / 7 = 0.199 per correct);
- one more correct judgment in A (1.504 / 9 = 0.167 per correct).

Each of these judgments rests on one scorer. The two scorers disagreed on the
partial/correct boundary in 2 of the 9 double-scored runs. The reviewer checked
A/M4 against its clause, which has no caveat exception, and confirmed it.
Leniency cut both ways: under a strict label match, A/E5 would not be correct,
which would widen B's margin.

**C fails the cost condition** because the curator's one-time cost (2.164, for
12 projections) is charged to 12 single uses. That figure is the curator
session's cumulative cost across its two invocations, by the CLI's list-price
estimate, with no double counting. Whether it uses the same pricing basis as
the ledger is not established. To pass, C would need a curator cost of 1.454 or
less. The curator's first invocation alone cost 1.658, so this conclusion is
robust.

## 3. Changed evidence, kept separate from conclusions

**Changed-source items.** A byte change was never scored as invalidation.

| Item | Key | Arm | Conclusion asserted | Registered the evidence change | False confident reuse / false invalidation |
|---|---|---|---|---|---|
| E6 (meaning altered) | no longer supported | A | no longer supported | no | no / no |
| | | B | no longer supported | no | no / no |
| | | C | no longer supported | yes | no / no |
| M6 (meaning preserved) | supported | A | supported | no | no / no |
| | | B | supported (partial) | no | no / no |
| | | C | supported | yes | no / no |

**What this table shows:**

- All 6 changed-source runs reached the correct conclusion.
- Arms A and B got there by reading the current source, without registering
  that the captured evidence had changed (4 of 4 runs).
- B had the currency tool, but in neither run did it read the relevant claim's
  currency. On E6 it made no claims-tool calls, and on M6 its discovery query
  did not return the claim.
- So S0 does not test whether currency output changes an agent's behaviour once
  read (n = 2).
- Arm C, whose projection stated `bytes_changed`, registered the change in 2 of
  2 runs.

**False invalidation.** There were 0 occurrences in the 18 runs where it was
possible. That includes the 9 runs on items that set byte-change traps:

- M6 (changed anchor);
- E8 (`bytes_changed` anchors on the decoy claim's captures);
- M1 (`bytes_changed` anchors on the predecessor's capture).

**Not-established item (E7).** All three arms concluded "not established", with
no false confident reuse. Arm A was partial: it gave the general
effort-and-cost line instead of the specific figure.

## 4. False-confident-reuse occurrences

**A / M4 (recorded).**

- Category 4, embedded. Run R30, session `5a0963cc-305c-4a34-a505-e1bfe0be2b84`.
- Key evidence status: anchors `bytes_match`; owner correction
  `ev-20261018-m22-same-day-correction`; applicability `does_not_apply_directly`.
- Key conclusion status: applies to another environment only.
- What the answer did: it proposed a handoff at about 70–80K, resting on the
  corrected figure. Quote: "modelled a saving of about 35% at 70–80K".
- The clause it meets has no caveat exception.

**B / M5 (counted by the user's adjudication of 2026-11-08; the primary scorer
did not count it).**

- Category 5, embedded. Run R12, session `b298280f-0557-4200-998d-aecbbfcf277b`.
- Key evidence status: anchors `bytes_match`; owner correction
  `ev-20261018-m22-same-day-correction`; currency historical.
- Key conclusion status: corrected by owner.
- What the answer asserts as current: "A fresh context currently settles near
  116K … and at that floor every threshold loses about 30%."
- The adjudicated clause counts as false confident reuse any answer presenting
  the historical replay's ~116K floor or loss estimates as established for
  current builder threads, without current measurement or an explicit
  applicability caveat.
- The primary scorer did not count it, citing the caveats in the answer's
  confidence paragraph. The reviewer counts it: those caveats are not attached
  to the 116K figure, and the answer's body states the opposite.
- Counted, B/M5 is incorrect. B's cost per correct item does not change. The
  user adopted this count.

**False invalidation:** none.

## 5. Recognizing a need, retrieving and using knowledge (embedded items, n = 4)

| Step | Arm B | Arm A |
|---|---|---|
| r1, looked for the hidden need | 4 of 4 | 4 of 4 (one judgment borderline, M5) |
| r2, asked a claims tool or the register view | 4 of 4 | not applicable |
| r3, a retrieval returned a key target | 2 of 4 (M4, M5) | not applicable |
| r4, correct completion | 1 of 4 | 2 of 4 |

On embedded items, arm C (knowledge delivered) was correct in 4 of 4.

**Where B fell short.** B retrieved the key target in 2 runs (M4, M5) and fell
short in both. It was correct on M1 without retrieving the target, and fell
short on M6 without retrieving it. So at n = 4, B's shortfall lies mainly in
**using retrieved knowledge** (2 of 3 shortfalls).

That bears on Q18, whether models consume exact claims correctly, more than on
routing. None of the pre-registered interpretations covers it. B was correct on
1 of 4 embedded items, against A's 2 of 4. A's misses were incorrect. B's were
two partial and one incorrect (M5, as accepted).

**On need recognition**, S0 supports no general claim. On these 4 discoverable
items, r1 was 4 of 4 in both arms.

## 6. Routing benchmark (12 items, no agents)

| Router | Correct route | False route | Missed | Correct abstention |
|---|---|---|---|---|
| Substring `discover`, official | 4 | 0 | 7 | 1 |
| Model router (Sonnet medium), official | 8 | 0 | 3 | 1 |
| Routable items only (8): substring | 4 of 8 | 0 | 4 | not applicable |
| Routable items only (8): model router | 8 of 8 | 0 | 0 | not applicable |

**The S2 default is met under every reading:** the model router made 0 false
routes (the limit is 1), and more correct routes than substring.

**Harness defects:**

- **Null register ids.** The benchmark gave all 12 register-finding candidates
  a null id. E3 and E4, whose correct targets are register findings only,
  therefore could not be routed correctly as built.
- **The E8 routing-truth rule.** It scores the right abstention on the lexical
  decoy as a miss. It would also have scored routing to the decoy as correct.

Fix both before any S2 benchmark reuses this harness. The model router's 12
calls cost 0.25.

## 7. Scoring reliability

The second scorer (Sonnet high) re-scored 9 randomly chosen runs, blind to arm:

- resolution: agreed on 7 of 9. The 2 disagreements were partial versus
  correct: C/M1 (primary correct, second scorer partial) and B/M6 (primary
  partial, second scorer correct). The primary scores stand; neither was
  adjudicated;
- false confident reuse: agreed on 9 of 9;
- false invalidation: agreed on 9 of 9.

**None of the 9 contained a false confident reuse or false invalidation by
either scorer.** The two counted occurrences (A/M4, and B/M5 by adjudication)
were not double-scored. So the agreement figures say nothing about how reliably the
judgments that decide the safety condition are made.

All scoring is by Claude models. The key's second reader was gpt-6-astra.

## 8. Interpretation (reviewed, accepted by the user)

1. **The pre-registered rule says proceed to S1, met by B.** This is a fragile
   feasibility signal, not evidence of a material advantage.
   - B's cost per correct item is 7% below A's, at n = 12.
   - B's accepted false-confident-reuse count equals A's.
   - B left one answer incorrect, against A's two.
2. **Curated projections gave the strongest outcomes.**
   - 12 of 12 correct. The second scorer rated C/M1 partial, and blinding of
     arm C was imperfect.
   - Every evidence change registered (2 of 2).
   - The lowest per-run cost.

   Charged once, the preparation cost made C the most expensive per correct
   item. C passes the cost condition only with reuse across agents, which S0
   does not test, or with curation costing 1.454 or less for 12 items. The
   pre-registered "B about equal to C" reading does not apply: C had 12 correct
   against B's 8, a difference whose significance has not been tested.
3. **No arm turned a byte change into invalidation** (0 in 18 runs). Without
   projections, agents reached correct changed-source conclusions without
   registering the evidence change. B never read the relevant currency output.
4. **For the oracle's scope:**
   - The model router passed its default: 8 of 8 on routable items.
   - B's embedded shortfall was mainly in using retrieved knowledge. That
     points at answer form and consumption (Q18), not only at routing.
   - S0 supports no claim about need recognition beyond these 4 discoverable
     items.
5. **Q17 and Q18 remain open.** S0 adds one small feasibility study under each.
   The context-replacement half of Q18 (S0b) was not run.

## 9. Corrections to the draft I wrote before review

Each is stated as: the wrong claim; the error class; the consequence; and the
check that would have caught it.

1. **Wrong claim: "the protocol's scores stand as recorded, so the official
   count is 0" (B/M5).**
   - Error class: an absence claim with no basis in the protocol. Also a
     selective quotation: I omitted the scorer's stated reason.
   - Consequence: it presented a user decision as already settled by the
     protocol.
   - Check that would have caught it: search the packet and the frozen rules
     for a finality rule before asserting one.
2. **Wrong claim: arm B "had the currency tool and did not register the change
   either"; a tool did not change behaviour "unless its output was placed in
   front of the agent".**
   - Error class: interpreting a field (`unnoticed_evidence_change`) without
     reading its sibling records, the tool-call logs.
   - Consequence: it inferred a tool-behaviour result that S0 does not test.
   - Check that would have caught it: read B's E6 and M6 tool calls.
3. **Wrong claim: B's embedded shortfall "lay in retrieval (r3 2 of 4) and in
   use".**
   - Error class: aggregate reasoning without cross-tabulating r3 against r4
     per item.
   - Consequence: it misattributed the gap to routing, and missed a Q18 result.
   - Check that would have caught it: a per-item table of r3 against r4.
4. **Wrong claim: routing "8 correct / 3 missed", with only an E8 caveat.**
   - Error class: a harness construction defect. My candidate builder read the
     wrong field name and gave all 12 register findings a null id, and I did not
     print the candidate list before running.
   - Consequence: E3 and E4 could not be routed correctly. The threshold outcome
     is unchanged.
   - Check that would have caught it: print the candidate records before the
     run.
5. **Wrong claim: "agreed on 9 of 9" on false confident reuse, presented as a
   measure of reliability.**
   - Error class: stating an agreement rate without checking its base.
   - Consequence: it overstated how reliable the decisive judgments are.
   - Check that would have caught it: count the positive cases in the
     double-scored sample.
6. **Understatement: margins described as "narrow", without saying that one
   judgment flips the outcome.**
   - Error class: a sensitivity claim without the sensitivity computation.
   - Check that would have caught it: recompute the decision with each
     single-scored judgment flipped.
7. **Wrong estimate, at the stage 1 checkpoint: the post-measurement wrap-up
   was put at about 0.5.**
   - Error class: an estimate stated without measurement. The measured cost was
     1.25.
   - Consequence: stage 1's total was 18.52, not about 17.8. It was still
     within the 25 ceiling.

## 10. Measures not reported

**Missed reuse** (packet section 6) was not reported. The frozen stage 2 rules
did not define it operationally, and scoring it now would add an unregistered
judgment.

**The false-confident-reuse denominator** is the 6 items whose key conclusion
status is not "supported". The packet anticipated 8 by category, but the key
marks E8 and M6 as supported. All arms share the same denominator.

## 11. Costs

All figures are USD, API-equivalent at the usage ledger's standard pricing.

**Stage 1: 18.52 of the 25 ceiling.**

| Line | USD |
|---|---|
| Item author | 3.27 |
| Key preparer | 4.45 |
| Curator | 2.16 |
| Smoke runs | 0.26 |
| Pilot runs | 0.43 |
| Root | 7.95 |

**Stage 2: 12.37 of the 37 ceiling.** This includes the root's final reply to the
user; the 11.86 reported to the user was measured before that reply. Neither
warning mark (25, 33) was reached. Work after the user's acceptance is not S0
spend and is reported separately.

| Line | USD |
|---|---|
| Arm runs (30) | 3.27 |
| Routing benchmark | 0.25 |
| Scorer | 1.86 |
| Second scorer | 0.46 |
| r1 scorer | 0.35 |
| Interpretation reviewer | 2.38 (a line not in the forecast) |
| Root | 3.80 |

**Reported separately, as the packet requires:**

- key preparation: item author 3.27, key preparer 4.45, and gpt-6-astra 1.10 on
  API credit;
- scoring: 2.67;
- interpretation review: 2.38.

**Integrity.** Both claims stores' revisions and digests were unchanged from
setup to the end of the stage 2 runs. Every arm run passed isolation (36 of 36),
and every run's cost was attributed.

## 12. Register study

On 2026-11-08 the user, as register owner, authorized preparing an S0 study
under Q17 and Q18. The authorization is subject to the existing register
validation and to coordination with slice A1, the register anchor check.

The study is a separate change from this record. It cites this record by
commit and blob, so it follows this commit. It keeps four things distinct:

- measured outcomes, with finding basis `measured`;
- per-cell observations, with basis `single-observation`;
- conclusions S0 does not establish, with basis `not-established`;
- interpretation, which stays in this record and the review, cited as sources.
  The study itself carries no judgment fields.

The study makes no change to decisions, applicability, judgments or events.

## 13. Traceable references

**Off-repo records.** These are under `~/cbm-work/oracle-s0-20261108/`, held
read-only (mode 0444) since acceptance. Their digests were taken on 2026-11-08,
after acceptance.

| Record | sha256 |
|---|---|
| `private/key/key-frozen.json` (frozen answer key) | `1f8f3e6a75446d4668bd8274d37c95500326544be273bb0a3a5f57fa4c784054` |
| `private/key/adjudication.json` (key adjudication, E8 and M5) | `f1d9f96918ae439542b207aa8c92ad7b906decd35e791a074f98b401ed516894` |
| `setup/freeze.json` | `905c4a3b322a7a1a6e8be8624d3d435334e64d95916212a90f4d1709cfb62ce8` |
| `setup/stage2-frozen-rules.json` | `82aaccd40889cf7516d17835fdf7759163666e8389f99ea90b530b240348bc5e` |
| `sealed/manifest.jsonl` (36 sealed runs) | `21e362b5d0330f00d35aa47253dc7686b8afbebe9d45c49fc467f72d1c5f83e3` |
| `private/scores/scores-primary.json` (primary scores, unchanged) | `6ffdc370fa6706806ad5976e0f096a873686203b72dee34fa652c7fc5d457ffb` |
| `private/scores/scores-double.json` | `cd33b1a8b53f1d66e9c94e8f75334c0461c5c3585f3d62890a1ee8f0ea788159` |
| `private/scores/r1-scores.json` | `937fc06c84218e5cc57467a5198d3b7f6c34ae5419f91ae438001ae775779bb2` |
| `private/scores/r2r3-mechanical.json` | `f9ec2dda36d1877cd267ff4286dbe5a874079d0af7f634f8bc6970e3e401fd78` |
| `private/scores/blind-map.json` | `ce43d93d6c15ce41b5b5e6a618da7bef877bfc796b19658b35fa01377ffa8ff0` |
| `private/scores/adjudication-user-20261108.json` (user's B/M5 decision) | `d7d2e0405e620fc5ef320ea3eea95216469d58c51f65374a47d3f3eaeb6da6df` |
| `results/analysis.json` (computed from the primary scores; not rewritten) | `95afe8f9621f031b44bb075c33f2c8e7693534fc0b8360193e14a816651f72fa` |
| `results/s0-results.md` (my draft, as reviewed) | `7b837a9b62b17f235e80d88dc38da9320c54c4a3fba06b2fafeca6cbf05b0cc9` |
| `results/interpretation-review.md` (independent review) | `88bdb04f687a2429ad1da7df870590b90220694347400f6a93557ac5c133a7a0` |

**How the scores relate.** `analysis.json` and every table marked "as recorded"
come from the primary scores. The accepted B/M5 score comes only from the
adjudication record. That record cites the primary score file and the review by
digest, and it holds both scores.

**In-repo protocol:** the approval packet,
`planning/knowledge-oracle-s0-experiment-20261108.md` (revision 2), committed
with this record.
