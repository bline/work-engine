# DC0: decision-card delivery feasibility (proposal, revision 3)

**Status: latest exploratory proposal. It is not an approved experiment.**
Nothing in it is authorized or running. Revision 3, 2026-11-09, prepared by
`dc0-revision-root` (Claude Code session `d6b8920e`, board adoption seq 4923)
on behalf of the Knowledge Oracle experiment owner.

- **Replaces revision 2 as the latest proposal.** Revision 2 stays committed,
  unchanged, at `planning/decision-card-delivery-feasibility-dc0-20261108.md`
  (sha256 `794a4ba9bbc4c706506d432cdf2d3c1aed87fd7543f57a8512f7f382ff4ca542`,
  commit `0405878c`). Revision 2's Stage 1 hold still applies.
- **Basis:** the decision-aware owner's K-1 to K-7 response (board seq 4916;
  `~/cbm-work/evidence-claims-integration-20261108/dc0-k-responses-20261109.md`,
  sha256 `2fcda5e0…`) and the user's decisions on it as relayed at seq 4918.
- **Pinned subject:** repository HEAD `9ac9826106d6afd303de384ddf86a6043fa11981`
  (branch `rescue-detached-20260911`, not pushed). Every register figure below
  is read at that commit.
- **Unchanged from revision 2 and not repeated here:** section 1 (why this
  step), sections 2.1 and 2.2 (revision 1's R0/R1 desk reading, kept as
  history) and section 9 (related work). Section 2.2 item 4 is corrected in
  section 6 below.

**Not authorized:** DC0 runs of any kind, the emulator or any adapter
implementation, claims or register changes, owner designations, applicability
approvals, skill release, push. This revision adds none of these.

## 0. What changed from revision 2

| Requirement (user, 2026-11-09) | Where |
|---|---|
| Separate strata replace the 10-of-12 gate; overlapping cases counted once per stratum | section 3 |
| R2 delivery plus metadata-only disclosure of nonqualifying records | sections 2 and 4 |
| Three-part card response, graded anchor warnings, corrective history kept | section 4 |
| `c4dd4826` corrected to the disposable K5 claims root | section 6 |
| Seq 4911 draft: differences listed | section 6 |
| Smaller first experiment, priced | section 5 |
| H2 authority note and five-record packet | off-repository, section 8 |

Revision 2's 72-run design (its sections 4 to 7) is kept as a possible later
stage. It is not proposed now. It would need re-pricing, the owner
dispositions in section 8, and its own authorization.

## 1. Register state at the pinned commit

- **Decisions:** 11 (`metrics/register/decisions.json`, counted at
  `9ac98261`). `decisions.json` and `applicability.jsonl` are byte-identical
  to `97f6486f`, the register state revision 2 read.
- **Applicability:** 13 records. One is authorized,
  `ap-k3-quota-fit-cost-table-quota-row`; 12 are `pending`.
- **Events:** 7, not revision 2's 6. Commit `46e5d008` added
  `ev-20261108-s0-routing-rescored-r3`, a correction between two
  `knowledge-oracle-s0-20261108` study findings. No decision lists either
  finding. The desk reading in section 2 confirms that it changes no
  decision's selection. Kinds: 3 corrections, 2 challenges, 2 status
  discrepancies. (The K response's basis line says the record files at
  `46e5d008` equal `97f6486f`. That holds for decisions and applicability, not
  for events.)
- **Materialized rows:** 0, so the claims leg is empty for every decision.
- **A3 fields:** `historical`, `revisions` and `evidence_state` exist as
  optional keys since `9ac98261`; no record uses them (seq 4920).
- **Anchors:** `register.py anchors --at 9ac98261`, output
  `~/cbm-work/dc0-revision-3/anchors-9ac98261/anchors.json/anchors.json`
  (sha256 `a30b698b7032966911f61cb0842579474e964a979857924bc97a64566b69da25`):
  4 unchanged, 4 moved, 3 modified, 0 relocated_verbatim, ambiguous, absent
  or error. All 4 moved anchors have `owner_review_required: false`. A1 sets
  that flag when a moved anchor's section identity changes, so all 4 moved
  within their section.

