# DC0: decision-card delivery feasibility (proposal)

**Status:** proposal, revision 2, 2026-11-08, from the Knowledge Oracle owner
(session `0440d798`). This is the next feasibility step after S0, which the
user accepted on 2026-11-08 (commit `daad4f2b`,
`metrics/probes/knowledge-oracle-s0-result-20261108.md`). **Stage 1 is on hold
by the user's decision**, and nothing here is authorized or running.

**Committed on 2026-11-08 as a held experimental proposal** (the user's third
decision). Committing it does not authorize Stage 1.

**Unmet prerequisites:**

1. **Interface and ownership.** The decision-aware owner has not answered K-1
   to K-7, and the card interface is not reconciled.
2. **Authorization.** 12 of the 13 applicability records are pending.
   - The user will not bulk-authorize them.
   - Each relationship needs an established owner and its own evidence-based
     disposition.
   - First: the owner of `context-boundary.h2-levels`, and its M5 record
     `ap-k5-conditional-context-boundary-h2`.
   - Pending records are never used as authorized card evidence.
3. **Coverage.** Under R2 the 5 reused items give 3 complete, 1 partial and
   1 absent. New items must not be chosen merely to reach the 10-of-12
   threshold.
4. **Budget.** Stage 1 must be re-priced and authorized afresh. Any all-in
   limit counts root and reviewer calls together.

**The user's second decision (2026-11-08):**

- **Hold:** DC0 Stage 1 waits for the decision-aware owner's response and a
  reconciled card interface and ownership.
- **Provisional ownership:**
  - the decision-aware capability owns the read-only decision card and the
    recognition of information needs;
  - the Oracle owns question resolution;
  - the register owns decision identities and the authorized links between
    decisions and questions.
- **Evidence the card selects:** findings linked through owner-authorized
  applicability records and correction events, not just the decision's own
  references.
- **Changed anchors:** a changed decision anchor gives an explicit warning and
  never silently suppresses corrective evidence.
- **No inference:** applicability and standing are never inferred.

**Revision 2 changes:**

- section 2.3 is new: the desk evaluation repeated under the user's selection
  rule (R2);
- R1 is replaced by R2 in sections 3 to 7;
- sections 8 to 10 are updated.

Revision 1 (sha256 `ad776376…`, cited on board seq 4908) is kept read-only at
`~/cbm-work/dc0-proposal-history/dc0-rev1-ad776376.md`.