## 2. Desk reading under R2 with disclosure (no model runs)

**R2 (K-2, adopted):** the decision's own findings; plus findings linked by an
applicability record that is `authorized` for this decision revision; plus
findings named by a `correction` event that reaches a selected finding.
Nothing is inferred, and a pending record is not a link.

**Disclosure (K-2 amendment, approved by the user at seq 4918, metadata
only):** a register record that references the decision or a selected
finding, but does not qualify, is listed under "not delivered". The listing
gives id, type, proposed relation, authorization status and owner state. It
never gives the finding's content as admitted evidence.

**Method:** a desk script, not the emulator, reading the register at
`9ac98261` through `git show`. Script
`~/cbm-work/dc0-revision-3/desk/desk_selection.py` (sha256 `bf56bac8…`),
output `desk-selection-9ac98261.json` (sha256 `9f852e73…`), both in that
folder. It renders no card. Stage 1's expected lists (section 5.1) must be
prepared and frozen separately.

### 2.1 The five S0 decision items

"Key elements" are the register ids in each item's `sources` list in S0's
frozen key (`~/cbm-work/oracle-s0-20261108/private/key/key-frozen.json`).
Whether `sources` is the complete required-element list that revision 2
section 4.4 asks for is **not established**. That list does not exist yet.

| Item | Decision | Anchor at `9ac98261`, K-3 grade | Delivered under R2 (link type) | Disclosed, not delivered | Key elements not delivered |
|---|---|---|---|---|---|
| E3 | `model-selection.builder-default` | moved within section: notice | `dogfood-round2-builder-comparison#repair-and-first-review-comparison` (decision_finding) | `ap-k1-comparison-builder-default` (supports, pending) | `ap-k1-comparison-builder-default`'s content |
| E4 | `model-selection.cost-table-effort-row` | moved within section (line 92 to 278): notice | `sol-effort-per-call-cost#per-call-cost-by-effort` (decision_finding) | `ap-k4-effort-cost-table-effort-row` (supports, pending) | `ap-k4…`'s rationale ("says nothing about task-level effect"). The finding's own caveat "task-level effect unmeasured", per the key's notes, comes with the finding |
| E5 | `model-selection.claude-host-builder-rows` | modified: warning, citing `git:4ff69ffd` | both `e1a-first-artifact-battery` findings (decision_finding), with correction `ev-20261027-e1a-traceability` | `ap-k2-traced-claude-host-rows` (supports, pending) | `ap-k2…`'s content; claim `9014fb20` (claims leg empty) |
| M4 | `pi-docker-convergence.context-replacement` | unchanged | `handoff-threshold-replay-m22#conditional-result` (decision_finding); `…#rediscovery-free-reading` (correction_event `ev-20261018-m22-same-day-correction`) | `ap-k5-conditional-pi-convergence` (does_not_apply_directly, pending); `ap-k6-no-limit-pi-convergence` (does_not_apply_directly, pending) | `codex-compaction-points#no-limit-compaction-points`; both pending records' content; claims `c4dd4826` and `7d72bc32` (claims leg empty) |
| M5 | `context-boundary.h2-levels` | unchanged | both `codex-compaction-points` findings (decision_finding) | `ap-k5-conditional-context-boundary-h2` (not_established, pending); `ap-k6-no-limit-context-boundary-h2` and `ap-k6-limit-context-boundary-h2` (supports, pending) | both m22 findings, the m22 correction event, `ap-k5…h2`'s content, claim `c4dd4826` |

These figures cover 5 items, with n = 1 reading per item.