**Coordination:** the proposal went to the decision-aware integration thread
(`evidence-integration-root`) on board topic `knowledge-oracle`. Its
concurrence is **pending**. So is its confirmation of the boundary (board seq
4900, now replaced by the user's provisional ownership above).

**Not authorized by the user's decision:** no Oracle S1 implementation, no live
claims publication, no deployed skill change, no push. DC0 adds none of these.
It does not implement the card in `register.py` (that is slice A4), and it
changes no register record.

## 1. Why this step

S0's strongest result was the gap between raw evidence access and curated
delivery:

- **Curated projections (arm C):** 12 of 12 correct.
- **Existing claims readers (arm B):** 8 correct, 3 partial, 1 incorrect.
- **Arm B on the embedded tasks:** 2 of its 3 shortfalls came **after** the key
  evidence had been retrieved (M4 and M5).

All of this is n = 12 per arm, with n = 1 per item and arm. The user's
direction for the next step:

- evaluate whether the proposed deterministic decision card can deliver
  compact, properly qualified evidence for known decisions, using the existing
  register and native claims readers;
- separate the contribution of evidence selection, evidence presentation,
  proactive delivery and agent interpretation;
- resolve known decisions through the shared decision-card mechanism, with no
  decision routing in the Oracle;
- leave novel or ambiguous questions as the Oracle's proposed responsibility.

## 2. Desk evaluation already done (deterministic, no model runs)

**Inputs:**

- S0's frozen key;
- the register at `14031bc2`, whose register files are byte-identical to S0's
  pin `97f6486f`;
- A1's anchor classification at `14031bc2`, from its round-2 output
  `~/cbm-work/register-anchor-a1-20261108/real-register-14031bc2-r2/anchors.json`.
  A1 is not yet accepted, so its classes are `unverified` here.

**Selection rules compared:**

- **R0:** the decision record's own `findings` list.
- **R1:** R0, plus findings that carry an applicability record on the decision,
  plus findings named by any event that reaches a selected finding (as subject
  or `by_finding`).

**Revision 2 note:** R1 followed applicability records whatever their
authorization state, and events of every kind. It therefore used links that
are not owner-authorized: 12 of the 13 applicability records are pending. The
user's rule excludes those links, and section 2.3 repeats this evaluation
under it (R2). Sections 2.1 and 2.2 are kept as revision 1's reading.

### 2.1 S0 items with a registered decision (5 of 12)

| Item | Decision (S0 item record) | A1 anchor class | Register evidence the key needs, under R0 | Under R1 | Through the card's claims leg |
|---|---|---|---|---|---|
| E3 | `model-selection.builder-default` | moved | present (K1 comparison, strength low, n = 19 not significant) | same | none (materialized is empty) |
| E4 | `model-selection.cost-table-effort-row` | moved | present (Codex scope, currency drifting, applicability rationale "says nothing about task-level effect") | same | none |
| E5 | `model-selection.claude-host-builder-rows` | **modified** | present (traceability correction event) | same | none |
| M4 | `pi-docker-convergence.context-replacement` | unchanged | partial: K5 and its correction only. The K6 record ("Codex compaction points do not govern Pi jobs") is missing | complete | none |
| M5 | `context-boundary.h2-levels` | unchanged | **absent.** The decision lists only the two K6 compaction findings. K5, the corrected 35% reading and the 116K-floor condition reach it only through `ap-k5-conditional-context-boundary-h2` (relation `not_established`) and the correction event | complete | none |

### 2.2 What this shows

These figures come from 5 items, with n = 1 per item.

1. **Coverage.** Only 5 of S0's 12 items name a registered decision. The other
   7 are outside the card's reach and stay with source reading, or with the
   Oracle's proposed resolution.
2. **The selection rule decides the outcome.** Under R0 the card would miss
   M5's evidence entirely, and part of M4's. M4 and M5 are the two runs in
   which arm B retrieved the evidence and still fell short. Under R1 the
   register evidence the key needs is present for all 5. R1 adds findings for
   5 of the 11 decisions.
3. **Anchor gating could withhold the needed evidence.** E5's decision is
   `modified` under A1. The A4 sketch answers such a decision with
   `anchor_unqualified`. If that state withholds the evidence, the card
   withholds exactly the owner correction E5 needs. The proposed rule is
   qualify, never withhold: deliver the evidence and state that the decision
   text changed after the revision the judgments cite.
4. **The claims leg is empty today.** `materialized.jsonl` has no rows, so no
   register finding resolves to a claim, and native claims readers add
   nothing through the card for any of the 5 decisions. The live store does
   hold a repository-fact claim, `claim-v1@c4dd4826…`, that states K5's
   conditional result, but nothing links it to the finding. Linking it is
   materialization, which is deferred with live K2 publication.
5. **The qualifying information exists.** For all 5 decisions the register
   carries currency, evidence strength, standing and events, and the
   applicability relation with its rationale. Whether rendering them prevents
   misuse is **not established**. B/M5 is the warning case: the answer cited
   "historical" currency in its confidence paragraph, yet presented the 116K
   floor as current. A caveat separated from its figure did not prevent
   misuse in that run. Presentation is therefore a variable to test, not a
   formatting detail.
6. **Size.** The raw register rows selected under R1 come to 2,844–11,745
   characters per decision over all 11 decisions, roughly 0.7K–3K tokens at 4
   characters per token (`unverified`). S0 runs began at about 21K input
   tokens. No card has been rendered, so the card's own size is **not
   established**.

### 2.3 Repeated under the user's selection rule (R2)

**R2:** R0, plus findings linked to the decision by an applicability record
whose `authorization.status` is `authorized`, plus findings named by a
`correction` event that reaches a selected finding (as subject or
`by_finding`). Nothing is inferred. A pending applicability record is not a
link.

**Inputs:** the register at `9fdf4b7e`. Its decision, applicability and event
files are byte-identical to `97f6486f`.

**What the register holds:**

- **Applicability records:** 1 of 13 is authorized,
  `ap-k3-quota-fit-cost-table-quota-row` (relation `not_established`). The
  other 12 are `pending`. Three of them name no owner, all on
  `context-boundary.h2-levels`: `ap-k5-conditional-context-boundary-h2`,
  `ap-k6-no-limit-context-boundary-h2` and `ap-k6-limit-context-boundary-h2`.
- **Events:** 2 of 6 are corrections:
  - `ev-20261027-e1a-traceability`;
  - `ev-20261018-m22-same-day-correction`.

  The other 4 are 2 challenges and 2 status discrepancies, which R2 does not
  follow.
- **Decision-to-question links:** none.
  - **Search:** every key of the 11 decisions, the 20 questions, and the
    applicability, event, judgment and materialized rows and study files.
  - **What exists instead:** decisions carry anchor, document, findings, lines,
    owner and revision. Questions carry area, id and text. Studies link to
    questions; applicability records link findings to decisions.
  - **Consequence:** the item-to-decision mapping DC0 uses comes from the S0
    item records, which are experiment-authored, not register-authorized.

| Item | Decision | Under R0 | Under R1 (revision 1, followed pending records) | Under R2 |
|---|---|---|---|---|
| E3 | `model-selection.builder-default` | present | same | **present** |
| E4 | `model-selection.cost-table-effort-row` | present (the finding's Codex scope) | same, plus the rationale of `ap-k4` | **present.** The finding and its scope are selected. The `ap-k4` rationale ("says nothing about task-level effect") is in a pending record, so it is not |
| E5 | `model-selection.claude-host-builder-rows` | present | same | **present**, through the traceability correction event. The decision is `modified` under A1, so the card warns and does not withhold |
| M4 | `pi-docker-convergence.context-replacement` | partial | complete | **partial.** K5 and its correction are selected. The K6 link (`ap-k6-no-limit-pi-convergence`, `does_not_apply_directly`) is pending |
| M5 | `context-boundary.h2-levels` | absent | complete | **absent.** K5 and its correction reach this decision only through `ap-k5-conditional-context-boundary-h2`, which is pending and names no owner. The correction event's findings are not in R0 for this decision |

**What this shows** (5 items, n = 1 per item):

- **R2's coverage today equals R0's: 3 complete, 1 partial, 1 absent.**
- **The M5 gap is now an authorization gap, not a gap in the selection rule.**
  M5 is the S0 case of false confident reuse in arm B. A card that does not
  infer applicability cannot deliver M5's corrective evidence until the owner
  authorizes, or rejects, the pending record that links it.
- **The record's owner is itself not established.** That has to be settled
  before anyone can authorize it.
- **Effect on the design:** R1's selection contrast is not available under
  the user's rule. Stage 1's criterion "R coverage of at least 10 of 12 items"
  would rest on the authorization state of pending records, not on the card.

**Options, for the user and the decision-aware owner** (no recommendation is
adopted here):

1. **Establish owners, then decide each record.**
   - First, establish the owner of each decision involved.
   - Each pending record then gets its own evidence-based disposition. The
     user ruled out bulk authorization.
   - For the 5 reused items the records are
     `ap-k5-conditional-context-boundary-h2` (its decision has no established
     owner), `ap-k6-no-limit-pi-convergence` and
     `ap-k4-effort-cost-table-effort-row`.
   - DC0 then measures the card on authorized links only.
2. **Show pending records as pending.** The card lists a pending record as a
   proposal, labelled as neither applicable nor authorized. Whether that
   counts as inferring applicability is the user's and the decision-aware
   owner's call, not the Oracle owner's.
3. **Narrow DC0** to decisions whose needed evidence R2 reaches today (E3, E4
   and E5). This measures presentation and delivery, but not the
   corrective-evidence case that motivated DC0. The user has ruled that new
   items must not be selected merely to meet the coverage threshold.

## 3. The four contributions and how DC0 separates them

| Contribution | What it means here | How it is isolated |
|---|---|---|
| Evidence selection | which records reach the agent | **Mechanical:** for each item, does the card's selected set contain every register element the frozen key requires (R0 and R2)? Measured with no agents, at Stage 1. **In runs:** Dp against C, where C uses a curator's selection and form, with each run attributed by whether the required evidence was present |
| Evidence presentation | the same records, raw or rendered with each figure's qualification attached | **Dr against Dp:** same selection, same delivery, different form |
| Proactive delivery | the card is pushed at the decision point, or the agent must decide to look | **Dp against Do:** same selection and form. Do also measures how often the agent looks, and whether it asks for the right decision |
| Agent interpretation | use of correct, qualified, delivered evidence | **Residual:** Dp runs in which the mechanical check shows complete evidence and attached qualifications, but the answer is incorrect or commits false confident reuse. Also B and Do runs that retrieved the evidence and misused it (S0's r3 and r4) |

**Recognition is not measured.** Proactive delivery uses the decision identity
written by the item author, so it assumes perfect recognition: an upper bound,
like S0's arm C. Measuring recognition belongs to the decision-aware path.

## 4. Design

### 4.1 Items

12 tasks, all tied to a registered decision. One run per item and arm.

- **5 reused from S0** (E3, E4, E5, M4, M5), with their frozen key entries and
  S0's W0 worktree (`git archive 97f6486f`).
- **7 new tasks, mostly with an embedded need,** written by a fresh item author
  over the decisions S0 did not cover:
  - `dogfood-plan.round2-status-line`;
  - `model-selection.cost-table-quota-row` (`modified`);
  - `model-selection.terra-mechanical-builder`;
  - `pi-coding-profile.acceptance-contract`;
  - `roadmap.current-strategic-route`;
  - two more, with at least one on a decision whose evidence carries a
    correction or challenge.

  The historical pin `...-at-df288c3d` is excluded.
- **No decision-free items.** Novel and ambiguous questions belong to the
  Oracle and are out of scope.

### 4.2 Arms

| Arm | Selection | Presentation | Delivery | Tools |
|---|---|---|---|---|
| A | none | none | none | worktree reading only (as in S0) |
| B | the agent | raw tool output | on request | S0's arm B: claims readers and the register view |
| Dr | card (R2) | raw register rows, verbatim | proactive (in the prompt) | worktree reading |
| Dp | card (R2) | card form: each figure with its scope, currency, strength, standing and events, and the applicability relation and rationale, in one entry; plus the resolution state, the anchor class and any changed-anchor warning | proactive | worktree reading |
| Do | card (R2) | card form | on request: a card index (decision ids and anchors) plus card files the agent may read | worktree reading plus the card directory |
| C | curator | curator's projection | proactive | as S0's arm C. The 5 reused items reuse S0's projections; 7 are new |

**Run count:** 72 runs, 12 per arm. The model and isolation controls are as
in S0: `claude-sonnet-5-5` at medium effort, no Bash, Edit or Write, deny
rules, a transcript scan, and sealed outputs.

### 4.3 The card emulator (an experiment fixture, not a mechanism)

**What it is:** a deterministic script in the experiment folder. It renders
the R2 selection, in Dr's raw form and Dp's card form, for the 11 decisions.

**Rules it follows, from the user's decision:**

- **Changed anchors.** A decision whose anchor A1 classes as anything other
  than `unchanged` gets an explicit warning in the card. The warning names the
  class, and the revision the judgments cite. The evidence is still delivered.
- **No inference.** Applicability comes only from authorized records, and
  standing only from recorded judgments and events. A pending record is not
  shown as applicability; how it is handled waits on section 2.3's options.

**What it reads:**

- the register at `97f6486f`;
- A1's `anchors.json` for anchor classes, so that A1's logic is not
  reimplemented.

**What it must not do:** use a model, open a claims store or write outside the
experiment folder.

**Format:** the A4 response draft (slice proposal section 5).

- **States:**
  - `claims_found`, `register_findings_only`, `source_unavailable` and
    `refused`, with the Oracle's meanings;
  - `no_findings_linked`, `none_found_in_corpus`, `decision_not_registered` and
    `anchor_unqualified`, with the decision-aware side's meanings.
- **Kept distinct:** decision state, resolution state and claim standing.
  This follows the user's reconciliation decision.

**Ownership** (coordination point K-1, section 8):

- The decision-aware owner owns the selection and rendering rules and confirms
  that the emulator matches them.
- The emulator is discarded after DC0, or handed to A4 as a reference.
- It is never deployed, and it gives the Oracle no decision routing.

### 4.4 Key and scoring

- **Key:** S0's format (evidence status separate from conclusion status; a
  byte change sets only evidence status).
  - The 7 new entries come from a fresh Opus key preparer, with a second
    reader (gpt-6-astra on API credit, as in S0).
  - The user adjudicates disagreements.
  - The key is frozen before any arm run.
  - For every item, the key lists the **register elements required**
    (finding, event and applicability ids), so that selection can be checked
    mechanically.
- **Scoring:** S0's frozen rubric. One primary scorer blind to arm.
- **Second scorer:** a random 18 runs, plus **every** run that either scorer
  marks as false confident reuse or false invalidation. This fixes S0's gap,
  where the decisive judgments were never double-scored.
- **A rule for challenged scores, registered before scoring.** A score
  challenged by the second scorer or the interpretation reviewer goes to the
  user. Both scores are kept, and the user's decision is a separate record.
- **Per-run attribution:**
  - evidence present (mechanical for Dr, Dp and C; transcript-based for B and
    Do);
  - qualification attached (mechanical for Dr and Dp);
  - looked up (Do and B);
  - used correctly (a model scorer, as for S0's r4).

## 5. Acceptance criteria

### Stage 1: setup and mechanical evaluation, ending at a checkpoint

| ID | Criterion |
|---|---|
| S1-a | The emulator is deterministic: two runs are byte-identical. Register digests are unchanged. No model call and no claims access, shown by code review and a run with no claims configuration. An independent check leaves no unresolved P1 or P2 |
| S1-b | The decision-aware owner confirms that the emulator's selection rule, card form and states match its A4 design, or its amendments are applied |
| S1-c | A mechanical selection report covers all 12 items: each required register element is present or absent under R0 and R2, with the authorization state of every applicability record involved |
| S1-d | A qualification audit checks every figure rendered in Dp form: are scope, currency, strength, standing and events, and the applicability relation all in the same entry? It also records the card size in characters, and in tokens as counted on pilot runs |
| S1-e | Key: 7 new entries prepared independently, second-reader concordance recorded, disagreements adjudicated by the user, and the key frozen with a digest before any arm run |
| S1-f | Pilot: 6 runs (Dr, Dp, Do on 2 items) pass isolation, and their costs are attributed |
| S1-g | Integrity: claims stores, register, S0 records and A1's outputs are unchanged |

**The checkpoint reports:** setup integrity, costs, the S1-c and S1-d
mechanical results, the key digest and the pilot cost figures.

**It contains no agent outcomes.** The mechanical results describe the card,
not the agents, and they decide whether Stage 2 is worth running. If R2 covers
fewer than 10 of 12 items, Stage 2 is not proposed. Section 2.3 shows that
this threshold currently depends on pending authorizations.

### Stage 2: runs, scoring and review

These are registered before any Stage 2 run.

| ID | Reading (feasibility signal only; n = 12 per arm, n = 1 per cell) |
|---|---|
| S2-1 Selection | S1-c coverage, plus the C − Dp difference split by per-run "evidence present" |
| S2-2 Presentation | Dp − Dr in correct count and in false confident reuse |
| S2-3 Proactive delivery | Dp − Do in correct count; also Do's look-up rate and the correctness of the decision ids it asks for |
| S2-4 Interpretation | Dp runs with complete, qualified evidence that are incorrect or commit false confident reuse; also retrieved-then-misused runs in B and Do |
| S2-5 Fragility | for each reading, which single judgments would change it |

**Decision rule.** This is a proposal; the user may set other thresholds. DC0
gives the signal **"deterministic card delivery is feasible; consider A4"**
only if all of these hold:

- Dp's false-confident-reuse count is no higher than both A's and B's;
- Dp's correct count is at least B's plus 2;
- Dp's correct count is at least C's minus 1;
- Dp's total cost per correct item is no higher than B's.

**Supporting readings:**

- **"Presentation matters":** Dp − Dr ≥ 2 correct, or Dp has fewer false
  confident reuses.
- **"Proactive delivery matters":** Dp − Do ≥ 2 correct.

**What DC0 cannot establish:** that the card is ready for production; how
often needs are recognized in ordinary work; or anything about undecided or
novel questions.

## 6. Costs

All figures are API-equivalent USD, Claude, forecast from S0's per-node actuals
(n = 1 per role) and `unverified` for DC0.

**Stage 1:**

| Line | Basis | Forecast (low / point / high) |
|---|---|---|
| Item author, Opus high, 7 items | S0: 3.27 for 12 items plus a patch | 1.5 / 1.9 / 2.9 |
| Key preparer, Opus high, 7 items | S0: 4.45 for 12 | 2.0 / 2.6 / 3.9 |
| Curator, Opus high, 7 projections | S0: 2.16 for 12 | 0.9 / 1.3 / 2.0 |
| Independent check of the emulator | Opus review: 2.38 in S0, 2.40 and 2.77 elsewhere today | 1.5 / 2.4 / 3.0 |
| Pilot, 6 runs | S0 pilot mean: 0.071 per run | 0.4 / 0.5 / 0.9 |
| Root, including the emulator script and mechanical reports | S0 stage 1 root: 7.95 | 4.0 / 6.0 / 8.0 |
| **Total** | | **10.3 / 14.7 / 20.7** |

**Proposed Stage 1 ceiling: USD 20, with a warning at 15.**

- **gpt-6-astra, as the second key reader,** is on the API credit,
  separately: about 0.7 for 7 entries (S0: 1.10 for 12). Proposed cap
  USD 2.50.
- **The decision-aware owner's confirmation (S1-b)** is spent under its own
  budget and is not included here.

**Stage 2** (the cap is set at the checkpoint from pilot actuals, by a formula
like S0's P2):

| Line | Basis | Forecast (low / point / high) |
|---|---|---|
| Arm runs, 72 | S0 per-run means: A 0.125, B 0.116, C 0.067; D arms estimated at 0.08–0.11 | 5.5 / 6.9 / 10.4 |
| Primary scorer, 72 answers | S0: 0.052 per answer | 3.0 / 3.7 / 5.5 |
| Second scorer, about 24 answers | S0: 0.051 per answer | 0.9 / 1.2 / 2.0 |
| Attribution and use scorer | S0's r1 scorer: 0.044 per run | 1.5 / 2.1 / 3.0 |
| Interpretation reviewer, Opus | S0: 2.38 | 2.0 / 2.4 / 3.5 |
| Root | S0 stage 2 root: 3.80 | 3.0 / 4.5 / 7.0 |
| **Total** | | **15.9 / 20.8 / 31.4** |

**Proposed Stage 2 ceiling:** decided at the checkpoint, and no more than
USD 32.

**Option R, a second replicate of every cell** (72 more runs and their
scoring): about 11 more at the point estimate. Not proposed now; to be
reconsidered after Stage 2 if the readings sit on a single judgment.

## 7. Stop conditions

1. A stage ceiling is reached. Stop, even if the stage is incomplete.
2. Any write to a claims store, the register, S0's records or A1's outputs.
3. The emulator is not deterministic, or it calls a model or reads claims.
4. The key is exposed to an arm, or an arm run fails isolation.
5. The decision-aware owner objects to the emulator's rules (S1-b). Stop and
   return to the user.
6. A1's anchor classes change before acceptance. Re-run S1-c, S1-d and the
   emulator before freezing the key.
7. R2 coverage is below 10 of 12 items (S1-c). Stage 2 is not proposed.

## 8. Ownership and coordination points

Ownership follows the user's provisional decision of 2026-11-08.

| Concern | Owner | In DC0 |
|---|---|---|
| Decision identities, applicability and its authorization, register rules | register owner (the user) | read-only; no edits |
| Authorized links between decisions and questions | register owner (the user) | **none exist** (section 2.3). DC0's item-to-decision mapping is experiment-authored and labelled so |
| The read-only decision card (selection, form, states, warnings) and recognition of information needs | decision-aware capability (A4) | emulated to its rules; it confirms (K-1) |
| Anchor classification | A1 (integrated at `4e133dfc`) | its output is consumed, not reimplemented |
| Experiment design, runner, key orchestration, scoring, analysis | Oracle owner | reuses S0's harness |
| Question resolution: novel or ambiguous questions | Oracle | out of scope; no Oracle in DC0's path, and no decision routing in the Oracle |
| Claims, materialization, K2 publication | claims host and trusted producer; deferred | none. B reads the stores in place, read-only, so no store write may happen during DC0's runs |

**Asked of the decision-aware owner** (posted on topic `knowledge-oracle`;
revised after the user's second decision):

- **K-1:** who writes the emulator (the Oracle owner to your rules, or you),
  and do you confirm the A4 form above for DC0?
- **K-2:** do you adopt R2 (authorized applicability records and correction
  events; nothing inferred) as the card's selection rule? How should the card
  treat pending records (section 2.3, options 1–3)?
- **K-3:** the user decided that changed anchors give explicit warnings and
  never suppress corrective evidence. Does A4's `anchor_unqualified` state
  adopt that?
- **K-4:** the empty claims leg, and the unlinked claim `c4dd4826…` that
  states K5's result. Is this left to materialization?
- **K-5:** do the shared items and key format serve your qualification plan,
  as your proposal says?
- **K-6:** do you accept the user's provisional ownership (top of this
  document)? It replaces the boundary question of seq 4900.
- **K-7:** how does a known decision identity reach the card? Options are a
  decision id the caller supplies, or a decision-to-question link in the
  register; the register has no such link today. What does the card answer
  for an id it does not know?

## 9. Related work, separate from DC0

- **Routing harness v2:**
  `~/cbm-work/routing-harness-v2-20261108/`
  - Fixes S0's two defects: null register ids, and decoy routing truth.
  - Truth is now an explicit table, frozen before any run, with an expected
    route or abstention, answer targets and decoys.
  - The candidate list is validated and written out before any router is
    called.
  - **Independent qualification** (fresh Opus, two rounds; report in
    `qualification/report.md`): **D1 and D2 are qualified.** A blind test
    against the spec agreed on 16 of 20 cases; the 4 differences were on
    points the spec left ambiguous, which revision 2 resolved. The tests catch
    all 9 deliberate D1 and D2 breakages.
  - **Also found:** router failures that scored as abstentions. Round 1 found
    3 of these (H-1 to H-3, P2); round 2 found one more (R-1, P2: the
    substring router ignored a failed claims-tool call), plus P3s.
  - **Revision 3 fixes all of them.** A failure is now its own class,
    `router_error`, which counts as a false route under any S2 threshold.
    Live runs refuse missing profiles, directories and digests. 24 tests
    pass. Digests are in `SHA256SUMS-r3`.
  - **Round 3 (final; fresh Opus 5.5 high, stubs only, no new runs):
    revision 3 is qualified for keeping tool failure, abstention, false route
    and missed route apart.** No P1 or P2 is open; there are 6 P3s.
    - F-1: the S2 rule that `router_error` counts as a false route is in
      SPEC only; no code computes it.
    - F-2 and F-3: three failure-path mutants and the empty-cwd check are
      untested.
    - F-4: a failure reported only in the response body would look like no
      hits.
  - **S0 corrections:**
    - E8 changes from missed to correct abstention, for both routers;
    - `router_error` changes no S0 figure.
    - **S0's statement "the S2 default is met under every reading" is
      superseded (record section 14).** The default holds under the
      pre-registered truth rule, with zero margin if E5's null-id strings are
      mapped. It fails if E4, E6 and M4 are treated as abstain items with
      decoys, which the key leaves unsettled.
  - Report: `qualification/report-round3.md`, sha256 `22e9fe35…`. The round-2
    report is kept read-only as `qualification/report-round2.md`.
  - **By the user's decision,** F-1 and the unsettled truth designations are
    prerequisites for any S2 qualification decision.
  - It is a precondition for proposing S2 qualification. It is not part of
    DC0.
- **S0 register study:** integrated as commit `9fdf4b7e` (one new file,
  `metrics/register/studies/knowledge-oracle-s0-20261108.json`; not pushed),
  on the user's authorization. The routing-benchmark finding is marked
  provisional until the harness re-review reports.

## 10. Decisions requested

**Held by the user (2026-11-08):** DC0 Stage 1 (ceiling USD 20, warning 15;
Astra API-credit cap USD 2.50). It is not authorized. The section 6 costs
were forecast for revision 1's design and are re-priced after
reconciliation.

**Open once the decision-aware owner has answered K-1 to K-7:**

1. **Owners and dispositions:**
   - establish the owner of `context-boundary.h2-levels` (none is named);
   - then decide each of `ap-k5-conditional-context-boundary-h2`,
     `ap-k6-no-limit-pi-convergence` and `ap-k4-effort-cost-table-effort-row`
     separately, on its evidence.

   No bulk authorization (user, 2026-11-08).
2. **Decision-to-question links:** whether the register gains authorized
   decision-to-question links, or the card takes caller-supplied decision ids
   (K-7).
3. **Re-authorizing Stage 1** on the reconciled design, with a re-priced
   budget.
4. **The decision rule** in section 5: confirm or replace it.