- **Every item has at least one key element in a pending record.**
  Seq 4918 notes only E4's `ap-k4-effort-cost-table-effort-row` among the P items. By the key's `sources`,
  E3's `ap-k1…` and E5's `ap-k2…` are also disclosed only, and E5 also cites a
  claim. Revision 2 called E3 and E5 "present" by revision 1's reading of the
  evidence needed. Which reading defines completeness waits on the
  required-element list (revision 2 section 4.4).
- **M4 depends on two pending records, not one.** Revision 2 section 2.3
  names only the K6 record. S0's frozen key also lists
  `ap-k5-conditional-pi-convergence` among M4's sources. That record is not
  in the five-record packet (section 8), and this revision does not add it.
  It is reported to the user as a relevant record outside the packet.
- **Records not covered by the disclosure rule.** The rule says "references
  the decision or a selected finding". Read literally, that includes
  applicability records belonging to *another* decision that name a selected
  finding. At `9ac98261` this applies to 5 of the 11 decisions, for example
  `ap-k6-no-limit-pi-convergence` on M5's card. Whether those records are
  disclosed is an open contract question for the decision-aware owner. It must
  be settled before Stage 1's expected lists are frozen.

## 3. Coverage strata (replaces the 10-of-12 gate)

This follows the user's coverage design at seq 4918 item 4. There is no
aggregate gate and there are no new items. The 7 non-decision items of S0
stay outside card coverage.

| Stratum | What it measures | Cases at `9ac98261` |
|---|---|---|
| **P**, presentation | the card form of evidence delivered under R2 today | E3, E4, E5 (n = 3). E4's `ap-k4-effort-cost-table-effort-row` rationale is disclosed only |
| **D**, metadata disclosure | whether nonqualifying records are listed as metadata and not taken as evidence establishing applicability | M4's K6 part (and its other pending record, section 2.1) and M5 (n = 2) |
| **A**, authorized corrective delivery | delivery of corrective evidence that reaches the decision only through an owner-authorized record | M4 and M5 enter only after their owners' separate dispositions. **Empty today** |

**Counting overlapping cases:**

- Each stratum reports its own n, and per-stratum counts are never summed.
- The number of distinct items (5) is stated once, separately.
- A case may sit in more than one stratum. Each of its register elements sits
  in exactly one stratum: delivered today, disclosed only, or delivered after
  authorization. Because no element is counted twice, M4 and M5 never count
  twice.
- If owners authorize a record, its element moves from D to A. The case
  appears in D only for whatever is still nonqualifying.
- M4's K5 finding and its m22 correction are delivered today. They are
  reported in M4's row as delivered elements. M4 is not counted as a P case,
  following seq 4918's assignment of P to E3 to E5.
- **If A is empty, the report states that corrective-evidence delivery was
  not tested.**

## 4. The card contract the emulator follows

The contract belongs to the decision-aware owner (K-1, K-2, K-3, K-4, K-7).
The user accepted it at seq 4918. The emulator is an experiment fixture
written by the Oracle experiment owner. It is never deployed and is not A4's
implementation. Before any agent run, the decision-aware owner checks its
rendered output for all 11 decisions and records concurrence or defects, under
its own budget.

**Input:** a caller-supplied exact `decision_id`, optionally `@revision`. In
DC0 the source is the experiment-authored item-to-decision mapping from S0's
item records, labelled as experiment-authored. There are no decision-to-question
links, and the card takes no questions.

**Response, three separate fields:**

1. **decision_state**
   - `registered`, `not_registered` (the id is echoed with the register
     commit; no evidence; no record invented) or `revision_unknown` (names the
     current revision).
   - `evidence_state` of `none_linked`, or `none_found` with its corpus, once
     a record uses A3's key. No record does at `9ac98261`, and every one of
     the 11 decisions has at least one finding.
   - `anchor`: `{class, decision_revision, target_commit,
     owner_review_required}` with its graded warning (below).
2. **evidence**
   - Delivered findings, each with its link type (`decision_finding`,
     `authorized_applicability`, `correction_event`). Each figure is shown in
     one entry with its scope, currency, strength, standing from recorded
     judgments, events, and, for authorized records, the applicability
     relation and rationale.
   - The "not delivered" list, as in section 2.
   - A coverage line: "selection limited to authorized links; N records
     disclosed, not delivered".
3. **claim_standing**
   - One entry per exact materialized ref, copied from the claims owner. The
     card never searches a store.
   - At `9ac98261` it is empty, and the coverage line says "no materialized
     rows at 9ac98261; native claims not consulted".

**Graded anchor warnings (K-3).** Evidence is delivered in every class.

| A1 class | Card output | Decisions at `9ac98261` |
|---|---|---|
| unchanged | none | 4 |
| moved, same section (`owner_review_required` false) | notice | 4 |
| moved across sections (`owner_review_required` true), relocated_verbatim | warning, owner review | 0 |
| modified, ambiguous, absent | warning naming the revision the judgments cite, stating that the decision text changed after it | 3 (modified) |
| error (unresolvable revision) | error, with the evidence still listed by id | 0 |

**Corrective history is never suppressed.** A correction event is delivered
with both of its findings. Once A3 revision history is used, an applicability
record authorized at a prior revision is shown as history at that revision:
never as current, never hidden. No record carries prior revisions at
`9ac98261`.

**Historical pin.** `model-selection.claude-host-builder-rows-at-df288c3d` is
historical by README prose only. A1 notes that no field marks it, and no
record uses A3's `historical` key. The card cannot mark it historical without
inference, so the emulator answers `registered` with a modified warning. Whether
the register should set the key is for the register owner.

**The emulator must not** use a model, open a claims store, or write outside
the experiment folder. **It reads** the register at the pinned commit and
A1's `anchors.json` produced with `--at` set to that commit, with the digest
recorded.

## 5. Smaller first experiment (proposed in place of the 72-run design)

### 5.1 Part 1: deterministic contract checks (no model runs in the experiment)

- **Cases:** all 11 registered decisions, plus one unknown id and one known id
  with an unknown revision.
- **Expected lists:** before the emulator first runs, an expected list per case
  is written and frozen with a digest, independently of the emulator code. It
  covers decision_state, the anchor fields and grade, delivered ids with link
  types, disclosed ids with their fields, the coverage line, and an empty
  claim_standing with its coverage line.
  - The open disclosure question in section 2.1 is settled first.
  - The desk reading in section 2 is a reference, not the frozen list.
- **Checks:**
  - every field matches exactly, and each mismatch is recorded as a defect,
    never silently fixed;
  - two runs are byte-identical;
  - register and A1 digests are unchanged;
  - no model call and no claims access.
- **Then** the decision-aware owner's concurrence check (K-1) on the rendered
  output for all 11 decisions.
- **It would establish** whether the emulator applies the contract to every
  registered decision and to the two unknown cases.
- **It would not establish** anything about agents, or anything about the
  real card implementation, because the emulator is a fixture.

### 5.2 Part 2: small agent comparison (separately authorized, after Part 1)

**Design.** 5 decision items (E3, E4, E5, M4, M5) × 2 arms × 2 repetitions =
20 runs.

- **Arm N, no card:** S0 arm A, worktree reading only.
- **Arm K, card:** arm N plus the emulator's card for the item's mapped
  decision id, in the prompt.
- **Why N and not S0's arm B:** A against K differs only by the card. B would
  add tool differences.
- S0's frozen key entries are reused, so no new key or curator work is needed.
  The model and isolation controls are S0's: `claude-sonnet-5-5` at medium
  effort, no Bash, Edit or Write, deny rules, transcript scan, sealed outputs.
- The primary scorer is blind to arm, and a second scorer reads all 20
  answers.
- **What is read:**
  - P items: correct, partial or incorrect, and false confident reuse, by arm;
  - D items: whether each K answer acknowledges the records listed as not
    delivered and does not treat them as evidence establishing applicability.
- No pass threshold is proposed. The report is per item and per stratum, and
  for each reading it names the single judgments that would change it.

**What it measures and what it does not:**

- **The combined card effect.** Arm K changes selection, presentation and
  delivery together, so N against K cannot separate their causal effects.
  Revision 2's Dr, Dp and Do arms were designed for that separation and are
  deferred with the 72-run design.
- **Not ten independent decision cases.** The two repetitions show run-to-run
  variation within an item. The independent unit is the item, n = 5 (3 in P,
  2 in D), so the results are anecdotal.
- **Authorized corrective delivery (stratum A) is not tested.** It is
  deferred until owner decisions authorize the M4 and M5 records. Until then
  the report states that corrective-evidence delivery was not tested.
- **Need recognition is not tested,** because the decision id is supplied.
- **Claim-standing behaviour is not tested,** because there are no
  materialized rows.
- **Also not tested:** any comparison with the Oracle, and any decision
  outside the register.

### 5.3 Cost of the smaller experiment

All figures are API-equivalent USD, forecast from S0 per-role actuals
(n = 1 per role), and `unverified` for DC0. Per-run means come from
`~/cbm-work/oracle-s0-20261108/results/s0-results.md` lines 36-44 (A 0.125,
B 0.116, C 0.067). Scorer and pilot rates are as quoted in revision 2
section 6 and are not re-derived here. The review rate is from 2026-11-08
Opus reviews: 2.38, 2.40 and 2.77.

| Part | Line | Basis | Low / point / high |
|---|---|---|---|
| 1 | Root: emulator script, expected lists, determinism runs | about 30-60 root calls at 0.05-0.11 per call (this objective: 0.058 per call over 35 calls; 2026-11-08: 0.11 at 350K context) | 2.0 / 3.5 / 6.0 |
| 1 | Independent check of the emulator, one fresh Opus round | 2.38-2.77 | 2.4 / 2.8 / 3.5 |
| | **Part 1 total** | | **4.4 / 6.3 / 9.5** |
| 2 | Arm runs, 20 | N 10 × 0.125; K 10 × 0.08-0.11 (revision 2's D-arm estimate); high adds 30% | 2.05 / 2.2 / 3.1 |
| 2 | Pilot, 2 runs | S0 pilot mean 0.071 | 0.15 / 0.2 / 0.25 |
| 2 | Primary scorer (20) and second scorer (20) | 0.052 and 0.051 per answer | 2.0 / 2.1 / 2.8 |
| 2 | Root | S0 stage 2 root 3.80, scaled to 20 runs | 1.5 / 2.5 / 4.0 |
| | **Part 2 total** | | **5.7 / 7.0 / 10.2** |

The decision-aware owner's concurrence check is on its own budget and is not
included. An Opus interpretation review (about 2.4-3.5) is not proposed for
n = 5 and is the user's option. **Proposed ceilings:** Part 1 USD 10, warning
7. Part 2 USD 11, warning 8, confirmed at the Part 1 checkpoint from pilot
actuals. For comparison, revision 2 forecast 14.7 and 20.8 at the point
estimate for its two stages.

### 5.4 Stop conditions

1. A part's ceiling is reached. Stop, even if the part is incomplete.
2. Any write to a claims store, the register, S0's records or A1's outputs.
3. The emulator is not deterministic, or it calls a model or reads claims.
4. A Part 1 expected-list mismatch is open, or the decision-aware owner
   objects (K-1). Part 2 is not proposed until it is resolved.
5. The key is exposed to an arm, or a run fails isolation.
6. The register or A1's classes change after the expected lists are frozen.
   Re-freeze before running.

## 6. Corrections to revision 2

**`c4dd4826` (revision 2 section 2.2 item 4).**

- **Wrong claim:** "The live store does hold a repository-fact claim,
  `claim-v1@c4dd4826…`, that states K5's conditional result."
- **Correction:** `c4dd4826` is the disposable K5 claims root's claim. Its
  creation result,
  `~/cbm-work/evidence-map-20261106/disposable-claims-root/results/k5-create.json`
  (sha256 `4352b6d0…`), records root id `work-engine-research-trial-20261107`.
  In S0's `~/cbm-work/oracle-s0-20261108/setup/corpus-export-w0.json` (sha256
  `b4a44d7f…`), a search of every string in the file finds the id only under
  `stores/disposable` (`claims/1`, `revisions/1`), never under `stores/live`.
  No claims store was opened, and no claim is changed.
- **Error class:** store attribution stated without reading the store path.
- **Consequence:** it implied that K5 had been published live and that
  materialization only needed to link an existing live claim.
- **Check that would have caught it:** read the store key of the id in the
  corpus export.

**The draft cited at seq 4911 (sha256 `6f2653e1…`) is recovered.** It is the
Oracle owner session's Claude Code file-history version
`~/.claude/file-history/0440d798-f8ee-43d7-bc70-a3700359872f/460834bc68ad2e01@v3`.
A read-only copy is at `~/cbm-work/dc0-revision-3/dc0-rev2-draft-6f2653e1.md`,
and the diff is `diff-6f2653e1-vs-794a4ba9.txt` beside it.

Against the committed revision 2, from the diff:

- **Status preamble:** the committed text adds the "Committed on 2026-11-08 as
  a held experimental proposal" paragraph and the four unmet prerequisites.
- **Section 2.3, option 1:** "Authorize first" becomes "Establish owners, then
  decide each record". The new text adds no bulk authorization and the
  unestablished owner of the h2 record.
- **Section 2.3, option 3:** drops "plus new items chosen on that basis" and
  adds the user's ruling against new items chosen to meet the threshold.
- **Section 9:** S0's "S2 default met under every reading" becomes
  "superseded (record section 14)", with the zero-margin E5 condition. The
  F-1 line becomes a user-decided prerequisite.
- **Section 10 item 1:** rewritten as owners first, then each record
  separately.

Sections 1, 2.1, 2.2 and 3 to 8 are byte-identical. This answers seq 4916's
and 4918's request: sections 1 to 8 differ only in section 2.3's options 1
and 3, plus the unnumbered preamble.

## 7. Ownership

As in revision 2 section 8, with these changes:

- K-6 is accepted (seq 4916). It confirms the user's provisional split.
- The emulator is written by the Oracle experiment owner as a fixture, and
  the decision-aware owner checks its output (K-1).
- The card contract (selection, disclosure, states, warnings) is the
  decision-aware owner's.
- Disclosure is approved by the user as metadata only (seq 4918).
- Owner designation and applicability decisions stay with the register owner
  and each decision's owner.

## 8. Decisions requested

1. **The smaller first experiment** (section 5) in place of the 72-run design
   as DC0's first step. The user decides.
2. **Part 1:** authorization and ceiling (proposed USD 10, warning 7).
   **Part 2:** a separate authorization after the Part 1 checkpoint (proposed
   USD 11, warning 8).
3. **Disclosure scope** (section 2.1): whether records on other decisions that
   name a selected finding are disclosed. The decision-aware owner decides.
4. **H2 continuing owner.** The user designates the owner after reading the
   evidence note
   `~/cbm-work/dc0-revision-3/h2-historical-authority-note.md`. Recording the
   designation in the register needs separate authorization.
5. **The five records**, each by its own owner and never in bulk:
   `~/cbm-work/dc0-revision-3/five-record-applicability-packet.md`. Three of
   them wait on item 4.
6. **Outside the packet:** `ap-k5-conditional-pi-convergence` (M4's other
   pending record, section 2.1). It stays out of the packet unless the user
   adds it.
7. **The historical pin marking** (section 4). The register owner decides.

Still open from seq 4912 and addressed to the Oracle owner: the three A3
questions. This revision does not answer them.
